<#
.SYNOPSIS
  Entfernt Worktrees (nicht die Branches). Verweigert, wenn ungesicherte Aenderungen da sind.
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File tools\cleanup-worktrees.ps1 -Roles net-core,ui-shell
.NOTES
  Branches bleiben erhalten. Loeschen erst nach dem Merge: git branch -d agent/<rolle>
#>
param(
  [Parameter(Mandatory = $true)][string[]]$Roles,
  [string]$WorktreeRoot = (Join-Path (Split-Path -Parent (Get-Location).Path) 'tvpn-wt'),
  [switch]$Force
)

$ErrorActionPreference = 'Continue'

foreach ($role in $Roles) {
  $dir = Join-Path $WorktreeRoot $role
  if (-not (Test-Path $dir)) { Write-Host "Nicht vorhanden: $dir"; continue }

  $dirty = git -C $dir status --porcelain
  if ($dirty -and -not $Force) {
    Write-Warning "Ungesicherte Aenderungen in $role. Erst committen und mergen oder -Force (verwirft sie)."
    continue
  }
  if ($Force) { git worktree remove --force $dir } else { git worktree remove $dir }
}

git worktree prune
git worktree list
Write-Host 'Branches bleiben erhalten. Loeschen nur nach dem Merge: git branch -d agent/<rolle>'
