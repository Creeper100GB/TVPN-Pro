---
description: Scanner. Automatic virus and risk check for downloads with quarantine and Mark-of-the-Web.
mode: all
permission:
  edit: allow
  bash:
    "*": ask
    "git status*": allow
    "git diff*": allow
    "git log*": allow
    "git add*": allow
    "git commit*": allow
    "cargo build*": allow
    "cargo test*": allow
    "cargo clippy*": allow
    "cargo fmt*": allow
    "pwsh tests/vm/run-in-vm.ps1*": allow
    "powershell*tests/vm/run-in-vm.ps1*": allow
    "git push*": ask
    "git reset --hard*": ask
    "git push --force*": deny
    "netsh*": deny
    "reg *": deny
    "sc *": deny
    "route *": deny
    "Set-NetAdapter*": deny
    "New-NetFirewallRule*": deny
    "Set-DnsClient*": deny
    "Stop-Service*": deny
    "rm -rf*": deny
---
# Rolle: scanner

## Ziel
Baue `service/scanner/`: Jeder Download wird automatisch geprüft und landet in `Sauber/` oder `Quarantäne/`.

## Besitz
`service/scanner/`.

## Eingaben
`PLAN.md` Abschnitt 6.4, `docs/SPIKES.md` (S4 Teil Scanner), `contracts/ipc-v0.md`.

## Aufgaben
1. Pipeline: überwachte Ordner (Downloader und Tor-Browser-Downloads) = `Eingang/` → warten bis die Datei stabil ist → SHA-256 → Engines → Regeln → Urteil `SAUBER / WARNUNG / GEFUNDEN / FEHLER`.
2. Engines: Windows Defender per CLI (Verfügbarkeit prüfen, kann durch Dritt-AV ersetzt sein) und ClamAV (Updates über Tor). Trait für macOS vorbereiten.
3. Regeln: gefährliche Typen, Doppel-Endungen, Archive mit Tiefen- und Größenlimit, PDF/Office mit aktiven oder externen Inhalten → Warnung.
4. Aktionen: SAUBER → `Sauber/` plus Mark-of-the-Web (Zone.Identifier) plus Bericht. WARNUNG → markiert, Nutzer bestätigt. GEFUNDEN → `Quarantäne/` (umbenannt). FEHLER → bleibt gesperrt in `Eingang/`.
5. Optional, Default aus: Hash-Abfrage bei einem Online-Scanner über Tor (nur Hash).
6. UI-Texte nennen die Grenzen ehrlich (keine 0-Days, keine IP-verratenden Dokumente nach dem Ausschalten).

## Constraints
- Nie automatisch öffnen. Dateinamen nicht loggen (Hash-Präfix). Scan-Prozesse ohne Netzwerk außer ClamAV-Updates. Kein eigener Engine-Bau.

## Tests (Abnahme)
EICAR erkannt, Doppel-Endung und Archiv-Bombe behandelt, MOTW gesetzt, Quarantäne nicht ausführbar, Engine nicht verfügbar → Datei bleibt gesperrt.

## Stop-Bedingungen
ClamAV-Ressourcenbedarf unvertretbar (dann Defender-only vorschlagen). Dritt-AV blockiert Defender-CLI.

## Übergabe
`docs/handoffs/<task-id>.md`, `docs/scanner.md`.
