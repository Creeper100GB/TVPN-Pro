# Roadmap: Was fehlt und in welcher Reihenfolge es gebaut wird

Diese Datei beantwortet eine Frage: **Was ist nicht implementiert, und was kommt
in welcher Reihenfolge?** Quelle sind die Audit-Abschnitte 11 (fehlende
Funktionen) und 13 (Fix-Reihenfolge) in
[`docs/security/AUDIT-2026-11-22-full.md`](security/AUDIT-2026-11-22-full.md).
Was heute funktioniert, steht in [`README.md`](../README.md); was die App
ausdrücklich nicht kann, ebenfalls.

Grundsätze für alle Pakete:

- **Fail-closed bleibt.** Kein Paket darf einen Fehlerzustand in „läuft ohne
  Schutz weiter“ verwandeln.
- **Erst der Filter, dann die Bequemlichkeit.** Nichts aus Paket 5 und 6 wird
  gebaut, solange die Interface-Bindung (Paket 2) offen ist.
- **Keine Termine.** Die Reihenfolge ist eine Abhängigkeitskette, kein
  Zeitplan. Aufwände sind grobe Größenordnungen (S/M/L), keine Zusagen.
- **Hardware-/VM-Gates sind markiert.** Punkt „VM“ heißt: nur in der
  Test-VM bzw. auf physischer Hardware prüfbar, nicht auf dem Host.

## Paket 1: IPC- und Dienst-Härtung

**Danach funktioniert:** Der Dienst ist unter Last und bei Angriffen von einem
unprivilegierten Prozess aus nicht mehr zum Stillstand zu bringen; ein
`enable` blockiert die Pipe nicht; ein Uninstall lässt keine Filter zurück.

**Nötig:** Enable asynchron aus dem Mutex lösen, eigener Panik-Kanal,
Pipe-Read-Timeout, Rate-Limit, First-Instance-Binding, Remote-Client-Reject,
maximal vier Verbindungen (Audit I1–I6). Dienststart prüfen, Failure-Actions
setzen, Uninstall mit Stop/Disarm/Verify, `%ProgramData%`-Cleanup,
NSIS-Exitcodes auswerten, Upgrade-Stopp, eigenständiges `tvpn-recover.exe`
(Audit D1–D8).

**Aufwand:** L. **Gates:** VM (Uninstall, Recovery, Dienststart).

## Paket 2: WFP-Interface-Bindung

**Danach funktioniert:** Der Tor-Permit gilt nur noch für den tatsächlichen
VPN-Pfad. `shield.firewall_armed` kommt aus `verify_expected()` statt aus einer
Filteranzahl, und `disarm()` meldet Fehler, statt sie zu schlucken.

**Nötig:** Permits an Interface-LUID bzw. Quell-IP binden (S1), Loopback nur
für konkrete App-IDs und Ports statt global freigeben (W1/W2), `verify_expected`
einführen (S9), `disarm` mit Rückgabewert und Verifikation (S10), FailClosed
muss Tor/VPN stoppen oder den Tor-Permit entfernen (S8).

**Aufwand:** L. **Gates:** VM; zusätzlich physische Hardware für den echten
VPN-Adapter.

## Paket 3: Mullvad V1/V3

**Danach funktioniert:** Die VPN-Kette ist nicht nur „gestartet“, sondern
geprüft: Version, Hash und Signatur der Mullvad-Binaries, Timeout auf den
Verbindungsaufbau, echtes Randomisieren von `vpn_country=random`.

**Nötig:** Hash-/Signaturprüfung und Timeout (V1), Randomisierung (V3), die
Abweichung „Mullvad liegt außerhalb des Installationsordners und wird nicht
gehasht“ auflösen (Audit DOC8).

**Aufwand:** M. **Gates:** VM mit Mullvad-Konto; ohne Konto bleibt die Kette
ungeprüft.

## Paket 4: MAC-Wechsel und Hostname

**Danach funktioniert:** Der MAC-Wechsel ist verdrahtet (nicht nur vorhanden),
mit Original-Restore und Adapter-Neustart; der Hostname wird einmalig im Setup
neutralisiert und der Neustart-Workflow ist beschrieben.

**Nötig:** `MacAddress`/`randomize_mac` verdrahten (heute immer `false`),
Original-Restore, Adapter-Neustart; `HostnamePlan` nutzen, Setup-Neutralisierung
(S3 verlangt einen Neustart).

**Aufwand:** M. **Gates:** physische Hardware — der Hyper-V-Adapter meldet
`unsupported`.

## Paket 5: Tor Browser und Portal

**Danach funktioniert:** Der Tor Browser startet aus der App gegen das
System-Tor, mit Verifier, Mindestversion und GPG-Prüfung; der Portal-Zustand
existiert mit Zeitfenster und Block-Erkennung.

**Nötig:** Verifier/Launcher/Start am System-Tor, GPG, Mindestversion, Port A/B;
`PORTAL`-Zustand mit Erkennung und Zeitfenster; Fenstertitel-/URL-Erkennung für
die Block-Erkennung, Mullvad-Browser und `open_in_fallback`.

Grenze: Der Tor Browser bleibt ein fremdes Programm. Die App kann ihn starten
und prüfen, nicht absichern; im Tor Browser sind weiterhin keine Add-ons und
kein Marionette/WebDriver erlaubt.

**Aufwand:** M. **Gates:** VM.

## Paket 6: Downloader-/Scanner-Pipeline, Logging, IPC-Events

**Danach funktioniert:** Download, Scan und Ablage sind eine Kette mit klaren
Zuständen: unbekannte Dateien landen in `Eingang/`, saubere in `Sauber/`,
Quarantäne ist restriktiv, Fortschritt und Urteile laufen über echte
IPC-Events; Logs existieren, sind redigiert und exportierbar.

**Nötig:** aria2/Resume/VPN-Download/Link-Übergabe/Clipboard (Audit 11),
Scanner-Pipeline mit Doppel-Endungs-Warnung, Ablage-Ordnern, EICAR-Test und
Archiv-Bomben-Schutz, Quarantäne-Verwaltung, Download-UI (Pause/Resume/Retry/
Fehlercode/Historie), `tracing` plus Redaktions-Layer und Export, die fehlenden
IPC-Events (`state_changed`, `progress`, `banner`, `alarm`, `settings_changed`,
`download_event`, `scan_result`, `block_detected`, `watchdog`,
`resync_required`) und die Kommandos
`downloads.add`, `downloads.control`, `open_in_fallback`, `scan.report`,
`export_logs`.

**Aufwand:** L. **Gates:** VM für den Redaktions-Gate-Lauf mit echten Logs.

## Paket 7: Signierung, Release, macOS

**Danach funktioniert:** Ein Release ist signiert und aktualisierbar: signierter
Installer, Update-Kanal mit Signaturprüfung, SBOM und Third-Party-Notices.

**Nötig:** Code-Signierung, Update-Kanal, USB-Kit, Winget-PR-Pflege pro Release
(siehe [`docs/WINGET.md`](WINGET.md)), SBOM, reproduzierbarer Build,
`cargo audit`/`cargo deny` in der CI; macOS: Firewall/VPN/MAC,
Signierung/Notarisierung, `com.apple.quarantine`.

**Aufwand:** L. **Gates:** Zertifikat und Release-Rechte durch den
Repository-Inhaber; macOS auf echter Hardware.

## Paket 8: Bedienung und Netz-Ereignisse

**Danach funktioniert:** Die App merkt einen Netzwechsel oder Standby und
bewertet den Schutz neu; die Allowlist ist in der UI pflegbar; Tray und Fenster
verhalten sich wie ein normales Windows-Programm.

**Nötig:** Adapter-/WLAN-/Standby-Ereignisse mit Neu-Bewertung (Audit 11,
schutzrelevant und deshalb zusammen mit Paket 2 fällig); Allowlist-UI mit
App-Freigaben, Liste und Ports; Tray-Zustandsfarbe, Status und die Semantik
„Nur UI beenden“; Fenster: Single-Instance, Größe/Position merken, native
Snap-Layouts.

**Aufwand:** M. **Gates:** VM für Netzwechsel und Standby; der Rest ist auf dem
Host prüfbar.

## Was ausdrücklich **nicht** geplant ist

Die folgenden drei Punkte stehen als **Vorschlag** (D41–D43) im ADR-Log. Bis der
Mensch sie bestätigt, gelten die alten Entscheidungen D30 (Torrents über VPN)
und D7 (Defender **und** ClamAV) formal weiter; die Roadmap beschreibt den
beabsichtigten Zustand.

- **Torrents.** Es wird keinen Torrent-Client, keine `.torrent`-Datei und kein
  Seeding geben. Magnet-Links bleiben abgelehnt. Die UI-Option „Nur VPN“ für
  Torrents ist wirkungslos und wird entfernt, nicht implementiert. Entscheidung:
  [`docs/DECISIONS.md`](DECISIONS.md) D41 (ersetzt D30).
- **ClamAV.** Der Scanner ist Windows Defender (`MpCmdRun.exe`). ClamAV wird
  nicht als zweite Engine nachgezogen; Nennungen in älteren Texten und
  Screenshots sind überholt. Entscheidung: [`docs/DECISIONS.md`](DECISIONS.md)
  D42 (ersetzt D7).
- **Ein getrennter Wächterprozess.** Der Wächter läuft als Thread im Dienst.
  Ein eigener Prozess mit Herzschlag ist nicht geplant; Tray-Icon und
  Panik-Hotkey funktionieren bereits. Entscheidung:
  [`docs/DECISIONS.md`](DECISIONS.md) D43.

## Reihenfolge in einem Satz

Erst der Dienst und die Pipe (1), dann die Firewall-Bindung (2), dann die
VPN-Kette (3), dann MAC/Hostname (4), dann Tor Browser und Portal (5), dann die
Download-/Scan-Pipeline mit Logging und Events (6), dazu die Bedienung samt
Netzwechsel-Ereignissen (8), zuletzt Signierung und macOS (7). Torrents,
ClamAV und ein separater Wächterprozess stehen auf keiner Liste.
