//! The journal stores reason *categories*, derived from real evaluation reports.
//! These tests build real reports with `evaluate` so the category mapping cannot
//! drift from the wording of the reasons.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use opshero_core::journal::categorize;
use opshero_core::policy;
use opshero_core::record::{
    Command, Execution, Observation, RECORD_VERSION, Record, RecordBody, Recorder, Subject,
    new_record,
};
use opshero_core::verify::{SubjectView, evaluate};

fn rec(before: &str, after: &str, exit: i64, assurance: &str, prev: Option<&str>) -> Record {
    new_record(RecordBody {
        record_version: RECORD_VERSION,
        prev: prev.map(str::to_owned),
        subject: Subject {
            worktree_tree: before.into(),
            tree_after: after.into(),
            head_commit: None,
            dirty: true,
            exclude: vec![],
        },
        command: Command {
            argv: vec!["pytest".into()],
            env_names: vec![],
            cwd: ".".into(),
        },
        execution: Execution {
            exit_code: Some(exit),
            signal: None,
            started_at: "2026-01-01T00:00:00Z".into(),
            duration_ms: 1,
        },
        observation: Observation {
            method: "none".into(),
        },
        recorder: Recorder {
            name: "opshero".into(),
            version: "0".into(),
            assurance: assurance.into(),
            adapter: None,
        },
    })
    .unwrap()
}

fn cats(extra_policy: &str, records: &[Record]) -> Vec<String> {
    let p = policy::parse(&format!(
        "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"pytest\"]\n{extra_policy}"
    ))
    .unwrap();
    let subject = SubjectView {
        worktree_tree: "now".into(),
        head_commit: None,
        dirty: true,
        exclude: vec![],
    };
    categorize(&evaluate(&p, &subject, records, vec![]))
}

#[test]
fn each_blocking_reason_has_its_category() {
    assert_eq!(cats("", &[]), ["missing"]);
    assert_eq!(cats("", &[rec("old", "old", 0, "A1", None)]), ["stale"]);
    assert_eq!(cats("", &[rec("now", "now", 1, "A1", None)]), ["failed"]);
    assert_eq!(
        cats(
            "min_assurance=\"A2\"\n",
            &[rec("now", "now", 0, "A1", None)]
        ),
        ["assurance"]
    );
    assert_eq!(
        cats(
            "post_run_changes=\"deny\"\n",
            &[rec("old", "now", 0, "A1", None)]
        ),
        ["tree-changed-during-run"]
    );
    let a = rec("now", "now", 0, "A1", None);
    let b = rec("now", "now", 1, "A1", Some(&a.id));
    assert_eq!(
        cats(
            "repeat={alpha=\"25%\",flake_rate=\"50%\"}\n",
            &[a.clone(), b]
        ),
        ["inconsistent"]
    );
    assert_eq!(
        cats("repeat={alpha=\"25%\",flake_rate=\"50%\"}\n", &[a]),
        ["insufficient-runs"]
    );
}

#[test]
fn a_passing_evaluation_has_no_reasons() {
    assert!(cats("", &[rec("now", "now", 0, "A1", None)]).is_empty());
}

mod stop_message {
    use super::*;
    use opshero_core::render::stop_reason;

    fn message(records: &[Record]) -> Option<String> {
        let p = policy::parse(
            "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"pytest\"]\n",
        )
        .unwrap();
        let subject = SubjectView {
            worktree_tree: "now".into(),
            head_commit: None,
            dirty: true,
            exclude: vec![],
        };
        stop_reason(&evaluate(&p, &subject, records, vec![]))
    }

    #[test]
    fn missing_evidence_tells_the_agent_to_run_the_command_on_its_own() {
        let m = message(&[]).unwrap();
        assert!(m.contains("no pipe"), "{m}");
        assert!(m.contains("no recorded run"), "{m}");
    }

    #[test]
    fn stale_evidence_does_not_get_the_pipe_hint() {
        let m = message(&[rec("old", "old", 0, "A1", None)]).unwrap();
        assert!(!m.contains("no pipe"), "{m}");
        assert!(m.contains("stale evidence"), "{m}");
    }

    #[test]
    fn nothing_blocks_when_the_evidence_is_fresh() {
        assert_eq!(message(&[rec("now", "now", 0, "A1", None)]), None);
    }
}
