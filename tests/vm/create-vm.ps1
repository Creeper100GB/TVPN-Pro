#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Creates the Windows 11 test VM from an ISO with an unattended installation.

.DESCRIPTION
    Builds a Generation 2 Hyper-V VM named tvpn-test, installs Windows 11 Pro
    from the given ISO with a local administrator (default tvpn), waits for
    PowerShell Direct, creates the checkpoint sauber and runs the smoke test.

    The answer file is placed on a small FAT32 VHD that is attached as a second
    disk. Windows Setup reads autounattend.xml from it. No host disk is touched:
    the script refuses disks that are not file-backed virtual disks.

.PARAMETER IsoPath
    Path to the Windows 11 x64 ISO.

.PARAMETER Recreate
    Remove an existing tvpn-test VM, its OS VHD and its answer ISO first. Only
    the VM and the files under VMPath with the VM name are removed.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$IsoPath,

    [string]$VMName = 'tvpn-test',
    [string]$VMPath = 'C:\Users\Jonah\VMs',
    [int]$MemoryGB = 8,
    [int]$CPUCount = 4,
    [string]$SwitchName = 'Default Switch',
    [string]$AdminUser = 'tvpn',
    [string]$AdminPassword = 'TVPN-Test-2026!',
    [string]$Oscdimg = 'C:\Program Files (x86)\Windows Kits\10\Assessment and Deployment Kit\Deployment Tools\amd64\Oscdimg\oscdimg.exe',
    [string]$CheckpointName = 'sauber',
    [int]$TimeoutMinutes = 60,
    [switch]$Recreate
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $IsoPath)) {
    throw "ISO not found: $IsoPath"
}

if (-not (Get-VMSwitch -Name $SwitchName -ErrorAction SilentlyContinue)) {
    throw "Hyper-V switch not found: $SwitchName"
}

if (-not (Test-Path -LiteralPath $Oscdimg)) {
    throw "oscdimg not found: $Oscdimg"
}

$existing = Get-VM -Name $VMName -ErrorAction SilentlyContinue
if ($existing) {
    if (-not $Recreate) {
        throw "VM $VMName already exists. Use -Recreate only if you want to delete it."
    }
    Stop-VM -Name $VMName -TurnOff -Force -ErrorAction SilentlyContinue
    Remove-VM -Name $VMName -Force
    Remove-Item -LiteralPath (Join-Path $VMPath "$VMName.vhdx") -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $VMPath "$VMName-answer.vhdx") -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $VMPath "$VMName-answer.iso") -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $VMPath "$VMName-answer") -Recurse -Force -ErrorAction SilentlyContinue
    Get-ChildItem -LiteralPath $VMPath -Filter "$VMName*.avhdx" -File -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
}

New-Item -ItemType Directory -Path $VMPath -Force | Out-Null
$answerDir = Join-Path $VMPath "$VMName-answer"
$answerIso = Join-Path $VMPath "$VMName-answer.iso"
$osVhd = Join-Path $VMPath "$VMName.vhdx"

$mounted = $null
try {
    Write-Host "==> Mount ISO"
    $mounted = Mount-DiskImage -ImagePath $IsoPath -PassThru
    $volume = Get-Volume -DiskImage $mounted
    $drive = "$($volume.DriveLetter):"
    if (-not $volume.DriveLetter) {
        throw 'ISO has no drive letter.'
    }

    $wim = Join-Path $drive 'sources\install.wim'
    if (-not (Test-Path -LiteralPath $wim)) {
        $wim = Join-Path $drive 'sources\install.esd'
    }
    if (-not (Test-Path -LiteralPath $wim)) {
        throw 'install.wim or install.esd not found on the ISO.'
    }

    # The setup inside the guest selects the edition by name. The host DISM
    # may be older than the ISO and must not be used to inspect the WIM.
    Write-Host "==> Edition: Windows 11 Pro (selected by Setup)"

    # Release the host mount before Hyper-V opens the ISO as a DVD.
    Dismount-DiskImage -ImagePath $IsoPath | Out-Null
    $mounted = $null

    Write-Host "==> Build answer ISO"
    $template = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'autounattend.xml') -Raw
    $xml = $template.Replace('__PASSWORD__', $AdminPassword)

    New-Item -ItemType Directory -Path $answerDir -Force | Out-Null
    $answerXml = Join-Path $answerDir 'autounattend.xml'
    Set-Content -LiteralPath $answerXml -Value $xml -Encoding UTF8
    if (Test-Path -LiteralPath $answerIso) {
        Remove-Item -LiteralPath $answerIso -Force
    }
    & $Oscdimg -m -o -u2 -lANSWER $answerDir $answerIso | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "oscdimg failed with exit code $LASTEXITCODE"
    }

    Write-Host "==> Create VM"
    New-VHD -Path $osVhd -SizeBytes 80GB -Dynamic | Out-Null
    New-VM -Name $VMName -Generation 2 -MemoryStartupBytes ($MemoryGB * 1GB) -VHDPath $osVhd -SwitchName $SwitchName | Out-Null
    Set-VMProcessor -VMName $VMName -Count $CPUCount
    Set-VMMemory -VMName $VMName -DynamicMemoryEnabled $false
    Set-VMFirmware -VMName $VMName -EnableSecureBoot On -SecureBootTemplate MicrosoftWindows
    Set-VMKeyProtector -VMName $VMName -NewLocalKeyProtector
    Enable-VMTPM -VMName $VMName
    Add-VMDvdDrive -VMName $VMName -Path $IsoPath | Out-Null
    Add-VMDvdDrive -VMName $VMName -Path $answerIso | Out-Null
    $dvd = Get-VMDvdDrive -VMName $VMName | Where-Object { $_.Path -eq $IsoPath }
    $osDisk = Get-VMHardDiskDrive -VMName $VMName | Where-Object { $_.Path -eq $osVhd }
    Set-VMFirmware -VMName $VMName -BootOrder $dvd, $osDisk

    Write-Host "==> Start VM and wait for unattended install"
    Start-VM -Name $VMName
    Start-Sleep -Seconds 1

    # Windows Setup waits for a key on "Press any key to boot from CD or DVD".
    $vmWmi = Get-CimInstance -Namespace root\virtualization\v2 -ClassName Msvm_ComputerSystem -Filter "ElementName='$VMName'"
    $keyboard = $null
    for ($attempt = 0; $attempt -lt 10 -and -not $keyboard; $attempt++) {
        $keyboard = Get-CimInstance -Namespace root\virtualization\v2 -ClassName Msvm_Keyboard |
            Where-Object { $_.SystemName -eq $vmWmi.Name } |
            Select-Object -First 1
        if (-not $keyboard) { Start-Sleep -Milliseconds 500 }
    }
    if ($keyboard) {
        for ($press = 0; $press -lt 30; $press++) {
            Invoke-CimMethod -InputObject $keyboard -MethodName TypeKey -Arguments @{ keyCode = [byte]0x20 } | Out-Null
            Start-Sleep -Milliseconds 300
        }
    }

    $secure = ConvertTo-SecureString $AdminPassword -AsPlainText -Force
    $credential = New-Object System.Management.Automation.PSCredential($AdminUser, $secure)
    $deadline = (Get-Date).AddMinutes($TimeoutMinutes)
    $ready = $false
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds 20
        try {
            $hostname = Invoke-Command -VMName $VMName -Credential $credential -ScriptBlock { hostname } -ErrorAction Stop
            if ($hostname) {
                $ready = $true
                break
            }
        }
        catch {
            # Guest not ready yet.
        }
    }
    if (-not $ready) {
        throw "VM did not become reachable within $TimeoutMinutes minutes. Check: vmconnect.exe localhost `"$VMName`""
    }

    Write-Host "==> Prepare guest"
    Invoke-Command -VMName $VMName -Credential $credential -ScriptBlock {
        powercfg /change standby-timeout-ac 0
        powercfg /change hibernate-timeout-ac 0
        powercfg /change monitor-timeout-ac 10
        Enable-PSRemoting -Force -SkipNetworkProfileCheck
        Get-NetConnectionProfile | Set-NetConnectionProfile -NetworkCategory Private -ErrorAction SilentlyContinue
    } | Out-Null

    Write-Host "==> Create checkpoint $CheckpointName"
    Checkpoint-VM -Name $VMName -SnapshotName $CheckpointName | Out-Null

    Write-Host "==> Run smoke test"
    & (Join-Path $PSScriptRoot 'run-in-vm.ps1') -Script (Join-Path $PSScriptRoot 'smoke.ps1') -Credential $credential

    Write-Host '==> Done'
}
finally {
    if ($mounted) {
        Dismount-DiskImage -ImagePath $IsoPath -ErrorAction SilentlyContinue | Out-Null
    }
}
