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
python3 scripts/gen_extras.py --bin "$BIN" --out "$T/extras" > /dev/null || fail "gen_extras.py"
python3 scripts/package_release.py --target "$TARGET" --binary "$BIN" --version "$VERSION" --out "$T/a" --extras "$T/extras" > "$T/a.sum"
python3 scripts/package_release.py --target "$TARGET" --binary "$BIN" --version "$VERSION" --out "$T/b" --extras "$T/extras" > "$T/b.sum"
cmp -s "$T/a.sum" "$T/b.sum" || fail "two packagings of the same inputs differ"
ok "packaging is reproducible ($(cut -c1-12 "$T/a.sum"))"

# 2. its contents and metadata are what we promise
python3 - "$T/a/$NAME.tar.gz" "$NAME" <<'PY' || fail "archive contents"
import sys, tarfile
t = tarfile.open(sys.argv[1]); name = sys.argv[2]
members = {m.name: m for m in t.getmembers()}
want = {f"{name}/{p}" for p in ["opshero", "LICENSE", "NOTICE", "README.md", "share/man/man1/opshero.1",
        "completions/opshero.bash", "completions/_opshero", "completions/opshero.fish",
        "completions/opshero.ps1", "completions/opshero.elv"]}
assert set(members) == want, sorted(members)
assert all(m.mtime == 0 and m.uid == 0 and m.gid == 0 for m in members.values()), "non-neutral metadata"
assert members[f"{name}/opshero"].mode == 0o755 and members[f"{name}/LICENSE"].mode == 0o644
PY
ok "archive holds exactly the binary, LICENSE, NOTICE, README, manual page and completions with neutral metadata"

# 2b. the Windows archive is a reproducible zip with opshero.exe (the Linux binary stands in for it)
WIN="opshero-$VERSION-x86_64-pc-windows-msvc"
python3 scripts/package_release.py --target x86_64-pc-windows-msvc --binary "$BIN" --version "$VERSION" --out "$T/w1" --extras "$T/extras" > "$T/w1.sum"
python3 scripts/package_release.py --target x86_64-pc-windows-msvc --binary "$BIN" --version "$VERSION" --out "$T/w2" --extras "$T/extras" > "$T/w2.sum"
cmp -s "$T/w1.sum" "$T/w2.sum" || fail "two zip packagings of the same inputs differ"
python3 - "$T/w1/$WIN.zip" "$WIN" <<'PY' || fail "windows archive contents"
import sys, zipfile
z = zipfile.ZipFile(sys.argv[1]); name = sys.argv[2]
names = set(z.namelist())
assert f"{name}/opshero.exe" in names and f"{name}/opshero" not in names, sorted(names)
assert f"{name}/share/man/man1/opshero.1" in names and f"{name}/LICENSE" in names
assert all(i.date_time == (1980, 1, 1, 0, 0, 0) for i in z.infolist()), "timestamps are not neutral"
assert z.testzip() is None
PY
ok "the Windows archive is a reproducible zip holding opshero.exe"

cp "$T/a/$NAME.tar.gz" "$REL/"; cp "$T/a/$NAME.tar.gz.sha256" "$REL/SHA256SUMS"
run_install() { # extra env words..., prefix in $P
  env OPSHERO_BASE_URL="file://$REL" OPSHERO_VERSION="$VERSION" PREFIX="$P" "$@" sh scripts/install.sh
}

# 3. a good install
P="$T/p1"; run_install > "$T/out1" 2>&1 || { cat "$T/out1"; fail "good install failed"; }
[ -x "$P/bin/opshero" ] || fail "binary not installed or not executable"
"$P/bin/opshero" --version | grep -q "$VERSION" || fail "installed binary reports another version"
[ -f "$P/share/man/man1/opshero.1" ] || fail "the manual page was not installed"
ok "installs and runs, with the manual page"

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

# 4b. no version given: it is found through the releases API (a fake local one)
API="$T/api"; run_noversion() { env OPSHERO_BASE_URL="file://$REL" OPSHERO_API_URL="file://$1" PREFIX="$P" sh scripts/install.sh; }
json() { printf '[{"tag_name": "v%s", "prerelease": true}, {"tag_name": "v0.0.0"}]\n' "$VERSION"; }
# (a) a stable release exists: /releases/latest answers
mkdir -p "$API/a/releases"; printf '{"tag_name": "v%s"}\n' "$VERSION" > "$API/a/releases/latest"
P="$T/p3"; run_noversion "$API/a" > "$T/out" 2>&1 || { cat "$T/out"; fail "latest via /releases/latest"; }
[ -x "$P/bin/opshero" ] || fail "nothing installed via /releases/latest"; ok "finds the latest stable release"
# (b) only pre-releases exist: /releases/latest is a 404, so the list is used (the situation of v0.0.1)
mkdir -p "$API/b"; json > "$API/b/releases"
P="$T/p4"; run_noversion "$API/b" > "$T/out" 2>&1 || { cat "$T/out"; fail "pre-release only: the list fallback is missing"; }
[ -x "$P/bin/opshero" ] || fail "nothing installed from a pre-release-only repository"; ok "falls back to the releases list when only pre-releases exist"
# (c) no release at all
mkdir -p "$API/c"; P="$T/p5"
if run_noversion "$API/c" > "$T/out" 2>&1; then fail "no release: should have been refused"; fi
grep -q "cannot find the latest release" "$T/out" || fail "no release: wrong message"; ok "says so clearly when there is no release"

P="$T/p2" # back to the prefix that holds the "previous" install used by the refusal tests
DESC="missing archive"; rm "$REL/$NAME.tar.gz"; refuse; grep -q "cannot download" "$T/out" || fail "$DESC: wrong message"
cp "$T/keep.tgz" "$REL/$NAME.tar.gz"; ok "refuses when the archive cannot be downloaded"

[ "$(cat "$P/bin/opshero")" = previous ] || fail "an existing install was modified by a refused run"
ok "no refused run changed the existing install"
echo "installer and packaging: all checks passed"
