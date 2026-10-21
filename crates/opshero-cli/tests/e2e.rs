//! End-to-end tests: the real binary against real git repositories.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed");
}

fn new_repo() -> TempDir {
    let d = TempDir::new().unwrap();
    git(d.path(), &["init", "-q", "-b", "main"]);
    fs::write(d.path().join("code.txt"), "v1\n").unwrap();
    fs::write(d.path().join(".gitignore"), "ignored.txt\n").unwrap();
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "init"]);
    d
}

fn write_policy(dir: &Path, extra: &str) {
    fs::create_dir_all(dir.join(".opshero")).unwrap();
    fs::write(
        dir.join(".opshero/policy.toml"),
        format!(
            "version = 1\nid = \"t\"\n[[require]]\nid = \"tests\"\ncheck = \"evidence.fresh\"\ncommand = [\"sh\", \"*\"]\n{extra}"
        ),
    )
    .unwrap();
}

fn opshero(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opshero"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

fn code(o: &Output) -> i32 {
    o.status.code().unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn verify_without_a_policy_is_a_usage_error() {
    let d = new_repo();
    let o = opshero(d.path(), &["verify"]);
    assert_eq!(code(&o), 2);
    assert!(err(&o).contains("opshero init"));
}

#[test]
fn no_evidence_is_unknown() {
    let d = new_repo();
    write_policy(d.path(), "");
    let o = opshero(d.path(), &["verify"]);
    assert_eq!(code(&o), 11, "{}", out(&o));
    assert!(out(&o).contains("verdict: UNKNOWN"));
}

#[test]
fn pass_then_stale_after_an_edit_then_pass_again() {
    let d = new_repo();
    write_policy(d.path(), "");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    let o = opshero(d.path(), &["verify"]);
    assert_eq!(code(&o), 0, "{}", out(&o));
    assert!(out(&o).contains("assurance: A1"));

    fs::write(d.path().join("code.txt"), "v2\n").unwrap();
    let o = opshero(d.path(), &["verify"]);
    assert_eq!(code(&o), 10, "{}", out(&o));
    assert!(out(&o).contains("stale evidence"));

    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    assert_eq!(code(&opshero(d.path(), &["verify"])), 0);
}

#[test]
fn failing_command_exit_code_is_passed_through_and_recorded() {
    let d = new_repo();
    write_policy(d.path(), "");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 3"])),
        3
    );
    let o = opshero(d.path(), &["verify"]);
    assert_eq!(code(&o), 10);
    assert!(out(&o).contains("exit code 3"), "{}", out(&o));
}

#[test]
fn ignored_files_do_not_change_the_subject_but_untracked_files_do() {
    let d = new_repo();
    write_policy(d.path(), "");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    fs::write(d.path().join("ignored.txt"), "x").unwrap();
    assert_eq!(
        code(&opshero(d.path(), &["verify"])),
        0,
        "ignored file must not invalidate"
    );
    fs::write(d.path().join("new.txt"), "x").unwrap();
    assert_eq!(
        code(&opshero(d.path(), &["verify"])),
        10,
        "untracked file must invalidate"
    );
}

#[test]
fn a_tampered_record_is_ignored_reported_and_never_counts_as_a_pass() {
    let d = new_repo();
    write_policy(d.path(), "");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 1"])),
        1
    );
    let rec_dir = d.path().join(".git/opshero/records");
    let file = fs::read_dir(&rec_dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"exit_code\":1"));
    // Forge a green result by editing the recorded exit code.
    fs::write(&file, text.replace("\"exit_code\":1", "\"exit_code\":0")).unwrap();
    let o = opshero(d.path(), &["verify"]);
    assert_eq!(code(&o), 11, "forged record must not pass: {}", out(&o));
    assert!(out(&o).contains("ignored"), "{}", out(&o));
}

#[test]
fn json_output_is_canonical_and_deterministic() {
    let d = new_repo();
    write_policy(d.path(), "");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    let a = opshero(d.path(), &["verify", "--format", "json"]);
    let b = opshero(d.path(), &["verify", "--format", "json"]);
    assert_eq!(a.stdout, b.stdout);
    let v: serde_json::Value = serde_json::from_slice(&a.stdout).unwrap();
    assert_eq!(v["verdict"], "PASS");
    assert_eq!(v["schema"], "opshero.verify/v0");
    // Canonical form: compact, keys sorted.
    let text = out(&a);
    assert!(
        text.starts_with("{\"assurance\":\"A1\",\"checks\":["),
        "{text}"
    );
}

#[test]
fn recording_failure_never_breaks_the_wrapped_command() {
    let d = new_repo();
    // A regular file where the store directory should be makes recording impossible.
    fs::write(d.path().join(".git/opshero"), "blocker").unwrap();
    let o = opshero(d.path(), &["run", "--", "sh", "-c", "echo hello; exit 7"]);
    assert_eq!(code(&o), 7);
    assert!(out(&o).contains("hello"));
    assert!(err(&o).contains("warning"), "{}", err(&o));
}

#[test]
fn run_outside_a_git_repository_still_runs_the_command() {
    let d = TempDir::new().unwrap();
    let o = opshero(d.path(), &["run", "--", "sh", "-c", "exit 5"]);
    assert_eq!(code(&o), 5);
    assert!(err(&o).contains("not inside a git repository"));
}

#[test]
fn init_writes_a_policy_and_refuses_to_overwrite() {
    let d = new_repo();
    assert_eq!(code(&opshero(d.path(), &["init"])), 0);
    assert!(d.path().join(".opshero/policy.toml").exists());
    assert_eq!(code(&opshero(d.path(), &["init"])), 2);
    assert_eq!(code(&opshero(d.path(), &["init", "--force"])), 0);
}

#[test]
fn the_real_index_is_never_modified_by_recording_or_verifying() {
    let d = new_repo();
    write_policy(d.path(), "");
    fs::write(d.path().join("new.txt"), "x").unwrap();
    let before = Command::new("git")
        .arg("-C")
        .arg(d.path())
        .args(["status", "--porcelain"])
        .output()
        .unwrap()
        .stdout;
    opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"]);
    opshero(d.path(), &["verify"]);
    let after = Command::new("git")
        .arg("-C")
        .arg(d.path())
        .args(["status", "--porcelain"])
        .output()
        .unwrap()
        .stdout;
    assert_eq!(
        before, after,
        "git status must be unchanged (untracked stays untracked)"
    );
}

#[test]
fn missing_command_after_the_separator_is_a_usage_error() {
    let d = new_repo();
    assert_eq!(code(&opshero(d.path(), &["run"])), 2);
}

// ---------------------------------------------------------------------------
// Claude Code adapter: real hook events piped to `opshero hook claude-code`.
// ---------------------------------------------------------------------------

use std::io::Write;
use std::process::Stdio;

fn hook(dir: &Path, event: serde_json::Value) -> Output {
    hook_raw(dir, &event.to_string())
}

fn hook_raw(dir: &Path, input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_opshero"))
        .current_dir(dir)
        .args(["hook", "claude-code"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn echo_policy(dir: &Path) {
    fs::create_dir_all(dir.join(".opshero")).unwrap();
    fs::write(
        dir.join(".opshero/policy.toml"),
        "version = 1\nid = \"t\"\n[[require]]\nid = \"tests\"\ncheck = \"evidence.fresh\"\ncommand = [\"mytests\", \"*\"]\n",
    )
    .unwrap();
}

fn bash_ok(dir: &Path, command: &str) -> serde_json::Value {
    serde_json::json!({
        "hook_event_name": "PostToolUse", "tool_name": "Bash", "cwd": dir,
        "tool_input": {"command": command}, "tool_response": {"stdout": "", "stderr": "", "interrupted": false},
        "duration_ms": 1200
    })
}

fn bash_failed(dir: &Path, command: &str, error: &str) -> serde_json::Value {
    serde_json::json!({
        "hook_event_name": "PostToolUseFailure", "tool_name": "Bash", "cwd": dir,
        "tool_input": {"command": command}, "error": error, "is_interrupt": false
    })
}

fn record_count(dir: &Path) -> usize {
    fs::read_dir(dir.join(".git/opshero/records"))
        .map(|d| d.count())
        .unwrap_or(0)
}

#[test]
fn hook_records_a_successful_matching_command_and_verify_passes() {
    let d = new_repo();
    echo_policy(d.path());
    let o = hook(d.path(), bash_ok(d.path(), "mytests -q tests/"));
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_eq!(record_count(d.path()), 1);
    let v = opshero(d.path(), &["verify"]);
    assert_eq!(code(&v), 0, "{}", out(&v));
    // The record says which integration produced it.
    let f = fs::read_dir(d.path().join(".git/opshero/records"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(
        fs::read_to_string(f)
            .unwrap()
            .contains("\"adapter\":\"claude-code\"")
    );
}

#[test]
fn hook_uses_the_exit_code_from_a_failure_event() {
    let d = new_repo();
    echo_policy(d.path());
    let o = hook(
        d.path(),
        bash_failed(d.path(), "mytests", "Exit code 3\nFAILED test_x"),
    );
    assert_eq!(code(&o), 0);
    let v = opshero(d.path(), &["verify"]);
    assert_eq!(code(&v), 10);
    assert!(out(&v).contains("exit code 3"), "{}", out(&v));
}

#[test]
fn hook_ignores_commands_the_policy_does_not_ask_for_and_untrusted_shapes() {
    let d = new_repo();
    echo_policy(d.path());
    for cmd in [
        "ls -la",               // not in the policy
        "mytests | tail -5",    // pipeline can mask a failure
        "mytests && echo done", // list
        "mytests; true",        // list
        "echo $(mytests)",      // substitution
    ] {
        hook(d.path(), bash_ok(d.path(), cmd));
    }
    // background / interrupted / non-Bash / no-exit-code failures are skipped too
    let mut bg = bash_ok(d.path(), "mytests");
    bg["tool_input"]["run_in_background"] = serde_json::json!(true);
    hook(d.path(), bg);
    let mut intr = bash_failed(d.path(), "mytests", "Exit code 1");
    intr["is_interrupt"] = serde_json::json!(true);
    hook(d.path(), intr);
    hook(
        d.path(),
        bash_failed(d.path(), "mytests", "Permission denied by user"),
    );
    let mut edit = bash_ok(d.path(), "mytests");
    edit["tool_name"] = serde_json::json!("Edit");
    hook(d.path(), edit);
    assert_eq!(
        record_count(d.path()),
        0,
        "nothing untrusted may become evidence"
    );
}

#[test]
fn hook_understands_env_prefix_and_cd_prefix() {
    let d = new_repo();
    echo_policy(d.path());
    fs::create_dir_all(d.path().join("sub")).unwrap();
    hook(d.path(), bash_ok(d.path(), "CI=true TZ=UTC mytests -x"));
    hook(d.path(), bash_ok(d.path(), "cd sub && mytests -x"));
    assert_eq!(record_count(d.path()), 2);
    let texts: Vec<String> = fs::read_dir(d.path().join(".git/opshero/records"))
        .unwrap()
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .collect();
    assert!(
        texts
            .iter()
            .any(|t| t.contains("\"env_names\":[\"CI\",\"TZ\"]"))
    );
    assert!(texts.iter().any(|t| t.contains("\"cwd\":\"sub\"")));

    // A variable that is not on the allowlist (it could redirect the toolchain) is not evidence.
    hook(d.path(), bash_ok(d.path(), "PATH=/tmp/fake mytests -x"));
    hook(d.path(), bash_ok(d.path(), "MYAPP_ENV=test mytests -x"));
    assert_eq!(
        record_count(d.path()),
        2,
        "unlisted variables must not produce evidence"
    );
}

#[test]
fn hook_is_silent_without_a_policy_and_never_fails_on_garbage() {
    let d = new_repo();
    assert_eq!(code(&hook(d.path(), bash_ok(d.path(), "mytests"))), 0);
    assert_eq!(record_count(d.path()), 0);
    echo_policy(d.path());
    for garbage in [
        "",
        "not json",
        "{}",
        "[1,2]",
        "{\"hook_event_name\":\"PostToolUse\"}",
    ] {
        assert_eq!(
            code(&hook_raw(d.path(), garbage)),
            0,
            "garbage {garbage:?} must not fail"
        );
    }
    // outside any repository
    let nowhere = TempDir::new().unwrap();
    let o = hook(nowhere.path(), bash_ok(nowhere.path(), "mytests"));
    assert_eq!(code(&o), 0);
}

#[test]
fn stop_hook_blocks_until_evidence_is_fresh_then_lets_the_agent_finish() {
    let d = new_repo();
    echo_policy(d.path());
    let stop = |active: bool| {
        hook(
            d.path(),
            serde_json::json!({"hook_event_name": "Stop", "cwd": d.path(), "stop_hook_active": active}),
        )
    };

    // No evidence: block, with a deterministic reason.
    let o = stop(false);
    assert_eq!(code(&o), 0);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["decision"], "block");
    assert!(v["reason"].as_str().unwrap().contains("tests"));
    // Already continuing because of a stop hook: never block twice in a row.
    assert!(out(&stop(true)).trim().is_empty());

    // Evidence recorded, then the code changes after it: stale -> block again.
    hook(d.path(), bash_ok(d.path(), "mytests"));
    assert!(
        out(&stop(false)).trim().is_empty(),
        "fresh evidence lets the agent stop"
    );
    fs::write(d.path().join("code.txt"), "edited after the tests\n").unwrap();
    let v: serde_json::Value = serde_json::from_slice(&stop(false).stdout).unwrap();
    assert_eq!(v["decision"], "block");
    assert!(v["reason"].as_str().unwrap().contains("stale evidence"));
}

#[test]
fn stop_hook_without_a_policy_never_blocks() {
    let d = new_repo();
    let o = hook(
        d.path(),
        serde_json::json!({"hook_event_name": "Stop", "cwd": d.path()}),
    );
    assert_eq!(code(&o), 0);
    assert!(out(&o).trim().is_empty());
}

#[test]
fn install_is_a_dry_run_by_default_then_applies_idempotently_and_uninstalls_cleanly() {
    let d = new_repo();
    let settings = d.path().join(".claude/settings.local.json");
    let o = opshero(d.path(), &["hook", "install"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(out(&o).contains("dry run"));
    assert!(!settings.exists(), "dry run must not write");

    // A pre-existing file with a foreign hook and an unrelated key.
    fs::create_dir_all(d.path().join(".claude")).unwrap();
    let original = r#"{"theme":"dark","hooks":{"PostToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"other-tool"}]}]}}"#;
    fs::write(&settings, original).unwrap();
    assert_eq!(
        code(&opshero(
            d.path(),
            &["hook", "install", "--enforce-stop", "--yes"]
        )),
        0
    );
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
    assert_eq!(v["theme"], "dark");
    assert!(
        v["hooks"]["Stop"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .ends_with("hook claude-code")
    );
    assert_eq!(
        v["hooks"]["PostToolUse"][0]["hooks"][0]["command"],
        "other-tool"
    );
    assert!(
        d.path()
            .join(".claude/settings.local.json.opshero-bak")
            .exists()
    );

    let again = opshero(d.path(), &["hook", "install", "--enforce-stop", "--yes"]);
    assert!(out(&again).contains("nothing to change"));

    assert_eq!(code(&opshero(d.path(), &["hook", "uninstall", "--yes"])), 0);
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
    assert_eq!(
        v,
        serde_json::from_str::<serde_json::Value>(original).unwrap()
    );
}

#[test]
fn install_refuses_to_touch_a_settings_file_that_is_not_valid_json() {
    let d = new_repo();
    fs::create_dir_all(d.path().join(".claude")).unwrap();
    let path = d.path().join(".claude/settings.local.json");
    fs::write(&path, "{ this is not json").unwrap();
    let o = opshero(d.path(), &["hook", "install", "--yes"]);
    assert_eq!(code(&o), 2);
    assert_eq!(fs::read_to_string(&path).unwrap(), "{ this is not json");
}

// ---------------------------------------------------------------------------
// Subject exclusions (found by a real Claude Code session: files written by
// tools after the recorded run made the evidence look stale).
// ---------------------------------------------------------------------------

#[test]
fn files_written_by_tools_after_the_run_do_not_make_evidence_stale() {
    let d = new_repo();
    write_policy(d.path(), "");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    for dir in [".claude", ".cursor", ".codex", ".opshero"] {
        fs::create_dir_all(d.path().join(dir)).unwrap();
        fs::write(d.path().join(dir).join("session.json"), "{}").unwrap();
    }
    let o = opshero(d.path(), &["verify"]);
    assert_eq!(code(&o), 0, "{}", out(&o));
    // ...but a real source change still does.
    fs::write(d.path().join("code.txt"), "changed\n").unwrap();
    assert_eq!(code(&opshero(d.path(), &["verify"])), 10);
}

#[test]
fn policy_exclusions_apply_and_changing_them_invalidates_old_evidence() {
    let d = new_repo();
    write_policy(d.path(), "[subject]\nexclude = [\"*.log\"]\n");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    fs::write(d.path().join("debug.log"), "noise").unwrap();
    assert_eq!(
        code(&opshero(d.path(), &["verify"])),
        0,
        "excluded pattern must not invalidate"
    );

    // Dropping the exclusion changes how the subject is defined: old evidence no longer applies.
    write_policy(d.path(), "");
    let o = opshero(d.path(), &["verify"]);
    assert_eq!(code(&o), 10, "{}", out(&o));
}

#[test]
fn exclusions_are_reported_in_the_json_output_and_stored_in_the_record() {
    let d = new_repo();
    write_policy(d.path(), "[subject]\nexclude = [\"dist\"]\n");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    let v: serde_json::Value =
        serde_json::from_slice(&opshero(d.path(), &["verify", "--format", "json"]).stdout).unwrap();
    assert_eq!(
        v["subject"]["exclude"],
        serde_json::json!([".claude", ".codex", ".cursor", ".opshero", "dist"])
    );
    let f = fs::read_dir(d.path().join(".git/opshero/records"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(
        fs::read_to_string(f)
            .unwrap()
            .contains("\"exclude\":[\".claude\",\".codex\",\".cursor\",\".opshero\",\"dist\"]")
    );
}

#[test]
fn an_invalid_exclusion_in_the_policy_is_a_usage_error() {
    let d = new_repo();
    write_policy(d.path(), "[subject]\nexclude = [\":(top)x\"]\n");
    assert_eq!(code(&opshero(d.path(), &["verify"])), 2);
}

// Repeat rule (RFC-0005): a flaky command must not pass on one lucky run.
#[test]
fn repeat_rule_needs_two_passes_and_rejects_a_mixed_history() {
    let d = new_repo();
    // alpha 25 %, flake rate 50 % -> two consecutive passes are required.
    write_policy(
        d.path(),
        "repeat = { alpha = \"25%\", flake_rate = \"50%\" }\n",
    );
    let run = |c: &str| code(&opshero(d.path(), &["run", "--", "sh", "-c", c]));

    assert_eq!(run("exit 0"), 0);
    let one = opshero(d.path(), &["verify"]);
    assert_eq!(code(&one), 11, "one pass is not enough: {}", out(&one));
    assert!(
        out(&one).contains("1 of the 2 runs required"),
        "{}",
        out(&one)
    );

    assert_eq!(run("exit 0"), 0);
    assert_eq!(code(&opshero(d.path(), &["verify"])), 0);

    // A failure on the same content afterwards makes the history inconsistent.
    assert_eq!(run("exit 1"), 1);
    let mixed = opshero(d.path(), &["verify"]);
    assert_eq!(code(&mixed), 10, "{}", out(&mixed));
    assert!(out(&mixed).contains("inconsistent"), "{}", out(&mixed));
}

#[test]
fn an_invalid_repeat_rule_is_a_policy_error_not_a_silent_default() {
    let d = new_repo();
    write_policy(
        d.path(),
        "repeat = { alpha = \"5\", flake_rate = \"5%\" }\n",
    );
    let o = opshero(d.path(), &["verify"]);
    assert_eq!(code(&o), 2, "{}", err(&o));
    assert!(err(&o).contains("repeat.alpha"), "{}", err(&o));
}

// `opshero run --repeat` (RFC-0005 OQ3).
/// A command that fails on its second invocation; its counter lives in a
/// git-ignored file so it never changes the measured content.
const FAILS_SECOND_TIME: &str =
    "n=$(cat ignored.txt 2>/dev/null || echo 0); echo $((n+1)) > ignored.txt; [ $((n+1)) -ne 2 ]";

#[test]
fn run_repeat_records_every_run_and_passes_when_all_pass() {
    let d = new_repo();
    write_policy(
        d.path(),
        "repeat = { alpha = \"25%\", flake_rate = \"50%\" }\n",
    );
    let o = opshero(
        d.path(),
        &["run", "--repeat", "3", "--", "sh", "-c", "exit 0"],
    );
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_eq!(record_count(d.path()), 3);
    assert!(
        err(&o).contains("3 runs: 3 passed, 0 failed"),
        "{}",
        err(&o)
    );
    assert_eq!(code(&opshero(d.path(), &["verify"])), 0);
}

#[test]
fn run_repeat_exposes_a_flaky_command_and_verify_rejects_it() {
    let d = new_repo();
    write_policy(
        d.path(),
        "repeat = { alpha = \"25%\", flake_rate = \"50%\" }\n",
    );
    let o = opshero(
        d.path(),
        &["run", "--repeat", "3", "--", "sh", "-c", FAILS_SECOND_TIME],
    );
    assert_eq!(
        code(&o),
        1,
        "the exit code of the failing run is passed through: {}",
        err(&o)
    );
    assert_eq!(
        record_count(d.path()),
        3,
        "all runs are recorded, not just up to the first failure"
    );
    assert!(err(&o).contains("2 passed, 1 failed"), "{}", err(&o));
    assert!(err(&o).contains("inconsistent"), "{}", err(&o));
    let v = opshero(d.path(), &["verify"]);
    assert_eq!(code(&v), 10, "{}", out(&v));
    assert!(out(&v).contains("inconsistent"), "{}", out(&v));
}

#[test]
fn run_repeat_auto_takes_the_count_from_the_policy() {
    let d = new_repo();
    write_policy(
        d.path(),
        "repeat = { alpha = \"12.5%\", flake_rate = \"50%\" }\n", // N = 3
    );
    let o = opshero(
        d.path(),
        &["run", "--repeat", "auto", "--", "sh", "-c", "exit 0"],
    );
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert_eq!(record_count(d.path()), 3);
    assert_eq!(code(&opshero(d.path(), &["verify"])), 0);
}

#[test]
fn run_repeat_auto_without_a_matching_repeat_rule_is_a_usage_error() {
    let d = new_repo();
    write_policy(d.path(), ""); // requirement matches but has no repeat rule
    let o = opshero(
        d.path(),
        &["run", "--repeat", "auto", "--", "sh", "-c", "exit 0"],
    );
    assert_eq!(code(&o), 2, "{}", err(&o));
    assert!(err(&o).contains("repeat"), "{}", err(&o));
    assert_eq!(
        record_count(d.path()),
        0,
        "nothing runs when the request is invalid"
    );
}

#[test]
fn run_repeat_rejects_invalid_counts_before_running_anything() {
    let d = new_repo();
    write_policy(d.path(), "");
    for bad in ["0", "-1", "abc", "1001", ""] {
        let o = opshero(
            d.path(),
            &["run", "--repeat", bad, "--", "sh", "-c", "exit 0"],
        );
        assert_eq!(code(&o), 2, "--repeat {bad:?}: {}", err(&o));
    }
    assert_eq!(record_count(d.path()), 0);
}

#[test]
fn run_repeat_one_is_the_same_as_a_plain_run() {
    let d = new_repo();
    write_policy(d.path(), "");
    let o = opshero(
        d.path(),
        &["run", "--repeat", "1", "--", "sh", "-c", "exit 7"],
    );
    assert_eq!(code(&o), 7);
    assert_eq!(record_count(d.path()), 1);
    assert!(
        !err(&o).contains("runs:"),
        "no summary for a single run: {}",
        err(&o)
    );
}

// post_run_changes (docs/formal/Freshness.tla, RFC-0005 §7).
const WRITES_A_FILE: &str = "echo generated >> generated.txt";

#[test]
fn the_default_rejects_a_run_that_changed_the_tree_and_the_exclude_remedy_works() {
    let d = new_repo();
    write_policy(d.path(), ""); // no post_run_changes: the default is deny
    assert_eq!(
        code(&opshero(
            d.path(),
            &["run", "--", "sh", "-c", WRITES_A_FILE]
        )),
        0
    );
    let v = opshero(d.path(), &["verify"]);
    assert_eq!(code(&v), 10, "{}", out(&v));
    assert!(
        out(&v).contains("working tree changed while it ran"),
        "{}",
        out(&v)
    );
    assert!(
        out(&v).contains("[subject] exclude"),
        "the message must say how to fix it: {}",
        out(&v)
    );

    // The advice in the message: declare the generated path as excluded.
    fs::remove_file(d.path().join("generated.txt")).unwrap();
    write_policy(d.path(), "[subject]\nexclude = [\"generated.txt\"]\n");
    assert_eq!(
        code(&opshero(
            d.path(),
            &["run", "--", "sh", "-c", WRITES_A_FILE]
        )),
        0
    );
    let v = opshero(d.path(), &["verify"]);
    assert_eq!(code(&v), 0, "{}", out(&v));
}

#[test]
fn allow_is_an_explicit_opt_out_that_passes_with_a_finding() {
    let d = new_repo();
    write_policy(d.path(), "post_run_changes = \"allow\"\n");
    assert_eq!(
        code(&opshero(
            d.path(),
            &["run", "--", "sh", "-c", WRITES_A_FILE]
        )),
        0
    );
    let v = opshero(d.path(), &["verify"]);
    assert_eq!(code(&v), 0, "{}", out(&v));
    assert!(out(&v).contains("modified the worktree"), "{}", out(&v));
}

#[test]
fn deny_still_accepts_a_run_that_leaves_the_tree_unchanged() {
    let d = new_repo();
    write_policy(d.path(), "post_run_changes = \"deny\"\n");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    assert_eq!(code(&opshero(d.path(), &["verify"])), 0);
}

// Regression: repositories that gitignore the tool directories they also
// exclude (`.claude/`, `.cursor/`, `.opshero/` are very common entries).
#[test]
fn works_when_the_repository_ignores_the_excluded_tool_directories() {
    let d = new_repo();
    fs::write(
        d.path().join(".gitignore"),
        "ignored.txt\n.claude/\n.cursor/\n.opshero/\n",
    )
    .unwrap();
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "ignore tool dirs"]);
    fs::create_dir_all(d.path().join(".claude")).unwrap();
    fs::write(d.path().join(".claude/settings.local.json"), "{}").unwrap();
    write_policy(d.path(), "");
    let r = opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"]);
    assert_eq!(code(&r), 0, "{}", err(&r));
    assert!(!err(&r).contains("cannot measure"), "{}", err(&r));
    assert_eq!(record_count(d.path()), 1, "evidence must be recorded");
    let v = opshero(d.path(), &["verify"]);
    assert_eq!(code(&v), 0, "{}{}", out(&v), err(&v));
}

// ---------------------------------------------------------------------------
// Dogfooding journal (docs/validation-plan.md H1): opt-in, local, content-free.
// ---------------------------------------------------------------------------

fn stop_event(dir: &Path, session: &str, active: bool) -> serde_json::Value {
    serde_json::json!({
        "hook_event_name": "Stop", "cwd": dir, "session_id": session,
        "stop_hook_active": active
    })
}

fn bash_ok_in(dir: &Path, session: &str, command: &str) -> serde_json::Value {
    let mut v = bash_ok(dir, command);
    v["session_id"] = serde_json::json!(session);
    v
}

fn journal_text(dir: &Path) -> String {
    fs::read_to_string(dir.join(".git/opshero/journal.jsonl")).unwrap_or_default()
}

#[test]
fn the_journal_is_off_by_default_and_writes_nothing() {
    let d = new_repo();
    echo_policy(d.path());
    let o = hook(d.path(), stop_event(d.path(), "s1", false));
    assert!(out(&o).contains("\"decision\":\"block\""), "{}", out(&o));
    assert!(!d.path().join(".git/opshero/journal.jsonl").exists());
    let s = opshero(d.path(), &["journal", "status"]);
    assert_eq!(code(&s), 0);
    assert!(out(&s).contains("disabled"), "{}", out(&s));
}

#[test]
fn an_enabled_journal_records_stop_decisions_and_runs_without_any_content() {
    let d = new_repo();
    echo_policy(d.path());
    let e = opshero(d.path(), &["journal", "enable"]);
    assert_eq!(code(&e), 0, "{}", err(&e));
    assert!(
        out(&e).contains("never stored"),
        "must state what is not stored: {}",
        out(&e)
    );

    hook(
        d.path(),
        stop_event(d.path(), "SECRET-SESSION-ID-123", false),
    ); // blocks: no evidence
    hook(
        d.path(),
        bash_ok_in(
            d.path(),
            "SECRET-SESSION-ID-123",
            "mytests --flag-with-secret-token",
        ),
    );
    hook(
        d.path(),
        stop_event(d.path(), "SECRET-SESSION-ID-123", false),
    ); // allows: fresh evidence

    let text = journal_text(d.path());
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert!(
        lines[0].contains("\"kind\":\"stop\"") && lines[0].contains("\"decision\":\"block\""),
        "{}",
        lines[0]
    );
    assert!(lines[0].contains("\"missing\""), "{}", lines[0]);
    assert!(
        lines[1].contains("\"kind\":\"record\"") && lines[1].contains("\"passed\":true"),
        "{}",
        lines[1]
    );
    assert!(
        lines[2].contains("\"decision\":\"allow\"") && lines[2].contains("\"verdict\":\"PASS\""),
        "{}",
        lines[2]
    );

    // Privacy: nothing identifying may be in the file.
    let tree = {
        let v = opshero(d.path(), &["verify", "--format", "json"]);
        let j: serde_json::Value = serde_json::from_str(&out(&v)).unwrap();
        j["subject"]["tree"].as_str().unwrap().to_owned()
    };
    for secret in [
        "SECRET-SESSION-ID-123",
        "flag-with-secret-token",
        "mytests",
        d.path().to_str().unwrap(),
        &tree[..12],
    ] {
        assert!(!text.contains(secret), "journal leaks {secret:?}:\n{text}");
    }
}

#[test]
fn a_second_stop_after_a_block_is_journaled_as_allow_and_marked() {
    let d = new_repo();
    echo_policy(d.path());
    opshero(d.path(), &["journal", "enable"]);
    hook(d.path(), stop_event(d.path(), "s", false));
    let o = hook(d.path(), stop_event(d.path(), "s", true));
    assert_eq!(out(&o).trim(), "", "never blocks twice in a row");
    let text = journal_text(d.path());
    let last = text.lines().last().unwrap();
    assert!(
        last.contains("\"decision\":\"allow\"") && last.contains("\"stop_active\":true"),
        "{last}"
    );
}

#[test]
fn disabling_stops_the_logging() {
    let d = new_repo();
    echo_policy(d.path());
    opshero(d.path(), &["journal", "enable"]);
    hook(d.path(), stop_event(d.path(), "s", false));
    assert_eq!(code(&opshero(d.path(), &["journal", "disable"])), 0);
    hook(d.path(), stop_event(d.path(), "s", false));
    assert_eq!(journal_text(d.path()).lines().count(), 1);
    assert!(out(&opshero(d.path(), &["journal", "status"])).contains("disabled"));
}

#[test]
fn a_broken_journal_never_breaks_the_agent() {
    let d = new_repo();
    echo_policy(d.path());
    opshero(d.path(), &["journal", "enable"]);
    // The journal path is a directory: appending is impossible.
    fs::create_dir_all(d.path().join(".git/opshero/journal.jsonl")).unwrap();
    let o = hook(d.path(), stop_event(d.path(), "s", false));
    assert_eq!(code(&o), 0);
    assert!(
        out(&o).contains("\"decision\":\"block\""),
        "the gate still works: {}",
        out(&o)
    );
}

#[test]
fn label_list_and_report_flow() {
    let d = new_repo();
    echo_policy(d.path());
    opshero(d.path(), &["journal", "enable"]);
    hook(d.path(), stop_event(d.path(), "A", false)); // line 1: block
    hook(d.path(), stop_event(d.path(), "B", false)); // line 2: block
    hook(d.path(), bash_ok_in(d.path(), "B", "mytests")); // line 3: record
    hook(d.path(), stop_event(d.path(), "B", false)); // line 4: allow

    let list = opshero(d.path(), &["journal", "list"]);
    assert_eq!(code(&list), 0);
    assert!(
        out(&list).contains("1 ")
            && out(&list).contains("missing")
            && out(&list).contains("unlabeled"),
        "{}",
        out(&list)
    );

    assert_eq!(
        code(&opshero(d.path(), &["journal", "label", "1", "real"])),
        0
    );
    assert_eq!(
        code(&opshero(d.path(), &["journal", "label", "2", "noise"])),
        0
    );
    // Only blocking stops can be labelled.
    assert_eq!(
        code(&opshero(d.path(), &["journal", "label", "4", "real"])),
        2
    );
    assert_eq!(
        code(&opshero(d.path(), &["journal", "label", "99", "real"])),
        2
    );
    assert_eq!(
        code(&opshero(d.path(), &["journal", "label", "1", "maybe"])),
        2
    );

    let r = opshero(d.path(), &["journal", "report", "--format", "json"]);
    assert_eq!(code(&r), 0, "{}", err(&r));
    let j: serde_json::Value = serde_json::from_str(&out(&r)).unwrap();
    assert_eq!(j["sessions"], 2);
    assert_eq!(j["blocks"], 2);
    assert_eq!(j["blocks_real"], 1);
    assert_eq!(j["blocks_noise"], 1);
    assert_eq!(j["sessions_with_real_finding"], 1);
    assert_eq!(j["outcome"], "not-enough-data");
    assert_eq!(j["needed_sessions"], 100);

    let text = opshero(d.path(), &["journal", "report"]);
    assert!(out(&text).contains("1 of 2 sessions"), "{}", out(&text));
    assert!(out(&text).contains("not enough data"), "{}", out(&text));
}

#[test]
fn journal_commands_outside_a_repository_are_usage_errors() {
    let d = TempDir::new().unwrap();
    for args in [["journal", "status"], ["journal", "report"]] {
        let o = opshero(d.path(), &args);
        assert_ne!(code(&o), 0);
    }
}

// ---------------------------------------------------------------------------
// Concurrency: many recorders at once must not lose, fork or mis-measure evidence.
// Found by a stress probe (2026-10-21): 40 parallel runs gave 39 records, 11 of them
// measured against the EMPTY tree, and a forked chain.
// ---------------------------------------------------------------------------

const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

fn read_records(dir: &Path) -> Vec<serde_json::Value> {
    let Ok(entries) = fs::read_dir(dir.join(".git/opshero/records")) else {
        return vec![]; // no record was ever written
    };
    entries
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap())
        .collect()
}

#[test]
fn parallel_recorders_lose_nothing_fork_nothing_and_measure_the_real_tree() {
    let d = new_repo();
    write_policy(d.path(), "");
    fs::write(d.path().join("work.txt"), "content\n").unwrap();
    let expected_tree = {
        let v = opshero(d.path(), &["verify", "--format", "json"]);
        let j: serde_json::Value = serde_json::from_str(&out(&v)).unwrap();
        j["subject"]["tree"].as_str().unwrap().to_owned()
    };
    assert_ne!(expected_tree, EMPTY_TREE);

    const N: usize = 40;
    for round in 0..2 {
        let children: Vec<_> = (0..N)
            .map(|_| {
                Command::new(env!("CARGO_BIN_EXE_opshero"))
                    .current_dir(d.path())
                    .args(["run", "--", "sh", "-c", "exit 0"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap()
            })
            .collect();
        for c in children {
            let o = c.wait_with_output().unwrap();
            assert_eq!(code(&o), 0);
            assert!(err(&o).is_empty(), "no warning expected: {}", err(&o));
        }

        let recs = read_records(d.path());
        assert_eq!(
            recs.len(),
            N * (round + 1),
            "a recorded run went missing (round {round})"
        );
        for r in &recs {
            assert_eq!(
                r["subject"]["tree_after"],
                expected_tree.as_str(),
                "wrong tree in {r}"
            );
            assert_eq!(
                r["subject"]["worktree_tree"],
                expected_tree.as_str(),
                "wrong tree in {r}"
            );
        }
        // One linear chain: exactly one root, every other record has a distinct parent that exists.
        let ids: std::collections::HashSet<&str> =
            recs.iter().map(|r| r["id"].as_str().unwrap()).collect();
        let mut parents = std::collections::HashSet::new();
        let mut roots = 0;
        for r in &recs {
            match r["prev"].as_str() {
                None => roots += 1,
                Some(p) => {
                    assert!(ids.contains(p), "dangling parent");
                    assert!(
                        parents.insert(p),
                        "chain forked: two records share the parent {p}"
                    );
                }
            }
        }
        assert_eq!(roots, 1, "exactly one chain root");
    }

    let v = opshero(d.path(), &["verify"]);
    assert_eq!(code(&v), 0, "{}", out(&v));
    assert!(
        out(&v).contains("80 run(s)"),
        "all runs must count: {}",
        out(&v)
    );
}

// ---------------------------------------------------------------------------
// `opshero gc`: the evidence store must stay bounded without losing what matters.
// ---------------------------------------------------------------------------

#[test]
fn gc_prunes_old_evidence_keeps_the_current_tree_and_supports_dry_run() {
    let d = new_repo();
    write_policy(d.path(), "");
    for i in 0..12 {
        fs::write(d.path().join("code.txt"), format!("v{i}\n")).unwrap();
        assert_eq!(
            code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
            0
        );
    }
    assert_eq!(record_count(d.path()), 12);

    let dry = opshero(d.path(), &["gc", "--keep", "3", "--dry-run"]);
    assert_eq!(code(&dry), 0, "{}", err(&dry));
    assert!(out(&dry).contains("would remove 9"), "{}", out(&dry));
    assert_eq!(record_count(d.path()), 12, "dry run deletes nothing");

    let real = opshero(d.path(), &["gc", "--keep", "3"]);
    assert_eq!(code(&real), 0, "{}", err(&real));
    assert!(out(&real).contains("removed 9"), "{}", out(&real));
    assert_eq!(record_count(d.path()), 3);
    assert_eq!(
        code(&opshero(d.path(), &["verify"])),
        0,
        "evidence for the current content survives"
    );
}

#[test]
fn gc_never_deletes_evidence_bound_to_the_current_tree_even_when_it_is_old() {
    let d = new_repo();
    write_policy(d.path(), "");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    ); // on the current content
    for i in 0..6 {
        // History on other content, recorded after it.
        fs::write(d.path().join("code.txt"), format!("other{i}\n")).unwrap();
        assert_eq!(
            code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
            0
        );
    }
    fs::write(d.path().join("code.txt"), "v1\n").unwrap(); // back to the original content
    let g = opshero(d.path(), &["gc", "--keep", "2"]);
    assert_eq!(code(&g), 0, "{}", err(&g));
    assert_eq!(
        code(&opshero(d.path(), &["verify"])),
        0,
        "old evidence for the current content must survive: {}",
        out(&g)
    );
}

#[test]
fn gc_rejects_a_zero_keep_and_works_on_an_empty_store() {
    let d = new_repo();
    write_policy(d.path(), "");
    assert_eq!(code(&opshero(d.path(), &["gc", "--keep", "0"])), 2);
    let o = opshero(d.path(), &["gc"]);
    assert_eq!(code(&o), 0, "{}", err(&o));
    assert!(out(&o).contains("removed 0"), "{}", out(&o));
}

// ---------------------------------------------------------------------------
// Turn baseline and path scoping (found by the 30-session pilot, 2026-10-21): without them the
// Stop gate blocked 11 of 11 sessions where nothing changed and 5 of 5 documentation-only
// sessions, forcing the agent to run tests for nothing.
// ---------------------------------------------------------------------------

fn prompt_event(dir: &Path, session: &str) -> serde_json::Value {
    serde_json::json!({
        "hook_event_name": "UserPromptSubmit", "cwd": dir, "session_id": session, "prompt": "do something"
    })
}

fn blocked(o: &Output) -> bool {
    out(o).contains("\"decision\":\"block\"")
}

#[test]
fn a_turn_in_which_nothing_changed_is_never_blocked() {
    let d = new_repo();
    echo_policy(d.path());
    opshero(d.path(), &["journal", "enable"]);
    hook(d.path(), prompt_event(d.path(), "s"));
    let o = hook(d.path(), stop_event(d.path(), "s", false));
    assert!(
        !blocked(&o),
        "nothing changed, nothing to prove: {}",
        out(&o)
    );
    assert!(
        journal_text(d.path()).contains("\"verdict\":\"UNCHANGED\""),
        "{}",
        journal_text(d.path())
    );
}

#[test]
fn a_turn_that_changed_the_code_is_blocked_until_the_evidence_exists() {
    let d = new_repo();
    echo_policy(d.path());
    hook(d.path(), prompt_event(d.path(), "s"));
    fs::write(d.path().join("code.txt"), "changed during the turn\n").unwrap();
    assert!(blocked(&hook(d.path(), stop_event(d.path(), "s", false))));
    hook(d.path(), bash_ok_in(d.path(), "s", "mytests"));
    assert!(!blocked(&hook(d.path(), stop_event(d.path(), "s", false))));
}

#[test]
fn without_a_baseline_the_gate_keeps_its_previous_behaviour() {
    let d = new_repo();
    echo_policy(d.path());
    // No UserPromptSubmit event was seen: be strict rather than guess.
    assert!(blocked(&hook(d.path(), stop_event(d.path(), "s", false))));
}

#[test]
fn a_new_prompt_resets_the_baseline() {
    let d = new_repo();
    echo_policy(d.path());
    hook(d.path(), prompt_event(d.path(), "s"));
    fs::write(d.path().join("code.txt"), "turn one edit\n").unwrap();
    assert!(blocked(&hook(d.path(), stop_event(d.path(), "s", false))));
    // The user sends a second prompt; the agent changes nothing this time.
    hook(d.path(), prompt_event(d.path(), "s"));
    assert!(
        !blocked(&hook(d.path(), stop_event(d.path(), "s", false))),
        "the old edit belongs to the previous turn"
    );
}

#[test]
fn baselines_of_different_sessions_do_not_mix() {
    let d = new_repo();
    echo_policy(d.path());
    hook(d.path(), prompt_event(d.path(), "A"));
    fs::write(d.path().join("code.txt"), "edit by A\n").unwrap();
    hook(d.path(), prompt_event(d.path(), "B")); // B starts after A's edit: nothing changes for B
    assert!(!blocked(&hook(d.path(), stop_event(d.path(), "B", false))));
    assert!(blocked(&hook(d.path(), stop_event(d.path(), "A", false))));
}

#[test]
fn paths_scope_a_requirement_to_the_files_that_changed_this_turn() {
    let d = new_repo();
    write_policy(d.path(), "paths = [\"*.rs\"]\n"); // the requirement concerns Rust sources only
    hook(d.path(), prompt_event(d.path(), "s"));
    fs::write(d.path().join("README.md"), "docs only\n").unwrap();
    let docs = hook(d.path(), stop_event(d.path(), "s", false));
    assert!(
        !blocked(&docs),
        "a documentation change does not need the tests: {}",
        out(&docs)
    );

    fs::write(d.path().join("lib.rs"), "fn main() {}\n").unwrap();
    assert!(
        blocked(&hook(d.path(), stop_event(d.path(), "s", false))),
        "a source change does"
    );
}

#[test]
fn the_not_applicable_decision_is_journaled_distinctly() {
    let d = new_repo();
    write_policy(d.path(), "paths = [\"*.rs\"]\n");
    opshero(d.path(), &["journal", "enable"]);
    hook(d.path(), prompt_event(d.path(), "s"));
    fs::write(d.path().join("README.md"), "docs only\n").unwrap();
    hook(d.path(), stop_event(d.path(), "s", false));
    assert!(
        journal_text(d.path()).contains("\"verdict\":\"NOT-APPLICABLE\""),
        "{}",
        journal_text(d.path())
    );
}

#[test]
fn verify_ignores_paths_and_demands_every_requirement() {
    // `verify` is the CI semantics: no baseline, everything applies.
    let d = new_repo();
    write_policy(d.path(), "paths = [\"*.rs\"]\n");
    let v = opshero(d.path(), &["verify"]);
    assert_eq!(code(&v), 11, "{}", out(&v));
}

#[test]
fn enforce_stop_installs_the_prompt_hook_and_uninstall_removes_it() {
    let d = new_repo();
    assert_eq!(
        code(&opshero(
            d.path(),
            &["hook", "install", "--enforce-stop", "--yes"]
        )),
        0
    );
    let settings: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(d.path().join(".claude/settings.local.json")).unwrap(),
    )
    .unwrap();
    assert!(
        settings["hooks"]["UserPromptSubmit"].is_array(),
        "{settings}"
    );
    assert_eq!(code(&opshero(d.path(), &["hook", "uninstall", "--yes"])), 0);
    let after = fs::read_to_string(d.path().join(".claude/settings.local.json")).unwrap();
    assert!(!after.contains("UserPromptSubmit"), "{after}");
}

// ---------------------------------------------------------------------------
// Evidence transport (RFC-0002 OQ1) and `verify --ci`: publish evidence for the current
// tree to a dedicated git ref, fetch it in CI, verify it. Real bare remote, real clones.
// ---------------------------------------------------------------------------

fn git_out(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .output()
        .unwrap();
    String::from_utf8_lossy(&o.stdout).trim().to_owned()
}

/// A developer repository with a committed policy, a bare remote named `origin`, and the
/// code committed and pushed. Returns (developer repo, remote).
fn dev_with_remote() -> (TempDir, TempDir) {
    let dev = new_repo();
    write_policy(dev.path(), "");
    git(dev.path(), &["add", "-A"]);
    git(dev.path(), &["commit", "-q", "-m", "policy"]);
    let remote = TempDir::new().unwrap();
    git(remote.path(), &["init", "-q", "--bare", "-b", "main"]);
    git(
        dev.path(),
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    git(dev.path(), &["push", "-q", "origin", "main"]);
    (dev, remote)
}

fn clone_of(remote: &Path) -> TempDir {
    let c = TempDir::new().unwrap();
    let ok = Command::new("git")
        .args(["clone", "-q"])
        .arg(remote)
        .arg(c.path())
        .status()
        .unwrap()
        .success();
    assert!(ok, "clone failed");
    c
}

fn subject_tree(dir: &Path) -> String {
    let v = opshero(dir, &["verify", "--format", "json"]);
    let j: serde_json::Value = serde_json::from_str(&out(&v)).unwrap();
    j["subject"]["tree"].as_str().unwrap().to_owned()
}

fn evidence_ref(tree: &str) -> String {
    format!("refs/opshero/evidence/{tree}")
}

#[test]
fn push_publishes_only_the_records_bound_to_the_current_tree() {
    let (dev, remote) = dev_with_remote();
    assert_eq!(
        code(&opshero(dev.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    ); // tree T1
    let t1 = subject_tree(dev.path());
    fs::write(dev.path().join("code.txt"), "v2\n").unwrap(); // uncommitted change: tree T2
    assert_eq!(
        code(&opshero(dev.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    let t2 = subject_tree(dev.path());
    assert_ne!(t1, t2);

    let p = opshero(dev.path(), &["evidence", "push"]);
    assert_eq!(code(&p), 0, "{}{}", out(&p), err(&p));
    assert!(out(&p).contains("1 record"), "{}", out(&p));
    assert!(
        out(&p).contains("command line"),
        "must say that command lines are published: {}",
        out(&p)
    );

    let files = git_out(
        remote.path(),
        &["ls-tree", "-r", "--name-only", &evidence_ref(&t2)],
    );
    assert_eq!(files.lines().count(), 1, "{files}");
    assert!(files.lines().all(|l| l.ends_with(".json")));
    let old = Command::new("git")
        .arg("-C")
        .arg(remote.path())
        .args(["rev-parse", "--verify", "-q", &evidence_ref(&t1)])
        .output()
        .unwrap();
    assert!(
        !old.status.success(),
        "history for other trees must not be published"
    );
}

#[test]
fn push_is_idempotent_and_merges_with_what_is_already_published() {
    let (dev, remote) = dev_with_remote();
    assert_eq!(
        code(&opshero(dev.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    let t = subject_tree(dev.path());
    assert_eq!(code(&opshero(dev.path(), &["evidence", "push"])), 0);
    let first = git_out(remote.path(), &["rev-parse", &evidence_ref(&t)]);
    assert_eq!(code(&opshero(dev.path(), &["evidence", "push"])), 0);
    assert_eq!(
        git_out(remote.path(), &["rev-parse", &evidence_ref(&t)]),
        first,
        "same records, same commit"
    );

    assert_eq!(
        code(&opshero(dev.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    ); // a second run, same tree
    assert_eq!(code(&opshero(dev.path(), &["evidence", "push"])), 0);
    let files = git_out(
        remote.path(),
        &["ls-tree", "-r", "--name-only", &evidence_ref(&t)],
    );
    assert_eq!(files.lines().count(), 2, "{files}");

    // Another machine published a different record for the same tree: it must survive our push.
    let other = clone_of(remote.path());
    git(other.path(), &["config", "user.email", "o@e.c"]);
    // A different command line: two records with identical content (same second, same
    // duration, same chain position) are the same record by construction.
    assert_eq!(
        code(&opshero(
            other.path(),
            &["run", "--", "sh", "-c", "exit 0 # second machine"]
        )),
        0
    );
    assert_eq!(code(&opshero(other.path(), &["evidence", "push"])), 0);
    let files = git_out(
        remote.path(),
        &["ls-tree", "-r", "--name-only", &evidence_ref(&t)],
    );
    assert_eq!(
        files.lines().count(),
        3,
        "union of both machines' evidence: {files}"
    );
}

#[test]
fn ci_in_a_fresh_clone_passes_with_the_published_evidence() {
    let (dev, remote) = dev_with_remote();
    assert_eq!(
        code(&opshero(dev.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    assert_eq!(code(&opshero(dev.path(), &["evidence", "push"])), 0);

    let ci = clone_of(remote.path());
    assert_eq!(
        subject_tree(ci.path()),
        subject_tree(dev.path()),
        "same content, same tree id"
    );
    let v = opshero(ci.path(), &["verify", "--ci"]);
    assert_eq!(code(&v), 0, "{}{}", out(&v), err(&v));
    assert!(out(&v).contains("PASS"), "{}", out(&v));
    assert!(out(&v).contains("imported 1 record"), "{}", out(&v));
}

#[test]
fn ci_fails_closed_when_nothing_was_published_for_this_tree() {
    let (_dev, remote) = dev_with_remote();
    let ci = clone_of(remote.path());
    let v = opshero(ci.path(), &["verify", "--ci"]);
    assert_eq!(
        code(&v),
        11,
        "no evidence must not pass: {}{}",
        out(&v),
        err(&v)
    );
    assert!(out(&v).contains("no evidence published"), "{}", out(&v));
}

#[test]
fn evidence_for_an_older_tree_does_not_vouch_for_a_newer_one() {
    let (dev, remote) = dev_with_remote();
    assert_eq!(
        code(&opshero(dev.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    assert_eq!(code(&opshero(dev.path(), &["evidence", "push"])), 0);
    // A later commit changes the code; nobody re-ran the tests on it.
    fs::write(dev.path().join("code.txt"), "v2 after the tests\n").unwrap();
    git(dev.path(), &["add", "-A"]);
    git(
        dev.path(),
        &["commit", "-q", "-m", "change after the tests"],
    );
    git(dev.path(), &["push", "-q", "origin", "main"]);
    let ci = clone_of(remote.path());
    let v = opshero(ci.path(), &["verify", "--ci"]);
    assert_eq!(code(&v), 11, "{}", out(&v));
}

#[test]
fn a_forged_published_record_is_rejected_by_its_hash() {
    let (dev, remote) = dev_with_remote();
    assert_eq!(
        code(&opshero(dev.path(), &["run", "--", "sh", "-c", "exit 1"])),
        1
    ); // the tests FAIL
    let t = subject_tree(dev.path());
    let rec_file = fs::read_dir(dev.path().join(".git/opshero/records"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let name = rec_file.file_name().unwrap().to_str().unwrap().to_owned();
    // Forge: flip the exit code to 0 but keep the id (the hash no longer matches).
    let forged = fs::read_to_string(&rec_file)
        .unwrap()
        .replace("\"exit_code\":1", "\"exit_code\":0");
    assert_ne!(forged, fs::read_to_string(&rec_file).unwrap());
    let blob = {
        let mut c = Command::new("git")
            .arg("-C")
            .arg(dev.path())
            .args(["hash-object", "-w", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        c.stdin
            .take()
            .unwrap()
            .write_all(forged.as_bytes())
            .unwrap();
        String::from_utf8_lossy(&c.wait_with_output().unwrap().stdout)
            .trim()
            .to_owned()
    };
    let tree_obj = {
        let mut c = Command::new("git")
            .arg("-C")
            .arg(dev.path())
            .arg("mktree")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        c.stdin
            .take()
            .unwrap()
            .write_all(format!("100644 blob {blob}\t{name}\n").as_bytes())
            .unwrap();
        String::from_utf8_lossy(&c.wait_with_output().unwrap().stdout)
            .trim()
            .to_owned()
    };
    let commit = git_out(dev.path(), &["commit-tree", &tree_obj, "-m", "forged"]);
    git(dev.path(), &["update-ref", &evidence_ref(&t), &commit]);
    git(
        dev.path(),
        &[
            "push",
            "-q",
            "origin",
            &format!("+{}:{}", evidence_ref(&t), evidence_ref(&t)),
        ],
    );

    let ci = clone_of(remote.path());
    let v = opshero(ci.path(), &["verify", "--ci"]);
    assert_eq!(code(&v), 11, "a forged pass must not verify: {}", out(&v));
    assert!(
        out(&v).contains("id mismatch"),
        "the rejection must be reported: {}",
        out(&v)
    );
    assert!(!out(&v).contains("PASS"), "{}", out(&v));
    // The forged file must not even enter the local store (the loader would ignore it, but
    // it would stay there for good): the check happens at import time too.
    assert_eq!(
        record_count(ci.path()),
        0,
        "a forged record must not be stored"
    );
    let imported = fs::read_dir(ci.path().join(".git/opshero/imported"))
        .map(|d| d.count())
        .unwrap_or(0);
    assert_eq!(
        imported, 0,
        "a forged record must not enter the imported store either"
    );
}

#[test]
fn an_unreachable_remote_fails_closed_with_a_clear_message() {
    let (dev, _remote) = dev_with_remote();
    let v = opshero(
        dev.path(),
        &["verify", "--ci", "--remote", "does-not-exist"],
    );
    assert_eq!(code(&v), 11, "{}{}", out(&v), err(&v));
    assert!(out(&v).contains("could not fetch"), "{}", out(&v));
}

#[test]
fn push_without_a_remote_is_a_usage_error_and_remote_needs_ci() {
    let d = new_repo();
    write_policy(d.path(), "");
    assert_eq!(
        code(&opshero(d.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    let p = opshero(d.path(), &["evidence", "push"]);
    assert_eq!(code(&p), 2, "{}{}", out(&p), err(&p));
    assert!(err(&p).contains("remote"), "{}", err(&p));
    assert_eq!(
        code(&opshero(d.path(), &["verify", "--remote", "origin"])),
        2,
        "--remote only makes sense with --ci"
    );
}

#[test]
fn plain_verify_never_fetches_anything() {
    let (dev, remote) = dev_with_remote();
    assert_eq!(
        code(&opshero(dev.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    assert_eq!(code(&opshero(dev.path(), &["evidence", "push"])), 0);
    let ci = clone_of(remote.path());
    let v = opshero(ci.path(), &["verify"]); // no --ci: local evidence only
    assert_eq!(code(&v), 11, "{}", out(&v));
    assert_eq!(
        git_out(ci.path(), &["for-each-ref", "refs/opshero"]),
        "",
        "nothing fetched"
    );
}

// ---------------------------------------------------------------------------
// Assurance A2: the CI executes the required commands itself and records them.
// ---------------------------------------------------------------------------

/// Run opshero as a CI job would: the `CI` environment variable is set.
fn opshero_ci(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opshero"))
        .current_dir(dir)
        .env("CI", "true")
        .args(args)
        .output()
        .unwrap()
}

fn ci_policy(dir: &Path, command: &str, extra: &str) {
    fs::create_dir_all(dir.join(".opshero")).unwrap();
    fs::write(
        dir.join(".opshero/policy.toml"),
        format!("version = 1\nid = \"ci\"\n[[require]]\nid = \"tests\"\ncheck = \"evidence.fresh\"\ncommand = {command}\n{extra}"),
    )
    .unwrap();
}

fn only_record(dir: &Path) -> serde_json::Value {
    let recs = read_records(dir);
    assert_eq!(
        recs.len(),
        1,
        "expected exactly one record, got {}",
        recs.len()
    );
    recs.into_iter().next().unwrap()
}

#[test]
fn ci_run_executes_the_required_command_and_records_it_as_a2() {
    let d = new_repo();
    fs::write(d.path().join("mytests.sh"), "echo ci-tests-ran\nexit 0\n").unwrap();
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "tests"]);
    ci_policy(d.path(), "[\"sh\", \"mytests.sh\"]", "");
    let o = opshero_ci(d.path(), &["ci", "run"]);
    assert_eq!(code(&o), 0, "{}{}", out(&o), err(&o));
    assert!(
        out(&o).contains("ci-tests-ran"),
        "the command output must reach the CI log: {}",
        out(&o)
    );
    assert!(out(&o).contains("PASS"), "{}", out(&o));
    assert!(out(&o).contains("A2"), "{}", out(&o));
    let r = only_record(d.path());
    assert_eq!(r["recorder"]["assurance"], "A2");
    assert_eq!(r["recorder"]["adapter"], "ci");
    assert_eq!(r["execution"]["exit_code"], 0);
}

#[test]
fn ci_run_reports_fail_and_still_runs_the_other_requirements() {
    let d = new_repo();
    fs::write(d.path().join("bad.sh"), "exit 3\n").unwrap();
    fs::write(d.path().join("good.sh"), "echo ran > ignored.txt\n").unwrap(); // marker in a git-ignored file
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "scripts"]);
    fs::create_dir_all(d.path().join(".opshero")).unwrap();
    fs::write(
        d.path().join(".opshero/policy.toml"),
        "version = 1\nid = \"ci\"\n[[require]]\nid = \"first\"\ncheck = \"evidence.fresh\"\ncommand = [\"sh\", \"bad.sh\"]\n[[require]]\nid = \"second\"\ncheck = \"evidence.fresh\"\ncommand = [\"sh\", \"good.sh\"]\n",
    )
    .unwrap();
    let o = opshero_ci(d.path(), &["ci", "run"]);
    assert_eq!(code(&o), 10, "{}{}", out(&o), err(&o));
    assert!(out(&o).contains("FAIL"), "{}", out(&o));
    assert!(
        d.path().join("ignored.txt").exists(),
        "a failing requirement must not stop the others"
    );
    assert_eq!(read_records(d.path()).len(), 2);
}

#[test]
fn a_wildcard_requirement_needs_a_run_command_and_run_must_match_it() {
    let d = new_repo();
    fs::write(d.path().join("t.sh"), "exit 0\n").unwrap();
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "t"]);
    ci_policy(d.path(), "[\"sh\", \"*\"]", "");
    let o = opshero_ci(d.path(), &["ci", "run"]);
    assert_eq!(code(&o), 2, "{}{}", out(&o), err(&o));
    assert!(
        err(&o).contains("run"),
        "must tell how to fix it: {}",
        err(&o)
    );
    assert_eq!(read_records(d.path()).len(), 0, "nothing was executed");

    ci_policy(d.path(), "[\"sh\", \"*\"]", "run = [\"sh\", \"t.sh\"]\n");
    assert_eq!(code(&opshero_ci(d.path(), &["ci", "run"])), 0);

    ci_policy(d.path(), "[\"sh\", \"*\"]", "run = [\"bash\", \"t.sh\"]\n"); // does not match the pattern
    assert_eq!(code(&opshero_ci(d.path(), &["ci", "run"])), 2);
}

#[test]
fn a_policy_that_needs_a2_rejects_local_a1_evidence_until_the_ci_runs_it() {
    let d = new_repo();
    fs::write(d.path().join("t.sh"), "exit 0\n").unwrap();
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "t"]);
    ci_policy(d.path(), "[\"sh\", \"t.sh\"]", "min_assurance = \"A2\"\n");
    assert_eq!(code(&opshero(d.path(), &["run", "--", "sh", "t.sh"])), 0); // a developer's local run: A1
    let v = opshero(d.path(), &["verify"]);
    assert_eq!(
        code(&v),
        11,
        "A1 must not satisfy an A2 requirement: {}",
        out(&v)
    );
    assert!(out(&v).contains("below the required A2"), "{}", out(&v));
    let c = opshero_ci(d.path(), &["ci", "run"]);
    assert_eq!(code(&c), 0, "{}{}", out(&c), err(&c));
}

#[test]
fn published_evidence_can_never_claim_a2() {
    // A dishonest publisher writes a perfectly well-formed record that claims assurance A2.
    let (dev, remote) = dev_with_remote();
    fs::write(dev.path().join("t.sh"), "exit 0\n").unwrap();
    ci_policy(dev.path(), "[\"sh\", \"t.sh\"]", "min_assurance = \"A2\"\n");
    git(dev.path(), &["add", "-A"]);
    git(dev.path(), &["commit", "-q", "-m", "ci policy"]);
    git(dev.path(), &["push", "-q", "origin", "main"]);
    assert_eq!(code(&opshero(dev.path(), &["run", "--", "sh", "t.sh"])), 0);
    let t = subject_tree(dev.path());
    let mut rec: serde_json::Value = only_record(dev.path());
    rec.as_object_mut().unwrap().remove("id");
    let mut body: opshero_core::record::RecordBody = serde_json::from_value(rec).unwrap();
    body.recorder.assurance = "A2".into(); // the lie
    let forged = opshero_core::record::new_record(body).unwrap(); // valid id for the lie
    let json = serde_json::to_string(&forged).unwrap();
    let blob = {
        let mut c = Command::new("git")
            .arg("-C")
            .arg(dev.path())
            .args(["hash-object", "-w", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        c.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
        String::from_utf8_lossy(&c.wait_with_output().unwrap().stdout)
            .trim()
            .to_owned()
    };
    let tree_obj = {
        let mut c = Command::new("git")
            .arg("-C")
            .arg(dev.path())
            .arg("mktree")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        c.stdin
            .take()
            .unwrap()
            .write_all(format!("100644 blob {blob}\t{}.json\n", forged.id).as_bytes())
            .unwrap();
        String::from_utf8_lossy(&c.wait_with_output().unwrap().stdout)
            .trim()
            .to_owned()
    };
    let commit = git_out(dev.path(), &["commit-tree", &tree_obj, "-m", "lie"]);
    git(dev.path(), &["update-ref", &evidence_ref(&t), &commit]);
    git(
        dev.path(),
        &[
            "push",
            "-q",
            "origin",
            &format!("+{}:{}", evidence_ref(&t), evidence_ref(&t)),
        ],
    );

    let ci = clone_of(remote.path());
    let v = opshero(ci.path(), &["verify", "--ci"]);
    assert_eq!(
        code(&v),
        11,
        "a published record must never count as A2: {}{}",
        out(&v),
        err(&v)
    );
    assert!(out(&v).contains("below the required A2"), "{}", out(&v));
    assert!(out(&v).contains("imported 1 record"), "{}", out(&v));
}

#[test]
fn a_command_that_changes_the_tree_fails_under_the_default_rule() {
    let d = new_repo();
    fs::write(d.path().join("t.sh"), "echo made > generated.txt\n").unwrap(); // untracked, not ignored
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "t"]);
    ci_policy(d.path(), "[\"sh\", \"t.sh\"]", "");
    let o = opshero_ci(d.path(), &["ci", "run"]);
    assert_eq!(code(&o), 10, "{}{}", out(&o), err(&o));
    assert!(
        out(&o).contains("working tree changed while it ran"),
        "{}",
        out(&o)
    );
}

#[test]
fn ci_run_can_use_a_policy_from_outside_the_working_tree() {
    // The trusted policy (for example from the base branch) must win over the one in the pull request.
    let d = new_repo();
    fs::write(d.path().join("ok.sh"), "exit 0\n").unwrap();
    fs::write(d.path().join("bad.sh"), "exit 1\n").unwrap();
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "scripts"]);
    ci_policy(d.path(), "[\"sh\", \"bad.sh\"]", ""); // the weakened/changed policy in the PR
    let trusted = TempDir::new().unwrap();
    let trusted_policy = trusted.path().join("policy.toml");
    fs::write(
        &trusted_policy,
        "version = 1\nid = \"trusted\"\n[[require]]\nid = \"tests\"\ncheck = \"evidence.fresh\"\ncommand = [\"sh\", \"ok.sh\"]\n",
    )
    .unwrap();
    let o = opshero_ci(
        d.path(),
        &["ci", "run", "--policy", trusted_policy.to_str().unwrap()],
    );
    assert_eq!(code(&o), 0, "{}{}", out(&o), err(&o));
    assert!(out(&o).contains("trusted"), "{}", out(&o));
}

#[test]
fn ci_run_without_a_policy_is_a_usage_error() {
    let d = new_repo();
    let o = opshero_ci(d.path(), &["ci", "run"]);
    assert_eq!(code(&o), 2, "{}{}", out(&o), err(&o));
}

#[test]
fn ci_run_refuses_to_run_outside_ci_unless_told_so() {
    let d = new_repo();
    fs::write(d.path().join("t.sh"), "exit 0\n").unwrap();
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "t"]);
    ci_policy(d.path(), "[\"sh\", \"t.sh\"]", "");
    // No CI variable: an A2 claim made on a developer's machine would be worth nothing.
    let o = Command::new(env!("CARGO_BIN_EXE_opshero"))
        .current_dir(d.path())
        .env_remove("CI")
        .args(["ci", "run"])
        .output()
        .unwrap();
    assert_eq!(code(&o), 2, "{}{}", out(&o), err(&o));
    assert!(err(&o).contains("--allow-local"), "{}", err(&o));
    assert_eq!(read_records(d.path()).len(), 0, "nothing was executed");
    let ok = Command::new(env!("CARGO_BIN_EXE_opshero"))
        .current_dir(d.path())
        .env_remove("CI")
        .args(["ci", "run", "--allow-local"])
        .output()
        .unwrap();
    assert_eq!(code(&ok), 0, "{}{}", out(&ok), err(&ok));
}

#[test]
fn imported_records_stay_capped_at_a1_in_every_later_command() {
    // On a runner that keeps its checkout between jobs, a record fetched earlier must not
    // turn into A2 for a later `verify` or `ci run`.
    let (dev, remote) = dev_with_remote();
    fs::write(dev.path().join("t.sh"), "exit 0\n").unwrap();
    ci_policy(dev.path(), "[\"sh\", \"t.sh\"]", "min_assurance = \"A2\"\n");
    git(dev.path(), &["add", "-A"]);
    git(dev.path(), &["commit", "-q", "-m", "policy"]);
    git(dev.path(), &["push", "-q", "origin", "main"]);
    assert_eq!(code(&opshero(dev.path(), &["run", "--", "sh", "t.sh"])), 0);
    let t = subject_tree(dev.path());
    let mut rec = only_record(dev.path());
    rec.as_object_mut().unwrap().remove("id");
    let mut body: opshero_core::record::RecordBody = serde_json::from_value(rec).unwrap();
    body.recorder.assurance = "A2".into();
    let forged = opshero_core::record::new_record(body).unwrap();
    let json = serde_json::to_string(&forged).unwrap();
    let blob = {
        let mut c = Command::new("git")
            .arg("-C")
            .arg(dev.path())
            .args(["hash-object", "-w", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        c.stdin.take().unwrap().write_all(json.as_bytes()).unwrap();
        String::from_utf8_lossy(&c.wait_with_output().unwrap().stdout)
            .trim()
            .to_owned()
    };
    let tree_obj = {
        let mut c = Command::new("git")
            .arg("-C")
            .arg(dev.path())
            .arg("mktree")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        c.stdin
            .take()
            .unwrap()
            .write_all(format!("100644 blob {blob}\t{}.json\n", forged.id).as_bytes())
            .unwrap();
        String::from_utf8_lossy(&c.wait_with_output().unwrap().stdout)
            .trim()
            .to_owned()
    };
    let commit = git_out(dev.path(), &["commit-tree", &tree_obj, "-m", "lie"]);
    git(dev.path(), &["update-ref", &evidence_ref(&t), &commit]);
    git(
        dev.path(),
        &[
            "push",
            "-q",
            "origin",
            &format!("+{}:{}", evidence_ref(&t), evidence_ref(&t)),
        ],
    );

    let ci = clone_of(remote.path());
    assert_eq!(code(&opshero(ci.path(), &["evidence", "fetch"])), 0); // the import happens here
    let later = opshero(ci.path(), &["verify"]); // a different command, later, no --ci
    assert_eq!(
        code(&later),
        11,
        "the imported claim must still be capped: {}",
        out(&later)
    );
    assert!(
        out(&later).contains("below the required A2"),
        "{}",
        out(&later)
    );
}

// ---------------------------------------------------------------------------
// Hardening after the automated security reviews (2026-10-21).
// ---------------------------------------------------------------------------

#[test]
fn a_command_run_outside_the_repository_is_never_recorded_as_a_run_inside_it() {
    let d = new_repo();
    echo_policy(d.path());
    let outside = TempDir::new().unwrap();
    let cmd = format!("cd {} && mytests", outside.path().display());
    hook(d.path(), bash_ok(d.path(), &cmd));
    hook(d.path(), bash_ok(d.path(), "cd .. && mytests"));
    assert_eq!(
        record_count(d.path()),
        0,
        "a run in another directory is not evidence for this repository"
    );

    // Inside the repository it is fine, and the directory is recorded.
    fs::create_dir_all(d.path().join("sub")).unwrap();
    hook(d.path(), bash_ok(d.path(), "cd sub && mytests"));
    assert_eq!(record_count(d.path()), 1);
    let recs = read_records(d.path());
    assert_eq!(recs[0]["command"]["cwd"], "sub");
}

#[test]
fn an_oversized_published_blob_is_skipped_without_being_read() {
    let (dev, remote) = dev_with_remote();
    assert_eq!(
        code(&opshero(dev.path(), &["run", "--", "sh", "-c", "exit 0"])),
        0
    );
    let t = subject_tree(dev.path());
    let big = "x".repeat(2 * 1024 * 1024);
    let blob = {
        let mut c = Command::new("git")
            .arg("-C")
            .arg(dev.path())
            .args(["hash-object", "-w", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        c.stdin.take().unwrap().write_all(big.as_bytes()).unwrap();
        String::from_utf8_lossy(&c.wait_with_output().unwrap().stdout)
            .trim()
            .to_owned()
    };
    let name = format!("{}.json", "a".repeat(64));
    let tree_obj = {
        let mut c = Command::new("git")
            .arg("-C")
            .arg(dev.path())
            .arg("mktree")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        c.stdin
            .take()
            .unwrap()
            .write_all(format!("100644 blob {blob}\t{name}\n").as_bytes())
            .unwrap();
        String::from_utf8_lossy(&c.wait_with_output().unwrap().stdout)
            .trim()
            .to_owned()
    };
    let commit = git_out(dev.path(), &["commit-tree", &tree_obj, "-m", "big"]);
    git(dev.path(), &["update-ref", &evidence_ref(&t), &commit]);
    git(
        dev.path(),
        &[
            "push",
            "-q",
            "origin",
            &format!("+{}:{}", evidence_ref(&t), evidence_ref(&t)),
        ],
    );

    let ci = clone_of(remote.path());
    let v = opshero(ci.path(), &["verify", "--ci"]);
    assert_eq!(code(&v), 11, "{}{}", out(&v), err(&v));
    assert!(
        out(&v).contains("larger than"),
        "the skip must be reported: {}",
        out(&v)
    );
}

#[test]
fn ci_commands_warn_when_the_policy_comes_from_the_checked_out_content() {
    let d = new_repo();
    fs::write(d.path().join("t.sh"), "exit 0\n").unwrap();
    git(d.path(), &["add", "-A"]);
    git(d.path(), &["commit", "-q", "-m", "t"]);
    ci_policy(d.path(), "[\"sh\", \"t.sh\"]", "");
    let inside = opshero_ci(d.path(), &["ci", "run"]);
    assert!(
        err(&inside).contains("trusted"),
        "must warn that this policy is not trusted: {}",
        err(&inside)
    );

    let trusted = TempDir::new().unwrap();
    let p = trusted.path().join("policy.toml");
    fs::write(&p, "version = 1\nid = \"t\"\n[[require]]\nid = \"tests\"\ncheck = \"evidence.fresh\"\ncommand = [\"sh\", \"t.sh\"]\n").unwrap();
    let outside = opshero_ci(d.path(), &["ci", "run", "--policy", p.to_str().unwrap()]);
    assert!(
        !err(&outside).contains("trusted"),
        "no warning for a policy from outside: {}",
        err(&outside)
    );

    let v = opshero(d.path(), &["verify", "--ci"]);
    assert!(err(&v).contains("trusted"), "verify --ci too: {}", err(&v));
}
