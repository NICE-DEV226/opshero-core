#!/usr/bin/env python3
"""Generate docs/cli.md from the real `--help` output of the binary, or check that it is current.

    scripts/gen-cli-docs.py            rewrite docs/cli.md
    scripts/gen-cli-docs.py --check    exit 1 (and show the first differences) if it is out of date

The page is produced by running the program, never typed by hand, so it cannot drift from the code:
CI runs `--check`. Stdlib only. Uses target/debug/opshero (built first) unless --bin is given.
"""
import argparse, difflib, os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "docs", "cli.md")

HEADER = """# Command-line reference

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
"""


def run_help(binary, args):
    env = dict(os.environ, COLUMNS="100", NO_COLOR="1", CLICOLOR="0")
    out = subprocess.run([binary, *args, "--help"], capture_output=True, text=True, env=env, check=True)
    return out.stdout.rstrip("\n") + "\n"


def subcommands(help_text):
    names, inside = [], False
    for line in help_text.splitlines():
        if line.startswith("Commands:"):
            inside = True
            continue
        if inside:
            if not line.startswith("  "):
                break
            m = re.match(r"^  ([a-z][a-z0-9-]*)\s", line)
            if m and m.group(1) != "help":
                names.append(m.group(1))
    return names


def walk(binary, path, sections):
    text = run_help(binary, path)
    sections.append((path, text))
    for name in subcommands(text):
        walk(binary, path + [name], sections)


def render(binary):
    sections = []
    walk(binary, [], sections)
    parts = [HEADER]
    for path, text in sections:
        title = " ".join(["opshero", *path])
        parts.append(f"\n## `{title}`\n\n```text\n{text}```\n")
    return "".join(parts)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--bin")
    ap.add_argument("--no-build", action="store_true")
    a = ap.parse_args()
    binary = a.bin or os.path.join(ROOT, "target", "debug", "opshero")
    if not a.bin and not a.no_build:
        subprocess.run(["cargo", "build", "--locked", "-q"], cwd=ROOT, check=True)
    new = render(binary)
    if a.check:
        old = open(OUT, encoding="utf-8").read() if os.path.exists(OUT) else ""
        if old != new:
            diff = difflib.unified_diff(old.splitlines(), new.splitlines(), "docs/cli.md (committed)", "docs/cli.md (from the program)", lineterm="", n=1)
            print("\n".join(list(diff)[:40]))
            sys.exit("FAILED: docs/cli.md is out of date; run scripts/gen-cli-docs.py and commit the result")
        print("docs/cli.md is current")
        return
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(new)
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
