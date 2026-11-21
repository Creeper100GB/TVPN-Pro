## Übergabe: orchestrator, P2-01/P2-02 (Kern, ohne Pipe)

- **Fertig:**
  - `service/core/src/state.rs`: Zustandsautomat aus Vertrag Abschnitt 8. `next(state, event)` ist eine reine Funktion, alle Übergänge, Invarianten (Panic aus jedem Zustand nach `TEARDOWN`, `Recover` nicht aus `OFF`, `FAILCLOSED` öffnet nicht direkt, `Enable` nur aus `OFF`).
  - `service/core/src/settings.rs`: Whitelist mit zehn Schlüsseln, Defaults laut Vertrag, `deny_unknown_fields`, Alles-oder-nichts-Patch, Ländercode- und Timeout-Prüfung, `net_mode: system` wird abgelehnt.
  - `service/core/src/url.rs`: HTTPS, Port 443, kein IP-Literal, keine lokalen Namen, keine Userinfo, 8 KiB. Handgeschrieben, keine zusätzliche Abhängigkeit.
  - `service/core/src/ipc.rs`: Envelope mit Version, ID-Regeln, Kommando-Whitelist (14 Kommandos), 64-KiB-Limit, Antwort- und Ereignisformen mit den Vertragsnamen.
  - `service/core/src/redact.rs`: Scanner für IPv4, IPv6, MAC und URLs; Loopback ist erlaubt; Treffer nennen nur Zeile und Kategorie, nie den Wert.
  - 45 Unit-Tests, `#![forbid(unsafe_code)]`.
- **Getestet (Host):** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p tvpn-core --lib` → 45 grün. Kein Systemeingriff, kein Netzwerk im Code.
- **Offen / Risiken:**
  - Named-Pipe-Server, ACL und Sitzungs-Token fehlen (P2-01-Rest).
  - `tracing`-Anbindung mit Redaktions-Layer fehlt; der Scanner ist die Logik, nicht der Logger (P2-02-Rest).
  - Kein `tvpnd`-Hauptprogramm, keine Persistenz der Einstellungen.
  - Der Automat modelliert „Filter bleiben gesetzt" nur als Zustandseigenschaft; die Durchsetzung ist net-core.
  - `cargo test` braucht die MSVC-Umgebung, siehe `tools/with-msvc.ps1`.
- **Brauche von anderen (Contract-Änderung?):** Nein. Der Vertrag passt; `E_STATE_CONFLICT` wird für abgelehnte Übergänge benutzt.
- **Nächster Schritt:** Pipe-Server (tokio `named_pipe` oder `windows`-Crate) mit ACL und Token, `tvpnd`-Hauptprogramm, `tracing`-Layer mit `redact::scan`, dann UI von Mock auf IPC umstellen.
