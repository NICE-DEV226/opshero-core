//! Operations shared by the CLI commands and the agent adapters.

use crate::{git, store};
use opshero_core::policy::Policy;
use opshero_core::record::{
    self, Command, Execution, Observation, RECORD_VERSION, RecordBody, Recorder, Subject,
};
use opshero_core::timefmt;
use opshero_core::verify::{self, Report, SubjectView};
use std::path::{Path, PathBuf};

/// Everything needed to write one evidence record.
pub struct Capture {
    pub argv: Vec<String>,
    pub env_names: Vec<String>,
    pub cwd_rel: String,
    pub tree_before: String,
    pub tree_after: String,
    pub head: Option<String>,
    pub dirty: bool,
    pub exclude: Vec<String>,
    pub exit_code: Option<i64>,
    pub signal: Option<i64>,
    pub started_secs: u64,
    pub duration_ms: u64,
    pub adapter: Option<&'static str>,
    /// `A1` for local runs, `A2` for runs executed by the CI (`opshero ci run`).
    pub assurance: &'static str,
}

/// How long a recorder waits for the chain lock. Recording is more important
/// than a linear chain, so after this the record is written without the lock
/// (the chain may then fork, which does not change any verdict).
const CHAIN_LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// The store is pruned automatically when it holds more than `AUTO_PRUNE_ABOVE`
/// records, down to `AUTO_PRUNE_KEEP` (plus everything bound to the tree just
/// recorded). Verification time grows with the number of records (about 140 ms
/// at 6,000, measured), and without a bound the loader would one day have to
/// ignore some of them.
pub const AUTO_PRUNE_ABOVE: usize = 5_000;
pub const AUTO_PRUNE_KEEP: usize = 4_000;

pub fn record(gitdir: &Path, c: Capture) -> Result<(), String> {
    record_with_limits(gitdir, c, AUTO_PRUNE_ABOVE, AUTO_PRUNE_KEEP)
}

pub(crate) fn record_with_limits(
    gitdir: &Path,
    c: Capture,
    prune_above: usize,
    prune_keep: usize,
) -> Result<(), String> {
    let (tree_after, exclude) = (c.tree_after.clone(), c.exclude.clone());
    let store = store::Store::open(gitdir);
    // Reading the chain head and appending must be one step: otherwise two
    // recorders get the same parent, the chain forks, and two runs with the same
    // content can even collapse into a single record.
    let dir = gitdir.join("opshero");
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let _chain = crate::lock::acquire(&dir.join("store.flock"), CHAIN_LOCK_WAIT);
    let body = RecordBody {
        record_version: RECORD_VERSION,
        prev: store.head(),
        subject: Subject {
            worktree_tree: c.tree_before,
            tree_after: c.tree_after,
            head_commit: c.head,
            dirty: c.dirty,
            exclude: c.exclude,
        },
        command: Command {
            argv: c.argv,
            env_names: c.env_names,
            cwd: c.cwd_rel,
        },
        execution: Execution {
            exit_code: c.exit_code,
            signal: c.signal,
            started_at: timefmt::rfc3339_utc(c.started_secs),
            duration_ms: c.duration_ms,
        },
        observation: Observation {
            method: "none".into(),
        },
        recorder: Recorder {
            name: "opshero".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            assurance: c.assurance.into(),
            adapter: c.adapter.map(str::to_owned),
        },
    };
    let rec = record::new_record(body).map_err(|e| e.to_string())?;
    store.append(&rec)?;
    // Still under the chain lock: nobody else is writing while we prune.
    if store.count() > prune_above {
        let protect = |r: &record::Record| {
            r.body.subject.tree_after == tree_after && r.body.subject.exclude == exclude
        };
        if let Err(e) = store.prune(prune_keep, false, &protect) {
            eprintln!("opshero: warning: could not prune old evidence: {e}");
        }
    }
    Ok(())
}

/// `cwd` relative to the repository root ("." when equal or unresolvable).
pub fn rel_cwd(root: &Path, cwd: &Path) -> String {
    cwd.canonicalize()
        .ok()
        .zip(root.canonicalize().ok())
        .and_then(|(c, r)| {
            c.strip_prefix(r)
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| ".".to_owned())
}

/// `dir` relative to the repository root, or `None` when it is outside the repository or
/// cannot be resolved. Unlike [`rel_cwd`] it never answers "." for a directory that is
/// somewhere else: a run in another directory is not evidence for this repository.
pub fn inside_root(root: &Path, dir: &Path) -> Option<String> {
    let d = dir.canonicalize().ok()?;
    let r = root.canonicalize().ok()?;
    let rel = d.strip_prefix(r).ok()?;
    Some(if rel.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        rel.to_string_lossy().into_owned()
    })
}

/// Is `path` inside the repository, that is, content the author of the change controls?
pub fn is_inside_repo(root: &Path, path: &Path) -> bool {
    inside_root(root, path.parent().unwrap_or(path)).is_some()
}

pub fn policy_path(root: &Path) -> PathBuf {
    root.join(".opshero").join("policy.toml")
}

/// Exclusions to use when no parsed policy is at hand (for `run`): the defaults
/// plus the policy's own, if a valid policy exists in the repository.
pub fn subject_excludes(root: &Path) -> Vec<String> {
    std::fs::read_to_string(policy_path(root))
        .ok()
        .and_then(|t| opshero_core::policy::parse(&t).ok())
        .map(|p| p.effective_excludes())
        .unwrap_or_else(opshero_core::policy::default_excludes)
}

/// Measure the current subject and evaluate `policy` against stored evidence.
pub fn evaluate_now(root: &Path, gitdir: &Path, policy: &Policy) -> Result<Report, String> {
    let exclude = policy.effective_excludes();
    let tree = git::worktree_tree(root, gitdir, &exclude)?;
    let subject = SubjectView {
        head_commit: git::head_commit(root),
        dirty: git::is_dirty(root, &exclude),
        worktree_tree: tree,
        exclude,
    };
    let (records, notes) = store::Store::open(gitdir).load();
    Ok(verify::evaluate(policy, &subject, &records, notes))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::{Arc, Barrier};
    use tempfile::TempDir;

    fn capture() -> Capture {
        // Every field identical on purpose: only the chain position tells two runs apart.
        Capture {
            argv: vec!["true".into()],
            env_names: vec![],
            cwd_rel: ".".into(),
            tree_before: "t".into(),
            tree_after: "t".into(),
            head: None,
            dirty: false,
            exclude: vec![],
            exit_code: Some(0),
            signal: None,
            started_secs: 1_700_000_000,
            duration_ms: 1,
            adapter: None,
            assurance: "A1",
        }
    }

    /// Many recorders released at the same instant must produce one linear
    /// chain with every record present. Threads, not processes, because they
    /// reach the read-head-then-append window far more often.
    #[test]
    fn concurrent_recorders_build_one_linear_chain_and_lose_nothing() {
        const THREADS: usize = 16;
        const EACH: usize = 20;
        let dir = TempDir::new().unwrap();
        let gitdir = Arc::new(dir.path().to_path_buf());
        let start = Arc::new(Barrier::new(THREADS));
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let (g, b) = (gitdir.clone(), start.clone());
                std::thread::spawn(move || {
                    b.wait();
                    for _ in 0..EACH {
                        record(&g, capture()).unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let (records, notes) = store::Store::open(&gitdir).load();
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(
            records.len(),
            THREADS * EACH,
            "records were lost or collapsed"
        );
        let ids: HashSet<&str> = records.iter().map(|r| r.id.as_str()).collect();
        let mut parents = HashSet::new();
        let mut roots = 0;
        for r in &records {
            match r.body.prev.as_deref() {
                None => roots += 1,
                Some(p) => {
                    assert!(ids.contains(p), "dangling parent");
                    assert!(parents.insert(p), "chain forked at {p}");
                }
            }
        }
        assert_eq!(roots, 1);
        // The head points at the only record that nobody names as a parent.
        let leaf = records
            .iter()
            .find(|r| !parents.contains(r.id.as_str()))
            .unwrap();
        assert_eq!(
            store::Store::open(&gitdir).head().as_deref(),
            Some(leaf.id.as_str())
        );
    }

    fn capture_on(tree: &str) -> Capture {
        Capture {
            tree_before: tree.into(),
            tree_after: tree.into(),
            ..capture()
        }
    }

    fn passes_on(dir: &Path, tree: &str) -> bool {
        let policy = opshero_core::policy::parse(
            "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"true\"]\n",
        )
        .unwrap();
        let (records, _) = store::Store::open(dir).load();
        let subject = SubjectView {
            worktree_tree: tree.into(),
            head_commit: None,
            dirty: false,
            exclude: vec![],
        };
        verify::evaluate(&policy, &subject, &records, vec![]).verdict == verify::Verdict::Pass
    }

    #[test]
    fn prune_keeps_the_newest_and_everything_protected_and_leaves_a_valid_store() {
        let dir = TempDir::new().unwrap();
        for i in 0..30 {
            record(dir.path(), capture_on(&format!("t{i}"))).unwrap();
        }
        let st = store::Store::open(dir.path());
        let protect = |r: &record::Record| r.body.subject.tree_after == "t3";
        let dry = st.prune(10, true, &protect).unwrap();
        assert_eq!((dry.total, dry.removed, dry.kept), (30, 19, 11));
        assert_eq!(st.count(), 30, "a dry run deletes nothing");

        let done = st.prune(10, false, &protect).unwrap();
        assert_eq!((done.removed, done.kept, done.protected), (19, 11, 1));
        let (records, notes) = st.load();
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(records.len(), 11);
        assert!(passes_on(dir.path(), "t3"), "protected evidence survives");
        assert!(passes_on(dir.path(), "t29"), "the newest survives");
        assert!(
            !passes_on(dir.path(), "t5"),
            "old unprotected evidence is gone"
        );
        // The chain head still names the newest record, and recording goes on.
        record(dir.path(), capture_on("t30")).unwrap();
        assert!(passes_on(dir.path(), "t30"));
        assert_eq!(
            st.prune(10, false, &protect).unwrap().removed,
            1,
            "only the overflow goes"
        );
    }

    #[test]
    fn prune_leaves_unverified_and_temporary_files_alone() {
        let dir = TempDir::new().unwrap();
        for i in 0..5 {
            record(dir.path(), capture_on(&format!("t{i}"))).unwrap();
        }
        let recs = dir.path().join("opshero/records");
        std::fs::write(recs.join("corrupt.json"), "not json").unwrap();
        std::fs::write(recs.join(".tmp-x"), "partial").unwrap();
        store::Store::open(dir.path())
            .prune(1, false, &|_| false)
            .unwrap();
        assert!(recs.join("corrupt.json").exists());
        assert!(recs.join(".tmp-x").exists());
    }

    #[test]
    fn the_store_is_bounded_automatically_without_losing_the_current_tree() {
        let dir = TempDir::new().unwrap();
        for i in 0..120 {
            // Every fifth run is on the "current" tree "cur"; the rest are history.
            let tree = if i % 5 == 0 {
                "cur".to_owned()
            } else {
                format!("t{i}")
            };
            record_with_limits(dir.path(), capture_on(&tree), 40, 20).unwrap();
            assert!(
                store::Store::open(dir.path()).count() <= 41 + 24,
                "the store grew past its bound at record {i}"
            );
        }
        assert!(
            passes_on(dir.path(), "cur"),
            "evidence for the tree being recorded is never pruned"
        );
        assert!(passes_on(dir.path(), "t119"));
        let (_, notes) = store::Store::open(dir.path()).load();
        assert!(notes.is_empty(), "{notes:?}");
    }
}
