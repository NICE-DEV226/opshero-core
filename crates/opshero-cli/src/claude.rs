//! Claude Code hook adapter.
//!
//! Contract used (official hooks reference, checked 2026-10-21; re-verify
//! against the installed version): JSON on stdin with `hook_event_name`,
//! `cwd`, `tool_name`, `tool_input.command`; `PostToolUse` runs after a
//! successful tool call (no exit code is provided, success means 0);
//! `PostToolUseFailure` carries `error` that starts with `Exit code N` for a
//! Bash command that ran and failed; `Stop` carries `stop_hook_active`.
//!
//! Principles: recording never breaks the agent (every problem is a stderr
//! warning and exit 0, RFC-0001 P8); only simple commands that the policy
//! asks for are recorded (see `opshero_core::shellcmd`).

use crate::{baseline, git, journal, ops};
use opshero_core::journal::{self as jcore, Decision, Event};
use opshero_core::policy::{self, Policy, command_matches};
use opshero_core::shellcmd::{self, Analysis};
use opshero_core::{canon, render};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Handle one hook event. `Ok(Some(text))` is printed to stdout.
pub fn handle(input: &str) -> Result<Option<String>, String> {
    let v: Value =
        serde_json::from_str(input).map_err(|e| format!("hook input is not JSON: {e}"))?;
    match v.get("hook_event_name").and_then(Value::as_str) {
        Some("PostToolUse") => bash_event(&v, false),
        Some("PostToolUseFailure") => bash_event(&v, true),
        Some("UserPromptSubmit") => prompt_event(&v),
        Some("Stop") => stop_event(&v),
        _ => Ok(None),
    }
}

fn event_dir(v: &Value) -> PathBuf {
    v.get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("CLAUDE_PROJECT_DIR").map(PathBuf::from))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn load_policy(root: &Path) -> Result<Option<Policy>, String> {
    let path = ops::policy_path(root);
    match std::fs::read_to_string(&path) {
        Ok(text) => policy::parse(&text)
            .map(Some)
            .map_err(|e| format!("{}: {e}", path.display())),
        Err(_) => Ok(None), // no policy: nothing to record or enforce
    }
}

/// Exit code from a PostToolUseFailure `error` string ("Exit code 3\n...").
fn exit_code_from_error(error: &str) -> Option<i64> {
    let rest = error.strip_prefix("Exit code ")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

fn bash_event(v: &Value, failure: bool) -> Result<Option<String>, String> {
    if v.get("tool_name").and_then(Value::as_str) != Some("Bash") {
        return Ok(None);
    }
    let input = v.get("tool_input");
    let command = match input.and_then(|i| i.get("command")).and_then(Value::as_str) {
        Some(c) => c,
        None => return Ok(None),
    };
    // A background command has no result yet; an interrupted one never finished.
    if input
        .and_then(|i| i.get("run_in_background"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        return Ok(None);
    }
    let interrupted = if failure {
        v.get("is_interrupt").and_then(Value::as_bool)
    } else {
        v.get("tool_response")
            .and_then(|r| r.get("interrupted"))
            .and_then(Value::as_bool)
    };
    if interrupted == Some(true) {
        return Ok(None);
    }
    let exit_code = if failure {
        match v
            .get("error")
            .and_then(Value::as_str)
            .and_then(exit_code_from_error)
        {
            Some(c) => c,
            None => return Ok(None), // the command did not complete (denied, spawn error, ...)
        }
    } else {
        0
    };

    let Analysis::Simple {
        env_names,
        argv,
        cd,
    } = shellcmd::analyze(command)
    else {
        return Ok(None);
    };
    let dir = event_dir(v);
    let root = git::repo_root(&dir)?;
    let Some(policy) = load_policy(&root)? else {
        return Ok(None);
    };
    if !policy
        .require
        .iter()
        .any(|r| command_matches(&r.command, &argv))
    {
        return Ok(None); // record only what the policy cares about
    }
    let gitdir = git::git_dir(&root)?;
    let ran_in = match &cd {
        Some(d) if Path::new(d).is_absolute() => PathBuf::from(d),
        Some(d) => dir.join(d),
        None => dir.clone(),
    };
    // A command that ran in another directory says nothing about this repository.
    let Some(cwd_rel) = ops::inside_root(&root, &ran_in) else {
        return Ok(None);
    };
    let exclude = policy.effective_excludes();
    let after = git::worktree_tree(&root, &gitdir, &exclude)?;
    let duration_ms = v.get("duration_ms").and_then(Value::as_u64).unwrap_or(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // The hook runs after the command, so the "before" state is unknown: use
    // the state after the run for both (documented limitation).
    ops::record(
        &gitdir,
        ops::Capture {
            argv,
            env_names,
            cwd_rel,
            tree_before: after.clone(),
            head: git::head_commit(&root),
            dirty: git::is_dirty(&root, &exclude),
            exclude,
            tree_after: after,
            exit_code: Some(exit_code),
            signal: None,
            started_secs: now.saturating_sub(duration_ms / 1000),
            duration_ms,
            adapter: Some("claude-code"),
            assurance: "A1",
        },
    )?;
    journal::log(
        &gitdir,
        &Event::Record {
            session: session_of(v),
            passed: exit_code == 0,
        },
    );
    Ok(None)
}

fn session_of(v: &Value) -> String {
    jcore::session_key(
        v.get("session_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
    )
}

/// Record the state of the working tree at the start of a turn.
fn prompt_event(v: &Value) -> Result<Option<String>, String> {
    let root = git::repo_root(&event_dir(v))?;
    let Some(policy) = load_policy(&root)? else {
        return Ok(None);
    };
    let gitdir = git::git_dir(&root)?;
    let exclude = policy.effective_excludes();
    let tree = git::worktree_tree(&root, &gitdir, &exclude)?;
    baseline::save(
        &gitdir,
        &session_of(v),
        &baseline::Baseline { tree, exclude },
    )?;
    Ok(None)
}

/// What this turn changed, relative to the baseline taken at its start.
enum Turn {
    /// No baseline (the prompt hook did not run): judge everything, as before.
    Unknown,
    Unchanged,
    /// Changed, but no file that any requirement cares about.
    NotApplicable,
    /// Changed: only these requirements apply.
    Applicable(Vec<opshero_core::policy::Require>),
}

fn judge_turn(v: &Value, root: &Path, gitdir: &Path, policy: &Policy) -> Turn {
    let Some(base) = baseline::load(gitdir, &session_of(v)) else {
        return Turn::Unknown;
    };
    let exclude = policy.effective_excludes();
    if base.exclude != exclude {
        return Turn::Unknown; // the trees are not comparable
    }
    let Ok(now) = git::worktree_tree(root, gitdir, &exclude) else {
        return Turn::Unknown; // fail closed
    };
    if now == base.tree {
        return Turn::Unchanged;
    }
    let applicable: Vec<_> = policy
        .require
        .iter()
        .filter(|r| {
            r.paths.is_empty()
                // If the comparison itself fails, assume the requirement applies.
                || git::changed_paths(root, &base.tree, &now, &r.paths).map_or(true, |p| !p.is_empty())
        })
        .cloned()
        .collect();
    if applicable.is_empty() {
        Turn::NotApplicable
    } else {
        Turn::Applicable(applicable)
    }
}

fn stop_event(v: &Value) -> Result<Option<String>, String> {
    // Never block twice in a row: avoids loops when a check cannot be satisfied.
    let stop_active = v.get("stop_hook_active").and_then(Value::as_bool) == Some(true);
    let root = git::repo_root(&event_dir(v))?;
    let Some(mut policy) = load_policy(&root)? else {
        return Ok(None);
    };
    let gitdir = git::git_dir(&root)?;
    let journaling = journal::enabled(&gitdir);
    if stop_active && !journaling {
        return Ok(None); // nothing to decide and nothing to log: skip the evaluation
    }
    let skip = match judge_turn(v, &root, &gitdir, &policy) {
        Turn::Unknown => None,
        Turn::Unchanged => Some("UNCHANGED"),
        Turn::NotApplicable => Some("NOT-APPLICABLE"),
        Turn::Applicable(reqs) => {
            policy.require = reqs;
            None
        }
    };
    if let Some(verdict) = skip {
        journal::log(
            &gitdir,
            &Event::Stop {
                session: session_of(v),
                decision: Decision::Allow,
                verdict: verdict.to_owned(),
                reasons: vec![],
                stop_active,
            },
        );
        return Ok(None);
    }
    let report = ops::evaluate_now(&root, &gitdir, &policy)?;
    let reason = if stop_active {
        None
    } else {
        render::stop_reason(&report)
    };
    journal::log(
        &gitdir,
        &Event::Stop {
            session: session_of(v),
            decision: if reason.is_some() {
                Decision::Block
            } else {
                Decision::Allow
            },
            verdict: report.verdict.as_str().to_owned(),
            reasons: if reason.is_some() {
                jcore::categorize(&report)
            } else {
                vec![]
            },
            stop_active,
        },
    );
    match reason {
        None => Ok(None),
        Some(reason) => {
            let out = canon::canonicalize(&json!({"decision": "block", "reason": reason}))
                .map_err(|e| e.to_string())?;
            Ok(Some(out))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn parses_exit_codes_from_failure_messages() {
        assert_eq!(exit_code_from_error("Exit code 1\nError: nope"), Some(1));
        assert_eq!(exit_code_from_error("Exit code 127"), Some(127));
        assert_eq!(exit_code_from_error("Command was denied"), None);
        assert_eq!(exit_code_from_error("Exit code x"), None);
        assert_eq!(exit_code_from_error(""), None);
    }

    #[test]
    fn unrelated_events_and_garbage_are_ignored_or_reported() {
        assert_eq!(
            handle(r#"{"hook_event_name":"SessionStart"}"#).unwrap(),
            None
        );
        assert_eq!(
            handle(r#"{"hook_event_name":"PostToolUse","tool_name":"Edit"}"#).unwrap(),
            None
        );
        assert!(handle("not json").is_err());
        assert_eq!(handle("{}").unwrap(), None);
    }
}
