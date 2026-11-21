---
description: Launcher. Tor Browser, downloader integration, block detection and fallback browser.
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
