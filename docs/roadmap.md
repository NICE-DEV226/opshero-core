# Roadmap

Status as of 2026-10-21. Each milestone has an **exit criterion** that can be checked; a milestone is not done until its criterion holds. Order is by risk reduction, not by effort. Items marked (owner) need decisions or actions the maintainers must take; they are not code.

Quality gates that apply to every milestone: `cargo clippy -D warnings`, `cargo fmt --check`, all tests green, no `unsafe`, no `unwrap`/`expect`/`panic` in the core, every new behaviour covered by a test that fails without it (checked by mutation for the evaluation core), docs updated in the same change.

## M0: Foundations (done)

Evidence records, canonical JSON, policy v0, verdict function, git tree binding, Claude Code adapter with installer, 66+ tests, RFC-0001..0004, ADR-0001..0009, measured limitations in docs/status.md.

## M1: Mathematical rigour of the core (in progress)

| Item | State |
|---|---|
| Verdict laws L1-L8 as property tests, mutation-checked | done |
| `stats::min_passes` integer-only, oracle-tested, simulated | done |
| KI-1 resolution: `repeat` policy field and mixed-result semantics (RFC-0005 §3.1), 6/6 injected defects killed | done |
| `opshero run --repeat N\|auto` (RFC-0005 OQ3) | done |
| Formal model of freshness (TLA+, TLC): found KI-2, closed by `post_run_changes = "deny"` (docs/formal) | done for local evidence; local-to-CI transport and concurrent recorders planned |
| Deterministic mutation fuzzing of every untrusted-input parser, 300k iterations clean, 3/3 injected defects detected | done |
| Coverage-guided fuzzing (`cargo-fuzz`, nightly toolchain, corpus in repo, run in CI) | planned, needs a toolchain decision |
| Differential test of the canonical JSON against the ECMAScript serializer (found a misparse in the reference itself) | done |
| Default of `post_run_changes` (KI-2): flipped to `deny` on 2026-10-21, before any dogfooding data; friction measured by the journal | done |
| CI workflow (`.github/workflows/ci.yml`): fmt, clippy, tests with `OPSHERO_REQUIRE_NODE=1`, TLC models via `scripts/check-formal.sh`, 68-mutant check via `scripts/mutation-check.py`; actions pinned by commit | written, every command run locally, **first push on 2026-10-21: GitHub did not start any job (account billing / spending limit on a private repository), so it is still unverified on a runner; `scripts/ci-local.sh` runs the same steps locally** |

Exit criterion: KI-1 closed (done), mutation fuzzing clean (done), one formal model checked (done; its two intended counter-examples document the limits of local evidence), CI workflow (written, unverified on a runner), coverage-guided fuzzers run 10 minutes in CI without findings (open).

## M2: Performance and robustness

| Item | State |
|---|---|
| Persistent private git index: `run` 70 to 26 ms (3k files), 109 ms (30k files); equality with the from-scratch tree checked by property test | done |
| Critical defect fixed: repositories that gitignore an excluded path (`.claude/`) broke measurement entirely | done |
| Benchmarks in CI with the revised budgets (RFC-0003 §6): record <= 50 ms at 3k files, `verify` < 1 s | planned |
| Large repositories (100k files), submodules, sparse checkout, SHA-256 repositories, Windows paths | planned |
| Concurrency: parallel recorders (locks around index and chain; 40-way probe clean; two lock mutants killed) | done |
| Quadratic `verify` fixed (6,000 records: 2,637 to 142 ms) and bounded store (`opshero gc`, auto-prune) | done |
| Probed shapes: worktree, submodule, unicode and newline names, symlinks, no commits, subdirectory | done (no defect found) |

Exit criterion: budgets enforced by a failing CI job; every listed repository shape has an end-to-end test.

## M3: Dogfooding and validation (the real risk)

Demand is unproven. This milestone decides whether M4+ is worth building.

- Install `opshero hook install --enforce-stop --yes` on the team's real repositories with real agents. **Done for this repository** (takes effect at the next session); other repositories are the team's to do, procedure in docs/validation-plan.md.
- **Pilot P1 done (70 scripted real sessions, 0.75 USD): found and fixed the gate demanding evidence in turns that changed nothing (wrong blocks 60 percent to 13 percent); recall on code changed after tests 9 of 9; details and limits in docs/validation-plan.md.** This validates the mechanism, not the demand.
- Opt-in local journal with labels and a report against the pre-registered rule: `opshero journal` (done: pure core with Wilson interval and decision rule, 9 mutants killed, privacy test on the file content).
- H0 interviews led by the owners; threshold documented in docs/validation-plan.md.

Exit criterion (pre-registered): at least 10 % of at least 100 agent sessions produce a real finding (a missing or stale check that a human confirms), and at least 3 interviewed engineers would put the CI gate on a real repository.
Kill criterion: below 3 % after 100 sessions, stop feature work and reassess the direction.

## M4: Integrations (only if M3 passes)

1. `opshero verify --ci` with fail-closed semantics: **done as a prototype** (docs/ci-gate.md). Packaged CI action, required status check and job summary: not done.
2. Evidence transport local to CI: **done as a prototype** with one git ref per tree (`opshero evidence push|fetch`); 66 end-to-end tests including a forged record and a fresh clone; 6 mutants killed. Not done: locking of concurrent pushes, `evidence prune`, running the example workflow on a real runner.
3. Codex and Cursor adapters, each verified with a real session before being documented as supported.
4. SARIF output.
5. Assurance A2: **done as a prototype** (`opshero ci run`: the CI executes the required commands and records them as A2; published evidence is capped at A1 everywhere; trusted policy from the base branch; 10 end-to-end tests, 5 mutants killed). Signing (DSSE envelope, in-toto predicate; key and OIDC) for A3: not done.

Exit criterion: one public repository where the CI gate blocks a real stale-evidence pull request.

## M5: Depth

- Read-set tracing on Linux (ADR-0008) and scoped freshness: a check stays valid if no file it read changed (Merkle subtree comparison).
- Flaky-aware bisection.
- Dependency rule (RFC-0004 §6), optional and network-gated.
- Conformance suite and a second independent verifier implementation.

## M6: Project and business (owner)

- License decision (ADR-0006), contributor policy, governance.
- Legal entity and bank country (Burkina Faso is not supported by Stripe or GitHub Sponsors).
- Security contact for SECURITY.md, vulnerability process.
- Name: OpsHero retained (domain owned); verify crates.io, npm, Homebrew; the PyPI name `opshero` is already taken by an earlier project of the same owner.
- Bind each evidence record to a hash of the policy it was recorded under, so that a policy edited in the same change cannot be confused with the trusted one (open; today the mitigation is `--policy` from the target branch plus a warning).
- Wind down the previous hosted service (kept in private notes) and archive its repositories.
- First commit and public repository.

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Nobody needs this (demand unproven) | medium | fatal | M3 with a pre-registered kill criterion before M4 |
| Platforms absorb the feature | medium | high | forge-agnostic, standard attestation formats, open core |
| False sense of safety from A1 (local, advisory) evidence | medium | high | assurance shown in every verdict; only A2/A3 resist a dishonest operator |
| Agent vendors change hook formats | high | medium | versioned adapters, real-session smoke test, fail open for recording |
| Statistical claims misread as guarantees about future runs | medium | medium | RFC-0005 states the exact meaning; explanations print the assumption |
| Single maintainer capacity | high | medium | small core, conformance suite, documented specs |
