//! Editing agent settings files: pure merge/remove functions plus nothing else,
//! so the behaviour is unit-testable. File I/O lives in main.rs.

use serde_json::{Map, Value, json};

/// Suffix that identifies hook entries written by this tool.
pub const MARKER: &str = "hook claude-code";

fn is_ours(entry: &Value) -> bool {
    entry
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(|c| c.ends_with(MARKER))
}

fn matcher_of(group: &Value) -> Option<&str> {
    group.get("matcher").and_then(Value::as_str)
}

/// Add our hooks to `settings` idempotently. Returns the new settings and a
/// human-readable list of what changed (empty when nothing did).
pub fn merge(
    mut settings: Value,
    command: &str,
    enforce_stop: bool,
) -> Result<(Value, Vec<String>), String> {
    let root = settings
        .as_object_mut()
        .ok_or("the settings file must contain a JSON object")?;
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or("`hooks` must be a JSON object")?;

    let mut targets: Vec<(&str, Option<&str>, u32)> = vec![
        ("PostToolUse", Some("Bash"), 10),
        ("PostToolUseFailure", Some("Bash"), 10),
    ];
    if enforce_stop {
        // The prompt hook records the state at the start of each turn, so that the
        // Stop gate only judges what this turn changed.
        targets.push(("UserPromptSubmit", None, 10));
        targets.push(("Stop", None, 30));
    }
    let mut changes = Vec::new();
    for (event, matcher, timeout) in targets {
        let groups = hooks
            .entry(event)
            .or_insert_with(|| Value::Array(vec![]))
            .as_array_mut()
            .ok_or_else(|| format!("`hooks.{event}` must be an array"))?;
        let entry = json!({"type": "command", "command": command, "timeout": timeout});
        let idx = groups.iter().position(|g| matcher_of(g) == matcher);
        match idx {
            Some(i) => {
                let list = groups[i]
                    .as_object_mut()
                    .ok_or_else(|| format!("`hooks.{event}` entries must be objects"))?
                    .entry("hooks")
                    .or_insert_with(|| Value::Array(vec![]))
                    .as_array_mut()
                    .ok_or_else(|| format!("`hooks.{event}[].hooks` must be an array"))?;
                if list.iter().any(is_ours) {
                    continue;
                }
                list.push(entry);
            }
            None => {
                let mut group = Map::new();
                if let Some(m) = matcher {
                    group.insert("matcher".into(), json!(m));
                }
                group.insert("hooks".into(), json!([entry]));
                groups.push(Value::Object(group));
            }
        }
        changes.push(format!(
            "add {event} hook{}",
            matcher
                .map(|m| format!(" (matcher {m})"))
                .unwrap_or_default()
        ));
    }
    Ok((settings, changes))
}

/// Remove only the entries we wrote; foreign hooks are untouched.
pub fn remove(mut settings: Value) -> Result<(Value, Vec<String>), String> {
    let mut changes = Vec::new();
    let Some(root) = settings.as_object_mut() else {
        return Err("the settings file must contain a JSON object".into());
    };
    if let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) {
        let events: Vec<String> = hooks.keys().cloned().collect();
        for event in events {
            if let Some(groups) = hooks.get_mut(&event).and_then(Value::as_array_mut) {
                for g in groups.iter_mut() {
                    if let Some(list) = g.get_mut("hooks").and_then(Value::as_array_mut) {
                        let before = list.len();
                        list.retain(|h| !is_ours(h));
                        if list.len() != before {
                            changes.push(format!("remove {event} hook"));
                        }
                    }
                }
                groups.retain(|g| {
                    g.get("hooks")
                        .and_then(Value::as_array)
                        .is_none_or(|l| !l.is_empty())
                });
            }
            if hooks
                .get(&event)
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
            {
                hooks.remove(&event);
            }
        }
        if hooks.is_empty() {
            root.remove("hooks");
        }
    }
    Ok((settings, changes))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const CMD: &str = "/usr/bin/opshero hook claude-code";

    #[test]
    fn merge_into_empty_settings_and_be_idempotent() {
        let (s1, c1) = merge(json!({}), CMD, true).unwrap();
        assert_eq!(
            c1.len(),
            4,
            "two Bash hooks, the prompt hook and the Stop hook"
        );
        let (s2, c2) = merge(s1.clone(), CMD, true).unwrap();
        assert!(c2.is_empty(), "second merge must change nothing: {c2:?}");
        assert_eq!(s1, s2);
        assert_eq!(s1["hooks"]["PostToolUse"][0]["matcher"], "Bash");
        assert!(s1["hooks"]["Stop"][0].get("matcher").is_none());
        assert!(s1["hooks"]["UserPromptSubmit"][0].get("matcher").is_none());
    }

    #[test]
    fn stop_hook_is_opt_in() {
        let (s, c) = merge(json!({}), CMD, false).unwrap();
        assert_eq!(c.len(), 2);
        assert!(s["hooks"].get("Stop").is_none());
        assert!(
            s["hooks"].get("UserPromptSubmit").is_none(),
            "the prompt hook only serves the Stop gate"
        );
    }

    #[test]
    fn foreign_settings_and_hooks_are_preserved() {
        let existing = json!({
            "model": "x",
            "hooks": {"PostToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "other-tool"}]}]}
        });
        let (s, c) = merge(existing, CMD, false).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(s["model"], "x");
        let list = s["hooks"]["PostToolUse"][0]["hooks"].as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["command"], "other-tool");
    }

    #[test]
    fn remove_deletes_only_our_entries() {
        let existing = json!({
            "hooks": {"PostToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "other-tool"}]}]}
        });
        let (merged, _) = merge(existing.clone(), CMD, true).unwrap();
        let (cleaned, changes) = remove(merged).unwrap();
        assert_eq!(changes.len(), 4);
        assert_eq!(cleaned, existing);
        let (empty, _) = remove(merge(json!({}), CMD, true).unwrap().0).unwrap();
        assert_eq!(empty, json!({}));
    }

    #[test]
    fn malformed_settings_are_refused_not_overwritten() {
        assert!(merge(json!([1]), CMD, false).is_err());
        assert!(merge(json!({"hooks": 3}), CMD, false).is_err());
        assert!(merge(json!({"hooks": {"Stop": {}}}), CMD, true).is_err());
        assert!(remove(json!("x")).is_err());
    }
}
