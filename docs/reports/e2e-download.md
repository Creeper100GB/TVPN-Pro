# E2E: Download-Flow und Aufräumen

Stand: 21. November 2026. Test: `ui/tests/e2e/downloads.spec.ts`. Ausführen mit `just e2e` oder `bun --cwd ui run e2e`.

## Was der Test wirklich prüft

| Teil | Real oder Mock | Prüfung |
|---|---|---|
| Schutz ein/aus, Zustände, Banner | Mock (kein Dienst, kein Netz) | Klick auf den Power-Knopf führt zu „Geschützt"; erneuter Klick öffnet die Sitzungs-Zusammenfassung |
| Download-Liste, URL-Prüfung, Fortschritt, Urteil | Mock | HTTPS-Pflicht, Port 443, keine IP-Literale, keine lokalen Namen; Eintrag erreicht „Sauber" |
| Sitzungs-Zusammenfassung | Mock, aber aus der Download-Liste berechnet | Zähler „Sauber" ist 1, nicht hartkodiert |
| Aufräumen in der App | Mock | „Liste leeren" hinterlässt den leeren Zustand |
| VPN-Bulk-Route (Torrent) | Mock | `torrent_mode = vpn_only` nötig; Magnet-Link nur auf der VPN-Route; Route-Chip und Warnung sichtbar |
| VPN-Route bei `blocked` | Mock | Hinzufügen wird mit `E_DOWNLOAD_ROUTE_FORBIDDEN` abgelehnt, Liste bleibt leer |
| Datei-Download | **echt** | lokaler HTTP-Server liefert 256 KiB, Playwright lädt sie herunter, SHA-256 wird geprüft |
| Aufräumen auf der Platte | **echt** | Download-Ordner und `.tmp` werden gelöscht, `existsSync` bestätigt das |

Der Datei-Teil benutzt bewusst einen lokalen Server auf `127.0.0.1` mit zufälligem Port. Kein Testverkehr verlässt den Rechner, und kein Test hängt von externen Diensten ab.

## Ablauf

1. `beforeAll`: `.tmp/downloads` anlegen, HTTP-Server auf einem freien Port starten, deterministische 256 KiB erzeugen.
2. UI-Test: App öffnen, einschalten, Downloads-Sheet öffnen, `https://example.org/sample.bin` hinzufügen, auf „Sauber" warten, ausschalten, Zusammenfassung prüfen, Liste leeren.
3. Datei-Test: Datei über HTTP holen, in `.tmp/downloads/sample.bin` schreiben, `manifest.json` mit SHA-256 schreiben, Hash erneut prüfen, Ordner löschen und Abwesenheit bestätigen.
4. `afterAll`: Server schließen, `.tmp` erneut löschen. Auch ein fehlgeschlagener Lauf hinterlässt keine Dateien.

## Ergebnis des letzten Laufs

```
Running 4 tests using 1 worker
✓ UI: enable, add a download, verdict, disable, clear the list (11.9s)
✓ bulk route: enable VPN torrents, add a magnet, clear the list (9.8s)
✓ bulk route is refused while torrents are blocked (6.5s)
✓ files: download to disk, verify the hash, remove everything (44ms)
4 passed (33.4s)
```

`ui/tests/e2e/.tmp` existiert danach nicht mehr.

## Desktop-Download (Tauri)

Gegen die echte App geprüft (`scripts/test-desktop-download.mjs`, WebView2-Debug-Port):

| Test | Ergebnis |
|---|---|
| `https://proof.ovh.net/files/1Mb.dat` über „Direkt" | 1.048.576 Bytes, Datei korrekt, danach gelöscht |
| `https://www.gnu.org/licenses/gpl-3.0.txt` über „Direkt" | 35.149 Bytes, Urteil „Unklar" |
| Maximiertes Fenster | `window-full` gesetzt, Radius 0 px |
| Wiederhergestellt | Radius 14 px |

„Unklar" ist an dieser Stelle richtig: Windows Defender läuft auf dem Host nicht (`AntivirusEnabled: False`), `MpCmdRun` endet mit `0x80004005`. Lieber „Unklar" als ein falsches „Sauber".

Nicht geprüft: Tor-Route (kein SOCKS-Proxy auf 9150), Abbruch mitten im Transfer, Scan mit laufendem Defender.

## Grenzen

- Der UI-Test läuft gegen den Mock, weil es noch keinen IPC-Server und keinen echten Downloader gibt. Er beweist den Ablauf und die UI-Zustände, nicht die aria2-/Tor-Kette.
- Der Datei-Test prüft Transport und Aufräumen, nicht den Virenscan. EICAR, MOTW und Quarantäne kommen mit P4-05/P4-06.
- Der Test braucht den Dev-Server. In CI startet Playwright ihn selbst; lokal wird ein laufender Server auf Port 5173 wiederverwendet.
