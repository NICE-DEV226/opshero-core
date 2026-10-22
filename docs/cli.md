# Command-line reference

Generated from the program's own `--help` by `scripts/gen-cli-docs.py`; do not edit by hand (CI fails if this page is out of date).

## Exit codes (public contract)

| Code | Meaning |
|---|---|
| 0 | `PASS` (for `run`: the command's own exit code is passed through) |
| 1 | internal error (a bug or an I/O failure) |
| 2 | usage error (bad flag, unreadable policy, a ref that does not resolve) |
| 10 | `FAIL`: the policy is violated by the evidence that exists |
| 11 | `UNKNOWN`: evidence is missing or insufficient (blocking in CI) |

`opshero hook claude-code` always exits 0: a problem in a hook must never break the agent.

## `opshero`

```text
Deterministic evidence that your checks ran on the exact content you are about to merge

Usage: opshero <COMMAND>

Commands:
  init         Write a sample policy to .opshero/policy.toml
  run          Run a command and record it as evidence (exit code is passed through)
  verify       Evaluate the policy against the recorded evidence for the current tree
  ci           Commands for the CI job (assurance A2)
  evidence     Publish or fetch evidence through a dedicated git ref
  hook         Agent integrations
  gc           Delete old evidence, keeping the newest records and everything bound to the current content
  doctor       Check that git, the repository, the policy and the evidence store are in working order
  completions  Print a shell completion script to stdout
  man          Print the manual page (roff) to stdout
  journal      Opt-in local journal for the dogfooding experiment (no commands, paths or ids are stored)
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## `opshero init`

```text
Write a sample policy to .opshero/policy.toml

Usage: opshero init [OPTIONS]

Options:
      --force  Overwrite an existing policy file
  -h, --help   Print help
```

## `opshero run`

```text
Run a command and record it as evidence (exit code is passed through)

Usage: opshero run [OPTIONS] <CMD>...

Arguments:
  <CMD>...  

Options:
      --repeat <N|auto>  Run the command several times and record every run: a count (1..=1000) or `auto` (the number required by the matching policy requirement's `repeat` rule). The exit code is that of the last failing run, or 0 if every run passed
  -h, --help             Print help
```

## `opshero verify`

```text
Evaluate the policy against the recorded evidence for the current tree

Usage: opshero verify [OPTIONS]

Options:
      --policy <POLICY>   Policy file (relative paths are resolved from the repository root) [default: .opshero/policy.toml]
      --format <FORMAT>   [default: text] [possible values: text, json]
      --ci                CI mode: first fetch the evidence published for this exact content from the remote (see `opshero evidence push`). Anything missing or unreadable makes the verdict UNKNOWN
      --remote <REMOTE>   Remote holding the published evidence (with --ci) [default: origin]
      --policy-ref <REF>  Read the policy from this git ref (for example `origin/main`, the target branch) instead of the checked-out content, which the author of the change controls. A ref of the form `<remote>/<branch>` that is not present locally is fetched
  -h, --help              Print help
```

## `opshero ci`

```text
Commands for the CI job (assurance A2)

Usage: opshero ci <COMMAND>

Commands:
  run   Execute the commands the policy requires, record them as assurance A2, and give the verdict. Use the policy of the target branch (--policy-ref), not the one in the pull request
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `opshero ci run`

```text
Execute the commands the policy requires, record them as assurance A2, and give the verdict. Use the policy of the target branch (--policy-ref), not the one in the pull request

Usage: opshero ci run [OPTIONS]

Options:
      --policy <POLICY>   Policy file to enforce (relative paths are resolved from the repository root)
      --allow-local       Run even when the CI environment variable is not set (for testing only)
      --policy-ref <REF>  Read the policy from this git ref (for example `origin/main`) instead of the checked-out content. A `<remote>/<branch>` ref that is not present locally is fetched
  -h, --help              Print help
```

## `opshero evidence`

```text
Publish or fetch evidence through a dedicated git ref

Usage: opshero evidence <COMMAND>

Commands:
  push   Publish the records bound to the current content to refs/opshero/evidence/<tree> on the remote
  fetch  Fetch the evidence published for the current content and add it to the local store
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `opshero evidence push`

```text
Publish the records bound to the current content to refs/opshero/evidence/<tree> on the remote

Usage: opshero evidence push [OPTIONS]

Options:
      --remote <REMOTE>  [default: origin]
  -h, --help             Print help
```

## `opshero evidence fetch`

```text
Fetch the evidence published for the current content and add it to the local store

Usage: opshero evidence fetch [OPTIONS]

Options:
      --remote <REMOTE>  [default: origin]
  -h, --help             Print help
```

## `opshero hook`

```text
Agent integrations

Usage: opshero hook <COMMAND>

Commands:
  claude-code  Event handler invoked BY Claude Code (reads one JSON event on stdin)
  install      Register the hooks in an agent's settings file (dry run unless --yes)
  uninstall    Remove the hooks this tool registered (others are left alone)
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `opshero hook claude-code`

```text
Event handler invoked BY Claude Code (reads one JSON event on stdin)

Usage: opshero hook claude-code

Options:
  -h, --help  Print help
```

## `opshero hook install`

```text
Register the hooks in an agent's settings file (dry run unless --yes)

Usage: opshero hook install [OPTIONS]

Options:
      --agent <AGENT>  [default: claude-code] [possible values: claude-code]
      --scope <SCOPE>  local = .claude/settings.local.json (personal, not committed); project = .claude/settings.json; user = ~/.claude/settings.json [default: local] [possible values: local, project, user]
      --enforce-stop   Also block the agent from finishing while required evidence is missing or stale
      --yes            Apply the change (without it, only show what would change)
  -h, --help           Print help
```

## `opshero hook uninstall`

```text
Remove the hooks this tool registered (others are left alone)

Usage: opshero hook uninstall [OPTIONS]

Options:
      --agent <AGENT>  [default: claude-code] [possible values: claude-code]
      --scope <SCOPE>  [default: local] [possible values: local, project, user]
      --yes            
  -h, --help           Print help
```

## `opshero gc`

```text
Delete old evidence, keeping the newest records and everything bound to the current content

Usage: opshero gc [OPTIONS]

Options:
      --keep <KEEP>  How many of the newest records to keep (the evidence for the current content is kept as well) [default: 2000]
      --dry-run      Only report what would be removed
  -h, --help         Print help
```

## `opshero doctor`

```text
Check that git, the repository, the policy and the evidence store are in working order

Usage: opshero doctor

Options:
  -h, --help  Print help
```

## `opshero completions`

```text
Print a shell completion script to stdout

Usage: opshero completions <SHELL>

Arguments:
  <SHELL>  [possible values: bash, elvish, fish, powershell, zsh]

Options:
  -h, --help  Print help
```

## `opshero man`

```text
Print the manual page (roff) to stdout

Usage: opshero man

Options:
  -h, --help  Print help
```

## `opshero journal`

```text
Opt-in local journal for the dogfooding experiment (no commands, paths or ids are stored)

Usage: opshero journal <COMMAND>

Commands:
  enable   Start recording Stop-gate decisions and recorded runs in `.git/opshero/journal.jsonl`
  disable  Stop recording (the journal is kept unless --delete)
  status   
  list     List the blocking Stop decisions with their line numbers
  label    Say whether a blocking Stop (by line number from `list`) was a real problem or noise
  report   Summary against the pre-registered threshold (at least 10 % of at least 100 sessions)
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## `opshero journal enable`

```text
Start recording Stop-gate decisions and recorded runs in `.git/opshero/journal.jsonl`

Usage: opshero journal enable

Options:
  -h, --help  Print help
```

## `opshero journal disable`

```text
Stop recording (the journal is kept unless --delete)

Usage: opshero journal disable [OPTIONS]

Options:
      --delete  
  -h, --help    Print help
```

## `opshero journal status`

```text
Usage: opshero journal status

Options:
  -h, --help  Print help
```

## `opshero journal list`

```text
List the blocking Stop decisions with their line numbers

Usage: opshero journal list

Options:
  -h, --help  Print help
```

## `opshero journal label`

```text
Say whether a blocking Stop (by line number from `list`) was a real problem or noise

Usage: opshero journal label <LINE> <LABEL>

Arguments:
  <LINE>   
  <LABEL>  [possible values: real, noise]

Options:
  -h, --help  Print help
```

## `opshero journal report`

```text
Summary against the pre-registered threshold (at least 10 % of at least 100 sessions)

Usage: opshero journal report [OPTIONS]

Options:
      --format <FORMAT>  [default: text] [possible values: text, json]
  -h, --help             Print help
```
