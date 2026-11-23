## Übergabe: Worker, round2-fixes

- Fertig:
  - **P0-1 Staging:** `ui/src/lib/teardown.ts`, `ui/tests/unit/teardown.test.ts`,
    `docs/ROADMAP.md`, `docs/security/AUDIT-2026-11-22-full.md` sind gestaged
    (`git add`, kein Commit, kein Push).
  - **P1-2 System-Proxy:** `client_builder` setzt auf der Direkt-Route
    `no_proxy()`. Ohne das umging `HTTP(S)_PROXY`/`ALL_PROXY` (und die
    Windows-Proxy-Einstellungen) den SSRF-Pin. Regressionstest
    `the_direct_route_ignores_a_system_proxy` (Env-Mutex, Restore,
    `tokio::time::timeout`).
  - **P1-3 `state_reached`:** `shouldWriteSummary(reached)` in `teardown.ts`;
    beide Panik-Zweige in `mock.svelte.ts` nutzen sie. `TEARDOWN` schreibt keine
    Zusammenfassung mehr.
  - **P1-4 Buchhaltung:** `teardownStart(banners)` in `teardown.ts` liefert
    `{state: 'TEARDOWN', banners, failureCode: null}`; alle vier Abbau-Einträge
    rufen sie auf. Unit-Test ab `('FAILCLOSED', ['failclosed','no_filter'])`:
    `isFailedTeardown` false, `no_filter` bleibt.
  - **P2-5 Denylist:** 0.0.0.0/8, 198.18.0.0/15, 240.0.0.0/4, 192.0.0.0/24,
    192.88.99.0/24, `::/96`, `64:ff9b::/96`, `2002::/16`, `2001::/32`,
    `fec0::/10`; Tabellentest erweitert (`64:ff9b::7f00:1`, `198.18.0.1`).
  - **P2-6 `pin_hop`:** `Pinned.host` ist das Literal ohne Klammern
    (`host_str()` nur für den Resolver); Test
    `an_ipv6_literal_is_pinned_without_its_brackets`.
  - **P2-7 `move_into_place`:** löscht den reservierten Platzhalter nicht mehr
    vor dem `rename`; im Copy-Fallback erst nach dem gescheiterten `rename`.
  - **P2-8 `quarantine_failed`:** zählt jetzt als `failed`; Zählwerk in
    `ui/src/lib/summary.ts` (`summarizeDownloadList`) ausgelagert und mit
    `ui/tests/unit/summary.test.ts` geprüft.
  - **P2-9 deutsche Fehlertexte:** `blocked_address`, `redirect`, `url`,
    `too_large`, `move`, `quarantine_dir`, `quarantine_move` in `backend.ts`.
  - **P2-10 Scanner-Reader:** `join_until` mit 5-s-Gnadenfrist statt
    unbegrenztem `join`; hängender Reader ⇒ `unknown`.
  - **P2-11 Pin-Verdrahtung:** `a_pinned_name_connects_to_the_checked_address`
    (Resolver `pinned.test` → 127.0.0.1, `AllowLoopback`, Body kommt an).
  - **P2-12:** `teardownFailed`-Hinweis in `+page.svelte` prüft
    `app.firewallArmed`; ohne Filter die Offen-Warnung.
  - **P2-13:** `teardownDuration` im Preview 900 → 2000 ms.
  - **P2-14:** CR-02 neu begründet (Desync-Sicherheitsnetz), CR-03 ergänzt
    (`panic` öffnet das Netz vs. Vertrag „Netz zu").
  - **P2-15:** D37 korrigiert (Vertrag §8 erlaubt `TEARDOWN → RECOVERY`, die
    Entscheidung ist bewusst dagegen); D41 (ersetzt D30/Torrents), D42 (ersetzt
    D7/ClamAV), D43 (kein getrennter Wächterprozess) ergänzt.
  - **P2-16:** ROADMAP um `progress`/`settings_changed`, Paket 8 (Nutzung:
    Netzwechsel/Standby, Allowlist-UI, Fenster, Tray) und Verweise auf
    D41–D43 erweitert; weiterhin ohne Termine.
  - **P2-17:** `ui/README.md`: Unit-Test-Zeile ergänzt, „fünf" → „vier",
    „Nur VPN" ehrlich beschrieben, ClamAV/Torrents aus „noch nicht enthalten"
    zu „nicht geplant" verschoben.
  - **P2-18:** `justfile` und `README.md` sagen jetzt „alles außer den
    Playwright-E2E" und nennen die UI-Unit-Tests.
  - **P2-19:** `docs/STATE.md` (Panik öffnet das Netz, Wächter/Tray/Hotkey
    vorhanden, Verify-Umfang) aktualisiert.
  - **P2-20:** Audit-Kopf ohne persönlichen Pfad.
  - **P2-21:** `.gitignore` auf `/progress.md` verankert.
  - **P2-22:** CHANGELOG: „Abbruch wird angefordert".
- Getestet (Host, Befehle, Ergebnis; alle mit `CARGO_TARGET_DIR=%TEMP%/tvpn-verify`,
  MSVC über `tools/with-msvc.ps1`):
  - `cargo fmt --all -- --check` → 0; `cargo clippy --workspace --all-targets -- -D warnings` → 0
  - `cargo test --workspace` → 0 (alle Test-Binaries grün, 1 WFP-Test ignoriert)
  - Tauri: `cargo fmt -- --check` → 0; `clippy --locked … -D warnings` → 0; `cargo test --locked` → 18/18 (9,3 s)
  - `bun run check` → 0 Fehler / 0 Warnungen; `bun run build` → ok
  - `bun run test` → 7/7 (2 Dateien); `bun run e2e` → 8/8 (1,4 min)
  - `tools/check-powershell.ps1` → 22 Skripte; Redaktions-Selbsttest → PASS
  - `winget validate --manifest …/0.1.6` → „Die Manifestüberprüfung war erfolgreich.“
  - Unabhängig geprüft: Release-`SHA256SUMS.txt` = `f2243187…4cf24f`; beide
    gebündelten EXEs importieren nur `api-ms-win-crt-*`, kein `vcruntime140.dll`.
- Offen / Risiken:
  - `the_direct_route_ignores_a_system_proxy` verändert Prozess-Env; alle
    Client-Tests im Modul nehmen dafür denselben Mutex. Ein künftiger
    Netzwerk-Test in `download.rs` muss ihn ebenfalls nehmen.
  - Die Reader-Gnadenfrist detacht im Fehlerfall einen Thread (Pipe hält ein
    Enkelprozess); das ist bewusst, der Scan liefert dann `unknown`.
  - D41 ersetzt eine vom Menschen am 21.11. abgenommene Entscheidung (D30,
    Torrents über VPN). Die Roadmap hat das bereits so festgehalten; wenn der
    Mensch die Torrent-Funktion will, muss D41 zurückgenommen werden.
- Brauche von anderen (Contract-Änderung?): CR-03 (`panic` öffnet das Netz)
  offen; CR-01/CR-02 unverändert offen. Keine Codeänderung nötig.
- Achtung beim Commit: nichts committet, nichts gepusht. Gestaged sind alle
  neu hinzugekommenen Pflichtdateien (`teardown.ts`, `summary.ts`, beide
  Unit-Tests, `docs/ROADMAP.md`, der Audit-Bericht, diese Übergabe,
  `service/tvpnd/build.rs`). `progress.md` bleibt ungetrackt (`/progress.md`).
  Vor dem Commit muss `git add -A` alle Änderungen stagen — die 42 geänderten
  Dateien sind derzeit ungestaged —, danach ist `git status` erneut zu prüfen:
  ein Commit aus dem jetzigen Teil-Index würde nicht bauen (z. B. ruft
  `service/tvpnd/build.rs` `static_vcruntime::metabuild()` auf, während die
  `[build-dependencies]`-Änderung in `service/tvpnd/Cargo.toml` ungestaged
  ist).
- Nächster Schritt: Review des Diffs; danach Commit (`fix: close round-2 review
  findings`) durch den Orchestrator. Nicht in diesem Slice: Failover über
  mehrere gepinnte Adressen, Request-/Read-Timeouts, `direct` im Vertrag,
  Desktop-E2E-Abdeckung, Awaited-Download-Cancel.
