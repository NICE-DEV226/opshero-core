# Releasing

Status: the pipeline is built and was exercised by a manual run on 2026-10-22; **no release has been published yet**. Publishing is the maintainer's decision.

## What a release contains
For each of four targets, an archive `opshero-<version>-<target>.tar.gz` holding the binary, `LICENSE`, `NOTICE` and `README.md`, plus one `SHA256SUMS` file and a signed build-provenance attestation for the archives.

| Target | Build | Notes |
|---|---|---|
| `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` | static (the build fails if the binary is not static) | no dependency on the glibc of the build machine |
| `aarch64-apple-darwin`, `x86_64-apple-darwin` | native, and cross-compiled for Intel | the Intel binary is not executed on the arm64 runner |
| Windows | none | not a supported platform yet (docs/status.md) |

Archives are reproducible: sorted entries, zero timestamps, neutral ownership, no gzip name or time. The same inputs give the same bytes, so a checksum can be recomputed from a rebuilt archive. (Two builds of the *binary* on different machines are not claimed to be identical.)

## Cutting a release
1. Set `version` in `Cargo.toml` (`[workspace.package]`) and commit; update `Cargo.lock` if it changes.
2. Tag it: `git tag vX.Y.Z && git push origin vX.Y.Z`. The tag **must** equal `v` plus the Cargo version or the publish job refuses.
3. The `release` workflow builds the four targets, checks that the Linux binaries are static, writes `SHA256SUMS`, signs a provenance attestation with the workflow's identity, and publishes a **pre-release** (the project is pre-alpha).
4. Before announcing it: download one archive, run `sha256sum -c SHA256SUMS`, run `gh attestation verify <archive> --repo NICE-DEV226/opshero-core`, and try `scripts/install.sh`.

A manual run (`gh workflow run release.yml`) builds and uploads the archives as workflow artifacts and publishes nothing; it is how the pipeline was tested.

## Installing
```
curl -fsSL https://raw.githubusercontent.com/NICE-DEV226/opshero-core/main/scripts/install.sh -o install.sh
less install.sh          # read it first
sh install.sh            # installs to ~/.local/bin; PREFIX=... and OPSHERO_VERSION=... are honoured
```
The installer downloads the archive for the system, verifies it against `SHA256SUMS`, and installs without root. It installs nothing, and leaves an existing install untouched, if the archive was altered, the checksum is missing or wrong, the platform has no binary, or the download fails. This is tested against a fake local release (`scripts/test-install.sh`, run in CI on Linux and macOS, five injected defects all detected) and was run once against the archive built by GitHub.

What the checksum does and does not give: it detects a corrupted or altered download **from the same release page**. It does not protect against a compromised release, which is what the attestation is for (`gh attestation verify` checks that the archive was built by this repository's workflow).

## Not done
Windows binaries; `crates.io` publication (crate names not checked, `publish = false`); a Homebrew formula; reproducible builds of the binary itself; signing with a long-lived key (the attestation uses the workflow identity instead).
