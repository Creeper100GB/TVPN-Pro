#requires -Version 5.1
<#
.SYNOPSIS
    Captures one application window to a PNG, without the rest of the desktop.

.DESCRIPTION
    Finds the process by exact window title, brings it to the foreground, takes
    its rectangle and copies only that region. Used to look at the Tauri shell
    instead of guessing from the code.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File tools\capture-window.ps1 -Title 'TVPN Pro' -Out ui\.artifacts\tauri.png
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Title,

    [Parameter(Mandatory = $true)]
    [string]$Out
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;

public struct TVPN_RECT
{
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
}

public static class TVPN_Win32
{
    [DllImport("user32.dll")]
    public static extern bool SetProcessDPIAware();

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr handle, out TVPN_RECT rect);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr handle);

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr handle, int command);

    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(IntPtr handle, IntPtr insertAfter, int x, int y, int width, int height, uint flags);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr handle);
}
'@

[void][TVPN_Win32]::SetProcessDPIAware()

$process = Get-Process |
    Where-Object { $_.MainWindowTitle -eq $Title -and $_.MainWindowHandle -ne 0 } |
    Select-Object -First 1

if (-not $process) {
    throw "No window with title '$Title' found."
}

Write-Host ("visible before: {0}" -f [TVPN_Win32]::IsWindowVisible($process.MainWindowHandle))

[void][TVPN_Win32]::ShowWindow($process.MainWindowHandle, 9) # SW_RESTORE
[void][TVPN_Win32]::SetForegroundWindow($process.MainWindowHandle)
# Put it above the terminal even when this shell cannot take focus.
[void][TVPN_Win32]::SetWindowPos($process.MainWindowHandle, [IntPtr](-1), 0, 0, 0, 0, 0x0001 -bor 0x0002)
Start-Sleep -Milliseconds 700
[void][TVPN_Win32]::SetWindowPos($process.MainWindowHandle, [IntPtr](-2), 0, 0, 0, 0, 0x0001 -bor 0x0002)
Start-Sleep -Milliseconds 200

$rect = New-Object TVPN_RECT
if (-not [TVPN_Win32]::GetWindowRect($process.MainWindowHandle, [ref]$rect)) {
    throw 'GetWindowRect failed.'
}

$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
if ($width -le 0 -or $height -le 0) {
    throw "Window has no size ($width x $height)."
}

$bitmap = New-Object System.Drawing.Bitmap($width, $height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
    $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
    $target = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Out))
    $folder = [System.IO.Path]::GetDirectoryName($target)
    if (-not (Test-Path -LiteralPath $folder)) {
        New-Item -ItemType Directory -Path $folder -Force | Out-Null
    }
    $bitmap.Save($target, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Host ("captured {0}x{1} to {2}" -f $width, $height, $target)
}
finally {
    $graphics.Dispose()
    $bitmap.Dispose()
}
