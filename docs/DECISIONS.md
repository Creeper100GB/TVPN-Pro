# Entscheidungen (ADR-Log)

Neue Einträge hängt nur der Orchestrator an. Status: `Vorschlag` oder `Abgenommen` (vom Menschen).

| ID | Entscheidung | Begründung | Überdenken, wenn | Status |
|---|---|---|---|---|
| D1 | Rust (Dienst) plus Tauri (UI) | Sicher, ein Binary, Web-Skills des Menschen nutzbar | Plattformgrenzen bei macOS-Erweiterungen | Vorschlag |
| D2 | **Fokus-Modus:** Blocken statt Umleiten, kein TUN in v1 | Weniger Angriffsfläche, kein Loop, kein DNS-Leak durch Umleitung | Nutzer will wirklich alle Apps über Tor (System-Modus, S8) | Vorschlag |
| D3 | Reihenfolge VPN → Tor | Provider sieht Tor-Nutzung nicht | Tor-Projekt oder Anbieter raten klar ab | Vorschlag |
| D4 | Mullvad als VPN (Kontonummer, anonyme Zahlung) | Kein Personenbezug nötig, CLI vorhanden | Konflikte mit WFP (S1) | Vorschlag |
| D5 | aria2 als Standard-Downloader, FDM als Adapter | Open Source, per RPC steuerbar, Proxy per HTTP-CONNECT | S4 zeigt Probleme | Vorschlag |
| D6 | Mullvad Browser als Fallback-Browser | Für VPN-Nutzung gebaut, gleiche Fingerprint-Schutz-Basis | S5 zeigt Probleme | Vorschlag |
| D7 | Scanner orchestriert Defender und ClamAV, kein eigener Engine-Bau | Ehrlicher und wartbar | Defender nicht verfügbar | Vorschlag |
| D8 | Hostname einmalig neutralisieren (Plan A) | Neustart pro Sitzung unpraktisch | S3 findet Weg ohne Neustart | Vorschlag |
| D9 | Block-Erkennung per Fenstertitel, kein Add-on, kein WebDriver | Fingerprint bleibt unverändert | S5 findet bessere Methode | Vorschlag |
| D10 | Logging: Dev-Logs ohne sensible Daten, Release aus | Debugbar ohne Spuren | Datenschutzprobleme in Tests | Vorschlag |
| D11 | Dienstkonto: so wenig Rechte wie möglich (offen, LocalSystem oder eigenes Konto) | Angriffsfläche | Technisch nicht machbar | Offen, Orchestrator klärt in Phase 1 |
| D12 | Mullvad-Lockdown als Basis oder eigene WFP-Basis | Eine Quelle der Wahrheit | S1 | Offen, entscheidet S1 |
| D13 | opencode **v2.3.2**: Agent-Frontmatter auf natives V2-Format umgestellt (`permissions`-Array, Actions `shell`/`edit`/`external_directory`) | Die V1-Felder (`permission`, `bash`) sind in V2 abgekündigt und werden nur noch übersetzt; das Array macht Reihenfolge und „letzte Regel gewinnt" explizit | opencode ändert das Schema | Abgenommen (Auftrag P0-01) |
| D14 | `external_directory: ask` für alle Rollen | V2 erlaubt Zugriffe außerhalb des Projekts standardmäßig; V1 fragte nach. `ask` stellt das alte Verhalten wieder her und hält Agenten im Repo | Arbeit außerhalb des Repos wird regelmäßig nötig | Abgenommen (Auftrag P0-01) |
| D15 | Rollen bleiben `mode: all`, Orchestrator `mode: primary` | Rollen starten als primärer Agent im eigenen Worktree-Tab und können zusätzlich als Subagent dienen | Subagent-Nutzung wird zum Problem | Abgenommen (Auftrag P0-01) |
| D16 | Test-VM: **Hyper-V** (PowerShell Direct, Checkpoint-Rollback) | Windows 11 Enterprise, Hyper-V-Modul vorhanden, kein Netzwerk in der VM nötig, Snapshots schnell | Mensch wechselt auf VirtualBox/VMware | Abgenommen (Mensch: „entscheide selbst") |
| D17 | Rust-Workspace mit sechs leeren Crates (`service/{core,net,privacy,launcher,scanner,watchdog}`), Edition 2021, `rust-version = 1.85`, keine Logik | Gerüst für die Rollen, CI prüfbar, ohne den Spikes vorzugreifen | Edition 2024 oder andere Aufteilung wird nötig | Abgenommen (Auftrag P0-02) |
| D18 | CI auf GitHub Actions, `windows-latest`, Rust über das vorinstallierte `rustup` statt Drittanbieter-Action | Kein zusätzlicher Supply-Chain-Knoten, Windows ist die Zielplattform | Linux/macOS-Jobs werden nötig | Vorschlag |
| D19 | Redaktions-Gate prüft IPv4, IPv6, MAC und URLs; Loopback ist erlaubt | Diese vier Klassen sind billig und zuverlässig erkennbar; Dateinamen und Domains brauchen Regeln im Code und Review | Gate erzeugt Fehlalarme oder übersieht echte Fälle | Vorschlag |
| D20 | IPC-Vertrag v0 erweitert: Transport, Token, Envelope, Settings-Whitelist, vollständige Übergangstabelle, Fehlercodes, URL-Regeln; JSON-Schema-Dateien erst in Phase 2 | Ohne diese Punkte ist der Vertrag nicht implementierbar (Lücken siehe `docs/reports/phase-0.md`) | Schema-Dateien werden früher gebraucht | Vorschlag |
| D21 | Toolchain: rustup + Rust MSVC, VS Build Tools 2022 (VCTools), PowerShell 7, tauri-cli; Node 26.13 LTS und bun 1.3.14 waren vorhanden | Ohne MSVC-Linker und Rust ist kein Build möglich; Tauri braucht beides | eine Komponente macht Probleme | Abgenommen (Mensch: „installiere alles") |
| D22 | Eigene Projektlizenz bleibt offen | Entscheidung des Menschen, beeinflusst Bündelung der GPL-Komponenten | vor dem ersten Release oder öffentlichen Push | Offen |
| D23 | UI: Svelte 5 + SvelteKit 3 + Vite 8 + Tailwind 4 + Lucide + lokal gebündeltes Inter; Playwright nur als Dev-Abhängigkeit | Aktuelle Generationen, keine CDN-Abhängigkeit, konsistente Schrift auf Windows und macOS, prüfbares Design | SvelteKit 3 macht Probleme oder eine Komponentenbibliothek wird nötig | Abgenommen (Mensch: „baue und prüfe das Design") |
| D24 | Relative Importe statt `$lib`/`#lib` | SvelteKit 3 hat `alias` abgekündigt, und Subpath-Importe lösen `.svelte.ts`-Dateien nur mit expliziter Endung auf. Relative Pfade sind eindeutig und typisierbar | SvelteKit liefert eine saubere Alias-Lösung | Abgenommen (Auftrag „Fehler fixen") |
| D25 | MSVC-Umgebung über `tools/with-msvc.ps1` (vcvars64 via vswhere), nicht über globale Umgebungsvariablen | Ein normales Terminal hat `LIB`/`INCLUDE` nicht; der Wrapper ist reproduzierbar, CI hat die Umgebung ohnehin | Eine dauerhafte Developer-Shell wird bevorzugt | Vorschlag |
| D26 | Tauri-Fenster: frameless, transparent, eigener Titelbar mit Drag-Region; Inhalt als gerundetes Panel | Windows-Chrome würde die macOS-Anmutung brechen; Transparenz liefert die weichen Ecken | Transparente Fenster machen unter WebView2 Probleme | Vorschlag |
| D27 | UI läuft bis zur IPC-Implementierung gegen Mock-Daten; Playwright-Screenshots sind das Design-Gate | Ohne Dienst ist die UI sonst nicht prüfbar; die Screenshots zeigen alle Zustände in Hell und Dunkel | Der Dienst liefert echte Daten | Abgenommen (Auftrag „checke alles") |
| D28 | E2E: Playwright gegen den Dev-Server, UI gegen Mock, plus echter Datei-Download von einem lokalen HTTP-Server mit SHA-256 und garantierter Löschung | Ohne Dienst und Downloader ist nur so ein vollständiger Ablauf prüfbar; keine externen Abhängigkeiten, kein Testverkehr nach außen | IPC und aria2-Kette existieren | Abgenommen (Auftrag „E2E einrichten") |
| D29 | Paketmanager **bun** statt npm; `ui/bun.lock` ersetzt `package-lock.json` | AGENTS.md bevorzugt bun; ein Lockfile, keine zwei | Ein Werkzeug setzt npm zwingend voraus | Vorschlag |
| D30 | Torrents **nur über VPN** (`torrent_mode = vpn_only`), nie über Tor; Standard `blocked` | Tor plus BitTorrent ist Netzmissbrauch, die Exit-Policy blockt, DHT/LPD/PEX/UPnP sind Leak-Pfade. Die VPN-Route gibt Privatsphäre auf VPN-Niveau, keine Anonymität. Mensch will Betriebssystem-Images per Torrent. | Tor-Projekt ändert die Haltung, oder der Mensch akzeptiert die Restrisiken nicht | Abgenommen (Mensch, 21.11.) |
