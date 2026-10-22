# RFC-0002: Evidence model, subject binding and attestation format

- Status: Draft
- Audience: implementers of recorders, verifiers and CI integrations
- Requires: RFC-0001

## 1. Subject identification

The subject of every statement is a **git tree object id** (in the repository's hash algorithm). Rationale: a tree id is a content address of the checked-in state; two states with the same tree id are byte-identical for every tracked path.

### 1.1 Computing the subject of a dirty working directory

Evidence is often produced before the work is committed. The recorder computes the *worktree tree id* without touching the user's index:

1. Create a temporary index file (`GIT_INDEX_FILE`).
2. Populate it from the working directory honouring ignore rules (`git add -A` against the temporary index).
3. `git write-tree` against the temporary index. The result is the worktree tree id.

This never modifies the real index, refs, or working tree.

### 1.2 Excluded paths

Files written by developer tools (agent session files, logs) must not change the subject. The subject is therefore the worktree tree **minus a set of excluded pathspecs**: the built-in tool directories (`.claude`, `.codex`, `.cursor`, `.opshero`) plus the policy's `[subject] exclude` entries (RFC-0004), sorted and de-duplicated. The list is stored in the record (`subject.exclude`) and evidence only applies to a verification that uses the same list; changing the exclusions makes older evidence stale. *Found by a real agent session in which a payload log written after the recorded run made fresh evidence look stale.*

### 1.3 Ambient inputs

Ignored files and the environment can change a result without changing the tree (for example a git-ignored `.env.local`). Policy may name **ambient inputs**: ignored paths and environment variable *names*. The recorder stores, for each, a keyed digest (value never stored). A change in an ambient input invalidates evidence exactly like a tree change.

## 2. Evidence record

One record per command execution. Canonical form: JSON Canonicalization Scheme (RFC 8785). Identifier: lowercase hex SHA-256 of the canonical bytes of the record **excluding** the `id` field.

```json
{
  "record_version": 1,
  "id": "<sha256 of canonical record without id>",
  "prev": "<id of previous record in this chain, or null>",
  "subject": { "worktree_tree": "<tree id before the run>", "tree_after": "<tree id after the run>", "head_commit": "<id or null>", "dirty": true },
  "command": { "argv": ["pytest", "-q"], "cwd": "tests/..", "env_names": ["CI", "PYTHONPATH"] },
  "ambient": { "paths": { ".env.local": "<keyed digest>" }, "env": { "PAYMENTS_MODE": "<keyed digest>" } },
  "execution": { "exit_code": 0, "signal": null, "started_at": "<RFC3339>", "duration_ms": 1234 },
  "observation": {
    "method": "none | ptrace | fanotify | coverage-import",
    "read_set": [ { "path": "src/a.py", "blob": "<git blob id>" } ],
    "results": [ { "format": "junit", "path": "report.xml", "digest": "<sha256>", "summary": { "tests": 120, "failed": 0, "skipped": 3 } } ]
  },
  "recorder": { "name": "opshero", "version": "0.0.0", "assurance": "A1" }
}
```

Notes:

- `started_at` and `duration_ms` are informational and **never** used to order evidence or decide a verdict (P1). Ordering is by the hash chain.
- Command output is **not stored**, only digests of declared result files. This avoids storing secrets and bounds record size.
- `read_set` is optional. Its absence is a fact ("not observed"), never an error.
- Paths are repository-relative, normalised, and rejected if they escape the repository.

## 3. Hash chain

Each record names the previous record's `id`. A verifier that holds an earlier chain head can detect truncation or rewriting after that point. The chain does **not** prevent a dishonest operator from regenerating it (RFC-0001, T3).

## 4. Freshness semantics

A record `E` is **fresh** for subject `S` when one of the following holds:

- **Strict**: `E.subject.tree_after == S.tree` and every ambient digest in `E` equals the current one. (Decision made while implementing v0: a run that merely leaves untracked artifacts behind stays fresh until the code changes. If `worktree_tree != tree_after`, the verifier reports a finding that the command modified the worktree.)
- **Scoped** (only if the check allows it and `E.observation.read_set` is present): every path that differs between `E.subject.worktree_tree` and `S.tree` is outside `E.read_set` and outside the check's declared inputs.

Known limitation of scoped freshness: if a build step consumed a changed source file in an *earlier, unrecorded* invocation and the recorded command only ran pre-built artifacts, the changed file will not appear in the read set and will be reported as unexercised. Scoped checks therefore start as **warnings** (RFC-0004).

## 5. Attestation

An in-toto Statement v1 inside a DSSE envelope.

```json
{
  "_type": "https://in-toto.io/Statement/v1",
  "subject": [ { "name": "git-tree", "digest": { "gitTree": "<tree id>" } } ],
  "predicateType": "<PLACEHOLDER — URI to be reserved>/verification/v0.1",
  "predicate": {
    "verifier": { "id": "opshero", "version": "0.0.0" },
    "policy": { "id": "agent-pr", "digest": { "sha256": "<digest>" } },
    "inputs": [ { "digest": { "sha256": "<evidence record id>" } } ],
    "checks": [ { "id": "tests-on-final-tree", "result": "PASS", "evidence": ["<record id>"], "assurance": "A2" } ],
    "verdict": "PASS",
    "assurance": "A2",
    "verified_at": "<RFC3339>"
  }
}
```

- Envelope: DSSE with `payloadType` `application/vnd.in-toto+json`.
- The predicate type URI is a **placeholder**: we must control the namespace before publishing a stable URI.
- Structure and parsing rules are modelled on the SLSA Verification Summary (consumers ignore unknown fields; minor changes are backward compatible; a major change changes the URI).
- `verified_at` is informational.

### 5.1 Signing

- CI: keyless signing with the CI platform's OIDC identity (Sigstore-compatible).
- Local: an Ed25519 key; the public key is distributed out of band. Locally signed attestations are A1 at best.
- Verification needs no service once it holds the trust roots: an auditor can replay it years later. (Obtaining a keyless signature, and checking a transparency log, do use the network at signing time; neither is part of recomputing the verdict.)

## 6. Storage and transport

- Records: `.git/opshero/records/<id>.json` (never inside the working tree).
- Optional transport: a dedicated ref namespace (`refs/opshero/evidence/<tree id>`) so CI can fetch evidence produced locally. Exact ref layout is an open question.
- Attestations: attached to the pull request by the CI integration (RFC-0003); never required to be committed.

## 7. Privacy

No environment values, no command output, no file contents are stored. Paths and git object ids are stored. Anything beyond that requires an explicit policy opt-in and is out of scope for v1.

## 8. Open questions

- OQ1. SHA-1 vs SHA-256 git repositories: define subject digests for both.
- OQ2. Do we store the read set as paths+blobs (large) or as a Merkle summary with on-demand expansion?
- OQ3. Reserve and publish the predicate type namespace.
- OQ4. Whether GitHub's artifact attestation service accepts and verifies our custom predicate through its own CLI is **unverified**; confirm against its official documentation before relying on it.

## Addendum (2026-10-21): transport decision for the prototype (OQ1)
Evidence travels through a dedicated git ref per subject tree, `refs/opshero/evidence/<tree id>`, pointing at a parentless commit with one `<record id>.json` file per record, built with fixed identity and dates so the same records give the same commit. Reasons: it needs no service and no account (ADR-0007), it uses the credentials the CI already has to fetch the repository, and the subject tree id is exactly the key CI needs. Alternatives kept for later: CI artifacts (no ref pollution, but tied to one CI product) and git notes (tied to a commit id, which is not stable across amends). Details, threat coverage and pitfalls: docs/ci-gate.md.
