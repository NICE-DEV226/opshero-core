# RFC-0001: Principles, scope, threat model and assurance levels

- Status: Draft
- Audience: core maintainers, future contributors, security reviewers
- Supersedes: none
- Requires: none

## 1. Problem

A pull request that contains agent-written code usually comes with an *implicit claim*: "I ran the tests and they pass." Reviewers cannot cheaply tell whether that claim holds for the exact content under review. We want a tool that makes this claim **checkable by machine**, and that is honest about how far its own guarantees go.

## 2. Goals

- G1. Record what verification commands actually ran, and bind each record to the content it ran against.
- G2. Evaluate those records against a written policy and produce a deterministic verdict with machine-readable reasons.
- G3. Emit a signed attestation that a third party can verify later, with no service of ours involved.
- G4. Integrate into the existing workflow (agent hooks, shell, git, CI, pull-request status checks) without a mandatory service, account or daemon.
- G5. Be language-agnostic: operate on processes, files and git objects, not on language semantics.

## 3. Non-goals

- N1. Judging whether a change is **correct**, secure, well-designed, or well-tested.
- N2. Code review, code generation, or fixing failures.
- N3. Any use of a machine-learning model in the product, including optional use. (ADR-0001)
- N4. Replacing CI, test runners, SBOM tools, or vulnerability scanners. We consume their outputs.
- N5. An IDE plugin or a mandatory hosted service.

## 4. Principles

- **P1 Determinism.** Same inputs produce byte-identical outputs (canonical JSON, RFC 8785). No wall-clock, locale, or hash-map ordering dependence in verdicts.
- **P2 Explainability.** Every verdict names the rule and the evidence records that led to it. Explanations are templates over structured facts.
- **P3 Fail closed.** Missing or malformed evidence yields `UNKNOWN`, never `PASS`. Policy decides whether `UNKNOWN` blocks.
- **P4 Bind to content.** Evidence refers to git object ids, never to branch names, commit messages or timestamps.
- **P5 Standards first.** Prefer in-toto, DSSE, SARIF, JUnit, lcov/Cobertura, OpenTelemetry CI/CD conventions over bespoke formats.
- **P6 Minimal trusted surface.** The pure core performs no I/O. Parsers treat all input as hostile.
- **P7 Honest assurance.** Every attestation states the assurance level at which its evidence was produced.
- **P8 Capture never breaks the host.** A failure to *record* must not break an agent session or a build; a failure to *gate* must not pass silently.

## 5. Definitions

- **Subject**: the content being judged, identified by a git tree object id (RFC-0002).
- **Evidence record**: an immutable record of one command execution, bound to a subject state.
- **Check**: a named expectation in policy (e.g. "tests ran on the final tree").
- **Policy**: a versioned file stating which checks are required, for which paths, at which minimum assurance.
- **Verdict**: `PASS`, `FAIL` or `UNKNOWN`, plus reasons.
- **Attestation**: an in-toto Statement wrapped in a DSSE envelope carrying the verdict and references to its inputs.

## 6. Threat model

Adversaries, in increasing capability:

| ID | Adversary | Example |
|---|---|---|
| T1 | Misreporting agent | Claims tests passed without running them, or edits files after the last green run |
| T2 | Negligent operator | Runs tests on a different branch, forgets to re-run |
| T3 | Dishonest operator with control of the dev machine | Rewrites the local evidence log, forges outputs |
| T4 | Hostile content author | Crafts logs/reports/paths to exploit the verifier (parser bugs, path traversal, decompression or regex bombs) |
| T5 | Supply-chain attacker | Compromises our release artifacts or a dependency |

Mitigations and **residual risk**:

- T1/T2 are addressed by binding evidence to the exact tree (RFC-0002) and by policy checks (RFC-0004).
- T3 is **not** defeated by locally recorded evidence. A hash chain makes tampering *evident to an honest verifier who holds an earlier head*, but whoever controls the machine can regenerate the whole chain. Only evidence produced where the operator has no control (CI, assurance A2) or re-execution by an independent party (A3) resists T3. This limit is stated in every attestation via the assurance level.
- T4 is addressed by the engineering standards: fuzzed parsers, size/time limits, no execution of content-derived commands, no network in the verifier. (docs/engineering-standards.md)
- T5 is addressed by reproducible builds, signed releases with build provenance, a minimal dependency set with automated policy checks.

Out of scope: a fully compromised CI platform, a compromised signing identity, side-channel attacks.

## 7. Assurance levels

| Level | Definition | Resists |
|---|---|---|
| A0 | Claim made by an agent or human, no recorded evidence | nothing |
| A1 | Recorded locally by the wrapper/hook, hash-chained, subject-bound | T1, T2 |
| A2 | Recorded by an execution environment the committer does not control (CI job), subject-bound | T1–T3 for that environment |
| A3 | Independently re-executed on the exact subject by the verifier's own environment, with identical inputs | T1–T3 |

Policy states a minimum level per check. Local evidence (A1) is **advisory** unless policy explicitly accepts it.

## 8. What an attestation asserts

An attestation asserts: *"At time of verification, evaluating policy P (digest D) over evidence records {E1…En} for subject S yielded verdict V at assurance level L."* It asserts nothing about correctness (N1).

## 9. Compatibility and versioning

- Attestation predicate: Semantic Versioning; consumers must ignore unknown fields; a breaking change changes the predicate type URI (same convention as the SLSA verification summary).
- CLI: exit codes and JSON output schema are public API (RFC-0003).
- Policy files carry an explicit `version`.

## 10. Open questions

- OQ1. Should the pure core ship as a library with a stable ABI, or only as a CLI in v1? (Proposal: CLI + versioned JSON in v1; library API after the conformance suite exists.)
- OQ2. Minimum viable A3: re-execute only commands declared hermetic by policy, or any? (Proposal: only declared.)
- OQ3. How to treat flaky checks without introducing nondeterminism into the verdict? (Proposal: policy may request N re-runs with an explicit pass rule; the verdict is a pure function of the recorded results.)
