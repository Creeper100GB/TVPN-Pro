# Aufgabenboard (nur der Orchestrator ändert Status und Zuteilung)

Status: `offen` · `läuft` · `Review` · `fertig` · `blockiert`. Rollen melden per `docs/handoffs/<id>.md`.

## Phase 0: Fundament (Gate G0)
| ID | Rolle | Aufgabe | Abnahme | Status |
|---|---|---|---|---|
| P0-01 | orchestrator | opencode-Version prüfen, Agent-Dateien gegen Docs abgleichen | Alle Agenten starten ohne Fehler | offen |
| P0-02 | orchestrator | Repo, `.gitignore`, CI-Skelett, `docs/LICENSES.md` | CI läuft grün (Lint, Unit-Tests, Log-Redaktions-Gate-Stub) | offen |
| P0-03 | orchestrator | `docs/THREAT_MODEL.md` aus PLAN Abschnitt 2 | Mensch hat gelesen | offen |
| P0-04 | orchestrator | `contracts/ipc-v0.md` prüfen und ergänzen | Keine offenen Lücken | offen |
| P0-05 | orchestrator | `tests/vm/README.md` und `run-in-vm.ps1` (Hyper-V oder VBoxManage), Snapshot-Revert | Testskript läuft in der VM und stellt Snapshot wieder her | offen |
| P0-06 | orchestrator | `docs/canary.md`: Anleitung für Canary-Endpunkt auf eigenem Server | Mensch kann ihn aufsetzen | offen |
| P0-07 | orchestrator | Toolchain-Liste (Rust, Node LTS, Tauri-CLI, Installer-Tools) an den Menschen | Mensch bestätigt Installation | offen |

## Phase 1: Spikes (Gate G1), Details in `docs/SPIKES.md`
| ID | Rolle | Aufgabe | Abnahme | Status |
|---|---|---|---|---|
| S1 | net-core | WFP-Filter neben Mullvad-Lockdown | Entscheidung in DECISIONS.md | offen |
| S2 | launcher | Tor Browser am System-Tor | Entscheidung in DECISIONS.md | offen |
| S3 | privacy-shield | Hostname ohne Neustart | Entscheidung in DECISIONS.md | offen |
| S4 | launcher, scanner | aria2/FDM über Tor, Scanner-Engines | Entscheidung in DECISIONS.md | offen |
| S5 | launcher | Block-Titel, URL auslesen, Fallback-Browser | Entscheidung in DECISIONS.md | offen |
| S6 | net-core | Captive Portal unter Default-Deny | Entscheidung in DECISIONS.md | offen |

## Phase 2: Net-Core MVP (Gate G2)
| ID | Rolle | Aufgabe | Abnahme | Status |
|---|---|---|---|---|
| P2-01 | orchestrator | Zustandsautomat und IPC-Server | Zustandsübergänge laut Vertrag, Unit-Tests | offen |
| P2-02 | orchestrator | Logging mit Redaktions-Gate | Gate-Test in CI grün | offen |
| P2-03 | net-core | WFP-Layer (arm, permit_app, verify, disarm) | Allowlist-Tests in der VM grün | offen |
| P2-04 | net-core | Tor-Controller | Bootstrap und Circuit-Status zuverlässig | offen |
| P2-05 | net-core | VPN-Controller (Mullvad) | Verbinden, Land, Status, Fehlerfälle | offen |
| P2-06 | net-core | `tvpn-recover.exe` | Entfernt nur eigene Filter, läuft ohne Dienst | offen |
| P2-07 | ui-shell | UI-Gerüst mit Button gegen IPC-Mock | Alle Zustände darstellbar | offen |
| P2-08 | qa-security | Leak-Tests Phase 2 (Matrix Abschnitt 8) | Alle relevanten Punkte grün | offen |

## Phase 3: Privacy und Netzwechsel (Gate G3)
| ID | Rolle | Aufgabe | Abnahme | Status |
|---|---|---|---|---|
| P3-01 | privacy-shield | MAC-Wechsel pro Adapter, Original merken | `ipconfig /all` vorher/nachher, Wiederherstellung | offen |
| P3-02 | privacy-shield | Hostname laut S3 | wie in S3 entschieden | offen |
| P3-03 | orchestrator | Voller Ein/Aus-Ablauf (ohne Browser) | Ablauf Abschnitt 5 in der VM | offen |
| P3-04 | net-core | Zustand PORTAL | simuliertes Portal klappt | offen |
| P3-05 | net-core, watchdog | Netzwechsel und Standby | Matrix-Punkte grün | offen |
| P3-06 | qa-security | Tests Phase 3 | Matrix-Punkte grün | offen |

## Phase 4: Launcher, Downloader, Scanner (Gate G4)
| ID | Rolle | Aufgabe | Abnahme | Status |
|---|---|---|---|---|
| P4-01 | launcher | Tor-Browser-Verifier (Quelle, GPG, Mindestversion) | Manipulierte Datei wird abgelehnt | offen |
| P4-02 | launcher | Start am System-Tor | Fingerprint unverändert | offen |
| P4-03 | launcher | aria2-Integration (RPC, Proxy, Resume) | Download über Port B, HTTP-Link abgelehnt | offen |
| P4-04 | launcher | Link-Übergabe (Feld, optional Zwischenablage) | Link landet im Downloader | offen |
| P4-05 | scanner | Pipeline, Engines, Regeln | EICAR erkannt, Archiv-Bombe behandelt | offen |
| P4-06 | scanner | MOTW und Quarantäne | Zone.Identifier gesetzt, Quarantäne unlesbar | offen |
| P4-07 | ui-shell | Downloads und Scan-Ergebnisse | Alle Urteile darstellbar | offen |
| P4-08 | qa-security | Tests Phase 4 | Matrix-Punkte grün | offen |

## Phase 5: Wächter, Fallback, Härtung (Gate G5)
| ID | Rolle | Aufgabe | Abnahme | Status |
|---|---|---|---|---|
| P5-01 | watchdog | Wächter mit allen Takten und Eskalation | Jede Störung löst die richtige Stufe aus | offen |
| P5-02 | launcher | Block-Erkennung (je S5) | Muster-Tests, Challenge löst nichts aus | offen |
| P5-03 | launcher, net-core | Fallback-Browser und Firewall-Permit | Nur über VPN-Adapter erreichbar | offen |
| P5-04 | ui-shell, orchestrator | Panik-Taste, Notfall-Reset in der UI, Session-Zusammenfassung | Funktioniert in der VM | offen |
| P5-05 | qa-security | Sicherheits-Review und volle Matrix | Alle Punkte grün | offen |

## Phase 6 bis 9
| ID | Rolle | Aufgabe | Gate |
|---|---|---|---|
| P6-xx | orchestrator, qa-security | Windows-Installer, Signierung, USB-Kit, Prüfanleitung | G6 |
| P7-xx | alle (nach S7) | macOS-Port hinter den Traits, DMG, Notarisierung | G7 |
| P8-xx | qa-security | Unabhängiges Abschluss-Audit | G8 |
| P9-xx | net-core (nach S8) | Optionaler System-Modus (TUN) | G9 |
