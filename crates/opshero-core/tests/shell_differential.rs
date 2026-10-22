//! Differential test of the shell command analyzer against bash itself.
//!
//! The analyzer decides which commands count as evidence. Its worst failure is a
//! *parser differential*: it says "one simple command" while bash runs something else
//! (a hidden second command, a different program). For every input the analyzer calls
//! `Simple`, this test runs the input in a real bash, with every unknown command
//! intercepted by `command_not_found_handle`, and requires that bash executed at most one
//! command, that if it ran one its program name is the one the analyzer reported, and that if
//! it ran none the shell reported a failure (no false success).
//!
//! Safety of the harness: the alphabet contains no parentheses or braces (so no function can
//! be defined or recursed), the only words are `a b c f` (none exists on PATH, so each call
//! goes to the harmless handler), and each run happens in a throw-away directory with a
//! timeout and an empty environment.
//!
//! `OPSHERO_REQUIRE_BASH=1` turns a missing bash into a failure instead of a loud skip.
//! `OPSHERO_SHELL_ITERS` raises the number of random inputs (default 4000) and
//! `OPSHERO_SHELL_SEED` changes the sequence.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use opshero_core::shellcmd::{Analysis, analyze};
use std::process::{Command, Stdio};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Absolute path of a program, resolved with the *parent's* PATH: the child runs with an
/// empty one on purpose.
fn locate(name: &str) -> Option<std::path::PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// Names bash handles itself (builtins and keywords): they never reach `command_not_found_handle`,
/// so this harness cannot observe them. Asked from bash, not guessed.
fn bash_builtins() -> &'static std::collections::HashSet<String> {
    static SET: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    SET.get_or_init(|| {
        let Some(bash) = locate("bash") else {
            return Default::default();
        };
        let out = Command::new(bash)
            .args(["--noprofile", "--norc", "-c", "compgen -b; compgen -k"])
            .output();
        out.map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
    })
}

fn bash_available() -> bool {
    Command::new("bash")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One private sandbox per test thread, reused for every call: an empty working directory (so globs
/// expand to nothing) and a log kept outside it. It is emptied before each call and removed when
/// the thread ends, so a run, however it ends, leaves nothing behind (an earlier version created
/// three files per call and exhausted the inodes of /tmp).
struct Sandbox {
    base: std::path::PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let id: String = format!("{:?}", std::thread::current().id())
            .chars()
            .filter(char::is_ascii_digit)
            .collect();
        let base =
            std::env::temp_dir().join(format!("opshero-shelldiff-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::create_dir_all(base.join("work"));
        Sandbox { base }
    }

    /// Empty the working directory and the log. Returns false if the sandbox is unusable.
    fn reset(&self) -> bool {
        let work = self.base.join("work");
        if let Ok(entries) = std::fs::read_dir(&work) {
            for e in entries.flatten() {
                let p = e.path();
                let _ = std::fs::remove_file(&p).or_else(|_| std::fs::remove_dir_all(&p));
            }
        } else if std::fs::create_dir_all(&work).is_err() {
            return false;
        }
        let _ = std::fs::remove_file(self.base.join("calls.log"));
        true
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

thread_local! {
    static SANDBOX: Sandbox = Sandbox::new();
}

/// What bash executed: each unknown command is appended to a log file (not stdout, which the
/// input itself may redirect) as `CALL\0prog\0args...END\0`. Also returns bash's exit status.
fn run_in_bash(input: &str) -> Option<(Vec<Vec<String>>, i32)> {
    SANDBOX.with(|sb| {
        if !sb.reset() {
            return None;
        }
        let work = sb.base.join("work");
        let log = sb.base.join("calls.log");
        let script = format!(
            "command_not_found_handle() {{ {{ printf 'CALL\\0%s\\0' \"$1\"; shift; for x in \"$@\"; do printf '%s\\0' \"$x\"; done; printf 'END\\0'; }} >> '{}'; return 0; }}\n{input}\n",
            log.display()
        );
        let (timeout, bash) = (locate("timeout")?, locate("bash")?);
        let status = Command::new(timeout)
            .arg("3")
            .arg(bash)
            .args(["--noprofile", "--norc", "-c", &script])
            .current_dir(&work)
            .env_clear()
            .env("PATH", "/nonexistent")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok()?;
        let exit = status.code()?;
        if exit == 124 {
            return None; // timed out: the harness, not the input, failed
        }
        let text = std::fs::read(&log)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        let mut calls = Vec::new();
        for call in text.split("END\0").filter(|c| !c.is_empty()) {
            let mut parts: Vec<String> = call.split('\0').map(str::to_owned).collect();
            if parts.first().map(String::as_str) != Some("CALL") {
                return None;
            }
            parts.remove(0);
            while parts.last().is_some_and(String::is_empty) {
                parts.pop();
            }
            calls.push(parts);
        }
        Some((calls, exit))
    })
}

fn check(input: &str) -> Result<(), String> {
    let Analysis::Simple { argv, .. } = analyze(input) else {
        return Ok(()); // the analyzer refused: always safe
    };
    // Bash runs a program given with a slash directly, without consulting the
    // `command_not_found_handle` used to observe it: this harness cannot see such a run.
    if argv.first().is_some_and(|p| p.contains('/')) {
        return Ok(());
    }
    // Same for builtins and keywords (`fc`, `cd`, `exit`, `if`, ...): handled inside bash.
    if argv.first().is_some_and(|p| bash_builtins().contains(p)) {
        return Ok(());
    }
    let (calls, exit) = run_in_bash(input).ok_or_else(|| format!("harness failed on {input:?}"))?;
    // More than one command, or a different program, is a forgery risk.
    if calls.len() > 1 {
        return Err(format!(
            "analyzer said ONE simple command {argv:?}, but bash executed {} commands: {calls:?}\n  input: {input:?}",
            calls.len()
        ));
    }
    if calls.is_empty() {
        // Nothing ran. That is only safe if the shell reported a failure: an assignment alone
        // (`v=1<FF>a`) runs nothing and exits 0, which would record a run that never happened.
        return if exit != 0 {
            Ok(())
        } else {
            Err(format!(
                "analyzer said ONE simple command {argv:?}, but bash ran nothing and exited 0 (a false success)\n  input: {input:?}"
            ))
        };
    }
    if calls[0].first() != argv.first() {
        return Err(format!(
            "analyzer said program {:?}, bash ran {:?}\n  input: {input:?}",
            argv.first(),
            calls[0].first()
        ));
    }
    Ok(())
}

/// Inputs that are classic differential traps between a POSIX-style word splitter and bash.
const TRAPS: &[&str] = &[
    r"a $'x\'; b '",
    r"a $'\'' ; b '",
    r#"a $"x\"; b ""#,
    r"a \' ; b \'",
    r"a 'x' ; b 'y'",
    "a x\\\ny",
    "a $'a\\nb'",
    r"a $'\x3b b'",
    // Balanced: bash reads `$'\''` as one quote character, then `;` ends the command and runs `b`.
    r"a $'\'' ; b ' '",
    r"a $'\'' ; b '' ",
    r#"a $'\'' ; b "" "#,
    // Balanced for a POSIX splitter AND two commands for bash (the escaped apostrophe at the
    // end restores the parity): the exploit shape from the security review.
    r"a $'\'' ; b \'",
    r"a $'\'' ; evil \'",
    r"pytest $'\'' ; rm \'",
    r"a $'; b'",
    r#"a "x`b`""#,
    r"a x$'y'z",
    r"a #; b",
    r"a '#' ; b",
    "a >& 2",
    "a 2>&1",
    "a &> out",
    "v=1 a b",
    "v=1 w=2 a",
    "a \\\n; b",
    r"a ${x:-$'\'' ; b '}",
    r"a $'\'$'\'' ; b '",
    "a \t b",
    "- #$x\\\na",
    "'v=1a=b'\\\n \\xf",
    "\\xv=1? bxx",
    "'A=b' a",
    "a #b\nc",
    // Bash separates words only on space and tab; these are ordinary characters in a word.
    "CI=1\x0ca",
    "CI=1\ra",
    "CI=1\u{a0}a",
    "a\x0cb",
    "a\rb",
    "a\x0bb",
    "a\u{a0}b",
    "a\u{2003}b",
    "CI=1\u{3000}a",
    "a\x0c#b",
    "a \x0c; b",
    "CI=1\x0bTZ=UTC\x0ca",
    // A redirection ends an assignment value: nothing runs, exit status 0.
    "TZ=UTC>> c",
    "TZ=UTC/> f",
    "LANG=C/>> b\n",
    "CI=1< x a",
    // Line continuation: bash deletes `\<newline>`, so the `#` that follows is a comment start and
    // the next line is a SECOND command. The apostrophe in the comment and the escaped one on the
    // last line cancel out for a POSIX splitter, which therefore sees balanced quotes.
    "a \\\n#'\nc \\'",
    "a \\\n#'\nc",
    "a \\\n#\nc",
    "a \\\n# x\nevil",
    "pytest \\\n#'\nrm \\'",
    "a b\\\n #'\nc \\'",
    "TZ=UTC\ta",
    "CI=1 \x0c a",
    "a '#' b",
];

#[test]
fn known_traps_never_turn_into_a_simple_command_that_bash_runs_differently() {
    if !bash_available() {
        assert!(
            std::env::var("OPSHERO_REQUIRE_BASH").is_err(),
            "bash is required"
        );
        eprintln!("SKIPPED: bash not found; the differential test did not run");
        return;
    }
    let failures: Vec<String> = TRAPS.iter().filter_map(|t| check(t).err()).collect();
    assert!(
        failures.is_empty(),
        "{} differential(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn random_inputs_are_never_misread_as_one_simple_command() {
    if !bash_available() {
        assert!(
            std::env::var("OPSHERO_REQUIRE_BASH").is_err(),
            "bash is required"
        );
        eprintln!("SKIPPED: bash not found");
        return;
    }
    let iters: usize = std::env::var("OPSHERO_SHELL_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4000);
    // Mostly words and quoting characters, so that many inputs are classified `Simple`.
    let atoms: &[&str] = &[
        "a",
        "b",
        "c",
        "f",
        "a",
        "b",
        " ",
        " ",
        " ",
        "'",
        "'",
        "\"",
        "\"",
        "\\",
        "$",
        "$'",
        "$\"",
        "`a`",
        "$(a)",
        ";",
        "&",
        "&&",
        "|",
        "||",
        ">",
        "<",
        ">>",
        "2>&1",
        "#",
        "~",
        "*",
        "?",
        "\n",
        "=",
        "v=1",
        "x",
        "$x",
        "\\\n",
        ",",
        "-",
        "/",
        "a=b",
        // Allowlisted assignments (a name that is not allowlisted is refused before anything else).
        "CI=1",
        "TZ=UTC",
        "LANG=C",
        // Characters that bash does NOT treat as word separators, but a Unicode-aware splitter does.
        "\x0c",
        "\r",
        "\x0b",
        "\u{a0}",
        "\u{2003}",
        "\u{3000}",
        // Composites, so that rare combinations (a continuation, then a comment, then a command)
        // are reachable by chance.
        "\\\n#",
        " #",
        "#'",
        "\\'",
        "\\\n \\\n",
        "\n#",
    ];
    // The campaign is only worth running if the alphabet really contains what it claims to.
    for needed in ["CI=1", "\x0c", "\u{a0}", "\\\n#"] {
        assert!(atoms.contains(&needed), "alphabet is missing {needed:?}");
    }
    let seed: u64 = std::env::var("OPSHERO_SHELL_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0xD1FF_0001);
    let mut rng = Rng(seed);
    let mut simple = 0usize;
    let mut failures = Vec::new();
    for _ in 0..iters {
        let n = 1 + rng.below(9);
        let input: String = (0..n).map(|_| atoms[rng.below(atoms.len())]).collect();
        if matches!(analyze(&input), Analysis::Simple { .. }) {
            simple += 1;
        }
        if let Err(e) = check(&input) {
            failures.push(e);
            if failures.len() >= 5 {
                break;
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} differential(s) found:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        simple >= iters / 50,
        "too few inputs reached the check ({simple} of {iters}): the generator is not useful"
    );
}
