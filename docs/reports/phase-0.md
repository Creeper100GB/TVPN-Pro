# Phase 0: Fundament (Gate G0)

Stand: 21. November 2026. Auftrag: `PLAN.md` Abschnitt 9, Aufgaben P0-01 bis P0-07 aus `docs/TASKS.md`. Zusätzlich auf Wunsch des Menschen vorgezogen: die UI-Shell (P2-07) als Mock. Dieses Dokument behauptet nichts, was nicht geprüft wurde.

## 1. Ergebnis

| ID | Ergebnis | Evidenz |
|---|---|---|
| P0-01 | opencode `v2.3.2` geprüft, acht Agenten-Dateien auf natives V2-Frontmatter umgestellt | `opencode debug agents` listet alle acht mit `mode` und 26 Regeln, ohne Fehler |
| P0-02 | Repo-Gerüst, `.gitignore`, `.gitattributes`, `justfile`, Rust-Workspace, CI-Skelett, Lizenzen-Gerüst | `just verify` grün (fmt, clippy, tests, pscheck, Redaktions-Gate) |
| P0-03 | `docs/THREAT_MODEL.md` | muss vom Menschen gelesen werden (G0) |
| P0-04 | `contracts/ipc-v0.md` erweitert | Lückenliste in Abschnitt 2 |
| P0-05 | `tests/vm/README.md`, `run-in-vm.ps1`, `smoke.ps1` | Syntax geprüft; **nicht** in einer VM gelaufen (keine VM vorhanden) |
| P0-06 | `docs/canary.md`, `tests/canary/canary.py` | `python -m py_compile` ok; nicht auf einem Server deployt |
| P0-07 | Toolchain installiert und verifiziert | Versionen in Abschnitt 6 |
| UI (vorgezogen) | `ui/`: macOS-artige Shell, 11 Zustände, Einstellungen, Sitzungs-Zusammenfassung, Tester-Infos in den Einstellungen | `svelte-check` 0 Fehler, Build grün, Playwright-Prüfung grün, Screenshots in `docs/reports/screens/` |

**Nicht bewiesen:** der GitHub-CI-Lauf (nichts gepusht), der VM-Runner (keine VM), das Tauri-Fenster auf einem echten Bildschirm (keine GUI-Prüfung in dieser Sitzung).

## 2. Gefundene Lücken im IPC-Vertrag

Der Entwurf war nicht implementierbar. Ergänzt wurden:

1. **Transport:** Pipe-Name, Framing (NDJSON), UTF-8, 64-KiB-Limit, Verbindungslimit.
2. **Authentisierung:** Sitzungs-Token, Ablage, ACL, Rotation, ehrliche Grenze (gleicher Benutzer = gleiche Rechte).
3. **Envelope:** `v`, `id`, `cmd`/`evt`, `seq`, Antwort- und Fehlerform, `detail` als Maschinen-Code.
4. **Rate-Limit:** 20/s, Burst 40, Verbindung wird geschlossen.
5. **Kommandos:** vollständige Tabelle inklusive `hello`, `export_logs`, `get_session_summary`; Idempotenz-Regel; Verbot von Pfaden, Programmnamen und Argumenten.
6. **Ereignisse:** Typen, Payloads, Best-Effort-Zustellung, Puffergröße, `resync_required`, `get_status` als Wahrheit.
7. **Status-Payload:** Felder, Enum-Werte, keine IP/Domain.
8. **Settings-Whitelist:** zehn Schlüssel mit Typ, Default und Wertebereich; unbekannter Schlüssel = Fehler.
9. **Zustandsautomat:** fehlende Übergänge ergänzt (Abbruch aus `PREFLIGHT`/`ARMING`, Portal-Timeout, `TEARDOWN → RECOVERY`, `OFF → RECOVERY`) plus Invarianten (UI-Tod ist kein Sicherheitsereignis; `panic` endet nie in `RECOVERY`).
10. **Fehlercodes:** 30 konkrete Codes statt bloßer Präfixe.
11. **URL-Regeln:** HTTPS, Port 443, kein IP-Literal, keine lokalen Namen, keine Userinfo, 8 KiB.
12. **Versionierung:** additive Änderungen in `v0`, Breaking Change = neuer Pipe-Name.

Offen bleiben: Dienstkonto (D11), Wächter-Kanal, JSON-Schema-Dateien, Signatur des Log-Exports, S1-Einfluss auf den Lockdown.

## 3. UI-Shell (vorgezogen, Mock)

Auf Wunsch des Menschen vor Phase 1 gebaut, ausdrücklich gegen Mock-Daten. Kein IPC, keine Systemwirkung.

- **Stack:** Svelte 5.59, SvelteKit 3.4, Vite 8.3, Tailwind 4.3, TypeScript 6.0, Lucide 1.0, Inter Variable (lokal gebündelt), Playwright 1.65 (nur Dev).
- **Design:** macOS-Anmutung, keine eckigen Windows-Controls. 30-px-Fensterradius, Blur, Haarlinien, Inter mit negativer Laufweite, iOS-Schalter, Pill-Buttons, ein großer Power-Button mit Fortschrittsring.
- **Zustände:** alle elf aus dem Vertrag, jeder mit eigener Farbe, Text und Aktion. Dazu Banner, Fehlercode, Statuszeile.
- **Dialoge:** Einstellungen (VPN, Land, Bridges, MAC-Modus, Portal-Zeitfenster, Block-Fallback, Torrent-Block, Hash-Abfrage, Erscheinungsbild, Panik) und Sitzungs-Zusammenfassung.
- **A11y:** `role="switch"`, `aria-pressed`, `aria-live` für den Zustand, Fokusringe, `prefers-reduced-motion`, 44-px-Ziele.
- **Prüfung:** `scripts/shots.mjs` läuft alle Zustände in Hell und Dunkel durch, prüft Konsole, horizontales und vertikales Überlaufen, Fensterradius und das schmale Layout (520 × 860). Ergebnis: `UI check passed`, 0 Konsolenfehler.

Screenshots: `docs/reports/screens/light-state-ready.png`, `dark-state-ready.png`, `light-settings.png`, `light-narrow.png`.

Was die UI **nicht** ist: keine echte Verbindung, keine Persistenz der Einstellungen (nur Theme), keine Tray-Integration, kein Panik-Hotkey.

## 4. Vorschlag: Reihenfolge der Spikes

Risk-first heißt: der Spike mit dem größten Architekturrisiko zuerst, und zwar der, der andere blockiert.

| Welle | Rolle(n) | Spike | Warum jetzt |
|---|---|---|---|
| A | net-core | **S1** WFP-Filter neben Mullvad-Lockdown | Entscheidet D12 und den ganzen Firewall-Bau (P2-03). Schlimmster Fehlerfall: Aussperren oder widersprüchliche Filter. Nichts am Netz-Layer ist ohne S1 baubar. |
| A | privacy-shield | **S3** Hostname ohne Neustart | Unabhängig, billig, entscheidet D8 und die Reihenfolge in `PREFLIGHT`/`ARMING`. Blockiert niemanden. |
| B | launcher | **S2** Tor Browser am System-Tor | Entscheidet, ob der Launcher überhaupt so gebaut wird. Bei Plan B ändern sich P4-02 und S5. Größtes Produktrisiko nach S1. |
| B | scanner | **S4a** ClamAV-RAM/Startzeit, Defender-CLI | Vom Netz-Stack unabhängig, im selben VM-Zyklus messbar. Entscheidet, ob ClamAV überhaupt mitgeliefert wird. |
| B | net-core | **S6** Captive Portal unter Default-Deny | Braucht S1-Ergebnis (welche Permit-Regel ist erlaubt). |
| C | launcher | **S4b** aria2 über Port B, Resume; **S5** Block-Titel, URL, Fallback-Browser | Braucht S2 (laufender Tor Browser) und die Firewall-Permit-Regel aus S1/S6. |

Maximal zwei bis drei Rollen gleichzeitig. S1 und S3 starten direkt nach G0; S2 und S4a, sobald Wave A durch ist; S6 und S4b/S5 in Wave C. Kein Spike wartet auf einen anderen außer denen, die oben explizit genannt sind.

## 5. Vorschlag: erste Worktree-Rollen

```powershell
powershell -ExecutionPolicy Bypass -File tools\start-worktrees.ps1 -Roles net-core,privacy-shield
```

- **net-core** beginnt mit S1, weil es den längsten Vorlauf hat und P2-03 daran hängt.
- **privacy-shield** beginnt mit S3, weil es unabhängig ist und schnell ein Ergebnis liefert.
- Danach **launcher** (S2) und **scanner** (S4a). `ui-shell`, `watchdog` und `qa-security` kommen erst, wenn es etwas zu prüfen gibt (Phase 2/5).

## 6. Toolchain

### Installiert und geprüft

| Werkzeug | Version | Zweck |
|---|---|---|
| rustup | 1.29.3 | Toolchain-Verwaltung |
| Rust (MSVC) | 1.100.0 | Dienst, Tools, Tests |
| Visual Studio Build Tools 2022 | 17.14.37906.3, Workload VCTools | MSVC-Linker |
| Windows SDK | 10.0.26100.0 | Header und Libs |
| PowerShell | 5.1 (System) + 7.7 (winget) | Skripte, VM-Runner |
| Node.js | 26.13.0 LTS („Lithium") | UI-Build |
| npm / bun | 11.20.0 / 1.3.14 | Pakete |
| git / gh | 2.56.1 / 2.103.3 | Versionierung, CI |
| just | 1.59.0 | Task-Runner |
| Tauri CLI | 2.12.6 | UI-Shell |
| Python / uv | 3.12 / vorhanden | Canary, Icon-Generator |
| Playwright | 1.65.1 (Chromium 1255) | Visuelle Prüfung, nur Dev |

**Wichtig für Windows:** Ein normales Terminal hat die MSVC-Umgebung nicht. `just test` und `just build` laden sie über `tools/with-msvc.ps1` (vcvars64 via vswhere). `cargo fmt` und `cargo clippy` brauchen sie nicht.

### Was der Mensch selbst tun muss

1. **Nichts an der Toolchain.** Alles Nötige ist installiert und geprüft.
2. **Test-VM:** Hyper-V, Windows 11, lokales Admin-Konto, Checkpoint `sauber` (Anleitung: `tests/vm/README.md`).
3. **Mullvad-Konto** (nur Kontonummer) und die Mullvad-App in der VM.
4. **Optional Canary:** kleiner VPS mit IPv4 und IPv6, Domain optional (`docs/canary.md`).
5. **Später, mit Kosten:** Code-Signing-Zertifikat für den Windows-Installer (Phase 6, eigenes Gate).

Installer-Werkzeuge (NSIS, WiX) lädt der Tauri-Bundler zur Build-Zeit; sie werden nicht vorinstalliert. Wenn Offline-Builds nötig werden, werden sie mit Version und Hash gepinnt.

## 7. Offene Risiken

| Risiko | Schwere | Umgang |
|---|---|---|
| P0-05 ist nicht in einer VM gelaufen | mittel | G0-Checkliste: VM anlegen, `smoke.ps1` laufen lassen, Ergebnis nachtragen |
| GitHub-CI ist unbewiesen | niedrig | nach Freigabe pushen und Lauf prüfen; lokal ist alles grün |
| Tauri-Fenster wurde nie auf einem Bildschirm geöffnet | mittel | `cargo check` grün, aber Layout/CSP im WebView2 ungeprüft. Erster Test in Phase 2 |
| CSP `script-src 'self'` und ein Inline-Script von SvelteKit | mittel | Tauri hasht Inline-Scripts normalerweise selbst; wenn das Fenster leer bleibt, ist das die erste Stelle |
| `link.exe` aus Git (`usr\bin`) verschattet den MSVC-Linker | mittel | umgangen mit `tools/with-msvc.ps1`; globale PATH-Bereinigung ist eine Entscheidung des Menschen |
| Rust 1.100 ist sehr neu, Tauri/WebView2-Kompatibilität ungeprüft | mittel | erster `tauri build` in Phase 2; `rust-version = 1.85` bleibt Untergrenze |
| UI ist Mock, keine IPC | niedrig | bewusst; Vertrag v0 ist die Grundlage für Phase 2 |
| Vertrag v0 ist gegen keine Implementierung getestet | mittel | Phase 2 schreibt den IPC-Server dagegen; Abweichungen in `docs/contract-requests.md` |
| Agenten-Glob-Regeln sind Komfort, keine Sandbox | mittel | Durchsetzung bleibt Review und VM-Zwang |
| Eigene Lizenz offen | niedrig | vor dem ersten öffentlichen Push entscheiden (D22) |

## 8. Was der Mensch tun muss (G0-Checkliste)

1. `docs/THREAT_MODEL.md` lesen. Widerspruch oder Ergänzung? Dann ändern wir es vor Phase 1.
2. `docs/DECISIONS.md` D13 bis D27 lesen, besonders **D22 (eigene Lizenz)**.
3. UI ansehen: `cd ui; bun run dev` oder die Screenshots in `docs/reports/screens/`.
4. Test-VM anlegen, Checkpoint `sauber` setzen, dann:
   ```powershell
   powershell -ExecutionPolicy Bypass -File tests\vm\run-in-vm.ps1 -Script tests\vm\smoke.ps1
   ```
5. Entscheiden, ob gepusht werden darf (GitHub-CI beweisen).
6. Optional: Canary-Server aufsetzen.
7. **G0 geben** oder Änderungen verlangen.

Nach G0 startet Wave A mit `net-core` und `privacy-shield`, siehe Abschnitt 5.

## 9. Aufräumen

- Keine Worktrees angelegt, keine Branches gelöscht, nichts gepusht.
- `target/`, `ui/.artifacts/`, `ui/build/`, `ui/.svelte-kit/` sind ignoriert; `__pycache__` entfernt; temporäre Debug-Skripte gelöscht.
- Der Kit-Ordner `files.zip` und die `.opencode/goals`-Zustände des Sitzungsordners gehören nicht ins Repo und bleiben unangetastet.
