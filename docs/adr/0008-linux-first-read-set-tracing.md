# ADR-0008: Linux-first read-set tracing with coverage-import fallback

- Status: Proposed

## Context
Detecting "the checks never touched the changed files" language-agnostically needs the set of files a run actually read. This is the approach used by hermetic build systems. Platform support differs widely.

## Decision
- v0: observe the read set on Linux with an unprivileged mechanism (candidates: ptrace or seccomp-based tracing; fanotify and eBPF need privileges). The choice is to be settled by a spike with measured overhead.
- Other platforms: record `observation.method = coverage-import` using lcov/Cobertura/language coverage output where available, otherwise `none`.
- Scoped-freshness checks stay **warnings** until false-"unexercised" rates are measured.

## Consequences
- Dev machines on macOS/Windows have weaker evidence; CI (typically Linux) gives the strong signal.
- Known false positives: changed files consumed by an earlier unrecorded build step; tests running inside an already-loaded process. Documented in RFC-0002 §4.
- Hardened containers may restrict tracing; the verifier must degrade, not fail.

## Alternatives considered
- Language-specific analyzers: rejected, breaks language-agnosticism and multiplies maintenance.
- Relying on coverage only: kept as the portable fallback, not the primary mechanism.
