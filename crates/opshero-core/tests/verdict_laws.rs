//! Laws of the verdict function (docs/rfc/0005-verdict-semantics-and-flaky-evidence.md).
//!
//! Each test states one law and checks it over randomly generated policies,
//! subjects and evidence sets. A failing case is shrunk by proptest to a
//! minimal counter-example. These tests use only the public API.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use opshero_core::policy::{self, Policy};
use opshero_core::record::{
    Command, Execution, Observation, RECORD_VERSION, Record, RecordBody, Recorder, Subject,
    new_record, parse_verified,
};
use opshero_core::verify::{Report, SubjectView, Verdict, evaluate};
use proptest::prelude::*;

// ---------------------------------------------------------------- generators

#[derive(Clone, Debug)]
struct Spec {
    tree: u8,
    after_differs: bool,
    exit: u8, // 0 = exit 0, 1 = exit 1, 2 = killed by signal, 3 = exit 0 but a signal is recorded
    cmd: u8,  // 0 = pytest -q, 1 = echo ok
    assurance: u8,
    excluded: bool,
    prev: Option<usize>,
}

fn spec() -> impl Strategy<Value = Spec> {
    (
        0u8..4,
        any::<bool>(),
        0u8..4,
        0u8..2,
        0u8..4,
        any::<bool>(),
        proptest::option::of(0usize..8),
    )
        .prop_map(
            |(tree, after_differs, exit, cmd, assurance, excluded, prev)| Spec {
                tree,
                after_differs,
                exit,
                cmd,
                assurance,
                excluded,
                prev,
            },
        )
}

fn tree_name(n: u8) -> String {
    format!("tree{n}")
}

fn body(s: &Spec, prev: Option<String>) -> RecordBody {
    let after = if s.after_differs {
        (s.tree + 1) % 4
    } else {
        s.tree
    };
    RecordBody {
        record_version: RECORD_VERSION,
        prev,
        subject: Subject {
            worktree_tree: tree_name(s.tree),
            tree_after: tree_name(after),
            head_commit: None,
            dirty: true,
            exclude: if s.excluded {
                vec![".x".into()]
            } else {
                vec![]
            },
        },
        command: Command {
            argv: if s.cmd == 0 {
                vec!["pytest".into(), "-q".into()]
            } else {
                vec!["echo".into(), "ok".into()]
            },
            env_names: vec![],
            cwd: ".".into(),
        },
        execution: Execution {
            exit_code: match s.exit {
                0 | 3 => Some(0),
                1 => Some(1),
                _ => None,
            },
            signal: if s.exit >= 2 { Some(9) } else { None },
            started_at: "2026-01-01T00:00:00Z".into(),
            duration_ms: 1,
        },
        observation: Observation {
            method: "none".into(),
        },
        recorder: Recorder {
            name: "opshero".into(),
            version: "0.0.1".into(),
            assurance: format!("A{}", s.assurance),
            adapter: None,
        },
    }
}

fn build(specs: &[Spec]) -> Vec<Record> {
    let mut out: Vec<Record> = Vec::new();
    for (i, s) in specs.iter().enumerate() {
        let prev = s.prev.filter(|p| *p < i).map(|p| out[p].id.clone());
        out.push(new_record(body(s, prev)).unwrap());
    }
    out
}

fn records() -> impl Strategy<Value = Vec<Record>> {
    let dense = proptest::collection::vec(dense_spec(), 0..8).prop_map(|s| build(&s));
    let wide = proptest::collection::vec(spec(), 0..8).prop_map(|s| build(&s));
    // Half the cases concentrate on one tree and command so that several runs
    // compete on the same subject, which is where the repeat rule matters.
    prop_oneof![wide, dense]
}

/// Evidence concentrated on subject tree1 / `pytest`: mostly passes, mixed assurance.
fn dense_spec() -> impl Strategy<Value = Spec> {
    (0u8..8, 0u8..4, proptest::option::of(0usize..8)).prop_map(|(e, assurance, prev)| Spec {
        tree: 1,
        after_differs: false,
        exit: if e == 0 { 1 } else { 0 },
        cmd: 0,
        assurance,
        excluded: false,
        prev,
    })
}

#[derive(Clone, Debug)]
struct ReqSpec {
    cmd: u8,
    min: u8,
    block: bool,
    /// 0 = no repeat rule, 1 = N = 2 passes required, 2 = N = 3 passes required.
    repeat: u8,
    /// true = the default (`deny`): a run during which the tree changed is not evidence;
    /// false = the explicit opt-out `post_run_changes = "allow"`.
    deny: bool,
}

fn policy_of(reqs: &[ReqSpec]) -> Policy {
    let mut t = String::from("version=1\nid=\"p\"\n");
    for (i, r) in reqs.iter().enumerate() {
        let command = if r.cmd == 0 {
            "[\"pytest\",\"*\"]"
        } else {
            "[\"echo\"]"
        };
        let repeat = match r.repeat {
            0 => "",
            1 => "repeat = { alpha = \"25%\", flake_rate = \"50%\" }\n",
            _ => "repeat = { alpha = \"12.5%\", flake_rate = \"50%\" }\n",
        };
        // `deny` is the policy default; `deny: false` writes the explicit opt-out.
        let deny = if r.deny {
            ""
        } else {
            "post_run_changes = \"allow\"\n"
        };
        t.push_str(&format!(
            "[[require]]\nid=\"r{i}\"\ncheck=\"evidence.fresh\"\ncommand={command}\nmin_assurance=\"A{}\"\nseverity=\"{}\"\n{repeat}{deny}",
            r.min,
            if r.block { "block" } else { "warn" }
        ));
    }
    policy::parse(&t).unwrap()
}

fn policies() -> impl Strategy<Value = Vec<ReqSpec>> {
    proptest::collection::vec(
        (0u8..2, 0u8..4, any::<bool>(), 0u8..3, any::<bool>()).prop_map(
            |(cmd, min, block, repeat, deny)| ReqSpec {
                cmd,
                min,
                block,
                repeat,
                deny,
            },
        ),
        1..4,
    )
}

/// Policies without any `repeat` rule (deterministic-command semantics).
fn plain_policies() -> impl Strategy<Value = Vec<ReqSpec>> {
    policies().prop_map(|v| v.into_iter().map(|r| ReqSpec { repeat: 0, ..r }).collect())
}

fn subject(tree: u8, excluded: bool) -> SubjectView {
    SubjectView {
        worktree_tree: tree_name(tree),
        head_commit: None,
        dirty: true,
        exclude: if excluded { vec![".x".into()] } else { vec![] },
    }
}

fn json(r: &Report) -> String {
    serde_json::to_string(r).unwrap()
}

fn rank(v: Verdict) -> u8 {
    match v {
        Verdict::Pass => 0,
        Verdict::Unknown => 1,
        Verdict::Fail => 2,
    }
}

fn cases() -> ProptestConfig {
    ProptestConfig {
        cases: 2000,
        ..ProptestConfig::default()
    }
}

// ---------------------------------------------------------------------- laws

proptest! {
    #![proptest_config(cases())]

    /// L1. The report does not depend on the order evidence is supplied in.
    #[test]
    fn l1_order_independence(
        recs in records(),
        reqs in policies(),
        t in 0u8..5,
        x in any::<bool>(),
        shuffled in any::<proptest::sample::Index>(),
    ) {
        let p = policy_of(&reqs);
        let s = subject(t, x);
        let base = json(&evaluate(&p, &s, &recs, vec![]));
        let mut rotated = recs.clone();
        if !rotated.is_empty() {
            let k = shuffled.index(rotated.len());
            rotated.rotate_left(k);
            rotated.reverse();
        }
        prop_assert_eq!(base, json(&evaluate(&p, &s, &rotated, vec![])));
    }

    /// L2. Soundness of PASS: every blocking check that passed names a real
    /// record that matches its command, passed, is bound to exactly the
    /// subject (same tree and same exclusions) and has enough assurance.
    #[test]
    fn l2_pass_has_a_witness(
        recs in records(),
        reqs in policies(),
        t in 0u8..5,
        x in any::<bool>(),
    ) {
        let p = policy_of(&reqs);
        let s = subject(t, x);
        let rep = evaluate(&p, &s, &recs, vec![]);
        for (req, chk) in p.require.iter().zip(&rep.checks) {
            if chk.result != Verdict::Pass { continue; }
            prop_assert!(!chk.evidence.is_empty());
            for id in &chk.evidence {
                let r = recs.iter().find(|r| &r.id == id).expect("evidence id exists");
                prop_assert!(policy::command_matches(&req.command, &r.body.command.argv));
                prop_assert_eq!(r.body.execution.exit_code, Some(0));
                prop_assert!(r.body.execution.signal.is_none());
                prop_assert_eq!(&r.body.subject.tree_after, &s.worktree_tree);
                prop_assert_eq!(&r.body.subject.exclude, &s.exclude);
                let a = policy::Assurance::parse(&r.body.recorder.assurance).unwrap();
                prop_assert!(a >= req.min_assurance);
            }
        }
    }

    /// L3. Fail closed: a subject no record is bound to is never PASS while
    /// at least one blocking requirement exists.
    #[test]
    fn l3_unseen_subject_never_passes(recs in records(), reqs in policies()) {
        let p = policy_of(&reqs);
        // tree "tree9" is never produced by the generator.
        let s = SubjectView {
            worktree_tree: "tree9".into(),
            head_commit: None,
            dirty: true,
            exclude: vec![],
        };
        let rep = evaluate(&p, &s, &recs, vec![]);
        if reqs.iter().any(|r| r.block) {
            prop_assert_ne!(rep.verdict, Verdict::Pass);
        }
    }

    /// L4. The overall verdict is the worst blocking check (FAIL > UNKNOWN > PASS);
    /// warn-severity checks never influence it.
    #[test]
    fn l4_verdict_is_worst_blocking_check(
        recs in records(),
        reqs in policies(),
        t in 0u8..5,
    ) {
        let p = policy_of(&reqs);
        let rep = evaluate(&p, &subject(t, false), &recs, vec![]);
        let worst = p.require.iter().zip(&rep.checks)
            .filter(|(r, _)| r.severity == policy::Severity::Block)
            .map(|(_, c)| rank(c.result))
            .max()
            .unwrap_or(0);
        prop_assert_eq!(rank(rep.verdict), worst);
    }

    /// L5. Adding evidence never turns PASS into something else (monotone PASS).
    /// Holds for requirements WITHOUT a `repeat` rule. It is the property that
    /// makes "a later failing rerun" invisible when an earlier run passed on the
    /// same tree (KI-1); `repeat` requirements deliberately break it (L10).
    #[test]
    fn l5_pass_is_monotone_under_added_evidence(
        recs in records(),
        extra in records(),
        reqs in plain_policies(),
        t in 0u8..5,
        x in any::<bool>(),
    ) {
        let p = policy_of(&reqs);
        let s = subject(t, x);
        let before = evaluate(&p, &s, &recs, vec![]).verdict;
        let mut all = recs.clone();
        all.extend(extra);
        let after = evaluate(&p, &s, &all, vec![]).verdict;
        if before == Verdict::Pass {
            prop_assert_eq!(after, Verdict::Pass);
        }
    }

    /// L6. Duplicating evidence changes nothing about the verdict (idempotent set semantics).
    #[test]
    fn l6_duplicates_do_not_change_the_verdict(
        recs in records(),
        reqs in policies(),
        t in 0u8..5,
    ) {
        let p = policy_of(&reqs);
        let s = subject(t, false);
        let once = evaluate(&p, &s, &recs, vec![]).verdict;
        let mut twice = recs.clone();
        twice.extend(recs.clone());
        prop_assert_eq!(once, evaluate(&p, &s, &twice, vec![]).verdict);
    }

    /// L7. The order of requirements in the policy does not change the overall verdict.
    #[test]
    fn l7_requirement_order_is_irrelevant(
        recs in records(),
        reqs in policies(),
        t in 0u8..5,
    ) {
        let s = subject(t, false);
        let mut rev = reqs.clone();
        rev.reverse();
        prop_assert_eq!(
            evaluate(&policy_of(&reqs), &s, &recs, vec![]).verdict,
            evaluate(&policy_of(&rev), &s, &recs, vec![]).verdict
        );
    }

    /// L8. Tamper evidence: changing any single field of a stored record is
    /// detected by the integrity check (the id is the hash of the content).
    #[test]
    fn l8_any_single_field_edit_is_detected(
        s in spec(),
        field in 0usize..6,
    ) {
        let r = new_record(body(&s, None)).unwrap();
        let mut raw = serde_json::to_value(&r).unwrap();
        match field {
            0 => raw["subject"]["tree_after"] = serde_json::json!("forged"),
            1 => raw["execution"]["exit_code"] = serde_json::json!(0),
            2 => raw["command"]["argv"] = serde_json::json!(["true"]),
            3 => raw["recorder"]["assurance"] = serde_json::json!("A3"),
            4 => raw["execution"]["duration_ms"] = serde_json::json!(999_999),
            _ => raw["prev"] = serde_json::json!("0".repeat(64)),
        }
        if raw == serde_json::to_value(&r).unwrap() {
            // The edit happened to write the value already present: not a tamper.
            return Ok(());
        }
        prop_assert!(parse_verified(&raw).is_err());
    }

    /// L9. Differential test: the per-check result equals an independent
    /// restatement of the specification (RFC-0005 section 3.1) written in this
    /// file from the definitions only, for any policy, subject and evidence.
    #[test]
    fn l9_matches_the_specification_oracle(
        recs in records(),
        reqs in policies(),
        t in 0u8..5,
        x in any::<bool>(),
    ) {
        let p = policy_of(&reqs);
        let s = subject(t, x);
        let rep = evaluate(&p, &s, &recs, vec![]);
        for ((spec, req), chk) in reqs.iter().zip(&p.require).zip(&rep.checks) {
            prop_assert_eq!(chk.result, oracle(spec, req, &s, &recs));
        }
    }

    /// L10. With a `repeat` rule, one failing run on the subject excludes PASS,
    /// whatever else was recorded; and adding such a failure to evidence that
    /// passed always changes the check away from PASS.
    #[test]
    fn l10_repeat_rule_is_antimonotone_in_failures(
        recs in records(),
        t in 0u8..4,
        which in 1u8..3,
    ) {
        let reqs = [ReqSpec { cmd: 0, min: 0, block: true, repeat: which, deny: false }];
        let p = policy_of(&reqs);
        let s = subject(t, false);
        let failing = Spec {
            tree: t, after_differs: false, exit: 1, cmd: 0,
            assurance: 3, excluded: false, prev: None,
        };
        let mut with_failure = recs.clone();
        with_failure.extend(build(&[failing]));
        let rep = evaluate(&p, &s, &with_failure, vec![]);
        prop_assert_ne!(rep.checks[0].result, Verdict::Pass);
    }

    /// L11. With a `repeat` rule, PASS needs at least N distinct passing
    /// witnesses and zero failing runs on the subject.
    #[test]
    fn l11_repeat_pass_needs_n_distinct_witnesses_and_no_failure(
        recs in records(),
        t in 0u8..5,
        which in 1u8..3,
    ) {
        let reqs = [ReqSpec { cmd: 0, min: 0, block: true, repeat: which, deny: false }];
        let p = policy_of(&reqs);
        let s = subject(t, false);
        let rep = evaluate(&p, &s, &recs, vec![]);
        if rep.checks[0].result == Verdict::Pass {
            let n = if which == 1 { 2 } else { 3 };
            let mut ids = rep.checks[0].evidence.clone();
            ids.sort();
            ids.dedup();
            prop_assert!(ids.len() >= n);
            let any_failure = recs.iter().any(|r| is_current(r, &s, &p.require[0]) && !passed_ok(r));
            prop_assert!(!any_failure);
        }
    }
}

proptest! {
    #![proptest_config(cases())]

    /// L12. With `post_run_changes = "deny"`, every witness of a PASS ran on
    /// content that did not change while it ran (tree before equals tree after).
    /// This is the property TLC proves for the "strict" rule in
    /// docs/formal/Freshness.tla (valid for one edit per run; see the ABA caveat).
    #[test]
    fn l12_deny_witnesses_are_stable_runs(
        recs in records(),
        t in 0u8..5,
        which in 0u8..3,
    ) {
        let reqs = [ReqSpec { cmd: 0, min: 0, block: true, repeat: which, deny: true }];
        let p = policy_of(&reqs);
        let rep = evaluate(&p, &subject(t, false), &recs, vec![]);
        if rep.checks[0].result == Verdict::Pass {
            for id in &rep.checks[0].evidence {
                let r = recs.iter().find(|r| &r.id == id).unwrap();
                prop_assert_eq!(&r.body.subject.worktree_tree, &r.body.subject.tree_after);
            }
        }
    }
}

#[test]
fn run_that_changed_the_tree_fails_by_default_and_passes_only_with_explicit_allow() {
    // tree before = tree1, tree after = tree2: the command ran while content changed.
    let spec = Spec {
        tree: 1,
        after_differs: true,
        exit: 0,
        cmd: 0,
        assurance: 1,
        excluded: false,
        prev: None,
    };
    let recs = build(&[spec]);
    let current = subject(2, false); // content now equals tree_after
    let allow = policy_of(&[ReqSpec {
        cmd: 0,
        min: 0,
        block: true,
        repeat: 0,
        deny: false,
    }]);
    let deny = policy_of(&[ReqSpec {
        cmd: 0,
        min: 0,
        block: true,
        repeat: 0,
        deny: true,
    }]);
    assert_eq!(
        evaluate(&allow, &current, &recs, vec![]).verdict,
        Verdict::Pass
    );
    let rep = evaluate(&deny, &current, &recs, vec![]);
    assert_eq!(rep.verdict, Verdict::Fail);
    assert!(
        rep.checks[0]
            .reason
            .contains("working tree changed while it ran"),
        "{}",
        rep.checks[0].reason
    );
    assert!(
        rep.checks[0].reason.contains("[subject] exclude"),
        "{}",
        rep.checks[0].reason
    );
}

// ------------------------------------------------ independent specification

fn passed_ok(r: &Record) -> bool {
    r.body.execution.exit_code == Some(0) && r.body.execution.signal.is_none()
}

fn is_current(r: &Record, s: &SubjectView, req: &policy::Require) -> bool {
    policy::command_matches(&req.command, &r.body.command.argv)
        && r.body.subject.tree_after == s.worktree_tree
        && r.body.subject.exclude == s.exclude
        && (req.post_run_changes == policy::PostRunChanges::Allow
            || r.body.subject.worktree_tree == r.body.subject.tree_after)
}

/// RFC-0005 section 3.1, restated from the definitions (not from the code).
fn oracle(spec: &ReqSpec, req: &policy::Require, s: &SubjectView, recs: &[Record]) -> Verdict {
    use std::collections::BTreeSet;
    let matching = recs
        .iter()
        .filter(|r| policy::command_matches(&req.command, &r.body.command.argv))
        .count();
    let current: BTreeSet<&str> = recs
        .iter()
        .filter(|r| is_current(r, s, req))
        .map(|r| r.id.as_str())
        .collect();
    let by_id = |id: &&str| recs.iter().find(|r| r.id == **id).unwrap();
    let passing: BTreeSet<&str> = current
        .iter()
        .copied()
        .filter(|i| passed_ok(by_id(i)))
        .collect();
    let failing = current.len() - passing.len();
    let enough: BTreeSet<&str> = passing
        .iter()
        .copied()
        .filter(|i| {
            policy::Assurance::parse(&by_id(i).body.recorder.assurance).unwrap()
                >= req.min_assurance
        })
        .collect();
    let needed = match spec.repeat {
        1 => Some(2),
        2 => Some(3),
        _ => None,
    };
    if let Some(n) = needed
        && !passing.is_empty()
    {
        if failing > 0 {
            return Verdict::Fail;
        }
        return if enough.len() >= n {
            Verdict::Pass
        } else {
            Verdict::Unknown
        };
    }
    if !enough.is_empty() {
        Verdict::Pass
    } else if !passing.is_empty() {
        Verdict::Unknown
    } else if !current.is_empty() {
        Verdict::Fail
    } else if matching == 0 {
        Verdict::Unknown
    } else {
        Verdict::Fail
    }
}

// -------------------------------------------------- characterization (KI-1)

/// KI-1 (RFC-0005): WITHOUT a `repeat` rule a pass followed by a failure on the
/// same tree is still PASS ("any pass wins"), which is right for deterministic
/// commands. The default is unchanged in policy v0; the next test shows the
/// opt-in rule that closes the issue.
#[test]
fn ki1_pass_then_fail_on_the_same_tree_is_pass_today() {
    let p = policy_of(&[ReqSpec {
        cmd: 0,
        min: 0,
        block: true,
        repeat: 0,
        deny: false,
    }]);
    let pass = Spec {
        tree: 1,
        after_differs: false,
        exit: 0,
        cmd: 0,
        assurance: 1,
        excluded: false,
        prev: None,
    };
    let fail = Spec {
        exit: 1,
        prev: Some(0),
        ..pass.clone()
    };
    let recs = build(&[pass, fail]);
    let rep = evaluate(&p, &subject(1, false), &recs, vec![]);
    assert_eq!(rep.verdict, Verdict::Pass);
}

#[test]
fn ki1_closed_by_the_repeat_rule_pass_then_fail_is_inconsistent() {
    let p = policy_of(&[ReqSpec {
        cmd: 0,
        min: 0,
        block: true,
        repeat: 1,
        deny: false,
    }]);
    let pass = Spec {
        tree: 1,
        after_differs: false,
        exit: 0,
        cmd: 0,
        assurance: 1,
        excluded: false,
        prev: None,
    };
    let fail = Spec {
        exit: 1,
        prev: Some(0),
        ..pass.clone()
    };
    let recs = build(&[pass, fail]);
    let rep = evaluate(&p, &subject(1, false), &recs, vec![]);
    assert_eq!(rep.verdict, Verdict::Fail);
    assert!(
        rep.checks[0].reason.contains("inconsistent"),
        "{}",
        rep.checks[0].reason
    );
}

#[test]
fn repeat_rule_needs_n_passes_before_it_passes() {
    let p = policy_of(&[ReqSpec {
        cmd: 0,
        min: 0,
        block: true,
        repeat: 1,
        deny: false,
    }]); // N = 2
    let one = Spec {
        tree: 1,
        after_differs: false,
        exit: 0,
        cmd: 0,
        assurance: 1,
        excluded: false,
        prev: None,
    };
    let two = Spec {
        prev: Some(0),
        assurance: 2,
        ..one.clone()
    };
    let s = subject(1, false);
    assert_eq!(
        evaluate(&p, &s, &build(std::slice::from_ref(&one)), vec![]).verdict,
        Verdict::Unknown
    );
    assert_eq!(
        evaluate(&p, &s, &build(&[one, two]), vec![]).verdict,
        Verdict::Pass
    );
}

#[test]
fn repeat_rule_counts_only_runs_with_enough_assurance() {
    // N = 3, required assurance A2. Three passes, but only two at A2 or above.
    let p = policy_of(&[ReqSpec {
        cmd: 0,
        min: 2,
        block: true,
        repeat: 2,
        deny: false,
    }]);
    let mk = |assurance, prev| Spec {
        tree: 1,
        after_differs: false,
        exit: 0,
        cmd: 0,
        assurance,
        excluded: false,
        prev,
    };
    let recs = build(&[mk(2, None), mk(2, Some(0)), mk(1, Some(1))]);
    let rep = evaluate(&p, &subject(1, false), &recs, vec![]);
    assert_eq!(
        rep.checks[0].result,
        Verdict::Unknown,
        "{}",
        rep.checks[0].reason
    );
}

#[test]
fn repeat_rule_counts_a_duplicated_record_once() {
    // N = 2: one passing run supplied twice is still one run.
    let p = policy_of(&[ReqSpec {
        cmd: 0,
        min: 0,
        block: true,
        repeat: 1,
        deny: false,
    }]);
    let one = Spec {
        tree: 1,
        after_differs: false,
        exit: 0,
        cmd: 0,
        assurance: 1,
        excluded: false,
        prev: None,
    };
    let recs = build(&[one]);
    let doubled = [recs[0].clone(), recs[0].clone()];
    let rep = evaluate(&p, &subject(1, false), &doubled, vec![]);
    assert_eq!(
        rep.checks[0].result,
        Verdict::Unknown,
        "{}",
        rep.checks[0].reason
    );
}
