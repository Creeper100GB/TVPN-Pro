#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Exercises the real tvpnd pipe, preflight failure and panic path in the VM.

.DESCRIPTION
    Starts tvpnd --console as the local admin. Without tor.exe and Mullvad the
    real backend must refuse enable, report FAILCLOSED and keep the state
    honest. panic must then remove any filters and return to OFF. No filter is
    armed by this script because preflight fails before arming.
#>
$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$exe = Join-Path $root 'tvpnd.exe'
$tokenPath = Join-Path $env:ProgramData 'TVPN\session.token'
$outLog = Join-Path $root 'tvpnd-smoke.out.log'
$errLog = Join-Path $root 'tvpnd-smoke.err.log'
$pipe = $null

Remove-Item -LiteralPath $tokenPath -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $outLog, $errLog -Force -ErrorAction SilentlyContinue

$process = Start-Process -FilePath $exe -PassThru `
    -RedirectStandardOutput $outLog -RedirectStandardError $errLog -WindowStyle Hidden

try {
    $deadline = (Get-Date).AddSeconds(20)
    while (-not (Test-Path -LiteralPath $tokenPath) -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 200
    }
    if (-not (Test-Path -LiteralPath $tokenPath)) {
        throw "token file missing; stdout=$(Get-Content $outLog -Raw -ErrorAction SilentlyContinue) stderr=$(Get-Content $errLog -Raw -ErrorAction SilentlyContinue)"
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

    $before = Send-Request @{ v = 0; id = 's1'; cmd = 'get_status'; args = @{} }
    if ($before.data.state -ne 'OFF') { throw "expected OFF, got $($before.data.state)" }

    $enable = Send-Request @{
        v = 0; id = 'e'; cmd = 'enable'
        args = @{ net_mode = 'focus'; protect_mode = 'tor_vpn' }
    }
    if ($enable.ok) { throw 'enable unexpectedly succeeded without tor/mullvad' }

    $after = Send-Request @{ v = 0; id = 's2'; cmd = 'get_status'; args = @{} }
    if ($after.data.state -ne 'FAILCLOSED') {
        throw "expected FAILCLOSED, got $($after.data.state)"
    }

    $panic = Send-Request @{ v = 0; id = 'p'; cmd = 'panic'; args = @{} }
    if (-not $panic.ok) { throw 'panic was refused' }

    $final = Send-Request @{ v = 0; id = 's3'; cmd = 'get_status'; args = @{} }
    if ($final.data.state -ne 'OFF') {
        throw "expected OFF after panic, got $($final.data.state)"
    }

    Write-Host 'service-smoke: ok'
    exit 0
}
finally {
    if ($pipe) { $pipe.Dispose() }
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        $process.WaitForExit(5000)
    }
}
