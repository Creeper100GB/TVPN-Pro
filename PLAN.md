# Projektplan: TVPN Pro (Master-Plan v2 für den Orchestrator-Agenten)

> Du bist der **Orchestrator**. Lesereihenfolge: `AGENTS.md` → dieser Plan → `contracts/ipc-v0.md` → `docs/SPIKES.md` → `docs/TASKS.md`. Starte mit Phase 0.
> Rollen-Agenten: `.opencode/agents/`. Entscheidungen: `docs/DECISIONS.md`. Übergaben: `docs/handoffs/`.

## 0. Spielregeln für dich

1. **Risk-first:** Baue kein Feature, bevor die zugehörigen Spikes (`docs/SPIKES.md`) entschieden sind. Jeder Spike endet mit einem Eintrag in `docs/DECISIONS.md`. Widerspricht ein Ergebnis diesem Plan, änderst du den Plan nur mit Mensch-Gate.
2. **Host-Schutz:** Nichts mit Netzwerk-, Firewall-, MAC-, Hostname-, Dienst- oder Treiberwirkung läuft auf dem Host-PC. Nur in der Test-VM über `tests/vm/run-in-vm.ps1`. Ein Bug darf den Menschen nicht aus dem eigenen Internet aussperren.
3. **Mensch-Gates (stoppen und fragen):** Abnahme am Ende jeder Phase (G0 bis G9), vor Löschen, Force-Push oder Installation von Systemsoftware, bei jeder Abweichung von den Sicherheitsregeln, bei Kosten (Zertifikate, Developer-Programm).
4. **Versionscheck:** `opencode --version`, dann die Agent-Dateien gegen die Docs dieser Version abgleichen (V1 und V2 unterscheiden Config-Keys, z. B. `permission` und `permissions`). Korrigiere Dateien bei Bedarf und notiere es in `docs/DECISIONS.md`.
5. **Scheitern begrenzen:** Scheitert ein Agent zweimal am selben Problem, schreibt er eine Übergabe (`docs/handoffs/`) und fragt den Menschen.
6. **Ehrlichkeit:** Nirgends (Code, UI, Doku) steht "anonym", "sicher" oder "nicht zurückverfolgbar". Formulierung: "reduziert Spuren, ersetzt kein vorsichtiges Verhalten."
7. **Mensch mitnehmen:** Erkläre Terminal-, Git-, Windows-Dienst- und Firewall-Konzepte, die du nutzt, in ein bis zwei Sätzen.

## 1. Übersicht

- **Ziel:** Windows-App mit einem großen Button. Ein Klick: Firewall scharf, MAC wechseln, VPN, Tor, Tor Browser und Download-Manager mit Virenscan. Zweiter Klick: alles sauber abbauen.
- **Plattform:** Windows zuerst, macOS danach. Auslieferung per **USB-Stick mit Installern** (kein Portable-Build).
- **Team:** 1 Mensch + Orchestrator + 7 Rollen-Agenten in git-Worktrees.
- **Stand:** Grüne Wiese, nur Idee und diese Spezifikation.
- **Die App leistet:** IP verbergen (VPN, dann Tor), **Blocken statt Hoffen** (alles, was nicht über Tor darf, wird von der Firewall verboten), MAC-Wechsel, unveränderter Tor Browser, Download-Pipeline mit Scan, laufende Selbstprüfung, Notfall-Reset.
- **Die App leistet nicht:** Schutz vor Gegnern, die beide Enden des Tor-Netzes beobachten; vor Browser-0-Days; vor eigenen Fehlern (Logins, Schreibstil, Dateien öffnen); vor einem kompromittierten Windows; Unsichtbarkeit.
- **Nicht-Ziele:** eigener Browser, eigener VPN-Anbieter, eigener Kernel-Treiber, eigene AV-Engine.

## 2. Threat Model (Kurzfassung, ausschreiben als `docs/THREAT_MODEL.md`)

| Beobachter | Sieht ohne App | Maßnahme | Restrisiko |
|---|---|---|---|
| Website/Tracker | IP, Fingerprint, Cookies | Tor-Exit-IP, Tor Browser unverändert, getrennte Konten | Verhalten, Browser-0-Days |
| Lokales Netz/Café | MAC, Hostname, WLAN-Aktivität | MAC-Wechsel, Firewall blockt alles Fremde | Hostname (S3), Portal sieht das Gerät |
| ISP | Ziele, Tor-Nutzung | VPN zuerst | Sieht VPN-Nutzung, Mengen, Zeiten |
| VPN-Anbieter | echte IP, Tor-Nutzung | Konto ohne Personenbezug, anonyme Zahlung | "No-log" ist nur ein Versprechen |
| Tor-Relays | Entry: VPN-IP. Exit: Klartext | VPN vor Tor, HTTPS-only | Korrelation bei beobachteten Enden |
| Andere Software, Downloads | alles, was Apps melden | Firewall-Allowlist, Scanner | 0-Days, IP-verratende Dokumente nach dem Ausschalten |
| Zugriff aufs Gerät | lokale Spuren | Aufräumen, Pflichthinweis Plattenverschlüsselung | Beschlagnahme |

## 3. Architektur

```
UI (Tauri, ohne Rechte) --IPC (Named Pipe, ACL)--> Dienst tvpnd (Admin)
Wächter (eigener Prozess) <--Herzschlag--------->  Dienst tvpnd
tvpnd steuert: Mullvad-Daemon (VPN) | tor.exe | WFP-Firewall (eigene Filter) | Scanner | Launcher
```

**Netzpfade (Fokus-Modus, v1):**
- Tor Browser → `127.0.0.1:Port A` → `tor.exe` → VPN-Adapter → Internet
- Downloader → `127.0.0.1:Port B` → `tor.exe` → VPN-Adapter → Internet
- Fallback-Browser (nur auf Abruf) → direkt → VPN-Adapter → Internet (ohne Tor)
- Alles andere → Firewall blockt

```
tvpn-pro/
├── AGENTS.md  PLAN.md  START-HIER.md  KICKOFF-PROMPT.md
├── contracts/     # IPC-Schema, Zustände, Fehlercodes = gemeinsame Wahrheit
├── ui/            # Tauri + Web-Frontend                      (ui-shell)
├── service/
│   ├── core/      # Zustandsautomat, IPC-Server, Logging       (orchestrator)
│   ├── net/       # VPN, Tor, WFP-Firewall, Recovery           (net-core)
│   ├── privacy/   # MAC, Hostname                              (privacy-shield)
│   ├── launcher/  # Tor Browser, Downloader, Block-Fallback    (launcher)
│   ├── scanner/   # Download-Scan                              (scanner)
│   └── watchdog/  # Wächter                                    (watchdog)
├── installer/     # Windows, macOS, USB-Kit                    (orchestrator)
├── tests/         # Leak-Tests, VM-Runner                      (qa-security)
├── tools/         # Worktree-Skripte
└── docs/          # TASKS, DECISIONS, SPIKES, handoffs, reports
```

- **Default-Stack:** Rust (Dienst) + Tauri (UI) + PowerShell (Tools). OS-Code hinter Traits (`FirewallPlatform`, `MacChanger`, `ProcessGuard`) mit `windows/` und `macos/`, damit der macOS-Port nichts umwirft.
- **Bausteine (fertig nutzen):** `tor` (Expert Bundle), Mullvad-App/CLI, Tor Browser, Mullvad Browser (Fallback), aria2 (Downloader-Default), ClamAV, Windows Defender (CLI). Alle mit **fester Version und SHA-256 gepinnt**, Tor Browser per GPG-Signatur geprüft. Lizenzen (teils GPL) in `docs/LICENSES.md`, bevor etwas weitergegeben wird.

## 4. Netzwerk-Design (Fokus-Modus ist v1)

**4.1 Prinzip: Blocken statt umleiten.** Nichts darf an Tor vorbei, weil die Firewall es verbietet, nicht weil eine Umleitung hoffentlich greift. Kein TUN in v1: weniger Angriffsfläche, kein Routing-Loop, kein DNS-Leak durch Umleitung. Das System-Modus-Konzept (TUN) kommt erst nach dem Audit (4.9).

**4.2 Regelwerk (Windows Filtering Platform).** Eigener Provider und Sublayer mit fester GUID. Filter werden bei ARMING gesetzt und sind **persistent**, damit sie auch halten, wenn der Dienst abstürzt (fail-closed).
1. Default: jeder ausgehende Verkehr blockiert (IPv4 und IPv6, TCP und UDP).
2. Erlaubt: Loopback; DHCP/ARP/NDP nur soweit für den Link nötig; Mullvad-Daemon zum gewählten VPN-Endpunkt; `tor.exe` ausgehend **nur über den VPN-Adapter**; Allowlist-Apps **nur zu den Tor-Ports auf Loopback**; Fallback-Browser nur über den VPN-Adapter und nur, wenn aktiviert; Torrent-Client nur über den VPN-Adapter und nur bei `torrent_mode = vpn_only` (kein DHT/LPD/PEX/UPnP, keine eingehenden Verbindungen).
3. Windows Update, Cloud-Sync, Telemetrie und fremde Apps bleiben offline, solange die App an ist.
4. Block-Ereignisse werden mitgelesen (für den Wächter, ohne Ziele zu speichern).
5. Ob die Interface-Bedingung auf der gewählten Filterebene verfügbar ist (sonst Quell-IP des VPN-Adapters), klärt **Spike S1**.

**4.3 Mullvad-Zusammenspiel.** Der Mullvad-Daemon hat einen eigenen Lockdown-/Always-require-VPN-Modus (Name je Version). Es darf **eine** Quelle der Wahrheit pro Regel geben, keine widersprüchlichen Filter. S1 entscheidet: Mullvad-Lockdown als Basis plus unsere Allowlist-Filter, oder umgekehrt.

**4.4 DNS.** Apps schicken Hostnamen an Tor (SOCKS5 mit Remote-Auflösung bzw. HTTP-CONNECT). Lokales DNS wird nicht gebraucht und ist blockiert. Test: kein DNS-Paket verlässt den Rechner außer im Tunnel.

**4.5 Isolation.** Port A für den Tor Browser (Tor-Browser-Standard), Port B für den Downloader mit eigenen Isolationsflags. Wer für IP-gebundene Download-Links dieselbe Leitung will, schaltet das bewusst ein (UI-Warnung: Browser und Downloader werden verknüpfbar).

**4.6 Tor-Betrieb.** `DataDirectory` bleibt erhalten (feste Entry-Guards sind sicherer als neue pro Sitzung). Control-Port nur Loopback mit Cookie-Auth. Alle Ports nur Loopback. IPv6 aus. Bridges optional (Einstellung).

**4.7 Zustand PORTAL.** Öffentliche WLANs verlangen oft einen Login, bevor ein VPN funktioniert. Kurzes Zeitfenster (z. B. 3 Minuten), in dem nur der **Fallback-Browser** (nicht der normale Browser) direkt ins Netz darf, um die Portal-Seite zu öffnen. Banner: "Das Portal sieht dein Gerät ohne VPN." Danach wieder voll scharf, weiter mit VPN. Klärt S6.

**4.8 Netzwechsel und Standby.** Ereignisse (Adapter hoch/runter, WLAN-Wechsel, Standby/Resume, VPN-Serverwechsel, Schnellstart/Ruhezustand) bewerten den Zustand neu, bei Unklarheit `FAILCLOSED`. Die MAC wird pro Adapter gemerkt, ein neuer Adapter bekommt eine neue MAC, bevor er genutzt wird.

**4.9 System-Modus (optional, nach Audit).** TUN-Umleitung für Apps außerhalb der Allowlist. Erst wenn Fokus-Modus stabil ist. Details in S8.

## 5. Zustandsautomat und Abläufe

Zustände: `OFF, PREFLIGHT, ARMING, PORTAL, VPN_UP, TOR_UP, READY, DEGRADED, FAILCLOSED, TEARDOWN, RECOVERY` (Details `contracts/ipc-v0.md`).

**Einschalten (ein Klick):**
1. PREFLIGHT: Admin-Rechte, Hashes und Signaturen aller Bausteine, Mullvad installiert und angemeldet, Tor-Browser-Mindestversion, Reste einer alten Sitzung aufräumen.
2. ARMING: Firewall scharf (Default-Deny, nur Link-Betrieb erlaubt).
3. MAC wechseln (Hostname je S3), Netz abwarten. So fließt unter der alten MAC nichts mehr nach außen.
4. PORTAL, falls nötig (4.7).
5. VPN verbinden (zufälliges Land), Status prüfen.
6. Tor starten (nur über den VPN-Adapter erlaubt), Bootstrap 100 %, Circuit steht.
7. Lokaler Leak-Check (Filtersatz vollständig, Standardroute, IPv6 blockiert) plus einmaliger Exit-Check über Tor.
8. Allowlist-Apps freischalten (nur Tor-Ports), Tor Browser und Downloader starten.
9. READY, Wächter aktiv. Bei jedem Fehler vorher: `FAILCLOSED`, Meldung, Angebot Neustart oder Notfall-Reset.

**Ausschalten (ein Klick):**
1. Laufende Downloads prüfen: Dialog "Warten / Pausieren / Abbrechen" (Pausieren nur, wenn der Server Resume kann).
2. Apps schließen (Browser-Session wird gelöscht), Scanner zu Ende laufen lassen.
3. Tor stoppen, VPN trennen. Die Firewall bleibt noch scharf.
4. Neue MAC (oder Original, je Einstellung), Hostname je S3.
5. Firewall entwaffnen, Aufräumen (Verlauf, temporäre Dateien), Session-Zusammenfassung anzeigen.
6. OFF.

**Recovery:** `tvpn-recover.exe` (eigenständig, ohne Dienst, auch aus dem abgesicherten Modus) entfernt nur Filter mit unserer Provider-GUID und stellt Adapter und MAC wieder her. Der Dienst prüft beim Start auf Reste und bietet Recovery an.

## 6. Feature-Spezifikationen

### 6.1 privacy-shield
- Nur physische Adapter (WLAN, Ethernet), keine virtuellen und keine VPN-Adapter.
- MAC: lokal administriert, unicast, zufällig. Registry `NetworkAddress` plus Adapter-Neustart. Reihenfolge laut Abschnitt 5.
- Original sicher merken. Modi: "neu würfeln" (Default) und "Original zurück" (Heimnetz mit MAC-Filter oder DHCP-Reservierung).
- Treiber ohne Unterstützung: Zustand melden, Betrieb nur mit Warnung und Bestätigung.
- **Hostname:** Windows übernimmt einen neuen Rechnernamen meist erst nach Neustart. **S3** klärt, ob pro Sitzung ein Wechsel ohne Neustart geht. Plan A: Hostname einmalig neutral und zufällig im Setup (ein Neustart), pro Sitzung nur die MAC. UI-Hinweis: gleicher Hostname kann Sitzungen im selben Netz verknüpfen.
- WLAN-Profile: Auto-Verbinden mit bekannten Netzen verrät Netznamen. In v1 nur Hinweis, später optional abschalten.

### 6.2 launcher
**Tor Browser**
- Nur von torproject.org, GPG-Signatur prüfen, Mindestversion erzwingen (älter: Start verweigern). Der eigene Updater darf laufen (über Port A).
- Start am System-Tor (Umgebungsvariablen wie `TOR_SKIP_LAUNCH`, SOCKS-/Control-Port, Cookie-Datei; **S2** prüft). Kein zweites Tor.
- **Prefs-Regel:** Nur Einstellungen, die Webseiten nicht beobachten können (Download-Ordner, Update-Verhalten). Alles andere bleibt unverändert. Die Sicherheitsstufe wird nie still geändert, die UI erklärt Standard, Safer, Safest.
- Keine Add-ons, kein Marionette/WebDriver (würde `navigator.webdriver` setzen).

**Downloader**
- aria2 (Default): RPC nur Loopback mit Secret-Token, `--all-proxy` auf Port B (HTTP-CONNECT, daher nur HTTPS-Ziele; HTTP-Links werden abgelehnt), kein BitTorrent/DHT/LPD auf der Tor-Route, wenige Verbindungen pro Server, `--continue` für Resume.
- **Bulk-Route** (`torrent_mode = vpn_only`): Torrents und große Images laufen **nur über den VPN-Adapter**, nie über Tor. DHT, Local Peer Discovery, Peer Exchange, UPnP/NAT-PMP und eingehende Verbindungen sind aus; die Firewall erlaubt auf dieser Route ausschließlich den Torrent-Client. Peers sehen die VPN-IP, der VPN-Anbieter sieht den Traffic, der ISP sieht VPN-Traffic. Banner `vpn_route` bleibt sichtbar, solange die Route aktiv ist. Standard ist `blocked`.
- Free Download Manager nur als Adapter, wenn **S4** zeigt, dass er sauber über den Proxy geht und Torrent aus ist.
- Übergabe: Link-Feld, optional Zwischenablage-Beobachtung (nur wenn eingeschaltet). Hoster-Seiten mit Timer oder Captcha lädt der Tor Browser selbst.

### 6.3 Block-Erkennung und Fallback
**Ziel:** Wird eine Seite blockiert und es erscheint keine Mensch-Abfrage, wechselt die App selbst.
- **Erkennung ohne Add-on:** Fenstertitel des Tor Browsers lesen (Windows: `GetWindowText`, lokal, kein Eingriff). Nur harte Block-Muster ("Access Denied", "403", "Error 1020", "Blocked"). Challenge-Seiten ("Just a moment", Captcha) lösen nichts aus.
- **Kernproblem URL:** Der Titel verrät die URL nicht. Optionen: (a) Adressleiste per UI Automation nur im Moment des Blocks lesen, nur im Speicher, wenn S5 zeigt, dass das nichts Web-Sichtbares ändert; (b) Nutzer kopiert die URL, die App übernimmt sie; (c) Marionette/WebDriver ist verboten.
- **Aktion:** URL im **Fallback-Browser** (Mullvad Browser) öffnen. Dank Fokus-Modus reicht dafür eine Firewall-Regel, die ihm nur den Weg über den VPN-Adapter erlaubt, ohne Tor. Der Tor Browser bleibt unberührt, kein globaler Moduswechsel.
- **Banner immer:** "Diese Seite sieht deine VPN-IP. Nicht mit denselben Konten wie im Tor-Fenster anmelden." Log nur "Block erkannt, Domain-Hash".
- **Stufen:** aus / Vorschlag per Klick / automatisch (Default: automatisch, wie gewünscht). Fallback-Fenster schließt nach Inaktivität, Profil flüchtig.
- **Ehrlich:** Heuristik. Die manuelle Taste "In VPN-Browser öffnen" ist immer da. Scheitert S5, bleibt nur sie (mit URL-Eingabe).

### 6.4 scanner
Kein eigener Engine-Bau, sondern Orchestrierung.
- **Pipeline:** Überwachte Ordner (Downloader-Ordner und Tor-Browser-Download-Ordner) = `Eingang/` → warten bis die Datei stabil ist → SHA-256 → Engines → Regeln → Urteil `SAUBER / WARNUNG / GEFUNDEN / FEHLER`.
- **Engines:** Windows Defender per CLI (`MpCmdRun.exe -Scan -ScanType 3 -File`, Verfügbarkeit per `Get-MpComputerStatus`, kann durch Dritt-AV ersetzt sein) plus ClamAV (Signatur-Updates über Tor). ClamAV braucht mit `clamd` viel RAM, S4 misst das. macOS später XProtect/ClamAV.
- **Regeln:** gefährliche Typen (`exe, scr, lnk, js, vbs, msi, iso, docm, xlsm`), Doppel-Endungen, Archive mit Tiefen- und Größenlimit (Zip-Bomb), PDF/Office mit aktiven oder externen Inhalten → Warnung.
- **Aktionen:** SAUBER → `Sauber/`, **Mark-of-the-Web setzen** (Zone.Identifier, ZoneId=3, damit SmartScreen warnt; macOS `com.apple.quarantine`), Bericht. WARNUNG → `Sauber/` markiert, Nutzer bestätigt. GEFUNDEN → `Quarantäne/` (umbenannt, nie öffnen). FEHLER (Engine fehlt) → bleibt in `Eingang/` gesperrt.
- Optional, Default aus: Hash-Abfrage bei einem Online-Scanner über Tor (nur der Hash, kein Upload).
- **Nie automatisch öffnen.** UI nennt die Grenzen: keine 0-Days, keine Dokumente, die nach dem Ausschalten beim Öffnen die echte IP verraten. Dagegen hilft die Firewall nur, solange die App läuft. Optionaler Sandbox-Schalter bleibt für später offen.

### 6.5 watchdog (Wächter)
Eigener Prozess, gegenseitiger Herzschlag mit `tvpnd` (fällt einer aus, schlägt der andere Alarm), Binärdatei per Hash geschützt.
- **Sekundentakt (lokal):** Prozesse (`tor`, Mullvad-Daemon, `tvpnd`), Ports nur Loopback, WFP-Filtersatz vollständig (Hash über die Filterliste), Adapterstatus, Standardroute über VPN-Adapter, IPv6 blockiert, Zahl der Block-Ereignisse.
- **30-Sekunden-Takt:** Tor `status/bootstrap-phase` und `status/circuit-established`, VPN-Status, Systemzeit plausibel (Tor braucht sie), MAC/Hostname wie gesetzt, Tor Browser hängt an Port A, keine Verbindungsversuche außerhalb der Allowlist.
- **Selten (zufällig, etwa alle 10 bis 15 Minuten):** externer Exit-Check über Tor, Versionscheck des Tor Browsers. Nie die echte IP abfragen.
- **Reaktion:** WARN (Log, UI) → DEGRADED (Komponente neu starten) → CRITICAL (`FAILCLOSED`, Alarm, Angebot Neustart oder Notfall-Reset).
- Der Wächter sendet nichts nach außen außer den oben genannten Checks.

### 6.6 Logging
`tracing` mit Build-Feature `dev-logging`. Dev-Logs: Zustandswechsel, Komponentenstatus, Fehlercodes, Timings. **Nie loggen:** URLs, Fenstertitel, Ziel-Domains/IPs, echte IP, volle MAC, Dateinamen (nur Hash-Präfix). Datei unter `%ProgramData%\TVPN\logs`, Rotation 5×5 MB, Export-Bündel mit Redaktion. Release-Build: standardmäßig aus, nur lokale anonyme Fehlerzähler, Logs beim Ausschalten löschen. **CI-Gate:** Test, der Logs per Regex nach IP, MAC und URL durchsucht und fehlschlägt.

### 6.7 ui-shell
Riesen-Button (Aus, Wird eingeschaltet, An, Fehler farbig), Statuszeile (Zustand, Modus, Land, Wächter), Tray-Icon mit Zustandsfarbe. Einstellungen: VPN an/aus, Land, Bridges, Block-Fallback (aus/Vorschlag/automatisch), MAC "Original zurück", Torrent-Block, Netz-Modus, Notfall-Reset. Immer sichtbares Banner bei Downgrade. Dialog beim Ausschalten mit Downloads. **Session-Zusammenfassung** nach dem Ausschalten (Downloads sauber/Quarantäne, Warnungen). **Panik-Taste** (Hotkey): Apps schließen, Netz zu, Teardown. Dunkel/Hell, keine Telemetrie, ehrliche Texte.

### 6.8 Installer, USB-Stick, macOS
- **Windows:** Installer (MSI/NSIS über Tauri-Bundler) nach Program Files (nur Admin schreibbar), installiert Dienst, Tor, Tools, optional ClamAV. Code-Signing-Zertifikat einplanen (sonst SmartScreen-Warnung). Hash-Prüfung der Bausteine bei der Installation und bei jedem Start.
- **USB-Kit:** `/Windows/TVPN-Setup-x.y.z.exe`, `/macOS/TVPN-x.y.z.dmg`, `SHA256SUMS` plus Signatur (minisign oder GPG), README mit Prüfanleitung.
- **macOS (Phase 7):** Apple Developer ID und Notarisierung. Firewall-Ansatz offen (NetworkExtension-Filter vs. `pf` plus Tor-TransPort), MAC-Wechsel unter neueren Versionen eingeschränkt, Berechtigungen für Fenstertitel (Bedienungshilfen/Bildschirmaufnahme). Alles **Spike S7**.
- **Updates:** signiert, nur über Port A, mit Rollback, nie automatisch ohne Hinweis.

## 7. Sicherheit des Projekts selbst

- **IPC:** Named Pipe mit ACL (nur der installierende Benutzer) plus Sitzungs-Token, feste Kommandoliste, Schema-Validierung, keine Shell-Strings, **keine Pfade, Programmnamen oder Argumente aus der UI**, Rate-Limit, Fuzz-Tests.
- **Prozessstart:** Der Dienst startet nur Programme aus dem Installationsordner, prüft vorher den Hash, kein PATH-Lookup.
- **Rechte:** so wenige wie möglich. Entscheidung (z. B. eigenes Dienstkonto statt `LocalSystem`) als ADR.
- **Secrets:** Mullvad-Kontonummer im Windows Credential Manager (DPAPI), nie in Logs, Config oder IPC-Ausgaben.
- **Lieferkette:** `Cargo.lock` und `package-lock.json` committen, `cargo audit` und `cargo deny`, `npm audit`, wenige Abhängigkeiten, Fremd-Binaries mit Version, SHA-256 und Signaturprüfung.
- **Telemetrie:** keine. **`unsafe`:** nur mit Begründung und Review durch qa-security.
- **Reproduzierbare Builds:** Stretch-Ziel.

## 8. Test- und Leak-Matrix (`tests/`, Rolle qa-security)

Alles in der Windows-11-VM mit Snapshots vor jedem Test. Als Gegenstelle dient ein **Canary-Endpunkt auf eigenem Server** (zeigt Quell-IP, Protokoll und Zeit jeder Verbindung, `docs/canary.md`).
- [ ] IP-Leak: Direktzugriff ohne Proxy scheitert; Zugriff über Tor zeigt Exit-IP, nie die echte
- [ ] DNS-, IPv6-, WebRTC- und UDP/QUIC-Leak
- [ ] Kill-Switch: `tor.exe` killen, dann VPN-Daemon killen → in beiden Fällen kein Paket nach außen
- [ ] Allowlist: nicht freigegebene App kommt nicht raus; Allowlist-App erreicht nur Tor-Ports
- [ ] MAC wechselt beim Einschalten und beim Ausschalten (`ipconfig /all`), pro Adapter
- [ ] Standby/Resume, WLAN-Wechsel, Adapter ziehen, VPN-Serverwechsel → `FAILCLOSED` oder sauberes Wiederherstellen
- [ ] Neustart und Absturz bei aktiven Filtern → Recovery klappt, kein Aussperren
- [ ] Manipulierte Binärdatei (Hash falsch) → Start verweigert
- [ ] IPC: ungültige, zu große, schnelle Nachrichten; fremder Benutzer kommt nicht an die Pipe
- [ ] Tor Browser unverändert (Prefs, Add-ons, Fenstergröße, Fingerprint-Vergleich)
- [ ] Portal-Zustand (simuliertes Captive Portal) funktioniert und schließt sich wieder
- [ ] EICAR erkannt, Doppel-Endung und Archiv-Bombe behandelt, MOTW gesetzt
- [ ] Log-Redaktion (kein IP/MAC/URL in Logs)
- [ ] Wächter löst bei jeder obigen Störung die richtige Stufe aus

## 9. Phasen und Meilensteine (Reihenfolge, kein Kalender)

**Phase 0: Fundament (Orchestrator)** → Gate G0
- [ ] opencode-Version prüfen, Agent-Dateien validieren
- [ ] Repo, `.gitignore`, CI-Skelett, `docs/THREAT_MODEL.md`, `docs/LICENSES.md`, `docs/canary.md`
- [ ] `contracts/ipc-v0.md` prüfen und ergänzen
- [ ] `tests/vm/README.md` und `run-in-vm.ps1` (Hyper-V PowerShell Direct oder VBoxManage), Snapshot-Revert
- [ ] Toolchain-Liste an den Menschen

**Phase 1: Spikes S1 bis S6 (risk-first)** → Gate G1: Jeder Spike entschieden, `DECISIONS.md` aktuell, Plan angepasst.

**Phase 2: Net-Core MVP** → Gate G2
- [ ] Zustandsautomat, IPC, Logging, WFP-Layer, Tor-Controller, VPN-Controller, Recovery-Tool, UI-Gerüst mit Button
- [ ] Meilenstein: Button an/aus in der VM, Leak-Tests der Phase grün

**Phase 3: Privacy und Netzwechsel** → Gate G3
- [ ] MAC (und Hostname je S3), voller Ein/Aus-Ablauf aus Abschnitt 5 ohne Browser, Portal-Zustand, Netzwechsel/Standby

**Phase 4: Launcher, Downloader, Scanner** → Gate G4
- [ ] Tor-Browser-Verifier und Start am System-Tor, aria2-Integration, Link-Übergabe, Scanner-Pipeline mit MOTW und Quarantäne, UI für Downloads
- [ ] Meilenstein: kompletter Download-Workflow, EICAR-Test grün

**Phase 5: Wächter, Fallback, Härtung** → Gate G5
- [ ] Wächter inkl. Fail-closed, Notfall und Panik-Taste, Block-Erkennung und Fallback-Browser (je S5), Sicherheits-Review, volle Matrix aus Abschnitt 8

**Phase 6: Windows-Installer und USB-Kit** → Gate G6
- [ ] Installer, Signierung, USB-Kit, Prüfanleitung. Meilenstein: frische VM, Installation vom Stick, Button funktioniert

**Phase 7: macOS-Port** → Gate G7 (Spike S7 zuerst)

**Phase 8: Abschluss-Audit (qa-security)** → Gate G8: unabhängiger Review, Threat Model aktualisiert, Restrisiken dokumentiert

**Phase 9 (optional): System-Modus** → Gate G9 (Spike S8 zuerst)

## 10. Zusammenarbeit

- **Worktrees:** Ein Worktree und ein Terminal-Tab pro Rolle (`tools/start-worktrees.ps1`). Branches `agent/<rolle>`.
- **Parallelität:** maximal 3 bis 4 Rollen gleichzeitig. Jede Rolle ändert nur ihre Ordner. `contracts/` ändert nur der Orchestrator, Änderungswünsche gehen in `docs/contract-requests.md`.
- **Board:** `docs/TASKS.md` ist die einzige Aufgabenliste. Der Orchestrator pflegt Status und Zuteilung, Rollen melden per Übergabe-Datei zurück.
- **Merge-Reihenfolge:** contracts → net → privacy → launcher/scanner → watchdog → ui → tests. Vor jedem Merge: rebase auf `main`, Tests grün, Leak-Test der Phase grün.
- **Aufräumen:** `tools/cleanup-worktrees.ps1`, Branches erst nach Merge löschen.

## 11. Risiken und Gegenmaßnahmen

| Risiko | Wahrscheinlichkeit | Maßnahme |
|---|---|---|
| Bug sperrt Nutzer aus dem Netz | Hoch | Persistente Filter mit eigener GUID, `tvpn-recover.exe`, Start-Recovery, nur VM-Tests |
| Konflikt zwischen Mullvad-Filtern und unseren | Mittel | Spike S1, eine Quelle der Wahrheit pro Regel |
| Tor Browser lässt sich nicht sauber an System-Tor hängen | Mittel | Spike S2, Plan B: eigenes Tor des Browsers hinter dem VPN |
| Hostname-Wechsel nur mit Neustart | Hoch | Spike S3, Plan A: einmalig neutralisieren |
| MAC-Wechsel vom Treiber nicht unterstützt | Mittel | Erkennen, melden, nur mit Bestätigung weiter |
| Fenstertitel-Heuristik übersieht Blocks | Hoch | Komfortfunktion, manuelle Taste immer da |
| URL aus dem Tor Browser nicht auslesbar | Mittel | Spike S5, Plan B: URL-Eingabe oder Zwischenablage |
| Scanner gibt falsche Sicherheit | Mittel | Grenzen in der UI, MOTW, Firewall-Allowlist, späterer Sandbox-Schalter |
| Öffentliches WLAN mit Portal blockiert alles | Hoch | Zustand PORTAL, Spike S6 |
| Fremd-Binaries kompromittiert, GPL-Fragen | Mittel | Version und Hash pinnen, Signaturen prüfen, `LICENSES.md` |
| Torrent über VPN wird für Anonymität gehalten | Mittel | Default `blocked`, Banner `vpn_route`, ehrliche Texte: Peers und Anbieter sehen die VPN-IP |
| Dienst als privilegierter Prozess angreifbar | Mittel | Minimale IPC, Pipe-ACL, Hash-Check vor Start, Review |
| macOS deutlich aufwendiger | Hoch | Spike S7, Windows zuerst fertig |

## 12. Default-Entscheidungen (Mensch kann widersprechen)

Rust + Tauri · Fokus-Modus (Blocken statt Umleiten) in v1, TUN erst später · Reihenfolge VPN → Tor · Mullvad (Kontonummer, anonyme Zahlung) · aria2 als Standard-Downloader, FDM als Adapter · Windows Defender plus ClamAV · Mullvad Browser als Fallback-Browser · Hostname einmalig neutralisieren (Plan A) · Torrents standardmäßig blockiert, VPN-Bulk-Route nur auf ausdrücklichen Wunsch · Doku Deutsch, Code und Commits Englisch.

## 13. Nächste Schritte (jetzt)

1. `opencode --version` prüfen, Agent-Dateien gegen die Docs der Version abgleichen.
2. Repo anlegen (falls noch nicht), Kit committen, Worktree-Skripte testen (`-NoLaunch`).
3. `THREAT_MODEL.md`, `LICENSES.md`, `canary.md` schreiben, `contracts/ipc-v0.md` prüfen.
4. VM-Runner schreiben, dem Menschen Toolchain-Liste und VM-Anleitung geben.
5. **Stoppen und Abnahme von Phase 0 (G0) abwarten.**
