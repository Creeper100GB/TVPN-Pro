# Projektstand (kompakt)

Stand: 22. November 2026. Details stehen in `docs/reports/`, `docs/DECISIONS.md` und `docs/TASKS.md`.

## Läuft und ist geprüft

- **opencode 2.3.2**, acht Agenten mit V2-Frontmatter.
- **Rust-Kern `service/core`**: Zustandsautomat nach Vertrag, Settings inklusive `torrent_mode`, URL- und Magnet-Regeln, IPC-Envelope, Redaktions-Scanner.
- **UI** (Svelte 5, SvelteKit 3, Tailwind 4, Inter lokal): alle elf Zustände, Einstellungen, Sitzungs-Zusammenfassung, Download-Sheet mit Routen Tor/Direkt. Torrents sind ausdrücklich nicht implementiert. Die Desktop-App liest Status und Einstellungen jetzt über den echten Dienst und pollt alle 2,5 Sekunden.
- **Tauri-Shell**: echter Download (`reqwest`, Routen `direct` und `tor` über SOCKS 9250), SHA-256, Defender-Scan, Mark-of-the-Web, Quarantäne bei Fund, Abbruch löscht die Datei. Die Oberfläche redet über `service_status` und `service_command` mit `tvpnd`. Tray-Menü mit Panik, globaler Panik-Hotkey `Ctrl+Alt+Shift+F12`, Schließen blendet ins Tray.
- **`service/net`**: Tor-Control-Client, Mullvad-CLI-Wrapper und WFP-Default-Deny mit Loopback- und Programm-Freigaben. In der VM: 8 Block-/Loopback-Filter plus 4 Tor-Permits, `firewall_filters=12`, danach `0`. Die WFP-Session ist statisch: Nach einem Hard-Kill bleiben die Filter, der nächste Dienststart geht in `RECOVERY`; `recover` entfernt sie. In der VM geprüft.
- **`tvpnd`**: Protokoll (`hello`, `get_status`, `get_settings`, `set_settings`, `enable`, `disable`, `panic`, `recover`), Named Pipe mit Token/ACL, Dienst-Wrapper mit `--install`/`--uninstall`, echter Backend-Ablauf (WFP, VPN, Tor, Fail-Closed) und Session-Zusammenfassung. In der VM geprüft: Installation als Dienst, Pipe als Besitzer, Enable mit echtem `tor.exe`. Eine Panik beendet Tor und VPN und entfernt die Filter: das Netz ist danach offen, nicht gesperrt. Ein Hintergrund-Wächter prüft alle fünf Sekunden und schaltet auf `FAILCLOSED`; Einstellungen liegen in `%ProgramData%\TVPN\settings.json`.
- **Windows-Installer**: NSIS perMachine, bündelt `tvpnd.exe` und den geprüften Tor Expert Bundle. Postinstall installiert den Dienst, Preuninstall entfernt ihn. In der VM grün: Installation nach `Program Files`, Dienststart, Enable bis `DEGRADED`, blockierter Verkehr, Panik, Netz zurück, Uninstall. Signierung fehlt.
- **`service/privacy`**, **`service/launcher`**, **`service/scanner`**, **`service/watchdog`**: MAC- und Hostname-Primitive (S3: Neustart), Hash-/Versionsprüfung, Defender-Urteilslogik, Wächter-Bewertung, jeweils mit Unit-Tests.
- **Test-VM `tvpn-test`**: Windows 11 Enterprise Evaluation 22621, 4 vCPU, 4 GB RAM, Checkpoint `sauber`, lokale Admins `User` und `tvpn`. Der Dienst läuft dort als SYSTEM und als Konsolenprozess; der Tor Expert Bundle 15.0.26 ist mit signierter SHA-256 geprüft.
- **macOS-Build**: `macos.yml` baut eine ad-hoc signierte `.app` als Artefakt. Nicht gestartet, nicht notarisiert; der Nutzer übernimmt macOS später selbst.

## Fehlt noch

- **S1 / Mullvad**: Der WFP-Permit bindet noch nicht an das VPN-Interface; ohne Mullvad-App und Konto ist die VPN-zu-Tor-Kette ungeprüft. Bis dahin ist der Tor-only-Betrieb bewusst `DEGRADED`, nicht `READY`.
- **MAC-Wechsel**: Primitive und Tests stehen; der Hyper-V-Adapter meldet `unsupported`. Der echte Adapter-Write (Registry `NetworkAddress` plus Adapter-Neustart) muss auf physischer Hardware geprüft werden.
- **Hostname**: S3 zeigt `REBOOT_REQUIRED`; die UI zeigt `hostname_unchanged`. Die einmalige Neutralisierung im Setup ist noch nicht in den Installer eingebaut.
- **Launcher/Fallback**: Tor-Browser-Start, Block-Erkennung und Fallback-Browser sind nicht verdrahtet.
- **Downloader**: Der VPN-Pfad und die aria2-Integration fehlen; der Shell-Downloader deckt Direkt und Tor ab.
- **Wächter**: Der Hintergrund-Wächter (Thread im Dienst, 5-s-Takt) und der Status-Poll schalten bei fehlenden Filtern oder fehlendem Circuit auf `FAILCLOSED`. Tray-Icon und Panik-Hotkey `Ctrl+Alt+Shift+F12` funktionieren; ein getrennter Wächterprozess ist nicht geplant (`docs/ROADMAP.md`).
- **Installer**: Bundling und VM-Installation sind grün; Code-Signing-Zertifikat, USB-Kit und Update-Kanal fehlen.
- **Spikes S2, S4, S5, S6** und die vollständige Leak-Matrix.

## Befehle

```powershell
just verify                 # alles außer den Playwright-E2E: fmt, clippy, Tests, pscheck,
                            # Redaktions-Gate, UI-Unit-Tests, UI-Check, UI-Build
just e2e                    # Playwright inkl. Download und Aufräumen
cd ui; bun run check        # Svelte/TS
cd ui; bun run tauri dev    # echtes Fenster
cd ui; bun run tauri build  # Installer/Portable
```

Die VM-Tests laufen über `tests/vm/run-in-vm.ps1`; Beispiele stehen in `tests/vm/README.md`.

## Kernentscheidungen

Fokus-Modus (Blocken statt Umleiten), VPN → Tor, Torrents nur über VPN und standardmäßig blockiert, Tor Browser unverändert, keine Telemetrie, Host nur bauen/linten, alles Netzwerkwirksame in die VM. Vollständig: `docs/DECISIONS.md`.

## Nächster Schritt

1. S1 mit einem Mullvad-Konto in der VM abschließen und den Permit an das VPN-Interface binden.
2. MAC-Adapter-Write auf physischer Hardware prüfen; Hostname-Neutralisierung in den Installer aufnehmen.
3. Installer signieren, USB-Kit bauen und die Leak-Matrix fahren.
