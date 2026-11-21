# TVPN Pro installer for Windows.
#
# Downloads the newest release, checks the SHA-256 checksum and starts the NSIS
# installer. The build is unsigned and protects nothing yet.
#
#   irm https://raw.githubusercontent.com/Creeper100GB/TVPN-Pro/main/install.ps1 | iex
#
# Run the file for options: install.ps1 -NoInstall | -Silent

[CmdletBinding()]
param(
    [switch]$NoInstall,
    [switch]$Silent
)

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$repository = 'Creeper100GB/TVPN-Pro'
$releasesUrl = "https://api.github.com/repos/$repository/releases?per_page=10"

function Write-Step([string]$Text) {
    Write-Host "==> $Text" -ForegroundColor Cyan
}

function Get-Release {
    $headers = @{
        'User-Agent' = 'TVPN-Pro-Installer'
        'Accept' = 'application/vnd.github+json'
    }

    try {
        $releases = Invoke-RestMethod -Uri $releasesUrl -Headers $headers
    }
    catch {
        throw "GitHub-Abfrage fehlgeschlagen: $($_.Exception.Message)"
    }

    foreach ($release in $releases) {
        if ($release.draft) { continue }

        $asset = $release.assets | Where-Object { $_.name -match '(?i)x64-setup\.exe$' } | Select-Object -First 1
        if (-not $asset) {
            $asset = $release.assets | Where-Object { $_.name -match '(?i)setup\.exe$' } | Select-Object -First 1
        }
        if (-not $asset) { continue }

        $sums = $release.assets | Where-Object { $_.name -eq 'SHA256SUMS.txt' } | Select-Object -First 1
        return [pscustomobject]@{
            Tag = $release.tag_name
            Asset = $asset
            Sums = $sums
        }
    }

    throw 'Kein Windows-Installer im GitHub-Release gefunden.'
}

Write-Step 'Suche neuesten Release'
$release = Get-Release
Write-Host "Release: $($release.Tag)"

$temp = Join-Path $env:TEMP ("tvpn-pro-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp | Out-Null

try {
    $installer = Join-Path $temp $release.Asset.name

    Write-Step "Lade Installer herunter ($($release.Asset.name))"
    Invoke-WebRequest -Uri $release.Asset.browser_download_url -OutFile $installer -UseBasicParsing

    if ($release.Sums) {
        Write-Step 'Kontrolliere SHA-256'
        $sumFile = Join-Path $temp 'SHA256SUMS.txt'
        Invoke-WebRequest -Uri $release.Sums.browser_download_url -OutFile $sumFile -UseBasicParsing

        # GitHub replaces spaces in uploaded asset names with dots. Compare the
        # names without spaces and dots so the local and uploaded names match.
        $wanted = $release.Asset.name -replace '[ .]', ''
        $line = Get-Content -LiteralPath $sumFile |
            Where-Object {
                $parts = $_.Trim() -split '\s+', 2
                $parts.Count -eq 2 -and (($parts[1] -replace '[ .]', '') -eq $wanted)
            } |
            Select-Object -First 1
        if (-not $line) {
            throw "Keine SHA-256-Zeile zu $($release.Asset.name) gefunden."
        }

        $expected = ($line.Trim() -split '\s+', 2)[0].ToLowerInvariant()
        $actual = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -ne $expected) {
            throw "SHA-256 stimmt nicht. Erwartet: $expected. Erhalten: $actual."
        }
        Write-Host "SHA-256: $actual"
    }
    else {
        Write-Warning 'Keine SHA256SUMS.txt im Release, daher keine SHA-256-Kontrolle.'
    }

    if ($NoInstall) {
        Write-Step "Nur geladen und kontrolliert, nicht installiert: $installer"
        return
    }

    Write-Step 'Starte den Installer. Windows fragt nach der Zustimmung.'
    $arguments = @()
    if ($Silent) { $arguments += '/S' }

    $process = Start-Process -FilePath $installer -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "Der Installer endete mit Code $($process.ExitCode)."
    }

    Write-Step 'Fertig. TVPN Pro ist installiert.'
}
finally {
    if (-not $NoInstall) {
        Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
    }
}
