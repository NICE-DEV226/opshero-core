# Tests the Windows packaging and installer against a fake local release (a directory, no network):
# a good install, and refusal when the archive or its checksum is wrong. Run from the repository root
# after `cargo build --release --locked`. Needs python3 (as the packaging script does).
$ErrorActionPreference = 'Stop'
function Fail($m) { Write-Host "FAILED: $m"; exit 1 }
function Ok($m) { Write-Host "ok    $m" }

$py = if (Get-Command python3 -ErrorAction SilentlyContinue) { 'python3' } else { 'python' }
$bin = (Resolve-Path 'target\release\opshero.exe').Path
$version = ((& $bin --version) -replace '^opshero ', '').Trim()
$t = Join-Path ([IO.Path]::GetTempPath()) ("opshero-test-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $t | Out-Null
$target = 'x86_64-pc-windows-msvc'
$name = "opshero-$version-$target"

& $py scripts\gen_extras.py --bin $bin --out "$t\extras" | Out-Null
if ($LASTEXITCODE -ne 0) { Fail 'gen_extras.py' }
$rel = "$t\release"; New-Item -ItemType Directory -Path $rel | Out-Null
& $py scripts\package_release.py --target $target --binary $bin --version $version --out "$t\a" --extras "$t\extras" | Out-Null
if ($LASTEXITCODE -ne 0) { Fail 'packaging' }
Copy-Item "$t\a\$name.zip" $rel
Copy-Item "$t\a\$name.zip.sha256" "$rel\SHA256SUMS"
Ok 'packaged the Windows archive'

function Install($prefix) {
    $env:OPSHERO_BASE_URL = $rel; $env:OPSHERO_VERSION = $version; $env:PREFIX = $prefix
    # A refusal writes to stderr; with ErrorActionPreference=Stop PowerShell would turn that text into
    # a fatal error of this test instead of letting us read the message and the exit code.
    $prev = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    $out = & powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install.ps1 2>&1 | Out-String
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    return @{ Code = $code; Out = $out }
}

$r = Install "$t\p1"
if ($r.Code -ne 0) { Write-Host $r.Out; Fail 'a good install failed' }
if (-not (Test-Path "$t\p1\bin\opshero.exe")) { Fail 'the binary was not installed' }
if ((& "$t\p1\bin\opshero.exe" --version) -notmatch [regex]::Escape($version)) { Fail 'installed binary reports another version' }
Ok 'installs and runs'

# a tampered archive is refused and the previous install survives
New-Item -ItemType Directory -Force -Path "$t\p2\bin" | Out-Null
Set-Content "$t\p2\bin\opshero.exe" 'previous'
$keep = "$t\keep.zip"; Copy-Item "$rel\$name.zip" $keep
[IO.File]::AppendAllText("$rel\$name.zip", 'x')
$r = Install "$t\p2"
if ($r.Code -eq 0) { Fail 'a tampered archive was installed' }
if ($r.Out -notmatch 'checksum mismatch') { Write-Host $r.Out; Fail 'wrong message for a tampered archive' }
if ((Get-Content "$t\p2\bin\opshero.exe") -ne 'previous') { Fail 'the existing install was damaged' }
Copy-Item $keep "$rel\$name.zip" -Force
Ok 'refuses a tampered archive and leaves the previous install alone'

# a missing checksum entry is refused
Set-Content "$rel\SHA256SUMS" ''
$r = Install "$t\p2"
if ($r.Code -eq 0) { Fail 'installed without a checksum entry' }
if ($r.Out -notmatch 'no entry') { Write-Host $r.Out; Fail 'wrong message without a checksum entry' }
Ok 'refuses when SHA256SUMS has no entry'

Remove-Item -Recurse -Force $t
Write-Host 'installer and packaging: all checks passed'
