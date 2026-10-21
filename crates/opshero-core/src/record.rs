//! Evidence records (RFC-0002 §2): immutable, content-addressed, hash-chained.

use crate::canon::{CanonError, canonicalize};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;

pub const RECORD_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    /// Worktree tree id *before* the command ran (what it ran against).
    pub worktree_tree: String,
    /// Worktree tree id *after* the command finished. Freshness uses this
    /// state: a run that only leaves untracked artifacts behind stays fresh
    /// until the code changes. `worktree_tree != tree_after` is reported.
    pub tree_after: String,
    pub head_commit: Option<String>,
    pub dirty: bool,
    /// Pathspecs excluded when the tree was computed (tool-owned directories,
    /// policy `[subject] exclude`). Evidence is only comparable between trees
    /// computed with the same exclusions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub argv: Vec<String>,
    /// Names (never values) of environment assignments written before the command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_names: Vec<String>,
    /// Directory the command ran in, relative to the repository root.
    pub cwd: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Execution {
    pub exit_code: Option<i64>,
    pub signal: Option<i64>,
    /// Informational only; never used to order evidence or decide a verdict.
    pub started_at: String,
    pub duration_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    /// `none` in v0. Reserved: `ptrace`, `coverage-import` (ADR-0008).
    pub method: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Recorder {
    pub name: String,
    pub version: String,
    pub assurance: String,
    /// Which integration produced the record (for example `claude-code`);
    /// absent for records written by `opshero run`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RecordBody {
    pub record_version: u32,
    /// Id of the previous record in this chain, if any.
    pub prev: Option<String>,
    pub subject: Subject,
    pub command: Command,
    pub execution: Execution,
    pub observation: Observation,
    pub recorder: Recorder,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub id: String,
    #[serde(flatten)]
    pub body: RecordBody,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IntegrityError {
    Malformed(String),
    UnsupportedVersion(u64),
    IdMismatch { claimed: String, actual: String },
    Canon(CanonError),
}

impl fmt::Display for IntegrityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(m) => write!(f, "malformed record: {m}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported record_version {v}"),
            Self::IdMismatch { claimed, actual } => {
                write!(
                    f,
                    "id mismatch (claimed {claimed}, content hashes to {actual})"
                )
            }
            Self::Canon(e) => write!(f, "not canonicalizable: {e}"),
        }
    }
}

impl std::error::Error for IntegrityError {}

/// Id of a record body: lowercase hex SHA-256 of its canonical JSON,
/// computed over the record *without* its `id` field.
pub fn id_of(value_without_id: &Value) -> Result<String, CanonError> {
    let canonical = canonicalize(value_without_id)?;
    let digest = Sha256::digest(canonical.as_bytes());
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// Build a record from a body, computing its content-addressed id.
pub fn new_record(body: RecordBody) -> Result<Record, CanonError> {
    let value = serde_json::to_value(&body).map_err(|_| CanonError::NonIntegerNumber)?;
    Ok(Record {
        id: id_of(&value)?,
        body,
    })
}

/// Parse and *verify* a raw JSON record: the claimed id must equal the hash
/// of the canonical content, so edited records are detected. Unknown fields
/// are tolerated (forward compatibility) but still covered by the id.
pub fn parse_verified(raw: &Value) -> Result<Record, IntegrityError> {
    let obj = raw
        .as_object()
        .ok_or_else(|| IntegrityError::Malformed("not a JSON object".into()))?;
    let claimed = obj
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| IntegrityError::Malformed("missing string field `id`".into()))?
        .to_owned();
    let version = obj
        .get("record_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| IntegrityError::Malformed("missing `record_version`".into()))?;
    if version != u64::from(RECORD_VERSION) {
        return Err(IntegrityError::UnsupportedVersion(version));
    }
    let mut without_id = obj.clone();
    without_id.remove("id");
    let actual = id_of(&Value::Object(without_id)).map_err(IntegrityError::Canon)?;
    if actual != claimed {
        return Err(IntegrityError::IdMismatch { claimed, actual });
    }
    serde_json::from_value::<Record>(raw.clone())
        .map_err(|e| IntegrityError::Malformed(e.to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn sample_body(tree: &str, exit: i64, prev: Option<&str>) -> RecordBody {
        RecordBody {
            record_version: RECORD_VERSION,
            prev: prev.map(str::to_owned),
            subject: Subject {
                worktree_tree: tree.into(),
                tree_after: tree.into(),
                head_commit: Some("c0ffee".into()),
                dirty: false,
                exclude: vec![],
            },
            command: Command {
                argv: vec!["pytest".into(), "-q".into()],
                env_names: vec![],
                cwd: ".".into(),
            },
            execution: Execution {
                exit_code: Some(exit),
                signal: None,
                started_at: "2026-01-01T00:00:00Z".into(),
                duration_ms: 10,
            },
            observation: Observation {
                method: "none".into(),
            },
            recorder: Recorder {
                name: "opshero".into(),
                version: "0.0.1".into(),
                assurance: "A1".into(),
                adapter: None,
            },
        }
    }

    #[test]
    fn id_is_stable_and_verifies() {
        let r = new_record(sample_body("aaaa", 0, None)).unwrap();
        let r2 = new_record(sample_body("aaaa", 0, None)).unwrap();
        assert_eq!(r.id, r2.id);
        assert_eq!(r.id.len(), 64);
        let raw = serde_json::to_value(&r).unwrap();
        assert_eq!(parse_verified(&raw).unwrap(), r);
    }

    #[test]
    fn editing_any_field_breaks_the_id() {
        let r = new_record(sample_body("aaaa", 0, None)).unwrap();
        let mut raw = serde_json::to_value(&r).unwrap();
        raw["execution"]["exit_code"] = serde_json::json!(1);
        assert!(matches!(
            parse_verified(&raw),
            Err(IntegrityError::IdMismatch { .. })
        ));
    }

    #[test]
    fn unknown_fields_are_tolerated_but_covered_by_the_id() {
        let r = new_record(sample_body("aaaa", 0, None)).unwrap();
        let mut raw = serde_json::to_value(&r).unwrap();
        // Adding a field without re-hashing must be detected...
        raw["future_field"] = serde_json::json!("x");
        assert!(matches!(
            parse_verified(&raw),
            Err(IntegrityError::IdMismatch { .. })
        ));
        // ...while a properly re-hashed record with the extra field is accepted.
        let mut without = raw.as_object().unwrap().clone();
        without.remove("id");
        let new_id = id_of(&Value::Object(without)).unwrap();
        raw["id"] = serde_json::json!(new_id);
        assert!(parse_verified(&raw).is_ok());
    }

    #[test]
    fn rejects_other_major_versions_and_garbage() {
        let r = new_record(sample_body("aaaa", 0, None)).unwrap();
        let mut raw = serde_json::to_value(&r).unwrap();
        raw["record_version"] = serde_json::json!(2);
        assert_eq!(
            parse_verified(&raw),
            Err(IntegrityError::UnsupportedVersion(2))
        );
        assert!(matches!(
            parse_verified(&serde_json::json!([1, 2])),
            Err(IntegrityError::Malformed(_))
        ));
    }
}
