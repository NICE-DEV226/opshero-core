# RFC-0003: Integration contracts and user workflows

- Status: Draft
- Audience: implementers, integrators, early adopters
- Requires: RFC-0001, RFC-0002

Commands and outputs below are **proposed contracts**, not existing behaviour.

## 1. Integration surfaces

| Surface | Mechanism | Contract |
|---|---|---|
| Coding agents | Agent hook systems that run a command around tool calls | A thin adapter converts the agent payload to `opshero record` |
| Shell | `opshero run -- <cmd>` | Records one execution, passes through stdout/stderr and the exit code unchanged |
| Git | Optional `pre-push` hook calling `opshero verify` | Never installed without consent |
| CI | Composite action / CI component / plain command | `opshero verify --ci`, minimal permissions |
| Pull request | Required status check + job summary | Verdict and reasons; never a bot comment by default |
| Attestation | DSSE envelope attached to the check or the run | RFC-0002 |

### 1.1 Agent adapters (verification status of the underlying facts)

- **Claude Code** (verified against Claude Code 2.1.302 with a real session, see docs/status.md): hooks are configured in `settings.json` / `settings.local.json`. `PostToolUse` fires after a successful call with `tool_name`, `tool_input` (Bash: `command`, `description`), `tool_response` (`stdout`, `stderr`, `interrupted`, `isImage`; **no exit code**), `duration_ms`, `cwd`. A Bash command that exits non-zero fires `PostToolUseFailure` with `error` starting `Exit code N` and `is_interrupt`. `Stop` carries `stop_hook_active` and `last_assistant_message`; printing `{"decision":"block","reason":…}` makes the agent continue. Adapter rules: record only simple commands the policy asks for; never record pipelines or lists; never block a Stop when `stop_hook_active` is true.
- **Cursor**: hooks in `.cursor/hooks.json` with `beforeShellExecution` / `afterShellExecution`. The CLI variant is reported to support fewer events than the IDE. *Source: Cursor docs plus a community forum thread; unverified for the current version.*
- **Codex CLI**: hooks reported generally available with PreToolUse/PostToolUse style events, configured in `config.toml` or `hooks.json`. *Source: third-party blogs; the primary OpenAI reference was not retrieved. Unverified.*

Adapter contract: an adapter is a small versioned program that maps one agent payload to one `opshero record` invocation. **Adapters fail open** for recording (P8): if recording fails, the agent continues and a warning is logged.

## 2. CLI surface (v0 proposal)

```
opshero init                       # write .opshero/policy.toml (committed) and print next steps
opshero hook install --agent <a>   # show a diff of the agent config, ask before writing
opshero run -- <cmd> [args…]       # execute and record
opshero record --from-stdin        # used by adapters
opshero verify [--ci] [--policy P] [--format text|json|sarif]
opshero explain <verdict-id|check-id>
opshero attest --sign <key|oidc>   # emit DSSE attestation
```

### 2.1 Exit codes (public API)

| Code | Meaning |
|---|---|
| 0 | `PASS` |
| 1 | Internal error (bug, I/O failure) |
| 2 | Usage error |
| 10 | `FAIL` (policy violated by present evidence) |
| 11 | `UNKNOWN` (insufficient or malformed evidence) |

Whether `UNKNOWN` blocks is a **policy** decision; in `--ci` mode the default is to block.

### 2.2 Output formats

`text` (human), `json` (versioned schema), `sarif` (SARIF 2.1.0 for code-scanning UIs), plus the attestation. JSON and SARIF include rule ids and evidence record ids for every finding.

## 3. Workflows

### 3.1 One-time setup (developer, ~2 minutes)

1. Install a single static binary.
2. `opshero init` creates `.opshero/policy.toml`.
3. `opshero hook install --agent <a>` shows the exact configuration change and asks for confirmation.

Nothing else changes: no daemon, no account, no network.

### 3.2 Daily work with an agent (silent)

The agent runs tests; the hook records command, exit code, duration, the worktree tree id and (where available) the read set. The developer sees nothing unless something is wrong.

### 3.3 Before pushing

```
opshero verify
  subject: tree 9f3c…  (14 files changed)
  FAIL  tests-on-final-tree   last passing run was on tree 41ab…; 3 files changed since
  WARN  change-exercised      src/billing/tax.py not read by any recorded test run
  Verdict: FAIL   Assurance: A1 (local, advisory)
  Next: opshero run -- pytest
```

The same result is available as JSON with an exit code so an agent can correct itself.

### 3.4 Pull-request time (CI)

A CI job runs `opshero verify --ci`. Depending on the policy it either records the checks itself (A2) or re-executes declared-hermetic checks on the exact tree (A3). It publishes a **required status check**, a job summary, and a signed attestation. The default is no pull-request comments.

### 3.5 Review

The reviewer sees one status. Expanding it lists: which checks ran, on which tree, at which assurance level, under which policy, with `opshero explain` available for every line.

### 3.6 Team policy (maintainer, occasionally)

`policy.toml` lives in the repository and is reviewed like code: required checks per path, minimum assurance, waivers with an expiry date.

## 4. Failure modes

| Situation | Behaviour |
|---|---|
| No evidence for a required check | `UNKNOWN`; blocks in CI by default |
| Evidence recorded on another tree | `FAIL` for strict checks (stale) |
| Platform without read-set tracing | `observation.method` is `none` or `coverage-import`; scoped checks degrade to warnings |
| Recording fails inside an agent session | Agent continues; warning logged; later `verify` reports `UNKNOWN` |
| Hostile or oversized report file | Parse rejected with a bounded error; never a crash or hang |
| Flaky test | Policy-defined re-run rule; the verdict is a pure function of recorded results |

## 5. How it is *not* detected that a pull request is agent-written

There is no reliable, unforgeable signal for "written by an agent" (commit trailers can be forged). v0 policies therefore apply **by path pattern or by label**, not by supposed author.

## 6. Performance budgets (to be enforced in CI)

- Overhead of recording one command: at most 50 ms on a 3,000-file repository and 200 ms on a 30,000-file repository (measured: 26 ms and 109 ms, docs/status.md). The earlier 10 ms target was withdrawn: it is not reachable without a resident daemon (ADR-0007).
- `verify` on a typical pull request (hundreds of files, tens of records): under 1 s (measured 17 ms and 79 ms, and 142 ms with 6,000 records in the store; the store is pruned automatically above 5,000 records).

## 7. Open questions

- OQ1. Exact transport of locally recorded evidence to CI (git refs vs artifacts).
- OQ2. A GitHub-App distribution as an optional hosted layer is deferred (ADR-0009 candidate); the core must not depend on it.
- OQ3. Windows support level in v0.
