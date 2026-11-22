#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Runs a PowerShell script inside the Windows test VM and rolls the VM back to a checkpoint.

.DESCRIPTION
    Hyper-V only. The script restores the checkpoint, boots the VM, copies the
    given script (and optional extra files) into the guest, runs it through
    PowerShell Direct and returns the guest exit code. Unless -KeepRunning is
    set, the VM is switched off afterwards. The next run restores the checkpoint
    again, so every test starts from the same state.

    PowerShell Direct talks to the guest over the Hyper-V VMBus (an internal
    channel), not over the network. No host firewall, route, MAC or service is
    touched by this script.

.PARAMETER Script
    Path to the .ps1 file that runs inside the guest.

.PARAMETER ScriptArguments
    Arguments passed to the guest script.

.PARAMETER Include
    Additional files or folders copied next to the script in the guest.
    Leaf names are preserved; nested structures are not.

.PARAMETER VMName
    Hyper-V VM name. Default: tvpn-test.

.PARAMETER CheckpointName
    Checkpoint restored before every run. Default: sauber.

.PARAMETER GuestRoot
    Working folder inside the guest. Default: C:\tvpn-test.

.PARAMETER Credential
    Local administrator account of the guest. Prompted when omitted.

.PARAMETER BootTimeoutSec
    Seconds to wait for heartbeat and PowerShell Direct. Default: 300.

.PARAMETER NoRevert
    Skip the checkpoint restore. Only for debugging a dirty VM.

.PARAMETER KeepRunning
    Leave the VM running after the test instead of switching it off.

.PARAMETER ListCheckpoints
    List checkpoints of the VM and exit.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File tests\vm\run-in-vm.ps1 -Script tests\vm\smoke.ps1

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File tests\vm\run-in-vm.ps1 `
        -Script tests\vm\leak-test.ps1 -ScriptArguments '-Case','ipv6' -Include tests\fixtures
#>
[CmdletBinding(DefaultParameterSetName = 'Run')]
param(
    [Parameter(ParameterSetName = 'Run', Mandatory = $true, Position = 0)]
    [string]$Script,

    [Parameter(ParameterSetName = 'Run')]
    [string[]]$ScriptArguments = @(),

    [Parameter(ParameterSetName = 'Run')]
    [string[]]$Include = @(),

    [string]$VMName = 'tvpn-test',
    [string]$CheckpointName = 'sauber',
    [string]$GuestRoot = 'C:\tvpn-test',
    [System.Management.Automation.PSCredential]$Credential,
    [int]$BootTimeoutSec = 300,
    [switch]$NoRevert,
    [switch]$KeepRunning,

    [Parameter(ParameterSetName = 'List')]
    [switch]$ListCheckpoints
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$outDir = Join-Path $PSScriptRoot 'out'

Import-Module Hyper-V -ErrorAction Stop

if ($VMName -eq $env:COMPUTERNAME) {
    throw "VMName '$VMName' ist der Host. Dieses Skript startet nur Gaeste."
}

$vm = Get-VM -Name $VMName -ErrorAction SilentlyContinue
if (-not $vm) {
    throw "VM '$VMName' nicht gefunden. Ersteinrichtung: siehe tests/vm/README.md"
}

$checkpoints = @(Get-VMSnapshot -VMName $VMName)

if ($ListCheckpoints) {
    if ($checkpoints.Count -eq 0) {
        Write-Host "Keine Checkpoints fuer '$VMName'."
    }
    else {
        $checkpoints | Select-Object Name, SnapshotType, CreationTime | Format-Table -AutoSize
    }
    return
}

if (-not $NoRevert) {
    $checkpoint = $checkpoints | Where-Object { $_.Name -eq $CheckpointName } | Select-Object -First 1
    if (-not $checkpoint) {
        throw "Checkpoint '$CheckpointName' fehlt. Im sauberen VM-Zustand anlegen: Checkpoint-VM -Name '$VMName' -SnapshotName '$CheckpointName'"
    }
    if ($vm.State -ne 'Off') {
        Stop-VM -Name $VMName -TurnOff -Confirm:$false
    }
    Restore-VMSnapshot -VMName $VMName -Name $CheckpointName -Confirm:$false | Out-Null
    Write-Host "Checkpoint '$CheckpointName' wiederhergestellt."
}

$vm = Get-VM -Name $VMName
if ($vm.State -ne 'Running') {
    Start-VM -Name $VMName | Out-Null
}

Write-Host "Warte auf Heartbeat von '$VMName' (max. $BootTimeoutSec s) ..."
Wait-VM -Name $VMName -For Heartbeat -Timeout $BootTimeoutSec

if (-not $Credential) {
    $Credential = Get-Credential -Message "Lokales Administratorkonto in der Test-VM '$VMName'"
}

$session = $null
$deadline = (Get-Date).AddSeconds($BootTimeoutSec)
while (-not $session) {
    try {
        $session = New-PSSession -VMName $VMName -Credential $Credential -ErrorAction Stop
    }
    catch {
        if ((Get-Date) -ge $deadline) {
            throw "PowerShell Direct zu '$VMName' fehlgeschlagen: $($_.Exception.Message). Laeuft ein lokales Administratorkonto und sind die Integrationsdienste aktiv?"
        }
        Start-Sleep -Seconds 5
    }
}

$exitCode = 1

try {
    Invoke-Command -Session $session -ScriptBlock {
        param($Root)
        if (-not (Test-Path -LiteralPath $Root)) {
            New-Item -ItemType Directory -Path $Root -Force | Out-Null
        }
    } -ArgumentList $GuestRoot

    $localScript = (Resolve-Path -LiteralPath $Script).Path
    $remoteScript = Join-Path $GuestRoot (Split-Path -Leaf $localScript)
    Copy-Item -LiteralPath $localScript -Destination $remoteScript -ToSession $session

    foreach ($item in $Include) {
        $resolved = (Resolve-Path -LiteralPath $item).Path
        $destination = Join-Path $GuestRoot (Split-Path -Leaf $resolved)
        Copy-Item -LiteralPath $resolved -Destination $destination -Recurse -ToSession $session
    }

    $argumentsJson = ConvertTo-Json -InputObject @($ScriptArguments) -Compress -Depth 3

    $result = Invoke-Command -Session $session -ScriptBlock {
        param($RemoteScript, $ArgumentsJson)

        $guestArguments = @()
        $parsed = $ArgumentsJson | ConvertFrom-Json
        if ($null -ne $parsed) {
            $guestArguments = @($parsed)
        }

        # Run in a fresh Windows PowerShell process so a broken guest script cannot
        # poison the session, and so $LASTEXITCODE is reliable.
        $text = & powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $RemoteScript @guestArguments 2>&1 | Out-String
        $code = $LASTEXITCODE
        if ($null -eq $code) { $code = 0 }

        [pscustomobject]@{
            Guest    = $env:COMPUTERNAME
            ExitCode = $code
            Output   = $text
        }
    } -ArgumentList $remoteScript, $argumentsJson

    $exitCode = [int]$result.ExitCode

    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
    $stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ')
    $logName = '{0}-{1}.log' -f $stamp, [System.IO.Path]::GetFileNameWithoutExtension($localScript)
    $logPath = Join-Path $outDir $logName
    Set-Content -LiteralPath $logPath -Value $result.Output -Encoding UTF8

    Write-Host ("--- Gast {0}, Exitcode {1} ---" -f $result.Guest, $exitCode)
    Write-Host $result.Output
    Write-Host ("Protokoll: {0}" -f $logPath)
}
finally {
    if ($session) {
        Remove-PSSession -Session $session -ErrorAction SilentlyContinue
    }
    if (-not $KeepRunning) {
        $current = Get-VM -Name $VMName -ErrorAction SilentlyContinue
        if ($current -and $current.State -ne 'Off') {
            Stop-VM -Name $VMName -TurnOff -Confirm:$false -ErrorAction SilentlyContinue
        }
    }
}

exit $exitCode
