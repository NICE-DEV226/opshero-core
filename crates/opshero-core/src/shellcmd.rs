//! Conservative analysis of a shell command line (pure).
//!
//! An agent hands us one shell string. Evidence is only trustworthy for a
//! **simple command** — one program with arguments, optionally preceded by
//! environment assignments and a single `cd DIR &&`. Pipelines, lists and
//! substitutions are rejected because their exit status can mask a failure
//! (`pytest | tail` exits with the status of `tail`).

#[derive(Debug, PartialEq, Eq)]
pub enum Analysis {
    Simple {
        env_names: Vec<String>,
        argv: Vec<String>,
        /// Directory from a leading `cd DIR &&`, if any (as written).
        cd: Option<String>,
    },
    /// Not usable as evidence; the reason is for diagnostics only.
    Compound(&'static str),
}

/// First shell operator that is not inside quotes. `2>&1`, `>&2` and `&>`
/// are redirections, not control operators, and are allowed.
fn first_unquoted_operator(s: &str) -> Option<(usize, &'static str)> {
    let b = s.as_bytes();
    let (mut i, mut single, mut double) = (0usize, false, false);
    while i < b.len() {
        let c = b[i];
        if single {
            single = c != b'\'';
        } else if double {
            if c == b'\\' {
                i += 1;
            } else if c == b'"' {
                double = false;
            } else if c == b'`' || (c == b'$' && b.get(i + 1) == Some(&b'(')) {
                return Some((i, "command substitution"));
            }
        } else {
            match c {
                b'\\' => i += 1,
                b'\'' => single = true,
                b'"' => double = true,
                b'`' => return Some((i, "command substitution")),
                b'$' if b.get(i + 1) == Some(&b'(') => return Some((i, "command substitution")),
                b'\n' => return Some((i, "newline")),
                b';' => return Some((i, ";")),
                b'(' | b')' => return Some((i, "subshell")),
                b'|' => {
                    return Some((
                        i,
                        if b.get(i + 1) == Some(&b'|') {
                            "||"
                        } else {
                            "pipe"
                        },
                    ));
                }
                b'&' => {
                    let redirect = (i > 0 && b[i - 1] == b'>') || b.get(i + 1) == Some(&b'>');
                    if !redirect {
                        return Some((
                            i,
                            if b.get(i + 1) == Some(&b'&') {
                                "&&"
                            } else {
                                "&"
                            },
                        ));
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Variables that decide *which* program or library runs, or inject options into the test
/// runner. A run that sets one of them is not evidence that the real tests ran, so it is
/// not recorded. Ordinary variables (`CI=1`, `RUST_BACKTRACE=1`, `LANG=C`) are fine.
fn changes_how_programs_run(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "PATH",
        "IFS",
        "SHELL",
        "ENV",
        "BASH_ENV",
        "PYTHONPATH",
        "PYTHONHOME",
        "PYTHONSTARTUP",
        "NODE_OPTIONS",
        "NODE_PATH",
        "RUBYOPT",
        "RUBYLIB",
        "PERL5OPT",
        "PERL5LIB",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_RUSTC",
        "CLASSPATH",
        "JAVA_TOOL_OPTIONS",
        "GOFLAGS",
    ];
    EXACT.contains(&name)
        || name.starts_with("LD_")
        || name.starts_with("DYLD_")
        || name.ends_with("_ADDOPTS")
}

fn is_assignment(word: &str) -> Option<&str> {
    let (name, _) = word.split_once('=')?;
    let mut chars = name.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic() || first == '_')
        .then_some(())
        .filter(|_| chars.all(|c| c.is_ascii_alphanumeric() || c == '_'))
        .map(|_| name)
}

pub fn analyze(command: &str) -> Analysis {
    let command = command.trim();
    let (cd, rest) = match first_unquoted_operator(command) {
        Some((pos, "&&")) if command.starts_with("cd ") => {
            let head = shell_words::split(&command[..pos]).unwrap_or_default();
            match head.as_slice() {
                [cd, dir] if cd == "cd" && !dir.contains(['$', '~', '`']) => {
                    (Some(dir.clone()), command[pos + 2..].trim())
                }
                _ => return Analysis::Compound("unsupported cd prefix"),
            }
        }
        _ => (None, command),
    };
    if let Some((_, what)) = first_unquoted_operator(rest) {
        return Analysis::Compound(what);
    }
    let Ok(mut words) = shell_words::split(rest) else {
        return Analysis::Compound("unparsable quoting");
    };
    let mut env_names = Vec::new();
    while let Some(name) = words.first().and_then(|w| is_assignment(w)) {
        if changes_how_programs_run(name) {
            return Analysis::Compound("environment variable that changes how programs run");
        }
        env_names.push(name.to_owned());
        words.remove(0);
    }
    if words.is_empty() {
        return Analysis::Compound("empty command");
    }
    Analysis::Simple {
        env_names,
        argv: words,
        cd,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn simple(cmd: &str) -> (Vec<String>, Vec<String>, Option<String>) {
        match analyze(cmd) {
            Analysis::Simple {
                env_names,
                argv,
                cd,
            } => (env_names, argv, cd),
            other => panic!("expected simple for {cmd:?}, got {other:?}"),
        }
    }

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn plain_commands_are_split_like_a_shell() {
        assert_eq!(
            simple("pytest -q tests/"),
            (vec![], v(&["pytest", "-q", "tests/"]), None)
        );
        assert_eq!(
            simple(r#"pytest -k "a or b" 'x y'"#).1,
            v(&["pytest", "-k", "a or b", "x y"])
        );
    }

    #[test]
    fn env_assignments_and_cd_prefix_are_understood() {
        let (env, argv, cd) = simple("CI=true NODE_ENV=test npm test");
        assert_eq!(
            (env, argv, cd),
            (v(&["CI", "NODE_ENV"]), v(&["npm", "test"]), None)
        );
        let (env, argv, cd) = simple("cd backend && pytest -q");
        assert_eq!(
            (env, argv, cd),
            (vec![], v(&["pytest", "-q"]), Some("backend".into()))
        );
    }

    #[test]
    fn redirections_to_files_and_stderr_merge_are_fine() {
        assert_eq!(simple("pytest 2>&1").1, v(&["pytest", "2>&1"]));
        assert_eq!(simple("pytest > out.txt").1, v(&["pytest", ">", "out.txt"]));
        assert_eq!(
            simple("cargo test &> log").1,
            v(&["cargo", "test", "&>", "log"])
        );
    }

    #[test]
    fn operators_that_can_mask_a_failure_are_rejected() {
        for bad in [
            "pytest | tail -5",
            "pytest; echo done",
            "pytest || true",
            "pytest && echo ok",
            "cd a && pytest | tail",
            "pytest &",
            "echo $(pytest)",
            "echo `pytest`",
            "(pytest)",
            "pytest\nls",
            "cd a && cd b && pytest",
            "cd $HOME && pytest",
        ] {
            assert!(
                matches!(analyze(bad), Analysis::Compound(_)),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn operators_inside_quotes_are_just_text() {
        assert_eq!(
            simple(r#"pytest -k "a|b;c && d""#).1,
            v(&["pytest", "-k", "a|b;c && d"])
        );
        assert_eq!(simple("grep '$(x)' f").1, v(&["grep", "$(x)", "f"]));
    }

    #[test]
    fn degenerate_inputs_do_not_panic() {
        for s in [
            "", "   ", "'", "\"", "FOO=1", "cd", "cd x &&", "&&", "|", "\\",
        ] {
            let _ = analyze(s);
        }
        assert!(matches!(
            analyze("FOO=1"),
            Analysis::Compound("empty command")
        ));
    }

    #[test]
    fn variables_that_change_how_programs_run_are_not_evidence() {
        for bad in [
            "PATH=/tmp/fake pytest",
            "LD_PRELOAD=/tmp/x.so pytest",
            "LD_LIBRARY_PATH=/tmp pytest",
            "DYLD_INSERT_LIBRARIES=/tmp/x pytest",
            "PYTHONPATH=/tmp pytest",
            "PYTEST_ADDOPTS=--co pytest",
            "NODE_OPTIONS=--require=/tmp/x npm test",
            "RUSTC_WRAPPER=/tmp/w cargo test",
            "BASH_ENV=/tmp/e sh t.sh",
            "CI=1 PATH=/tmp pytest",
        ] {
            assert!(
                matches!(analyze(bad), Analysis::Compound(_)),
                "{bad} must not be recorded as evidence"
            );
        }
        for ok in [
            "CI=1 pytest",
            "RUST_BACKTRACE=1 cargo test",
            "FOO=bar make test",
            "LANG=C pytest -q",
        ] {
            assert!(
                matches!(analyze(ok), Analysis::Simple { .. }),
                "{ok} is an ordinary command"
            );
        }
    }
}
