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

Not done: coverage-guided fuzzing (needs a nightly toolchain), formal model of local-to-CI transport and concurrent recorders. The CI workflow exists but has not run on a real runner. Since this update: persistent private git index (see Measured) and `scripts/mutation-check.py` (26 injected defects, all caught) and `scripts/check-formal.sh`.

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
