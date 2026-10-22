# opshero

> Deterministic, replayable evidence that the checks you rely on **actually ran on the exact content you are about to merge**.

**Status: pre-alpha, Linux only, not yet released.** Implemented and tested: evidence records bound to a git tree, a deterministic policy verdict (PASS / FAIL / UNKNOWN), a Claude Code adapter (hooks, optional Stop gate), flaky-command handling (`repeat`), publishing evidence to a git ref and fetching it in CI (`opshero verify --ci`), and CI-executed evidence at assurance A2 (`opshero ci run`). **Not built:** signed attestations (assurance A3), Codex and Cursor adapters, a packaged CI action, release binaries, macOS and Windows support. Exact state, measurements and known limits: [docs/status.md](docs/status.md). **Demand for this tool is unproven.**

### Try it
```
cargo build --release          # Rust 1.98, pinned by rust-toolchain.toml
target/release/opshero init    # writes .opshero/policy.toml: edit the required command
target/release/opshero run -- <your test command>
target/release/opshero verify  # exit 0 PASS, 10 FAIL, 11 UNKNOWN
```
CI gate and the A2 workflow: [docs/ci-gate.md](docs/ci-gate.md).

## Why this exists

Coding agents now write a large share of changes, and the bottleneck moved from *writing* code to *trusting* it. Two failure patterns are cheap to detect by machine and expensive for a human reviewer to catch:

1. **Stale evidence** — "tests pass" was true for an earlier version of the change, not for the one in the pull request.
2. **Unexercised change** — the checks that ran never touched the files that were modified.

This tool records what ran, binds that record to the **git tree** it ran against, evaluates it against a small written policy, and gives a verdict that a CI gate (or a human) can recompute later from the same inputs, with no service involved. A signed attestation of that verdict is planned, not built offline.

## What it certifies — and what it does not

| It certifies (process) | It does **not** certify |
|---|---|
| These commands ran, with these exit codes, on this exact content | That the change is *correct* |
| The final content equals (or is provably unaffected relative to) the content that was checked | That the tests are *good* |
| The evidence was produced at assurance level A0–A3 (see RFC-0001) | Anything about code quality, security, or intent |

If a statement in the documentation sounds like it certifies more than the left column, that is a bug in the documentation. Please open an issue.

## Design principles (summary)

- **No machine-learning model anywhere in the product.** A verifier must be more trustworthy than what it verifies, so it must be deterministic and auditable. Explanations are generated from rule identifiers and structured facts. ([ADR-0001](docs/adr/0001-no-llm-in-product.md))
- **Evidence is bound to content, not names.** The subject is a git tree object id. ([ADR-0002](docs/adr/0002-bind-evidence-to-git-tree.md))
- **Fail closed, say "unknown".** Missing evidence is never reported as a pass.
- **The verdict never depends on the network.** It is a pure function of the policy, the content and the recorded evidence, so anyone can recompute it later and get the same answer, and a gate does not fail because a service is down. The network is used at the edges, to *collect* inputs (publish and fetch evidence, `verify --ci`), never inside the verdict. No account, no daemon, no telemetry. ([ADR-0007](docs/adr/0007-no-account-no-telemetry-offline-verify.md))
- **Standards over inventions.** in-toto/DSSE attestations, SARIF, JUnit, lcov/Cobertura. ([ADR-0004](docs/adr/0004-standards-over-inventions.md))
- **Honest assurance levels.** A locally recorded log can be rewritten by whoever controls the machine; only independent execution resists a dishonest operator. ([RFC-0001](docs/rfc/0001-principles-threat-model-assurance.md))

## How it fits into a workflow (sketch)

```
 capture ─ agent hooks · shell wrapper · CI ──► evidence records (immutable files under .git/)
                                                      │
 git (tree, diff) ──► PURE CORE (no I/O, deterministic) ◄── policy (non-Turing-complete)
                       · bind evidence ↔ git tree
                       · staleness / coverage-of-change checks
                       · policy evaluation → canonical attestation
                                   │
  outputs: JSON · SARIF · Check Run · job summary · signed DSSE attestation
```

Daily use is silent: hooks record what ran. A developer or agent runs `opshero verify` before pushing; CI runs it again as a required status check. See [RFC-0003](docs/rfc/0003-integration-contracts-and-workflows.md) for the full workflows and integration contracts.

## Documentation map

| Document | Purpose |
|---|---|
| [docs/00-decision-record.md](docs/00-decision-record.md) | How we got here: evidence, what was refuted, what is still unproven |
| [docs/rfc/0001-…](docs/rfc/0001-principles-threat-model-assurance.md) | Principles, scope, threat model, assurance levels |
| [docs/rfc/0002-…](docs/rfc/0002-evidence-model-and-attestation.md) | Evidence record, subject binding, attestation format |
| [docs/rfc/0003-…](docs/rfc/0003-integration-contracts-and-workflows.md) | CLI contract, agent/CI/PR integrations, user workflows |
| [docs/rfc/0004-…](docs/rfc/0004-policy-language.md) | Policy language (draft, open decisions) |
| [docs/adr/](docs/adr/README.md) | Architecture decision records |
| [docs/engineering-standards.md](docs/engineering-standards.md) | The quality bar: testing, fuzzing, performance, supply chain |
| [docs/validation-plan.md](docs/validation-plan.md) | Hypotheses and pre-committed kill criteria |
| [docs/research/](docs/research/README.md) | Raw research reports and the pre-registered commons experiment |

## Honest status of the bet

The pain (verifying agent output) is the best-evidenced one we found, but much of that evidence comes from vendors, and **demand for this specific tool is not proven**. [docs/validation-plan.md](docs/validation-plan.md) fixes in advance the results that would make us stop.

## License

Not yet decided. Proposal: Apache-2.0 for the core and the attestation format, with any hosted layer under a source-available license ([ADR-0006](docs/adr/0006-licensing-proposal.md)). No license file is added until that decision is reviewed.
