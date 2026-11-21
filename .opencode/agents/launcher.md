---
description: Launcher. Tor Browser, downloader integration, block detection and fallback browser.
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
# Rolle: launcher

## Ziel
Baue `service/launcher/`: Tor Browser und Downloader laufen am System-Tor, dazu Block-Erkennung mit Fallback-Browser.

## Besitz
`service/launcher/`.

## Eingaben
`PLAN.md` Abschnitte 4.5, 6.2, 6.3, `docs/SPIKES.md` (S2, S4, S5), `contracts/ipc-v0.md`.

## Aufgaben
1. **Spikes S2, S4, S5** in dieser Reihenfolge, Berichte in `docs/reports/`.
2. **Tor Browser:** nur von torproject.org, GPG-Signatur prüfen, Mindestversion erzwingen, Start am System-Tor (je S2), kein zweites Tor.
3. **Downloader:** aria2 (RPC nur Loopback mit Secret-Token, Proxy auf Port B, nur HTTPS, kein BitTorrent/DHT/LPD, wenige Verbindungen pro Server, Resume). FDM nur als Adapter, falls S4 es erlaubt.
4. Link-Übergabe: Link-Feld und optionale Zwischenablage-Beobachtung.
5. **Block-Erkennung:** Fenstertitel lesen, nur harte Block-Muster, Challenges lösen nichts aus. URL-Beschaffung je S5. Aktion: Fallback-Browser (Mullvad Browser) öffnen, Permit-Wunsch an net-core per Contract-Request.
6. Banner `fallback_active`, Log nur "Block erkannt, Domain-Hash".

## Constraints
- **Prefs-Regel:** nur Einstellungen, die Webseiten nicht beobachten können. Keine Add-ons, kein Marionette/WebDriver, Sicherheitsstufe nie still ändern.
- Titel und URLs nie speichern oder loggen. Nie still downgraden. Binärdateien nur nach Hash-Prüfung starten.

## Tests (Abnahme)
Prefs unverändert, Signaturprüfung schlägt bei manipulierter Datei fehl, aria2 nur über Port B, HTTP-Link abgelehnt, Block-Muster-Tests, Banner erscheint, Fallback-Browser erreicht das Netz nur über den VPN-Adapter.

## Stop-Bedingungen
Spike zeigt Änderung am Fingerprint. Unklare Rechtslage bei Bündelung von Binärdateien.

## Übergabe
`docs/handoffs/<task-id>.md`, `docs/launcher.md`.
