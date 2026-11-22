#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Installs the real NSIS bundle, checks the service and Tor resources, removes it.
#>
$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$installer = Get-ChildItem -LiteralPath $root -Filter '*setup.exe' | Select-Object -First 1
if (-not $installer) { throw 'installer missing next to this script' }

Start-Process -FilePath $installer.FullName -ArgumentList '/S' -Wait

$deadline = (Get-Date).AddSeconds(45)
$service = $null
while ((Get-Date) -lt $deadline) {
    $service = Get-Service -Name tvpnd -ErrorAction SilentlyContinue
    if ($service -and $service.Status -eq 'Running') { break }
    Start-Sleep -Milliseconds 500
}
if (-not $service -or $service.Status -ne 'Running') {
    throw 'tvpnd service did not start after the installer'
}

$serviceInfo = Get-CimInstance Win32_Service -Filter "Name='tvpnd'"
$servicePath = $serviceInfo.PathName
Write-Host "service_path=$servicePath"
if ($servicePath -match '^"([^"]+)"') {
    $exePath = $matches[1]
}
else {
    $exePath = ($servicePath -split ' ')[0]
}
$installDir = Split-Path -Parent $exePath
Write-Host "install_dir=$installDir"
Get-ChildItem -LiteralPath $installDir -Recurse -File -ErrorAction SilentlyContinue |
    Select-Object FullName, Length | Format-Table -AutoSize | Out-String | Write-Host

$torCandidates = @(
    (Join-Path $installDir 'tor\tor.exe'),
    (Join-Path $installDir 'resources\tor\tor.exe')
)
$tor = $torCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $tor) { throw 'tor.exe missing next to the service' }
$geoip = Join-Path (Split-Path -Parent $tor) 'geoip'
if (-not (Test-Path -LiteralPath $geoip)) { throw 'geoip missing next to tor.exe' }

$tokenPath = Join-Path $env:ProgramData 'TVPN\session.token'
if (-not (Test-Path -LiteralPath $tokenPath)) { throw 'token missing after install' }
$token = (Get-Content -LiteralPath $tokenPath -Raw).Trim()
$pipe = [System.IO.Pipes.NamedPipeClientStream]::new(
    '.',
    'tvpn-pro-v0',
    [System.IO.Pipes.PipeDirection]::InOut
)
$pipe.Connect(5000)
$writer = [System.IO.StreamWriter]::new($pipe, [System.Text.UTF8Encoding]::new($false))
$writer.AutoFlush = $true
$reader = [System.IO.StreamReader]::new($pipe, [System.Text.Encoding]::UTF8)
$writer.WriteLine((@{
    v = 0; id = 'h'; cmd = 'hello'
    args = @{ token = $token; client = 'test'; client_version = '1' }
} | ConvertTo-Json -Compress))
$hello = $reader.ReadLine() | ConvertFrom-Json
if (-not $hello.ok) { throw 'hello failed after install' }
$writer.WriteLine((@{ v = 0; id = 's'; cmd = 'get_status'; args = @{} } | ConvertTo-Json -Compress))
$status = $reader.ReadLine() | ConvertFrom-Json
if (-not $status.ok) { throw 'status failed after install' }
if ($status.data.state -ne 'OFF') { throw "expected OFF after install, got $($status.data.state)" }

$baseline = Test-NetConnection 1.1.1.1 -Port 443 -InformationLevel Quiet -WarningAction SilentlyContinue
if (-not $baseline) { throw 'no internet before enabling the installed service' }

$writer.WriteLine((@{
    v = 0; id = 'set'; cmd = 'set_settings'
    args = @{ settings = @{ vpn_enabled = $false } }
} | ConvertTo-Json -Compress))
$settings = $reader.ReadLine() | ConvertFrom-Json
if (-not $settings.ok) { throw 'set_settings failed after install' }

$writer.WriteLine((@{
    v = 0; id = 'e'; cmd = 'enable'
    args = @{ net_mode = 'focus'; protect_mode = 'tor_vpn' }
} | ConvertTo-Json -Compress))
$enable = $reader.ReadLine() | ConvertFrom-Json
if (-not $enable.ok) { throw "enable failed after install: $($enable.error.code)" }

$writer.WriteLine((@{ v = 0; id = 's2'; cmd = 'get_status'; args = @{} } | ConvertTo-Json -Compress))
$armed = $reader.ReadLine() | ConvertFrom-Json
if ($armed.data.state -notin @('DEGRADED', 'READY')) {
    throw "unexpected state after installed enable: $($armed.data.state)"
}
if (-not $armed.data.shield.firewall_armed) { throw 'installed service did not arm the firewall' }
$blocked = -not (Test-NetConnection 1.1.1.1 -Port 443 -InformationLevel Quiet -WarningAction SilentlyContinue)
if (-not $blocked) { throw 'installed service did not block ordinary traffic' }

$writer.WriteLine((@{ v = 0; id = 'p'; cmd = 'panic'; args = @{} } | ConvertTo-Json -Compress))
$panic = $reader.ReadLine() | ConvertFrom-Json
if (-not $panic.ok) { throw 'panic failed after install' }
$writer.WriteLine((@{ v = 0; id = 's3'; cmd = 'get_status'; args = @{} } | ConvertTo-Json -Compress))
$final = $reader.ReadLine() | ConvertFrom-Json
if ($final.data.state -ne 'OFF') { throw "expected OFF after panic, got $($final.data.state)" }
$pipe.Dispose()

$reachable = Test-NetConnection 1.1.1.1 -Port 443 -InformationLevel Quiet -WarningAction SilentlyContinue
if (-not $reachable) { throw 'network did not return after panic' }

$uninstaller = Join-Path $installDir 'uninstall.exe'
if (-not (Test-Path -LiteralPath $uninstaller)) { throw 'uninstall.exe missing' }
Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait

$deadline = (Get-Date).AddSeconds(30)
while ((Get-Date) -lt $deadline) {
    if (-not (Get-Service -Name tvpnd -ErrorAction SilentlyContinue)) { break }
    Start-Sleep -Milliseconds 500
}
if (Get-Service -Name tvpnd -ErrorAction SilentlyContinue) {
    throw 'tvpnd service survived the uninstaller'
}

Write-Host ("installer-smoke: ok dir={0} state={1}" -f $installDir, $status.data.state)
exit 0
