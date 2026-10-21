# RFC-0005: Verdict semantics and statistical evidence for flaky commands

- Status: Draft (implemented and tested: laws L1-L12, statistics, `repeat`, `run --repeat`, `post_run_changes`, robustness harness, differential test, TLA+ model)
- Audience: maintainers, policy authors, reviewers of the evaluation core
- Requires: RFC-0001, RFC-0002, RFC-0004

## 1. Why this RFC

The verdict function is the product. Two questions must have precise answers:

1. Which properties does the function guarantee, independently of input?
2. What does a *pass* mean when the command is not deterministic?

## 2. Laws of the verdict function (implemented, tested)

Notation: a *witness* for a check is a record whose command matches, which passed (exit 0, no signal), whose `tree_after` equals the subject tree, whose exclusions equal the subject's, and whose assurance is at least the required one.

| Id | Law | Checked by |
|---|---|---|
| L1 | The report is independent of the order of the evidence | property test, 2000 cases |
| L2 | Soundness: a passing check names real witnesses only | property test |
| L3 | Fail closed: a subject no record is bound to is never PASS (with at least one blocking requirement) | property test |
| L4 | Overall verdict = worst blocking check, FAIL > UNKNOWN > PASS; warn checks never influence it | property test |
| L5 | PASS is monotone: adding evidence never turns PASS into another verdict | property test |
| L6 | Duplicated evidence does not change the verdict | property test |
| L7 | Order of requirements in the policy is irrelevant | property test |
| L8 | Editing any single field of a stored record is detected | property test |
| L9 | Per-check result equals an independent restatement of this RFC's rules written in the test | differential property test |
| L10 | With `repeat`, one failing run on the subject excludes PASS whatever else was recorded | property test |
| L11 | With `repeat`, PASS needs at least N distinct passing witnesses and no failing run | property test |
| L12 | With `post_run_changes = "deny"`, every witness of a PASS is a run whose tree before equals its tree after | property test, plus TLA+ model |

L5 (monotone PASS) holds only for requirements without `repeat`; that is the intended difference.

Test file: `crates/opshero-core/tests/verdict_laws.rs`. The suite was itself checked by mutation: five injected defects in `verify.rs` (ignore exclusions, compare the tree before instead of after, ignore the signal, ignore minimum assurance, let UNKNOWN outrank FAIL) were each detected. One defect initially survived (the signal guard) because the generator never produced "exit 0 with a recorded signal"; the generator was extended and the mutant is now killed.

## 3. Known issue KI-1: "any pass wins"

L5 has a consequence: if a command passed once on a tree and failed on a rerun of the **same** tree, the verdict is PASS. For a deterministic command this cannot happen. For a flaky one it hides the flake. This is pinned by the test `ki1_pass_then_fail_on_the_same_tree_is_pass_today` so the change below is deliberate.

### 3.1 Semantics (implemented; opt-in per requirement, default unchanged in v0)

For one requirement and one tree, let `p` be the number of passing and `f` the number of failing matching runs.

- `f = 0, p >= N`: PASS.
- `f = 0, p < N`: UNKNOWN (not enough repetitions to support the claim).
- `f >= 1, p >= 1`: FAIL with reason "inconsistent results on the same tree" (flaky); never PASS.
- `p = 0`: FAIL as today.

`N = min_passes(alpha, p1)` is the smallest number of consecutive passes such that a command whose true failure probability is at least `p1` would produce them with probability at most `alpha`. Policy syntax: `repeat = { alpha = "5%", flake_rate = "5%" }`, giving `N = 59`. Percentages are exact decimals (at most four decimals, strictly between 0 and 100%), parsed without floating point; anything else is a policy error. Records are counted by distinct id, so supplying the same record twice does not add a run. Only passes with sufficient assurance count toward `N`; a failure of any assurance level counts as an inconsistency.

The old behaviour (KI-1) remains the default for requirements without `repeat`, correct for deterministic commands. Closed for flaky commands by the tests `ki1_closed_by_the_repeat_rule_pass_then_fail_is_inconsistent` and the end-to-end test `repeat_rule_needs_two_passes_and_rejects_a_mixed_history`.

**Meaning of the guarantee**, stated precisely: after `N` passes with no failure, the data reject the hypothesis "failure rate >= p1" at significance level alpha. It is **not** a claim about the probability that the next run passes, and it assumes independent runs; a systematic fault (same environment, same seed) is not covered. Runs from one CI job are not independent in that sense: see the open questions.

### 3.2 Implementation (done)

`opshero_core::stats::min_passes(alpha_ppm, p1_ppm)`: integer fixed-point arithmetic only (scale 10^12), rounding always up, so the result is identical on every platform and never smaller than the exact answer. Verified by: literature values (59, 29, 44, 459, 299), an exact integer oracle over a 300,000-point grid, a floating-point oracle (property test), monotonicity (property test), exact-boundary tests, and a seeded Monte-Carlo simulation (400,000 trials per case) that checks both that the false-accept rate is at most alpha and that `N - 1` runs would not be enough. Mutation check: three of four injected defects killed. The fourth (round down instead of up) survives, and an independent exact computation found no input among 60,000 random ones where it changes the answer; rounding up is kept for the formal guarantee, not because a test needs it.

### 3.3 Mutation check of the `repeat` logic

Six injected defects in `verify.rs` (inconsistency check removed, off-by-one on N, counting passes without regard to assurance, no de-duplication by id, rule ignored, failures miscounted) are each killed. Two survived the first version of the suite (assurance counting, de-duplication) because random evidence rarely stacks several runs on one subject; a concentrated generator and two targeted tests were added, after which both are killed. One test also found a real parser bug while being written: `5.%` was accepted as a percentage.

## 4. Later: sequential decisions

A Wald sequential probability ratio test would decide earlier on mixed data and bound both error types. It needs logarithms; to keep determinism it must use a fixed-point table or rational bounds. Deferred until KI-1 is resolved and real flaky-test data from dogfooding exists.

## 5. Robustness of the parsers (implemented)

`crates/opshero-core/tests/robustness.rs` is a deterministic mutation fuzzer for every parser that sees untrusted bytes (policy files, record files, canonical JSON, shell command lines, timestamps): fixed seeds, corpus of valid inputs, dictionary splicing, bit flips, truncation, duplication; properties are no panic, no call over one second, acceptance implies integrity (a record is accepted only if its id is the hash of its content), canonical JSON idempotent, timestamps equal to a brute-force calendar. 300,000 iterations per seed ran clean. Three injected defects (accept a mismatching id, wrong month constant, unsorted exclusions) are each detected.

This is **not** coverage-guided fuzzing. That needs a nightly toolchain and `cargo-fuzz`, which are not installed here and were not added to the pinned toolchain without a decision. It remains in the roadmap.

## 6. Differential test of canonical JSON (implemented)

`crates/opshero-core/tests/canon_differential.rs` compares our canonical JSON with the ECMAScript serializer RFC 8785 is defined against, run by `node`, on 4,000 random documents per run (control characters, U+2028, surrogate pairs, BMP/astral key order, integer-like keys, the limits of the supported integer range) plus hand-picked traps. All agree. `OPSHERO_REQUIRE_NODE=1` turns a missing `node` into a failure instead of a loud skip.

**Finding about the reference itself.** With Node 24.19.0 (V8 13.6), `JSON.parse` of a large batch intermittently returned the key `\` for an object key that is a single escaped character (`"\r"`, `"\""`, `"\f"`); 1 to 3 documents in 4,000 per run. The same document parsed alone, an independent Python canonicalizer, and our output all agree, so the reference was wrong, not the code under test. The trigger was not minimized (a delta-debugging pass needed 3,818 of the 4,000 documents, so it depends on accumulated state, not a few documents) and it is not reproduced with synthetic data. Consequence for the test: a disagreement only counts when a cold, isolated reference run reproduces it; the number of reference misparses is printed. This is a report of an observation, not a root-cause analysis of V8.

## 7. Formal model and known issue KI-2 (implemented)

`docs/formal/Freshness.tla` (checked with TLC) asks whether the verifier can answer PASS on the strength of a run during which the content changed. Results: the v0 rule (`allow`) is unsound in 4 states; the stricter rule (`deny`: tree before must equal tree after) is exhaustively sound for one change per run; with two changes the ABA case (content changes then returns to what it was) defeats any local rule. See docs/formal/README.md.

**KI-2 (closed 2026-10-21 by changing the default):** the v0 rule accepted a pass for the tree after the run even when the tree before it was different. The option `post_run_changes` (per requirement) now defaults to `"deny"`: such a run does not count, and the message says how to fix it (rerun, or declare generated paths under `[subject] exclude`). `"allow"` remains as an explicit opt-out for commands that legitimately write untracked files; it passes with a finding, as v0 did.

Why the default was flipped before 0.1 and before any dogfooding data: the model shows `allow` is unsound; the cost of `deny` is a FAIL on the first run of a command that creates untracked, non-ignored files (for example a first `pytest` in a repository that does not ignore `__pycache__`), with an actionable message; flipping later would have changed the meaning of data collected under the old default. The Claude Code hook adapter is unaffected: it only knows the tree after the command, so it never sees "tree changed during the run". ABA stays an accepted limit of assurance A1. The dogfooding journal counts these cases under the reason `tree-changed-during-run`, so the friction is measured, not guessed; if it dominates, the answer is better exclusions, not going back to `allow`.

## 8. Open questions

- OQ1. Are repeated runs inside one CI job independent enough to count? Likely not for environment faults; policy may need `repeat.distinct = "job"`.
- OQ2. Cost: `N = 59` runs of a long test suite is unaffordable. The feature targets small, suspect tests and is opt-in per requirement.
- OQ3. How the CLI schedules the repetitions (`opshero run --repeat`) versus only judging runs that already exist. Today only existing runs are judged, and the verdict explains how many more are needed.
- OQ4. Extend the formal model to evidence transported from a local machine to CI, and to concurrent recorders.
- OQ5. Whether the first-run friction of `deny` is acceptable in practice: measured by the dogfooding journal (reason `tree-changed-during-run`).
