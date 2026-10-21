# Validation plan and pre-committed kill criteria

Principle: write the success and stop criteria **before** collecting data, record denominators, report intervals, and count null results as results. T0 is the first day the team uses a working prototype on its own agent-assisted work.

## H0 — Revealed demand (measured 2026-10-19 on public GitHub)
- **Pain reports exist but are few.** Searching four agent repositories for phrases about false success claims found relevant reports such as anthropics/claude-code #32281 (reports completion without executing operations) and #8738 (premature victory declarations), about 10 comments each; openai/codex #41626 (falsely marks a gated test as verified), 4 comments; google-gemini/gemini-cli #27177 (auditing final agent output claims), 2 comments. The search had poor precision, so these are small lower bounds.
- **People are already building it, at hobby scale.** At least 14 repositories pushed since 2026-06 implement stop hooks that block an unverified "done", agent attestations, or "done" verification (largest: 40 stars; examples: valentynkit/jev-belay, nolabs-ai/agent-sign, Black-Studio-ia/done-guard). None has traction.
- **What this does and does not show.** It shows the need is felt by practitioners and that **no standard-bearer exists**. It cannot distinguish "open niche" from "few people care". It also shows the core idea is easy to reproduce, which reinforces that the defensible parts are the open format, the CI gate and the conformance suite.
- **Week-1 sprint (decide before building more):** interview at least 8 of these builders or the authors of the issues above, with the fixed question in "Instrumentation and ethics". Continue only if at least 3 say they would put a CI gate on a real repository within 30 days **and** at least 1 can name who in their organisation would pay. Otherwise stop.

## Hypotheses

| ID | Hypothesis | Measure | Stop if (by) |
|---|---|---|---|
| H1 | Agent-assisted changes often carry claims that the evidence does not support (stale evidence or an unexercised change) | In at least 100 agent-assisted pull requests/sessions, the share where a **human maintainer, reviewing the finding, agrees it is real** | below 10% (T0 + 30 days) |
| H2 | Experienced engineers keep the tool enabled | Of at least 8 interviewed senior engineers who used it for 2 weeks, how many still have it enabled | fewer than 5 of 8 (T0 + 30 days) |
| H3 | Teams will enforce it as a required check | External teams with the CI gate enabled on a real repository | fewer than 3 (T0 + 60 days) |
| H4 | Teams will pay for a hosted layer (policy, audit ledger) | Pre-orders or signed letters of intent at the stated price hypothesis (49–99 USD per team per month — a hypothesis, not a validated price) | fewer than 5 (T0 + 90 days) |

If H1 or H2 fails, the project stops or falls back to the alternative recorded in docs/00-decision-record.md (private per-repository memory), which is itself unproven and must pass its own pre-registered test before any build.

## Definition of a "real finding" (H1)
A finding counts only if a human maintainer who did not write the change inspects the evidence and the diff and agrees that either (a) the claimed verification did not apply to the final content, or (b) a modified file was not exercised by any recorded check. Disagreements are recorded, not discarded. False positives are reported as a rate.

## Instrumentation and ethics
- Dogfooding counters are **local and opt-in** (ADR-0007): counts of findings by type, written to a file the participant reviews and shares voluntarily.
- No third-party repository is instrumented without the maintainers' consent.
- Interviews use one fixed opening question: "The last time an AI-assisted change looked fine and was not, how did you find out, and how long did it take?"

## Technical spikes (before the prototype is considered viable)
- S1. Read-set tracing: measured overhead and false "unexercised" rate on 5 real projects in different languages.
- S2. Attestation round trip: produce, sign and verify an attestation in CI, including whether the forge's own tooling accepts the custom predicate (unverified today).
- S3. Hook schemas: verify the real payloads and exit-code semantics for each supported agent against its current official documentation. **Claude Code: done (2026-10-21, 2.1.302), see docs/status.md.** Codex and Cursor: not done.
- S4. Policy library: evaluate maturity of a CEL implementation in the chosen language.
- S5. Conformance vectors: write the first 50 vectors by hand and run two independent implementations of freshness semantics against them.

## Reporting
Results (including nulls) are committed to docs/research/ with the protocol that was in force, the raw denominators and the stop decision.

## H1 procedure (dogfooding), as built 2026-10-21

Tooling: `opshero journal` (opt-in, local, content-free). The journal records only: the Stop-gate decision and verdict, coarse reason categories (`missing`, `stale`, `failed`, `assurance`, `inconsistent`, `insufficient-runs`, `tree-changed-during-run`), whether a recorded run passed, a one-way hash of the session id, a timestamp. It never records commands, arguments, paths, tree ids, contents or the raw session id; a test fails if any of them appears in the file.

Steps per repository, per developer:

1. `opshero init`, then set the required command in `.opshero/policy.toml` (for this repository: `cargo test *`).
2. `opshero hook install --enforce-stop` (dry run), then again with `--yes`. Hooks are read when a Claude Code session starts: **restart the session**.
3. `opshero journal enable`.
4. Work normally with the agent. When the Stop gate blocks, the agent is told what is missing and continues; it never blocks twice in a row.
5. At the end of a day: `opshero journal list`, then `opshero journal label <line> real|noise` for each block. **Real** means the block was right: the check had not been run on the final content, or had failed, and the agent was about to hand the work over. **Noise** means it was wrong or irrelevant (for example the command was run in a way the tool cannot see, such as through a pipeline).
6. `opshero journal report` against the pre-registered rule: at least 10 % of at least 100 sessions contain a real finding. Below 3 % with 100 sessions or more, the direction is reassessed (roadmap M3). Between the two: inconclusive, keep collecting. The 95 % Wilson interval is printed next to the point estimate; the rule itself uses the point estimate, as pre-registered.

Known bias, stated in advance: the people labelling are the people who built the tool. Labels should be made before looking at the running rate, and a second reader should label a sample of blocks; disagreements are reported, not hidden. Unlabeled blocks are excluded from the numerator, so the rate is a lower bound while labelling is incomplete.

Known blind spot: a command run through a pipeline, a list (`a && b`), a subshell or in the background is not evidence by design (a pipeline's exit status can hide a failure). The gate will then say `missing` even though the agent ran the tests. Whether that is real or noise is exactly what the labels measure; if "noise" dominates for this reason, the shell analysis (RFC-0001) needs revisiting, not the label.

State on 2026-10-21: the hook and the journal are installed in this repository (local scope, not committed: `.claude/settings.local.json`; `.opshero/policy.toml` requires `cargo test *`). Zero sessions recorded so far. The hook did not fire in the session that installed it, as expected (hooks load at session start).

## Pilot P1 (2026-10-21): the mechanism, measured with real agent sessions

**What it is, and what it is not.** 70 headless Claude Code sessions (haiku model, Claude Code 2.1.302) in throwaway repositories, with the hooks and the journal enabled, driven by `scripts/pilot-dogfood.sh` and analysed by `scripts/pilot_analyze.py`. Total API cost: about 0.75 USD. It tests whether the Stop gate blocks what it should and what friction it creates. It says **nothing about demand**: the tasks are scripted, the repository is a 4-function Python module, the agent is a small model, and the same person built the tool and wrote the tasks.

**Ground truth** is computed from the agent's own transcript, independently of opshero: did the content change (git status, plus the agent's edits before the gate acted), was it only Markdown, and did a test command succeed after the last edit. The transcript is cut at the first "Stop hook feedback" event, because what the agent does after the gate intervenes is the gate's effect, not the state it had to judge. (A first analysis without the cut wrongly counted post-block re-runs as prior verification; it was corrected before the numbers below.)

| | Pilot 1 (gate without turn baseline), 30 sessions | Pilot 2 (turn baseline and `paths`), 40 sessions |
|---|---|---|
| Sessions where nothing changed, blocked | **10 of 10** | 0 of 10 |
| Documentation-only sessions, blocked | **5 of 5** | 0 of 5 |
| Code changed and verified before stopping, blocked | 3 of 15 | 4 of 16 |
| All sessions that should be allowed, wrongly blocked | **18 of 30 (60 %)**, 95 % interval 42-75 % | **4 of 31 (13 %)**, 95 % interval 5-29 % |
| Code changed after the tests and not re-verified: caught | not measured | **9 of 9** (95 % interval 70-100 %) |

**Defect found, fixed, re-measured.** Without a baseline the gate demanded test evidence even in turns that changed nothing or only documentation; the agent ran tests for nothing. Fixed with a hook on `UserPromptSubmit` that records the tree at the start of each turn, a Stop gate that judges only what the turn changed, and per-requirement `paths` (git pathspecs) so that a documentation change does not require the test suite. `verify` (CI semantics) ignores both: it has no turn.

**What the remaining 4 wrong blocks are.** All four: the agent verified the code, but through a pipe or a `;` list (`... | tail -5`, `...; echo exit=$?`), which hides the exit status and is deliberately not evidence. The gate blocked once, the agent re-ran the command on its own, all four ended with a pass. Nothing sound lets the tool accept those runs, and none of the four was a pure `&&` chain, so a speculative extension was **not** built. The block message now tells the agent to run the command on its own when no run was seen.

**What the pilot says about the soft gate.** Of the 9 caught sessions, 7 re-ran the tests and finished clean; 2 ignored the block and finished with stale evidence, because the gate never blocks twice in a row (to avoid loops). The local Stop gate is a reminder with an effectiveness around 7 of 9 here, not a guarantee. Enforcement is the CI gate, which does not exist yet.

**Limitations to keep in mind.** Small n (confidence intervals are wide). One small model, one tiny repository, one test runner. The Bash allowlist used in the pilot denied some compound commands, which probably changed how often the agent used pipes. Tasks were written by the tool's author. Whether real engineers on real repositories meet the same frictions, and whether they care, is what H1 on real work and the H0 interviews are for.
