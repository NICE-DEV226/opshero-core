//! `opshero doctor`: checks that the tool can do its job in this directory.
//!
//! Every check reports one line; the pure parts (version parsing, rendering) are tested on their
//! own, the rest is exercised by end-to-end tests against real repositories.

use crate::{git, install, ops, store};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Oldest git this tool is tested against in CI (see `.github/workflows/ci.yml`). Older versions
/// may work; they are reported as a warning, not a failure.
pub const MIN_TESTED_GIT: (u32, u32) = (2, 34);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub level: Level,
    pub text: String,
}

fn check(level: Level, text: impl Into<String>) -> Check {
    Check {
        level,
        text: text.into(),
    }
}

/// `git version 2.43.0`, `git version 2.39.5 (Apple Git-154)`, `git version 2.45.1.windows.1`.
pub fn parse_git_version(s: &str) -> Option<(u32, u32)> {
    let rest = s.trim().strip_prefix("git version ")?;
    let mut parts = rest
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?
        .split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

pub fn render(checks: &[Check]) -> String {
    let mut out = String::new();
    for c in checks {
        let tag = match c.level {
            Level::Ok => "ok  ",
            Level::Warn => "warn",
            Level::Fail => "FAIL",
        };
        // Continuation lines (a multi-line parse error) stay under the message, not under the tag.
        out.push_str(&format!(
            "{tag}  {}\n",
            c.text.trim_end().replace('\n', "\n      ")
        ));
    }
    out
}

pub fn failed(checks: &[Check]) -> bool {
    checks.iter().any(|c| c.level == Level::Fail)
}

/// The settings files a Claude Code hook may be registered in, with their scope names.
fn hook_files(root: Option<&Path>) -> Vec<(&'static str, PathBuf)> {
    let mut files = Vec::new();
    if let Some(root) = root {
        files.push(("local", root.join(".claude").join("settings.local.json")));
        files.push(("project", root.join(".claude").join("settings.json")));
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        files.push((
            "user",
            PathBuf::from(home).join(".claude").join("settings.json"),
        ));
    }
    files
}

pub fn run(cwd: &Path) -> Vec<Check> {
    let mut out = vec![check(
        Level::Ok,
        format!("opshero {}", env!("CARGO_PKG_VERSION")),
    )];

    match Command::new("git").arg("--version").output() {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_owned();
            match parse_git_version(&text) {
                Some(v) if v >= MIN_TESTED_GIT => out.push(check(Level::Ok, text)),
                Some(_) => out.push(check(
                    Level::Warn,
                    format!(
                        "{text}: older than {}.{}, the oldest version this tool is tested with",
                        MIN_TESTED_GIT.0, MIN_TESTED_GIT.1
                    ),
                )),
                None => out.push(check(
                    Level::Warn,
                    format!("cannot read the git version from `{text}`"),
                )),
            }
        }
        _ => {
            out.push(check(
                Level::Fail,
                "git was not found on PATH; opshero needs it to identify the content it checks",
            ));
            return out;
        }
    }

    let root = match git::repo_root(cwd) {
        Ok(root) => root,
        Err(_) => {
            out.push(check(
                Level::Warn,
                "not inside a git repository; run `opshero doctor` from one to check the rest",
            ));
            push_hooks(&mut out, None);
            return out;
        }
    };
    out.push(check(Level::Ok, format!("repository {}", root.display())));
    let Ok(gitdir) = git::git_dir(&root) else {
        out.push(check(Level::Fail, "cannot locate the .git directory"));
        return out;
    };

    let policy_file = ops::policy_path(&root);
    let mut excludes = opshero_core::policy::default_excludes();
    match std::fs::read_to_string(&policy_file) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => out.push(check(
            Level::Warn,
            "no policy at .opshero/policy.toml; run `opshero init` and set your test command",
        )),
        Err(e) => out.push(check(Level::Fail, format!("cannot read the policy: {e}"))),
        Ok(text) => match opshero_core::policy::parse(&text) {
            Ok(p) => {
                excludes = p.effective_excludes();
                let n = p.require.len();
                out.push(check(
                    Level::Ok,
                    format!(
                        "policy `{}` with {n} requirement{}",
                        p.id,
                        if n == 1 { "" } else { "s" }
                    ),
                ));
            }
            Err(e) => out.push(check(Level::Fail, e.to_string())),
        },
    }

    let started = Instant::now();
    match git::worktree_tree(&root, &gitdir, &excludes) {
        Ok(tree) => out.push(check(
            Level::Ok,
            format!(
                "measured the working tree ({}) in {} ms",
                &tree[..tree.len().min(12)],
                started.elapsed().as_millis()
            ),
        )),
        Err(e) => out.push(check(
            Level::Fail,
            format!("cannot measure the working tree: {e}"),
        )),
    }

    let (records, rejected) = store::Store::open(&gitdir).load();
    out.push(check(
        Level::Ok,
        format!("evidence store: {} record(s)", records.len()),
    ));
    if !rejected.is_empty() {
        out.push(check(
            Level::Warn,
            format!(
                "{} record file(s) failed their integrity check and are ignored",
                rejected.len()
            ),
        ));
    }

    push_hooks(&mut out, Some(&root));
    out
}

fn push_hooks(out: &mut Vec<Check>, root: Option<&Path>) {
    let found: Vec<&str> = hook_files(root)
        .into_iter()
        .filter(|(_, path)| {
            std::fs::read_to_string(path).is_ok_and(|t| t.contains(install::MARKER))
        })
        .map(|(scope, _)| scope)
        .collect();
    if found.is_empty() {
        out.push(check(
            Level::Ok,
            "Claude Code hooks: not installed (optional: `opshero hook install --yes`)",
        ));
    } else {
        out.push(check(
            Level::Ok,
            format!("Claude Code hooks: installed ({})", found.join(", ")),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_versions_from_real_distributions() {
        assert_eq!(parse_git_version("git version 2.43.0\n"), Some((2, 43)));
        assert_eq!(
            parse_git_version("git version 2.39.5 (Apple Git-154)"),
            Some((2, 39))
        );
        assert_eq!(
            parse_git_version("git version 2.45.1.windows.1"),
            Some((2, 45))
        );
        assert_eq!(parse_git_version("git version 2.56.0"), Some((2, 56)));
    }

    #[test]
    fn unreadable_git_versions_are_none() {
        for s in [
            "",
            "git",
            "git version",
            "git version x.y",
            "2.43.0",
            "git version 2",
        ] {
            assert_eq!(parse_git_version(s), None, "{s:?}");
        }
    }

    #[test]
    fn only_a_failed_check_fails_the_run() {
        let ok = check(Level::Ok, "a");
        let warn = check(Level::Warn, "b");
        let fail = check(Level::Fail, "c");
        assert!(!failed(&[ok.clone(), warn.clone()]));
        assert!(failed(&[ok, warn, fail]));
    }

    #[test]
    fn render_gives_one_tagged_line_per_check() {
        let text = render(&[
            check(Level::Ok, "x"),
            check(Level::Warn, "y"),
            check(Level::Fail, "z"),
        ]);
        assert_eq!(text, "ok    x\nwarn  y\nFAIL  z\n");
        assert_eq!(
            render(&[check(Level::Fail, "a\n  |\nb\n")]),
            "FAIL  a\n        |\n      b\n"
        );
    }

    #[test]
    fn version_comparison_orders_minor_versions() {
        assert!(parse_git_version("git version 2.9.0").unwrap() < MIN_TESTED_GIT);
        assert!(parse_git_version("git version 3.0.0").unwrap() >= MIN_TESTED_GIT);
    }
}
