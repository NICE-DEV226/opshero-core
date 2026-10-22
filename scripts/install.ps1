# Install opshero on Windows from a GitHub release: download the archive for this system, verify its
# SHA-256 against the release's SHA256SUMS, and put opshero.exe in a per-user directory.
# No administrator rights are needed and nothing is installed if a check fails.
#
#   .\install.ps1                              latest release
#   $env:OPSHERO_VERSION = "0.1.0"; .\install.ps1   a specific version
#   $env:PREFIX = "C:\tools\opshero"; .\install.ps1 another directory (the binary goes in <PREFIX>\bin)
#
# Read the script before running it rather than piping it into a shell. The checksums and the build
# attestations (`gh attestation verify`) are on the release page.
#
# Test overrides: OPSHERO_BASE_URL (a URL or a local directory holding the archive and SHA256SUMS),
# OPSHERO_API_URL (where the releases list is read), OPSHERO_ARCH (what the system would report).
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Repo = 'NICE-DEV226/opshero-core'
function Die($msg) { [Console]::Error.WriteLine("opshero install: $msg"); exit 1 }

$arch = if ($env:OPSHERO_ARCH) { $env:OPSHERO_ARCH } else { $env:PROCESSOR_ARCHITECTURE }
if ($arch -ne 'AMD64' -and $arch -ne 'x86_64') {
    Die "no prebuilt binary for Windows/$arch. Build from source: cargo build --release (see the README)"
}
$target = 'x86_64-pc-windows-msvc'

function Fetch($src, $dest) {
    if ($src -match '^https?://') {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -UseBasicParsing -Uri $src -OutFile $dest
    } else {
        Copy-Item -LiteralPath $src -Destination $dest
    }
}

$version = $env:OPSHERO_VERSION
if (-not $version) {
    $api = if ($env:OPSHERO_API_URL) { $env:OPSHERO_API_URL } else { "https://api.github.com/repos/$Repo" }
    $tmp = New-TemporaryFile
    try {
        # `releases/latest` ignores pre-releases; the list does not.
        try { Fetch "$api/releases/latest" $tmp } catch { Fetch "$api/releases" $tmp }
        $m = [regex]::Match((Get-Content -Raw $tmp), '"tag_name"\s*:\s*"v?([^"]+)"')
        if (-not $m.Success) { Die 'cannot find the latest release (is there one yet?)' }
        $version = $m.Groups[1].Value
    } catch {
        Die 'cannot find the latest release (is there one yet?)'
    } finally {
        Remove-Item -Force $tmp -ErrorAction SilentlyContinue
    }
}
$base = if ($env:OPSHERO_BASE_URL) { $env:OPSHERO_BASE_URL.TrimEnd('/', '\') } else { "https://github.com/$Repo/releases/download/v$version" }
$sep = if ($base -match '^https?://') { '/' } else { [IO.Path]::DirectorySeparatorChar }

$name = "opshero-$version-$target"
$work = Join-Path ([IO.Path]::GetTempPath()) ("opshero-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
try {
    Write-Host "opshero install: $version for $target"
    $zip = Join-Path $work "$name.zip"
    $sums = Join-Path $work 'SHA256SUMS'
    try { Fetch "$base$sep$name.zip" $zip } catch { Die "cannot download $base$sep$name.zip" }
    try { Fetch "$base${sep}SHA256SUMS" $sums } catch { Die "cannot download $base${sep}SHA256SUMS" }

    $line = Get-Content $sums | Where-Object { $_ -match "\s$([regex]::Escape($name)).zip$" } | Select-Object -First 1
    if (-not $line) { Die "SHA256SUMS has no entry for $name.zip: refusing to install" }
    $expected = ($line -split '\s+')[0].ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $zip).Hash.ToLowerInvariant()
    if ($expected -ne $actual) { Die "checksum mismatch for $name.zip (expected $expected, got $actual): refusing to install" }

    Expand-Archive -LiteralPath $zip -DestinationPath $work -Force
    $exe = Join-Path $work "$name\opshero.exe"
    if (-not (Test-Path -LiteralPath $exe)) { Die 'the archive does not contain the binary' }

    $prefix = if ($env:PREFIX) { $env:PREFIX } else { Join-Path $env:LOCALAPPDATA 'Programs\opshero' }
    $bin = Join-Path $prefix 'bin'
    New-Item -ItemType Directory -Force -Path $bin | Out-Null
    $staged = Join-Path $bin 'opshero.exe.new'
    Copy-Item -LiteralPath $exe -Destination $staged -Force
    Move-Item -LiteralPath $staged -Destination (Join-Path $bin 'opshero.exe') -Force
    Write-Host "opshero install: installed $(Join-Path $bin 'opshero.exe')"
    & (Join-Path $bin 'opshero.exe') --version
    if (($env:PATH -split ';') -notcontains $bin) {
        Write-Host "opshero install: add $bin to your PATH"
    }
    Write-Host 'opshero install: shell completions: opshero completions powershell'
    Write-Host 'opshero install: check your setup in a repository with: opshero doctor'
} finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
