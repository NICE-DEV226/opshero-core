# RFC-0004: Policy language

- Status: **Draft — decision open**
- Audience: maintainers, security reviewers, early policy authors
- Requires: RFC-0001, RFC-0002

## 1. Requirements

A policy language for this tool must be:

1. **Total** — every evaluation terminates.
2. **Deterministic** — no I/O, no clock, no randomness during evaluation.
3. **Sandboxed** — cannot read files or call the network.
4. **Analyzable** — a policy can be validated and linted without running it.
5. **Small** — minimal specification surface for a conformance suite.
6. **Reviewable** — readable in a pull request by people who are not language experts.

## 2. Candidates

| Option | For | Against |
|---|---|---|
| Declarative TOML schema with a fixed set of built-in checks | Smallest attack surface; trivial to specify and test | Less expressive; new needs require a new built-in |
| CEL (Common Expression Language) expressions inside the TOML | Non-Turing-complete by design; familiar | Dependency on a library whose maturity in our language of choice is **unverified** |
| Rego (OPA) | Powerful, widely used | Larger runtime and spec; harder to guarantee totality and small footprint |

## 3. Proposal (to be confirmed)

- **v0**: declarative TOML with built-in checks only.
- **v1**: optionally allow CEL for custom predicates, after the conformance suite exists and library maturity is verified.

## 4. Draft examples (illustrative)

```toml
version = 1
id = "agent-pr"

[[require]]
id = "tests-on-final-tree"
check = "evidence.fresh"            # strict: record's tree == subject tree
command = ["pytest", "*"]
min_assurance = "A2"
on_unknown = "block"

[[require]]
id = "change-exercised"
paths = ["src/billing/**"]
check = "evidence.covers_changed_paths"   # scoped: warning in v0
command = ["pytest", "tests/billing*"]
severity = "warn"

[[require]]
id = "new-dependency-exists"
check = "dependency.added_exists_in_registry"   # optional online check
min_age_days = 14
severity = "block"

[subject]
exclude = ["*.log", "dist"]        # git pathspecs; tool directories are excluded by default

[[waiver]]
check = "change-exercised"
reason = "generated code"
expires = "2027-01-31"
```

Implemented since the first draft (RFC-0005): an optional per-requirement `repeat = { alpha = "5%", flake_rate = "5%" }` for commands that may be flaky, and `post_run_changes = "deny" | "allow"` (default `deny`: a run during which the working tree changed is not evidence; `allow` is the explicit opt-out).

## 5. Semantics

- A policy evaluation is a pure function `verdict = eval(policy, subject, evidence_set)`.
- `severity = "warn"` produces a finding but does not change the verdict.
- Waivers must carry an expiry; an expired waiver is ignored and reported.
- Unknown keys are an error (to avoid silently ignored typos).

## 6. The dependency rule

`dependency.added_exists_in_registry` motivates a narrow, deterministic check for dependencies **added** in a change: the package exists in the registry, is older than a threshold, and is not a near-duplicate name of a more popular package. It requires a registry lookup, so it is the only check that may use the network, and only when explicitly enabled. Background and why this is a rule rather than a product: docs/00-decision-record.md.

## 7. Open decisions

- D1. Confirm TOML + built-ins for v0.
- D2. Evaluate CEL library maturity.
- D3. Name the built-in check set and freeze it for v0.
- D4. Waiver governance: who may add a waiver, and how it is audited.
