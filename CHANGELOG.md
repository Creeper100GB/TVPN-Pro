# Changelog

## Unreleased

- Demo- und Testwerkzeuge entfernt. Stattdessen sitzt ein kleiner Knopf in den Einstellungen, der Tester-Infos für Fehlerberichte kopiert.
- macOS-Build in der CI: ad-hoc signierte, nicht notarisierte `.app` als Artefakt. Ohne Schutzfunktionen, nicht gestartet.

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
