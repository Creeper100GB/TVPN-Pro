#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Installs tvpnd as a Windows service, talks to it as the owner, removes it.
#>
$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$exe = Join-Path $root 'tvpnd.exe'
$tokenPath = Join-Path $env:ProgramData 'TVPN\session.token'
$pipe = $null

Remove-Item -LiteralPath $tokenPath -Force -ErrorAction SilentlyContinue

& $exe --install
if ($LASTEXITCODE -ne 0) { throw "install failed with $LASTEXITCODE" }

try {
    $deadline = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $deadline) {
        $service = Get-Service -Name tvpnd -ErrorAction SilentlyContinue
        if ($service -and $service.Status -eq 'Running' -and (Test-Path -LiteralPath $tokenPath)) {
            break
        }
        Start-Sleep -Milliseconds 300
    }
    $service = Get-Service -Name tvpnd -ErrorAction Stop
    if ($service.Status -ne 'Running') { throw "service is $($service.Status)" }
    if (-not (Test-Path -LiteralPath $tokenPath)) { throw 'token missing' }

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
    if (-not $hello.ok) { throw 'hello failed' }

    $writer.WriteLine((@{ v = 0; id = 's'; cmd = 'get_status'; args = @{} } | ConvertTo-Json -Compress))
    $status = $reader.ReadLine() | ConvertFrom-Json
    if (-not $status.ok) { throw 'status failed' }
    Write-Host ("service-install: ok state={0}" -f $status.data.state)
    exit 0
}
finally {
    if ($pipe) { $pipe.Dispose() }
    & cmd.exe /c "`"$exe`" --uninstall >nul 2>&1"
}
