# ADR-0005: Implementation language: Rust

- Status: Proposed

## Context
The tool parses hostile inputs (reports, logs, paths), must start in a few milliseconds inside hooks, ships as a single static binary, and wants a small, auditable trusted core.

## Decision
Rust for the core and CLI. The pure core crate performs no I/O; adapters live in separate crates.

## Consequences
- Memory safety for parsers; excellent fit for fuzzing and property testing.
- Single static binary, fast cold start.
- Higher contribution barrier than Go or Python. Mitigation: policy files, adapters and the conformance suite do not require Rust knowledge.
- Library maturity for some needs (CEL, in-toto/DSSE, Sigstore) must be verified per crate before adoption.

## Alternatives considered
- Go: strong ecosystem for in-toto/Sigstore and an easy contributor path; viable fallback. Slightly larger binary and less ergonomic for the parser-hardening work.
- Python: rejected for startup time and distribution weight.

## Evidence
Single-binary tools that engineers adopted in this space (for example actionlint in Go; zizmor and rtk in Rust) — docs/research/W1, W8.
