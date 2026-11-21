#requires -Version 5.1
<#
.SYNOPSIS
    Parses every PowerShell script in the repository and fails on syntax errors.

.DESCRIPTION
    Uses the built-in PowerShell parser, so it needs no extra modules and runs on
    both Windows PowerShell 5.1 and PowerShell 7. CI (.github/workflows/ci.yml)
    and `just pscheck` call it.

.PARAMETER Path
    Optional list of script files. Default: every *.ps1 in the repository,
    excluding build output and VM transcripts.
#>
[CmdletBinding()]
param(
    [string[]]$Path
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not $Path) {
    $Path = Get-ChildItem -Path $repoRoot -Recurse -File -Filter '*.ps1' |
        Where-Object { $_.FullName -notmatch '[\\/](target|node_modules|out)[\\/]' } |
        Select-Object -ExpandProperty FullName
}

$failures = 0
foreach ($file in $Path) {
    $tokens = $null
    $parseErrors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile($file, [ref]$tokens, [ref]$parseErrors)
    if ($parseErrors -and $parseErrors.Count -gt 0) {
        $failures += $parseErrors.Count
        foreach ($parseError in $parseErrors) {
            Write-Host ('{0}:{1}: {2}' -f $file, $parseError.Extent.StartLineNumber, $parseError.Message)
        }
    }
}

if ($failures -gt 0) {
    Write-Host ('pscheck: {0} syntax error(s).' -f $failures)
    exit 1
}

Write-Host ('pscheck: {0} script(s) parsed, no syntax errors.' -f @($Path).Count)
exit 0
