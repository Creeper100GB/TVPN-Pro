# IPC-Vertrag v0 (Entwurf)

Nur der Orchestrator ändert diese Datei. Änderungswünsche: `docs/contract-requests.md`. Jede Nachricht ist JSON mit Versionsfeld `v`, Schema-validiert, mit Größen- und Rate-Limit.

## Zustände

`OFF, PREFLIGHT, ARMING, PORTAL, VPN_UP, TOR_UP, READY, DEGRADED, FAILCLOSED, TEARDOWN, RECOVERY`

| Von | Nach | Auslöser |
|---|---|---|
| OFF | PREFLIGHT | `enable` |
| PREFLIGHT | ARMING | alle Vorabprüfungen ok |
| ARMING | PORTAL | kein Internet, Portal erkannt |
| ARMING / PORTAL | VPN_UP | VPN verbunden |
| VPN_UP | TOR_UP | Tor Bootstrap 100 %, Circuit steht |
| TOR_UP | READY | Leak-Check ok, Apps gestartet |
| READY | DEGRADED | Wächter WARN/DEGRADED |
| DEGRADED | READY | Komponente wiederhergestellt |
| beliebig | FAILCLOSED | Wächter CRITICAL, Fehler, unklarer Netzwechsel |
| READY / DEGRADED / FAILCLOSED | TEARDOWN | `disable` oder Panik |
| TEARDOWN | OFF | Abbau fertig |
| beliebig | RECOVERY | `recover` oder Reste beim Start |
| RECOVERY | OFF | Netzwerk wiederhergestellt |

## Kommandos (UI → Dienst)

| Kommando | Parameter (nur Whitelist-Werte) | Wirkung |
|---|---|---|
| `enable` | `net_mode: focus` (später `system`), `protect_mode: tor_vpn` | startet den Einschalt-Ablauf |
| `disable` | `downloads_policy: wait \| pause \| abort` | startet den Ausschalt-Ablauf |
| `panic` | – | Apps schließen, Netz zu, Teardown |
| `recover` | – | Notfall-Reset |
| `get_status` | – | Zustand, Modus, Land (nur Code), Wächter, aktive Banner |
| `get_settings` / `set_settings` | Schlüssel aus Whitelist | Einstellungen lesen/ändern |
| `downloads.add` | `url` (nur https) | Link an Downloader, URL nur im Speicher |
| `downloads.control` | `pause_all \| resume_all \| cancel(id)` | Downloads steuern |
| `open_in_fallback` | `url` (https, optional) | Fallback-Browser nur über VPN öffnen |
| `scan.report` | `file_id` | Scan-Bericht lesen |

## Ereignisse (Dienst → UI)

`state_changed{from,to,reason_code}` · `alarm{level,code}` · `banner{kind,active}` · `download_event{id,phase,progress}` · `scan_result{file_id,verdict}` · `block_detected{domain_hash}` · `watchdog{check_id,level}`

Banner-Arten: `portal_open`, `fallback_active`, `degraded`, `failclosed`, `mac_unsupported`, `hostname_unchanged`.

## Fehlercodes (Präfixe)

`E_PREFLIGHT_*` · `E_FIREWALL_*` · `E_MAC_*` · `E_HOSTNAME_*` · `E_VPN_*` · `E_TOR_*` · `E_LAUNCH_*` · `E_DOWNLOAD_*` · `E_SCAN_*` · `E_WATCHDOG_*` · `E_IPC_*` · `E_RECOVERY_*`

## Regeln

- Keine Shell-Strings. Keine Dateipfade, Programmnamen oder Argumente von der UI. Nur Werte aus Whitelists.
- URLs erscheinen nur flüchtig in `downloads.add` und `open_in_fallback`. Nie persistieren, nie loggen.
- Named Pipe mit ACL (nur der installierende Benutzer) plus Sitzungs-Token.
- Unbekannte Felder oder falsche Version → Nachricht verwerfen und `E_IPC_*` melden.
