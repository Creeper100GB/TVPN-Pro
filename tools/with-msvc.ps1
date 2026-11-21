#requires -Version 5.1
<#
.SYNOPSIS
    Runs a command with the MSVC environment loaded (vcvars64).

.DESCRIPTION
    On Windows, rustc needs the Visual C++ toolchain on PATH plus LIB and INCLUDE.
    A normal shell has neither; a "Developer PowerShell" has both. This wrapper
    finds vcvars64.bat through vswhere and runs the given command inside cmd with
    that environment.

    Stays out of the global environment on purpose: nothing is persisted.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File tools\with-msvc.ps1 cargo test --workspace

.NOTES
    Keep the command simple: arguments are joined with spaces.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)]
    [string[]]$Command
)

$ErrorActionPreference = 'Stop'

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) {
    throw 'vswhere.exe not found. Install Visual Studio Build Tools with the C++ workload.'
}

$vcvars = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -find 'VC\Auxiliary\Build\vcvars64.bat' 2>$null |
    Select-Object -First 1

if (-not $vcvars) {
    throw 'vcvars64.bat not found. Install the "Desktop development with C++" workload.'
}

$line = 'call "' + $vcvars + '" >nul && ' + ($Command -join ' ')
cmd /c $line
exit $LASTEXITCODE
