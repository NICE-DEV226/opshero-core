# ADR-0006: Licensing — Apache-2.0 core, source-available hosted layer

- Status: **Accepted for the core on 2026-10-22** (owner decision: Apache-2.0); the license of any hosted layer is still open. No legal review was obtained; the owner decided without it

## Context
Research on open-core developer tools (docs/research/W5) shows: winners monetize hosted state, not the local tool; relicensing a popular project triggered community forks (HashiCorp to OpenTofu; Semgrep rules to Opengrep); Sentry's Functional Source License kept the company independent.

## Decision (proposal)
- Core, CLI, attestation format and conformance suite: Apache-2.0, including a patent grant.
- Any future hosted layer (team ledger, organisation policy, audit): a source-available license such as FSL, with automatic conversion to a permissive license after a fixed period.
- Contribution via DCO sign-off, no CLA.

## Consequences
- Maximum adoption and standardization for the format; clear boundary for what is paid.
- License choice is hard to reverse (forks). Decide once, before the first public release.

## Alternatives considered
- AGPL core: rejected for adoption friction in company environments.
- BSL/FSL for the core: rejected, it undermines the standard-format goal.

## Open
Entity jurisdiction and payment channels affect how the paid layer can be sold; see docs/research/W7_market_gtm.md (some national payout options are not supported by the major platforms).

## Addendum (2026-10-22)
The owner chose Apache-2.0 for the whole repository as it stands. `LICENSE` holds the unmodified text from apache.org (11,358 bytes), `NOTICE` carries the copyright line, and `Cargo.toml` declares `license = "Apache-2.0"`. Inbound contributions are covered by section 5 of the license (inbound = outbound), so no CLA is needed; a DCO sign-off is optional and not enforced yet. Dependencies are checked for license compatibility in CI (`cargo deny`). Not decided: the license of a hosted layer, and the trademark policy for the name.
