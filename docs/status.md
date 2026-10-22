# Status of the implementation

Last updated: 2026-10-21 (after the Claude Code adapter and a real-agent test). This file states what exists, what does not, and what was measured. It must never claim more than the code does.

## What works (v0 walking skeleton)
- `opshero init` writes a sample `.opshero/policy.toml`.
- `opshero run -- <cmd>` runs a command unchanged (stdio and exit code pass through) and records an evidence record under `.git/opshero/records/` (content-addressed, hash-chained, assurance A1). If recording is impossible (not a git repository, unwritable store) it warns and still runs the command (RFC-0001 P8).
- `opshero verify [--format text|json]` evaluates the policy for the current working tree. Exit codes: 0 PASS, 10 FAIL, 11 UNKNOWN, 2 usage, 1 internal. JSON output is canonical (RFC 8785 subset) and byte-identical across runs.
- **Claude Code adapter** (`opshero hook claude-code`, installed with `opshero hook install [--enforce-stop] --yes`): records simple Bash commands that the policy asks for, from `PostToolUse` and `PostToolUseFailure` events; optionally blocks the agent from stopping while evidence is missing or stale (`Stop`, never twice in a row). Installation is a dry run unless `--yes`, edits only its own entries, keeps a one-time backup, and refuses invalid JSON.
- **Subject exclusions**: tool-owned directories (`.claude`, `.codex`, `.cursor`, `.opshero`) and policy `[subject] exclude` pathspecs never count as a change to the code. Evidence recorded with different exclusions is not comparable.
- One check: `evidence.fresh` — a matching command must have run and passed on the exact current tree (tracked plus untracked non-ignored files). Edited or forged records are rejected by their content hash and reported.
- Quality gates met today: 126 tests (core unit, property, differential and robustness tests; adapter and git-adapter tests; 37 end-to-end tests against real git repositories and simulated hook events), `cargo clippy -D warnings` clean, `cargo fmt` clean, `unsafe` forbidden, unwrap/expect/panic denied in the core.

## What does not exist yet
Read-set observation (ADR-0008), scoped freshness, attestation and signing (RFC-0002 §5), other agent adapters (Codex, Cursor), CI action, evidence transport between machines, the dependency rule, the conformance suite, fuzzing, benchmarks in CI, reproducible-build and release tooling, Windows and macOS test coverage (developed and tested on Linux only).

## Measured (2026-10-21, Linux, release build, synthetic repositories, mean of 10-20 runs)
| Operation | Before (fresh index) | After (persistent private index) | Budget (RFC-0003 §6, revised) |
|---|---|---|---|
| `opshero run -- true`, 3,000 files | 70 ms | **26 ms** | 50 ms: met |
| `opshero verify`, 3,000 files | 37 ms | **17 ms** | 1 s: met |
| `opshero run -- true`, 30,000 files | not measured | **109 ms** | 200 ms: met |
| `opshero verify`, 30,000 files | not measured | **79 ms** | 1 s: met |
| First `run` after one file was edited, 30,000 files | not measured | 168 ms (that file is re-hashed, plus the cold cost of the first run) | informational |
| Release binary size | 1.1 MB | — | — |

The original 10 ms budget (hook overhead p99) is **withdrawn**: each measurement needs several `git` processes (about 1 ms each to start) plus the index update, so 10 ms is not reachable without a resident daemon, which ADR-0007 rules out. The revised budget is grounded in these measurements; overhead is paid once per recorded command, which usually takes seconds or minutes.

How: git keeps a stat cache in the private index, so only files whose stat data changed are re-hashed. Correctness is not assumed: a property test compares the persistent tree with the from-scratch tree after random sequences of file writes, deletions, ignore-rule changes and commits, for four exclusion sets (400 cases clean); same-size rewrites in quick succession (the "racily clean" case) are checked without delay; a corrupt or locked private index falls back to the from-scratch computation. Six injected defects in the adapter are each detected (one more was an equivalent mutant with no effect).

**Defect found by this work (critical, pre-existing):** when the repository's `.gitignore` ignored a path that the tool also excludes (for example `.claude/`, `.cursor/` or `.opshero/`, all common), `git add` refused the exclusion pathspec and **the tool could not measure the working tree at all**: `run` recorded nothing and `verify` exited with an internal error. It escaped the earlier end-to-end tests, which never combined the two. It was found by the new differential test, reproduced with the real binary, fixed (an exclusion that names an ignored path is redundant and is dropped before calling git; `check-ignore --no-index` is required so that paths tracked in the user's real index are judged by the ignore rules alone), and is covered by unit, property and end-to-end regression tests.

## Verified with a real agent (2026-10-21, Claude Code 2.1.302, haiku model, scripts/smoke-claude-code.sh)
Three short headless sessions in a disposable repository, one run each:
- **A** tests pass: the real `PostToolUse` payload carried `tool_input.command`, `tool_response` (no exit code field), `duration_ms`, `cwd`; the record was written; `verify` returned PASS.
- **B** tests fail: the real `PostToolUseFailure` payload carried `error: "Exit code 1\n1 test FAILED"` and `is_interrupt: false`; `verify` returned FAIL (exit code 1).
- **C** the agent edited code after the tests, with the Stop gate on: the Stop hook blocked once, the agent re-ran the tests, then finished (`stop_hook_active=true` on the final Stop) and `verify` returned PASS.
This is an **integration proof, not a statistic**: n = 1 per scenario, one model, a toy script, a cooperative prompt.

## Defect found by that real test, and fixed
The first real run reported "stale evidence" although the agent changed nothing after the tests: files written by tools in the repository (a payload log, tool output, `.claude/`) changed the tree. Fix: subject exclusions (above). Lesson: staleness must be scoped to the files that matter.

## Known limitations
- The evidence chain can fork if two `run` commands record at the same moment; `verify` tolerates forks (ordering falls back to chain depth, then id).
- A very large repository makes every `run` and `verify` slow until the persistent-index change lands.
- The hook runs after the command, so the "before" tree is unknown: `tree_before` equals `tree_after` for hook-recorded evidence.
- Only **simple** commands are recorded (optionally with leading `VAR=value` assignments and one `cd DIR &&`). Pipelines, lists (`;`, `&&`, `||`), substitutions and background jobs are never evidence because their exit status can mask a failure (`pytest | tail` exits with the status of `tail`). An agent that always pipes its test output will therefore get `UNKNOWN`.
- Failures that are not `Exit code N` (denied by the user, could not start) and interrupted commands are not recorded.
- Hook event field names beyond the ones exercised above (and the exact semantics of `PostToolUse` versus `PostToolUseFailure` for every tool) come from documentation that was only partly retrievable; re-check on upgrade.
- Only A1 assurance exists, which is advisory (RFC-0001 §7).

## Design decisions made by implementing
- Freshness uses the tree **after** the run (`tree_after`), documented in RFC-0002 §4.
- A test caught that failing to measure the tree before the run would have aborted the command; `run` now fails open on recording problems.

## Update 2026-10-21: M1 (mathematical rigour), see docs/roadmap.md

Verified with `cargo fmt --check`, `cargo clippy -D warnings`, `OPSHERO_REQUIRE_NODE=1 cargo test` (116 tests, 0 failed) and TLC on three configurations.

| Area | What exists | Evidence |
|---|---|---|
| Verdict laws | L1-L12 as property tests (2,000 cases each) plus an independent specification oracle | 12 injected defects in `verify.rs` and `canon.rs` each detected |
| Flaky commands | policy `repeat = { alpha, flake_rate }`, `opshero run --repeat N\|auto`, integer-only `stats::min_passes` | literature values, exact integer oracle on a 300,000-point grid, seeded Monte-Carlo, 6 end-to-end tests |
| Run that changed the tree | policy `post_run_changes = "allow"\|"deny"` (default deny since 2026-10-21; see the update at the end) | TLA+ model: `allow` unsound in 4 states; `deny` exhaustively sound for one change per run; ABA is a limit of local evidence |
| Parser robustness | deterministic mutation fuzzer over policy, record, canonical JSON, shell analysis, timestamps | 300,000 iterations per seed clean; 3 injected defects detected |
| Canonical JSON | differential test against the ECMAScript serializer via `node` | 4,000 random documents per run agree; found an intermittent `JSON.parse` misparse in Node 24.19 itself (RFC-0005 section 6) |

Defects found by the new tests in this project's own code: the percentage parser accepted `5.%`. Known issues: KI-1 closed for flaky commands by `repeat` (default unchanged); KI-2 closed: the default of `post_run_changes` is now `deny` (RFC-0005 section 7).

Not done: coverage-guided fuzzing (needs a nightly toolchain), formal model of local-to-CI transport and concurrent recorders. The CI workflow has since run green on a real runner (see the update at the end). Since this update: persistent private git index (see Measured) and `scripts/mutation-check.py` (26 injected defects, all caught) and `scripts/check-formal.sh`.

## Update 2026-10-21 (later): dogfooding tooling
`opshero journal enable|disable|status|list|label|report`: opt-in, local, content-free (see docs/validation-plan.md, H1 procedure). The decision rule and the Wilson interval are pure and tested (property tests, known values checked against an independent computation, 9 injected defects killed). The hook and the journal are installed in this repository; zero sessions recorded yet. Test count: 146, all passing.

## Update 2026-10-21 (decision): `post_run_changes` defaults to `deny`
A run during which the working tree changed no longer counts as evidence unless the requirement says `post_run_changes = "allow"`. Reason: the TLA+ model (docs/formal) shows the former default accepts a pass for content the command did not run against. Cost: a first-run FAIL for commands that create untracked, non-ignored files, with a message that names the fix. Changed before any dogfooding session so that the data is collected under the final default.

## Update 2026-10-21 (hardening): stress probes and what they found
The question was "is it solid and fast?". Instead of asserting it, three probes were run with the real binary; each found a real defect, now fixed and covered by tests that fail without the fix.

| Probe | Result before | Defect | Fix | Result after |
|---|---|---|---|---|
| 40 parallel `run` | 39 records for 40 runs; 11 recorded against the **empty tree**; forked chain; no warning | the shared private index was deleted by one process while another used it; reading the chain head and appending were not one step; identical runs collapsed into one record | exclusive file locks (standard library `flock`, with timeout and fallback) around the index sequence and around head-read plus append; atomic head write | 40 of 40, one tree, zero forks, zero warnings; 15 of 15 repeats green |
| `verify` with 6,000 records | 2,637 ms (30,000 records: 330 s) | chain depth recomputed by walking every record's ancestry: quadratic | linear memoised computation, checked against the old definition on forests, duplicates and dangling parents | 142 ms at 6,000 records |
| Store growth | unbounded; beyond 20,000 records the loader silently ignored an arbitrary subset (by hash order), which could turn a PASS into UNKNOWN | no retention | `opshero gc [--keep N] [--dry-run]` and automatic pruning above 5,000 records down to 4,000; the evidence bound to the current content is never deleted; only records that pass the integrity check are ever removed | store stays bounded; verify time bounded |

Also probed with the real binary, all correct: secondary `git worktree`, a repository with a submodule, paths with spaces and accents and file names containing a newline or a quote, symlink loop and broken symlink, a repository with no commits, running from a subdirectory. One cost to know: a very large untracked file that is not ignored (200 MB tested) makes the first measurement take about 4 s (git has to hash it); later runs take 13 ms because git's stat cache applies.

Lesson recorded: the earlier test suite proved the verdict function well and the storage layer poorly. Concurrency and scale are now tested at the level where they fail (threads released by a barrier for the race window, 30,000-record chains for the complexity), and the mutation script injects the missing lock to prove the tests notice.

## Update 2026-10-21 (pilot P1)
Two pilots with real Claude Code sessions (70 sessions, about 0.75 USD) measured the Stop gate. Result: the gate demanded evidence in turns that changed nothing (10 of 10) and in documentation-only turns (5 of 5); fixed with a per-turn baseline (`UserPromptSubmit` hook) and per-requirement `paths`; wrong blocks fell from 60 percent to 13 percent; the gate caught 9 of 9 sessions that changed code after testing. 7 of those 9 then re-ran the tests; 2 ignored the one-time block. Remaining wrong blocks are tests run through pipes, which are deliberately not evidence. Full method, ground-truth rules and limits: docs/validation-plan.md, Pilot P1. 178 tests pass; 48 mutants are killed (scripts/mutation-check.py).

## Update 2026-10-21 (CI gate prototype)
`opshero evidence push|fetch` and `opshero verify --ci` implement the transport from a developer or agent to CI through `refs/opshero/evidence/<tree>`. Fail closed: no evidence, evidence for an older tree, an unreachable remote and a record whose hash does not match all give UNKNOWN (exit 11). Tested against a real bare remote and real clones. It catches forgotten or stale checks; it does **not** resist a dishonest publisher (assurance A1). The example workflow has not run on a real runner. See docs/ci-gate.md for the pitfalls found (pull-request merge commits, published command lines, fork pull requests). Tests: 187 passing; mutants: 54 killed.

## Update 2026-10-21 (assurance A2 prototype)
`opshero ci run [--policy P]`: the CI executes the commands the policy requires (`run = [...]` gives a concrete command for a wildcard pattern), records each as assurance A2, and gives the verdict (0/10/11). Published evidence is stored apart and capped at A1 wherever it is read, so a forged A2 claim never counts. `ci run` refuses to run outside CI unless `--allow-local`. Not built: signing (A3), the packaged CI action, a real-runner run of the example workflow. A2 trusts the CI configuration and the test commands themselves (see docs/ci-gate.md). Tests: 198 passing; mutants: 59 killed (one documented equivalent pair).

## Update 2026-10-21 (automated security reviews of the first commit)
Two automated reviews flagged six points. Each was checked against the code; none was dismissed without a test.

| Finding | Verdict | What was done |
|---|---|---|
| A run after `cd` to a directory outside the repository was recorded as a run in the repository (`rel_cwd` answered "." for any path it could not place) | **Real, fixed** | the Claude Code adapter no longer records a command whose directory is outside the repository (`ops::inside_root`); a directory inside it is recorded as such |
| Environment variables that change which program or library runs (`PATH`, `LD_*`, `DYLD_*`, `PYTHONPATH`, `*_ADDOPTS`, `NODE_OPTIONS`, `RUSTC_WRAPPER`, ...) did not stop a command from being recorded | **Real, fixed** | such a command is not recorded as evidence; ordinary variables (`CI=1`, `LANG=C`) are unaffected. Limit, stated plainly: at assurance A1 a dishonest agent can still run `sh fake.sh`; this closes the accidental and the cheap cases, not a determined adversary (that is what A2 is for) |
| A published blob was read into memory before its size was checked | **Real, fixed** | sizes come from `git ls-tree -l`, and an oversized file is skipped without being read |
| `[subject] exclude = ["*"]` (or `.`, `/`, `**`) would make the subject the empty tree, so any evidence looks fresh | **Real, fixed** | such an entry is a policy error |
| The policy file lives in a directory that is excluded from the subject, so a change can edit the policy it is judged by | **Acknowledged, mitigated, not eliminated** | the documented rule is to pass `--policy` from the target branch to `ci run` and `verify --ci`; both now print a warning when the policy is read from the checked-out content. Binding each record to a hash of the policy is not built and is in the roadmap |
| The `cd` target may contain parent components | covered by the first row | a path is accepted only if it resolves inside the repository |

Tests: 204 passing. Mutants: 64 killed (two documented equivalents).

## Update 2026-10-21 (second automated security review: the shell analyzer)
Two more findings on `shellcmd.rs`, one rated high. Instead of patching the single case, a **differential test against bash itself** was written (`crates/opshero-core/tests/shell_differential.rs`): for every input the analyzer calls a simple command, bash runs it with each unknown command intercepted, and the test requires that bash executed at most one command and that its program is the one the analyzer reported. Findings, all real:

| Differential | Example | Fix |
|---|---|---|
| ANSI-C quoting: inside `$'...'` a backslash escapes the apostrophe, which a POSIX splitter does not know. The input is balanced for the splitter and is two commands for bash | `a $'\'' ; b \'` | `$'` and `$"` outside quotes are refused |
| A comment ends at the newline whatever a backslash before it says; the next line is a new command | `- #x\` newline `a` | an unquoted word starting with `#` is refused |
| A quoted or escaped `NAME=value` is a command name in bash, not an assignment; assignments were recognised after unquoting | `'v=1a=b' xf` | assignments are recognised on the raw text |
| A "program" that is not a program name | `x->b` (bash runs `x-`), `$x`, `*`, `>`, empty | the program name must match a safe character set |

The environment-variable denylist (second finding) was replaced by an **allowlist** (`CI`, `LANG`, `LC_*`, `TZ`, `RUST_BACKTRACE`, `NODE_ENV`, ...): the set of variables that can redirect a toolchain (`PATH`, `LD_*`, `RUSTFLAGS`, `CARGO_*`, `PYTEST_*`, `NPM_CONFIG_*`, `GIT_*`, `*_ADDOPTS`, ...) has no end. Cost: an application variable such as `MYAPP_ENV=test pytest` is no longer evidence, so an agent is nudged to run it plainly; a policy-level opt-in list is a possible later addition.

Result: 1.5 million random inputs (six seeds), about 67,000 of them classified as simple commands and compared with bash, found no remaining differential. Limits of the harness, stated: programs given with a slash are executed by bash without going through the observation hook and are not checked; zero commands executed (for example a failed redirection) is accepted as safe because no false success can come out of it. Tests: 207 passing. Mutants: 68 killed (two documented equivalents).

## Update 2026-10-21 (first CI attempt)
The first two pushes started the workflow on GitHub; all three jobs failed after 3 to 5 seconds with no step run and no runner assigned. The check annotation gives the cause: *the job was not started because recent account payments have failed or the spending limit needs to be increased* (private repository, Actions minutes). It is an account setting, not a code failure, so the workflow stays unverified on a real runner. `scripts/ci-local.sh` runs the same steps locally (the quick form, without the mutation check, passes in about 45 seconds).

## Update 2026-10-21 (third automated review: word separators)
Finding (high): bash separates words on space and tab only; the analyzer also treated form feed, carriage return, vertical tab and Unicode spaces as separators. `CI=1<FF>a` is one assignment to bash (nothing runs, exit status 0) and "run a" to the analyzer: a recorded run that never happened. Reproduced by the differential test before fixing. Fix: any control character (except tab and newline) and any Unicode space other than space and tab is refused, and the raw scans split on space and tab only.

The differential test also exposed an error in an earlier assumption of mine: "bash ran nothing" is only safe when the shell **reported a failure**. An assignment alone runs nothing and exits 0. The test now requires a non-zero exit status in that case, and its alphabet was meant to include the new characters and allowlisted assignments (without them the fuzzer could no longer reach assignments at all). **That sentence was not true when written: see the correction below.** Bash builtins and keywords (`fc`, `cd`, `if`, ...) are skipped by the harness, as it cannot observe them; the list comes from `compgen -b; compgen -k`, not from a guess.

1.5 million further random inputs (six seeds) were reported as clean; see the correction below for why that claim was too strong. Tests: 208 passing. Mutants: 70 killed (two documented equivalents).

## Correction and fourth review (2026-10-22): the fuzzer alphabet, line continuations, redirections in assignments
**A claim of mine was wrong.** The previous update said the differential test's random alphabet included form feed, carriage return, no-break space and allowlisted assignments (`CI=1`, ...). It did not: the edit was applied with a string replacement that silently did nothing, because `rustfmt` had reshaped the array. The campaigns reported as "1.5 million inputs, clean" therefore never exercised those characters or any assignment; only the named trap list did. The test now asserts that its alphabet contains the atoms it claims to, so this cannot pass silently again.

With the alphabet really in place, the fuzzer found two more differentials, and the fourth automated review (high) found a third by reading the code:

| Differential | Example | Fix |
|---|---|---|
| A backslash-newline is deleted by bash before it tokenizes, so a `#` after it starts a comment and the next line is a second command. The apostrophe in the comment and an escaped one later cancel out, so a POSIX splitter sees balanced quotes. I introduced this when I narrowed the characters that may precede a `#` | `pytest \<newline>#'<newline>rm \'` is one command to the analyzer and two (`pytest`, `rm '`) to bash | an unquoted backslash-newline is refused (also inside double quotes), and a newline may precede a comment |
| A redirection operator ends a word in bash, so `TZ=UTC>> c` is an assignment plus a redirection to a file named `c`: nothing runs, exit status 0. The analyzer took `UTC>>` for the value and `c` for the command | `TZ=UTC>> c`, `LANG=C/>> b` | an assignment value stops at an unquoted `<` or `>` |

The random campaign had not found the first one in 1.5 million inputs, because it needs a specific combination of atoms; composite atoms (`\<newline>#`, ` #`, `#'`, `\'`) were added so that such combinations are reachable. After the fixes: ten seeds of 250,000 inputs (2.5 million) with the real alphabet found no differential.

**Test infrastructure bug, also mine:** the harness created three temporary files per call and did not always remove them, which exhausted the inodes of `/tmp` (334,000 leftover directories named `opshero-shelldiff-*`, all created by this test and deleted). It now uses one sandbox per test thread, emptied before each call and removed when the thread ends; a run leaves nothing behind.

Limits that remain: programs given with a slash and bash builtins are not observable by the harness and are skipped; the comment-after-newline predecessor is partly redundant with the newline refusal and is kept as defense in depth (listed as a documented equivalent mutant). Tests: 221 passing. Mutants: 85 killed, 3 documented equivalents.

## Update 2026-10-22 (first green CI run)
After the repository was made public (Actions minutes are free for public repositories; the earlier failure was an account billing limit on the private repository, not the code), the workflow ran for the first time on a real GitHub runner (`ubuntu-24.04`) and **all three jobs passed**: `test` (fmt, clippy, 210 tests with `node` and `bash` required) in 61 s, `formal` (three TLC models, jar checksum verified) in 8 s, `mutation` (73 injected defects, all killed) in 175 s. This also confirms on a second machine that the differential tests against `node` and `bash`, the TLC checksum pin and the mutation script do not depend on this laptop's setup. Still never run on a runner: the example gate workflows in docs/ci-gate.md (`opshero ci run`, `opshero verify --ci`), which need a built binary and a pull-request context.

## Update 2026-10-22 (platforms, supply chain, repository hygiene)
- **Platforms.** CI now runs the test suite on `ubuntu-24.04`, `macos-latest` and `windows-latest`. All three pass: 29 CLI unit tests, 79 end-to-end tests against real git repositories, and the core, property, differential and robustness tests. The bash differential test skips itself where `bash` or `timeout` is missing (macOS, Windows); `node`-based canonical JSON tests run everywhere. What this does **not** show: Windows is exercised through Git for Windows' `sh`; nothing has been tried with a real agent on macOS or Windows, and hook command paths with backslashes are untested. Linux and macOS are required to pass; Windows is reported but never fails the build.
- **Supply chain.** `cargo deny` (CI job `deny`) checks licenses against an allowlist (all 84 third-party dependencies are MIT, Apache-2.0, Zlib or Unlicense), known vulnerabilities (none) and sources (crates.io only). Dependabot is configured for Cargo and GitHub Actions; its pull requests run the full CI and are never merged automatically.
- **License.** Apache-2.0 (`LICENSE` unmodified from apache.org, `NOTICE`, manifests). GitHub detects it.
- **Repository.** Description and 14 topics set; private vulnerability reporting enabled (it was off, while SECURITY.md told people to use it).
- **Bug found by the new jobs, mine:** the differential tests treated an *empty* `OPSHERO_REQUIRE_*` variable as "required"; fixed.

## Update 2026-10-22 (`--policy-ref`)
`opshero verify` and `opshero ci run` accept `--policy-ref <ref>`: the policy is read from a git commit (for example the target branch) instead of the checked-out content, which the author of a change controls. A `<remote>/<branch>` ref that a shallow pull-request checkout does not hold is fetched with `--depth=1`. The report says `policy taken from <ref> (commit ...)`, the untrusted-policy warning is not printed, a missing ref or a missing policy file at the ref is a usage error (exit 2), and `--policy` and `--policy-ref` cannot be combined. Tests: a change that weakens its own policy passes under its own policy and is UNKNOWN under the target branch's; `ci run` executes the target branch's commands, not the change's; the shallow-checkout fetch; missing ref and missing file; the conflict between the two flags. This closes the policy-trust gap acknowledged in the security review (docs/status.md, 2026-10-21); binding each record to the hash of its policy is still open.

## Update 2026-10-22 (fifth review: ambiguous ref names)
Finding (medium): `--policy-ref` resolved the name with `git rev-parse <name>^{commit}`, and git's own lookup order prefers a **tag** over a branch for `main`, and a tag or a local branch over the remote-tracking ref for `origin/main`. An object of the same name in another namespace could therefore decide which policy judges a change. Confirmed by three failing tests before the fix: a tag called `main` on the weakened commit won over the branch; a tag and a branch called `origin/main` won over the fetched remote-tracking ref.

Fix: resolution is exact. A full commit id (40 or 64 hex digits), a fully qualified `refs/...` name, `<remote>/<branch>` for a configured remote (always fetched, then read from exactly `refs/remotes/<remote>/<branch>`), or a bare branch name (exactly `refs/heads/<name>`). Names are checked with `git check-ref-format` and refused if they start with `-` or contain whitespace or control characters, before any git process sees them. An abbreviated commit id is refused: it could collide with a ref name. A side effect worth knowing: `<remote>/<branch>` always needs the network (a fresh fetch), which is the point for a gate and a limitation for an offline check; a full commit id works offline. Tests: 88 end-to-end; mutants: two new ones (a loose fallback for a bare name and for the tracking ref), both killed; the name validation itself is a documented equivalent mutant because every refused name is also simply not found.

## Update 2026-10-22 (release pipeline)
Built, exercised, and published as the pre-release v0.0.1 (see the update below). `scripts/package_release.py` makes reproducible archives; `scripts/install.sh` verifies a download against `SHA256SUMS` and installs without root, and refuses (leaving any existing install untouched) on a tampered archive, a missing or wrong checksum, an unsupported platform or a failed download; `scripts/test-install.sh` tests all of that against a fake local release and runs in CI on Linux and macOS (5 mutants, all killed). The `release` workflow builds static musl binaries for Linux (x86_64, aarch64) and macOS binaries (aarch64, x86_64), fails if a Linux binary is not static, and on a version tag publishes a pre-release with `SHA256SUMS` and a signed build-provenance attestation. A manual run on 2026-10-22 built all four archives (633 to 752 KB); the Linux x86_64 one, downloaded from GitHub, installed with the real installer, is a static-pie ELF of 1.6 MB, and ran a PASS scenario. Not exercised: the publish job (it only runs for a tag), the macOS and aarch64 binaries (they cannot run on this machine), Windows (no binary). Two things found on the way and fixed: one pinned action commit I had written from memory did not exist, and another pointed at an annotated tag object instead of a commit; all five pins are now checked against GitHub. Details: docs/releasing.md.

## Update 2026-10-22 (v0.0.1 published)
The pre-release `v0.0.1` is published with four archives, `SHA256SUMS` and a signed build-provenance attestation. The publish job, never exercised before, worked. Verified as a user would: checksums match; the attestation verifies and names this repository, the workflow, the tag and the commit; a tampered archive is rejected; the installer installs `0.0.1` from the real release. Found by doing so and fixed: the installer failed without an explicit version, because GitHub's `releases/latest` ignores pre-releases (and its first fix chose the oldest tag on single-line JSON). Still not exercised: running the macOS and aarch64 binaries (no such machine here; CI builds them but only executes the native ones), and Windows (no binary). Details: docs/releasing.md.

## Update 2026-10-22 (the four Dependabot pull requests)
Read, tested and applied by hand rather than merged, because two of them touch what the tool is made of.
- **`sha2` 0.10 to 0.11.1 (PR #4):** it computes the id of every evidence record. The existing test only checked internal consistency, so a change in the hashing or the canonical form would have invalidated every stored record without any test noticing. A golden test now pins the canonical form and the SHA-256 id of a record and of three small documents, with values computed independently in Python (the `{}` vector is the well-known one). It passes before and after; the update needed no code change.
- **`toml` 0.8 to 1.1.8 (PR #5):** it defines the language of policy files. Measured before and after with the same probe: TOML 1.1 is now accepted (newlines and trailing commas in inline tables, `\e` escapes); duplicate keys, unknown keys, a byte order mark and CRLF behave as before. The dialect is now pinned by a test and documented (RFC-0004, "File format"). A 300,000-iteration robustness fuzz per seed ran clean on the new parser, and all 85 injected defects are still detected.
- **`actions/checkout` 4.2.2 to 7.0.1 (PR #3):** the release notes of 5, 6 and 7 were read (Node 24, credentials stored in a separate file, a block on dangerous `pull_request_target` checkouts); none affects how the workflows use it. Applied to both workflows, pinned to the commit.
- **`cargo-deny-action` (PR #2):** already pinned to that very commit on main (I had first pinned an annotated tag object instead of the commit).
Tests: 221 passing. Mutants: 85 killed.

## Update 2026-10-22 (generated CLI reference)
`docs/cli.md` is produced by running the binary (`scripts/gen-cli-docs.py`) and walking every subcommand's `--help`; CI (Linux) runs it with `--check` and fails if the committed page differs. It already paid for itself once: it exposed that the help of `ci run` still told people to use `--policy`, after `--policy-ref` had replaced it as the recommendation. Two injected defects (a help text changed without regenerating the page, a generator that forgets nested subcommands) are detected. Mutants: 87 killed.

## Update 2026-10-22 (coverage-guided fuzzing in CI)
A CI job installs the nightly toolchain and `cargo-fuzz` and runs four libFuzzer targets for 60 seconds each: `policy`, `record`, `canon`, `shellcmd` (`fuzz/`, seed corpora committed). The targets check **properties**, not just the absence of crashes: an accepted policy satisfies the invariants the evaluator relies on and no exclusion matches everything; an accepted record's id is the hash of its content; canonical JSON round-trips and is idempotent; a simple shell command has a sane program name and no exotic whitespace. First run on a real runner: 644,301, 3,969,366, 4,449,284 and 2,623,016 executions, no crash, no property violation. One of my own properties was wrong before the first run (a record may carry unknown fields covered by its id) and was corrected by reading it, not by the fuzzer.

Limits, stated plainly. This is a 60-second smoke run on every push, not a long campaign: a scheduled multi-hour job would be the next step. **Nothing has yet shown that these fuzzers can find a defect**: the injected-defect check (scripts/mutation-check.py) covers the unit, property, differential and end-to-end tests, not the fuzz targets, and the nightly toolchain is not installed on the development machine. The shell analyzer is covered more strongly by the differential test against bash than by this target.

## Update 2026-10-22 (installability)
What was added so that the tool can be installed and used without reading the source:

| Item | State | Evidence |
|---|---|---|
| `opshero doctor` (git version, repository, policy, working-tree measurement, evidence store, hooks) | done | 5 unit tests, 5 end-to-end tests including "git missing" and "invalid policy"; 6 mutants killed |
| Shell completions (`opshero completions <shell>`) and manual page (`opshero man`), generated from the program's definitions | done | end-to-end tests for all five shells and for the roff header; generated files are identical across runs |
| Release archives carry the manual page and completions; the installer installs the manual page | done | `scripts/test-install.sh` checks exact archive contents; 3 more mutants killed |
| Windows: `x86_64-pc-windows-msvc` build, `.zip` archive, `scripts/install.ps1` | **written; the installer test runs on a Windows runner in CI** (experimental) | zip reproducibility and contents are tested on Linux; the PowerShell installer is exercised only by the Windows CI job |
| GitHub Action (`action.yml`): installs a release, enforces the target branch's policy, job summary, `verdict` output | done | its shell logic was run locally with simulated GitHub variables (PASS, FAIL with summary, invalid mode); CI job `action` runs it on Linux and macOS with the build under test and with the published release |
| crates.io readiness | partial | names free on 2026-10-22; `opshero-core` packaged and built from its package; `opshero-cli` verifiable only after the first publication; `publish = false` until the maintainer decides |
| Oldest git version | being measured | CI job on Ubuntu 22.04 (git 2.34), informational; `doctor` warns below 2.34 |

Tests: 233 passing. Still not done: Homebrew (a tap needs its own repository), `.deb`/`.rpm`, a Windows run with a real agent, detection of unexercised changes.
