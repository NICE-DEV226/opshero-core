//! Opt-in local journal for the dogfooding experiment (docs/validation-plan.md, H1).
//!
//! The question it answers: *in real agent sessions, how often does the Stop gate
//! catch missing, stale or failed evidence that a human agrees was a real problem?*
//!
//! Pure: parsing, summarising and the decision rule. Reading and writing the file
//! is the CLI's job. What is stored (and what is not) is deliberate: verdicts, reason
//! categories and a **hash** of the session id. Never commands, paths, tree ids,
//! file contents or the raw session id, so the journal can be shared or deleted
//! without a privacy review.

use crate::canon::canonicalize;
use crate::verify::Report;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const JOURNAL_VERSION: u64 = 1;

/// Pre-registered thresholds (docs/validation-plan.md, docs/roadmap.md M3).
pub const MIN_SESSIONS: u64 = 100;
/// At least this share of sessions must contain a real finding.
pub const SUCCESS_PPM: u64 = 100_000; // 10 %
/// Below this share (with enough sessions) the direction is reassessed.
pub const KILL_PPM: u64 = 30_000; // 3 %

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Block,
    Allow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Label {
    Real,
    Noise,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The Stop gate evaluated the policy.
    Stop {
        session: String,
        decision: Decision,
        verdict: String,
        reasons: Vec<String>,
        stop_active: bool,
    },
    /// A command was recorded as evidence.
    Record { session: String, passed: bool },
    /// A human judged an earlier blocking Stop (by its line number in the journal).
    Label { target: usize, label: Label },
}

/// Stable, non-reversible session key: first 12 hex digits of SHA-256.
pub fn session_key(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// Coarse reason categories for a blocking evaluation. Categories only: the
/// check ids and command patterns from the policy are not recorded.
pub fn categorize(report: &Report) -> Vec<String> {
    let mut out = BTreeSet::new();
    for c in &report.checks {
        let r = c.reason.as_str();
        let cat = if r.starts_with("no recorded run") {
            "missing"
        } else if r.starts_with("stale evidence") {
            "stale"
        } else if r.contains("inconsistent results") {
            "inconsistent"
        } else if r.contains("working tree changed while it ran") {
            "tree-changed-during-run"
        } else if r.contains("below the required") {
            "assurance"
        } else if r.contains("runs required by the repeat rule") {
            "insufficient-runs"
        } else if r.contains("failed on the current tree") || r.contains("none passing") {
            "failed"
        } else {
            continue; // a passing check is not a reason
        };
        out.insert(cat.to_owned());
    }
    out.into_iter().collect()
}

impl Event {
    /// One canonical JSON line (no trailing newline).
    pub fn to_line(&self, unix_secs: u64) -> String {
        let v = match self {
            Self::Stop {
                session,
                decision,
                verdict,
                reasons,
                stop_active,
            } => json!({
                "v": JOURNAL_VERSION, "t": unix_secs, "kind": "stop", "session": session,
                "decision": match decision { Decision::Block => "block", Decision::Allow => "allow" },
                "verdict": verdict, "reasons": reasons, "stop_active": stop_active
            }),
            Self::Record { session, passed } => json!({
                "v": JOURNAL_VERSION, "t": unix_secs, "kind": "record",
                "session": session, "passed": passed
            }),
            Self::Label { target, label } => json!({
                "v": JOURNAL_VERSION, "t": unix_secs, "kind": "label",
                "target": target,
                "label": match label { Label::Real => "real", Label::Noise => "noise" }
            }),
        };
        canonicalize(&v).unwrap_or_default()
    }

    /// Tolerant reader: `None` for anything that is not a version-1 event.
    pub fn parse_line(line: &str) -> Option<Self> {
        let v: Value = serde_json::from_str(line).ok()?;
        if v.get("v")?.as_u64()? != JOURNAL_VERSION {
            return None;
        }
        let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
        match v.get("kind")?.as_str()? {
            "stop" => Some(Self::Stop {
                session: s("session")?,
                decision: match v.get("decision")?.as_str()? {
                    "block" => Decision::Block,
                    "allow" => Decision::Allow,
                    _ => return None,
                },
                verdict: s("verdict")?,
                reasons: v
                    .get("reasons")?
                    .as_array()?
                    .iter()
                    .filter_map(|r| r.as_str().map(str::to_owned))
                    .collect(),
                stop_active: v.get("stop_active")?.as_bool()?,
            }),
            "record" => Some(Self::Record {
                session: s("session")?,
                passed: v.get("passed")?.as_bool()?,
            }),
            "label" => Some(Self::Label {
                target: usize::try_from(v.get("target")?.as_u64()?).ok()?,
                label: match v.get("label")?.as_str()? {
                    "real" => Label::Real,
                    "noise" => Label::Noise,
                    _ => return None,
                },
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub sessions: u64,
    /// Sessions in which the gate had something to judge: at least one Stop whose
    /// verdict is not `UNCHANGED` or `NOT-APPLICABLE`. Reported next to the rate,
    /// but the pre-registered rule keeps using `sessions` (no post-hoc denominator).
    pub sessions_gated: u64,
    pub stops: u64,
    pub blocks: u64,
    pub blocks_real: u64,
    pub blocks_noise: u64,
    pub blocks_unlabeled: u64,
    pub sessions_with_real_finding: u64,
    pub records: u64,
    pub records_failed: u64,
    /// Labels that point at a line that is not a blocking Stop.
    pub labels_ignored: u64,
    /// Block counts per reason category.
    pub by_reason: BTreeMap<String, u64>,
}

/// Fold events (each with its 1-based line number in the journal) into counts.
/// Pure and independent of the order of `events`: a label points at a line
/// number, and when several labels point at the same block the highest line
/// number (the latest) wins.
pub fn summarize(events: &[(usize, Event)]) -> Summary {
    let mut sum = Summary::default();
    let mut sessions: BTreeSet<&str> = BTreeSet::new();
    let mut gated: BTreeSet<&str> = BTreeSet::new();
    let mut blocks: BTreeMap<usize, &str> = BTreeMap::new(); // line -> session
    let mut labels: BTreeMap<usize, (usize, Label)> = BTreeMap::new(); // target -> (label line, label)
    for (line, ev) in events {
        match ev {
            Event::Stop {
                session,
                decision,
                reasons,
                verdict,
                ..
            } => {
                sessions.insert(session);
                if verdict != "UNCHANGED" && verdict != "NOT-APPLICABLE" {
                    gated.insert(session);
                }
                sum.stops += 1;
                if *decision == Decision::Block {
                    sum.blocks += 1;
                    blocks.insert(*line, session);
                    for r in reasons {
                        *sum.by_reason.entry(r.clone()).or_insert(0) += 1;
                    }
                }
            }
            Event::Record { session, passed } => {
                sessions.insert(session);
                sum.records += 1;
                if !passed {
                    sum.records_failed += 1;
                }
            }
            Event::Label { target, label } => {
                let slot = labels.entry(*target).or_insert((*line, *label));
                if *line >= slot.0 {
                    *slot = (*line, *label);
                }
            }
        }
    }
    sum.sessions = sessions.len() as u64;
    sum.sessions_gated = gated.len() as u64;
    let mut found: BTreeSet<&str> = BTreeSet::new();
    for (target, (_, label)) in &labels {
        match blocks.get(target) {
            None => sum.labels_ignored += 1,
            Some(session) => match label {
                Label::Real => {
                    sum.blocks_real += 1;
                    found.insert(session);
                }
                Label::Noise => sum.blocks_noise += 1,
            },
        }
    }
    sum.sessions_with_real_finding = found.len() as u64;
    sum.blocks_unlabeled = sum.blocks - sum.blocks_real - sum.blocks_noise;
    sum
}

/// Wilson score interval, 95 %, for `successes` out of `n`. For reporting only:
/// it never influences a verdict, so floating point is acceptable here.
pub fn wilson(successes: u64, n: u64) -> Option<(f64, f64)> {
    if n == 0 || successes > n {
        return None;
    }
    let z = 1.959_963_984_540_054_f64;
    let (s, nf) = (successes as f64, n as f64);
    let p = s / nf;
    let denom = 1.0 + z * z / nf;
    let centre = (p + z * z / (2.0 * nf)) / denom;
    let half = z * (p * (1.0 - p) / nf + z * z / (4.0 * nf * nf)).sqrt() / denom;
    // At the extremes the bound is exactly 0 or 1; do not let rounding noise show.
    let lo = if successes == 0 {
        0.0
    } else {
        (centre - half).max(0.0)
    };
    let hi = if successes == n {
        1.0
    } else {
        (centre + half).min(1.0)
    };
    Some((lo, hi))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Fewer sessions than the pre-registered minimum: no conclusion.
    NotEnoughData { sessions: u64 },
    /// Point estimate at or above the success threshold.
    Meets,
    /// Between the kill and success thresholds.
    Inconclusive,
    /// Point estimate below the kill threshold with enough sessions.
    KillCriterion,
}

/// The pre-registered decision rule, on the point estimate; the interval is
/// reported next to it so the reader can judge the uncertainty.
pub fn outcome(sum: &Summary) -> Outcome {
    if sum.sessions < MIN_SESSIONS {
        return Outcome::NotEnoughData {
            sessions: sum.sessions,
        };
    }
    // Compare in integers: found / sessions >= ppm / 1e6.
    let found = sum.sessions_with_real_finding * 1_000_000;
    if found >= SUCCESS_PPM * sum.sessions {
        Outcome::Meets
    } else if found < KILL_PPM * sum.sessions {
        Outcome::KillCriterion
    } else {
        Outcome::Inconclusive
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn stop(session: &str, block: bool) -> Event {
        Event::Stop {
            session: session.into(),
            decision: if block {
                Decision::Block
            } else {
                Decision::Allow
            },
            verdict: if block { "UNKNOWN" } else { "PASS" }.into(),
            reasons: if block {
                vec!["missing".into()]
            } else {
                vec![]
            },
            stop_active: false,
        }
    }

    #[test]
    fn lines_round_trip_and_contain_no_raw_identifiers() {
        let evs = [
            stop("a1b2c3", true),
            Event::Record {
                session: "a1b2c3".into(),
                passed: false,
            },
            Event::Label {
                target: 7,
                label: Label::Real,
            },
        ];
        for e in &evs {
            let line = e.to_line(1_700_000_000);
            assert_eq!(Event::parse_line(&line).as_ref(), Some(e), "{line}");
            assert!(!line.contains('\n'));
        }
        let k = session_key("550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(k.len(), 12);
        assert!(!k.contains("550e8400"));
        assert_eq!(k, session_key("550e8400-e29b-41d4-a716-446655440000"));
        assert_ne!(k, session_key("550e8400-e29b-41d4-a716-446655440001"));
    }

    #[test]
    fn parser_ignores_foreign_and_malformed_lines() {
        for bad in [
            "",
            "garbage",
            "{}",
            r#"{"v":2,"kind":"stop"}"#,
            r#"{"v":1,"kind":"future"}"#,
            r#"{"v":1,"kind":"stop","session":"s","decision":"maybe","verdict":"X","reasons":[],"stop_active":false}"#,
            r#"{"v":1,"kind":"label","target":-1,"label":"real"}"#,
        ] {
            assert_eq!(Event::parse_line(bad), None, "{bad}");
        }
    }

    #[test]
    fn summary_of_a_small_journal() {
        let ev = vec![
            (1, stop("s1", true)),
            (2, stop("s1", false)),
            (3, stop("s2", true)),
            (4, stop("s3", false)),
            (
                5,
                Event::Record {
                    session: "s3".into(),
                    passed: true,
                },
            ),
            (
                6,
                Event::Label {
                    target: 1,
                    label: Label::Real,
                },
            ),
            (
                7,
                Event::Label {
                    target: 3,
                    label: Label::Noise,
                },
            ),
            (
                8,
                Event::Label {
                    target: 4,
                    label: Label::Real,
                },
            ), // not a block: ignored
            (
                9,
                Event::Label {
                    target: 99,
                    label: Label::Real,
                },
            ), // nothing there: ignored
        ];
        let s = summarize(&ev);
        assert_eq!((s.sessions, s.stops, s.blocks), (3, 4, 2));
        assert_eq!(
            (s.blocks_real, s.blocks_noise, s.blocks_unlabeled),
            (1, 1, 0)
        );
        assert_eq!(s.sessions_with_real_finding, 1);
        assert_eq!(s.labels_ignored, 2);
        assert_eq!((s.records, s.records_failed), (1, 0));
        assert_eq!(s.by_reason.get("missing"), Some(&2));
    }

    #[test]
    fn the_latest_label_on_a_block_wins() {
        let ev = vec![
            (1, stop("s", true)),
            (
                2,
                Event::Label {
                    target: 1,
                    label: Label::Real,
                },
            ),
            (
                3,
                Event::Label {
                    target: 1,
                    label: Label::Noise,
                },
            ),
        ];
        let s = summarize(&ev);
        assert_eq!((s.blocks_real, s.blocks_noise), (0, 1));
        assert_eq!(s.sessions_with_real_finding, 0);
        let mut reversed = ev.clone();
        reversed.reverse();
        assert_eq!(
            summarize(&reversed),
            s,
            "order of the slice must not matter"
        );
    }

    fn arb_events() -> impl Strategy<Value = Vec<(usize, Event)>> {
        let ev = prop_oneof![
            (0u8..4, any::<bool>()).prop_map(|(s, b)| stop(&format!("s{s}"), b)),
            (0u8..4, any::<bool>()).prop_map(|(s, p)| Event::Record {
                session: format!("s{s}"),
                passed: p
            }),
            (1usize..12, any::<bool>()).prop_map(|(t, r)| Event::Label {
                target: t,
                label: if r { Label::Real } else { Label::Noise }
            }),
        ];
        proptest::collection::vec(ev, 0..12)
            .prop_map(|v| v.into_iter().enumerate().map(|(i, e)| (i + 1, e)).collect())
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

        #[test]
        fn summary_invariants(ev in arb_events()) {
            let s = summarize(&ev);
            prop_assert_eq!(s.blocks, s.blocks_real + s.blocks_noise + s.blocks_unlabeled);
            prop_assert!(s.sessions_with_real_finding <= s.sessions);
            prop_assert!(s.sessions_with_real_finding <= s.blocks_real);
            prop_assert!(s.blocks <= s.stops);
            prop_assert!(s.records_failed <= s.records);
            prop_assert_eq!(s.by_reason.values().sum::<u64>(), s.blocks);
        }

        #[test]
        fn summary_is_independent_of_slice_order(ev in arb_events(), k in 0usize..12) {
            let mut rotated = ev.clone();
            if !rotated.is_empty() { let k = k % rotated.len(); rotated.rotate_left(k); rotated.reverse(); }
            prop_assert_eq!(summarize(&ev), summarize(&rotated));
        }

        #[test]
        fn wilson_contains_the_estimate_and_stays_in_range(n in 1u64..5000, s_raw in 0u64..5000) {
            let s = s_raw % (n + 1);
            let (lo, hi) = wilson(s, n).unwrap();
            let p = s as f64 / n as f64;
            prop_assert!(0.0 <= lo && lo <= p + 1e-12 && p <= hi + 1e-12 && hi <= 1.0);
        }

        #[test]
        fn wilson_narrows_with_more_data(s in 0u64..50, extra in 1u64..40) {
            // Same proportion, ten times the data: the interval must be narrower.
            let n = 50 + extra;
            let s = s % (n + 1);
            let (l1, h1) = wilson(s, n).unwrap();
            let (l2, h2) = wilson(s * 10, n * 10).unwrap();
            prop_assert!(h2 - l2 < h1 - l1);
        }
    }

    #[test]
    fn wilson_known_values() {
        // Reference values (Wilson 95 %): 10/100 -> [0.0552, 0.1744]; 0/100 -> [0, 0.0370].
        let (lo, hi) = wilson(10, 100).unwrap();
        assert!(
            (lo - 0.0552).abs() < 5e-4 && (hi - 0.1744).abs() < 5e-4,
            "{lo} {hi}"
        );
        let (lo, hi) = wilson(0, 100).unwrap();
        assert!(lo == 0.0 && (hi - 0.0370).abs() < 5e-4, "{lo} {hi}");
        let (lo, hi) = wilson(100, 100).unwrap();
        assert!((lo - 0.9630).abs() < 5e-4 && hi == 1.0, "{lo} {hi}");
        assert_eq!(wilson(0, 0), None);
        assert_eq!(wilson(3, 2), None);
    }

    fn with(sessions: u64, found: u64) -> Summary {
        Summary {
            sessions,
            sessions_with_real_finding: found,
            ..Summary::default()
        }
    }

    #[test]
    fn decision_rule_follows_the_pre_registered_thresholds() {
        assert_eq!(
            outcome(&with(99, 99)),
            Outcome::NotEnoughData { sessions: 99 }
        );
        assert_eq!(outcome(&with(100, 10)), Outcome::Meets); // exactly 10 %
        assert_eq!(outcome(&with(100, 9)), Outcome::Inconclusive);
        assert_eq!(outcome(&with(100, 3)), Outcome::Inconclusive); // exactly 3 % is not below
        assert_eq!(outcome(&with(100, 2)), Outcome::KillCriterion);
        assert_eq!(outcome(&with(1000, 100)), Outcome::Meets);
        assert_eq!(outcome(&with(1000, 29)), Outcome::KillCriterion);
    }

    #[test]
    fn sessions_where_nothing_needed_judging_are_counted_apart() {
        let mk = |session: &str, verdict: &str| Event::Stop {
            session: session.into(),
            decision: Decision::Allow,
            verdict: verdict.into(),
            reasons: vec![],
            stop_active: false,
        };
        let ev = vec![
            (1, mk("a", "UNCHANGED")),
            (2, mk("b", "NOT-APPLICABLE")),
            (3, mk("c", "PASS")),
            (4, mk("c", "UNCHANGED")), // one judged stop is enough to count the session
            (5, mk("d", "UNKNOWN")),
        ];
        let s = summarize(&ev);
        assert_eq!((s.sessions, s.sessions_gated), (4, 2));
    }
}
