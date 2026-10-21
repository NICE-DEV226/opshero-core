//! Evidence transport (RFC-0002 OQ1): publish the records bound to one tree to a
//! dedicated git ref, and fetch them in CI.
//!
//! One ref per subject tree: `refs/opshero/evidence/<tree id>`, pointing at a
//! parentless commit whose tree holds `<record id>.json` files. A fresh checkout
//! of the same content has the same tree id, so CI finds exactly the evidence
//! that was published for the content it is about to merge, and nothing else.
//!
//! Trust: published records are whatever the publisher recorded (assurance A1).
//! The reader checks every record's hash, so a record cannot be edited, but a
//! dishonest publisher can still invent a whole record. Resisting that needs
//! evidence produced by the CI itself or signed (assurance A2/A3, RFC-0002).

use crate::git::{run_git, run_git_stdin};
use crate::store::Store;
use opshero_core::record::{self, Record};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

pub const REF_PREFIX: &str = "refs/opshero/evidence/";
/// Never publish or import more than this many records for one tree.
pub const MAX_RECORDS: usize = 200;
const MAX_RECORD_BYTES: usize = 256 * 1024;

pub fn ref_for(tree: &str) -> String {
    format!("{REF_PREFIX}{tree}")
}

fn fetched_ref(tree: &str) -> String {
    format!("refs/opshero/fetched/{tree}")
}

/// Records read from a published commit, and what was wrong with the rest.
pub struct Fetched {
    pub records: Vec<Record>,
    pub notes: Vec<String>,
}

fn is_missing_ref(msg: &str) -> bool {
    msg.contains("couldn't find remote ref")
}

/// Fetch the evidence ref for `tree` into a local tracking ref.
/// `Ok(None)` when the remote simply has nothing for this tree.
fn fetch_ref(root: &Path, remote: &str, tree: &str) -> Result<Option<String>, String> {
    let spec = format!("+{}:{}", ref_for(tree), fetched_ref(tree));
    match run_git(root, &["fetch", "--quiet", "--no-tags", remote, &spec], &[]) {
        Ok(_) => run_git(root, &["rev-parse", &fetched_ref(tree)], &[]).map(Some),
        Err(e) if is_missing_ref(&e) => Ok(None),
        Err(e) => Err(e),
    }
}

/// `<mode> <type> <oid> <size>\t<path>` (from `ls-tree -l`): the path and the size.
/// Entries that are not blobs (a `-` size) are refused.
fn parse_ls_tree_entry(entry: &str) -> Option<(&str, usize)> {
    let (meta, name) = entry.split_once('\t')?;
    let mut fields = meta.split_whitespace();
    let (_mode, kind, _oid, size) = (
        fields.next()?,
        fields.next()?,
        fields.next()?,
        fields.next()?,
    );
    (kind == "blob").then_some(())?;
    Some((name, size.parse().ok()?))
}

/// Read and verify every record in a published commit.
fn read_commit(root: &Path, commit: &str) -> Result<Fetched, String> {
    // `-l` lists each blob's size, so an oversized file is skipped without ever being read.
    let listing = run_git(root, &["ls-tree", "-r", "-l", "-z", commit], &[])?;
    let mut records = Vec::new();
    let mut notes = Vec::new();
    for (i, entry) in listing.split('\0').filter(|n| !n.is_empty()).enumerate() {
        if i >= MAX_RECORDS {
            notes.push(format!(
                "more than {MAX_RECORDS} published records: the rest were ignored"
            ));
            break;
        }
        let Some((name, size)) = parse_ls_tree_entry(entry) else {
            notes.push(format!("{entry:?}: ignored (unreadable listing entry)"));
            continue;
        };
        if size > MAX_RECORD_BYTES {
            notes.push(format!(
                "{name}: ignored (larger than {MAX_RECORD_BYTES} bytes)"
            ));
            continue;
        }
        let blob = match run_git(
            root,
            &["cat-file", "blob", &format!("{commit}:{name}")],
            &[],
        ) {
            Ok(b) => b,
            Err(e) => {
                notes.push(format!("{name}: unreadable ({e})"));
                continue;
            }
        };
        let raw: serde_json::Value = match serde_json::from_str(&blob) {
            Ok(v) => v,
            Err(e) => {
                notes.push(format!("{name}: ignored ({e})"));
                continue;
            }
        };
        match record::parse_verified(&raw) {
            Ok(rec) if format!("{}.json", rec.id) == name => records.push(rec),
            Ok(_) => notes.push(format!("{name}: ignored (file name does not match its id)")),
            Err(e) => notes.push(format!("{name}: ignored ({e})")),
        }
    }
    Ok(Fetched { records, notes })
}

/// What `verify --ci` learned from the remote.
pub struct Imported {
    pub count: usize,
    /// Human-readable lines for the report's findings.
    pub notes: Vec<String>,
}

/// Fetch the evidence published for `tree` and add the verified records to the local store.
/// Failures are reported in `notes`, never as errors: a CI gate must fail closed (the
/// verdict becomes UNKNOWN), not crash.
pub fn fetch_and_import(root: &Path, gitdir: &Path, remote: &str, tree: &str) -> Imported {
    let mut notes = Vec::new();
    let commit = match fetch_ref(root, remote, tree) {
        Ok(Some(c)) => c,
        Ok(None) => {
            notes.push(format!("no evidence published for this tree on `{remote}`"));
            return Imported { count: 0, notes };
        }
        Err(e) => {
            notes.push(format!("could not fetch evidence from `{remote}`: {e}"));
            return Imported { count: 0, notes };
        }
    };
    let fetched = match read_commit(root, &commit) {
        Ok(f) => f,
        Err(e) => {
            notes.push(format!("could not read the published evidence: {e}"));
            return Imported { count: 0, notes };
        }
    };
    notes.extend(fetched.notes);
    let store = Store::open(gitdir);
    let mut count = 0;
    for rec in &fetched.records {
        match store.import(rec) {
            Ok(_) => count += 1,
            Err(e) => notes.push(format!("{}: not stored ({e})", rec.id)),
        }
    }
    notes.push(format!("imported {count} record(s) from `{remote}`"));
    Imported { count, notes }
}

/// Publish the local records bound to `tree` (merged with what the remote already
/// holds for it) and return how many records the published commit contains.
pub fn publish(
    root: &Path,
    gitdir: &Path,
    remote: &str,
    tree: &str,
    exclude: &[String],
) -> Result<usize, String> {
    let (local, _notes) = Store::open(gitdir).load();
    let mut merged: BTreeMap<String, Record> = local
        .into_iter()
        .filter(|r| r.body.subject.tree_after == tree && r.body.subject.exclude == exclude)
        .map(|r| (r.id.clone(), r))
        .collect();
    // Merge with what other machines already published for this tree.
    if let Some(commit) = fetch_ref(root, remote, tree)? {
        for rec in read_commit(root, &commit)?.records {
            merged.entry(rec.id.clone()).or_insert(rec);
        }
    }
    if merged.is_empty() {
        return Err("no recorded run is bound to the current content: run the checks first".into());
    }
    // Deterministic cap: newest by recorded start time, then id (the time is only used here).
    let mut all: Vec<&Record> = merged.values().collect();
    all.sort_by(|a, b| {
        b.body
            .execution
            .started_at
            .cmp(&a.body.execution.started_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    all.truncate(MAX_RECORDS);
    let mut lines: Vec<String> = Vec::new();
    for rec in &all {
        let body = serde_json::to_string(rec).map_err(|e| e.to_string())?;
        let oid = run_git_stdin(
            root,
            &["hash-object", "-w", "--stdin"],
            &[],
            body.as_bytes(),
        )?;
        lines.push(format!("100644 blob {oid}\t{}.json\n", rec.id));
    }
    lines.sort(); // git requires sorted entries; names are hex ids
    let tree_obj = run_git_stdin(root, &["mktree"], &[], lines.concat().as_bytes())?;
    // Fixed identity and dates: the same set of records always gives the same commit id.
    let fixed: [(&str, &OsStr); 6] = [
        ("GIT_AUTHOR_NAME", OsStr::new("opshero")),
        ("GIT_AUTHOR_EMAIL", OsStr::new("opshero@localhost")),
        ("GIT_AUTHOR_DATE", OsStr::new("1970-01-01T00:00:00Z")),
        ("GIT_COMMITTER_NAME", OsStr::new("opshero")),
        ("GIT_COMMITTER_EMAIL", OsStr::new("opshero@localhost")),
        ("GIT_COMMITTER_DATE", OsStr::new("1970-01-01T00:00:00Z")),
    ];
    let message = format!("opshero evidence for {tree}");
    let commit = run_git(root, &["commit-tree", &tree_obj, "-m", &message], &fixed)?;
    run_git(root, &["update-ref", &ref_for(tree), &commit], &[])?;
    let spec = format!("+{0}:{0}", ref_for(tree));
    run_git(root, &["push", "--quiet", remote, &spec], &[])
        .map_err(|e| format!("cannot publish to `{remote}`: {e}"))?;
    Ok(all.len())
}

/// Is `remote` a configured git remote?
pub fn remote_exists(root: &Path, remote: &str) -> bool {
    run_git(root, &["remote", "get-url", remote], &[]).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ls_tree_entries_give_the_path_and_the_size_and_refuse_anything_else() {
        let oid = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";
        assert_eq!(
            parse_ls_tree_entry(&format!("100644 blob {oid}      12\tabc.json")),
            Some(("abc.json", 12))
        );
        assert_eq!(
            parse_ls_tree_entry(&format!("100644 blob {oid} 2097152\tbig.json")),
            Some(("big.json", 2_097_152))
        );
        // A submodule or a tree has no size and is not a record.
        assert_eq!(
            parse_ls_tree_entry(&format!("160000 commit {oid}       -\tsub")),
            None
        );
        assert_eq!(
            parse_ls_tree_entry(&format!("040000 tree {oid}       -\tdir")),
            None
        );
        assert_eq!(parse_ls_tree_entry("garbage"), None);
        assert_eq!(
            parse_ls_tree_entry(&format!("100644 blob {oid} notanumber\tx")),
            None
        );
    }
}
