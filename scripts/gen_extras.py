#!/usr/bin/env python3
"""Generate the manual page and the shell completions that ship in the release archives.

    gen_extras.py --bin PATH --out DIR

Writes, from the program's own definitions (never typed by hand):
    DIR/share/man/man1/opshero.1
    DIR/completions/opshero.bash  _opshero (zsh)  opshero.fish  opshero.ps1  opshero.elv

The output depends only on the command-line definition, not on the platform, so any build of the
same source gives the same files. Stdlib only.
"""
import argparse, os, subprocess, sys

COMPLETIONS = [("bash", "opshero.bash"), ("zsh", "_opshero"), ("fish", "opshero.fish"),
               ("powershell", "opshero.ps1"), ("elvish", "opshero.elv")]


def capture(binary, *args):
    out = subprocess.run([binary, *args], capture_output=True, check=True)
    if not out.stdout:
        sys.exit(f"{binary} {' '.join(args)} printed nothing")
    return out.stdout


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    man = os.path.join(a.out, "share", "man", "man1")
    comp = os.path.join(a.out, "completions")
    os.makedirs(man, exist_ok=True)
    os.makedirs(comp, exist_ok=True)
    with open(os.path.join(man, "opshero.1"), "wb") as f:
        f.write(capture(a.bin, "man"))
    for shell, name in COMPLETIONS:
        with open(os.path.join(comp, name), "wb") as f:
            f.write(capture(a.bin, "completions", shell))
    print(f"wrote the manual page and {len(COMPLETIONS)} completion scripts under {a.out}")


if __name__ == "__main__":
    main()
