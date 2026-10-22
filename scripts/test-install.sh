#!/bin/sh
# Tests the release packaging and the installer against a fake local release (file:// URLs):
# reproducible archives, a correct install, and refusal to install when anything is wrong.
# Needs: python3, a built binary (target/release/opshero, built here if missing), tar, curl or wget.
set -eu
cd "$(dirname "$0")/.."
fail() { printf 'FAILED: %s\n' "$*" >&2; exit 1; }
ok() { printf 'ok    %s\n' "$*"; }
sha() { if command -v sha256sum >/dev/null 2>&1; then sha256sum | cut -d' ' -f1; else shasum -a 256 | cut -d' ' -f1; fi; }

[ -x target/release/opshero ] || cargo build --release --locked -q
BIN="$PWD/target/release/opshero"
VERSION="$("$BIN" --version | sed 's/^opshero //')"
OS="$(uname -s)"; ARCH="$(uname -m)"
case "$OS/$ARCH" in
  Linux/x86_64) TARGET=x86_64-unknown-linux-musl ;;
  Linux/aarch64|Linux/arm64) TARGET=aarch64-unknown-linux-musl ;;
  Darwin/arm64) TARGET=aarch64-apple-darwin ;;
  Darwin/x86_64) TARGET=x86_64-apple-darwin ;;
  *) echo "SKIPPED: no installer target for $OS/$ARCH"; exit 0 ;;
esac
NAME="opshero-$VERSION-$TARGET"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT INT TERM
REL="$T/release"; mkdir -p "$REL"

# 1. the archive is reproducible
python3 scripts/package_release.py --target "$TARGET" --binary "$BIN" --version "$VERSION" --out "$T/a" > "$T/a.sum"
python3 scripts/package_release.py --target "$TARGET" --binary "$BIN" --version "$VERSION" --out "$T/b" > "$T/b.sum"
cmp -s "$T/a.sum" "$T/b.sum" || fail "two packagings of the same inputs differ"
ok "packaging is reproducible ($(cut -c1-12 "$T/a.sum"))"

# 2. its contents and metadata are what we promise
python3 - "$T/a/$NAME.tar.gz" "$NAME" <<'PY' || fail "archive contents"
import sys, tarfile
t = tarfile.open(sys.argv[1]); name = sys.argv[2]
members = {m.name: m for m in t.getmembers()}
want = {f"{name}/opshero", f"{name}/LICENSE", f"{name}/NOTICE", f"{name}/README.md"}
assert set(members) == want, sorted(members)
assert all(m.mtime == 0 and m.uid == 0 and m.gid == 0 for m in members.values()), "non-neutral metadata"
assert members[f"{name}/opshero"].mode == 0o755 and members[f"{name}/LICENSE"].mode == 0o644
PY
ok "archive holds exactly the binary, LICENSE, NOTICE and README with neutral metadata"

cp "$T/a/$NAME.tar.gz" "$REL/"; cp "$T/a/$NAME.tar.gz.sha256" "$REL/SHA256SUMS"
run_install() { # extra env words..., prefix in $P
  env OPSHERO_BASE_URL="file://$REL" OPSHERO_VERSION="$VERSION" PREFIX="$P" "$@" sh scripts/install.sh
}

# 3. a good install
P="$T/p1"; run_install > "$T/out1" 2>&1 || { cat "$T/out1"; fail "good install failed"; }
[ -x "$P/bin/opshero" ] || fail "binary not installed or not executable"
"$P/bin/opshero" --version | grep -q "$VERSION" || fail "installed binary reports another version"
ok "installs and runs"

# 4. refusals: nothing may be installed, and an existing install must survive
refuse() { # description, then env words for run_install; $P is set by the caller
  if run_install "$@" > "$T/out" 2>&1; then cat "$T/out"; fail "$DESC: should have been refused"; fi
}
P="$T/p2"; mkdir -p "$P/bin"; echo "previous" > "$P/bin/opshero"

DESC="tampered archive"; cp "$REL/$NAME.tar.gz" "$T/keep.tgz"; printf 'x' >> "$REL/$NAME.tar.gz"
refuse; grep -q "checksum mismatch" "$T/out" || fail "$DESC: wrong message"; [ "$(cat "$P/bin/opshero")" = previous ] || fail "$DESC: existing install damaged"
cp "$T/keep.tgz" "$REL/$NAME.tar.gz"; ok "refuses a tampered archive and leaves the previous install alone"

DESC="no checksum entry"; cp "$REL/SHA256SUMS" "$T/keep.sums"; : > "$REL/SHA256SUMS"
refuse; grep -q "no entry" "$T/out" || fail "$DESC: wrong message"; ok "refuses when SHA256SUMS has no entry"

DESC="wrong checksum"; printf '%s  %s\n' "$(printf 0 | sha)" "$NAME.tar.gz" > "$REL/SHA256SUMS"
refuse; grep -q "checksum mismatch" "$T/out" || fail "$DESC: wrong message"; ok "refuses a wrong checksum"
cp "$T/keep.sums" "$REL/SHA256SUMS"

DESC="unsupported platform"; refuse OPSHERO_OS=FreeBSD OPSHERO_ARCH=riscv64
grep -q "no prebuilt binary" "$T/out" || fail "$DESC: wrong message"; ok "refuses an unsupported platform with a pointer to a source build"

DESC="missing archive"; rm "$REL/$NAME.tar.gz"; refuse; grep -q "cannot download" "$T/out" || fail "$DESC: wrong message"
cp "$T/keep.tgz" "$REL/$NAME.tar.gz"; ok "refuses when the archive cannot be downloaded"

[ "$(cat "$P/bin/opshero")" = previous ] || fail "an existing install was modified by a refused run"
ok "no refused run changed the existing install"
echo "installer and packaging: all checks passed"
