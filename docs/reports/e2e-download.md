# E2E: Download-Flow und Aufräumen

Stand: 21. November 2026. Test: `ui/tests/e2e/downloads.spec.ts`. Ausführen mit `just e2e` oder `bun --cwd ui run e2e`.

## Was der Test wirklich prüft

| Teil | Real oder Mock | Prüfung |
|---|---|---|
| Schutz ein/aus, Zustände, Banner | Mock (kein Dienst, kein Netz) | Klick auf „Einschalten" führt zu „Geschützt"; „Ausschalten" öffnet die Sitzungs-Zusammenfassung |
| Download-Liste, URL-Prüfung, Fortschritt, Urteil | Mock | HTTPS-Pflicht, Port 443, keine IP-Literale, keine lokalen Namen; Eintrag erreicht „Sauber" |
| Sitzungs-Zusammenfassung | Mock, aber aus der Download-Liste berechnet | Zähler „Sauber" ist 1, nicht hartkodiert |
| Aufräumen in der App | Mock | „Liste leeren" hinterlässt den leeren Zustand |
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
Running 2 tests using 1 worker
✓ UI: enable, add a download, verdict, disable, clear the list (12.3s)
✓ files: download to disk, verify the hash, remove everything (76ms)
2 passed (13.4s)
```

`ui/tests/e2e/.tmp` existiert danach nicht mehr.

## Grenzen

- Der UI-Test läuft gegen den Mock, weil es noch keinen IPC-Server und keinen echten Downloader gibt. Er beweist den Ablauf und die UI-Zustände, nicht die aria2-/Tor-Kette.
- Der Datei-Test prüft Transport und Aufräumen, nicht den Virenscan. EICAR, MOTW und Quarantäne kommen mit P4-05/P4-06.
- Der Test braucht den Dev-Server. In CI startet Playwright ihn selbst; lokal wird ein laufender Server auf Port 5173 wiederverwendet.
