//! Output formats. Text is for humans; JSON is canonical (RFC 8785) so the
//! same report is byte-identical across runs and machines.

use crate::canon::{CanonError, canonicalize};
use crate::verify::{Report, Verdict};

pub fn json(report: &Report) -> Result<String, CanonError> {
    let value = serde_json::to_value(report).map_err(|_| CanonError::NonIntegerNumber)?;
    let mut text = canonicalize(&value)?;
    text.push('\n');
    Ok(text)
}

pub fn text(report: &Report) -> String {
    let mut out = String::new();
    let tree = report
        .subject
        .tree
        .get(..12)
        .unwrap_or(&report.subject.tree);
    let head = report
        .subject
        .head_commit
        .as_deref()
        .map(|h| h.get(..9).unwrap_or(h))
        .unwrap_or("none");
    out.push_str(&format!(
        "subject: tree {tree}  (head {head}, {})\n",
        if report.subject.dirty {
            "dirty"
        } else {
            "clean"
        }
    ));
    out.push_str(&format!("policy:  {}\n", report.policy));
    for c in &report.checks {
        let tag = match c.result {
            Verdict::Pass => "PASS   ",
            Verdict::Fail => "FAIL   ",
            Verdict::Unknown => "UNKNOWN",
        };
        let sev = match c.severity {
            crate::policy::Severity::Block => "",
            crate::policy::Severity::Warn => " (warn)",
        };
        out.push_str(&format!("  {tag} {}{sev}: {}\n", c.id, c.reason));
    }
    for f in &report.findings {
        out.push_str(&format!("  note: {f}\n"));
    }
    out.push_str(&format!(
        "verdict: {}   assurance: {}\n",
        report.verdict.as_str(),
        report.assurance.as_deref().unwrap_or("n/a")
    ));
    out
}

/// Deterministic message for an agent whose evidence is missing or stale.
/// `None` when nothing blocks. It is a template over structured facts: no
/// model is involved (ADR-0001).
pub fn stop_reason(report: &Report) -> Option<String> {
    use crate::policy::Severity;
    if report.verdict == Verdict::Pass {
        return None;
    }
    let mut out = String::from(
        "opshero: the required verification evidence is missing or stale for the current content.\n",
    );
    for c in report
        .checks
        .iter()
        .filter(|c| c.severity == Severity::Block && c.result != Verdict::Pass)
    {
        out.push_str(&format!("- {}: {}\n", c.id, c.reason));
    }
    // A pilot (docs/status.md) found that agents often run the tests through a pipe or a
    // `;` list, which hides the exit status; the hint is only given when no run was seen.
    if report
        .checks
        .iter()
        .any(|c| c.severity == Severity::Block && c.reason.starts_with("no recorded run"))
    {
        out.push_str(
            "Run each required command on its own (no pipe, no `;` or `&&` list), so that its exit status can be recorded.\n",
        );
    }
    out.push_str("Run the required checks on the final content, then finish.\n");
    Some(out)
}
