---
description: Scanner. Automatic virus and risk check for downloads with quarantine and Mark-of-the-Web.
mode: all
permissions:
  - action: edit
    resource: "*"
    effect: allow
  - action: external_directory
    resource: "*"
    effect: ask
  - action: shell
    resource: "*"
    effect: ask
  - action: shell
    resource: "git status *"
    effect: allow
  - action: shell
    resource: "git diff *"
    effect: allow
  - action: shell
    resource: "git log *"
    effect: allow
  - action: shell
    resource: "git add *"
    effect: allow
  - action: shell
    resource: "git commit *"
    effect: allow
  - action: shell
    resource: "cargo build *"
    effect: allow
  - action: shell
    resource: "cargo test *"
    effect: allow
  - action: shell
    resource: "cargo clippy *"
    effect: allow
  - action: shell
    resource: "cargo fmt *"
    effect: allow
  - action: shell
    resource: "pwsh *run-in-vm.ps1 *"
    effect: allow
  - action: shell
    resource: "powershell *run-in-vm.ps1 *"
    effect: allow
  - action: shell
    resource: "git push *"
    effect: ask
  - action: shell
    resource: "git reset --hard *"
    effect: ask
  - action: shell
    resource: "git push --force *"
    effect: deny
  - action: shell
    resource: "netsh *"
    effect: deny
  - action: shell
    resource: "reg *"
    effect: deny
  - action: shell
    resource: "sc *"
    effect: deny
  - action: shell
    resource: "route *"
    effect: deny
  - action: shell
    resource: "Set-NetAdapter *"
    effect: deny
  - action: shell
    resource: "New-NetFirewallRule *"
    effect: deny
  - action: shell
    resource: "Set-DnsClient *"
    effect: deny
  - action: shell
    resource: "Stop-Service *"
    effect: deny
  - action: shell
    resource: "rm -rf *"
    effect: deny
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
