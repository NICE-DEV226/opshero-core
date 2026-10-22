# ADR-0007: No account, no telemetry; the verdict never depends on the network

- Status: Proposed

## Context
Engineers adopt local CLIs that "just work". A tool that verifies evidence must not leak the evidence. A tool that stores users' credentials or logs becomes a security and privacy burden; this design stores none.

## Decision
- No account is needed for any core feature.
- No telemetry by default, and none that is on by default in the future.
- `verify` makes no network calls, except the explicitly enabled dependency-registry check (RFC-0004 §6). `verify --ci` and `evidence fetch|push` do use the network, and only to collect or publish *inputs*.
- Command output and environment values are never stored.

## Consequences
- We cannot measure adoption automatically; validation relies on opt-in dogfooding and interviews (docs/validation-plan.md).
- Smaller security surface; nothing sensitive to hold.

## Alternatives considered
- Opt-out telemetry: rejected; trust matters more than metrics for a verifier.

## Addendum (2026-10-22): "offline" was the wrong word
The first version of this ADR, and the README, presented *offline* as a benefit. Developers and agents are almost always connected, and the tool already uses the network to move evidence, so "works offline" is not what we are offering. The word mixed four different properties; they are separated here because only some of them are worth keeping.

| Property | Keep? | Why |
|---|---|---|
| No account, no telemetry, nothing sensitive held by us | **Yes** | a verifier must be more trustworthy than what it verifies; this is about trust, not connectivity |
| **Hermetic verdict**: the verdict is a pure function of policy, content and evidence, with no network input | **Yes, and it is the real property** | anyone can recompute it later (audit, dispute, incident) without any service existing; a gate does not become unavailable because a service is down; no third party can change the answer |
| Fast and local at the point of use (hook, Stop gate) | Yes | the Stop gate runs on every turn of an agent; a network round trip there costs time on each one |
| Works with no connection at all (air-gapped) | Only as a consequence | real for a few regulated environments, not a selling point for the general case. Do not headline it |

**What this does not mean.** It does not mean the product should avoid the network. The rule is narrower: *the network may be used to collect or publish inputs, never inside the evaluation of a verdict*; anything fetched is frozen into the evidence so the verdict stays replayable. That leaves a lot of room to do better with a connection (see the roadmap, "Connected edge"): the trusted policy taken automatically from the target branch, reuse of a result already obtained for the same tree on another machine, cross-checking a CI claim with the forge, independent timestamps and transparency logs for signed evidence, and shared flake statistics for the hosted layer.

**One caveat to verify, not assume.** Some agent sandboxes restrict or disable network access by default (this was the case for hosted coding-agent environments when this was written; check the current behaviour of each before relying on it). The collection steps must therefore degrade to "evidence is missing, the verdict is UNKNOWN", never to a silent pass.
