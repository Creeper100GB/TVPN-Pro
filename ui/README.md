# UI-Shell

Svelte-5-Oberfläche für die Tauri-App. Im Desktop-Fenster reden die Kommandos über die Named Pipe mit `tvpnd`; im Browser (`bun run dev`) läuft derselbe Baum gegen den Mock in `src/lib/mock.svelte.ts`, damit die Ansicht ohne Dienst nutzbar bleibt. Der Zustandsautomat und die Einstellungen sind im Desktop echt, der Download läuft im Rust-Teil (siehe unten).

## Stack

| Baustein | Version | Zweck |
|---|---|---|
| Svelte | 5.59 | Komponenten, Runes |
| SvelteKit | 3.4 | Routing, statischer Build |
| Vite | 8.3 | Build und Dev-Server |
| Tailwind CSS | 4.3 | Utilities, Theme-Tokens |
| TypeScript | 6.0 | Typen (SvelteKit 3 braucht die JS-Compiler-API) |
| Lucide | 1.0 | Icons |
| Inter Variable | 5.3 | Schrift, lokal gebündelt (keine CDN) |
| Playwright | 1.65 | Screenshots und Layout-Prüfung (nur Dev) |

## Befehle

```bash
bun install
bun run dev        # http://localhost:5173
bun run check      # svelte-check, 0 Fehler erwartet
bun run build      # statischer Build nach build/
bun run preview    # baut nicht, serviert build/ auf 4173
bun run shots      # Screenshots nach .artifacts/ (preview muss laufen)
bun run e2e        # End-to-End-Tests (startet den Dev-Server selbst)
bun run test       # Unit-Tests der reinen UI-Logik (node:test über bun)
```

## End-to-End

`tests/e2e/downloads.spec.ts` prüft vier Dinge:

1. **Direkt gesperrt:** solange die Firewall scharf ist, lehnt das Sheet die
   Route „Direkt“ ab und legt keinen Eintrag an.
2. **Torrents gesperrt:** ein Magnet-Link wird als nicht implementiert
   abgelehnt, es entsteht kein Download-Eintrag.
3. **UI-Ablauf:** einschalten → Download hinzufügen → Urteil „Sauber" → ausschalten → Sitzungs-Zusammenfassung → Liste leeren.
4. **Test-Transport:** ein lokaler HTTP-Server liefert 256 KiB, die Datei landet in `tests/e2e/.tmp/downloads/`, wird per SHA-256 geprüft und danach mit dem Ordner gelöscht. `existsSync` bestätigt, dass nichts übrig bleibt; `afterAll` räumt auch nach einem Fehlschlag auf.

Der UI-Teil läuft im Browser gegen den Mock, mit dem Dienst gegen die echte Pipe. Der Datei-Teil prüft **nur den Test-Transport** gegen `127.0.0.1`: er zeigt, dass der Testaufbau schreibt, hasht und aufräumt. Den Rust-Downloader (`download.rs`) führt er nicht aus; den decken die Unit-Tests in `src-tauri/src/download.rs` und `scripts/test-desktop-download.mjs` ab.

Magnet-Links werden immer abgelehnt: es gibt keinen Torrent-Client und keine Seed-Funktion. Die Einstellung „Nur VPN“ ist noch sichtbar, hat aber keine Wirkung: laut `docs/ROADMAP.md` wird sie entfernt, nicht implementiert.

## Echter Download (Tauri)

In der Desktop-App läuft der Download im Rust-Teil (`src-tauri/src/download.rs`), nicht im Mock:

- **Direkt:** ungeschützt. Solange die Firewall scharf ist, lehnt das Sheet die Route ab: die Allowlist lässt nur Tor und VPN raus, ein direkter Download käme gar nicht durch.
- **Über Tor:** nutzt einen lokalen SOCKS-Proxy auf `127.0.0.1:9150`. Ohne laufenden Proxy bricht der Download mit einem Netzwerkfehler ab.
- **Über VPN:** wird abgelehnt (`vpn_requires_service`); diese Route gehört dem Dienst.

Dateien landen in `%USERPROFILE%\Downloads\TVPN`. Der Zielname wird vor dem Transfer mit `create_new` reserviert, damit zwei gleichzeitige Downloads nicht denselben Pfad wählen. Der Platzhalter hält den Namen bis zum `rename`; erst ein gescheitertes `rename` entfernt ihn vor dem Copy-Fallback, damit kein dritter Download den fertigen Namen vorher belegen kann. Während des Transfers wird SHA-256 berechnet; danach versucht die App einen Defender-Scan (`MpCmdRun.exe`, mit Zeitlimit). Ist Defender nicht aktiv oder hängt der Scan, lautet das Urteil „Unklar“ statt „Sauber“. Jede abgelegte Datei bekommt Mark-of-the-Web, auch bei „Unklar“. Ein Fund wandert in die Quarantäne und wird als „Quarantäne“ gezählt. Abbrechen entfernt die Datei.

Test: `bun run tauri dev --config tauri.debug.conf.json` startet die App mit WebView2-Debug-Port; `node scripts/test-desktop-download.mjs` lädt eine Testdatei über die echte UI, prüft die Größe und löscht sie wieder. **Nur in der Test-VM ausführen:** das Skript schaltet den echten WFP-Schutz ein und aus, lädt von einem externen Host und löscht alle neu erstellten Dateien in `Downloads\TVPN`. `scripts/check-window-state.mjs` prüft, dass maximierte Fenster keine runden Ecken mehr haben.

## Aufbau

- `src/routes/+page.svelte`: Fenster, Hero, Seitenleiste.
- `src/lib/mock.svelte.ts`: Zustandsautomat und Abläufe (Einschalten, Ausschalten, Panik, Recovery) mit Timern. Panik beendet den Schutz, öffnet das Netz und zeigt danach die Sitzungs-Zusammenfassung; eine Panik aus `Aus` schreibt keine neue und öffnet keine.
- `src/lib/states.ts`: Beschriftungen, Hinweise und Farbton je Zustand.
- `src/lib/components/`: PowerButton, Banner, StatRow, Segmented, Toggle, Sheet, SettingsSheet, DownloadSheet, SessionSummary.
- `src/lib/diagnostics.ts`: Tester-Infos für Fehlerberichte, ohne IP, MAC, URLs oder Dateinamen.
- `src/lib/theme.svelte.ts`: Auto/Hell/Dunkel, gespeichert in `localStorage`.

## Design-Regeln

- macOS-Anmutung: weiche Rundungen (14 bis 30 px), Blur, Haarlinien, großzügiger Weißraum, Inter mit negativer Laufweite.
- Keine eckigen Windows-Controls, keine Systemfarben, keine Emojis.
- Ein großer Button. Alles andere ist Status oder Einstellung.
- Farbe bedeutet etwas: grün geschützt, amber eingeschränkt, rot gesperrt, blau arbeitet.
- Ehrliche Texte. Nirgends „anonym“, „sicher“ oder „nicht zurückverfolgbar“.
- A11y: `role="switch"`, `aria-pressed`, `aria-live` für den Zustand, Fokusringe, 44-px-Ziele, `prefers-reduced-motion`.

## Tester-Infos

Unten in den Einstellungen sitzt der kleine Knopf **Tester-Infos kopieren**. Er legt Version, Plattform, WebView-Version, Fenstergröße, Zustand, Einstellungen und die letzte Sitzung in die Zwischenablage. Das reicht für einen Fehlerbericht und enthält keine IP-Adressen, MAC-Adressen, URLs oder Dateinamen.

Es gibt keine Demo-Zustände und keine Beispiel-Sitzung mehr. Die Oberfläche zeigt nur den normalen Ablauf.

## macOS

Auf einem Mac mit Xcode Command Line Tools erzeugt

```bash
cd ui
bun install --frozen-lockfile
bun run tauri build --config tauri.macos.conf.json
```

eine ad-hoc signierte, nicht notarisierte `.app` unter `src-tauri/target/release/bundle/macos/`. Die CI baut sie auf `macos-latest` und legt sie als Artefakt ab.

Für die Weitergabe an andere Macs braucht es ein Apple-Developer-Konto, eine Developer-ID-Signatur und eine Notarisierung. Ohne das blockiert Gatekeeper die App. Die Schutzfunktionen (Firewall, VPN, MAC) sind auf macOS nicht implementiert; das ist Phase 7.

## Noch nicht enthalten

- Tray-Icon mit Zustandsfarbe, Single-Instance-Schutz, Fensterzustand merken.
- Update-Kanal und Update-Prüfung.

Nicht geplant (siehe [`docs/ROADMAP.md`](../docs/ROADMAP.md)): ein zweiter Scanner
(ClamAV) und Torrent-Unterstützung. Geprüft wird über `svelte-check` (inkl.
`tests/`), `bun run test` für die reine UI-Logik und Playwright für die Abläufe.
