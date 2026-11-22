#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Arms the service, stops it cleanly, and verifies the filters stay armed.
    A restart must enter RECOVERY; recover must remove only our filters.
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

function Connect-Pipe {
    $token = (Get-Content -LiteralPath $tokenPath -Raw).Trim()
    $pipe = [System.IO.Pipes.NamedPipeClientStream]::new('.', 'tvpn-pro-v0', [System.IO.Pipes.PipeDirection]::InOut)
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
    return [pscustomobject]@{ Pipe = $pipe; Writer = $writer; Reader = $reader }
}

function Send($connection, $id, $cmd, $payload) {
    $connection.Writer.WriteLine((@{
        v = 0; id = $id; cmd = $cmd; args = $payload
    } | ConvertTo-Json -Compress))
    return $connection.Reader.ReadLine() | ConvertFrom-Json
}

$c = Connect-Pipe
$set = Send $c 'set' 'set_settings' @{ settings = @{ vpn_enabled = $false } }
if (-not $set.ok) { throw 'set_settings failed' }
$enable = Send $c 'e' 'enable' @{ net_mode = 'focus'; protect_mode = 'tor_vpn' }
if (-not $enable.ok) { throw "enable failed: $($enable.error.code)" }
$armed = Send $c 's' 'get_status' @{}
if ($armed.data.state -notin @('DEGRADED', 'READY')) { throw "unexpected state $($armed.data.state)" }
$blocked = -not (Test-NetConnection 1.1.1.1 -Port 443 -InformationLevel Quiet -WarningAction SilentlyContinue)
if (-not $blocked) { throw 'ordinary traffic was not blocked' }
$c.Pipe.Dispose()
$oldToken = (Get-Content -LiteralPath $tokenPath -Raw).Trim()

# Clean stop, no panic, no uninstall. Filters must stay.
Stop-Service -Name tvpnd -Force
$deadline = (Get-Date).AddSeconds(20)
while ((Get-Date) -lt $deadline) {
    if ((Get-Service -Name tvpnd).Status -eq 'Stopped') { break }
    Start-Sleep -Milliseconds 200
}
$stillBlocked = -not (Test-NetConnection 1.1.1.1 -Port 443 -InformationLevel Quiet -WarningAction SilentlyContinue)
if (-not $stillBlocked) { throw 'filters disappeared on a clean service stop' }

Start-Service -Name tvpnd
$deadline = (Get-Date).AddSeconds(30)
while ((Get-Date) -lt $deadline) {
    if ((Get-Service -Name tvpnd).Status -eq 'Running') { break }
    Start-Sleep -Milliseconds 300
}
$deadline = (Get-Date).AddSeconds(30)
while ((Get-Date) -lt $deadline) {
    $current = (Get-Content -LiteralPath $tokenPath -Raw -ErrorAction SilentlyContinue).Trim()
    if ($current -and $current -ne $oldToken) { break }
    Start-Sleep -Milliseconds 300
}
$c = Connect-Pipe
$status = Send $c 's' 'get_status' @{}
if ($status.data.state -ne 'RECOVERY') { throw "expected RECOVERY after clean stop, got $($status.data.state)" }
$recover = Send $c 'r' 'recover' @{}
if (-not $recover.ok) { throw "recover failed: $($recover.error.code)" }
$final = Send $c 'f' 'get_status' @{}
if ($final.data.state -ne 'OFF') { throw "expected OFF after recover, got $($final.data.state)" }
$c.Pipe.Dispose()

$reachable = Test-NetConnection 1.1.1.1 -Port 443 -InformationLevel Quiet -WarningAction SilentlyContinue
if (-not $reachable) { throw 'network did not return after recover' }

& $exePath --uninstall
if ($LASTEXITCODE -ne 0) { throw "uninstall failed with $LASTEXITCODE" }
Write-Host 'clean-stop-recovery-smoke: ok'
exit 0
