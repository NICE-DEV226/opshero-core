# Engineering standards

This is the quality bar for the project. Items marked **gate** must be enforced automatically in CI before a change can merge. Items marked **target** are goals we do not claim to have met.

## 1. Specification first
- Every user-visible behaviour is specified in an RFC or ADR *before* implementation. (gate: PR template asks for the RFC reference.)
- Public contracts: exit codes, JSON schema, attestation predicate, policy schema. They are versioned with SemVer; breaking changes require an RFC and a new predicate type URI where relevant.
- A **conformance suite** with golden test vectors (inputs and expected canonical outputs) lives in the repository so that independent verifier implementations can be tested. Vectors are authored and reviewed by humans.

## 2. Determinism
- Same inputs produce byte-identical outputs. (gate: a CI job runs the verifier twice under different locales, time zones, environment variable orderings and filesystem iteration orders and compares output bytes.)
- No wall-clock, randomness or hash-map ordering may influence a verdict. Timestamps are informational fields only.

## 3. Testing pyramid
- Unit tests for the pure core, written without I/O.
- **Property-based tests** for canonicalization, freshness semantics and policy evaluation. (gate)
- **Differential tests** against `git` plumbing for tree computation and diffs. (gate)
- **Fuzzing** of every parser (JUnit, TAP, lcov, Cobertura, policy, evidence records) with a continuously running corpus; a crash is a release blocker. (gate for a smoke run per PR; continuous elsewhere)
- End-to-end tests against real repositories created on the fly, including dirty working directories, renames, submodules and ignored files.
- Mutation testing on the core crate, tracked over time. (target)

## 4. Hostile-input hygiene
- All files read from the repository, reports and logs are untrusted.
- Hard limits on file size, nesting depth, record counts and processing time, with bounded, explicit errors.
- Paths are normalised and rejected if they escape the repository; symlinks are never followed outside it.
- The verifier never executes commands derived from content. Only `run` executes the user-supplied command, and only as the user.
- `unsafe` code is forbidden in the pure core and requires review and justification elsewhere. (gate)

## 5. Performance budgets
- Recording overhead: at most 50 ms on a 3,000-file repository and 200 ms on 30,000 files (RFC-0003 §6; the former 10 ms target was withdrawn with its measurements, docs/status.md). Gate once the benchmark exists.
- `verify` on a typical pull request: below 1 s. (gate)
- Benchmarks run in CI and regressions beyond a stated threshold fail the build.

## 6. Failure semantics
- Typed errors; no panics in the core; clippy with warnings denied. (gate)
- Missing evidence is `UNKNOWN`, never `PASS`. Capture failures never break the host process; gate failures never pass silently (RFC-0001 P8).

## 7. Supply chain
- Minimal dependency set; each new dependency requires a short written justification in the PR.
- Automated dependency policy checks (licenses, advisories, banned crates) and review of new transitive dependencies. (gate)
- Pinned toolchain and locked dependencies.
- Reproducible builds; signed releases with build provenance and an SBOM. (target: demonstrable reproducibility; we will state a supply-chain level only after it is independently verifiable)
- Third-party CI actions are pinned by full commit SHA. (gate)

## 7b. Release process
- Releases are cut from a tagged commit by CI only, never from a developer machine.
- Release notes list changes to public contracts explicitly.

## 8. Documentation
- Every exit code, JSON field, CLI flag and policy key is documented, with an example. (gate: schema-to-docs check)
- `opshero explain` must be able to describe every finding the tool can emit.
- Output honours `NO_COLOR` and is stable for scripting.

## 9. Review and governance
- Changes to the core crate require review by two maintainers. A CODEOWNERS file enforces it. (gate)
- Contributions use DCO sign-off.
- Security reports follow SECURITY.md.
- **AI-assisted contributions (open question):** the product contains no model (ADR-0001), but contributors may use assistants. Proposal: contributors are fully responsible for what they submit, must disclose substantial assistance, and conformance vectors must be validated by a human against the specification. To be decided by the maintainers.

## 10. Claims discipline
Documentation must not claim more than the tool does. In particular it certifies *process evidence*, not correctness (RFC-0001 §8). Any marketing or README statement that goes beyond that is a bug.
