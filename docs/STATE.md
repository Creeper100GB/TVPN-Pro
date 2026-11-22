# Projektstand (kompakt)

Stand: 22. November 2026. Einstieg nach einer Kontext-Kompaktierung. Details stehen in `docs/reports/`, `docs/DECISIONS.md` und `docs/TASKS.md`.

## Läuft und ist geprüft

- **opencode 2.3.2**, acht Agenten mit V2-Frontmatter.
- **Rust-Kern `service/core`**: Zustandsautomat nach Vertrag, Settings inklusive `torrent_mode`, URL- und Magnet-Regeln, IPC-Envelope, Redaktions-Scanner. 48 Tests, clippy sauber.
- **UI** (Svelte 5, SvelteKit 3, Tailwind 4, Inter lokal): alle elf Zustände, Einstellungen, Sitzungs-Zusammenfassung, Tester-Infos in den Einstellungen, Download-Sheet mit Routen Tor/Direkt/VPN, Bulk-Route für Torrents, Hell/Dunkel. `svelte-check` 0/0.
- **E2E** (Playwright): 5 Tests, UI-Ablauf, VPN-Bulk-Route, Ablehnung bei blockierten Torrents, echter Datei-Download mit Hash und Löschung, Tester-Infos in der Zwischenablage.
- **Tauri-Shell**: echter Download im Rust-Teil (`reqwest`, Routen `direct` und `tor` über SOCKS 9150), SHA-256, Defender-Scan (Windows), Abbruch löscht die Datei. Maximierte und Fullscreen-Fenster ohne runde Ecken (verifiziert). Windows-Installer (NSIS) und portable EXE gebaut und gestartet.
- **`service/net`**: Tor-Control-Client (Cookie-Auth, Bootstrap, Circuit, NEWNYM), Mullvad-CLI-Wrapper (Statusparser, feste Argumente, Länderprüfung) und WFP-Default-Deny mit Loopback-Freigabe. Host-Tests grün; WFP in der VM grün (`armed=8`, `disarmed=0`). tor.exe- und Mullvad-App-Integration bleiben VM-Arbeit.
- **macOS-Build**: `macos.yml` baut auf `macos-latest` eine ad-hoc signierte `.app` (12,4 MiB) und legt sie als Artefakt ab. Nicht gestartet, nicht notarisiert, ohne Schutzfunktionen.
- **VM-Runner**, **Canary-Dienst**, **CI-Definitionen**: geschrieben; CI läuft auf GitHub. Der VM-Runner ist seit 22.11. in `tvpn-test` geprüft (Checkpoint-Rollback, PowerShell Direct, Smoke-Test grün); Canary bleibt ungeprüft.
- **Test-VM `tvpn-test`**: Windows 11 Enterprise Evaluation 22621, 4 vCPU, 4 GB RAM, Checkpoint `sauber`, lokale Admins `User` und `tvpn`. WFP-Default-Deny grün (`armed=8`, `disarmed=0`); S3 ergibt `REBOOT_REQUIRED`.

## Fehlt noch

- `tvpnd`: UI-Anbindung und privilegierte Kommandos; Protokoll, Pipe, Token/ACL und Dienst-Wrapper stehen.
- Netzschicht: WFP-Allowlist, Tor/Mullvad in `tvpnd` einbinden, Recovery.
- privacy-shield: MAC-Wechsel; Hostname einmalig im Setup (S3).
- launcher/scanner: Tor Browser am System-Tor, aria2, EICAR/MOTW/Quarantäne.
- watchdog, Installer-Signierung, USB-Kit, macOS-Schutzfunktionen und Notarisierung.
- Spikes S1, S2, S4-S6 und Leak-Matrix in der VM; S3 ist entschieden.

## Befehle

```powershell
just verify                 # fmt, clippy, Tests, pscheck, Redaktions-Gate
just e2e                    # Playwright inkl. Download und Aufräumen
cd ui; bun run check        # Svelte/TS
cd ui; bun run tauri dev    # echtes Fenster
cd ui; bun run tauri dev --config tauri.debug.conf.json   # mit CDP-Port 9222
cd ui; bun run tauri build --config tauri.macos.conf.json # macOS, nur auf einem Mac
```

Downloads landen in `%USERPROFILE%\Downloads\TVPN`. Die Desktop-Downloads lassen sich mit `ui/scripts/test-desktop-download.mjs` prüfen (App muss mit Debug-Config laufen).

## Kernentscheidungen

Fokus-Modus (Blocken statt Umleiten), VPN → Tor, Torrents nur über VPN und standardmäßig blockiert, Tor Browser unverändert, keine Telemetrie, Host nur bauen/linten, alles Netzwerkwirksame in die VM. Vollständig: `docs/DECISIONS.md` (D1 bis D32).

## Nächster Schritt

1. `service/net` in `tvpnd` einbinden (enable/disable/panic/recover, fail closed).
2. UI von Mock auf IPC umstellen.
3. Spikes S1 und S2 in der VM; WFP-Allowlist-Tests.
