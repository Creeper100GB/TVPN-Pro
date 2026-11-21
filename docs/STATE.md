# Projektstand (kompakt)

Stand: 21. November 2026. Einstieg nach einer Kontext-Kompaktierung. Details stehen in `docs/reports/`, `docs/DECISIONS.md` und `docs/TASKS.md`.

## Läuft und ist geprüft

- **opencode 2.3.2**, acht Agenten mit V2-Frontmatter.
- **Rust-Kern `service/core`**: Zustandsautomat nach Vertrag, Settings inklusive `torrent_mode`, URL- und Magnet-Regeln, IPC-Envelope, Redaktions-Scanner. 48 Tests, clippy sauber.
- **UI** (Svelte 5, SvelteKit 3, Tailwind 4, Inter lokal): alle elf Zustände, Einstellungen, Sitzungs-Zusammenfassung, Tester-Infos in den Einstellungen, Download-Sheet mit Routen Tor/Direkt/VPN, Bulk-Route für Torrents, Hell/Dunkel. `svelte-check` 0/0.
- **E2E** (Playwright): 5 Tests, UI-Ablauf, VPN-Bulk-Route, Ablehnung bei blockierten Torrents, echter Datei-Download mit Hash und Löschung, Tester-Infos in der Zwischenablage.
- **Tauri-Shell**: echter Download im Rust-Teil (`reqwest`, Routen `direct` und `tor` über SOCKS 9150), SHA-256, Defender-Scan (Windows), Abbruch löscht die Datei. Maximierte und Fullscreen-Fenster ohne runde Ecken (verifiziert). Windows-Installer (NSIS) gebaut und gestartet.
- **macOS-Build**: `macos.yml` baut auf `macos-latest` eine ad-hoc signierte `.app` (12,4 MiB) und legt sie als Artefakt ab. Nicht gestartet, nicht notarisiert, ohne Schutzfunktionen.
- **VM-Runner**, **Canary-Dienst**, **CI-Definitionen**: geschrieben; CI läuft auf GitHub, VM-Runner und Canary sind weiterhin ungeprüft (keine VM, kein Server).

## Fehlt noch

- `tvpnd`: Named Pipe, Token/ACL, Kommandos, Logging-Anbindung.
- Netzschicht: WFP-Firewall, Tor-Controller, Mullvad-VPN, Recovery.
- privacy-shield: MAC, Hostname (S3).
- launcher/scanner: Tor Browser am System-Tor, aria2, EICAR/MOTW/Quarantäne.
- watchdog, Installer-Signierung, USB-Kit, macOS-Schutzfunktionen und Notarisierung.
- Spikes S1 bis S6, Leak-Matrix in der VM.

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

1. Test-VM mit Checkpoint `sauber`.
2. Spikes S1 und S3 parallel (net-core, privacy-shield).
3. `tvpnd` mit Pipe und Token, UI von Mock auf IPC umstellen.
