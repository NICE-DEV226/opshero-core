# ADR-0002: Bind evidence to a git tree object id

- Status: Proposed

## Context
"Stale evidence" is the core failure we detect. Branch names, commit ids and timestamps do not identify content: a commit id changes with message and parents; evidence is often produced before committing.

## Decision
The subject of evidence is a git **tree object id**. For uncommitted work the recorder computes a worktree tree id using a temporary index (`GIT_INDEX_FILE`, add all, write-tree) so the user's real index is never touched. Ignored files and environment that influence results are covered by explicitly declared **ambient inputs**, stored as keyed digests.

## Consequences
- Exact, cheap staleness test: equality of two ids.
- Works for any language and any test runner.
- Evidence is unaffected by amends and rebases that keep content equal.
- Ignored files are invisible to the tree: handled only through declared ambient inputs.
- SHA-1 vs SHA-256 repositories need explicit handling (RFC-0002 OQ1).

## Alternatives considered
- Commit id: rejected, unstable across amends and unavailable pre-commit.
- Hash of changed files only: rejected as the primary mechanism; kept for scoped freshness.
- Timestamps: rejected, not content-based and forgeable.

## Addendum (2026-10-21)
The subject is the worktree tree **minus excluded pathspecs** (tool directories and policy `[subject] exclude`), stored in each record. Discovered when files written by tools after a recorded run made fresh evidence look stale in a real agent session. See RFC-0002 §1.2.
