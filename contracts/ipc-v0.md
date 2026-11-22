# IPC-Vertrag v0 (Entwurf)

Nur der Orchestrator ändert diese Datei. Änderungswünsche: `docs/contract-requests.md`. Jede Nachricht ist JSON mit Versionsfeld `v`, Schema-validiert, mit Größen- und Rate-Limit.

Status: **Entwurf für Phase 2.** Offene Punkte stehen am Ende. Alles, was hier nicht steht, ist nicht Teil des Vertrags und darf von der UI nicht gesendet werden.

## 1. Transport

| Punkt | Festlegung |
|---|---|
| Kanal | Named Pipe `\\.\pipe\tvpn-pro-v0`, Nachrichtenstrom in beide Richtungen |
| Format | ein JSON-Objekt pro Zeile (NDJSON), UTF-8 ohne BOM, Zeilenende LF |
| Max. Nachricht | 64 KiB. Größer → Verbindung schließen, `E_IPC_TOO_LARGE` |
| ACL | DACL nur für den installierenden Benutzer und das Dienstkonto; alle anderen deny |
| Verbindungen | max. 4 gleichzeitig; die 5. wird abgelehnt |
| Rate-Limit | 20 Kommandos/s, Burst 40. Überschreitung → `E_IPC_RATE_LIMIT`, Verbindung wird geschlossen |
| Authentisierung | Sitzungs-Token, siehe 2 |
| Verschlüsselung | keine. Der Kanal ist lokal; Vertraulichkeit kommt aus der ACL, nicht aus TLS |

Warum NDJSON: trivial zu implementieren, trivial zu fuzzen, kein Längenfeld, das man falsch parsen kann. Ein kaputter Frame beendet die Verbindung, nicht den Dienst.

## 2. Sitzung und Token

- Der Dienst erzeugt bei jedem Start 32 zufällige Bytes (CSPRNG), base64url, und legt sie in `%ProgramData%\TVPN\session.token` ab (ACL: installierender Benutzer lesen, Dienstkonto lesen/schreiben, alle anderen deny).
- Die UI liest die Datei, sendet `hello` mit dem Token und bekommt Sitzungs-ID und Fähigkeiten.
- Falsches oder fehlendes Token → `E_IPC_AUTH`, Verbindung wird sofort geschlossen. Kein Retry-Backoff im Dienst.
- Token rotiert bei jedem Dienststart. Die UI verbindet neu, sobald `get_status` fehlschlägt.
- **Grenze, ehrlich benannt:** Jeder Prozess desselben Benutzers kann die Token-Datei lesen. Die Pipe trennt Benutzer, nicht Prozesse desselben Benutzers. Deshalb bleiben die Kommandos eine feste, kleine Liste ohne Pfade, Programmnamen oder Argumente.

## 3. Nachrichten

Anfrage:
```json
{"v":0,"id":"6f1c…","cmd":"enable","args":{"net_mode":"focus","protect_mode":"tor_vpn"}}
```

Antwort:
```json
{"v":0,"id":"6f1c…","ok":true,"data":{}}
{"v":0,"id":"6f1c…","ok":false,"error":{"code":"E_STATE_CONFLICT","detail":"not_off"}}
```

Ereignis:
```json
{"v":0,"evt":"state_changed","seq":4711,"data":{"from":"OFF","to":"PREFLIGHT","reason_code":"user_enable"}}
```

Regeln:

- `id` ist ein String, 1 bis 64 Zeichen, vom Client vergeben. Jede Anfrage bekommt **genau eine** Antwort mit derselben `id`.
- `seq` ist pro Verbindung monoton steigend. Lücken sind erlaubt (siehe 5).
- `detail` ist ein kurzer Maschinen-Code (`not_off`, `hash_mismatch`), **nie** Freitext mit Zielen, Pfaden oder Dateinamen.
- Unbekannte Felder, unbekannte Kommandos, falscher Typ oder falsche `v` → Nachricht verwerfen, `E_IPC_*` antworten.
- Antworten auf `hello` sind die einzige Ausnahme: ohne `hello` ist jedes andere Kommando `E_IPC_AUTH`.

## 4. Kommandos (UI → Dienst)

Alle Kommandos sind idempotent, außer `downloads.add` (jeder Aufruf startet einen Download). `enable` und `disable` antworten mit `accepted: false`, wenn der Ablauf schon in dieselbe Richtung läuft; das ist kein Fehler. `enable.args.net_mode` muss zum gespeicherten `net_mode` passen, sonst `E_IPC_BAD_SCHEMA`.

| Kommando | `args` | `data` der Antwort | Wirkung |
|---|---|---|---|
| `hello` | `token`, `client` (`ui`), `client_version` | `session_id`, `service_version`, `protocol`, `states[]` | Handshake, siehe 2 |
| `enable` | `net_mode: "focus"`, `protect_mode: "tor_vpn"` | `accepted: bool`, `state` | startet den Einschalt-Ablauf (PLAN 5) |
| `disable` | `downloads_policy: "wait" \| "pause" \| "abort"` | `accepted: bool`, `state` | startet den Ausschalt-Ablauf |
| `panic` | – | `accepted: true` | aus **jedem** Zustand: Apps zu, Netz zu, Teardown. Kein Rückfrage-Dialog |
| `recover` | – | `accepted: bool` | Notfall-Reset, entfernt nur Filter mit unserer Provider-GUID |
| `get_status` | – | vollständiger Status (6.1) | Snapshot für UI und Resync |
| `get_settings` | – | vollständige Einstellungen (6.2) | lesen |
| `set_settings` | `settings` (nur Schlüssel aus 6.2) | vollständige Einstellungen nach dem Schreiben | ändern; ungültig → `E_IPC_BAD_SCHEMA` |
| `downloads.add` | `url` (HTTPS, siehe 7) **oder** `torrent` (magnet, siehe 7.1); `route: "tor" \| "vpn"` (Default `tor`) | `id` | Link an den Downloader; URL nur im Speicher |
| `downloads.control` | `action: "pause_all" \| "resume_all" \| "cancel"`, `id` nur bei `cancel` | `affected: int` | Downloads steuern |
| `open_in_fallback` | `url` (optional, HTTPS) | `opened: bool` | Fallback-Browser nur über den VPN-Adapter |
| `scan.report` | `file_id` | `verdict`, `reasons[]`, `scanned_at` | Scan-Bericht lesen |
| `export_logs` | – | `path` | Export-Bündel (redigiert) unter `%ProgramData%\TVPN\exports` |
| `get_session_summary` | – | Zusammenfassung (6.4) oder `null` | Anzeige nach dem Ausschalten |

Was die UI **nie** sendet und der Dienst **nie** akzeptiert: Dateipfade, Programmnamen, Kommandozeilen, Argumente, Shell-Strings, Prozess-IDs, Portnummern, IP-Adressen, Kontonummern.

## 5. Ereignisse (Dienst → UI)

| Ereignis | `data` | Bemerkung |
|---|---|---|
| `state_changed` | `from`, `to`, `reason_code` | bei jedem Übergang aus 5.1 |
| `progress` | `phase`, `percent` | `phase`: `preflight`, `arming`, `mac`, `portal`, `vpn`, `tor`, `leakcheck`, `apps` |
| `banner` | `kind`, `active` | Anzeige-Zustand, kein Log |
| `alarm` | `level` (`warn` \| `critical`), `code` | Wächter-Eskalation |
| `download_event` | `id`, `phase` (`queued` \| `active` \| `paused` \| `done` \| `error`), `received_bytes`, `total_bytes` (optional) | keine URL, kein Dateiname |
| `scan_result` | `file_id`, `verdict`, `reasons[]` | `reasons` sind Codes, keine Namen |
| `block_detected` | `domain_hash` | z. B. `sha256:` plus 12 Hex-Zeichen |
| `watchdog` | `check_id`, `level` | einzelne Prüfung |
| `settings_changed` | `settings` | nach `set_settings` |
| `session_summary` | siehe 6.4 | nach `TEARDOWN` |
| `resync_required` | – | Client war zu langsam; `get_status` neu holen |

Zustellung: geordnet pro Verbindung, **best effort**. Der Dienst puffert 256 Ereignisse pro Verbindung. Läuft der Puffer über, verwirft er die ältesten und sendet `resync_required`. Die UI darf sich nie auf Ereignisse als einzige Wahrheit verlassen; nach jeder Verbindung gilt `get_status`.

Banner-Arten: `portal_open`, `fallback_active`, `degraded`, `failclosed`, `mac_unsupported`, `hostname_unchanged`, `vpn_route`.

## 6. Datenmodelle

### 6.1 Status

```json
{
  "state": "READY",
  "since": "2026-11-21T10:04:11Z",
  "net_mode": "focus",
  "protect_mode": "tor_vpn",
  "vpn": { "state": "connected", "country": "se" },
  "tor": { "bootstrap": 100, "circuit": true },
  "watchdog": { "level": "ok", "last_check_at": "2026-11-21T10:04:10Z" },
  "banners": ["fallback_active"],
  "downloads": { "active": 1, "quarantined": 0 },
  "session": { "id": "…", "started_at": "2026-11-21T10:00:00Z" },
  "shield": {
    "firewall_armed": true,
    "firewall_filters": 12,
    "mac_supported": false,
    "mac_randomized": false
  },
  "service": { "version": "0.0.0", "protocol": 0 }
}
```

`country` ist ISO 3166-1 alpha-2 in Kleinbuchstaben oder `null` (noch nicht verbunden). Keine IP, kein Hostname, keine Domain.

`shield.firewall_armed` ist die Wahrheit für die UI: `FAILCLOSED` kann auch eine fehlgeschlagene Vorprüfung bedeuten, bei der noch kein Filter gesetzt wurde. Die UI darf dann nicht "Netz gesperrt" behaupten.

### 6.2 Einstellungen (Whitelist)

| Schlüssel | Typ | Default | Erlaubte Werte |
|---|---|---|---|
| `vpn_enabled` | bool | `true` | – |
| `vpn_country` | string | `"random"` | `random` oder ISO-3166-1 alpha-2 (Großbuchstaben) |
| `tor_bridges` | bool | `false` | – |
| `block_fallback` | enum | `auto` | `off`, `suggest`, `auto` |
| `mac_mode` | enum | `random` | `random`, `original` |
| `torrent_mode` | enum | `blocked` | `blocked`, `vpn_only` |
| `net_mode` | enum | `focus` | nur `focus` in v0 |
| `clipboard_watch` | bool | `false` | – |
| `scanner_hash_lookup` | bool | `false` | – |
| `portal_timeout_sec` | int | `180` | 60 bis 600 |

Unbekannte Schlüssel → `E_IPC_BAD_SCHEMA`, nichts wird geschrieben. Alle Schlüssel werden auf einmal geschrieben oder gar nicht.

### 6.3 Download

`downloads.add` gibt eine `id` zurück (String, 1 bis 32 Zeichen). Die `id` ist keine Datei, kein Pfad und kein Hash. Sie lebt nur, solange der Dienst läuft.

### 6.4 Session-Zusammenfassung

```json
{
  "started_at": "…", "ended_at": "…",
  "downloads": { "clean": 3, "warned": 1, "quarantined": 1, "failed": 0 },
  "state_reached": "READY",
  "failclosed_count": 1,
  "reasons": ["E_TOR_BOOTSTRAP_TIMEOUT"]
}
```

Keine Dateinamen, keine Domains. Nur Zähler und Fehlercodes.

## 7. URL-Regeln

Gilt für `downloads.add` und `open_in_fallback`:

- nur `https`, Port 443 (implizit oder explizit), kein anderer Port
- Hostname, kein IP-Literal (kein IPv4/IPv6-Literal, auch nicht in Klammern)
- kein `localhost`, `.local`, `.internal`, `.lan`, `.home.arpa`
- keine Userinfo (`user:pass@`)
- maximale Länge 8192 Zeichen
- keine URL in Logs, keine Persistenz, kein Echo in Antworten oder Ereignissen

### 7.1 Torrent-Regeln

- `route: "vpn"` ist nur erlaubt, wenn `torrent_mode = "vpn_only"`. Sonst `E_DOWNLOAD_ROUTE_FORBIDDEN`.
- `torrent` (magnet) ist nur mit `route: "vpn"` erlaubt. Mit `route: "tor"` → `E_DOWNLOAD_TORRENT_BLOCKED`.
- Magnet-Format: `magnet:?xt=urn:btih:<40 Hex oder 32 Base32>`, optional `&dn=`, keine weiteren Parameter werden ausgewertet. Maximale Länge 8192 Zeichen.
- `.torrent`-Dateien sind normale HTTPS-Links und laufen über `url` mit `route: "vpn"`.
- Auf der VPN-Route gilt: kein DHT, kein Local Peer Discovery, kein Peer Exchange, kein UPnP/NAT-PMP, keine eingehenden Verbindungen. Peers sehen die VPN-IP, nie die echte.
- Die Route wird im Ereignis `download_event` als `route` mitgeführt, damit die UI den Banner `vpn_route` zeigen kann.

## 8. Zustände und Übergänge

Zustände: `OFF, PREFLIGHT, ARMING, PORTAL, VPN_UP, TOR_UP, READY, DEGRADED, FAILCLOSED, TEARDOWN, RECOVERY`

| Von | Nach | Auslöser |
|---|---|---|
| OFF | PREFLIGHT | `enable` |
| PREFLIGHT | ARMING | alle Vorabprüfungen ok |
| PREFLIGHT | FAILCLOSED | Vorprüfung fehlgeschlagen (`E_PREFLIGHT_*`) |
| PREFLIGHT | TEARDOWN | `disable` während der Prüfung |
| ARMING | PORTAL | kein Internet, Portal erkannt |
| ARMING | VPN_UP | VPN verbunden, kein Portal |
| ARMING | FAILCLOSED | Firewall-, MAC- oder VPN-Fehler |
| PORTAL | VPN_UP | Portal-Login fertig, VPN verbunden |
| PORTAL | FAILCLOSED | Zeitfenster (`portal_timeout_sec`) abgelaufen oder Fehler |
| VPN_UP | TOR_UP | Bootstrap 100 %, Circuit steht |
| VPN_UP | FAILCLOSED | Tor-Timeout oder Tor-Fehler |
| TOR_UP | READY | Leak-Check ok, Apps gestartet |
| TOR_UP | FAILCLOSED | Leak-Check negativ |
| READY | DEGRADED | Wächter `warn` |
| DEGRADED | READY | Komponente wiederhergestellt |
| READY, DEGRADED | FAILCLOSED | Wächter `critical` |
| ARMING, PORTAL, VPN_UP, TOR_UP, READY, DEGRADED, FAILCLOSED | TEARDOWN | `disable` oder `panic` |
| TEARDOWN | OFF | Abbau vollständig |
| TEARDOWN | RECOVERY | Filterreste oder Adapter nicht wiederherstellbar |
| beliebig außer OFF | RECOVERY | `recover` |
| OFF | RECOVERY | Reste beim Dienststart gefunden |
| RECOVERY | OFF | Netzwerk nachweislich wiederhergestellt |

Invarianten:

- `FAILCLOSED` heißt: unsere Filter bleiben gesetzt. Nur `TEARDOWN` und `RECOVERY` entfernen sie.
- `panic` ist aus jedem Zustand erlaubt, auch aus `FAILCLOSED` und `RECOVERY`, und endet immer in `TEARDOWN`, nie in `RECOVERY`.
- Scheitert das Recovery, bleibt der Zustand `RECOVERY` und es kommt ein `alarm` mit `critical`; es gibt **keinen** Selbstübergang und kein `state_changed`.
- Der Tod der UI ist **kein** Sicherheitsereignis: der Dienst bleibt im aktuellen Zustand, die Filter bleiben scharf. UI-Abriss führt nie zu `TEARDOWN`.
- Jeder Übergang erzeugt genau ein `state_changed`. Der Dienst schreibt den Zustand vor dem Ereignis, nie umgekehrt.

## 9. Fehlercodes

Präfixe bleiben stabil; konkrete Codes dürfen innerhalb eines Präfixes ergänzt werden.

| Code | Bedeutung |
|---|---|
| `E_PREFLIGHT_NO_ADMIN` | Dienst hat nicht die nötigen Rechte |
| `E_PREFLIGHT_HASH_MISMATCH` | Binärdatei stimmt nicht mit dem gepinnten Hash überein |
| `E_PREFLIGHT_MISSING_COMPONENT` | Baustein fehlt (Tor, Mullvad, Browser, Downloader) |
| `E_PREFLIGHT_VPN_NOT_LOGGED_IN` | Mullvad nicht angemeldet |
| `E_PREFLIGHT_STALE_SESSION` | Reste einer alten Sitzung gefunden, `recover` nötig |
| `E_FIREWALL_ARM_FAILED` | Filter konnten nicht gesetzt werden |
| `E_FIREWALL_VERIFY_MISMATCH` | Filtersatz unvollständig oder verändert |
| `E_FIREWALL_RECOVERY_FAILED` | eigene Filter nicht entfernbar |
| `E_SETTINGS_PERSIST` | Einstellungen konnten nicht sicher gespeichert werden |
| `E_MAC_UNSUPPORTED` | Adaptertreiber kann die MAC nicht ändern |
| `E_MAC_APPLY_FAILED` | MAC geschrieben, Adapter kam nicht wieder hoch |
| `E_HOSTNAME_APPLY_FAILED` | Hostname konnte nicht gesetzt werden |
| `E_VPN_CONNECT_TIMEOUT` | VPN-Verbindung kam nicht zustande |
| `E_VPN_LOCKDOWN_CONFLICT` | Mullvad-Filter kollidieren mit unseren (S1) |
| `E_TOR_BOOTSTRAP_TIMEOUT` | Tor erreichte 100 % nicht |
| `E_TOR_CIRCUIT_FAILED` | kein Circuit, obwohl Bootstrap fertig |
| `E_TOR_CONTROL_AUTH` | Control-Port-Authentisierung fehlgeschlagen |
| `E_LAUNCH_HASH_MISMATCH` | zu startende Datei hat falschen Hash |
| `E_LAUNCH_NOT_INSTALLED` | Programm fehlt im Installationsordner |
| `E_LAUNCH_VERSION_TOO_OLD` | Tor Browser unter Mindestversion |
| `E_DOWNLOAD_URL_INVALID` | URL verletzt Abschnitt 7 |
| `E_DOWNLOAD_HTTP_REJECTED` | nur HTTPS erlaubt |
| `E_DOWNLOAD_TORRENT_BLOCKED` | Torrent auf der Tor-Route oder `torrent_mode = blocked` |
| `E_DOWNLOAD_ROUTE_FORBIDDEN` | `route: "vpn"` ohne `torrent_mode = "vpn_only"` |
| `E_DOWNLOAD_PROXY_FAILED` | Downloader kommt nicht über Port B |
| `E_SCAN_ENGINE_MISSING` | keine Scan-Engine verfügbar |
| `E_SCAN_TIMEOUT` | Scan lief in das Zeitlimit |
| `E_WATCHDOG_CRITICAL` | Wächter hat `failclosed` ausgelöst |
| `E_WATCHDOG_HEARTBEAT_LOST` | Gegenseite des Herzschlags fehlt |
| `E_IPC_AUTH` | Token falsch oder fehlt |
| `E_IPC_BAD_VERSION` | `v` passt nicht zum Dienst |
| `E_IPC_BAD_SCHEMA` | Pflichtfeld fehlt, Typ falsch, unbekanntes Feld |
| `E_IPC_RATE_LIMIT` | zu viele Nachrichten |
| `E_IPC_TOO_LARGE` | Nachricht über 64 KiB |
| `E_IPC_UNKNOWN_COMMAND` | Kommando nicht in der Liste |
| `E_STATE_CONFLICT` | Kommando im aktuellen Zustand nicht erlaubt |
| `E_RECOVERY_FAILED` | Recovery konnte den Zustand nicht herstellen |

## 10. Versionierung

- `v` ist eine ganze Zahl, aktuell `0`.
- Additive Felder sind innerhalb von `v:0` erlaubt, solange Clients unbekannte Felder ignorieren.
- Jede Breaking Change erhöht `v` und den Pipe-Namen (`tvpn-pro-v1`). Ein Dienst spricht genau eine Protokollversion.
- Der Dienst lehnt Nachrichten mit fremder `v` ab, statt sie zu interpretieren.

## 11. Offene Punkte (vor Phase 2 zu klären)

1. **Dienstkonto** (D11): LocalSystem oder eigenes Konto. Bestimmt ACL und Token-Datei endgültig.
2. **Wächter-Kanal**: eigener Kanal (eigene Pipe oder Shared Memory) statt UI-Pipe; braucht einen eigenen kleinen Vertrag.
3. **JSON-Schema-Dateien** unter `contracts/schema/v0/`: sinnvoll, sobald die Kommandos stabil sind; bis dahin ist dieser Text die Wahrheit.
4. **`export_logs`**: Zielpfad und Öffnen des Ordners sind UI-Sache; der Dienst liefert nur den Pfad. Ob das Bündel signiert wird, ist offen.
5. **S1-Ergebnis**: ob `enable` einen Lockdown-Modus aktivieren muss, ändert `args` von `enable` nicht, aber die Fehlercodes (`E_VPN_LOCKDOWN_CONFLICT`).
