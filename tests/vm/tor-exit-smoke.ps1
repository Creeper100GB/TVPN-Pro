#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Enables the installed service in Tor-only mode and asks the Tor Project's
    check API whether the connection is really using Tor.
#>
$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$installer = Get-ChildItem -LiteralPath $root -Filter '*setup.exe' | Select-Object -First 1
$newService = Join-Path $root 'tvpnd.exe'
if (-not $installer) { throw 'installer missing next to this script' }
if (-not (Test-Path -LiteralPath $newService)) { throw 'new tvpnd.exe missing next to this script' }

Start-Process -FilePath $installer.FullName -ArgumentList '/S' -Wait

$deadline = (Get-Date).AddSeconds(45)
while ((Get-Date) -lt $deadline) {
    $service = Get-Service -Name tvpnd -ErrorAction SilentlyContinue
    if ($service -and $service.Status -eq 'Running') { break }
    Start-Sleep -Milliseconds 500
}
if (-not $service -or $service.Status -ne 'Running') { throw 'service did not start' }

$serviceInfo = Get-CimInstance Win32_Service -Filter "Name='tvpnd'"
$servicePath = $serviceInfo.PathName
if ($servicePath -match '^"([^"]+)"') { $exePath = $matches[1] } else { $exePath = ($servicePath -split ' ')[0] }

Stop-Service -Name tvpnd -Force
$deadline = (Get-Date).AddSeconds(20)
while ((Get-Date) -lt $deadline) {
    if ((Get-Service -Name tvpnd).Status -eq 'Stopped') { break }
    Start-Sleep -Milliseconds 200
}
Copy-Item -LiteralPath $newService -Destination $exePath -Force
Start-Service -Name tvpnd

$tokenPath = Join-Path $env:ProgramData 'TVPN\session.token'
$deadline = (Get-Date).AddSeconds(30)
while ((Get-Date) -lt $deadline -and -not (Test-Path -LiteralPath $tokenPath)) {
    Start-Sleep -Milliseconds 300
}
if (-not (Test-Path -LiteralPath $tokenPath)) { throw 'token missing' }

$token = (Get-Content -LiteralPath $tokenPath -Raw).Trim()
$pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', 'tvpn-pro-v0', [System.IO.Pipes.PipeDirection]::InOut)
$pipe.Connect(5000)
$writer = [System.IO.StreamWriter]::new($pipe, [System.Text.UTF8Encoding]::new($false))
$writer.AutoFlush = $true
$reader = [System.IO.StreamReader]::new($pipe, [System.Text.Encoding]::UTF8)

function Send($id, $cmd, $payload) {
    $writer.WriteLine((@{
        v = 0; id = $id; cmd = $cmd; args = $payload
    } | ConvertTo-Json -Compress))
    return $reader.ReadLine() | ConvertFrom-Json
}

$hello = Send 'h' 'hello' @{ token = $token; client = 'test'; client_version = '1' }
if (-not $hello.ok) { throw 'hello failed' }
$set = Send 'set' 'set_settings' @{ settings = @{ vpn_enabled = $false } }
if (-not $set.ok) { throw 'set_settings failed' }
$enable = Send 'e' 'enable' @{ net_mode = 'focus'; protect_mode = 'tor_vpn' }
if (-not $enable.ok) { throw "enable failed: $($enable.error.code)" }
$status = Send 's' 'get_status' @{}
if ($status.data.state -notin @('DEGRADED', 'READY')) { throw "unexpected state $($status.data.state)" }
if (-not $status.data.tor.circuit) { throw 'no Tor circuit' }

$api = & curl.exe -sS --fail --ssl-no-revoke --max-time 60 --socks5-hostname 127.0.0.1:9250 https://check.torproject.org/api/ip
if ($LASTEXITCODE -ne 0) { throw "Tor check request failed with $LASTEXITCODE" }
$result = $api | ConvertFrom-Json
if ($result.IsTor -ne $true) {
    throw 'check.torproject.org reports that the connection is not using Tor'
}
# Never print the exit IP; only the boolean evidence.
Write-Host 'tor-exit-smoke: IsTor=True (exit IP hidden)'

$panic = Send 'p' 'panic' @{}
if (-not $panic.ok) { throw "panic failed: $($panic.error.code)" }
$pipe.Dispose()
& $exePath --uninstall
if ($LASTEXITCODE -ne 0) { throw "uninstall failed with $LASTEXITCODE" }
Write-Host 'tor-exit-smoke: ok'
exit 0
