# Releasing

Status: **v0.0.1 was published on 2026-10-22 as a pre-release; the next release adds a Windows build, the manual page and shell completions (see below)** (https://github.com/NICE-DEV226/opshero-core/releases/tag/v0.0.1). Publishing a release is the maintainer's decision.

## What a release contains
For each target, an archive `opshero-<version>-<target>.tar.gz` (`.zip` on Windows) holding the binary, `LICENSE`, `NOTICE`, `README.md`, the manual page (`share/man/man1/opshero.1`) and shell completions (`completions/`, for bash, zsh, fish, PowerShell and elvish), plus one `SHA256SUMS` file and a signed build-provenance attestation for the archives. The manual page and completions are generated from the program's own definitions by `scripts/gen_extras.py` (also available as `opshero man` and `opshero completions <shell>`).

| Target | Build | Notes |
|---|---|---|
| `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` | static (the build fails if the binary is not static) | no dependency on the glibc of the build machine |
| `aarch64-apple-darwin`, `x86_64-apple-darwin` | native, and cross-compiled for Intel | the Intel binary is not executed on the arm64 runner |
| `x86_64-pc-windows-msvc` | native, `.zip` | **experimental**: the test suite passes on Windows in CI, but it has not been tried with a real agent (docs/status.md) |

Archives are reproducible: sorted entries, zero timestamps, neutral ownership, no gzip name or time. The same inputs give the same bytes, so a checksum can be recomputed from a rebuilt archive. (Two builds of the *binary* on different machines are not claimed to be identical.)

## Cutting a release
1. Set `version` in `Cargo.toml` (`[workspace.package]`), the `opshero-core` dependency version in `crates/opshero-cli/Cargo.toml` and the `version` default in `action.yml` to the same value (the publish job refuses a mismatch), and commit; update `Cargo.lock` if it changes.
2. Tag it: `git tag vX.Y.Z && git push origin vX.Y.Z`. The tag **must** equal `v` plus the Cargo version or the publish job refuses.
3. The `release` workflow builds the five targets, checks that the Linux binaries are static, writes `SHA256SUMS`, signs a provenance attestation with the workflow's identity, and publishes a **pre-release** (the project is pre-alpha).
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

On Windows, `scripts/install.ps1` does the same with `opshero.exe` (installs to `%LOCALAPPDATA%\Programs\opshero\bin`; `PREFIX` and `OPSHERO_VERSION` are honoured). It is tested in CI on a Windows runner against a fake local release (`scripts/test-install.ps1`).

## crates.io
The names `opshero`, `opshero-cli` and `opshero-core` were free on 2026-10-22. The crates carry the metadata crates.io needs and `opshero-core` was packaged and built from its package (`cargo package`); `opshero-cli` can only be verified after `opshero-core` is published. `publish = false` stays until the maintainer decides to publish: it is irreversible (a published version can be yanked, never deleted).

## Not done
`crates.io` publication (decision pending, see above); a Homebrew formula (a tap needs its own repository); reproducible builds of the binary itself; signing with a long-lived key (the attestation uses the workflow identity instead).

## The first release, as verified (2026-10-22)
Tag `v0.0.1` on commit `f2826ac` (CI green). The `release` workflow built four archives (633 to 752 KB) and published them with `SHA256SUMS`. Checked afterwards, as a user would: the four checksums match; `gh attestation verify` succeeds for the archive and names this repository, the workflow `release.yml@refs/tags/v0.0.1`, the commit `f2826ac` and a GitHub-hosted runner; the same command **rejects** an archive with one byte appended; the installer installed `0.0.1` from the real release.

One defect was found in the installer by doing that, and fixed: without an explicit version it asked `releases/latest`, which GitHub answers with a 404 while only pre-releases exist, so a user following the README would have failed. It now falls back to the releases list (and picks the newest tag even when the JSON is on one line; the first fix of that parsing chose the oldest, caught by a test).
