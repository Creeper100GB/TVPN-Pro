# Changelog

## 0.1.6 – Alpha (2026-11-22)

- Fail-closed: WFP-Filter bleiben bei einem Dienststopp oder Absturz erhalten; nur `disable`, `panic`, `recover` oder Uninstall entfernen sie. Ein Neustart geht in `RECOVERY`.
- `%ProgramData%\TVPN` bekommt eine restriktive DACL; Token, Owner-SID und Settings werden atomar mit expliziter DACL geschrieben. Owner-SIDs werden vor der SDDL-Nutzung validiert.
- Tor wird vor dem Start gegen einen gepinnten SHA-256 geprüft. Der Tor-Permit wird erst nach dem VPN-Schritt gesetzt, nicht während des Verbindungsaufbaus.
- Test-Environment-Overrides gelten nur in Debug-Builds. Der unnötige Webview-Befehl `service_install` wurde entfernt.
- Downloader: Staging außerhalb des Download-Ordners, Größenlimit, Redirect-Validierung, reservierte Windows-Namen, Aufräumen partieller Dateien, Abbruch aller Downloads bei Panic/Disable.
- UI: ehrliche Zustände (Tor-only, ungeprüfte VPN-Kette), korrekte Panic-Taste, keine Fake-Angaben zu MAC, Hostname, Portal, Torrents oder Hash-Abfrage. Dienstausfall zeigt keinen falschen READY-Zustand mehr.
- Torrents sind ausdrücklich nicht implementiert: kein Client, keine `.torrent`-Datei, kein Seeding. Magnet-Links werden abgelehnt.
- `install.ps1` verweigert die Installation ohne `SHA256SUMS.txt`.
- VM-Tests: sauberer Dienststopp mit erhaltener Sperre, Uninstall im bewaffneten Zustand, `check.torproject.org` meldet `IsTor=True` (Exit-IP wird nicht ausgegeben).
- Winget-Manifest für `0.1.6` ist mit `winget validate` geprüft; der PR `microsoft/winget-pkgs#486611` wartet auf die Microsoft-CLA und die finale Installation-Validierung. Lokal ist `winget install --manifest packaging\winget\...` sofort nutzbar.

## 0.1.2 – Alpha (2026-11-21)

- Portable Windows-EXE: herunterladen, doppelklicken, starten, ohne Installation.
- Stabile Download-Namen: `TVPN-Pro-Windows-x64-Portable.exe`, `TVPN-Pro-Windows-x64-Setup.exe`, `SHA256SUMS.txt`.
- README mit direkten Download-Links; das PowerShell-Skript ist nur noch eine Alternative.
- Installationsskripte gleichen die von GitHub ersetzten Leerzeichen in Asset-Namen ab und starten den Installer ohne leere Argumentliste.

## 0.1.1 – Alpha (2026-11-21)

- Demo- und Testwerkzeuge entfernt. Stattdessen sitzt ein kleiner Knopf in den Einstellungen, der Tester-Infos für Fehlerberichte kopiert.
- macOS-Build in der CI: ad-hoc signierte, nicht notarisierte `.app` als Artefakt. Ohne Schutzfunktionen, nicht gestartet.
- Installationsskripte: `install.ps1` für Windows, `install.sh` für macOS (wartet auf einen macOS-Release).
- Release veröffentlicht `SHA256SUMS.txt`; das Windows-Skript prüft die Prüfsumme vor dem Start.
- README mit sichtbarer Installationsanleitung.

## 0.1.0 – Alpha (2026-11-21)

Erste öffentliche Alpha. **Kein Schutz.** Firewall, VPN und Tor sind nicht implementiert.

### Enthalten

- Oberfläche mit Svelte 5, SvelteKit 3, Tailwind 4, Tauri 2: elf Zustände, Einstellungen, Sitzungs-Zusammenfassung, Hell/Dunkel, macOS-artiges Fenster.
- Echter Download in der Tauri-Shell: HTTPS über `reqwest`, Routen Direkt und Über Tor (lokaler SOCKS-Proxy 9150), SHA-256, Defender-Scan-Versuch, Abbruch löscht die Datei.
- Download-Liste mit Fortschritt, Route und Urteilen; Torrent-Vertrag nur über VPN, Standard blockiert.
- Service-Kern: Zustandsautomat, Settings-Whitelist, URL-/Magnet-Regeln, IPC-Envelope, Redaktions-Scanner (48 Tests).
- Tests: Playwright-E2E (4), Tauri-Tests (4), PowerShell-Parse-Check, Redaktions-Gate.
- Doku: Plan, Threat Model, IPC-Vertrag, Spikes, Aufgaben, Entscheidungen, Canary-Anleitung, VM-Runner.
- Lizenz: GPL-3.0-or-later.

### Bekannte Grenzen

- Kein Kill-Switch, keine Allowlist, kein VPN, kein eigener Tor-Controller.
- Kein Tor Browser, kein aria2, keine Scanner-Pipeline, kein Wächter.
- Kein Installer mit Signatur, kein USB-Kit, kein macOS.
