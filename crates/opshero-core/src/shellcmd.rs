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
                // ANSI-C (`$'...'`) and locale (`$"..."`) quoting follow different rules from
                // plain quotes (`\'` is an escaped quote inside `$'...'`), so a splitter that
                // does not know them reads the wrong boundaries. Not supported: refuse.
                b'$' if matches!(b.get(i + 1), Some(b'\'' | b'"')) => {
                    return Some((i, "unsupported quoting"));
                }
                // A word starting with `#` begins a comment that ends at the NEWLINE, whatever
                // a backslash before it says: what follows on the next line is a new command.
                b'#' if i == 0 || matches!(b[i - 1], b' ' | b'\t') => return Some((i, "comment")),
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

/// Environment variables that do not change *which* program or library runs or what options the
/// test runner receives. Everything else is refused: a run that sets an unlisted variable is not
/// recorded as evidence. An allowlist, not a denylist, because the list of variables that can
/// redirect a toolchain (`PATH`, `LD_*`, `RUSTFLAGS`, `CARGO_*`, `PYTEST_*`, `NPM_CONFIG_*`,
/// `GIT_*`, `*_ADDOPTS`, ...) has no end. The price is that an application variable such as
/// `MYAPP_ENV=test pytest` is not evidence either: the agent is nudged to run it plainly.
fn harmless_variable(name: &str) -> bool {
    const ALLOWED: &[&str] = &[
        "CI",
        "TERM",
        "TZ",
        "LANG",
        "LANGUAGE",
        "NO_COLOR",
        "FORCE_COLOR",
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "COLORTERM",
        "DEBUG",
        "VERBOSE",
        "NODE_ENV",
        "RUST_BACKTRACE",
        "RUST_LOG",
        "RUST_TEST_THREADS",
        "RUST_MIN_STACK",
        "CARGO_TERM_COLOR",
        "PYTHONUNBUFFERED",
        "PYTHONDONTWRITEBYTECODE",
        "PYTHONHASHSEED",
        "PYTHONUTF8",
        "PYTHONIOENCODING",
        "GOMAXPROCS",
    ];
    ALLOWED.contains(&name) || name.starts_with("LC_")
}

fn is_program_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '+' | '-' | ':' | '@' | '%')
}

/// Peel `NAME=value` words off the front of `s`, working on the raw text. Returns the names
/// and the remainder. A value may contain quotes and backslashes; it runs to the next
/// unquoted whitespace.
fn split_assignments(mut s: &str) -> (Vec<String>, &str) {
    let mut names = Vec::new();
    loop {
        s = s.trim_start_matches([' ', '\t']);
        let b = s.as_bytes();
        let mut i = 0;
        if !b
            .first()
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        {
            return (names, s);
        }
        while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
            i += 1;
        }
        if b.get(i) != Some(&b'=') {
            return (names, s);
        }
        let name = &s[..i];
        i += 1;
        let (mut single, mut double) = (false, false);
        while i < b.len() {
            let c = b[i];
            if single {
                single = c != b'\'';
            } else if double {
                if c == b'\\' {
                    i += 1;
                } else if c == b'"' {
                    double = false;
                }
            } else {
                match c {
                    b'\\' => i += 1,
                    b'\'' => single = true,
                    b'"' => double = true,
                    b' ' | b'\t' => break,
                    _ => {}
                }
            }
            i += 1;
        }
        names.push(name.to_owned());
        s = &s[i.min(s.len())..];
    }
}

pub fn analyze(command: &str) -> Analysis {
    // Bash separates words on space and tab only. Form feed, carriage return, vertical tab, no-break
    // space and the other Unicode spaces are ordinary characters inside a word for bash but
    // separators for a Unicode-aware splitter: `CI=1<FF>a` is one assignment (nothing runs, exit 0)
    // to bash and "run a" to a splitter. Refuse them, and every other control character, before
    // anything is parsed. (Newline is handled as an operator when unquoted.)
    if command.chars().any(|c| {
        (c.is_control() && c != '\t' && c != '\n')
            || (c.is_whitespace() && !matches!(c, ' ' | '\t' | '\n'))
    }) {
        return Analysis::Compound("unusual whitespace or control character");
    }
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
    // Leading assignments are recognised on the RAW text: `'A=b'` and `\A=b` are command
    // names in bash, not assignments, and a splitter that unquotes first cannot tell.
    let (env_names, rest) = split_assignments(rest);
    let Ok(words) = shell_words::split(rest) else {
        return Analysis::Compound("unparsable quoting");
    };
    if words.is_empty() {
        return Analysis::Compound("empty command");
    }
    if env_names.iter().any(|n| !harmless_variable(n)) {
        return Analysis::Compound("environment variable not on the allowlist");
    }
    // The program name must look like one. Bash splits `x->b` into the program `x-` and a
    // redirection, and `$x` or `*` expand to something else: a splitter that keeps them as
    // one word reports a program that is not the one that runs.
    if words[0].is_empty() || !words[0].chars().all(is_program_char) {
        return Analysis::Compound("unusual program name");
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
    fn only_allowlisted_variables_may_precede_a_command_that_counts_as_evidence() {
        for bad in [
            "PATH=/tmp/fake pytest",
            "LD_PRELOAD=/tmp/x.so pytest",
            "LD_LIBRARY_PATH=/tmp pytest",
            "DYLD_INSERT_LIBRARIES=/tmp/x pytest",
            "PYTHONPATH=/tmp pytest",
            "PYTEST_ADDOPTS=--co pytest",
            "PYTEST_PLUGINS=evil pytest",
            "NODE_OPTIONS=--require=/tmp/x npm test",
            "NPM_CONFIG_REGISTRY=http://x npm test",
            "RUSTC_WRAPPER=/tmp/w cargo test",
            "RUSTFLAGS=-Cx cargo test",
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=/tmp/r cargo test",
            "CARGO_BUILD_RUSTC=/tmp/r cargo test",
            "GIT_DIR=/tmp git status",
            "BASH_ENV=/tmp/e sh t.sh",
            "CI=1 PATH=/tmp pytest",
            "FOO=bar make test",
            "MYAPP_ENV=test pytest",
        ] {
            assert!(
                matches!(analyze(bad), Analysis::Compound(_)),
                "{bad} must not be recorded as evidence"
            );
        }
        for ok in [
            "CI=1 pytest",
            "RUST_BACKTRACE=1 cargo test",
            "LANG=C pytest -q",
            "LC_ALL=C.UTF-8 pytest",
            "CI=true NODE_ENV=test npm test",
            "PYTHONDONTWRITEBYTECODE=1 PYTHONHASHSEED=0 pytest",
        ] {
            assert!(
                matches!(analyze(ok), Analysis::Simple { .. }),
                "{ok} is an ordinary command"
            );
        }
        // An assignment with no command stays "empty command", not an allowlist message.
        assert_eq!(analyze("FOO=1"), Analysis::Compound("empty command"));
    }

    #[test]
    fn comments_quoted_assignments_and_empty_names_are_not_simple_commands() {
        // A comment ends at the newline even when it ends with a backslash: `a` is a second command.
        for bad in [
            "- #x\\\na",
            "pytest # note",
            "# only a comment",
            "a #b",
            "a\t#b",
        ] {
            assert!(
                matches!(analyze(bad), Analysis::Compound(_)),
                "{bad:?} must be refused"
            );
        }
        // Quoted or escaped `NAME=value` is a command NAME in bash, not an assignment.
        for bad in [
            "'v=1a=b' xf",
            "\\xv=1? bxx",
            "\"A=b\" pytest",
            "v=1 \"\"",
            "''",
        ] {
            assert!(
                matches!(analyze(bad), Analysis::Compound(_)),
                "{bad:?} must be refused"
            );
        }
        // A program name must look like one: bash splits `x->b` into `x-` and a redirection,
        // and `$x`, `*`, `>` are not program names at all.
        for bad in [
            "x->b", "$x", "*", ">", "a>b", "$'a'", "$\"a\"", "~", "{a,b}",
        ] {
            assert!(
                matches!(analyze(bad), Analysis::Compound(_)),
                "{bad:?} must be refused"
            );
        }
        // `#` inside a word, or as `$#`, is not a comment.
        for ok in ["pytest tests/a#b", "echo a#b", "pytest -k 'not #1'"] {
            assert!(
                matches!(analyze(ok), Analysis::Simple { .. }),
                "{ok:?} is an ordinary command"
            );
        }
        // Real assignments keep working, including quoted values.
        match analyze("CI=1 TZ='America/New York' pytest -q") {
            Analysis::Simple {
                env_names, argv, ..
            } => {
                assert_eq!(env_names, ["CI", "TZ"]);
                assert_eq!(argv, ["pytest", "-q"]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn only_space_and_tab_separate_words_and_other_control_or_space_characters_are_refused() {
        for bad in [
            "CI=1\x0ca",
            "CI=1\ra",
            "CI=1\x0bTZ=UTC a",
            "a\x0cb",
            "a\rb",
            "a\x0bb",
            "a\u{a0}b",
            "a\u{2003}b",
            "pytest\u{3000}-q",
            "pytest\u{a0}",
            "\u{a0}pytest",
            "pytest \x00",
            "pytest \x1b[0m",
            "pytest\x7f",
        ] {
            assert!(
                matches!(analyze(bad), Analysis::Compound(_)),
                "{bad:?} must be refused"
            );
        }
        for ok in [
            "pytest -q",
            "pytest\t-q",
            "CI=1\tpytest",
            "pytest -k 'a b'",
            "echo 'two\nlines'",
        ] {
            assert!(
                matches!(analyze(ok), Analysis::Simple { .. }),
                "{ok:?} is an ordinary command"
            );
        }
    }
}
