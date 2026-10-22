# Architecture Decision Records

Each ADR states context, the decision, consequences and the alternatives we rejected. Status values: **Accepted**, **Proposed** (needs review/decision), **Superseded**.

| # | Decision | Status |
|---|---|---|
| [0001](0001-no-llm-in-product.md) | No machine-learning model anywhere in the product | Accepted (maintainer decision) |
| [0002](0002-bind-evidence-to-git-tree.md) | Bind evidence to a git tree object id | Proposed |
| [0003](0003-stateless-first-evidence-in-git-dir.md) | Stateless-first; evidence as immutable files under `.git/` | Proposed |
| [0004](0004-standards-over-inventions.md) | Standards over inventions (in-toto/DSSE, SARIF, JUnit, lcov) | Proposed |
| [0005](0005-implementation-language-rust.md) | Implementation language: Rust | Proposed |
| [0006](0006-licensing-proposal.md) | Licensing: Apache-2.0 core, source-available hosted layer | Proposed — needs legal review |
| [0007](0007-no-account-no-telemetry-offline-verify.md) | No account, no telemetry; the verdict never depends on the network | Proposed |
| [0008](0008-linux-first-read-set-tracing.md) | Linux-first read-set tracing with coverage-import fallback | Proposed |
| [0009](0009-github-app-deferred.md) | Forge-specific application deferred; core stays forge-agnostic | Proposed |
