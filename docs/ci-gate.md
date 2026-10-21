# CI gate: publish evidence, verify it in CI

Status: prototype, 2026-10-21 (transport and A2 execution implemented). The commands below are implemented and tested end to end against a real bare remote and real clones (`crates/opshero-cli/tests/e2e.rs`, section "Evidence transport"). The CI workflow at the end is an **example that has not run on a real runner**.

## The loop

1. A developer or an agent works; the checks run through `opshero run` or the agent hook, which records evidence bound to the exact content (git tree id).
2. `opshero evidence push [--remote origin]` publishes the records bound to the **current content** to `refs/opshero/evidence/<tree id>` on the remote. Records for other trees are not published; records already published for the same tree by other machines are merged in, not overwritten. Pushing twice with the same records gives the same commit (fixed identity and dates).
3. CI checks out the content and runs `opshero verify --ci [--remote origin] [--format json]`. It computes the tree id of the checkout, fetches `refs/opshero/evidence/<that id>`, checks the hash of every record, stores the valid ones, and evaluates the policy exactly as `verify` does.
4. Exit codes are the public ones: 0 PASS, 10 FAIL, 11 UNKNOWN. **Anything missing makes the verdict UNKNOWN, which blocks**: no evidence for this tree, evidence for an older tree, remote unreachable, a record whose hash does not match. Each case is a line under `findings`.

`opshero verify` without `--ci` never touches the network.

## What it protects against, and what it does not

| Threat | Covered | Why |
|---|---|---|
| The agent or developer forgot to run the checks, or changed the code after running them | **Yes** | evidence is bound to the content; a changed file is a different tree id |
| An honest mistake or a stale local run | **Yes** | same reason |
| A published record edited after the fact (for example a failing run turned into a pass) | **Yes** | every record carries the hash of its own content; a mismatch is rejected at import and never stored |
| A dishonest publisher inventing a complete, well-formed record | **No** | the record is assurance A1, produced by the publisher. Resisting this needs evidence produced by the CI itself or signed (assurance A2/A3, RFC-0002), not built yet |
| Someone with write access to `refs/opshero/*` deleting evidence | Fail closed | the verdict becomes UNKNOWN, never PASS |

So today the gate catches the mistakes the pilot measured (docs/validation-plan.md, Pilot P1); it does not stop an adversary.

## Assurance A2: the CI executes the checks itself

Publishing evidence (above) is assurance A1: the developer's machine writes the proof, so a dishonest developer can write a false one. **A2 removes the developer from the proof.** The CI job runs the required commands itself, on the exact content of the pull request, and records the results.

```
opshero ci run --policy <trusted policy>
```

What it does, in order: reads the policy; refuses to start if a requirement cannot be executed as written (a command pattern ending with `*` needs `run = ["pytest", "-q"]`, a concrete command that must match the pattern); runs each distinct required command with its output going to the CI log; records each run with assurance **A2** (adapter `ci`); evaluates the policy and exits with the usual codes (0 PASS, 10 FAIL, 11 UNKNOWN). A failing command does not stop the others. A command that changes the working tree fails under the default `post_run_changes = "deny"` rule.

Policy side: `min_assurance = "A2"` on a requirement means a developer's local run no longer satisfies it, only a run by the CI does.

How A2 stays honest:

- **Published evidence can never be A2.** Records fetched with `evidence fetch` or `verify --ci` are stored apart (`.git/opshero/imported/`) and read back with their assurance capped at A1 *everywhere*, whatever they claim, including in later commands on a runner that keeps its checkout. A publisher can write a well-formed record that says A2; it is still counted as A1 (tested with a record whose hash is valid for the lie).
- **`ci run` refuses to run when the `CI` environment variable is not set** (`--allow-local` overrides it, for testing). Otherwise a developer could stamp A2 on a local run by accident.
- **Use the policy of the target branch, not the one in the pull request.** A pull request that changes `.opshero/policy.toml` could remove or weaken a requirement. Fetch the trusted file from the base branch and pass it with `--policy`.

What A2 does **not** do, stated plainly:

- It trusts the CI. Whoever can change the workflow, or the runner, can change what is recorded. The workflow file must be protected (branch rules or required reviewers, `CODEOWNERS`).
- It trusts the test commands. The commands run the pull request's own code and scripts; a pull request that edits the tests so that they always pass produces a genuine A2 pass. Reading the diff of the tests is still a human job.
- It proves that the command exited 0 on that content, not that the command tests what matters.
- Signing (assurance A3) would let a third party check that a record came from the CI; it is not built.

## Pitfalls found while designing it

- **Pull-request checkouts.** GitHub's `pull_request` event checks out the *merge commit* of the pull request into the base branch by default. That tree is not the tree the developer ran the checks on, so there will be no evidence for it. Check out the head of the pull request instead (`ref: ${{ github.event.pull_request.head.sha }}`). The consequence to accept: the gate vouches for the content of the pull request, not for the result of merging it; the ordinary CI test run still covers the merge result.
- **What is published.** The records contain the command line of each recorded run, arguments included, and the working directory. `evidence push` says so every time. Only commands that the policy asks for are recorded, but a secret passed on such a command line would be published. Do not put secrets in required commands.
- **Permissions.** Publishing needs permission to create refs under `refs/opshero/`. Pull requests from forks cannot publish to the upstream repository, so they have no evidence and the gate says UNKNOWN: a maintainer re-runs the checks (A2 evidence, once it exists) or publishes after review.
- **Same tree, two machines at once.** The merge on push reads the remote ref and writes a new commit; two simultaneous pushes for the same tree can lose one machine's records (last writer wins). The lost records can be published again by the next push from that machine. Not locked in this prototype.
- **Refs accumulate** (one per tree). There is no `evidence prune` yet; they are cheap (a few kilobytes) and can be deleted with `git push origin --delete <ref>`.

## Example workflows (not yet run on a real runner)

A2, the strong form: the CI runs the checks. Nothing from the author is trusted.

```yaml
name: opshero-gate
on: pull_request
permissions:
  contents: read
jobs:
  gate:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with:
          # The content the author wrote, not the merge commit.
          ref: ${{ github.event.pull_request.head.sha }}
      - name: Take the policy from the target branch, not from the pull request
        run: |
          git fetch --depth=1 origin "${{ github.base_ref }}"
          git show FETCH_HEAD:.opshero/policy.toml > "$RUNNER_TEMP/policy.toml"
      # Install the opshero binary here (no release tooling exists yet).
      - run: opshero ci run --policy "$RUNNER_TEMP/policy.toml"
```

A1, the light form: the author publishes the evidence; useful to catch forgotten or stale checks without re-running them.

```yaml
      - run: opshero verify --ci --format json
```

Make the job a required status check in the branch protection rules. Teach developers and agents to run `opshero evidence push` before pushing (a `pre-push` git hook can do it; it is never installed without consent).
