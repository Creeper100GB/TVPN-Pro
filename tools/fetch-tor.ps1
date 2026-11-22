#requires -Version 5.1
<#
.SYNOPSIS
    Downloads and verifies the pinned Tor Expert Bundle for the installer.

.DESCRIPTION
    Stages tor.exe, geoip and geoip6 under tools/tor/. The files are build
    inputs and are not committed. The SHA-256 comes from the signed
    sha256sums-signed-build.txt published by the Tor Project.
#>
$ErrorActionPreference = 'Stop'

$version = '15.0.26'
$sha256 = 'c1988c701de2f24f7d098d1d6d7f507b58f27740d097f29c246e0524d639b252'
$base = "https://archive.torproject.org/tor-package-archive/torbrowser/$version"
$name = "tor-expert-bundle-windows-x86_64-$version.tar.gz"
$root = Split-Path -Parent $PSScriptRoot
$target = Join-Path $root 'tools\tor'
$cache = Join-Path $env:TEMP $name
$archive = Join-Path $env:TEMP "tvpn-$name"

New-Item -ItemType Directory -Force -Path $target | Out-Null

& curl.exe -sS -L --fail --max-time 600 -o $archive "$base/$name"
if ($LASTEXITCODE -ne 0) { throw "download failed: $LASTEXITCODE" }

$actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $sha256) {
    Remove-Item -LiteralPath $archive -Force
    throw "SHA-256 mismatch: $actual"
}

$extract = Join-Path $env:TEMP "tvpn-tor-$version"
Remove-Item -LiteralPath $extract -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $extract | Out-Null
& tar.exe -xzf $archive -C $extract
if ($LASTEXITCODE -ne 0) { throw "extract failed: $LASTEXITCODE" }

Copy-Item -LiteralPath (Join-Path $extract 'tor\tor.exe') -Destination (Join-Path $target 'tor.exe') -Force
Copy-Item -LiteralPath (Join-Path $extract 'data\geoip') -Destination (Join-Path $target 'geoip') -Force
Copy-Item -LiteralPath (Join-Path $extract 'data\geoip6') -Destination (Join-Path $target 'geoip6') -Force

Write-Host "Tor $version staged in $target (sha256 $sha256)"
