//! Git adapter: everything that shells out to `git`. The pure core never sees this.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Result<T> = std::result::Result<T, String>;

pub(crate) fn run_git(root: &Path, args: &[&str], envs: &[(&str, &OsStr)]) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root).args(args);
    cmd.env("GIT_OPTIONAL_LOCKS", "0");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Repository root containing `cwd`.
pub fn repo_root(cwd: &Path) -> Result<PathBuf> {
    run_git(cwd, &["rev-parse", "--show-toplevel"], &[])
        .map(PathBuf::from)
        .map_err(|_| "not inside a git repository".to_owned())
}

/// Absolute path of the git directory (handles worktrees and submodules).
pub fn git_dir(root: &Path) -> Result<PathBuf> {
    run_git(root, &["rev-parse", "--absolute-git-dir"], &[]).map(PathBuf::from)
}

pub fn head_commit(root: &Path) -> Option<String> {
    run_git(root, &["rev-parse", "--verify", "-q", "HEAD"], &[]).ok()
}

fn pathspecs(root: &Path, exclude: &[String]) -> Vec<String> {
    let mut args: Vec<String> = ["add", "-A", "--", "."]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    args.extend(
        effective_exclude(root, exclude)
            .iter()
            .map(|e| format!(":(exclude){e}")),
    );
    args
}

/// Run git with `input` on stdin and return the raw output without judging the
/// exit status (some commands, like `check-ignore`, use it to answer).
fn git_output_stdin(
    root: &Path,
    args: &[&str],
    envs: &[(&str, &OsStr)],
    input: &[u8],
) -> Result<std::process::Output> {
    use std::io::Write;
    use std::process::Stdio;
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(root).args(args);
    cmd.env("GIT_OPTIONAL_LOCKS", "0");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input)
            .map_err(|e| format!("cannot write to git: {e}"))?;
    }
    child
        .wait_with_output()
        .map_err(|e| format!("cannot run git: {e}"))
}

/// Like [`run_git`] but feeds `input` on stdin.
pub(crate) fn run_git_stdin(
    root: &Path,
    args: &[&str],
    envs: &[(&str, &OsStr)],
    input: &[u8],
) -> Result<String> {
    let out = git_output_stdin(root, args, envs, input)?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// `git add` refuses a pathspec that names an existing path which
/// `.gitignore` ignores ("The following paths are ignored by one of your
/// .gitignore files"). Such an exclusion is redundant, since ignored paths are
/// never added, so it is dropped. Only literal paths are considered: patterns
/// with wildcards never trigger the refusal. Any doubt keeps the exclusion.
fn effective_exclude(root: &Path, exclude: &[String]) -> Vec<String> {
    let literal = |e: &str| !e.contains(['*', '?', '[', '\\']);
    let existing: Vec<&str> = exclude
        .iter()
        .map(String::as_str)
        .filter(|e| literal(e))
        .map(|e| e.trim_end_matches('/'))
        // The refusal also fires when only an ancestor exists (`dir/sub` with an
        // ignored `dir/`), so test the first path component.
        .filter(|e| {
            let first = e.split('/').next().unwrap_or(e);
            !first.is_empty() && fs::symlink_metadata(root.join(first)).is_ok()
        })
        .collect();
    if existing.is_empty() {
        return exclude.to_vec();
    }
    let mut input = Vec::new();
    for e in &existing {
        input.extend_from_slice(e.as_bytes());
        input.push(0);
    }
    // `--no-index`: judge by the ignore rules alone. Our private index starts
    // empty, so a path tracked in the user's real index is still "untracked
    // and ignored" to `git add`.
    // Exit status 1 means "none of them is ignored"; 128 means a fatal error.
    let ignored: Vec<String> = match git_output_stdin(
        root,
        &["check-ignore", "--no-index", "-z", "--stdin"],
        &[],
        &input,
    ) {
        Ok(out) if matches!(out.status.code(), Some(0 | 1)) => String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect(),
        _ => return exclude.to_vec(),
    };
    exclude
        .iter()
        .filter(|e| {
            let t = e.trim_end_matches('/');
            !(literal(e) && ignored.iter().any(|i| i == t))
        })
        .cloned()
        .collect()
}

/// Tree id of the *working directory* (tracked files plus untracked,
/// non-ignored files, minus `exclude`), computed with a private index so that
/// the user's real index, refs and files are never touched (ADR-0002).
///
/// Uses a persistent private index (git re-hashes only files whose stat data
/// changed, about 6 ms instead of 34 ms for 3,000 files). If that index is
/// unusable (corrupt, locked by another process) the answer is computed from
/// scratch, so the optimisation can never change the result.
pub fn worktree_tree(root: &Path, git_dir: &Path, exclude: &[String]) -> Result<String> {
    match persistent_tree(root, git_dir, exclude) {
        Ok(tree) => Ok(tree),
        // Busy, corrupt or locked by git itself: answer from scratch. A bad index
        // is removed by `persistent_tree` itself, while it holds the lock.
        Err(_) => fresh_tree(root, git_dir, exclude),
    }
}

/// One private index per exclusion set: entries added under one set must never
/// leak into another (`git add -A` does not remove excluded paths).
pub(crate) fn persistent_index_path(git_dir: &Path, exclude: &[String]) -> PathBuf {
    let key = opshero_core::record::id_of(&serde_json::json!(exclude)).unwrap_or_default();
    git_dir
        .join("opshero")
        .join(format!("index-{}", key.get(..32).unwrap_or("default")))
}

/// How long to wait for another recorder using the same private index before
/// giving up and computing the tree from scratch.
const INDEX_LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// The cleanup, add and write-tree steps form one sequence over a shared file.
/// Without mutual exclusion another process can delete the index between two
/// steps (an empty tree was recorded under 40 parallel runs, 2026-10-21) or
/// write a tree from a state that is newer than the one just measured. The
/// whole sequence therefore runs under an exclusive lock.
pub(crate) fn persistent_tree(root: &Path, git_dir: &Path, exclude: &[String]) -> Result<String> {
    let scratch = git_dir.join("opshero");
    fs::create_dir_all(&scratch)
        .map_err(|e| format!("cannot create {}: {e}", scratch.display()))?;
    let index = persistent_index_path(git_dir, exclude);
    let mut lock_name = index.clone().into_os_string();
    lock_name.push(".flock");
    let Some(_guard) = crate::lock::acquire(Path::new(&lock_name), INDEX_LOCK_WAIT) else {
        return Err("the private index is busy".into());
    };
    let result = persistent_tree_locked(root, exclude, &index);
    if result.is_err() {
        // Safe to remove: we hold the lock, so nobody is using it. Never touch
        // git's own `.lock` file, which may belong to a live git process.
        let _ = fs::remove_file(&index);
    }
    result
}

fn persistent_tree_locked(root: &Path, exclude: &[String], index: &Path) -> Result<String> {
    let envs: [(&str, &OsStr); 1] = [("GIT_INDEX_FILE", index.as_os_str())];
    // `add -A` never removes an entry for a file that still exists but has
    // since become ignored, and refuses pathspecs that name such entries; a
    // from-scratch index would not contain them. Drop them BEFORE adding.
    let ignored = run_git(
        root,
        &["ls-files", "-ci", "--exclude-standard", "-z"],
        &envs,
    )?;
    let ignored: Vec<&str> = ignored.split('\0').filter(|p| !p.is_empty()).collect();
    if !ignored.is_empty() {
        if ignored.iter().any(|p| p.contains('\u{fffd}')) {
            return Err("non-UTF-8 path in the index; using the from-scratch computation".into());
        }
        let mut input = Vec::new();
        for p in &ignored {
            input.extend_from_slice(p.as_bytes());
            input.push(0);
        }
        run_git_stdin(
            root,
            &["update-index", "--force-remove", "-z", "--stdin"],
            &envs,
            &input,
        )?;
    }
    let add = pathspecs(root, exclude);
    let add_refs: Vec<&str> = add.iter().map(String::as_str).collect();
    run_git(root, &add_refs, &envs)?;
    run_git(root, &["write-tree"], &envs)
}

/// The reference computation: a brand-new temporary index every time.
pub(crate) fn fresh_tree(root: &Path, git_dir: &Path, exclude: &[String]) -> Result<String> {
    let scratch = git_dir.join("opshero");
    fs::create_dir_all(&scratch)
        .map_err(|e| format!("cannot create {}: {e}", scratch.display()))?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let index = scratch.join(format!("tmp-index-{}-{nanos}", std::process::id()));
    let envs: [(&str, &OsStr); 1] = [("GIT_INDEX_FILE", index.as_os_str())];
    let add = pathspecs(root, exclude);
    let add_refs: Vec<&str> = add.iter().map(String::as_str).collect();
    let result =
        run_git(root, &add_refs, &envs).and_then(|_| run_git(root, &["write-tree"], &envs));
    let _ = fs::remove_file(&index);
    result
}

/// Paths that differ between two tree ids, restricted to `pathspecs` (git pathspec
/// semantics) when given. Both trees were written by `worktree_tree`, so they are
/// in the object database.
pub fn changed_paths(
    root: &Path,
    from: &str,
    to: &str,
    pathspecs: &[String],
) -> Result<Vec<String>> {
    let mut args: Vec<&str> = vec![
        "diff-tree",
        "-r",
        "--name-only",
        "--no-renames",
        "-z",
        from,
        to,
    ];
    if !pathspecs.is_empty() {
        args.push("--");
        args.extend(pathspecs.iter().map(String::as_str));
    }
    let out = run_git(root, &args, &[])?;
    Ok(out
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Resolve `refname` to a commit id, EXACTLY. Git's own name lookup is ambiguous on purpose
/// (`main` finds a tag before a branch, `origin/main` finds a tag or a local branch before the
/// remote-tracking ref), and a gate that reads its policy from a ref must not be steerable by an
/// object of the same name in another namespace. So only these forms are accepted:
///
/// - a full commit id (40 or 64 hex digits);
/// - a fully qualified ref, `refs/...`;
/// - `<remote>/<branch>` for a configured remote: always fetched, then read from exactly
///   `refs/remotes/<remote>/<branch>` (a shallow pull-request checkout does not have it);
/// - a bare branch name, read from exactly `refs/heads/<name>`.
///
/// Anything else, and any name that could be taken for an option, is refused before git sees it.
pub fn resolve_or_fetch(root: &Path, refname: &str) -> Result<String> {
    validate_refname(refname)?;
    if is_full_commit_id(refname) {
        return resolve_exact(root, refname);
    }
    if refname.starts_with("refs/") {
        return resolve_exact(root, refname);
    }
    if let Some((remote, branch)) = refname.split_once('/')
        && crate::evidence::remote_exists(root, remote)
    {
        let tracking = format!("refs/remotes/{remote}/{branch}");
        let spec = format!("+refs/heads/{branch}:{tracking}");
        run_git(
            root,
            &["fetch", "--quiet", "--no-tags", "--depth=1", remote, &spec],
            &[],
        )
        .map_err(|e| format!("cannot fetch `{refname}` from `{remote}`: {e}"))?;
        return resolve_exact(root, &tracking);
    }
    resolve_exact(root, &format!("refs/heads/{refname}"))
}

fn is_full_commit_id(s: &str) -> bool {
    matches!(s.len(), 40 | 64) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Reject names that git could read as an option, a revision expression or a range, before any
/// git process sees them.
fn validate_refname(name: &str) -> Result<()> {
    let bad = |why: &str| Err(format!("`{name}` is not an acceptable ref name: {why}"));
    if name.is_empty() {
        return bad("it is empty");
    }
    if name.starts_with('-') {
        return bad("it starts with `-`");
    }
    if name.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return bad("it contains whitespace or control characters");
    }
    if is_full_commit_id(name) {
        return Ok(());
    }
    // git's own grammar for ref names (no `..`, `@{`, `^`, `~`, `:`, `?`, `*`, `[`, `\`, ...).
    let probe = if name.starts_with("refs/") {
        name.to_owned()
    } else {
        format!("refs/heads/{name}")
    };
    let ok = Command::new("git")
        .args(["check-ref-format", &probe])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        bad("git does not accept it as a ref name")
    }
}

fn resolve_exact(root: &Path, full: &str) -> Result<String> {
    run_git(
        root,
        &["rev-parse", "--verify", "-q", &format!("{full}^{{commit}}")],
        &[],
    )
    .map_err(|_| format!("`{full}` is not a known commit or branch"))
}

/// Contents of `path` in commit `sha`.
pub fn show_file_at(root: &Path, sha: &str, path: &str) -> Result<String> {
    run_git(root, &["show", &format!("{sha}:{path}")], &[])
}

/// `dirty` = tracked or untracked (non-ignored) changes exist outside the
/// excluded paths, according to `git status`.
pub fn is_dirty(root: &Path, exclude: &[String]) -> bool {
    let mut args: Vec<String> = ["status", "--porcelain", "--", "."]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    args.extend(exclude.iter().map(|e| format!(":(exclude){e}")));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_git(root, &refs, &[])
        .map(|out| !out.is_empty())
        .unwrap_or(true)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    //! The persistent private index is an optimisation: it must never change
    //! the answer. Every test compares it with the from-scratch computation.
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    fn git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    }

    fn repo() -> (TempDir, PathBuf) {
        let d = TempDir::new().unwrap();
        git(d.path(), &["init", "-q", "-b", "main"]);
        git(d.path(), &["commit", "-q", "--allow-empty", "-m", "init"]);
        let gd = git_dir(d.path()).unwrap();
        (d, gd)
    }

    const FILES: [&str; 7] = ["a", "b.log", "dir/c", "dir/sub/d", "x.txt", "e", "e/f"];
    const CONTENTS: [&str; 4] = ["x", "y", "yy", "zzz"];
    const IGNORES: [&str; 4] = ["", "*.log\n", "dir/\n", "a\ne\n"];
    const EXCLUDES: [&[&str]; 4] = [&[], &["dir"], &["*.log"], &["a", "dir/sub"]];

    #[derive(Clone, Debug)]
    enum Op {
        Write(usize, usize),
        Remove(usize),
        SetIgnore(usize),
        Commit,
    }

    fn op() -> impl Strategy<Value = Op> {
        prop_oneof![
            4 => (0..FILES.len(), 0..CONTENTS.len()).prop_map(|(f, c)| Op::Write(f, c)),
            2 => (0..FILES.len()).prop_map(Op::Remove),
            2 => (0..IGNORES.len()).prop_map(Op::SetIgnore),
            1 => Just(Op::Commit),
        ]
    }

    fn apply(dir: &Path, op: &Op) {
        match op {
            Op::Write(f, c) => {
                let p = dir.join(FILES[*f]);
                if let Some(parent) = p.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(p, CONTENTS[*c]); // may fail on file/dir clashes: fine
            }
            Op::Remove(f) => {
                let p = dir.join(FILES[*f]);
                let _ = fs::remove_file(&p).or_else(|_| fs::remove_dir_all(&p));
            }
            Op::SetIgnore(i) => {
                let _ = fs::write(dir.join(".gitignore"), IGNORES[*i]);
            }
            Op::Commit => {
                let _ = Command::new("git")
                    .arg("-C")
                    .arg(dir)
                    .args(["add", "-A"])
                    .output();
                let _ = Command::new("git")
                    .arg("-C")
                    .arg(dir)
                    .args([
                        "-c",
                        "user.name=t",
                        "-c",
                        "user.email=t@x",
                        "commit",
                        "-q",
                        "-m",
                        "c",
                        "--allow-empty",
                    ])
                    .output();
            }
        }
    }

    fn excludes(i: usize) -> Vec<String> {
        EXCLUDES[i].iter().map(|s| (*s).to_owned()).collect()
    }

    fn assert_same(root: &Path, gd: &Path, ex: &[String], context: &str) {
        let fresh = fresh_tree(root, gd, ex).unwrap();
        let persistent = persistent_tree(root, gd, ex).unwrap();
        assert_eq!(persistent, fresh, "persistent index diverged: {context}");
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 24, ..ProptestConfig::default() })]

        /// After any sequence of file operations, ignore-rule changes and
        /// commits, the persistent index yields exactly the from-scratch tree,
        /// for every exclusion set (each set has its own index).
        #[test]
        fn persistent_tree_equals_fresh_tree(ops in proptest::collection::vec(op(), 1..14)) {
            let (d, gd) = repo();
            for (n, o) in ops.iter().enumerate() {
                apply(d.path(), o);
                for set in EXCLUDES {
                    let ex: Vec<String> = set.iter().map(|s| (*s).to_owned()).collect();
                    assert_same(d.path(), &gd, &ex, &format!("after op {n} {o:?} with exclude {set:?}; ops {ops:?}"));
                }
            }
        }
    }

    /// Same-size rewrites in quick succession are the "racily clean" case of
    /// the git index: the stat cache cannot tell them apart by size, so git
    /// must re-hash. Checked without any sleep between writes.
    #[test]
    fn rapid_same_size_rewrites_are_never_missed() {
        let (d, gd) = repo();
        let ex = excludes(0);
        let mut seen = std::collections::HashSet::new();
        for i in 0..150 {
            fs::write(d.path().join("a"), if i % 2 == 0 { "x" } else { "y" }).unwrap();
            let t = persistent_tree(d.path(), &gd, &ex).unwrap();
            assert_eq!(t, fresh_tree(d.path(), &gd, &ex).unwrap(), "iteration {i}");
            seen.insert(t);
        }
        assert_eq!(
            seen.len(),
            2,
            "two different contents must give two different trees"
        );
    }

    /// A file that becomes ignored after it was recorded in the private index
    /// must disappear from the tree.
    #[test]
    fn a_file_that_becomes_ignored_leaves_the_tree() {
        let (d, gd) = repo();
        let ex = excludes(0);
        fs::write(d.path().join("b.log"), "x").unwrap();
        let with = persistent_tree(d.path(), &gd, &ex).unwrap();
        fs::write(d.path().join(".gitignore"), "*.log\n").unwrap();
        let without = persistent_tree(d.path(), &gd, &ex).unwrap();
        assert_ne!(with, without);
        assert_eq!(without, fresh_tree(d.path(), &gd, &ex).unwrap());
    }

    /// Different exclusion sets must not share an index: entries added for a
    /// path under one set would otherwise leak into the other.
    #[test]
    fn exclusion_sets_do_not_contaminate_each_other() {
        let (d, gd) = repo();
        fs::create_dir_all(d.path().join("dir")).unwrap();
        fs::write(d.path().join("dir/c"), "x").unwrap();
        fs::write(d.path().join("a"), "x").unwrap();
        let none = excludes(0);
        let dir = excludes(1);
        let t_none = persistent_tree(d.path(), &gd, &none).unwrap();
        let t_dir = persistent_tree(d.path(), &gd, &dir).unwrap();
        assert_ne!(t_none, t_dir);
        assert_eq!(persistent_tree(d.path(), &gd, &none).unwrap(), t_none);
        assert_eq!(persistent_tree(d.path(), &gd, &dir).unwrap(), t_dir);
    }

    /// If the private index is unusable (corrupt, or locked by another process)
    /// the public function still answers, correctly, from scratch.
    #[test]
    fn a_corrupt_or_locked_private_index_falls_back_to_the_fresh_tree() {
        let (d, gd) = repo();
        let ex = excludes(0);
        fs::write(d.path().join("a"), "x").unwrap();
        let want = fresh_tree(d.path(), &gd, &ex).unwrap();
        assert_eq!(worktree_tree(d.path(), &gd, &ex).unwrap(), want);
        let idx = persistent_index_path(&gd, &ex);
        fs::write(&idx, b"not an index").unwrap();
        assert_eq!(
            worktree_tree(d.path(), &gd, &ex).unwrap(),
            want,
            "corrupt index"
        );
        let _ = fs::remove_file(&idx);
        fs::write(idx.with_extension("lock"), b"").unwrap_or(());
        let mut lock = idx.clone().into_os_string();
        lock.push(".lock");
        fs::write(&lock, b"").unwrap();
        assert_eq!(
            worktree_tree(d.path(), &gd, &ex).unwrap(),
            want,
            "locked index"
        );
    }

    /// Regression: `git add` refuses a pathspec that names an existing path
    /// which `.gitignore` ignores ("paths are ignored by one of your .gitignore
    /// files"). Many repositories ignore `.claude/` or `.cursor/`, which are
    /// excluded by default, so this made the tool unusable there.
    #[test]
    fn an_excluded_path_that_is_also_ignored_does_not_break_the_tree() {
        let (d, gd) = repo();
        fs::write(d.path().join(".gitignore"), ".claude/\nbuild/\n").unwrap();
        fs::create_dir_all(d.path().join(".claude")).unwrap();
        fs::write(d.path().join(".claude/settings.json"), "x").unwrap();
        fs::create_dir_all(d.path().join("build/out")).unwrap();
        fs::write(d.path().join("build/out/o"), "x").unwrap();
        fs::write(d.path().join("code.txt"), "x").unwrap();
        let ex: Vec<String> = [".claude", "build", "build/out", ".claude/"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let none: Vec<String> = vec![];
        let want = fresh_tree(d.path(), &gd, &none).unwrap(); // ignored paths are never in the tree
        assert_eq!(fresh_tree(d.path(), &gd, &ex).unwrap(), want);
        assert_eq!(persistent_tree(d.path(), &gd, &ex).unwrap(), want);
        assert_eq!(worktree_tree(d.path(), &gd, &ex).unwrap(), want);
    }

    /// The refusal also fires for an exclusion that does not exist itself when
    /// an ancestor does and is ignored (`dir/sub` with an ignored `dir/`).
    #[test]
    fn an_exclusion_below_an_ignored_directory_does_not_break_the_tree() {
        let (d, gd) = repo();
        fs::write(d.path().join(".gitignore"), "dir/\n").unwrap();
        fs::create_dir_all(d.path().join("dir")).unwrap();
        fs::write(d.path().join("dir/c"), "x").unwrap();
        let ex = vec!["dir/sub".to_owned()]; // dir/sub itself does not exist
        let want = fresh_tree(d.path(), &gd, &[]).unwrap();
        assert_eq!(fresh_tree(d.path(), &gd, &ex).unwrap(), want);
        assert_eq!(persistent_tree(d.path(), &gd, &ex).unwrap(), want);
    }

    /// A path tracked in the user's real index and later added to `.gitignore`:
    /// `git check-ignore` stays silent about tracked paths unless told to
    /// ignore the index, but our private index starts empty.
    #[test]
    fn a_tracked_path_that_becomes_ignored_and_excluded_does_not_break_the_tree() {
        let (d, gd) = repo();
        fs::create_dir_all(d.path().join("dir")).unwrap();
        fs::write(d.path().join("dir/c"), "x").unwrap();
        git(d.path(), &["add", "-A"]);
        git(
            d.path(),
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@x",
                "commit",
                "-q",
                "-m",
                "track",
            ],
        );
        fs::write(d.path().join(".gitignore"), "dir/\n").unwrap();
        let ex = vec!["dir".to_owned()];
        let want = fresh_tree(d.path(), &gd, &[]).unwrap();
        assert_eq!(fresh_tree(d.path(), &gd, &ex).unwrap(), want);
        assert_eq!(persistent_tree(d.path(), &gd, &ex).unwrap(), want);
    }

    /// `git status` has the same sensitivity; `dirty` must still be right.
    #[test]
    fn dirty_is_computed_with_ignored_exclusions() {
        let (d, _gd) = repo();
        fs::write(d.path().join(".gitignore"), ".claude/\n").unwrap();
        git(d.path(), &["add", "-A"]);
        git(
            d.path(),
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@x",
                "commit",
                "-q",
                "-m",
                "ignore",
            ],
        );
        fs::create_dir_all(d.path().join(".claude")).unwrap();
        fs::write(d.path().join(".claude/s"), "x").unwrap();
        let ex = vec![".claude".to_owned()];
        assert!(
            !is_dirty(d.path(), &ex),
            "only ignored/excluded files changed"
        );
        fs::write(d.path().join("code.txt"), "x").unwrap();
        assert!(is_dirty(d.path(), &ex));
    }

    /// Concurrent callers (many recorders at once) must each get the true tree,
    /// never the empty tree and never an error. A probe with 40 parallel runs
    /// once recorded the empty tree for 11 of them.
    #[test]
    fn concurrent_callers_always_get_the_right_tree() {
        use std::sync::{Arc, Barrier};
        let (d, gd) = repo();
        fs::write(d.path().join("a"), "x").unwrap();
        fs::create_dir_all(d.path().join("dir")).unwrap();
        fs::write(d.path().join("dir/c"), "y").unwrap();
        let ex = excludes(0);
        let want = fresh_tree(d.path(), &gd, &ex).unwrap();
        assert_ne!(
            want, "4b825dc642cb6eb9a060e54bf8d69288fbee4904",
            "fixture must not be empty"
        );
        let root = Arc::new(d.path().to_path_buf());
        let gdir = Arc::new(gd);
        let start = Arc::new(Barrier::new(12));
        let handles: Vec<_> = (0..12)
            .map(|_| {
                let (r, g, b, e) = (root.clone(), gdir.clone(), start.clone(), ex.clone());
                std::thread::spawn(move || {
                    b.wait();
                    (0..8)
                        .map(|_| worktree_tree(&r, &g, &e).unwrap())
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in handles {
            for tree in h.join().unwrap() {
                assert_eq!(tree, want);
            }
        }
    }

    /// The user's real index must stay untouched (ADR-0002).
    #[test]
    fn the_real_index_is_never_modified() {
        let (d, gd) = repo();
        fs::write(d.path().join("a"), "x").unwrap();
        let before = fs::read(gd.join("index")).ok();
        persistent_tree(d.path(), &gd, &excludes(0)).unwrap();
        assert_eq!(fs::read(gd.join("index")).ok(), before);
    }
}
