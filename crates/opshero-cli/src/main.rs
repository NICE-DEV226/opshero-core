//! opshero command-line adapter around the pure core (RFC-0003).
//!
//! Exit codes (public API): 0 PASS, 1 internal error, 2 usage error,
//! 10 FAIL, 11 UNKNOWN. `run` passes the wrapped command's exit code through.
//! `hook claude-code` always exits 0: a recording problem must never break the agent.

mod baseline;
mod claude;
mod evidence;
mod git;
mod install;
mod journal;
mod lock;
mod ops;
mod store;

use clap::{Parser, Subcommand, ValueEnum};
use opshero_core::policy;
use opshero_core::render;
use opshero_core::verify::Verdict;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command as Proc, ExitCode};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const EXIT_INTERNAL: u8 = 1;
const EXIT_USAGE: u8 = 2;
const EXIT_FAIL: u8 = 10;
const EXIT_UNKNOWN: u8 = 11;
/// Upper bound on a hook payload: hostile or runaway input must not exhaust memory.
const MAX_HOOK_INPUT: u64 = 4 * 1024 * 1024;

const SAMPLE_POLICY: &str = r#"# opshero policy (v0) — review it like code.
version = 1
id = "default"

[[require]]
id = "tests-on-final-tree"
check = "evidence.fresh"
# Replace with your real test command. A trailing "*" matches any extra arguments.
command = ["pytest", "*"]
min_assurance = "A1"   # A1 = recorded locally (advisory); A2/A3 need CI evidence
severity = "block"     # or "warn"
"#;

#[derive(Parser)]
#[command(
    name = "opshero",
    version,
    about = "Deterministic evidence that your checks ran on the exact content you are about to merge"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Write a sample policy to .opshero/policy.toml
    Init {
        /// Overwrite an existing policy file
        #[arg(long)]
        force: bool,
    },
    /// Run a command and record it as evidence (exit code is passed through)
    Run {
        /// Run the command several times and record every run: a count (1..=1000) or `auto`
        /// (the number required by the matching policy requirement's `repeat` rule).
        /// The exit code is that of the last failing run, or 0 if every run passed.
        #[arg(long, value_name = "N|auto")]
        repeat: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true, num_args = 1..)]
        cmd: Vec<String>,
    },
    /// Evaluate the policy against the recorded evidence for the current tree
    Verify {
        /// Policy file (relative paths are resolved from the repository root)
        #[arg(long, default_value = ".opshero/policy.toml")]
        policy: PathBuf,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// CI mode: first fetch the evidence published for this exact content from the remote
        /// (see `opshero evidence push`). Anything missing or unreadable makes the verdict UNKNOWN.
        #[arg(long)]
        ci: bool,
        /// Remote holding the published evidence (with --ci)
        #[arg(long, requires = "ci", default_value = "origin")]
        remote: String,
    },
    /// Commands for the CI job (assurance A2)
    Ci {
        #[command(subcommand)]
        cmd: CiCmd,
    },
    /// Publish or fetch evidence through a dedicated git ref
    Evidence {
        #[command(subcommand)]
        cmd: EvidenceCmd,
    },
    /// Agent integrations
    Hook {
        #[command(subcommand)]
        cmd: HookCmd,
    },
    /// Delete old evidence, keeping the newest records and everything bound to the current content
    Gc {
        /// How many of the newest records to keep (the evidence for the current content is kept as well)
        #[arg(long, default_value_t = 2000, value_parser = clap::value_parser!(u32).range(1..))]
        keep: u32,
        /// Only report what would be removed
        #[arg(long)]
        dry_run: bool,
    },
    /// Opt-in local journal for the dogfooding experiment (no commands, paths or ids are stored)
    Journal {
        #[command(subcommand)]
        cmd: JournalCmd,
    },
}

#[derive(Subcommand)]
enum CiCmd {
    /// Execute the commands the policy requires, record them as assurance A2, and give the verdict.
    /// Use the policy of the target branch (--policy), not the one in the pull request.
    Run {
        /// Policy file to enforce (relative paths are resolved from the repository root)
        #[arg(long)]
        policy: Option<PathBuf>,
        /// Run even when the CI environment variable is not set (for testing only)
        #[arg(long)]
        allow_local: bool,
    },
}

#[derive(Subcommand)]
enum EvidenceCmd {
    /// Publish the records bound to the current content to refs/opshero/evidence/<tree> on the remote
    Push {
        #[arg(long, default_value = "origin")]
        remote: String,
    },
    /// Fetch the evidence published for the current content and add it to the local store
    Fetch {
        #[arg(long, default_value = "origin")]
        remote: String,
    },
}

#[derive(Subcommand)]
enum JournalCmd {
    /// Start recording Stop-gate decisions and recorded runs in `.git/opshero/journal.jsonl`
    Enable,
    /// Stop recording (the journal is kept unless --delete)
    Disable {
        #[arg(long)]
        delete: bool,
    },
    Status,
    /// List the blocking Stop decisions with their line numbers
    List,
    /// Say whether a blocking Stop (by line number from `list`) was a real problem or noise
    Label {
        line: usize,
        #[arg(value_enum)]
        label: LabelArg,
    },
    /// Summary against the pre-registered threshold (at least 10 % of at least 100 sessions)
    Report {
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum LabelArg {
    Real,
    Noise,
}

#[derive(Subcommand)]
enum HookCmd {
    /// Event handler invoked BY Claude Code (reads one JSON event on stdin)
    ClaudeCode,
    /// Register the hooks in an agent's settings file (dry run unless --yes)
    Install {
        #[arg(long, value_enum, default_value_t = Agent::ClaudeCode)]
        agent: Agent,
        /// local = .claude/settings.local.json (personal, not committed); project = .claude/settings.json; user = ~/.claude/settings.json
        #[arg(long, value_enum, default_value_t = Scope::Local)]
        scope: Scope,
        /// Also block the agent from finishing while required evidence is missing or stale
        #[arg(long)]
        enforce_stop: bool,
        /// Apply the change (without it, only show what would change)
        #[arg(long)]
        yes: bool,
    },
    /// Remove the hooks this tool registered (others are left alone)
    Uninstall {
        #[arg(long, value_enum, default_value_t = Agent::ClaudeCode)]
        agent: Agent,
        #[arg(long, value_enum, default_value_t = Scope::Local)]
        scope: Scope,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum Agent {
    ClaudeCode,
}

#[derive(Clone, Copy, ValueEnum)]
enum Scope {
    Local,
    Project,
    User,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Init { force } => init(force),
        Cmd::Run { repeat, cmd } => run(repeat.as_deref(), &cmd),
        Cmd::Verify {
            policy,
            format,
            ci,
            remote,
        } => verify_cmd(&policy, format, ci.then_some(remote.as_str())),
        Cmd::Evidence { cmd } => evidence_cmd(cmd),
        Cmd::Ci { cmd } => match cmd {
            CiCmd::Run {
                policy,
                allow_local,
            } => ci_run(policy.as_deref(), allow_local),
        },
        Cmd::Journal { cmd } => journal_cmd(cmd),
        Cmd::Gc { keep, dry_run } => gc_cmd(keep as usize, dry_run),
        Cmd::Hook { cmd } => match cmd {
            HookCmd::ClaudeCode => Ok(hook_claude_code()),
            HookCmd::Install {
                agent: _,
                scope,
                enforce_stop,
                yes,
            } => settings_edit(scope, yes, |s, bin| {
                install::merge(
                    s,
                    &format!("{} {}", shell_words::quote(bin), install::MARKER),
                    enforce_stop,
                )
            }),
            HookCmd::Uninstall {
                agent: _,
                scope,
                yes,
            } => settings_edit(scope, yes, |s, _| install::remove(s)),
        },
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err((code, msg)) => {
            eprintln!("opshero: {msg}");
            ExitCode::from(code)
        }
    }
}

type Outcome = Result<u8, (u8, String)>;

fn internal(msg: impl Into<String>) -> (u8, String) {
    (EXIT_INTERNAL, msg.into())
}

fn usage(msg: impl Into<String>) -> (u8, String) {
    (EXIT_USAGE, msg.into())
}

fn locate_from(dir: &Path) -> Result<(PathBuf, PathBuf), (u8, String)> {
    let root = git::repo_root(dir).map_err(usage)?;
    let gitdir = git::git_dir(&root).map_err(internal)?;
    Ok((root, gitdir))
}

fn locate() -> Result<(PathBuf, PathBuf), (u8, String)> {
    let cwd = std::env::current_dir()
        .map_err(|e| internal(format!("cannot read current directory: {e}")))?;
    locate_from(&cwd)
}

fn init(force: bool) -> Outcome {
    let (root, _) = locate()?;
    let path = ops::policy_path(&root);
    if path.exists() && !force {
        return Err(usage(format!(
            "{} already exists (use --force to overwrite)",
            path.display()
        )));
    }
    std::fs::create_dir_all(root.join(".opshero"))
        .map_err(|e| internal(format!("cannot create .opshero: {e}")))?;
    std::fs::write(&path, SAMPLE_POLICY)
        .map_err(|e| internal(format!("cannot write policy: {e}")))?;
    println!("wrote {}", path.display());
    println!(
        "next: edit the command pattern, then `opshero run -- <your test command>` and `opshero verify`"
    );
    Ok(0)
}

/// Upper bound for an explicit `--repeat`; `auto` is bounded by the policy parser.
const MAX_REPEAT: u32 = 1000;

fn parse_repeat(spec: &str, cmd: &[String]) -> Result<u32, (u8, String)> {
    if spec == "auto" {
        let (root, _) = locate()?;
        let text = std::fs::read_to_string(ops::policy_path(&root)).map_err(|e| {
            usage(format!(
                "--repeat auto needs the policy: cannot read {}: {e}",
                ops::policy_path(&root).display()
            ))
        })?;
        let pol = policy::parse(&text).map_err(|e| usage(e.to_string()))?;
        let runs = pol
            .require
            .iter()
            .filter(|r| policy::command_matches(&r.command, cmd))
            .filter_map(|r| r.repeat.map(|rep| rep.runs))
            .max()
            .ok_or_else(|| {
                usage("--repeat auto: no policy requirement with a `repeat` rule matches this command")
            })?;
        if runs > MAX_REPEAT {
            return Err(usage(format!(
                "--repeat auto: the policy asks for {runs} runs (more than {MAX_REPEAT}); pass an explicit --repeat if you really want that"
            )));
        }
        return Ok(runs);
    }
    match spec.parse::<u32>() {
        Ok(n) if (1..=MAX_REPEAT).contains(&n) => Ok(n),
        _ => Err(usage(format!(
            "--repeat {spec:?} is not a number between 1 and {MAX_REPEAT} or `auto`"
        ))),
    }
}

fn run(repeat: Option<&str>, cmd: &[String]) -> Outcome {
    let runs = match repeat {
        Some(spec) => parse_repeat(spec, cmd)?,
        None => 1,
    };
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut last_failure = None;
    for _ in 0..runs {
        let status = run_once(cmd)?;
        if status.success() {
            passed += 1;
        } else {
            failed += 1;
            last_failure = Some(exit_code_of(&status));
        }
    }
    if runs > 1 {
        let note = if passed > 0 && failed > 0 {
            " (inconsistent results: flaky or environment-dependent)"
        } else {
            ""
        };
        eprintln!("opshero: {runs} runs: {passed} passed, {failed} failed{note}");
    }
    Ok(last_failure.unwrap_or(0))
}

fn run_once(cmd: &[String]) -> Result<std::process::ExitStatus, (u8, String)> {
    exec_recorded(cmd, "A1", None)
}

/// Run `cmd` and record it as evidence with the given assurance. Recording never
/// breaks the host: if it is impossible, warn and still run the command.
fn exec_recorded(
    cmd: &[String],
    assurance: &'static str,
    adapter: Option<&'static str>,
) -> Result<std::process::ExitStatus, (u8, String)> {
    // Capture must never break the host (RFC-0001 P8): if anything needed to
    // *record* fails, warn and still run the command with its own exit code.
    let ctx = match locate() {
        Ok(c) => Some(c),
        Err((_, msg)) => {
            eprintln!("opshero: warning: {msg}; evidence will not be recorded");
            None
        }
    };
    let exclude = ctx
        .as_ref()
        .map(|(root, _)| ops::subject_excludes(root))
        .unwrap_or_default();
    let pre = ctx.as_ref().and_then(|(root, gitdir)| match git::worktree_tree(root, gitdir, &exclude) {
        Ok(tree) => Some((tree, git::head_commit(root))),
        Err(e) => {
            eprintln!("opshero: warning: cannot measure the working tree: {e}; evidence will not be recorded");
            None
        }
    });
    let started_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let clock = Instant::now();

    let status = match Proc::new(&cmd[0]).args(&cmd[1..]).status() {
        Ok(s) => s,
        Err(e) => return Err((127, format!("cannot run `{}`: {e}", cmd[0]))),
    };
    let duration_ms = u64::try_from(clock.elapsed().as_millis()).unwrap_or(u64::MAX);

    if let (Some((root, gitdir)), Some((before, head))) = (ctx, pre) {
        let cwd = std::env::current_dir().unwrap_or_else(|_| root.clone());
        let recorded = git::worktree_tree(&root, &gitdir, &exclude).and_then(|after| {
            ops::record(
                &gitdir,
                ops::Capture {
                    argv: cmd.to_vec(),
                    env_names: vec![],
                    cwd_rel: ops::rel_cwd(&root, &cwd),
                    dirty: git::is_dirty(&root, &exclude),
                    exclude: exclude.clone(),
                    tree_before: before,
                    tree_after: after,
                    head,
                    exit_code: status.code().map(i64::from),
                    signal: signal_of(&status),
                    started_secs,
                    duration_ms,
                    adapter,
                    assurance,
                },
            )
        });
        if let Err(e) = recorded {
            eprintln!("opshero: warning: evidence was not recorded: {e}");
        }
    }
    Ok(status)
}

#[cfg(unix)]
fn signal_of(status: &std::process::ExitStatus) -> Option<i64> {
    use std::os::unix::process::ExitStatusExt;
    status.signal().map(i64::from)
}

#[cfg(not(unix))]
fn signal_of(_status: &std::process::ExitStatus) -> Option<i64> {
    None
}

fn exit_code_of(status: &std::process::ExitStatus) -> u8 {
    match (status.code(), signal_of(status)) {
        (Some(c), _) => u8::try_from(c & 0xff).unwrap_or(EXIT_INTERNAL),
        (None, Some(s)) => u8::try_from(128 + s).unwrap_or(EXIT_INTERNAL),
        (None, None) => EXIT_INTERNAL,
    }
}

/// The author of a change controls everything inside the checked-out repository, including
/// the policy file. A gate that reads its policy from there can be weakened by the change
/// it is judging. Say so, loudly, when that is what is happening.
fn warn_if_policy_is_untrusted(root: &Path, policy_file: &Path) {
    if ops::is_inside_repo(root, policy_file) {
        eprintln!(
            "opshero: warning: the policy is read from the checked-out content, which the author of the change controls. For a gate, pass --policy with a file taken from the target branch (trusted): see docs/ci-gate.md"
        );
    }
}

fn verify_cmd(policy_path: &Path, format: Format, ci_remote: Option<&str>) -> Outcome {
    let (root, gitdir) = locate()?;
    let path = if policy_path.is_absolute() {
        policy_path.to_path_buf()
    } else {
        root.join(policy_path)
    };
    let text = std::fs::read_to_string(&path).map_err(|e| {
        usage(format!(
            "cannot read policy {}: {e} (run `opshero init` first)",
            path.display()
        ))
    })?;
    let pol = policy::parse(&text).map_err(|e| usage(e.to_string()))?;
    if ci_remote.is_some() {
        warn_if_policy_is_untrusted(&root, &path);
    }
    // CI mode: bring in the evidence published for this exact content first. Every
    // problem on the way becomes a finding and leaves the verdict UNKNOWN (fail closed).
    let mut notes = Vec::new();
    if let Some(remote) = ci_remote {
        match git::worktree_tree(&root, &gitdir, &pol.effective_excludes()) {
            Ok(tree) => notes = evidence::fetch_and_import(&root, &gitdir, remote, &tree).notes,
            Err(e) => notes.push(format!(
                "cannot measure the working tree, so no evidence was fetched: {e}"
            )),
        }
    }
    let mut report = ops::evaluate_now(&root, &gitdir, &pol).map_err(internal)?;
    report.findings.extend(notes);
    report.findings.sort();
    report.findings.dedup();
    let out = match format {
        Format::Text => render::text(&report),
        Format::Json => render::json(&report).map_err(|e| internal(e.to_string()))?,
    };
    print!("{out}");
    Ok(match report.verdict {
        Verdict::Pass => 0,
        Verdict::Fail => EXIT_FAIL,
        Verdict::Unknown => EXIT_UNKNOWN,
    })
}

/// Called by Claude Code. Always exits 0 so that a problem here never breaks the agent.
fn hook_claude_code() -> u8 {
    let mut input = String::new();
    if let Err(e) = std::io::stdin()
        .take(MAX_HOOK_INPUT)
        .read_to_string(&mut input)
    {
        eprintln!("opshero: warning: cannot read hook input: {e}");
        return 0;
    }
    match claude::handle(&input) {
        Ok(Some(stdout)) => println!("{stdout}"),
        Ok(None) => {}
        Err(e) => eprintln!("opshero: warning: {e}"),
    }
    0
}

fn settings_path(scope: Scope) -> Result<PathBuf, (u8, String)> {
    match scope {
        Scope::User => std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".claude").join("settings.json"))
            .ok_or_else(|| usage("HOME is not set")),
        Scope::Local | Scope::Project => {
            let (root, _) = locate()?;
            let file = if matches!(scope, Scope::Local) {
                "settings.local.json"
            } else {
                "settings.json"
            };
            Ok(root.join(".claude").join(file))
        }
    }
}

/// Shared by install/uninstall: read, transform with a pure function, show the plan,
/// and write only with --yes (after keeping a one-time backup of the original).
fn settings_edit(
    scope: Scope,
    yes: bool,
    transform: impl FnOnce(serde_json::Value, &str) -> Result<(serde_json::Value, Vec<String>), String>,
) -> Outcome {
    let path = settings_path(scope)?;
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(internal(format!("cannot read {}: {e}", path.display()))),
    };
    let current: serde_json::Value = match &existing {
        Some(text) => serde_json::from_str(text).map_err(|e| {
            usage(format!(
                "{} is not valid JSON ({e}); refusing to touch it",
                path.display()
            ))
        })?,
        None => serde_json::json!({}),
    };
    let exe = std::env::current_exe()
        .map_err(|e| internal(format!("cannot locate this executable: {e}")))?;
    let (updated, changes) = transform(current, &exe.to_string_lossy()).map_err(usage)?;
    if changes.is_empty() {
        println!("nothing to change in {}", path.display());
        return Ok(0);
    }
    println!("planned changes in {}:", path.display());
    for c in &changes {
        println!("  - {c}");
    }
    if !yes {
        println!("dry run: re-run with --yes to apply");
        return Ok(0);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| internal(format!("cannot create {}: {e}", parent.display())))?;
    }
    if let Some(text) = &existing {
        let backup = path.with_extension("json.opshero-bak");
        if !backup.exists() {
            std::fs::write(&backup, text)
                .map_err(|e| internal(format!("cannot write backup: {e}")))?;
            println!("backup of the original: {}", backup.display());
        }
    }
    let body = serde_json::to_string_pretty(&updated).map_err(|e| internal(e.to_string()))? + "\n";
    let tmp = path.with_extension("json.opshero-tmp");
    std::fs::write(&tmp, body).map_err(|e| internal(format!("cannot write settings: {e}")))?;
    std::fs::rename(&tmp, &path).map_err(|e| internal(format!("cannot replace settings: {e}")))?;
    println!("applied.");
    Ok(0)
}

const JOURNAL_NOTICE: &str = "The journal stores: the Stop-gate decision and verdict, coarse reason categories \
(missing, stale, failed, ...), whether a recorded run passed, a one-way hash of the session id and a timestamp.\n\
It is never stored: commands, arguments, file paths, tree ids, file contents, the raw session id.\n\
The file stays in .git/opshero/ on this machine; nothing is sent anywhere.";

fn journal_cmd(cmd: JournalCmd) -> Outcome {
    use opshero_core::journal as jc;
    let (_, gitdir) = locate()?;
    match cmd {
        JournalCmd::Enable => {
            journal::enable(&gitdir).map_err(internal)?;
            println!(
                "journal enabled: {}\n{JOURNAL_NOTICE}",
                journal::journal_path(&gitdir).display()
            );
            Ok(0)
        }
        JournalCmd::Disable { delete } => {
            journal::disable(&gitdir, delete).map_err(internal)?;
            println!(
                "journal disabled{}",
                if delete {
                    " and deleted"
                } else {
                    " (the file is kept)"
                }
            );
            Ok(0)
        }
        JournalCmd::Status => {
            let (events, skipped) = journal::load(&gitdir).map_err(internal)?;
            println!(
                "journal: {} ({} event(s), {skipped} unreadable line(s)) at {}",
                if journal::enabled(&gitdir) {
                    "enabled"
                } else {
                    "disabled"
                },
                events.len(),
                journal::journal_path(&gitdir).display()
            );
            Ok(0)
        }
        JournalCmd::List => {
            let (events, _) = journal::load(&gitdir).map_err(internal)?;
            let sum_labels = latest_labels(&events);
            let mut any = false;
            for (line, ev) in &events {
                if let jc::Event::Stop {
                    decision: jc::Decision::Block,
                    reasons,
                    stop_active,
                    ..
                } = ev
                {
                    any = true;
                    let label = match sum_labels.get(line) {
                        Some(jc::Label::Real) => "real",
                        Some(jc::Label::Noise) => "noise",
                        None => "unlabeled",
                    };
                    println!(
                        "{line:>5}  block  {:<28} {label}{}",
                        reasons.join(","),
                        if *stop_active {
                            "  (after a first block)"
                        } else {
                            ""
                        }
                    );
                }
            }
            if !any {
                println!("no blocking Stop decisions recorded yet");
            }
            Ok(0)
        }
        JournalCmd::Label { line, label } => {
            let (events, _) = journal::load(&gitdir).map_err(internal)?;
            let is_block = events.iter().any(|(l, e)| {
                *l == line
                    && matches!(
                        e,
                        jc::Event::Stop {
                            decision: jc::Decision::Block,
                            ..
                        }
                    )
            });
            if !is_block {
                return Err(usage(format!(
                    "line {line} is not a blocking Stop decision (see `opshero journal list`)"
                )));
            }
            let label = match label {
                LabelArg::Real => jc::Label::Real,
                LabelArg::Noise => jc::Label::Noise,
            };
            journal::append(
                &gitdir,
                &jc::Event::Label {
                    target: line,
                    label,
                },
            )
            .map_err(internal)?;
            println!(
                "line {line} labelled {}",
                if label == jc::Label::Real {
                    "real"
                } else {
                    "noise"
                }
            );
            Ok(0)
        }
        JournalCmd::Report { format } => {
            let (events, skipped) = journal::load(&gitdir).map_err(internal)?;
            let sum = jc::summarize(&events);
            let interval = jc::wilson(sum.sessions_with_real_finding, sum.sessions);
            let outcome = jc::outcome(&sum);
            let name = match outcome {
                jc::Outcome::NotEnoughData { .. } => "not-enough-data",
                jc::Outcome::Meets => "meets",
                jc::Outcome::Inconclusive => "inconclusive",
                jc::Outcome::KillCriterion => "kill-criterion",
            };
            match format {
                Format::Json => {
                    let j = serde_json::json!({
                        "sessions": sum.sessions, "stops": sum.stops, "blocks": sum.blocks,
                        "blocks_real": sum.blocks_real, "blocks_noise": sum.blocks_noise,
                        "blocks_unlabeled": sum.blocks_unlabeled,
                        "sessions_gated": sum.sessions_gated,
                        "sessions_with_real_finding": sum.sessions_with_real_finding,
                        "records": sum.records, "records_failed": sum.records_failed,
                        "labels_ignored": sum.labels_ignored, "unreadable_lines": skipped,
                        "by_reason": sum.by_reason,
                        "interval_95": interval.map(|(l, h)| serde_json::json!([l, h])),
                        "outcome": name, "needed_sessions": jc::MIN_SESSIONS,
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&j).map_err(|e| internal(e.to_string()))?
                    );
                }
                Format::Text => {
                    println!(
                        "sessions seen:               {} (a conclusion needs {}); the gate had something to judge in {}",
                        sum.sessions,
                        jc::MIN_SESSIONS,
                        sum.sessions_gated
                    );
                    println!(
                        "Stop-gate decisions:         {} (blocks {}: {} real, {} noise, {} unlabeled)",
                        sum.stops,
                        sum.blocks,
                        sum.blocks_real,
                        sum.blocks_noise,
                        sum.blocks_unlabeled
                    );
                    let pct = if sum.sessions == 0 {
                        0.0
                    } else {
                        100.0 * sum.sessions_with_real_finding as f64 / sum.sessions as f64
                    };
                    print!(
                        "sessions with a real finding: {} of {} sessions ({pct:.1}%)",
                        sum.sessions_with_real_finding, sum.sessions
                    );
                    if let Some((lo, hi)) = interval {
                        print!("  95% interval {:.1}% to {:.1}%", lo * 100.0, hi * 100.0);
                    }
                    println!();
                    println!(
                        "recorded runs:               {} ({} failed)",
                        sum.records, sum.records_failed
                    );
                    if !sum.by_reason.is_empty() {
                        let r: Vec<String> = sum
                            .by_reason
                            .iter()
                            .map(|(k, v)| format!("{k} {v}"))
                            .collect();
                        println!("block reasons:               {}", r.join(", "));
                    }
                    if sum.blocks_unlabeled > 0 {
                        println!(
                            "note: {} block(s) are unlabeled; only labelled real findings count (run `opshero journal list`)",
                            sum.blocks_unlabeled
                        );
                    }
                    println!(
                        "pre-registered rule:         at least {}% of at least {} sessions; below {}% reassess the direction",
                        jc::SUCCESS_PPM / 10_000,
                        jc::MIN_SESSIONS,
                        jc::KILL_PPM / 10_000
                    );
                    println!(
                        "outcome:                     {}",
                        match outcome {
                            jc::Outcome::NotEnoughData { sessions } => format!(
                                "not enough data ({sessions} of {} sessions)",
                                jc::MIN_SESSIONS
                            ),
                            jc::Outcome::Meets =>
                                "meets the threshold (point estimate; see the interval)".to_owned(),
                            jc::Outcome::Inconclusive =>
                                "inconclusive (between the thresholds)".to_owned(),
                            jc::Outcome::KillCriterion =>
                                "below the kill threshold: reassess the direction".to_owned(),
                        }
                    );
                }
            }
            Ok(0)
        }
    }
}

fn latest_labels(
    events: &[(usize, opshero_core::journal::Event)],
) -> std::collections::BTreeMap<usize, opshero_core::journal::Label> {
    let mut out = std::collections::BTreeMap::new();
    for (_, e) in events {
        if let opshero_core::journal::Event::Label { target, label } = e {
            out.insert(*target, *label); // events are in file order: the latest wins
        }
    }
    out
}

fn gc_cmd(keep: usize, dry_run: bool) -> Outcome {
    let (root, gitdir) = locate()?;
    let exclude = ops::subject_excludes(&root);
    // Without the current tree we cannot protect the evidence that matters: refuse.
    let tree = git::worktree_tree(&root, &gitdir, &exclude).map_err(|e| {
        internal(format!(
            "cannot measure the working tree, so nothing was deleted: {e}"
        ))
    })?;
    // Same lock as recording: no recorder writes while we delete.
    let dir = gitdir.join("opshero");
    std::fs::create_dir_all(&dir)
        .map_err(|e| internal(format!("cannot create {}: {e}", dir.display())))?;
    let _chain = lock::acquire(&dir.join("store.flock"), std::time::Duration::from_secs(10));
    let protect = |r: &opshero_core::record::Record| {
        r.body.subject.tree_after == tree && r.body.subject.exclude == exclude
    };
    let report = store::Store::open(&gitdir)
        .prune(keep, dry_run, &protect)
        .map_err(internal)?;
    println!(
        "{} {} of {} records; kept {} ({} only because they are bound to the current content)",
        if dry_run { "would remove" } else { "removed" },
        report.removed,
        report.total,
        report.kept,
        report.protected
    );
    Ok(0)
}

fn evidence_cmd(cmd: EvidenceCmd) -> Outcome {
    let (root, gitdir) = locate()?;
    let text = std::fs::read_to_string(ops::policy_path(&root)).map_err(|e| {
        usage(format!(
            "cannot read the policy: {e} (run `opshero init` first)"
        ))
    })?;
    let pol = policy::parse(&text).map_err(|e| usage(e.to_string()))?;
    let exclude = pol.effective_excludes();
    let tree = git::worktree_tree(&root, &gitdir, &exclude)
        .map_err(|e| internal(format!("cannot measure the working tree: {e}")))?;
    match cmd {
        EvidenceCmd::Push { remote } => {
            if !evidence::remote_exists(&root, &remote) {
                return Err(usage(format!(
                    "the git remote `{remote}` is not configured; pass --remote <name>"
                )));
            }
            let n =
                evidence::publish(&root, &gitdir, &remote, &tree, &exclude).map_err(internal)?;
            println!(
                "published {n} record(s) for tree {} to {}:{}",
                &tree[..tree.len().min(12)],
                remote,
                evidence::ref_for(&tree)
            );
            println!(
                "note: the records include the command line of each recorded run (arguments included) and are now visible to everyone who can read `{remote}`."
            );
            println!(
                "this is assurance A1 evidence: it shows the checks ran, it cannot resist a dishonest publisher."
            );
            Ok(0)
        }
        EvidenceCmd::Fetch { remote } => {
            let imported = evidence::fetch_and_import(&root, &gitdir, &remote, &tree);
            for n in &imported.notes {
                println!("{n}");
            }
            Ok(if imported.count > 0 { 0 } else { EXIT_UNKNOWN })
        }
    }
}

/// `opshero ci run`: the CI executes the required commands itself and records them as A2.
/// Evidence produced here does not depend on anything the author of the change supplied.
fn ci_run(policy_path: Option<&Path>, allow_local: bool) -> Outcome {
    if !allow_local && std::env::var_os("CI").is_none_or(|v| v.is_empty()) {
        return Err(usage(
            "`opshero ci run` records evidence as assurance A2, which only means something when it runs in CI, and the CI environment variable is not set. Pass --allow-local to run it anyway (testing only)",
        ));
    }
    let (root, gitdir) = locate()?;
    let path = match policy_path {
        Some(p) if p.is_absolute() => p.to_path_buf(),
        Some(p) => root.join(p),
        None => ops::policy_path(&root),
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|e| usage(format!("cannot read policy {}: {e}", path.display())))?;
    let pol = policy::parse(&text).map_err(|e| usage(e.to_string()))?;
    warn_if_policy_is_untrusted(&root, &path);
    // Decide everything before executing anything: a requirement that cannot be
    // executed as written is a configuration error, not a partial run.
    let mut commands: Vec<Vec<String>> = Vec::new();
    for r in &pol.require {
        let Some(cmd) = r.ci_command() else {
            return Err(usage(format!(
                "requirement `{}` has a wildcard command pattern, so the CI cannot know what to execute: add `run = [...]` with a concrete command that matches it",
                r.id
            )));
        };
        if !commands.contains(&cmd) {
            commands.push(cmd);
        }
    }
    std::env::set_current_dir(&root)
        .map_err(|e| internal(format!("cannot enter {}: {e}", root.display())))?;
    for cmd in &commands {
        eprintln!("opshero ci: running {}", cmd.join(" "));
        match exec_recorded(cmd, "A2", Some("ci")) {
            Ok(status) if status.success() => {}
            Ok(status) => eprintln!(
                "opshero ci: `{}` exited with {}",
                cmd.join(" "),
                exit_code_of(&status)
            ),
            // Could not even start: nothing was recorded, so the verdict will be UNKNOWN.
            Err((_, msg)) => eprintln!("opshero ci: {msg}"),
        }
    }
    let report = ops::evaluate_now(&root, &gitdir, &pol).map_err(internal)?;
    print!("{}", render::text(&report));
    Ok(match report.verdict {
        Verdict::Pass => 0,
        Verdict::Fail => EXIT_FAIL,
        Verdict::Unknown => EXIT_UNKNOWN,
    })
}
