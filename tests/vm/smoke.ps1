#requires -Version 5.1
<#
.SYNOPSIS
    Minimal smoke test for run-in-vm.ps1.

.DESCRIPTION
    Prints guest identity and exits 0. Touches no network, service or adapter.
    Use it to prove that the runner can reach a guest before trusting a real test.
#>
[CmdletBinding()]
param(
    [string]$Tag = 'smoke'
)

$ErrorActionPreference = 'Stop'

$os = Get-CimInstance -ClassName Win32_OperatingSystem
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$isAdmin = ([Security.Principal.WindowsPrincipal]$identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

Write-Host ('tag={0} guest={1} os={2} build={3} ps={4} admin={5}' -f `
    $Tag, $env:COMPUTERNAME, $os.Caption, $os.BuildNumber, $PSVersionTable.PSVersion, $isAdmin)

exit 0
