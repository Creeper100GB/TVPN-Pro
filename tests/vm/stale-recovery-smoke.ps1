#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Kills the armed service, verifies the filters persist, and recovers.

.DESCRIPTION
    WFP uses a static session. A hard kill therefore leaves the filters in BFE.
    The next service start must notice them, enter RECOVERY and refuse enable
    until recover removes only our keys. Tor is used only to reach an armed
    state; the VPN is disabled for this test.
#>
$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$exe = Join-Path $root 'tvpnd.exe'
$torDir = 'C:\Program Files\TVPN\tor'
$dataDir = Join-Path $env:ProgramData 'TVPN\tor'
$tokenPath = Join-Path $env:ProgramData 'TVPN\session.token'
$pipe = $null
$service = $null

New-Item -ItemType Directory -Force -Path $torDir, $dataDir | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'tor.exe') -Destination (Join-Path $torDir 'tor.exe') -Force
Copy-Item -LiteralPath (Join-Path $root 'geoip'), (Join-Path $root 'geoip6') -Destination $dataDir -Force

function Test-Internet {
    return (Test-NetConnection 1.1.1.1 -Port 443 -InformationLevel Quiet -WarningAction SilentlyContinue)
}

function Connect-Pipe([string]$token) {
    $stream = [System.IO.Pipes.NamedPipeClientStream]::new(
        '.', 'tvpn-pro-v0', [System.IO.Pipes.PipeDirection]::InOut
    )
    $stream.Connect(5000)
    $writer = [System.IO.StreamWriter]::new($stream, [System.Text.UTF8Encoding]::new($false))
    $writer.AutoFlush = $true
    $reader = [System.IO.StreamReader]::new($stream, [System.Text.Encoding]::UTF8)
    $writer.WriteLine((@{
        v = 0; id = 'h'; cmd = 'hello'
        args = @{ token = $token; client = 'test'; client_version = '1' }
    } | ConvertTo-Json -Compress))
    $hello = $reader.ReadLine() | ConvertFrom-Json
    if (-not $hello.ok) { throw 'hello failed' }
    return @{ Stream = $stream; Writer = $writer; Reader = $reader }
}

function Send-Pipe($session, [hashtable]$body) {
    $session.Writer.WriteLine(($body | ConvertTo-Json -Compress -Depth 5))
    return ($session.Reader.ReadLine() | ConvertFrom-Json)
}

function Start-Tvpn {
    Remove-Item -LiteralPath $tokenPath -Force -ErrorAction SilentlyContinue
    $process = Start-Process -FilePath $exe -PassThru -WindowStyle Hidden
    $deadline = (Get-Date).AddSeconds(20)
    while (-not (Test-Path -LiteralPath $tokenPath) -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 200
    }
    if (-not (Test-Path -LiteralPath $tokenPath)) { throw 'token missing' }
    return $process
}

try {
    $service = Start-Tvpn
    $token = (Get-Content -LiteralPath $tokenPath -Raw).Trim()
    $pipe = Connect-Pipe $token
    $set = Send-Pipe $pipe @{
        v = 0; id = 'set'; cmd = 'set_settings'
        args = @{ settings = @{ vpn_enabled = $false } }
    }
    if (-not $set.ok) { throw 'set_settings failed' }
    $enable = Send-Pipe $pipe @{
        v = 0; id = 'e'; cmd = 'enable'
        args = @{ net_mode = 'focus'; protect_mode = 'tor_vpn' }
    }
    if (-not $enable.ok) { throw "enable failed: $($enable.error.code)" }
    $armed = Send-Pipe $pipe @{ v = 0; id = 's'; cmd = 'get_status'; args = @{} }
    if (-not $armed.data.shield.firewall_armed) { throw 'not armed before kill' }
    $pipe.Stream.Dispose()
    $pipe = $null

    Stop-Process -Id $service.Id -Force
    $service = $null
    Start-Sleep -Seconds 2
    Get-Process tor -ErrorAction SilentlyContinue | Stop-Process -Force

    $stillBlocked = -not (Test-Internet)
    if (-not $stillBlocked) { throw 'filters vanished after the hard kill' }

    $service = Start-Tvpn
    $token = (Get-Content -LiteralPath $tokenPath -Raw).Trim()
    $pipe = Connect-Pipe $token
    $stale = Send-Pipe $pipe @{ v = 0; id = 's2'; cmd = 'get_status'; args = @{} }
    if ($stale.data.state -ne 'RECOVERY') {
        throw "expected RECOVERY after kill, got $($stale.data.state)"
    }
    if (-not $stale.data.shield.firewall_armed) { throw 'stale filters not reported' }
    $refused = Send-Pipe $pipe @{
        v = 0; id = 'e2'; cmd = 'enable'
        args = @{ net_mode = 'focus'; protect_mode = 'tor_vpn' }
    }
    if ($refused.ok) { throw 'enable was allowed in RECOVERY' }

    $recover = Send-Pipe $pipe @{ v = 0; id = 'r'; cmd = 'recover'; args = @{} }
    if (-not $recover.ok) { throw 'recover failed' }
    $final = Send-Pipe $pipe @{ v = 0; id = 's3'; cmd = 'get_status'; args = @{} }
    if ($final.data.state -ne 'OFF') { throw "expected OFF after recover, got $($final.data.state)" }
    if ($final.data.shield.firewall_armed) { throw 'filters still armed after recover' }
    $reachable = Test-Internet
    if (-not $reachable) { throw 'network did not return after recover' }

    Write-Host 'stale-recovery-smoke: ok'
    exit 0
}
finally {
    if ($pipe) { $pipe.Stream.Dispose() }
    if ($service -and -not $service.HasExited) {
        Stop-Process -Id $service.Id -Force -ErrorAction SilentlyContinue
    }
    Get-Process tor -ErrorAction SilentlyContinue | Stop-Process -Force
}
