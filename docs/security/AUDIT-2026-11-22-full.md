# TVPN Pro – Vollständiges Audit (zweiter Durchgang)

Stand: 22. November 2026  
Repository: TVPN-Pro (lokaler Checkout)  
Commit: `85e0cb0` (`docs: state torrent and winget status`)  
Art: unabhängige Code-, Konfigurations-, Test- und Doku-Prüfung. Keine Codeänderungen, keine Firewall-/Dienst-/VM-Eingriffe.

## 0. Bewertung in einem Satz

Die App hat einen ehrlichen Alpha-Rahmen und einzelne gute Härtungen, aber **die sicherheitskritische Kette ist noch nicht vertrauenswürdig**: mehrere Pfade können den tatsächlichen Netzschutz falsch darstellen, der Tor-Permit ist nicht an das VPN-Interface gebunden, die Desktop-Sitzungszusammenfassung ist durch eine Whitelist-Lücke faktisch defekt, und zentrale UI-Einstellungen werden entweder gar nicht gespeichert oder vom Dienst ignoriert.

Schweregrade:

- **P0 – kritisch:** kann Schutz unbemerkt aushebeln, Identität verraten oder eine falsche Sicherheitsanzeige erzeugen.
- **P1 – hoch:** ernster Funktions-, Sicherheits- oder Vertrauensfehler; vor „fertig“ beheben.
- **P2 – mittel:** relevanter Funktions-, Persistenz-, Test- oder Härtungsfehler.
- **P3 – niedrig:** Komfort, Defense-in-depth, kleinere Korrektheit oder fehlende Doku.

## 1. Reproduzierte Befunde (frische Evidenz)

| ID | Befund | Evidenz |
|---|---|---|
| R1 | `just verify` ist grün: 91 Service-Tests bestanden, 1 WFP-VM-Test ignoriert; Clippy, PowerShell-Parser und Redaktions-Self-Test grün. | `just verify` endet mit `verify: all checks passed`. |
| R2 | UI-Typprüfung und Produktionsbuild sind grün. | `bun run check` → 0 Fehler/0 Warnungen; `bun run build` → erfolgreich. |
| R3 | Playwright-E2E ist grün, prüft aber überwiegend den Mock. | `just e2e` → 4/4 bestanden. |
| R4 | Die visuelle UI-Prüfung ist **kaputt**. | `bun run shots` läuft nach 20 s in einen Timeout: `waiting for getByRole('heading', { name: 'Geschützt' })`. |
| R5 | Die eigenständige Tauri-Crate ist nicht formatiert und wird von `just verify`/CI nicht geprüft. | `cargo fmt --manifest-path ui/src-tauri/Cargo.toml -- --check` → Diff in `ui/src-tauri/src/download.rs:541`; Root-`cargo fmt` ist grün. |
| R6 | Tauri-Crate besteht Clippy und 4 Unit-Tests; diese decken aber nur Dateinamen und einen einfachen HTTP-Download ab. | `cargo clippy --manifest-path ui/src-tauri/Cargo.toml --all-targets -- -D warnings` → 0; `cargo test --manifest-path ...` → 4/4. |
| R7 | `bun audit` meldet aktuell keine bekannten npm-Schwachstellen; ein Rust-Audit fehlt. | `bun audit` → `No vulnerabilities found`; `cargo audit`/`cargo deny` sind nicht installiert und nicht in CI. |
| R8 | Der Dienst akzeptiert ungültige Kommandos und Länder. | Harness: `country_de` → `E_IPC_BAD_SCHEMA`; `country_DE` → ok; `country_ZZ` → ok. |
| R9 | Der Dienst ignoriert `enable`-/`disable`-Argumente und unbekannte Felder in `args`. | Harness: `enable` mit `net_mode=system`, `protect_mode=direct` und Extra-Feld → `ok=true`, `DEGRADED`; `disable` mit `downloads_policy=invalid` und Extra-Feld → `ok=true`. |
| R10 | Der Dienst meldet im Tor-only-Modus weiterhin `protect_mode=tor_vpn`; Download-Zähler sind hartkodiert 0. | Harness: `status_after_tor_only: state=DEGRADED protect_mode=tor_vpn`; `Service::status` setzt `downloads: {active:0, quarantined:0}`. |
| R11 | Nach `panic` gibt es keine Sitzungszusammenfassung. | Harness: `summary_after_panic: null`. |
| R12 | Der Mock meldet bei `mac_mode=original` trotzdem `mac_randomized=true`. | Harness: `mock_mac_original_reports_randomized: true`. |
| R13 | Der URL-Validator lässt eine hexadezimale Loopback-IP durch, die der HTTP-Client zu `127.0.0.1` normalisiert. | Harness: `https://0x7f.0x0.0x0.0x1/` → `validation=Ok`, `normalized_host=127.0.0.1`; `https://localhost.localdomain/` → `Ok`. |
| R14 | Die Test-VM `tvpn-test` existiert, läuft und hat den Checkpoint `sauber`; sie wurde in diesem Audit **nicht** zurückgesetzt, gestoppt oder verändert. | `Get-VM` → `Running`; `Get-VMSnapshot` → `sauber`, 22.11.2026 15:44. |
| R15 | Für das zweite Terminal ist das Modell verfügbar und authentifiziert. | `pi --list-models deepseek-v4` zeigt `mistral deepseek-v4-1-flash-1017`; `pi auth check --provider mistral --model deepseek-v4-1-flash-1017` → `ready`. |

## 2. Dienst, Zustandsautomat und Sitzung

| ID | Schwere | Befund |
|---|---|---|
| S1 | P0 | **`get_session_summary` fehlt in der Tauri-Whitelist.** `ui/src-tauri/src/service.rs:26-34` erlaubt nur `enable`, `disable`, `panic`, `recover`, `get_status`, `get_settings`, `set_settings`. `ui/src/lib/mock.svelte.ts:489` ruft nach `disable` `serviceCommand('get_session_summary')` auf. Der Tauri-Befehl lehnt das mit `E_IPC_UNKNOWN_COMMAND` ab; die UI fällt in den Catch-Zweig, zeigt keine Zusammenfassung und setzt den Zustand auf `FAILCLOSED`, bevor der Refresh wieder `OFF` liefert. |
| S2 | P1 | **`enable` validiert seine Argumente nicht.** `Service::enable` (`service/tvpnd/src/lib.rs:160-233`) liest `args` nie. Ein Client kann `net_mode=system`, `protect_mode=direct` oder beliebige Extra-Felder senden; der Dienst arbeitet trotzdem mit den gespeicherten Einstellungen. Der Vertrag verlangt `E_IPC_BAD_SCHEMA` bei Abweichung. |
| S3 | P1 | **`disable` ignoriert `downloads_policy`.** `Service::disable` (`lib.rs:235-270`) wertet das Argument nicht aus; `wait`/`pause`/`abort` sind wirkungslos. Die UI sendet immer `wait`, bricht Downloads aber vorher per `cancelAllDownloads` ab. |
| S4 | P1 | **`hello` ignoriert `client` und `client_version`.** `Service::hello` (`lib.rs:136-158`) prüft nur das Token. Der Vertrag verlangt beide Felder; ein fremder Client kann sich als UI ausgeben. |
| S5 | P2 | **Unbekannte Felder in `args` werden akzeptiert.** `RawRequest` verbietet nur unbekannte Top-Level-Felder; `args` ist `Value`. `set_settings` mit `{"settings": {...}, "unexpected": "accepted"}` ist erfolgreich. Der Vertrag verlangt Schema-Ablehnung. |
| S6 | P1 | **`panic` verletzt die Vertragsinvariante.** Bei einem Fehler in `shutdown`/`disarm` transitioniert `panic` über `TeardownIncomplete` nach `RECOVERY` (`lib.rs:272-297`). Der Vertrag sagt: Panik endet immer in `TEARDOWN`, nie in `RECOVERY`. |
| S7 | P1 | **`panic` erzeugt keine Sitzungszusammenfassung.** Anders als `disable` setzt `panic` kein `last_summary`; `get_session_summary` liefert danach `null`. Die UI bietet nach Panik keine Zusammenfassung. |
| S8 | P1 | **`fail()` räumt teilgestartete Komponenten nicht auf.** Wenn `tor_start`, `tor_wait` oder `leak_check` fehlschlägt, wechselt der Dienst nach `FAILCLOSED`, lässt aber den Tor-Prozess und den Tor-Permit aktiv. Tor kann weiterhin nach außen; die UI behauptet „Netz gesperrt“. |
| S9 | P1 | **Der Watchdog erkennt entfernte Filter nicht.** `RealBackend::status` setzt `firewall_armed: self.armed || filters > 0` (`backend.rs:591`). Wird der Filtersatz extern gelöscht, bleibt `self.armed=true`; `watchdog_tick` prüft nur diesen Bool und geht nicht auf `FAILCLOSED`. |
| S10 | P1 | **`disarm()` ignoriert Löschfehler.** `WfpEngine::disarm` (`service/net/src/wfp.rs:301-306`) ruft `delete_owned`, das alle `FwpmFilterDeleteByKey0`-Fehler mit `let _ =` verwirft; `disarm` gibt immer `Ok(())` zurück. `disable`/`panic` melden dann `OFF`, obwohl Filter bleiben können. |
| S11 | P2 | **`status` hardcodet `protect_mode` und Download-Zähler.** `lib.rs:380-435` setzt `net_mode: "focus"`, `protect_mode: "tor_vpn"` und `downloads: {0,0}` unabhängig von Einstellungen/Backend. |
| S12 | P2 | **Sitzungsstart und -zähler werden nicht pro Sitzung zurückgesetzt.** `started_at`, `session_id` und `failclosed_count` bleiben über mehrere Enable/Disable-Zyklen erhalten. Eine zweite Sitzung erbt die Zusammenfassung der ersten. |
| S13 | P1 | **Desktop-Sitzungsdauer ist faktisch null.** `disable()` ruft zuerst `setState('TEARDOWN')`, das `app.since = Date.now()` setzt; danach wird `app.lastSummary.startedAt = app.since` verwendet (`ui/src/lib/mock.svelte.ts:481-551`). Die Dauer ist damit die Zeit seit Beginn des Ausschaltens, nicht die Sitzungsdauer. |
| S14 | P2 | **`failclosed_count` zählt kumulativ und wird nicht pro Sitzung geführt.** Die Zusammenfassung kann Notfälle aus früheren Sitzungen enthalten. |
| S15 | P2 | **`watchdog_tick` ruft `backend.status()` doppelt pro `get_status` auf.** `Service::status` ruft zuerst `watchdog_tick()` (das `backend.status()` aufruft) und danach erneut `backend.status()` (`lib.rs:362-382`). Bei jedem UI-Poll entstehen zwei Mullvad-CLI-Aufrufe, zwei Tor-Control-Abfragen und zwei WFP-Verify-Läufe. |
| S16 | P2 | **`DEGRADED -> READY` wird nie erreicht.** `watchdog_tick` behandelt nur `Critical`; `WatchdogWarn`/`WatchdogHealed` werden nicht ausgewertet. Ein einmal degradierter Zustand bleibt bis zum Ausschalten. |
| S17 | P2 | **`watchdog.last_check_at` ist immer `now()`.** Auch wenn `watchdog_tick` vorzeitig zurückkehrt, meldet der Status eine frische Prüfung. |
| S18 | P3 | **`Service::with_backend_and_path` setzt `RECOVERY` direkt**, ohne `transition`/`failclosed_count`; das ist funktional okay, umgeht aber die Zustandslogik und Ereignisse. |

## 3. WFP / Firewall

| ID | Schwere | Befund |
|---|---|---|
| W1 | P0 | **Loopback ist global für alle Prozesse erlaubt.** `install_in_transaction` legt einen Permit mit `FWPM_CONDITION_FLAGS=IS_LOOPBACK` ohne App-ID/Port an (`service/net/src/wfp.rs:166-190`). Jede lokale Malware darf damit den Tor-SOCKS-Port 9150 und den Control-Port 9051 erreichen. Die UI-Aussage „nur Tor und freigegebene Apps“ ist falsch. |
| W2 | P0 | **Der Tor-Permit ist nur App-ID-basiert, nicht interface-/quell-IP-gebunden.** `permit_app` erzeugt Permits auf allen vier ALE-Layern ohne Interface-Bedingung (`wfp.rs:202-250`). Fällt das VPN aus, kann `tor.exe` direkt über den physischen Adapter gehen. Das ist die zentrale S1-Lücke. |
| W3 | P1 | **`verify()` zählt beliebige eigene Filter, nicht den erwarteten Satz.** `leak_check` akzeptiert `filters >= 8` (`backend.rs:497-517`); 8 App-Permits ohne Block-/Loopback-Filter würden die Prüfung bestehen. |
| W4 | P1 | **`disarm` ist nicht verifiziert.** Nach dem Löschen wird `verify()` nicht geprüft; `delete_owned` ignoriert Fehler. Ein fehlgeschlagenes Löschen bleibt unsichtbar. |
| W5 | P1 | **Nur ALE-Auth-Layer sind abgedeckt.** `LAYERS` enthält ausschließlich `ALE_AUTH_CONNECT`/`ALE_AUTH_RECV_ACCEPT` v4/v6 (`wfp.rs:48-53`). ICMP, UDP/DNS, Raw-Sockets und IPv6-Sonderfälle sind nicht Teil der Leak-Matrix. |
| W6 | P1 | **`permit_app` hat keinen Rollback.** Scheitert Layer 3 von 4, bleiben die ersten Permits installiert; `self.permitted` wird nicht erhöht. Ein erneuter Versuch kann an `FWP_E_ALREADY_EXISTS` scheitern. |
| W7 | P2 | **`arm_default_deny` löscht zuerst alle eigenen Filter, ohne Fehler zu prüfen.** Bleiben Reste, kann der anschließende `FwpmFilterAdd0` mit „already exists“ scheitern; der Dienst geht in `FAILCLOSED`, obwohl ein sauberer Neustart möglich wäre. |
| W8 | P2 | **Kein DHCP-/Captive-Portal-Permit.** Default-Deny blockt DHCP/DNS; ein Lease-Ablauf oder Netzwechsel kann die Verbindung verlieren. PORTAL ist nicht implementiert. |
| W9 | P2 | **Der statische WFP-Test ist veraltet.** `service/net/src/bin/wfp-smoke.rs:18-27` erwartet, dass `Drop` die Filter entfernt. `WfpEngine::drop` entfernt sie seit dem Fail-closed-Umbau bewusst nicht (`wfp.rs:379-385`). Der VM-Smoke-Test würde daher fehlschlagen. |
| W10 | P3 | **`WfpEngine::verify` prüft nur Schlüssel-Existenz, nicht Aktion, Gewicht oder Bedingungen.** Ein manipulierter Filter mit gleichem Key würde als vorhanden zählen. |

## 4. VPN und Tor

| ID | Schwere | Befund |
|---|---|---|
| V1 | P0 | **Mullvad wird ohne Hash-/Signaturprüfung aus einem Pfad außerhalb des Installationsordners gestartet.** `preflight` prüft nur `path.exists()` (`backend.rs:298-339`); `mullvad_path` zeigt fest auf `C:\Program Files\Mullvad VPN\resources\mullvad.exe`. Das verletzt die AGENTS-Regel „nur Installationsordner, nach Hash-Prüfung“. |
| V2 | P1 | **`vpn_country=random` wählt keinen zufälligen Endpunkt.** `vpn_connect` ruft `set_location` nur auf, wenn `country != "random"` (`backend.rs:398-423`). Mullvad behält die zuletzt gewählte Location; die UI behauptet „Zufällig wählt pro Sitzung einen anderen Endpunkt“. |
| V3 | P1 | **Mullvad-CLI-Aufrufe haben keinen Timeout.** `MullvadClient::run` nutzt `Command::output()` ohne Zeitlimit (`service/net/src/mullvad.rs`). Ein hängendes `mullvad.exe` blockiert Vorprüfung, Enable, Status und Shutdown. |
| V4 | P1 | **Der Tor-Permit wird vor dem Tor-Start, aber nach dem VPN gesetzt.** Das ist besser als vorher, aber weiterhin nicht interface-gebunden (W2). Zusätzlich bleibt der Permit bei einem späteren Fehler aktiv (S8). |
| V5 | P1 | **Feste Ports 9150/9051 kollidieren mit dem Tor Browser.** `tor_start` startet Tor mit `--SocksPort 9150 --ControlPort 9051` (`backend.rs:425-480`); Tor Browser nutzt standardmäßig ebenfalls 9150. Ein späterer Tor-Browser-Start (S2) kann den Port nicht binden. PLAN 4.5 verlangt getrennte Ports A/B. |
| V6 | P2 | **`tor_bridges` wird nicht umgesetzt.** `tor_start` übergibt keine Bridge-Optionen; die Einstellung wird gespeichert, hat aber keine Wirkung. |
| V7 | P2 | **Die Tor-Cookie-Datei wird nicht separat geschützt.** `tor_data_dir` wird mit `create_dir_all` angelegt; die DACL des `%ProgramData%\TVPN`-Verzeichnisses wird nur beim Schreiben von Token/Settings gesetzt. Ob `control_auth_cookie` für den Installationsbenutzer lesbar ist, muss geprüft werden; falls ja, kann jeder Prozess dieses Benutzers Tor steuern. |
| V8 | P2 | **Kein Tor-Reconnect/Selbstheilung.** Bei einem Abbruch der Control-Verbindung liefern `bootstrap()`/`circuit()` Fehler; `tor_wait` läuft in den Timeout, `status` meldet 0/false. Es gibt keinen Neustartversuch. |
| V9 | P2 | **Kein Mullvad-Lockdown/Interface-Check.** `set_lockdown` existiert, wird aber nie aufgerufen; `E_VPN_LOCKDOWN_CONFLICT` ist unbenutzt. S1/D12 bleiben offen. |
| V10 | P2 | **Der VPN-Disconnect wartet nur 8 s und prüft nicht die Route.** `shutdown` pollt `status`, aber nicht die Standardroute oder das Interface. Ein „disconnected“-Status bedeutet nicht, dass keine Route mehr existiert. |
| V11 | P3 | **`parse_status` liest nur die erste nicht-leere Zeile.** Warnungen/Formatänderungen der CLI führen zu `Unknown`; `preflight` akzeptiert `Ok(_)` und fährt fort. |
| V12 | P3 | **`set_location` bleibt persistent.** Nach dem Ausschalten bleibt die zuletzt gesetzte Mullvad-Location erhalten; die App stellt sie nicht zurück. |

## 5. IPC, Named Pipe und Nebenläufigkeit

| ID | Schwere | Befund |
|---|---|---|
| I1 | P0 | **`panic` und Watchdog sind während `enable` blockiert.** `serve_connection` hält den `Mutex<Service>` für den gesamten `handle()`-Aufruf (`pipe.rs:480-533`). `enable` kann bis zu 180 s auf Tor und 40 s auf Mullvad warten. Der Watchdog-Thread und jeder Panik-Client warten auf denselben Mutex. |
| I2 | P1 | **Nur eine Pipe-Instanz.** `CreateNamedPipeW` nutzt `nMaxInstances=1` (`pipe.rs:359-394`); der Vertrag erlaubt vier. Ein einzelner Client kann die UI blockieren. |
| I3 | P1 | **Kein Lese-/Schreib-Timeout.** `ReadFile` in `serve_connection` blockiert unbegrenzt; ein Client kann die einzige Pipe offen halten, ohne zu senden. `WriteFile` kann ebenfalls blockieren, wenn der Client die Antwort nicht liest. |
| I4 | P1 | **Kein Rate-Limit.** `IpcError::RateLimit` existiert, wird aber nie verwendet; der Vertrag verlangt 20 Kommandos/s, Burst 40. |
| I5 | P1 | **Kein `PIPE_REJECT_REMOTE_CLIENTS`, kein `FILE_FLAG_FIRST_PIPE_INSTANCE`.** Ein lokaler/entfernter Prozess kann die Pipe vor dem Dienst belegen; der Dienst startet dann nicht und die UI kann an einen falschen Server geraten. |
| I6 | P1 | **Dienststopp kann an einer offenen Verbindung hängen.** `run_with_stop` prüft `stop` nur zwischen Verbindungen; `wake()` versucht eine neue Pipe-Verbindung, was bei belegter Instanz fehlschlägt. `serve_connection` bleibt in `ReadFile`. |
| I7 | P2 | **Ein kaputter Frame beendet die Verbindung nicht.** Bei `parse_request`-Fehlern wird eine Antwort mit `id:"?"` geschrieben und weitergelesen; der Vertrag sagt, ein kaputter Frame beendet die Verbindung. |
| I8 | P2 | **`write_response` ignoriert Teil-Writes.** `WriteFile` liefert `written`, aber der Wert wird nicht geprüft; eine gekürzte Antwort ist möglich. |
| I9 | P3 | **Token-Vergleich ist nicht constant-time.** `token != self.token` in `Service::hello`; bei einem lokalen 32-Byte-Token ist das Risiko gering, aber leicht zu beheben. |
| I10 | P3 | **`getrandom().expect` kann den Dienst panisch beenden.** In `generate_token`/`MacAddress::random`/`neutral_name`; bei CSPRNG-Ausfall stirbt der Prozess statt sauber zu melden. |
| I11 | P2 | **Ereignisstrom fehlt vollständig.** Der Dienst implementiert nur Request/Response; `state_changed`, `progress`, `banner`, `alarm`, `settings_changed`, `resync_required` usw. werden nie gesendet. Die UI pollt stattdessen. |
| I12 | P2 | **`hello`-Antwort und `states[]` werden von der UI nicht genutzt.** Die UI kennt die Zustände statisch; bei einer Vertragsänderung driftet sie. |
| I13 | P2 | **`get_status` blockiert den Mutex und macht teure Backend-Aufrufe.** Jeder Poll kann durch Mullvad/Tor/WFP mehrere Sekunden brauchen; die UI hat keinen Timeout. |

## 6. Dienstinstallation, Installer und Recovery

| ID | Schwere | Befund |
|---|---|---|
| D1 | P1 | **Der Dienststart wird ignoriert.** `install()` ruft `let _ = service.start(...)` (`service.rs`); ein fehlgeschlagener Start wird als Erfolg gemeldet und vom NSIS-Hook nicht geprüft. |
| D2 | P1 | **Keine Failure-Actions, kein DelayedAutoStart.** `install()` legt nur `AutoStart` an; stirbt `tvpnd`, startet es nicht automatisch neu. |
| D3 | P1 | **Uninstall ignoriert Stop-Fehler und disarmed möglicherweise einen laufenden Dienst.** `uninstall()` macht `let _ = service.stop()`, wartet 10 s und ruft dann `disarm_own_filters()` auf, auch wenn der Dienst noch läuft. Danach kann `delete()` scheitern; der Installer ignoriert den Exitcode. |
| D4 | P1 | **Uninstall lässt `%ProgramData%\TVPN` stehen.** Token, `owner.sid`, `settings.json`, Tor-Daten und Cookie bleiben auf der Platte; das ist ein Forensik-/Identitätsrest. |
| D5 | P1 | **NSIS-Hook ignoriert Exitcodes.** `installer.nsh` ruft `ExecWait` ohne `$0`-Prüfung; ein fehlgeschlagener `--install`/`--uninstall` führt nicht zu `Abort`. |
| D6 | P1 | **Upgrade-Pfad ist nicht abgesichert.** Der Installer stoppt den Dienst nicht vor dem Ersetzen von `tvpnd.exe`; `install()` scheitert, wenn der Dienst schon existiert; der Hook ignoriert das. Ein Upgrade kann eine alte Dienstbinary/-konfiguration hinterlassen. |
| D7 | P2 | **Fehlende `tvpnd.exe`/Tor-Dateien führen nicht zum Installationsabbruch.** `IfFileExists ... 0 +2` überspringt die Dienstinstallation still; der Installer meldet Erfolg, die UI zeigt später `service_unavailable`. |
| D8 | P1 | **Kein eigenständiges `tvpn-recover.exe`.** Recovery ist nur über den laufenden Dienst möglich; bei einem Startproblem bleibt der Nutzer auf Reinstall/Uninstall angewiesen. PLAN/P2-06 verlangen die eigenständige EXE. |
| D9 | P2 | **Kein `ServiceRecovery`/`sc failure`.** Ein Absturz hinterlässt den Dienst gestoppt; Filter bleiben ggf. aktiv. |
| D10 | P2 | **Dienstkonto ist LocalSystem.** D11 ist offen; das ist die mächtigste Variante und vergrößert die Angriffsfläche. |
| D11 | P2 | **Kein Dienst-SID-Typ/Least-Privilege.** `ServiceInfo.account_name=None`; keine Einschränkung auf ein minimales Konto. |

## 7. Downloader

| ID | Schwere | Befund |
|---|---|---|
| DL1 | P0 | **Die Route „Direkt“ ist im bewaffneten Zustand faktisch blockiert.** Die WFP-Allowlist permitet `tor.exe` und Mullvad, aber nicht `tvpn-pro.exe`. Ein direkter Download aus der Tauri-Shell wird von Default-Deny geblockt. Die UI bietet „Direkt“ trotzdem an und warnt „der Server sieht deine echte IP“ – das ist im bewaffneten Zustand falsch (der Download scheitert). |
| DL2 | P1 | **Scan-TOCTOU.** Nach `scan(&staging)` wird die Datei mit `move_into_place` verschoben. Ein gleicher Benutzer/Prozess kann die Staging-Datei zwischen Scan und Move ersetzen; die verschobene Datei ist dann ungeprüft. |
| DL3 | P1 | **Staging-Datei ist vorhersagbar und symlink-anfällig.** `staging_path` ist `app_local_data_dir/staging/<id>.part`; `File::create` folgt Symlinks. Ein lokaler Angreifer kann eine Symlink auf eine Zieldatei legen und sie beim Download truncaten/überschreiben. |
| DL4 | P1 | **Unbekanntes Scan-Urteil landet ohne MOTW im Download-Ordner.** `transfer` verschiebt bei `unknown` nach `Downloads\TVPN`, setzt MOTW aber nur bei `clean`. Bei fehlendem Defender (oder Fehler) ist die Datei ungeprüft und ohne Mark-of-the-Web. |
| DL5 | P1 | **Quarantäne ist lesbar und ausführbar.** `quarantine()` verschiebt nach `app_local_data_dir/quarantine/<id>.bin` ohne DACL/Encryption/Ausführungsentzug. Die UI/Plan-Aussage „Quarantäne unlesbar“ ist falsch. |
| DL6 | P1 | **Quarantäne wird in der UI nie gezählt.** Der Backend-`Done.verdict` ist nur `clean`/`threat`/`unknown`; die UI mappt `threat` auf `threat`, nie auf `quarantined`. `quarantinedDownloadCount()` und die Sitzungszusammenfassung bleiben 0. |
| DL7 | P1 | **Gleichzeitige Downloads können denselben Zielpfad wählen.** `destination()` prüft `candidate.exists()` und gibt den Pfad zurück; zwei Tasks können beide denselben Namen sehen. `move_into_place` nutzt zuerst `rename` (schlägt bei existierendem Ziel fehl) und fällt dann auf `copy` zurück, das die andere Datei überschreibt. |
| DL8 | P1 | **Abbruch wirkt nicht vor dem Verbindungsaufbau.** `fetch_to_file` prüft `cancel` erst in der Stream-Schleife, nicht vor `client.get(url).send()`. Ein bereits gestarteter/queued Direktdownload kann nach dem Ausschalten noch eine Verbindung über die echte IP aufbauen. |
| DL9 | P1 | **`cancelAllDownloads` wartet nicht.** UI `disable`/`panic` setzt nur Flags und sendet sofort `disable`/`panic`; der Dienst removed die Firewall, während Download-/Scan-Tasks weiterlaufen. |
| DL10 | P1 | **Scan hat keinen Timeout.** `MpCmdRun` läuft in `spawn_blocking` ohne Zeitlimit; ein hängender Scanner blockiert den Download-Task und den Abbruch. |
| DL11 | P2 | **Der vollständige Dateipfad wird an die Webview gesendet.** `Done.path` enthält den Benutzernamen; der Vertrag verbietet Pfade an die UI. |
| DL12 | P2 | **Keine Aufräumroutine für Staging/Quarantäne.** Beim App-Ende/Absturz bleiben `.part`- und Quarantänedateien liegen; es gibt keinen Start-Up-Cleanup. |
| DL13 | P2 | **Keine Download-Historie/Persistenz.** Die Liste lebt nur im Speicher; nach Neustart sind Urteile/Quarantäne-Zähler weg. |
| DL14 | P2 | **„Liste leeren“ bricht laufende Downloads nicht ab.** `clearDownloads()` setzt nur `app.downloads=[]`; es ruft kein `download_cancel_all` auf. Die Tasks laufen weiter und emittieren Events für unbekannte IDs. |
| DL15 | P2 | **Kein Pause/Resume, kein VPN-Pfad, kein aria2.** `DownloadPhase 'paused'` existiert, aber es gibt keine Pause-UI; `route: vpn` wird abgelehnt; aria2 ist nicht integriert. |
| DL16 | P2 | **Kein Öffnen/Im-Ordner-anzeigen/Retry.** Nach `download-done` gibt es keine Aktion außer Entfernen/Liste leeren. |
| DL17 | P3 | **`filename_from_url` verliert Unicode/Prozentkodierung.** Nicht-ASCII wird zu `_`; die UI zeigt den dekodierten Namen, die Datei heißt anders. |
| DL18 | P3 | **Kein Größen-/Parallelitätslimit für die Download-Map.** Beliebige IDs und unbegrenzt viele Downloads wachsen im Speicher; `safe_id` begrenzt die Länge nicht. |
| DL19 | P3 | **`mark_of_the_web` ignoriert Fehler.** Ein fehlendes MOTW wird nicht gemeldet; die UI zeigt trotzdem „Sauber“. |
| DL20 | P3 | **`scan` dupliziert die Scanner-Logik.** `service/scanner` hat `parse_defender_exit`, `is_dangerous_extension`, `has_double_extension`, wird aber nicht verwendet. Warnungen für Doppel-Endungen/gefährliche Typen kommen nie. |

## 8. UI, Zustände und Einstellungen

| ID | Schwere | Befund |
|---|---|---|
| U1 | P0 | **Die Haupt-Panik-Schaltfläche verspricht „Apps zu, Netz zu“, macht aber „Apps bleiben, Netz auf“.** `+page.svelte` zeigt „Panik: Apps zu, Netz zu“; `Service::panic` beendet nur Tor/VPN, entfernt die Firewall und geht nach `OFF`. Keine fremde App wird geschlossen; das Netz ist offen. Die Einstellungen sagen korrekt „Das Netz ist danach offen“, der Hauptbutton widerspricht. |
| U2 | P0 | **Der Status-Chip „Scharf“ nutzt den Zustand statt `firewall_armed`.** `+page.svelte` verwendet `isArmed(app.state)`; bei `FAILCLOSED` nach fehlgeschlagener Vorprüfung ist `firewall_armed=false`, der Chip zeigt trotzdem „Scharf“ und der Panik-Button wird angeboten. |
| U3 | P1 | **Die Länderauswahl wird nicht gespeichert.** `SettingsSheet.svelte:88` bindet `bind:value={app.settings.vpnCountry}` direkt, ohne `updateSetting`. Im Desktop wird kein `set_settings` gesendet. Zusätzlich sind die `COUNTRIES`-Codes klein (`de`), der Dienst akzeptiert nur Großbuchstaben (`DE`). |
| U4 | P1 | **Die angezeigte Länderauswahl passt nicht zu den Dienstwerten.** `applySettings` übernimmt `vpn_country` unverändert; bei `"DE"` findet das `<select>` keine Option (`de`). `countryName("DE")` liefert „Zufällig“. |
| U5 | P1 | **Das Portal-Zeitfenster wird nicht gespeichert.** `SettingsSheet.svelte:156` bindet den Slider direkt an `app.settings.portalTimeoutSec`; kein `updateSetting`. Die Beschriftung sagt „Der Wert wird gespeichert“, das ist falsch. |
| U6 | P1 | **Die meisten Schutz-Einstellungen sind wirkungslos.** `tor_bridges`, `mac_mode`, `block_fallback`, `torrent_mode`, `scanner_hash_lookup`, `portal_timeout_sec` werden vom Backend nicht umgesetzt. Nur `vpn_enabled` und (indirekt) `vpn_country` haben eine reale Wirkung; `vpn_country` ist durch U3/U4 defekt. |
| U7 | P1 | **`clipboard_watch` hat keine UI.** Der Vertrag/PLAN sieht eine optionale Zwischenablage-Beobachtung vor; der Typ existiert, aber es gibt keinen Schalter. |
| U8 | P1 | **Keine Allowlist-Verwaltung.** Die UI behauptet „Firewall-Allowlist“ und „nur freigegebene Apps“, aber es gibt keine Liste, keine App-Auswahl und keine Freigabe. WFP permitet nur Tor/Mullvad; normale Apps bleiben blockiert. |
| U9 | P1 | **„Firewall-Allowlist: Immer an“ ist falsch.** Im Zustand `OFF`, `PREFLIGHT` und bei `FAILCLOSED` ohne Filter Ist keine Allowlist aktiv. |
| U10 | P1 | **Die UI behauptet ClamAV-Scans.** `+page.svelte` schreibt „Scans laufen über Defender und ClamAV“; der Downloader ruft nur `MpCmdRun.exe` auf. ClamAV ist nicht implementiert. |
| U11 | P1 | **Die UI behauptet „Land: Zufällig wählt pro Sitzung einen anderen Endpunkt“.** `vpn_country=random` setzt keine Location (V2); Mullvad behält die letzte Auswahl. |
| U12 | P1 | **`clearDownloads` bricht keine Downloads ab.** Der „Liste leeren“-Button entfernt nur die UI-Einträge; laufende Tasks schreiben weiter und bleiben in der Download-Map. |
| U13 | P1 | **Sitzungszusammenfassung ist im Desktop defekt.** Wegen S1 wird `get_session_summary` abgelehnt; selbst nach einem Whitelist-Fix ist `startedAt` durch S13 falsch, Quarantäne immer 0 (DL6) und `failclosedCount` kumulativ (S14). |
| U14 | P1 | **Nach Panik gibt es keine Zusammenfassung.** `panic()` öffnet kein Summary; der Dienst liefert `null` (S7). |
| U15 | P2 | **Dienstausfall lässt Seitenleisten-Daten stehen.** `markServiceLost` setzt nur `serviceReachable=false` und einen Banner; VPN/Tor/Watchdog-Werte bleiben auf dem letzten Stand (z. B. „Circuit steht“), obwohl der Status unbekannt ist. |
| U16 | P2 | **`failureCode` wird bei Watchdog-Failclosed nicht gesetzt.** `applyServiceStatus` setzt ihn nicht aus dem Status; die UI zeigt den Fehlercode nur nach einem Kommando-Fehler. |
| U17 | P2 | **Runtime-Downloadfehler werden nicht angezeigt.** `+page.svelte` loggt `download-error` nur in die Konsole; die Liste zeigt „Fehler“ ohne Code/Text. |
| U18 | P2 | **Einstellungen sind während des bewaffneten Zustands editierbar.** Der Dienst lehnt `set_settings` mit `E_STATE_CONFLICT` ab; die UI ändert optimistisch und rollt erst nach einem Refresh zurück. Kein globales Disable, keine Erklärung. |
| U19 | P2 | **`updateSetting` rollt bei Dienstfehler nicht zuverlässig zurück.** Es setzt `settingsLoaded=false` und ruft `refreshService`; scheitert der Refresh, bleibt der lokale Wert stehen. |
| U20 | P2 | **Kein Single-Instance-Schutz.** Zwei UI-Prozesse können gleichzeitig pollen/kommandieren, zwei Tray-Icons erzeugen und den Hotkey nur einmal registrieren. |
| U21 | P2 | **Schließen blendet ohne Tray-Fallback aus.** Wenn die Tray-Erstellung fehlschlägt (`main.rs` loggt nur), versteckt `CloseRequested` das Fenster; der Nutzer hat keinen Weg zurück. |
| U22 | P2 | **Tray-Icon hat keine Zustandsfarbe/Statusanzeige.** PLAN 6.7 verlangt ein Tray-Icon mit Zustandsfarbe; das Icon ist statisch. |
| U23 | P2 | **Keine Fensterzustands-Persistenz.** Größe/Position werden nicht gespeichert; das Fenster startet immer 1120×780 zentriert. |
| U24 | P2 | **Kein Update-Kanal/keine Update-Prüfung.** README nennt den Update-Kanal als fehlend; die UI hat keine Funktion. |
| U25 | P2 | **Kein „Beenden“-Verhalten für den Dienst.** Tray „Beenden“ beendet nur die UI; Dienst und Filter laufen weiter (fail-closed, aber unklar). |
| U26 | P2 | **Hotkey-Registrierung schlägt still fehl.** `hotkey.rs` kehrt bei `RegisterHotKey`-Fehler einfach zurück; die Einstellungen behaupten weiterhin, die Taste funktioniere. |
| U27 | P3 | **`Banner.svelte` hat keinen Fallback für unbekannte Banner.** `map[kind]` ist undefined; ein neuer Dienst-Banner würde die UI beim Rendern werfen. |
| U28 | P3 | **`Banner`-Texte für `degraded`/`mac_unsupported`/`failclosed` sind teilweise falsch.** „wird neu gestartet“ (kein Neustart), „Betrieb nur mit Bestätigung“ (keine Bestätigung), „Filter bleiben gesetzt“ (bei Vorprüfungsfehler keine Filter). |
| U29 | P3 | **`countryName` liefert für unbekannte Codes „Zufällig“.** `zz`/`DE` werden als „Zufällig“ angezeigt statt „Unbekannt“. |
| U30 | P3 | **`SettingsSheet`-Select/Slider sind nicht barrierefrei beschriftet?** Der Select hat `aria-label`, der Slider auch; die sichtbaren Titel sind nicht per `for`/`id` verknüpft. Niedrig. |

## 9. URL-, Settings- und Vertragsvalidierung

| ID | Schwere | Befund |
|---|---|---|
| C1 | P0 | **URL-Validator und HTTP-Client normalisieren unterschiedlich.** `https://0x7f.0x0.0x0.0x1/` wird akzeptiert; `url::Url` macht daraus `127.0.0.1`. Damit ist SSRF auf Loopback möglich (Reproduziert R13). |
| C2 | P1 | **`localhost.localdomain` und DNS-Rebinding-Hosts werden akzeptiert.** Der Validator prüft nur exakte Namen/Suffixe, keine aufgelöste IP. `https://localhost.localdomain/` ist erlaubt; `*.nip.io` kann auf `127.0.0.1`/`169.254.169.254` zeigen. |
| C3 | P1 | **`is_valid_country` akzeptiert beliebige zwei Großbuchstaben.** `ZZ`, `XX`, `AA` sind gültig; der Vertrag verlangt ISO 3166-1 alpha-2. |
| C4 | P2 | **`Settings`-Laden fällt still auf Defaults zurück.** Eine korrupte/ältere `settings.json` wird ohne Banner/Fehler durch Defaults ersetzt; Nutzer merkt nicht, dass Einstellungen verloren gingen. |
| C5 | P2 | **`Settings` hat kein Versionsfeld/Migration.** Künftige Schemaänderungen können alte Dateien nicht sauber migrieren. |
| C6 | P2 | **`enable`/`disable`-Schema ist nicht validiert** (S2/S3). |
| C7 | P2 | **`hello`-Schema ist nicht validiert** (S4). |
| C8 | P2 | **`set_settings` akzeptiert Extra-Felder** (S5). |
| C9 | P2 | **`status` liefert `protect_mode` falsch** (S11). |
| C10 | P2 | **Der Vertrag ist nicht umgesetzt.** `contracts/ipc-v0.md` beschreibt Events, `downloads.add/control`, `open_in_fallback`, `scan.report`, `export_logs`, 4 Verbindungen, Rate-Limit, Schema-Validierung und Panik-Semantik; der Dienst implementiert nur einen Teil der Request/Response-Kommandos. |

## 10. Tests, CI und Build

| ID | Schwere | Befund |
|---|---|---|
| T1 | P1 | **`just verify` und CI prüfen die Tauri-Crate nicht.** Root-`cargo fmt/clippy/test` umfasst `ui/src-tauri` wegen des separaten `[workspace]` nicht. `cargo fmt --manifest-path ui/src-tauri/Cargo.toml -- --check` schlägt aktuell fehl (R5). |
| T2 | P1 | **`just verify` prüft UI-Check/Build nicht.** `just verify` läuft nur `fmt-check`, `lint`, `test`, `pscheck`, `redact`; `ui-check`/`ui-build`/`e2e` fehlen. CI läuft `e2e`, aber nicht `ui-check`/`ui-build`. |
| T3 | P1 | **Die UI-E2E-Datei „echter Download“ testet nicht das Produkt.** `ui/tests/e2e/downloads.spec.ts:108-141` lädt per Playwright-`request` und schreibt per Node; der Rust-Downloader/Tauri-Befehl wird nicht aufgerufen. Der Test ist selbstbestätigend. |
| T4 | P1 | **`ui/scripts/shots.mjs` wartet auf den nicht mehr existierenden Zustand „Geschützt“.** Reproduziert R4; der Screenshot-/Layout-Check ist damit tot. |
| T5 | P1 | **`ui/scripts/test-desktop-download.mjs` wartet ebenfalls auf „Geschützt“.** Zeile 46; der Desktop-Download-Test läuft in einen Timeout. |
| T6 | P1 | **`test-desktop-download.mjs` ist destruktiv und extern.** Es lädt von `https://proof.ovh.net/files/1Mb.dat` und löscht **alle neu erstellten Dateien** in `Downloads\TVPN`; ein paralleler echter Download kann gelöscht werden. |
| T7 | P1 | **`tests/vm/enable-smoke.ps1` akzeptiert `FAILCLOSED` als Erfolg.** Zeile 88 erlaubt `DEGRADED`, `READY`, `FAILCLOSED`; `enable.ok` wird nur geloggt, nicht geprüft. Ein fehlgeschlagenes Enable kann den Test bestehen. |
| T8 | P2 | **`tests/vm/enable-smoke.ps1` kopiert `geoip`/`geoip6` in `%ProgramData%\TVPN\tor`, der Dienst sucht sie aber neben `tor.exe`.** Die GeoIP-Dateien fehlen am erwarteten Ort. |
| T9 | P2 | **`tests/vm/ui-smoke.ps1` prüft nur, dass ein Fenster erscheint.** Es wird kein Zustand, keine Verbindung, kein Text und kein Fehler geprüft; der Screenshot ist die einzige Evidenz. |
| T10 | P2 | **VM-Tests hängen an `1.1.1.1:443`.** `Test-NetConnection 1.1.1.1` ist kein zuverlässiger Block-/Erreichbarkeitsbeweis (Firewall, ICMP, Netzpolicy, Proxy). |
| T11 | P2 | **VM-Test-Staging ist nicht automatisiert.** `run-in-vm.ps1` kopiert nur `-Include`-Leafs; `tvpnd.exe`, `tor.exe`, `geoip`, Installer und `wfp-smoke.exe` müssen manuell neben das Skript gelegt werden. |
| T12 | P2 | **`tests/vm/wfp-smoke.ps1`/`wfp-smoke.rs` sind veraltet.** Der Test erwartet Drop-Cleanup, der Code macht bewusst Fail-closed (W9); der Test würde fehlschlagen. |
| T13 | P2 | **`ui/tsconfig.json` schließt `tests` nicht ein.** `include: ["src","test","*"]`; `ui/tests/e2e/*.ts` werden von `svelte-check` nicht geprüft. Die Node-Typen/`request`-Parameter sind dort ungeprüft. |
| T14 | P2 | **Keine UI-Unit-Tests.** `ui/src` hat keine `.test`/`.spec`-Dateien; `parseDownloadInput`, `validateDownloadUrl`, `formatDiagnostics`, `applyServiceStatus` und der Mock-Zustandsautomat sind ungetestet. |
| T15 | P2 | **Keine Tests für Fehlerpfade des Backends.** `MockBackend` kann nur `preflight` fehlschlagen; `arm`, `vpn_connect`, `tor_start`, `tor_wait`, `leak_check`, `shutdown`, `disarm` haben keine Fehlerinjektion. |
| T16 | P2 | **`service/net` hat nur einen ignorierten WFP-Test.** Keine Unit-Tests für Filterbedingungen, Loopback, Permit-Anzahl, Disarm-Fehler. |
| T17 | P2 | **CI pinnt Actions nicht auf SHA.** `actions/checkout@v5`, `oven-sh/setup-bun@v2`, `actions/upload-artifact@v4`; Supply-Chain-Risiko. |
| T18 | P2 | **CI hat kein `cargo audit`/`cargo deny`/`bun audit`.** `bun audit` ist lokal sauber, aber nicht in CI; Rust-Audit fehlt. |
| T19 | P2 | **Release-Workflow testet nicht.** Er baut und veröffentlicht ohne `cargo test`/`clippy`/UI-Check. |
| T20 | P2 | **Release ist nicht als Prerelease markiert.** `gh release create` ohne `--prerelease`; Alpha-Tags werden als normale Releases veröffentlicht, `releases/latest` zeigt darauf. |
| T21 | P2 | **macOS-Build ist konfigurationsseitig wahrscheinlich defekt.** `tauri.macos.conf.json` überschreibt nur `bundle.targets`/Signing; die Basis-`bundle.resources` verweisen weiter auf `../../target/release/tvpnd.exe` und `../../tools/tor/tor.exe`. `macos.yml` baut weder `tvpnd` noch Tor. Der Build muss die fehlenden Ressourcen melden oder Windows-Binaries in die `.app` packen. |
| T22 | P2 | **`ui/README.md` beschreibt den alten Stand.** „Schutz-Zustandsautomat ist weiterhin ein Mock, weil der Dienst fehlt“, „Tauri-Fenster und IPC folgen“, „Persistenz der Einstellungen fehlt“, „Magnet-Links werden nur mit vpn_only akzeptiert“ – alles falsch/veraltet. |
| T23 | P2 | **`SECURITY.md` behauptet, Firewall/VPN/Tor seien nicht implementiert.** Das widerspricht README/STATE und dem Code. |
| T24 | P2 | **`docs/reports/e2e-download.md` beschreibt 5 Tests und Bulk-Route-Tests, die es nicht mehr gibt.** Aktuell 4 Tests; Magnet-Links werden immer abgelehnt. |
| T25 | P2 | **`docs/security/AUDIT-2026-11-22.md` ist stale.** 90 Tests/5 E2E; mehrere C-/H-Punkte sind inzwischen teilweise gefixt (Drop, secure_write, Redirect, Staging, `service_install`), andere nicht. Die Fix-Liste beschreibt alten Code. |
| T26 | P2 | **`docs/TASKS.md`/`docs/handoffs/P2-01-core.md` sind stale.** 45 Tests, „Windows-Dienst-Wrapper offen“, „UI-Client offen“, obwohl implementiert. |
| T27 | P3 | **`CHANGELOG.md` überspringt 0.1.3–0.1.5.** Die Git-Historie hat diese Releases; der Changelog nicht. |
| T28 | P3 | **`docs/LICENSES.md` ist ein ungeprüftes Gerüst.** Keine geprüften Lizenzen/Hashes für Tauri, Svelte, Tor-Bundle, gebündelte Komponenten; keine Lizenztexte im Installer. |
| T29 | P3 | **Winget-Hash ist nicht verifiziert.** `InstallerSha256` im Manifest muss gegen das tatsächliche Release-Asset geprüft werden; `winget validate` ist nicht in CI. |
| T30 | P3 | **`install.sh` (macOS) installiert ohne Prüfsumme weiter.** Anders als `install.ps1` nur eine Warnung; ein ungeprüfter Download wird kopiert. |
| T31 | P3 | **`install.ps1` filtert keine Prereleases/Architektur.** Es nimmt den neuesten nicht-Draft-Release; ARM64/Windows-Version werden nicht geprüft. |
| T32 | P3 | **`install.ps1` prüft keine Authenticode-Signatur.** SHA-256 aus demselben Release schützt nicht gegen ein kompromittiertes Release. |
| T33 | P3 | **Test-VM-Passwort steht im Repo.** `tests/vm/create-vm.ps1` enthält ein festes Passwort für die wegwerfbare Test-VM (der Wert wird hier bewusst nicht wiederholt); dokumentiert, aber ein Secret im Repo. |
| T34 | P3 | **`tools/cleanup-worktrees.ps1` kann mit `-Force` ungesicherte Änderungen verwerfen.** Es ist ein Dev-Tool, aber ohne zweite Bestätigung. |
| T35 | P3 | **`tools/tor` ist gitignored, aber lokal vorhanden.** Ein frischer Clone hat kein Tor; `bun run tauri build`/macOS-Build brauchen `tools/fetch-tor.ps1`, das in der README-Build-Anleitung fehlt. |

## 11. Fehlende Funktionen (Plan/Vertrag vs. Code)

Diese Punkte sind keine „Bugs“ im engeren Sinn, aber ausdrücklich geplant/vertraglich und nicht umgesetzt:

- **S1/Interface-Bindung:** WFP-Permit an VPN-Interface/Quell-IP, `verify_expected` statt Anzahl.
- **MAC-Wechsel:** `MacAddress`/`randomize_mac` sind nicht verdrahtet; `randomize_mac` gibt immer `false` zurück. Kein Original-Restore, kein Adapter-Neustart.
- **Hostname:** `HostnamePlan` ist ungenutzt; keine Setup-Neutralisierung, kein Neustart-Workflow.
- **Tor Browser:** Verifier/Launcher/Start am System-Tor, GPG, Mindestversion, Port A/B; `TorBrowserPlan` ist ungenutzt.
- **Block-Erkennung/Fallback-Browser:** keine Fenstertitel-/URL-Erkennung, kein Mullvad-Browser, kein `open_in_fallback`.
- **aria2/Downloader-Pipeline:** kein aria2, kein Resume, kein VPN-Download, keine Link-Übergabe, keine Clipboard-Beobachtung.
- **Scanner-Pipeline:** keine ClamAV, keine Doppel-Endungs-/Gefahrenwarnung, keine `Sauber/`/`Eingang/`-Ablage, kein EICAR-Test, keine Archiv-Bombe.
- **Torrents:** kein Client, kein `.torrent`, kein Seeding; UI-Option „Nur VPN“ ist wirkungslos.
- **Portal:** kein PORTAL-Zustand, keine Portal-Erkennung/Zeitfenster.
- **Netzwechsel/Standby:** keine Adapter-/WLAN-/Standby-Ereignisse, kein Neu-Bewerten.
- **Eigener Wächterprozess:** nur Thread im Dienst; kein separater Prozess, kein Herzschlag, keine Sekundentakte.
- **Standalone `tvpn-recover.exe`:** fehlt; Recovery nur über Dienst.
- **Logging/Redaktion:** kein `tracing`, keine Logs, kein Export, kein Redaktions-Layer; CI-Gate prüft nur Fixtures.
- **IPC-Events:** `state_changed`, `progress`, `banner`, `alarm`, `settings_changed`, `download_event`, `scan_result`, `block_detected`, `watchdog`, `resync_required` fehlen.
- **IPC-Kommandos:** `downloads.add`, `downloads.control`, `open_in_fallback`, `scan.report`, `export_logs` liefern `not_implemented`.
- **IPC-Härtung:** 4 Verbindungen, Rate-Limit, Read-Timeout, First-Pipe-Instance, Remote-Client-Reject, Schema-Dateien/Fuzzing.
- **Allowlist-UI:** keine App-Freigaben, keine Liste, keine Ports.
- **Download-UI:** Pause/Resume, Retry, Öffnen, Im-Ordner-anzeigen, Fehlercode, Quarantäne-Verwaltung, Historie.
- **Tray:** Zustandsfarbe, Status, „Nur UI beenden“-Semantik.
- **Fenster:** Single-Instance, Größe/Position merken, native Snap-Layouts.
- **Update:** Update-Kanal/Prüfung, Signatur.
- **Installer:** Code-Signierung, USB-Kit, Dienst-Recovery, Daten-Cleanup, Upgrade-Stopp, Exitcode-Prüfung, WebView2-Strategie dokumentieren.
- **macOS:** Firewall/VPN/MAC, Signierung/Notarisierung, Quarantäne (`com.apple.quarantine`), XProtect/ClamAV.
- **Doku:** Lizenztexte/Third-Party-Notices, SBOM, reproduzierbarer Build.

## 12. Doku- und Aussage-Widersprüche

| ID | Befund |
|---|---|
| DOC1 | `README.md:94` listet „Tray-Icon und Panik-Hotkey“ als fehlend, während `README.md:82` beide als funktionierend aufführt. |
| DOC2 | `README.md:123-128` sagt für den Release-Build nur `cd ui; bun run tauri build`; `tvpnd.exe` (Release) und `tools/tor` fehlen in einem frischen Clone. Der Build bricht an den `bundle.resources`. |
| DOC3 | `README.md:137` sagt „Direkt: ungeschützt, der Server sieht deine echte IP“; im bewaffneten Zustand blockt WFP `tvpn-pro.exe` (DL1). |
| DOC4 | `README.md:73`/Screenshots nutzen „Geschützt“; die UI heißt jetzt „Tor aktiv“. `docs/reports/vm-service-firewall.md` und `docs/reports/e2e-download.md` sind ebenfalls stale. |
| DOC5 | `ui/README.md` behauptet weiterhin Mock-Zustandsautomat, fehlende IPC, fehlende Persistenz und Bulk-Route-Akzeptanz; alles falsch. |
| DOC6 | `SECURITY.md:5` sagt, Firewall/VPN/Tor seien nicht implementiert; falsch. |
| DOC7 | `docs/THREAT_MODEL.md` sagt „Firewall lässt nur Allowlist-Apps zu Tor-Ports auf Loopback“; tatsächlich globales Loopback-Permit (W1). Es sagt „MAC-Wechsel pro Sitzung“, „Hostname neutralisiert“, „Aufräumen“ – nicht implementiert. |
| DOC8 | `AGENTS.md` sagt „Dienst startet nur Programme aus dem Installationsordner, nach Hash-Prüfung“; Mullvad liegt außerhalb und wird nicht gehasht (V1). |
| DOC9 | `PLAN.md:176` fordert Panik „Apps schließen, Netz zu“; der Code öffnet das Netz und schließt keine Apps (U1). |
| DOC10 | `PLAN.md` fordert `tvpn-recover.exe`, Port A/B, Standalone-Wächter, Logging, Scanner-Pipeline, USB-Kit; der Code hat sie nicht. |
| DOC11 | `docs/security/AUDIT-2026-11-22.md` beschreibt alte Befunde (90 Tests, 5 E2E, Drop-Disarm, `service_install`, `std::fs::write`), die teilweise nicht mehr stimmen. |
| DOC12 | `CHANGELOG.md` behauptet „Abbruch aller Downloads bei Panic/Disable“; `cancelAllDownloads` ist fire-and-forget (DL9). Es behauptet keine Fake-Angaben, aber „ClamAV“, „Allowlist immer an“, „Panik Apps zu“ sind falsch. |
| DOC13 | `README.md:99` sagt „Firewall und Tor sind verdrahtet, die VPN-Kette und die Hardware-Features sind es nicht“ – das ist die ehrlichste Kurzfassung; die Detailtabelle darüber ist optimistischer. |

## 13. Empfohlene Fix-Reihenfolge

### P0 – sofort, vor jedem weiteren „funktioniert“-Claim

1. **S1:** `get_session_summary` in die Tauri-Whitelist aufnehmen und den Desktop-Summary-Pfad testen.
2. **U1/U2:** Panik-Semantik entscheiden (echter Kill-Switch vs. „Schutz beenden, Netz öffnen“) und UI/Chip/Panic-Button an `firewall_armed` und die tatsächliche Wirkung binden.
3. **W1/W2:** Loopback nur für konkrete App-IDs/Ports; Tor-Permit erst nach verifiziertem VPN und mit Interface-/Quell-IP-Bedingung (S1).
4. **C1/C2:** URL-Validierung auf einen echten URL-Parser + DNS-/IP-Prüfung umstellen; Redirect-Ziele erneut prüfen; private/link-local/loopback IPs blockieren.
5. **S8/S9/S10:** FailClosed muss Tor/VPN stoppen oder den Tor-Permit entfernen; `firewall_armed` nur aus `verify_expected()`; `disarm()` muss Fehler zurückgeben und verifizieren.
6. **DL1:** „Direkt“ im bewaffneten Zustand deaktivieren oder über den Dienst führen; keine falsche „echte IP“-Warnung.

### P1 – vor einem Beta-/Release-Claim

7. **V1/V3:** Mullvad-Hash/Signatur + Timeout; `vpn_country=random` wirklich randomisieren.
8. **U3/U4/U5/U6:** Länder-/Portal-Einstellungen korrekt uppercase senden, `updateSetting` verwenden, wirkungslose Optionen deaktivieren oder implementieren.
9. **DL2–DL10:** Staging mit `create_new`/Handle, Scan-TOCTOU schließen, unbekannte Dateien in `Eingang/`, Quarantäne restriktiv, Zähler korrigieren, Cancel vor `send`/await, Scan-Timeout.
10. **I1–I6:** Enable asynchron aus dem Mutex, eigener Panik-Kanal, Pipe-Timeout/Rate-Limit/First-Instance/Remote-Reject, max. 4 Verbindungen.
11. **D1–D8:** Dienststart prüfen, Failure-Actions, Uninstall-Stop/Disarm/Verify, ProgramData-Cleanup, NSIS-Exitcodes, Upgrade-Stopp, `tvpn-recover.exe`.
12. **T1–T7:** UI-/Tauri-CI, `ui/src-tauri` fmt/clippy/test, `shots`/`test-desktop` reparieren, E2E-Dateitest durch echten Tauri-Test ersetzen, Enable-Smoke strikt machen.
13. **U8–U12:** Allowlist-UI, ClamAV-Claim entfernen, Quarantäne-Zähler, `clearDownloads` canceln.

### P2/P3 – Härtung, Komfort, Doku

14. IPC-Events/Vertrag vervollständigen oder Vertrag ehrlich auf Polling reduzieren.
15. Settings-Schema/Version/Migration, `ZZ`-Validierung, `hello`-Felder, `enable`/`disable`-Schema.
16. macOS-Ressourcen/Signierung/Quarantäne, Release-Prerelease/SBOM/Lizenzen, Winget-Hash, Action-Pinning, `cargo audit/deny`.
17. Doku aktualisieren: README, ui/README, SECURITY, THREAT_MODEL, AUDIT, TASKS, CHANGELOG, e2e-report, vm-report, Screenshots.
18. Fehlende Features aus Abschnitt 11 in kleinen, testbaren Schritten.

## 14. Abdeckungsgrenzen

- **Keine VM-Tests ausgeführt.** Die vorhandene VM `tvpn-test` wurde nicht zurückgesetzt/gestoppt, um ihren Zustand nicht zu verlieren. WFP-/Tor-/Mullvad-/Installer-Aussagen sind daher Code-/Doku-basiert, nicht frisch in der VM reproduziert.
- **Kein Mullvad-Konto/App**: Die VPN-zu-Tor-Kette und S1-Interface-Bedingung sind nicht prüfbar.
- **Keine physische Hardware**: MAC-Adapterschreibvorgang, Hostname-Neustart, Suspend/Resume, WLAN-Wechsel sind nicht geprüft.
- **Kein macOS-Gerät**: macOS-Build/Ressourcen/Signierung sind nur konfigurationsseitig bewertet.
- **Kein echter Defender-Scan**: `MpCmdRun`-Pfad/Exitcodes sind Code-/Doku-basiert; EICAR/Archiv-Bombe nicht getestet.
- **Kein Fuzzing**: Pipe/JSON/URL-Fuzzing fehlt; die Bewertung ist statisch plus gezielte Harnesses.
- **Keine Codeänderungen**: Dieser Bericht dokumentiert nur; die einzige externe Änderung ist der Audit-Harness unter `%TEMP%\tvpn-audit-harness` (nicht im Repo).

## 15. Evidenz-Kommandos (kompakt)

```powershell
just verify
cd ui; bun run check; bun run build
cd ..; just e2e
cd ui; bun run shots
cargo fmt --manifest-path ui/src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path ui/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path ui/src-tauri/Cargo.toml
cd ui; bun audit
```

Die Harness-Ausgaben zu Ländern, Args, Summary und URL-Normalisierung stehen in Abschnitt 1 (R8–R13).
