# ADR-0003: Stateless-first; evidence as immutable files under `.git/`

- Status: Proposed

## Context
A persistent store with schemas and migrations is a common source of complexity ("a store becomes a quagmire"). Most information we need already lives in git and in the CI platform.

## Decision
- No database and no daemon. Derive everything possible from git and from evidence records.
- Evidence records are small immutable JSON files under `.git/opshero/records/` (never in the working tree). Deleting them loses only advisory local history.
- Readers tolerate unknown fields. A record is self-describing via `record_version`.
- Retention: a simple size/age cap, documented, and a `gc` command.

## Consequences
- Nothing to migrate or repair; easy to reason about and to back up.
- Cross-machine sharing requires explicit transport (RFC-0002 §6).
- No query engine; `verify` reads what it needs.

## Alternatives considered
- Embedded database (SQLite): rejected for v0, adds schema/migration burden for little gain.
- Hosted store: rejected as a dependency of the core; possible later as an optional layer (ADR-0009).

## Addendum (2026-10-21): concurrency and retention
Several recorders may run at once (agent hooks, parallel test jobs). Reading the chain head and appending a record is one step under an exclusive file lock, and the private index is used under its own lock; both fall back safely (record without the lock after 10 s, compute the tree from scratch after 5 s) because recording must never block the host. Retention: the store is pruned automatically above 5,000 records, and by `opshero gc`; evidence bound to the current content is never pruned. Deleting a record can only turn a PASS into UNKNOWN, never the reverse, so pruning cannot create a false pass. Found by stress probes, see docs/status.md.
