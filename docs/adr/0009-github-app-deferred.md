# ADR-0009: Forge-specific application deferred; core stays forge-agnostic

- Status: Proposed

## Context
A forge application (for example a GitHub App using the Checks API) would give one-click installation and organisation-wide policy. Research findings (docs/research): many organisations use several CI systems (about 41% use more than one in the JetBrains 2025 survey, reported); forges are absorbing adjacent features natively; a hosted app means holding repository access tokens, the exact liability that the previous SaaS handled poorly.

## Decision
- The core integrates through hooks, the CLI and a CI step that work on any forge.
- A forge application is deferred until the core is validated and is designed as an **optional hosted layer** (organisation policy, audit ledger), never a dependency of the core.

## Consequences
- Slower "install in one click" story at first.
- No stored repository tokens in the early life of the project.

## Alternatives considered
- Build the GitHub App first: rejected; higher platform risk and operational burden before demand is proven.
