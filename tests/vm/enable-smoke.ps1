#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    End-to-end service test with real WFP and a real tor.exe, without Mullvad.

.DESCRIPTION
    Installs the Tor Expert Bundle files next to the service, starts tvpnd,
    disables the VPN in settings, enables protection, checks that ordinary
    traffic is blocked while the filters are armed, then panics and checks that
    the network is reachable again. The VPN->Tor chain is not covered; S1 is
    still blocked on a Mullvad account.
#>
$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$exe = Join-Path $root 'tvpnd.exe'
$torDir = 'C:\Program Files\TVPN\tor'
$tokenPath = Join-Path $env:ProgramData 'TVPN\session.token'
$outLog = Join-Path $root 'enable-smoke.out.log'
$errLog = Join-Path $root 'enable-smoke.err.log'
$pipe = $null

New-Item -ItemType Directory -Force -Path $torDir | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'tor.exe') -Destination (Join-Path $torDir 'tor.exe') -Force
# tvpnd passes --GeoIPFile/--GeoIPv6File from tor.exe's own directory.
Copy-Item -LiteralPath (Join-Path $root 'geoip'), (Join-Path $root 'geoip6') -Destination $torDir -Force

Remove-Item -LiteralPath $tokenPath -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $outLog, $errLog -Force -ErrorAction SilentlyContinue

function Test-Internet {
    return (Test-NetConnection 1.1.1.1 -Port 443 -InformationLevel Quiet -WarningAction SilentlyContinue)
}

$baseline = Test-Internet
if (-not $baseline) { throw 'the VM has no internet before the test' }

$process = Start-Process -FilePath $exe -PassThru `
    -RedirectStandardOutput $outLog -RedirectStandardError $errLog -WindowStyle Hidden

try {
    $deadline = (Get-Date).AddSeconds(20)
    while (-not (Test-Path -LiteralPath $tokenPath) -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 200
    }
    if (-not (Test-Path -LiteralPath $tokenPath)) {
        throw "token file missing; stderr=$(Get-Content $errLog -Raw -ErrorAction SilentlyContinue)"
    }

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

    function Send-Request([hashtable]$body) {
        $writer.WriteLine(($body | ConvertTo-Json -Compress -Depth 5))
        $line = $reader.ReadLine()
        if (-not $line) { throw 'empty response' }
        Write-Host $line
        return $line | ConvertFrom-Json
    }

    $hello = Send-Request @{
        v = 0; id = 'h'; cmd = 'hello'
        args = @{ token = $token; client = 'vm'; client_version = 'test' }
    }
    if (-not $hello.ok) { throw 'hello failed' }

    $settings = Send-Request @{
        v = 0; id = 'set'; cmd = 'set_settings'
        args = @{ settings = @{ vpn_enabled = $false } }
    }
    if (-not $settings.ok) { throw 'set_settings failed' }

    $enable = Send-Request @{
        v = 0; id = 'e'; cmd = 'enable'
        args = @{ net_mode = 'focus'; protect_mode = 'tor_vpn' }
    }
    if (-not $enable.ok) {
        throw "enable failed: $($enable.error.code) / $($enable.error.detail)"
    }

    $status = Send-Request @{ v = 0; id = 's'; cmd = 'get_status'; args = @{} }
    if ($status.data.state -notin @('DEGRADED', 'READY')) {
        throw "unexpected state after enable: $($status.data.state)"
    }
    if (-not $status.data.shield.firewall_armed) {
        throw "expected armed filters, got $($status.data.shield.firewall_filters)"
    }

    $blocked = -not (Test-Internet)
    if (-not $blocked) { throw 'ordinary traffic was not blocked while armed' }

    $panic = Send-Request @{ v = 0; id = 'p'; cmd = 'panic'; args = @{} }
    if (-not $panic.ok) { throw 'panic was refused' }

    $final = Send-Request @{ v = 0; id = 's2'; cmd = 'get_status'; args = @{} }
    if ($final.data.state -ne 'OFF') { throw "expected OFF after panic, got $($final.data.state)" }
    if ($final.data.shield.firewall_armed) { throw 'filters are still armed after panic' }

    $reachable = Test-Internet
    if (-not $reachable) { throw 'the network did not come back after panic' }

    Write-Host ('enable-smoke: ok enable_ok={0} state={1} blocked={2}' -f $enable.ok, $status.data.state, $blocked)
    exit 0
}
finally {
    if ($pipe) { $pipe.Dispose() }
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        $process.WaitForExit(5000)
    }
}
