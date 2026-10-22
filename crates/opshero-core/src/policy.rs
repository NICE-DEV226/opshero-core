//! Policy v0 (RFC-0004): a small declarative TOML schema with built-in checks.
//! Parsing is pure; unknown keys are an error so typos never pass silently.

use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Assurance {
    A0,
    A1,
    A2,
    A3,
}

impl Assurance {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "A0" => Some(Self::A0),
            "A1" => Some(Self::A1),
            "A2" => Some(Self::A2),
            "A3" => Some(Self::A3),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::A0 => "A0",
            Self::A1 => "A1",
            Self::A2 => "A2",
            Self::A3 => "A3",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Block,
    Warn,
}

/// Opt-in rule for commands that may be flaky (RFC-0005 §3.1): on one tree,
/// any failing run next to a passing one is an inconsistency, and PASS needs
/// `runs` passing runs and no failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Repeat {
    pub alpha_ppm: u32,
    pub flake_ppm: u32,
    /// `stats::min_passes(alpha_ppm, flake_ppm)`, computed once at parse time.
    pub runs: u32,
}

/// What to do with a run during which the working tree changed (tree before
/// differs from tree after). `Deny` (the default since 2026-10-21) does not
/// count it, because the command may have run against content that is no
/// longer there (docs/formal/Freshness.tla, RFC-0005 §7). `Allow` counts it
/// for the tree after the run and reports a finding; it is an explicit
/// opt-out for commands that legitimately write untracked files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostRunChanges {
    Allow,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Require {
    pub id: String,
    pub command: Vec<String>,
    pub min_assurance: Assurance,
    pub severity: Severity,
    pub repeat: Option<Repeat>,
    pub post_run_changes: PostRunChanges,
    /// Git pathspecs. When non-empty, the Stop gate only demands this
    /// requirement in a turn that changed a matching file (a documentation-only
    /// change does not need the test suite). `verify` ignores it: CI has no
    /// turn, so every requirement applies.
    pub paths: Vec<String>,
    /// The concrete command the CI executes for this requirement (`opshero ci run`).
    /// Must match `command`. Needed when `command` ends with a wildcard.
    pub run: Option<Vec<String>>,
}

impl Require {
    /// What the CI should execute: `run` if given, else the command pattern itself when it
    /// has no wildcard. `None` means the requirement cannot be executed as written.
    pub fn ci_command(&self) -> Option<Vec<String>> {
        self.run.clone().or_else(|| {
            (self.command.last().map(String::as_str) != Some("*")).then(|| self.command.clone())
        })
    }
}

/// Parse a decimal percentage such as `5%`, `12.5%` or `0.01%` into parts per
/// million, exactly (no floating point). At most four decimals; the value must
/// be strictly between 0 and 100.
pub fn parse_percent_ppm(s: &str) -> Option<u32> {
    let body = s.strip_suffix('%')?;
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    if body.ends_with('.')
        || int.is_empty()
        || int.len() > 3
        || frac.len() > 4
        || !int.chars().all(|c| c.is_ascii_digit())
        || !frac.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let mut padded = frac.to_owned();
    while padded.len() < 4 {
        padded.push('0');
    }
    let ppm = int.parse::<u32>().ok()?.checked_mul(10_000)? + padded.parse::<u32>().ok()?;
    (ppm > 0 && ppm < 1_000_000).then_some(ppm)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub id: String,
    pub require: Vec<Require>,
    /// Extra pathspecs excluded from the subject (policy `[subject] exclude`).
    pub subject_exclude: Vec<String>,
}

/// Directories owned by developer tools. Files they write never count as a
/// change to the code under verification (found with a real agent session).
pub const DEFAULT_EXCLUDES: &[&str] = &[".claude", ".codex", ".cursor", ".opshero"];

/// The default exclusions, as owned strings, sorted.
pub fn default_excludes() -> Vec<String> {
    DEFAULT_EXCLUDES.iter().map(|s| (*s).to_owned()).collect()
}

impl Policy {
    /// Defaults plus the policy's own exclusions: sorted and de-duplicated so
    /// the same policy always yields the same list.
    pub fn effective_excludes(&self) -> Vec<String> {
        let mut all = default_excludes();
        all.extend(self.subject_exclude.iter().cloned());
        all.sort();
        all.dedup();
        all
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PolicyError(pub String);

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid policy: {}", self.0)
    }
}

impl std::error::Error for PolicyError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPolicy {
    version: u32,
    id: String,
    #[serde(default)]
    require: Vec<RawRequire>,
    subject: Option<RawSubject>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSubject {
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequire {
    id: String,
    check: String,
    command: Vec<String>,
    min_assurance: Option<String>,
    severity: Option<String>,
    repeat: Option<RawRepeat>,
    post_run_changes: Option<String>,
    paths: Option<Vec<String>>,
    run: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRepeat {
    alpha: String,
    flake_rate: String,
}

pub fn parse(text: &str) -> Result<Policy, PolicyError> {
    let raw: RawPolicy = toml::from_str(text).map_err(|e| PolicyError(e.to_string()))?;
    if raw.version != 1 {
        return Err(PolicyError(format!(
            "unsupported policy version {}",
            raw.version
        )));
    }
    if raw.require.is_empty() {
        return Err(PolicyError("at least one [[require]] is needed".into()));
    }
    let mut seen = HashSet::new();
    let mut require = Vec::new();
    for r in raw.require {
        if !seen.insert(r.id.clone()) {
            return Err(PolicyError(format!("duplicate require id `{}`", r.id)));
        }
        if r.check != "evidence.fresh" {
            return Err(PolicyError(format!(
                "require `{}`: unsupported check `{}` (v0 supports `evidence.fresh`)",
                r.id, r.check
            )));
        }
        if r.command.is_empty() {
            return Err(PolicyError(format!(
                "require `{}`: empty command pattern",
                r.id
            )));
        }
        if let Some(pos) = r.command.iter().position(|p| p == "*")
            && pos + 1 != r.command.len()
        {
            return Err(PolicyError(format!(
                "require `{}`: `*` is only allowed as the last pattern element",
                r.id
            )));
        }
        let min_assurance = match r.min_assurance.as_deref() {
            None => Assurance::A1,
            Some(s) => Assurance::parse(s).ok_or_else(|| {
                PolicyError(format!("require `{}`: unknown assurance `{s}`", r.id))
            })?,
        };
        let severity = match r.severity.as_deref() {
            None | Some("block") => Severity::Block,
            Some("warn") => Severity::Warn,
            Some(s) => {
                return Err(PolicyError(format!(
                    "require `{}`: unknown severity `{s}`",
                    r.id
                )));
            }
        };
        let repeat = match r.repeat {
            None => None,
            Some(raw) => {
                let bad = |what: &str, v: &str| {
                    PolicyError(format!(
                        "require `{}`: repeat.{what} = {v:?} is not a percentage like \"5%\" (0 < value < 100, at most 4 decimals)",
                        r.id
                    ))
                };
                let alpha_ppm =
                    parse_percent_ppm(&raw.alpha).ok_or_else(|| bad("alpha", &raw.alpha))?;
                let flake_ppm = parse_percent_ppm(&raw.flake_rate)
                    .ok_or_else(|| bad("flake_rate", &raw.flake_rate))?;
                let runs = crate::stats::min_passes(alpha_ppm, flake_ppm).ok_or_else(|| {
                    PolicyError(format!(
                        "require `{}`: repeat needs more than {} passing runs; choose a larger alpha or flake_rate",
                        r.id,
                        crate::stats::MAX_RUNS
                    ))
                })?;
                Some(Repeat {
                    alpha_ppm,
                    flake_ppm,
                    runs,
                })
            }
        };
        let post_run_changes = match r.post_run_changes.as_deref() {
            None | Some("deny") => PostRunChanges::Deny,
            Some("allow") => PostRunChanges::Allow,
            Some(s) => {
                return Err(PolicyError(format!(
                    "require `{}`: unknown post_run_changes `{s}` (use \"allow\" or \"deny\")",
                    r.id
                )));
            }
        };
        let paths = r.paths.unwrap_or_default();
        if paths.len() > 64 {
            return Err(PolicyError(format!(
                "require `{}`: `paths` has more than 64 entries",
                r.id
            )));
        }
        if let Some(bad) = paths
            .iter()
            .find(|e| e.is_empty() || e.starts_with([':', '!']) || e.chars().any(char::is_control))
        {
            return Err(PolicyError(format!(
                "require `{}`: `paths` entry {bad:?} is invalid (empty, starts with `:` or `!`, or contains control characters)",
                r.id
            )));
        }
        if let Some(run) = &r.run {
            let bad = run.is_empty()
                || run
                    .iter()
                    .any(|a| a == "*" || a.chars().any(char::is_control))
                || !command_matches(&r.command, run);
            if bad {
                return Err(PolicyError(format!(
                    "require `{}`: `run` must be a concrete command (no `*`, no control characters) that matches `command`",
                    r.id
                )));
            }
        }
        require.push(Require {
            id: r.id,
            command: r.command,
            min_assurance,
            severity,
            repeat,
            post_run_changes,
            paths,
            run: r.run,
        });
    }
    let subject_exclude = raw.subject.map(|s| s.exclude).unwrap_or_default();
    if subject_exclude.len() > 64 {
        return Err(PolicyError(
            "`[subject] exclude` has more than 64 entries".into(),
        ));
    }
    for e in &subject_exclude {
        if e.is_empty() || e.starts_with([':', '!']) || e.chars().any(char::is_control) {
            return Err(PolicyError(format!(
                "`[subject] exclude` entry {e:?} is invalid (empty, starts with `:` or `!`, or contains control characters)"
            )));
        }
        // Made only of `*`, `.` and `/`: matches the whole repository, so the subject would
        // be the empty tree and any evidence would look fresh for any content.
        if e.chars().all(|c| matches!(c, '*' | '.' | '/')) {
            return Err(PolicyError(format!(
                "`[subject] exclude` entry {e:?} would exclude the whole repository"
            )));
        }
    }
    Ok(Policy {
        id: raw.id,
        require,
        subject_exclude,
    })
}

/// Does `argv` match the pattern? Elements match exactly; a trailing `*`
/// matches any (possibly empty) remainder.
pub fn command_matches(pattern: &[String], argv: &[String]) -> bool {
    match pattern.split_last() {
        Some((last, head)) if last == "*" => {
            argv.len() >= head.len() && head.iter().zip(argv).all(|(p, a)| p == a)
        }
        _ => pattern == argv,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn parses_a_valid_policy_with_defaults() {
        let p = parse(
            r#"
            version = 1
            id = "agent-pr"
            [[require]]
            id = "tests"
            check = "evidence.fresh"
            command = ["pytest", "*"]
            "#,
        )
        .unwrap();
        assert_eq!(p.id, "agent-pr");
        assert_eq!(p.require[0].min_assurance, Assurance::A1);
        assert_eq!(p.require[0].severity, Severity::Block);
    }

    #[test]
    fn rejects_unknown_keys_bad_versions_and_misplaced_wildcards() {
        let base = |extra: &str| {
            format!(
                "version = 1\nid = \"p\"\n[[require]]\nid = \"t\"\ncheck = \"evidence.fresh\"\ncommand = [\"a\", \"*\"]\n{extra}"
            )
        };
        assert!(parse(&base("")).is_ok());
        assert!(parse(&base("typo = 1")).is_err());
        assert!(parse("version = 2\nid = \"p\"\n").is_err());
        assert!(
            parse("version = 1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"*\",\"a\"]\n")
                .is_err()
        );
        assert!(
            parse(
                "version = 1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"other\"\ncommand=[\"a\"]\n"
            )
            .is_err()
        );
        assert!(parse("version = 1\nid = \"p\"\n").is_err());
    }

    #[test]
    fn rejects_duplicate_ids_and_unknown_levels() {
        let dup = "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"a\"]\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"b\"]\n";
        assert!(parse(dup).is_err());
        let lvl = "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"a\"]\nmin_assurance=\"A9\"\n";
        assert!(parse(lvl).is_err());
    }

    #[test]
    fn command_matching() {
        assert!(command_matches(&v(&["pytest", "*"]), &v(&["pytest"])));
        assert!(command_matches(
            &v(&["pytest", "*"]),
            &v(&["pytest", "-q", "x"])
        ));
        assert!(!command_matches(&v(&["pytest", "*"]), &v(&["pytes"])));
        assert!(command_matches(&v(&["*"]), &v(&[])));
        assert!(command_matches(&v(&["a", "b"]), &v(&["a", "b"])));
        assert!(!command_matches(&v(&["a", "b"]), &v(&["a", "b", "c"])));
        assert!(!command_matches(&v(&["a", "b"]), &v(&["a"])));
    }

    #[test]
    fn subject_excludes_are_validated_and_merged_with_the_defaults() {
        let base = "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"a\"]\n";
        let p = parse(&format!(
            "{base}[subject]\nexclude=[\"*.log\", \".claude\", \"dist\"]\n"
        ))
        .unwrap();
        assert_eq!(
            p.effective_excludes(),
            vec!["*.log", ".claude", ".codex", ".cursor", ".opshero", "dist"]
        );
        assert_eq!(
            parse(base).unwrap().effective_excludes(),
            default_excludes()
        );
        for bad in ["\"\"", "\":(top)x\"", "\"!x\"", "\"a\\nb\""] {
            assert!(
                parse(&format!("{base}[subject]\nexclude=[{bad}]\n")).is_err(),
                "{bad}"
            );
        }
        assert!(parse(&format!("{base}[subject]\ntypo=1\n")).is_err());
    }

    #[test]
    fn assurance_is_ordered() {
        assert!(Assurance::A3 > Assurance::A2 && Assurance::A1 > Assurance::A0);
    }

    #[test]
    fn percentages_parse_exactly() {
        assert_eq!(parse_percent_ppm("5%"), Some(50_000));
        assert_eq!(parse_percent_ppm("12.5%"), Some(125_000));
        assert_eq!(parse_percent_ppm("0.01%"), Some(100));
        assert_eq!(parse_percent_ppm("99.9999%"), Some(999_999));
        for bad in [
            "", "5", "%", "0%", "100%", "101%", "-5%", "5.%", ".5%", "5.12345%", "5 %", "1e1%",
            "+5%", "٥%",
        ] {
            assert_eq!(parse_percent_ppm(bad), None, "{bad:?} must be rejected");
        }
    }

    #[test]
    fn repeat_rule_is_parsed_and_its_run_count_computed() {
        let p = parse(
            "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"a\"]\nrepeat={alpha=\"5%\",flake_rate=\"5%\"}\n",
        )
        .unwrap();
        let r = p.require[0].repeat.unwrap();
        assert_eq!((r.alpha_ppm, r.flake_ppm, r.runs), (50_000, 50_000, 59));
    }

    #[test]
    fn invalid_repeat_rules_are_rejected() {
        let base = "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"a\"]\n";
        for bad in [
            "repeat={alpha=\"5%\"}",
            "repeat={alpha=\"5%\",flake_rate=\"5%\",extra=1}",
            "repeat={alpha=0.05,flake_rate=\"5%\"}",
            "repeat={alpha=\"0%\",flake_rate=\"5%\"}",
            "repeat={alpha=\"5%\",flake_rate=\"100%\"}",
            "repeat={alpha=\"0.0001%\",flake_rate=\"0.0001%\"}",
        ] {
            assert!(
                parse(&format!("{base}{bad}\n")).is_err(),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn post_run_changes_parses_and_defaults_to_deny() {
        let base = "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"a\"]\n";
        assert_eq!(
            parse(base).unwrap().require[0].post_run_changes,
            PostRunChanges::Deny
        );
        for (v, want) in [
            ("allow", PostRunChanges::Allow),
            ("deny", PostRunChanges::Deny),
        ] {
            let p = parse(&format!("{base}post_run_changes=\"{v}\"\n")).unwrap();
            assert_eq!(p.require[0].post_run_changes, want);
        }
        for bad in ["\"Deny\"", "\"\"", "true", "1", "\"warn\""] {
            assert!(
                parse(&format!("{base}post_run_changes={bad}\n")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn paths_parse_default_to_empty_and_are_validated() {
        let base = "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"a\"]\n";
        assert!(parse(base).unwrap().require[0].paths.is_empty());
        let p = parse(&format!("{base}paths=[\"*.py\", \"src\"]\n")).unwrap();
        assert_eq!(p.require[0].paths, ["*.py", "src"]);
        for bad in [
            "[\"\"]",
            "[\":(top)x\"]",
            "[\"!x\"]",
            "[\"a\\u0000b\"]",
            "\"*.py\"",
            "[1]",
        ] {
            assert!(
                parse(&format!("{base}paths={bad}\n")).is_err(),
                "{bad} must be rejected"
            );
        }
        let many = format!(
            "[{}]",
            (0..65)
                .map(|i| format!("\"p{i}\""))
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(
            parse(&format!("{base}paths={many}\n")).is_err(),
            "more than 64 entries"
        );
    }

    #[test]
    fn run_gives_ci_a_concrete_command_and_must_match_the_pattern() {
        let base = |extra: &str| {
            format!(
                "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"pytest\",\"*\"]\n{extra}"
            )
        };
        let p = parse(&base("run=[\"pytest\",\"-q\"]\n")).unwrap();
        assert_eq!(
            p.require[0].run.as_deref(),
            Some(&["pytest".to_owned(), "-q".to_owned()][..])
        );
        assert_eq!(
            p.require[0].ci_command(),
            Some(vec!["pytest".to_owned(), "-q".to_owned()])
        );
        // A pattern with no wildcard is its own command.
        let q = parse("version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"make\",\"test\"]\n").unwrap();
        assert_eq!(
            q.require[0].ci_command(),
            Some(vec!["make".to_owned(), "test".to_owned()])
        );
        // A wildcard pattern without `run` cannot be executed.
        assert_eq!(parse(&base("")).unwrap().require[0].ci_command(), None);
        for bad in [
            "run=[]",
            "run=[\"pytest\",\"*\"]",
            "run=[\"cargo\",\"test\"]",
            "run=\"pytest\"",
            "run=[\"pytest\\u0000\"]",
        ] {
            assert!(
                parse(&base(&format!("{bad}\n"))).is_err(),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn an_exclusion_that_matches_everything_is_rejected() {
        let base = "version=1\nid=\"p\"\n[[require]]\nid=\"t\"\ncheck=\"evidence.fresh\"\ncommand=[\"a\"]\n[subject]\n";
        for bad in [
            "*", "**", ".", "./", "/", "**/*", "*/", "./*", "*/**", ".//",
        ] {
            let text = format!("{base}exclude=[\"{bad}\"]\n");
            assert!(
                parse(&text).is_err(),
                "{bad:?} would exclude the whole repository"
            );
        }
        for ok in [
            "*.log",
            "dist",
            ".venv",
            "build/*",
            "**/node_modules",
            "a.*",
            ".*cache",
        ] {
            let text = format!("{base}exclude=[\"{ok}\"]\n");
            assert!(parse(&text).is_ok(), "{ok:?} is a legitimate exclusion");
        }
    }

    /// The policy file dialect is a decision, not an accident of the parser version. Since the
    /// update to the `toml` crate 1.x the parser implements TOML 1.1: it accepts what TOML 1.0
    /// accepted, plus a few conveniences. It stays strict where it matters for a gate: duplicate
    /// keys and unknown keys are errors. If this test changes, RFC-0004 section "File format"
    /// must change with it.
    #[test]
    fn the_policy_dialect_is_toml_1_1_and_strict_about_duplicates_and_unknown_keys() {
        let head = "version = 1\nid = \"p\"\n";
        let req = "[[require]]\nid = \"t\"\ncheck = \"evidence.fresh\"\ncommand = [\"a\"]\n";
        // TOML 1.0, unchanged by the update.
        assert!(parse(&format!("{head}{req}")).is_ok());
        assert!(
            parse(&format!("{head}{req}").replace('\n', "\r\n")).is_ok(),
            "CRLF"
        );
        assert!(
            parse(&format!("\u{feff}{head}{req}")).is_ok(),
            "a byte order mark"
        );
        // TOML 1.1 conveniences, newly accepted.
        let multiline_inline =
            format!("{head}{req}repeat = {{\n  alpha = \"5%\",\n  flake_rate = \"5%\",\n}}\n");
        assert!(
            parse(&multiline_inline).is_ok(),
            "newlines and a trailing comma in an inline table"
        );
        assert!(
            parse(&format!(
                "{head}[[require]]\nid = \"t\\e\"\ncheck = \"evidence.fresh\"\ncommand = [\"a\"]\n"
            ))
            .is_ok(),
            "the \\e escape"
        );
        // Strictness that must never loosen.
        assert!(
            parse(&format!("{head}{req}id = \"again\"\n")).is_err(),
            "duplicate key"
        );
        assert!(
            parse(&format!("{head}{req}bogus = 1\n")).is_err(),
            "unknown key"
        );
        assert!(
            parse(&format!("{head}{head}{req}")).is_err(),
            "duplicate top-level keys"
        );
    }
}
