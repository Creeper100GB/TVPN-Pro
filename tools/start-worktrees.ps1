<#
.SYNOPSIS
  Legt git-Worktrees fuer Rollen an und oeffnet pro Rolle einen Windows-Terminal-Tab mit opencode.
.EXAMPLE
  # Im Repo-Hauptordner ausfuehren:
  powershell -ExecutionPolicy Bypass -File tools\start-worktrees.ps1 -Roles net-core,ui-shell
  # Nur anlegen, nichts starten:
  powershell -ExecutionPolicy Bypass -File tools\start-worktrees.ps1 -Roles net-core -NoLaunch
.NOTES
  Lernmoment: Ein Worktree ist ein zweiter Arbeitsordner desselben Repos auf eigenem Branch.
  `git worktree list` zeigt alle. Dieses Skript legt nur an und loescht nichts.
  Kennt dein opencode kein --agent, nimm -NoAgentFlag und waehle die Rolle im Tab per Tab-Taste.
#>
param(
  [Parameter(Mandatory = $true)][string[]]$Roles,
  [string]$WorktreeRoot = (Join-Path (Split-Path -Parent (Get-Location).Path) 'tvpn-wt'),
  [switch]$NoLaunch,
  [switch]$NoAgentFlag
)

$ErrorActionPreference = 'Continue'

# powershell -File passes "-Roles a,b" as one string; split it here.
$Roles = @($Roles | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$valid = @('net-core', 'privacy-shield', 'launcher', 'scanner', 'watchdog', 'ui-shell', 'qa-security')

if ($Roles.Count -gt 4) { throw 'Maximal 4 Rollen gleichzeitig (PLAN.md Abschnitt 10).' }
foreach ($r in $Roles) {
  if ($valid -notcontains $r) { throw "Unbekannte Rolle: $r. Erlaubt: $($valid -join ', ')" }
}

git rev-parse --is-inside-work-tree *> $null
if ($LASTEXITCODE -ne 0) { throw 'Kein git-Repo. Erst: git init' }
git rev-parse --verify HEAD *> $null
if ($LASTEXITCODE -ne 0) { throw 'Noch kein Commit. Erst: git add -A ; git commit -m "chore: kit"' }

New-Item -ItemType Directory -Force -Path $WorktreeRoot | Out-Null
$hasWt = [bool](Get-Command wt -ErrorAction SilentlyContinue)

foreach ($role in $Roles) {
  $dir = Join-Path $WorktreeRoot $role
  $branch = "agent/$role"

  if (-not (Test-Path $dir)) {
    git show-ref --verify --quiet "refs/heads/$branch"
    if ($LASTEXITCODE -eq 0) { git worktree add $dir $branch } else { git worktree add $dir -b $branch }
    if ($LASTEXITCODE -ne 0) { throw "git worktree add fehlgeschlagen fuer $role" }
  } else {
    Write-Host "Worktree existiert schon: $dir"
  }

  if ($NoLaunch) { continue }

  $cmd = if ($NoAgentFlag) { 'opencode' } else { "opencode --agent $role" }
  if ($hasWt) {
    Start-Process wt -ArgumentList @('-w', '0', 'new-tab', '--title', $role, '-d', "`"$dir`"", 'powershell', '-NoExit', '-Command', $cmd)
  } else {
    Write-Host "Windows Terminal (wt) nicht gefunden. Oeffne selbst ein Terminal:  cd `"$dir`" ; $cmd"
  }
}

git worktree list
