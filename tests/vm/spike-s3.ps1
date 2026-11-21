#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Spike S3: can the visible hostname change without a reboot?

.DESCRIPTION
    Renames the guest, then checks the live hostname, the NetBIOS names and the
    DHCP host-name option. No IP address, MAC address or URL is printed. The
    caller restores the checkpoint afterwards, so the pending rename is thrown
    away.

    Run through tests/vm/run-in-vm.ps1; do not run this on the host.
#>
$ErrorActionPreference = 'Stop'

function Write-Result {
    param([hashtable]$Result)
    $Result | ConvertTo-Json -Compress
}

$before = [Environment]::MachineName
$newName = 'tvpn-s3-test'
$result = @{
    spike = 'S3'
    before = $before
    requested = $newName
    rename_command_ok = $false
    hostname_after = ''
    netbios_after = @()
    dhcp_host_name = ''
    reboot_required = $null
    conclusion = ''
}

try {
    Rename-Computer -NewName $newName -Force -ErrorAction Stop
    $result.rename_command_ok = $true
}
catch {
    $result.conclusion = "RENAME_FAILED: $($_.Exception.Message)"
    Write-Result $result
    exit 1
}

$result.hostname_after = (hostname).Trim()
$result.netbios_after = @(
    nbtstat -n 2>$null |
        Select-String -Pattern '<00>' |
        ForEach-Object { $_.Line.Trim() }
)
$result.dhcp_host_name = (
    ipconfig /all |
        Select-String -Pattern 'Host Name' |
        Select-Object -First 1 |
        ForEach-Object { $_.Line.Trim() }
)

# Renew the lease without a reboot and see which name the client advertises.
ipconfig /release | Out-Null
ipconfig /renew | Out-Null

$result.reboot_required = $result.hostname_after -ne $newName
if ($result.reboot_required) {
    $result.conclusion = 'REBOOT_REQUIRED'
}
else {
    $result.conclusion = 'NO_REBOOT_NEEDED'
}

Write-Result $result
exit 0
