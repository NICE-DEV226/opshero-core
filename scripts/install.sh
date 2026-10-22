#!/bin/sh
# Install opshero from a GitHub release: download the archive for this system, verify its SHA-256
# against the release's SHA256SUMS, and put the binary in $PREFIX/bin (default ~/.local/bin).
# No root needed, nothing outside $PREFIX is touched, and nothing is installed if a check fails.
#
#   sh install.sh                       latest release
#   OPSHERO_VERSION=0.1.0 sh install.sh a specific version
#   PREFIX=/usr/local sh install.sh      another prefix (needs write access to it)
#
# Prefer to read the script before running it rather than piping it into a shell; the checksums
# and the build attestations (`gh attestation verify`) are on the release page.
#
# Test and unusual-setup overrides: OPSHERO_BASE_URL (where the archive and SHA256SUMS live),
# OPSHERO_OS and OPSHERO_ARCH (what `uname -s` and `uname -m` would say).
set -eu

REPO="NICE-DEV226/opshero-core"
PREFIX="${PREFIX:-$HOME/.local}"
die() { printf 'opshero install: %s\n' "$*" >&2; exit 1; }

os="${OPSHERO_OS:-$(uname -s)}"
arch="${OPSHERO_ARCH:-$(uname -m)}"
case "$os/$arch" in
  Linux/x86_64)            target="x86_64-unknown-linux-musl" ;;
  Linux/aarch64|Linux/arm64) target="aarch64-unknown-linux-musl" ;;
  Darwin/arm64)            target="aarch64-apple-darwin" ;;
  Darwin/x86_64)           target="x86_64-apple-darwin" ;;
  *) die "no prebuilt binary for $os/$arch. Build from source: cargo build --release (see the README)" ;;
esac

fetch() { # url dest
  if command -v curl >/dev/null 2>&1; then curl -fsSL -o "$2" "$1"
  elif command -v wget >/dev/null 2>&1; then wget -q -O "$2" "$1"
  else die "curl or wget is required"; fi
}
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | cut -d' ' -f1
  else die "sha256sum or shasum is required to verify the download"; fi
}

version="${OPSHERO_VERSION:-}"
if [ -z "${OPSHERO_BASE_URL:-}" ]; then
  if [ -z "$version" ]; then
    tmp_v="$(mktemp)"
    fetch "https://api.github.com/repos/$REPO/releases/latest" "$tmp_v" || die "cannot find the latest release (is there one yet?)"
    version="$(sed -n 's/.*"tag_name": *"v\{0,1\}\([^"]*\)".*/\1/p' "$tmp_v" | head -n 1)"
    rm -f "$tmp_v"
    [ -n "$version" ] || die "cannot read the latest version"
  fi
  base="https://github.com/$REPO/releases/download/v$version"
else
  base="$OPSHERO_BASE_URL"
  [ -n "$version" ] || die "set OPSHERO_VERSION when OPSHERO_BASE_URL is used"
fi

name="opshero-$version-$target"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT INT TERM

printf 'opshero install: %s for %s\n' "$version" "$target"
fetch "$base/$name.tar.gz" "$work/$name.tar.gz" || die "cannot download $base/$name.tar.gz"
fetch "$base/SHA256SUMS" "$work/SHA256SUMS" || die "cannot download $base/SHA256SUMS"

expected="$(grep "  $name.tar.gz\$" "$work/SHA256SUMS" | cut -d' ' -f1 | head -n 1)"
[ -n "$expected" ] || die "SHA256SUMS has no entry for $name.tar.gz: refusing to install"
actual="$(sha256_of "$work/$name.tar.gz")"
[ "$expected" = "$actual" ] || die "checksum mismatch for $name.tar.gz (expected $expected, got $actual): refusing to install"

tar -xzf "$work/$name.tar.gz" -C "$work"
[ -f "$work/$name/opshero" ] || die "the archive does not contain the binary"
mkdir -p "$PREFIX/bin"
cp "$work/$name/opshero" "$PREFIX/bin/opshero.new"
chmod 755 "$PREFIX/bin/opshero.new"
mv -f "$PREFIX/bin/opshero.new" "$PREFIX/bin/opshero"
printf 'opshero install: installed %s\n' "$PREFIX/bin/opshero"
"$PREFIX/bin/opshero" --version
case ":$PATH:" in *":$PREFIX/bin:"*) ;; *) printf 'opshero install: add %s/bin to your PATH\n' "$PREFIX" ;; esac
