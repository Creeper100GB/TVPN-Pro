---
description: UI shell. Tauri app with one big button, status, tray icon, settings and panic key.
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
# Rolle: ui-shell

## Ziel
Baue `ui/`: moderne, extrem einfache Oberfläche mit einem großen Button und einem Einstellungsdialog.

## Besitz
`ui/`.

## Eingaben
`PLAN.md` Abschnitt 6.7, `contracts/ipc-v0.md`.

## Aufgaben
1. Button mit Zuständen (Aus, Wird eingeschaltet, An, Fehler), Statuszeile (Zustand, Modus, Land, Wächter), Tray-Icon mit Zustandsfarbe.
2. Einstellungen: VPN an/aus, Land, Bridges, Block-Fallback (aus/Vorschlag/automatisch), MAC "Original zurück", Torrent-Block, Netz-Modus, Notfall-Reset.
3. Immer sichtbare Banner (`portal_open`, `fallback_active`, `degraded`, `failclosed`, `mac_unsupported`, `hostname_unchanged`).
4. Dialog beim Ausschalten mit laufenden Downloads, Session-Zusammenfassung, Panik-Taste.
5. Downloads und Scan-Ergebnisse darstellen, Link-Feld.
6. Ehrliche Texte: keine Versprechen wie "anonym" oder "sicher". Dunkel und Hell.

## Constraints
- Keine Telemetrie, keine externen Ressourcen, keine Secrets in der UI. Keine privilegierten Aktionen, nur feste IPC-Kommandos.
- Neue Pakete nur mit Lizenzcheck (`npm install` wird gefragt).

## Tests (Abnahme)
UI gegen einen IPC-Mock: alle Zustände, Banner und Fehlerfälle werden dargestellt. Keine externen Netzwerkaufrufe.

## Stop-Bedingungen
Ein IPC-Kommando fehlt (Contract-Request schreiben). UI-Wunsch würde eine Sicherheitsregel brechen.

## Übergabe
`docs/handoffs/<task-id>.md`, `docs/ui.md`.
