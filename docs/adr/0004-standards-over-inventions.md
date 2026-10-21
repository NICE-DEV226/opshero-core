# ADR-0004: Standards over inventions

- Status: Proposed

## Context
Credibility and adoption depend on interoperating with tools engineers already use. Inventing formats raises the cost of adoption and the burden of maintaining a spec.

## Decision
Use, in this order of preference:
- **Attestations**: in-toto Statement v1 in a DSSE envelope; structure and parsing rules modelled on the SLSA Verification Summary.
- **Canonicalization**: JSON Canonicalization Scheme (RFC 8785).
- **Findings output**: SARIF 2.1.0.
- **Test results**: JUnit XML, TAP, `go test -json`.
- **Coverage**: lcov, Cobertura.
- **Pipeline telemetry (optional)**: OpenTelemetry CI/CD semantic conventions (status: Development at the time of research; expect churn).

A bespoke format is allowed only for the evidence record (RFC-0002), whose requirements (content binding, ambient inputs, hash chain) are not covered by an existing standard.

## Consequences
- Third parties can verify attestations with generic tooling.
- Some constraints are inherited (e.g. predicate type URIs must be stable).
- Need to track standard evolution.

## Alternatives considered
- A fully custom attestation format: rejected, no interoperability.
- SPDX/CycloneDX for evidence: rejected, they describe components, not executions.

## Evidence
docs/research/W6_prior_art_science.md (standards and reusable parts); SLSA verification summary specification.
