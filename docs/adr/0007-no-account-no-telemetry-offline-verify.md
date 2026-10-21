# ADR-0007: No account, no telemetry, offline verification

- Status: Proposed

## Context
Engineers adopt local CLIs that "just work". A tool that verifies evidence must not leak the evidence. A tool that stores users' credentials or logs becomes a security and privacy burden; this design stores none.

## Decision
- No account is needed for any core feature.
- No telemetry by default, and none that is on by default in the future.
- `verify` makes no network calls, except the explicitly enabled dependency-registry check (RFC-0004 §6).
- Command output and environment values are never stored.

## Consequences
- We cannot measure adoption automatically; validation relies on opt-in dogfooding and interviews (docs/validation-plan.md).
- Smaller security surface; nothing sensitive to hold.

## Alternatives considered
- Opt-out telemetry: rejected; trust matters more than metrics for a verifier.
