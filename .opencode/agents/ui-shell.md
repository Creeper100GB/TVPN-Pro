---
description: UI shell. Tauri app with one big button, status, tray icon, settings and panic key.
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
