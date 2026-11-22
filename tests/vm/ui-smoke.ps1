#requires -Version 5.1
#requires -RunAsAdministrator
<#
.SYNOPSIS
    Launches the real Tauri UI against the real tvpnd in the VM and captures it.

.DESCRIPTION
    The app must show the service state instead of the browser mock. A PNG of
    the app window is written to C:\tvpn-test\ui-smoke.png so the host can copy
    it back for inspection.
#>
$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$serviceExe = Join-Path $root 'tvpnd.exe'
$appExe = Join-Path $root 'tvpn-pro.exe'
$tokenPath = Join-Path $env:ProgramData 'TVPN\session.token'
$shot = Join-Path $root 'ui-smoke.png'
$outLog = Join-Path $root 'ui-service.out.log'
$errLog = Join-Path $root 'ui-service.err.log'

Remove-Item -LiteralPath $tokenPath, $shot, $outLog, $errLog -Force -ErrorAction SilentlyContinue
$service = Start-Process -FilePath $serviceExe -PassThru `
    -RedirectStandardOutput $outLog -RedirectStandardError $errLog -WindowStyle Hidden

try {
    $deadline = (Get-Date).AddSeconds(20)
    while (-not (Test-Path -LiteralPath $tokenPath) -and (Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 200
    }
    if (-not (Test-Path -LiteralPath $tokenPath)) {
        throw 'service token missing'
    }

    $app = Start-Process -FilePath $appExe -PassThru
    $deadline = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $deadline) {
        $app.Refresh()
        if ($app.HasExited) { throw "app exited with $($app.ExitCode)" }
        if ($app.MainWindowHandle -ne 0) { break }
        Start-Sleep -Milliseconds 300
    }
    if ($app.MainWindowHandle -eq 0) { throw 'app window did not appear' }

    Start-Sleep -Seconds 3
    Add-Type -AssemblyName System.Drawing
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public class Win32UiSmoke {
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
'@
    $rect = New-Object Win32UiSmoke+RECT
    [void][Win32UiSmoke]::GetWindowRect($app.MainWindowHandle, [ref]$rect)
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) { throw 'app window has no size' }

    $bitmap = New-Object System.Drawing.Bitmap $width, $height
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    try {
        [void][Win32UiSmoke]::PrintWindow($app.MainWindowHandle, $hdc, 2)
    }
    finally {
        $graphics.ReleaseHdc($hdc)
        $graphics.Dispose()
    }
    $bitmap.Save($shot, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()

    Write-Host ("ui-smoke: ok title={0} size={1}x{2} shot={3}" -f $app.MainWindowTitle, $width, $height, $shot)
    Stop-Process -Id $app.Id -Force -ErrorAction SilentlyContinue
    exit 0
}
finally {
    if ($service -and -not $service.HasExited) {
        Stop-Process -Id $service.Id -Force -ErrorAction SilentlyContinue
        $service.WaitForExit(5000)
    }
}
