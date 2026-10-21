//! Evaluation of a policy against evidence: a pure function (RFC-0001 P1).
//!
//! `evaluate(policy, subject, records, findings)` never reads a clock, the
//! filesystem or the environment, and its result does not depend on the order
//! in which `records` are supplied.

use crate::policy::{Assurance, Policy, PostRunChanges, Require, Severity, command_matches};
use crate::record::Record;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

pub const REPORT_SCHEMA: &str = "opshero.verify/v0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    Pass,
    Fail,
    Unknown,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Unknown => "UNKNOWN",
        }
    }
}

/// The content being judged, as measured by the CLI adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectView {
    pub worktree_tree: String,
    pub head_commit: Option<String>,
    pub dirty: bool,
    /// Exclusions used to compute `worktree_tree`; evidence only counts when
    /// it was recorded with the same exclusions.
    pub exclude: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub id: String,
    pub result: Verdict,
    pub severity: Severity,
    pub reason: String,
    pub evidence: Vec<String>,
    pub assurance: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubjectOut {
    pub tree: String,
    pub head_commit: Option<String>,
    pub dirty: bool,
    pub exclude: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: &'static str,
    pub subject: SubjectOut,
    pub policy: String,
    pub verdict: Verdict,
    pub assurance: Option<String>,
    pub checks: Vec<CheckResult>,
    pub findings: Vec<String>,
}

fn short(id: &str) -> &str {
    id.get(..12).unwrap_or(id)
}

fn assurance_of(r: &Record) -> Assurance {
    Assurance::parse(&r.body.recorder.assurance).unwrap_or(Assurance::A0)
}

fn passed(r: &Record) -> bool {
    r.body.execution.exit_code == Some(0) && r.body.execution.signal.is_none()
}

/// Position of each record in its hash chain: the number of ancestors that
/// are present in the set. Used only to pick the "latest" record
/// deterministically; timestamps are never consulted.
///
/// Linear time: each record's depth is its parent's depth plus one, computed
/// once. (The original version re-walked the whole ancestry of every record,
/// which took 2.6 s for 6,000 records and minutes for 30,000.)
fn chain_depths(records: &[Record]) -> HashMap<&str, usize> {
    let by_id: HashMap<&str, &Record> = records.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut depth: HashMap<&str, usize> = HashMap::with_capacity(records.len());
    let mut path: Vec<&Record> = Vec::new();
    for start in records {
        if depth.contains_key(start.id.as_str()) {
            continue;
        }
        // Climb until a record with a known depth or no parent in the set.
        path.clear();
        let mut cur = start;
        let mut base = 0usize; // depth of the record above the last one on `path`, plus one
        loop {
            path.push(cur);
            match cur.body.prev.as_deref().and_then(|p| by_id.get(p)) {
                None => {
                    base = 0;
                    break;
                }
                Some(parent) => match depth.get(parent.id.as_str()) {
                    Some(d) => {
                        base = *d + 1;
                        break;
                    }
                    None => {
                        // Ids are content hashes, so a cycle cannot be built; this guard
                        // keeps a corrupted set from looping forever.
                        if path.len() > records.len() {
                            break;
                        }
                        cur = parent;
                    }
                },
            }
        }
        for (i, r) in path.iter().rev().enumerate() {
            depth.insert(r.id.as_str(), base + i);
        }
    }
    depth
}

/// Records from newest to oldest by position in the hash chain (deeper first),
/// ties broken by id. Independent of the order of `records`; no clock is read.
/// Used to decide which evidence to keep when the store is pruned.
pub fn newest_first(records: &[Record]) -> Vec<&Record> {
    let depth = chain_depths(records);
    let mut v: Vec<&Record> = records.iter().collect();
    v.sort_by(|a, b| {
        let da = depth.get(a.id.as_str()).copied().unwrap_or(0);
        let db = depth.get(b.id.as_str()).copied().unwrap_or(0);
        db.cmp(&da).then_with(|| a.id.cmp(&b.id))
    });
    v
}

fn latest<'a>(recs: &[&'a Record], depth: &HashMap<&str, usize>) -> Option<&'a Record> {
    recs.iter().copied().max_by(|a, b| {
        let da = depth.get(a.id.as_str()).copied().unwrap_or(0);
        let db = depth.get(b.id.as_str()).copied().unwrap_or(0);
        da.cmp(&db).then_with(|| b.id.cmp(&a.id))
    })
}

fn ordered_ids(recs: &[&Record], depth: &HashMap<&str, usize>) -> Vec<String> {
    let mut v: Vec<&Record> = recs.to_vec();
    v.sort_by(|a, b| {
        let da = depth.get(a.id.as_str()).copied().unwrap_or(0);
        let db = depth.get(b.id.as_str()).copied().unwrap_or(0);
        db.cmp(&da).then_with(|| a.id.cmp(&b.id))
    });
    v.iter().map(|r| r.id.clone()).collect()
}

fn check(
    req: &Require,
    subject: &SubjectView,
    records: &[Record],
    depth: &HashMap<&str, usize>,
    findings: &mut Vec<String>,
) -> CheckResult {
    let mk =
        |result: Verdict, reason: String, evidence: Vec<String>, assurance: Option<Assurance>| {
            CheckResult {
                id: req.id.clone(),
                result,
                severity: req.severity,
                reason,
                evidence,
                assurance: assurance.map(|a| a.as_str().to_owned()),
            }
        };
    let pattern = req.command.join(" ");
    let matching: Vec<&Record> = records
        .iter()
        .filter(|r| command_matches(&req.command, &r.body.command.argv))
        .collect();
    if matching.is_empty() {
        return mk(
            Verdict::Unknown,
            format!("no recorded run matches `{pattern}`"),
            vec![],
            None,
        );
    }
    let bound: Vec<&Record> = matching
        .iter()
        .copied()
        .filter(|r| {
            r.body.subject.tree_after == subject.worktree_tree
                && r.body.subject.exclude == subject.exclude
        })
        .collect();
    // `deny`: a run that saw the working tree change is not evidence for it.
    let deny = req.post_run_changes == PostRunChanges::Deny;
    let (current, moved): (Vec<&Record>, Vec<&Record>) = bound
        .into_iter()
        .partition(|r| !deny || r.body.subject.worktree_tree == r.body.subject.tree_after);
    // Distinct records only: the same record supplied twice is one run.
    let mut seen = HashSet::new();
    let current: Vec<&Record> = current
        .into_iter()
        .filter(|r| seen.insert(r.id.as_str()))
        .collect();
    let current_pass: Vec<&Record> = current.iter().copied().filter(|r| passed(r)).collect();
    let current_ok: Vec<&Record> = current_pass
        .iter()
        .copied()
        .filter(|r| assurance_of(r) >= req.min_assurance)
        .collect();

    // Repeat rule (RFC-0005 §3.1): only meaningful once a pass exists on the subject.
    if let Some(rep) = req.repeat
        && !current_pass.is_empty()
    {
        let failed = current.len() - current_pass.len();
        if failed > 0 {
            return mk(
                Verdict::Fail,
                format!(
                    "inconsistent results for `{pattern}` on the current tree: {} passed, {failed} failed (flaky or environment-dependent)",
                    current_pass.len()
                ),
                ordered_ids(&current, depth),
                None,
            );
        }
        let need = rep.runs as usize;
        if current_ok.len() < need {
            return mk(
                Verdict::Unknown,
                format!(
                    "`{pattern}` passed {} of the {need} runs required by the repeat rule on the current tree",
                    current_ok.len()
                ),
                ordered_ids(&current_pass, depth),
                None,
            );
        }
    }

    if !current_ok.is_empty() {
        for r in &current_ok {
            if r.body.subject.worktree_tree != r.body.subject.tree_after {
                findings.push(format!(
                    "record {}: the command modified the worktree (tree {} -> {}); freshness uses the state after the run",
                    short(&r.id),
                    short(&r.body.subject.worktree_tree),
                    short(&r.body.subject.tree_after)
                ));
            }
        }
        let best = current_ok.iter().map(|r| assurance_of(r)).max();
        return mk(
            Verdict::Pass,
            format!(
                "`{pattern}` passed on the current tree ({} run(s))",
                current_ok.len()
            ),
            ordered_ids(&current_ok, depth),
            best,
        );
    }
    if !current_pass.is_empty() {
        let have = current_pass
            .iter()
            .map(|r| assurance_of(r))
            .max()
            .unwrap_or(Assurance::A0);
        return mk(
            Verdict::Unknown,
            format!(
                "a passing run exists on the current tree but its assurance {} is below the required {}",
                have.as_str(),
                req.min_assurance.as_str()
            ),
            ordered_ids(&current_pass, depth),
            None,
        );
    }
    if let Some(last) = latest(&current, depth) {
        let how = match (last.body.execution.exit_code, last.body.execution.signal) {
            (Some(c), _) => format!("exit code {c}"),
            (None, Some(s)) => format!("signal {s}"),
            (None, None) => "unknown status".to_owned(),
        };
        return mk(
            Verdict::Fail,
            format!("`{pattern}` failed on the current tree ({how})"),
            ordered_ids(&current, depth),
            None,
        );
    }
    if moved.iter().any(|r| passed(r)) {
        return mk(
            Verdict::Fail,
            format!(
                "`{pattern}` passed, but the working tree changed while it ran ({} run(s)): the result may describe other content. Rerun it, or list generated paths under [subject] exclude",
                moved.len()
            ),
            ordered_ids(&moved, depth),
            None,
        );
    }
    let passing_elsewhere: Vec<&Record> = matching.iter().copied().filter(|r| passed(r)).collect();
    match latest(&passing_elsewhere, depth) {
        Some(last) => mk(
            Verdict::Fail,
            format!(
                "stale evidence: `{pattern}` last passed on tree {}, the current tree is {}",
                short(&last.body.subject.tree_after),
                short(&subject.worktree_tree)
            ),
            vec![last.id.clone()],
            None,
        ),
        None => mk(
            Verdict::Fail,
            format!(
                "`{pattern}` has {} recorded run(s), none passing, none on the current tree",
                matching.len()
            ),
            ordered_ids(&matching, depth),
            None,
        ),
    }
}

/// Evaluate `policy` for `subject` given the verified `records`.
/// `findings` carries notes gathered while loading evidence (for example
/// records rejected by the integrity check); they are reported verbatim.
pub fn evaluate(
    policy: &Policy,
    subject: &SubjectView,
    records: &[Record],
    mut findings: Vec<String>,
) -> Report {
    let depth = chain_depths(records);
    let checks: Vec<CheckResult> = policy
        .require
        .iter()
        .map(|req| check(req, subject, records, &depth, &mut findings))
        .collect();
    findings.sort();
    findings.dedup();

    let blocking = || checks.iter().filter(|c| c.severity == Severity::Block);
    let verdict = if blocking().any(|c| c.result == Verdict::Fail) {
        Verdict::Fail
    } else if blocking().any(|c| c.result == Verdict::Unknown) {
        Verdict::Unknown
    } else {
        Verdict::Pass
    };
    let assurance = if verdict == Verdict::Pass {
        blocking()
            .filter_map(|c| c.assurance.as_deref().and_then(Assurance::parse))
            .min()
            .map(|a| a.as_str().to_owned())
    } else {
        None
    };
    Report {
        schema: REPORT_SCHEMA,
        subject: SubjectOut {
            tree: subject.worktree_tree.clone(),
            head_commit: subject.head_commit.clone(),
            dirty: subject.dirty,
            exclude: subject.exclude.clone(),
        },
        policy: policy.id.clone(),
        verdict,
        assurance,
        checks,
        findings,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::policy;
    use crate::record::{new_record, tests::sample_body};

    fn pol(extra: &str) -> Policy {
        policy::parse(&format!(
            "version=1\nid=\"p\"\n[[require]]\nid=\"tests\"\ncheck=\"evidence.fresh\"\ncommand=[\"pytest\",\"*\"]\n{extra}"
        ))
        .unwrap()
    }

    fn subj(tree: &str) -> SubjectView {
        SubjectView {
            worktree_tree: tree.into(),
            head_commit: None,
            dirty: true,
            exclude: vec![],
        }
    }

    fn rec(tree: &str, exit: i64, prev: Option<&str>) -> Record {
        new_record(sample_body(tree, exit, prev)).unwrap()
    }

    fn first(r: &Report) -> &CheckResult {
        &r.checks[0]
    }

    #[test]
    fn no_evidence_is_unknown_never_pass() {
        let r = evaluate(&pol(""), &subj("t1"), &[], vec![]);
        assert_eq!(r.verdict, Verdict::Unknown);
        assert_eq!(first(&r).result, Verdict::Unknown);
    }

    #[test]
    fn passing_run_on_the_current_tree_passes() {
        let r = evaluate(&pol(""), &subj("t1"), &[rec("t1", 0, None)], vec![]);
        assert_eq!(r.verdict, Verdict::Pass);
        assert_eq!(r.assurance.as_deref(), Some("A1"));
        assert_eq!(first(&r).evidence.len(), 1);
    }

    #[test]
    fn evidence_on_another_tree_is_stale() {
        let r = evaluate(&pol(""), &subj("t2"), &[rec("t1", 0, None)], vec![]);
        assert_eq!(r.verdict, Verdict::Fail);
        assert!(first(&r).reason.starts_with("stale evidence"));
    }

    #[test]
    fn failing_run_on_the_current_tree_fails() {
        let r = evaluate(&pol(""), &subj("t1"), &[rec("t1", 3, None)], vec![]);
        assert_eq!(r.verdict, Verdict::Fail);
        assert!(first(&r).reason.contains("exit code 3"));
    }

    #[test]
    fn a_later_pass_on_the_same_tree_wins_over_an_earlier_failure() {
        let a = rec("t1", 1, None);
        let b = rec("t1", 0, Some(&a.id));
        let r = evaluate(&pol(""), &subj("t1"), &[a, b], vec![]);
        assert_eq!(r.verdict, Verdict::Pass);
    }

    #[test]
    fn insufficient_assurance_is_unknown() {
        let r = evaluate(
            &pol("min_assurance=\"A2\"\n"),
            &subj("t1"),
            &[rec("t1", 0, None)],
            vec![],
        );
        assert_eq!(r.verdict, Verdict::Unknown);
        assert!(first(&r).reason.contains("below the required A2"));
    }

    #[test]
    fn warn_severity_never_changes_the_verdict() {
        let r = evaluate(&pol("severity=\"warn\"\n"), &subj("t1"), &[], vec![]);
        assert_eq!(first(&r).result, Verdict::Unknown);
        assert_eq!(r.verdict, Verdict::Pass);
    }

    #[test]
    fn record_order_does_not_change_the_report() {
        let a = rec("t0", 0, None);
        let b = rec("t1", 1, Some(&a.id));
        let c = rec("t1", 0, Some(&b.id));
        let p = pol("");
        let s = subj("t1");
        let j1 = serde_json::to_string(&evaluate(
            &p,
            &s,
            &[a.clone(), b.clone(), c.clone()],
            vec![],
        ))
        .unwrap();
        let j2 = serde_json::to_string(&evaluate(
            &p,
            &s,
            &[c.clone(), a.clone(), b.clone()],
            vec![],
        ))
        .unwrap();
        let j3 = serde_json::to_string(&evaluate(&p, &s, &[b, c, a], vec![])).unwrap();
        assert_eq!(j1, j2);
        assert_eq!(j2, j3);
    }

    #[test]
    fn worktree_modification_is_reported_as_a_finding() {
        let mut body = sample_body("before", 0, None);
        body.subject.tree_after = "after".into();
        let r = new_record(body).unwrap();
        // `allow` is the explicit opt-out; the default (`deny`) is covered in tests/verdict_laws.rs.
        let rep = evaluate(
            &pol("post_run_changes=\"allow\"\n"),
            &subj("after"),
            std::slice::from_ref(&r),
            vec![],
        );
        assert_eq!(rep.verdict, Verdict::Pass);
        assert!(
            rep.findings
                .iter()
                .any(|f| f.contains("modified the worktree"))
        );
        let strict = evaluate(&pol(""), &subj("after"), std::slice::from_ref(&r), vec![]);
        assert_eq!(strict.verdict, Verdict::Fail);
    }

    #[test]
    fn evidence_recorded_with_different_exclusions_is_not_comparable() {
        let r = rec("t1", 0, None); // recorded with no exclusions
        let mut s = subj("t1");
        s.exclude = vec![".claude".into()];
        let rep = evaluate(&pol(""), &s, &[r], vec![]);
        assert_ne!(rep.verdict, Verdict::Pass);
    }

    #[test]
    fn non_matching_commands_do_not_count() {
        let mut body = sample_body("t1", 0, None);
        body.command.argv = vec!["echo".into(), "ok".into()];
        let r = new_record(body).unwrap();
        let rep = evaluate(&pol(""), &subj("t1"), &[r], vec![]);
        assert_eq!(rep.verdict, Verdict::Unknown);
    }

    /// The original quadratic definition: for each record, walk its ancestors.
    fn depths_reference(records: &[Record]) -> HashMap<String, usize> {
        let by_id: HashMap<&str, &Record> = records.iter().map(|r| (r.id.as_str(), r)).collect();
        records
            .iter()
            .map(|r| {
                let mut steps = 0usize;
                let mut cur = r;
                while let Some(prev) = cur.body.prev.as_deref().and_then(|p| by_id.get(p)) {
                    steps += 1;
                    cur = prev;
                }
                (r.id.clone(), steps)
            })
            .collect()
    }

    #[test]
    fn chain_depths_match_the_reference_on_forests_duplicates_and_dangling_parents() {
        // A fork, a duplicate, a record whose parent is absent, and a long tail.
        let a = rec("t1", 0, None);
        let b = rec("t2", 0, Some(&a.id));
        let c = rec("t3", 0, Some(&a.id)); // fork: same parent as b
        let d = rec("t4", 0, Some(&b.id));
        let orphan = rec("t5", 0, Some(&"f".repeat(64))); // parent not in the set
        let mut all = vec![a, b.clone(), c, d.clone(), orphan, b, d];
        let mut prev = all.last().unwrap().id.clone();
        for i in 0..20 {
            let r = rec(&format!("tail{i}"), 0, Some(&prev));
            prev = r.id.clone();
            all.push(r);
        }
        let fast = chain_depths(&all);
        let slow = depths_reference(&all);
        assert_eq!(fast.len(), slow.len());
        for (id, depth) in &slow {
            assert_eq!(fast.get(id.as_str()), Some(depth), "record {id}");
        }
    }

    /// A long history must not make verification quadratic: 6,000 records took
    /// 2.6 s in the field. 30,000 records in a single chain must stay far below
    /// what a quadratic walk (4.5e8 steps) could manage in a debug build.
    #[test]
    fn a_very_long_chain_is_evaluated_in_linear_time() {
        let mut recs: Vec<Record> = Vec::with_capacity(30_000);
        let mut prev: Option<String> = None;
        for i in 0..30_000u32 {
            let r = rec(&format!("tree{i}"), 0, prev.as_deref());
            prev = Some(r.id.clone());
            recs.push(r);
        }
        let started = std::time::Instant::now();
        let rep = evaluate(&pol(""), &subj("tree29999"), &recs, vec![]);
        let took = started.elapsed();
        assert_eq!(rep.verdict, Verdict::Pass);
        assert!(
            took < std::time::Duration::from_secs(4),
            "evaluation took {took:?} for 30,000 records"
        );
    }

    #[test]
    fn newest_first_puts_children_before_parents_whatever_the_input_order() {
        let a = rec("t1", 0, None);
        let b = rec("t2", 0, Some(&a.id));
        let c = rec("t3", 0, Some(&b.id));
        for input in [
            vec![a.clone(), b.clone(), c.clone()],
            vec![c.clone(), a.clone(), b.clone()],
            vec![b.clone(), c.clone(), a.clone()],
        ] {
            let order: Vec<&str> = newest_first(&input).iter().map(|r| r.id.as_str()).collect();
            assert_eq!(order, [c.id.as_str(), b.id.as_str(), a.id.as_str()]);
        }
    }
}
