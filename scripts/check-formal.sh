#!/usr/bin/env bash
# Runs the TLA+ models in docs/formal with TLC and checks the EXPECTED outcome of each.
# Two of the three configurations are meant to be violated: they document the limits
# of local evidence (docs/formal/README.md). A "fix" that makes them pass is a bug here.
#
# Needs: java, curl (only to fetch TLC the first time), sha256sum.
# Usage: scripts/check-formal.sh            (downloads tla2tools.jar into ./target/tla)
#        TLA_JAR=/path/tla2tools.jar scripts/check-formal.sh
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=1.8.0
SHA256=5bd7b64db99ae4c6c35f290de5e809c0ee41af43f8699cf696099ffb6452b1e2
JAR="${TLA_JAR:-target/tla/tla2tools.jar}"

if [ ! -f "$JAR" ]; then
  mkdir -p "$(dirname "$JAR")"
  curl -fsSL -o "$JAR" "https://github.com/tlaplus/tlaplus/releases/download/v${VERSION}/tla2tools.jar"
fi
JAR=$(realpath "$JAR")
actual=$(sha256sum "$JAR" | cut -d' ' -f1)
if [ "$actual" != "$SHA256" ]; then
  echo "tla2tools.jar checksum mismatch: expected $SHA256, got $actual" >&2
  exit 1
fi

fail=0
check() { # name expected(violated|ok)
  local out
  out=$(cd docs/formal && java -XX:+UseParallelGC -cp "$JAR" tlc2.TLC -config "$1.cfg" Freshness.tla 2>&1 || true)
  local got=unexpected
  if grep -q "Invariant Sound is violated" <<<"$out"; then got=violated
  elif grep -q "No error has been found" <<<"$out"; then got=ok; fi
  if [ "$got" = "$2" ]; then echo "ok    $1: $got (expected)"; else echo "FAIL  $1: got $got, expected $2" >&2; echo "$out" | tail -20 >&2; fail=1; fi
}

check Freshness_allow_edits1  violated   # v0 rule: PASS on content the command did not run against
check Freshness_strict_edits1 ok         # deny rule: sound for one change per run (exhaustive)
check Freshness_strict_edits2 violated   # ABA: a self-reverting change defeats any local rule
exit $fail
