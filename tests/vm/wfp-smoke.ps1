#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Runs the WFP default-deny smoke binary inside the test VM.

.DESCRIPTION
    The runner copies this script and wfp-smoke.exe into C:\tvpn-test. The
    binary arms eight filters, checks its own count and disarms them again.
    If it fails, the checkpoint restore removes any leftover filters.
#>
$ErrorActionPreference = 'Stop'

$binary = Join-Path $PSScriptRoot 'wfp-smoke.exe'
if (-not (Test-Path -LiteralPath $binary)) {
    throw "wfp-smoke.exe missing next to $PSCommandPath"
}

$output = & $binary 2>&1
$code = $LASTEXITCODE
$output | ForEach-Object { Write-Host $_ }
if ($code -ne 0) {
    throw "wfp-smoke failed with exit code $code"
}
