#!/usr/bin/env python3
"""Mutation check: inject known defects and require the test suite to notice.

For each mutant the script (1) replaces an exact string in a source file and fails if the
string is not found (a stale mutant must not pass silently), (2) runs the named tests,
(3) requires that the code COMPILES and that at least one test FAILS, (4) restores the file.
A mutant that survives means a test is missing or too weak. Equivalent mutants (no
observable effect) are listed separately and must be explained.

Usage: scripts/mutation-check.py [name-substring ...]     (stdlib only, Linux/macOS)
Needs: cargo; node (OPSHERO_REQUIRE_NODE=1 is set for the canonical JSON mutant).
"""
import os, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
V = "crates/opshero-core/src/verify.rs"
LAWS = ["cargo", "test", "-p", "opshero-core", "--test", "verdict_laws"]
STATS = ["cargo", "test", "-p", "opshero-core", "stats"]
ROBUST = ["cargo", "test", "-p", "opshero-core", "--test", "robustness"]
CANON = ["cargo", "test", "-p", "opshero-core", "--test", "canon_differential"]
GIT = ["cargo", "test", "-p", "opshero-cli", "--bin", "opshero", "git::"]
JOURNAL = ["cargo", "test", "-p", "opshero-core", "journal"]
CHAIN = ["cargo", "test", "-p", "opshero-cli", "--bin", "opshero", "concurrent_recorders"]
E2E = ["cargo", "test", "-p", "opshero-cli", "--test", "e2e"]
G = "crates/opshero-cli/src/git.rs"
J = "crates/opshero-core/src/journal.rs"
M = "crates/opshero-cli/src/main.rs"
C = "crates/opshero-cli/src/claude.rs"
JA = "crates/opshero-cli/src/journal.rs"
O = "crates/opshero-cli/src/ops.rs"
EV = "crates/opshero-cli/src/evidence.rs"
SH = "crates/opshero-core/src/shellcmd.rs"
SHELL_T = ["cargo", "test", "-p", "opshero-core", "--lib", "shellcmd"]
SHELL_DIFF = ["cargo", "test", "-p", "opshero-core", "--test", "shell_differential"]
POL = "crates/opshero-core/src/policy.rs"
INSTALL_T = ["sh", "scripts/test-install.sh"]
DOC = "crates/opshero-cli/src/doctor.rs"
DOCTOR_T = ["cargo", "test", "-p", "opshero-cli", "doctor"]
CLIDOC = ["python3", "scripts/gen-cli-docs.py", "--check"]
POLICY_T = ["cargo", "test", "-p", "opshero-core", "--lib", "policy"]
ST = "crates/opshero-cli/src/store.rs"
OPS = ["cargo", "test", "-p", "opshero-cli", "--bin", "opshero", "ops::"]
VERIFY = ["cargo", "test", "-p", "opshero-core", "--lib", "verify"]

MUTANTS = [
    # name, file, old, new, test command
    ("verify: ignore subject exclusions", V, "&& r.body.subject.exclude == subject.exclude", "&& true", LAWS),
    ("verify: compare tree before instead of after", V, "r.body.subject.tree_after == subject.worktree_tree", "r.body.subject.worktree_tree == subject.worktree_tree", LAWS),
    ("verify: ignore the signal", V, "r.body.execution.exit_code == Some(0) && r.body.execution.signal.is_none()", "r.body.execution.exit_code == Some(0)", LAWS),
    ("verify: ignore minimum assurance", V, ".filter(|r| assurance_of(r) >= req.min_assurance)", ".filter(|_r| true)", LAWS),
    ("verify: UNKNOWN outranks FAIL", V, "if blocking().any(|c| c.result == Verdict::Fail) {", "if blocking().any(|c| c.result == Verdict::Unknown) {", LAWS),
    ("repeat: no inconsistency check", V, "if failed > 0 {", "if failed > usize::MAX {", LAWS),
    ("repeat: off by one on N", V, "if current_ok.len() < need {", "if current_ok.len() + 1 < need {", LAWS),
    ("repeat: count passes ignoring assurance", V, "if current_ok.len() < need {", "if current_pass.len() < need {", LAWS),
    ("repeat: no de-duplication by id", V, ".filter(|r| seen.insert(r.id.as_str()))", ".filter(|r| seen.insert(r.id.as_str()) || true)", LAWS),
    ("repeat: rule ignored", V, "if let Some(rep) = req.repeat", "if let Some(rep) = req.repeat.filter(|_| false)", LAWS),
    ("repeat: failures miscounted", V, "let failed = current.len() - current_pass.len();", "let failed = 0usize;", LAWS),
    ("deny: option ignored", V, "let deny = req.post_run_changes == PostRunChanges::Deny;", "let deny = false && req.post_run_changes == PostRunChanges::Deny;", LAWS),
    ("deny: compares the wrong fields", V, "!deny || r.body.subject.worktree_tree == r.body.subject.tree_after", "!deny || r.body.subject.head_commit == r.body.subject.head_commit", LAWS),
    ("deny: moved runs never reported", V, "if moved.iter().any(|r| passed(r)) {", "if false && moved.iter().any(|r| passed(r)) {", LAWS),
    ("canon: sort keys by scalar value", "crates/opshero-core/src/canon.rs", "keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));", "keys.sort();", CANON),
    ("stats: strict comparison", "crates/opshero-core/src/stats.rs", "if pow <= limit", "if pow < limit", STATS),
    ("stats: loop starts at zero", "crates/opshero-core/src/stats.rs", "for n in 1..=MAX_RUNS", "for n in 0..=MAX_RUNS", STATS),
    ("stats: wrong complement", "crates/opshero-core/src/stats.rs", "let q = SCALE - p1 * (SCALE / PPM); // 1 - p1, scaled", "let q = p1 * (SCALE / PPM);", STATS),
    ("record: accept a mismatching id", "crates/opshero-core/src/record.rs", "return Err(IntegrityError::IdMismatch { claimed, actual });", "let _ = (claimed, actual);", ROBUST),
    ("timefmt: wrong month constant", "crates/opshero-core/src/timefmt.rs", "(5 * doy + 2) / 153", "(5 * doy + 2) / 154", ROBUST),
    ("policy: unsorted exclusions", "crates/opshero-core/src/policy.rs", "        all.sort();\n", "        // all.sort();\n", ROBUST),
    ("journal: success threshold is strict", J, "if found >= SUCCESS_PPM * sum.sessions {", "if found > SUCCESS_PPM * sum.sessions {", JOURNAL),
    ("journal: kill threshold is inclusive", J, "} else if found < KILL_PPM * sum.sessions {", "} else if found <= KILL_PPM * sum.sessions {", JOURNAL),
    ("journal: earliest label wins", J, "if *line >= slot.0 {", "if *line <= slot.0 {", JOURNAL),
    ("journal: session id stored unhashed", J, 'digest.iter().take(6).map(|b| format!("{b:02x}")).collect()', "raw.chars().take(12).collect()", JOURNAL),
    ("journal: labels on non-blocks counted", J, "None => sum.labels_ignored += 1,", "None => sum.blocks_real += 0,", JOURNAL),
    ("journal(cli): logs even when disabled", JA, "    if !enabled(git_dir) {\n        return;\n    }\n    if let Err(e) = append", "    if let Err(e) = append", E2E),
    ("journal(cli): any line can be labelled", M, "if !is_block {", "if false {", E2E),
    ("journal(cli): second stop not logged", C, "if stop_active && !journaling {", "if stop_active {", E2E),
    ("journal(cli): stop always logged as allow", C, "decision: if reason.is_some() {\n                Decision::Block\n            } else {\n                Decision::Allow\n            },", "decision: Decision::Allow,", E2E),
    ("concurrency: no lock on the evidence chain", O, 'let _chain = crate::lock::acquire(&dir.join("store.flock"), CHAIN_LOCK_WAIT);', "let _chain: Option<()> = None;", CHAIN),
    ("concurrency: no lock on the private index", G, '    let Some(_guard) = crate::lock::acquire(Path::new(&lock_name), INDEX_LOCK_WAIT) else {\n        return Err("the private index is busy".into());\n    };', "    let _ = (&lock_name, INDEX_LOCK_WAIT);", GIT),
    ("chain depth: off by one on known parents", V, "base = *d + 1;", "base = *d;", VERIFY),
    ("prune: protected records deleted anyway", ST, "            if protect(r) {", "            if false && protect(r) {", OPS),
    ("prune: dry run deletes", ST, "            if !dry_run {", "            if true {", OPS),
    ("prune: keeps one fewer than asked", ST, "            .skip(keep)\n", "            .skip(keep + 1)\n", OPS),
    ("prune: automatic bound disabled", O, "    if store.count() > prune_above {", "    if false && store.count() > prune_above {", OPS),
    ("gc: current tree not protected", M, "r.body.subject.tree_after == tree && r.body.subject.exclude == exclude\n    };\n    let report", "false\n    };\n    let report", E2E),
    ("turn: unchanged turns are judged anyway", C, "    if now == base.tree {", "    if false && now == base.tree {", E2E),
    ("turn: paths ignored (every change applies)", C, "map_or(true, |p| !p.is_empty())", "map_or(true, |_p| true)", E2E),
    ("turn: prompt hook not installed", "crates/opshero-cli/src/install.rs", '        targets.push(("UserPromptSubmit", None, 10));\n', "", E2E),
    ("turn: baselines of all sessions share one file", "crates/opshero-cli/src/baseline.rs", 'dir(git_dir).join(format!("{key}.json"))', 'dir(git_dir).join("shared.json")', E2E),
    ("journal: judged sessions not separated", J, 'if verdict != "UNCHANGED" && verdict != "NOT-APPLICABLE" {', "if true {", JOURNAL),
    ("transport: published records of other trees included", EV, "r.body.subject.tree_after == tree && r.body.subject.exclude == exclude", "r.body.subject.exclude == exclude", E2E),
    ("transport: remote evidence not merged on push", EV, "    if let Some(commit) = fetch_ref(root, remote, tree)? {", "    if let Some(commit) = fetch_ref(root, remote, tree).ok().flatten().filter(|_| false) {", E2E),
    ("transport: a missing ref is reported as a fetch failure", EV, "    msg.contains(\"couldn't find remote ref\")", "    false && msg.contains(\"couldn't find remote ref\")", E2E),
    ("transport: forged records accepted on import", EV, "match record::parse_verified(&raw) {", "match record::parse_verified(&raw).or_else(|_| serde_json::from_value::<Record>(raw.clone()).map_err(|e| record::IntegrityError::Malformed(e.to_string()))) {", E2E),
    ("transport: verify --ci does not fetch", M, "    if let Some(remote) = ci_remote {", "    if let Some(remote) = ci_remote.filter(|_| false) {", E2E),
    ("transport: plain verify fetches too", M, "    if let Some(remote) = ci_remote {", '    if let Some(remote) = ci_remote.or(Some("origin")) {', E2E),
    ("A2: ci run records the evidence as A1", M, 'exec_recorded(cmd, "A2", Some("ci"))', 'exec_recorded(cmd, "A1", Some("ci"))', E2E),
    ("A2: published evidence keeps the assurance it claims", "crates/opshero-cli/src/store.rs", '                rec.body.recorder.assurance = "A1".to_owned();', "                let _ = &rec;", E2E),
    ("A2: ci run allowed outside CI", M, 'if !allow_local && std::env::var_os("CI").is_none_or(|v| v.is_empty()) {', 'if false && !allow_local {', E2E),
    ("A2: a wildcard pattern is executed as written", POL, '(self.command.last().map(String::as_str) != Some("*")).then(|| self.command.clone())', "Some(self.command.clone())", POLICY_T),
    ("A2: run need not match the command pattern", POL, "|| !command_matches(&r.command, run);", ";", POLICY_T),
    ("hardening: a run outside the repository is recorded as inside", O, "let rel = d.strip_prefix(r).ok()?;", "let rel = d.strip_prefix(r).unwrap_or(std::path::Path::new(\"\"));", E2E),
    ("hardening: an exclusion matching everything is accepted", POL, "if e.chars().all(|c| matches!(c, '*' | '.' | '/')) {", "if false {", POLICY_T),
    ("hardening: no warning for a policy from the checked-out content", M, "    if ops::is_inside_repo(root, policy_file) {", "    if false && ops::is_inside_repo(root, policy_file) {", E2E),
    ("hardening: oversized published blob not skipped by size", EV, "        if size > MAX_RECORD_BYTES {", "        if false && size > MAX_RECORD_BYTES {", E2E),
    ("shell: ANSI-C quoting $'...' accepted", SH, "b'$' if matches!(b.get(i + 1), Some(b'\\'' | b'\"')) => {", "b'$' if false && matches!(b.get(i + 1), Some(b'\\'' | b'\"')) => {", SHELL_DIFF),
    ("shell: comments not detected", SH, "b'#' if i == 0 || matches!(b[i - 1], b' ' | b'\\t' | b'\\n') =>", "b'#' if false && (i == 0 || matches!(b[i - 1], b' ' | b'\\t' | b'\\n')) =>", SHELL_T),
    ("shell: variables not checked against the allowlist", SH, "if env_names.iter().any(|n| !harmless_variable(n)) {", "if false {", SHELL_T),
    ("shell: any word accepted as program name", SH, "if words[0].is_empty() || !words[0].chars().all(is_program_char) {", "if false {", SHELL_T),
    ("shell: assignments recognised after unquoting", SH, "let (env_names, rest) = split_assignments(rest);", 'let (env_names, rest): (Vec<String>, &str) = (vec![], rest);', SHELL_T),
    ("shell: other whitespace allowed as a separator", SH, "|| (c.is_whitespace() && !matches!(c, ' ' | '\\t' | '\\n'))", "|| false", SHELL_T),
    ("shell: control characters allowed", SH, "(c.is_control() && c != '\\t' && c != '\\n')", "false", SHELL_T),
    ("shell: backslash-newline continuation accepted outside quotes", SH, "b'\\\\' if b.get(i + 1) == Some(&b'\\n') => {", "b'\\\\' if false && b.get(i + 1) == Some(&b'\\n') => {", SHELL_T),
    ("shell: continuation accepted inside double quotes", SH, "                if b.get(i + 1) == Some(&b'\\n') {\n                    return Some((i, \"line continuation\"));", "                if false && b.get(i + 1) == Some(&b'\\n') {\n                    return Some((i, \"line continuation\"));", SHELL_T),
    ("shell: a redirection does not end an assignment value", SH, "b' ' | b'\\t' | b'<' | b'>' => break,", "b' ' | b'\\t' => break,", SHELL_DIFF),
    ("policy-ref: the ref is ignored and the checked-out file is used", M, "    if let Some(name) = policy_ref {", "    if let Some(name) = policy_ref.filter(|_| false) {", E2E),
    ("policy-ref: a missing target branch is never fetched", "crates/opshero-cli/src/git.rs", "        && crate::evidence::remote_exists(root, remote)\n", "        && false\n        && crate::evidence::remote_exists(root, remote)\n", E2E),
    ("policy-ref: the source of the policy is not stated", M, "    fn note(&self) -> Option<String> {\n        match self {", "    fn note(&self) -> Option<String> {\n        #[allow(unreachable_code)]\n        return None;\n        match self {", E2E),
    ("policy-ref: a bare name falls back to git's loose lookup (a tag can win)", "crates/opshero-cli/src/git.rs", '    resolve_exact(root, &format!("refs/heads/{refname}"))', "    resolve_exact(root, refname)", E2E),
    ("policy-ref: the remote-tracking ref is read by loose name (a tag or branch can win)", "crates/opshero-cli/src/git.rs", "        return resolve_exact(root, &tracking);", "        return resolve_exact(root, refname);", E2E),
    ("install: the checksum is not compared", "scripts/install.sh", '[ "$expected" = "$actual" ] || die', 'true || die', INSTALL_T),
    ("install: a missing SHA256SUMS entry is accepted", "scripts/install.sh", '[ -n "$expected" ] || die "SHA256SUMS has no entry', 'true || die "SHA256SUMS has no entry', INSTALL_T),
    ("install: an unsupported platform is not refused", "scripts/install.sh", '  *) die "no prebuilt binary for $os/$arch.', '  *) target=x86_64-unknown-linux-musl; true || die "no prebuilt binary for $os/$arch.', INSTALL_T),
    ("install: no fallback to the releases list (pre-releases not found)", "scripts/install.sh", '    fetch "$api/releases" "$tmp_v" 2>/dev/null ||', '    false ||', INSTALL_T),
    ("install: the OLDEST tag is chosen from the list", "scripts/install.sh", "| head -n 1 | sed 's/.*\"\\([^\"]*\\)\"$/\\1/; s/^v//')\"", "| tail -n 1 | sed 's/.*\"\\([^\"]*\\)\"$/\\1/; s/^v//')\"", INSTALL_T),
    ("cli docs: a help text changes without regenerating docs/cli.md", M, "    /// Delete old evidence, keeping the newest records and everything bound to the current content", "    /// Remove old evidence, keeping the newest records and everything bound to the current content", CLIDOC),
    ("cli docs: the generator forgets nested subcommands", "scripts/gen-cli-docs.py", "    for name in subcommands(text):\n        walk(", "    for name in []:\n        walk(", CLIDOC),
    ("package: files owned by a real user id", "scripts/package_release.py", "info.uid = info.gid = 0", "info.uid = info.gid = 1000", INSTALL_T),
    ("package: the binary is not executable", "scripts/package_release.py", "(f\"{top}/{exe}\", binary, 0o755)", "(f\"{top}/{exe}\", binary, 0o644)", INSTALL_T),
    ("git: no cleanup of newly ignored entries", G, "    if !ignored.is_empty() {\n        if ignored.iter().any", "    if false && !ignored.is_empty() {\n        if ignored.iter().any", GIT),
    ("git: one shared index for all exclusion sets", G, 'format!("index-{}", key.get(..32).unwrap_or("default"))', '"index-shared".to_owned()', GIT),
    ("git: check-ignore without --no-index", G, '"check-ignore", "--no-index", "-z"', '"check-ignore", "-z"', GIT),
    ("git: ignored exclusions not dropped", G, "effective_exclude(root, exclude)\n            .iter()", "exclude\n            .iter()", GIT),
    ("git: ancestor existence not considered", G, "let first = e.split('/').next().unwrap_or(e);", "let first = e;", GIT),
    ("doctor: an invalid policy is only a warning", DOC, "Err(e) => out.push(check(Level::Fail, e.to_string())),", "Err(e) => out.push(check(Level::Warn, e.to_string())),", DOCTOR_T),
    ("doctor: a missing git is not a failure", DOC, '            out.push(check(\n                Level::Fail,\n                "git was not found', '            out.push(check(\n                Level::Warn,\n                "git was not found', DOCTOR_T),
    ("doctor: failed() ignores failures", DOC, "checks.iter().any(|c| c.level == Level::Fail)", "checks.iter().any(|c| c.level == Level::Warn)", DOCTOR_T),
    ("doctor: git minor version ignored", DOC, "let minor = parts.next()?.parse().ok()?;", "let minor = 0;", DOCTOR_T),
    ("doctor: installed hooks never detected", DOC, "t.contains(install::MARKER)", "t.contains(\"never-present\")", DOCTOR_T),
    ("doctor: record count off by one", DOC, 'format!("evidence store: {} record(s)", records.len())', 'format!("evidence store: {} record(s)", records.len() + 1)', DOCTOR_T),
    ("installer: the manual page is not installed", "scripts/install.sh", '  cp "$work/$name/share/man/man1/opshero.1" "$PREFIX/share/man/man1/opshero.1"\n', "", INSTALL_T),
    ("package: the Windows binary is not named .exe", "scripts/package_release.py", 'exe = "opshero.exe" if windows else "opshero"', 'exe = "opshero"', INSTALL_T),
    ("package: zip timestamps are not neutral", "scripts/package_release.py", "date_time=(1980, 1, 1, 0, 0, 0)", "date_time=(2024, 1, 1, 0, 0, 0)", INSTALL_T),
]

# Mutants with no observable effect, kept so nobody "fixes" them by accident.
EQUIVALENT = {
    "shell: newline as a predecessor of a comment": "an unquoted newline is refused before any later `#` is examined, so the extra case is unreachable; kept as defense in depth",
    "policy-ref: validation of the ref name (leading dash, git check-ref-format)": "every name that validation refuses is also looked up as `refs/heads/<name>` and not found, so the user-visible result is the same usage error; kept because a name must never reach git unchecked",
    "store: non-atomic write of the chain head": "readers take the chain lock, so nobody observes a partial head; kept atomic as defense in depth",
    "stats: round down instead of up": "no input among 60,000 random ones changes the answer; rounding up is kept for the formal guarantee (RFC-0005 section 3.2)",
}

def run(cmd, env):
    return subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True)

def main():
    wanted = sys.argv[1:]
    env = dict(os.environ, OPSHERO_REQUIRE_NODE="1")
    bad = []
    selected = [m for m in MUTANTS if not wanted or any(w in m[0] for w in wanted)]
    for name, rel, old, new, cmd in selected:
        path = os.path.join(ROOT, rel)
        original = open(path, encoding="utf-8").read()
        if original.count(old) < 1:
            print(f"STALE   {name}: pattern not found in {rel}")
            bad.append(name)
            continue
        try:
            open(path, "w", encoding="utf-8").write(original.replace(old, new, 1))
            r = run(cmd, env)
        finally:
            open(path, "w", encoding="utf-8").write(original)
        out = r.stdout + r.stderr
        if "could not compile" in out or "error[E" in out:
            print(f"INVALID {name}: the mutant does not compile")
            bad.append(name)
        elif r.returncode != 0 and "FAILED" in out:
            failed = [l.split()[1] for l in out.splitlines() if l.startswith("test ") and l.endswith("FAILED")]
            print(f"killed  {name}  <- {', '.join(failed[:3]) or 'a test'}")
        else:
            print(f"SURVIVED {name}")
            bad.append(name)
    print(f"\n{len(selected) - len(bad)}/{len(selected)} mutants killed; {len(EQUIVALENT)} documented equivalent")
    for name, why in EQUIVALENT.items():
        print(f"  equivalent: {name}: {why}")
    sys.exit(1 if bad else 0)

if __name__ == "__main__":
    main()
