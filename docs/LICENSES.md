# Lizenzen und Fremdkomponenten (Gerüst)

**Stand: Phase 0, ungeprüft.** Dieses Dokument ist ein Gerüst. Die Spalte „Lizenz“ enthält Erwartungen aus der Projektplanung, **keine** geprüften Angaben. Vor jeder Weitergabe (USB-Kit, Installer, Release) wird jede Zeile gegen die `LICENSE`-Datei der gepinnten Version geprüft und hier mit Datum und Quelle eingetragen.

## Regeln

1. Neue Abhängigkeit oder neue Fremd-Binärdatei: **erst** Eintrag hier, dann Code.
2. Jede Fremd-Binärdatei wird mit fester Version, SHA-256 und, wo vorhanden, Signatur gepinnt.
3. GPL-Komponenten werden nur gebündelt, wenn die Pflichten erfüllt sind (Quellangebot, Lizenztext, keine zusätzlichen Einschränkungen).
4. Keine Komponente ohne Lizenztext im Installer oder im USB-Kit.
5. Dieses Dokument ist keine Rechtsberatung. Bei Unsicherheit: nicht ausliefern, fragen.

## Fremd-Binärdateien (werden mitgeliefert)

| Komponente | Zweck | Version (gepinnt) | SHA-256 | Signatur | Lizenz (Erwartung) | Pflichten | Status |
|---|---|---|---|---|---|---|---|
| tor (Expert Bundle) | Tor-Client | offen | offen | offen | BSD-3-Clause | Lizenztext, Quellhinweis | ungeprüft |
| Tor Browser | Browser für Tor | offen | offen | GPG (torproject.org) | MPL-2.0 (mit Komponenten) | Lizenztexte, Quellhinweis | ungeprüft |
| Mullvad VPN App | VPN-Daemon | offen | offen | offen | GPL-3.0 | vollständiger Quelltext oder schriftliches Angebot, Lizenztext | ungeprüft, GPL-Prüfung nötig |
| Mullvad Browser | Fallback-Browser | offen | offen | offen | MPL-2.0 | Lizenztexte, Quellhinweis | ungeprüft |
| aria2 | Downloader | offen | offen | offen | GPL-2.0-or-later | Quelle, Lizenztext | ungeprüft, GPL-Prüfung nötig |
| ClamAV | Virenscanner (optional) | offen | offen | offen | GPL-2.0 | Quelle, Lizenztext | ungeprüft, GPL-Prüfung nötig |
| NSIS | Installer-Bundler (Build-Zeit) | offen | – | – | zlib/libpng | Lizenztext, wenn mitgeliefert | ungeprüft |

Nicht mitgeliefert, nur genutzt: Windows Defender (`MpCmdRun.exe`) ist Teil von Windows und unterliegt der Windows-Lizenz. Es wird nur aufgerufen, nicht gebündelt.

## Bibliotheken (Build-Zeit, im Binary)

| Bibliothek | Zweck | Lizenz (Erwartung) | Status |
|---|---|---|---|
| Rust standard library / Kern-Crates | Dienst | MIT oder Apache-2.0 (pro Crate) | ungeprüft |
| Tauri | UI-Rahmen | MIT oder Apache-2.0 | ungeprüft |
| Svelte / SvelteKit | UI | MIT | ungeprüft |
| Vite | Build | MIT | ungeprüft |
| Tailwind CSS | UI | MIT | ungeprüft |
| Lucide | Icons | ISC | ungeprüft |
| Inter (Fontsource) | Schrift, lokal gebündelt | OFL-1.1 | ungeprüft |
| TypeScript | Build | Apache-2.0 | ungeprüft |
| Playwright | Visuelle Prüfung, nur Dev-Abhängigkeit | Apache-2.0 | ungeprüft |
| serde, serde_json (direkt) | IPC-Serialisierung | MIT oder Apache-2.0 | ungeprüft |
| base64 (tvpnd) | Token-Kodierung (base64url) | MIT oder Apache-2.0 | ungeprüft |
| getrandom (tvpnd) | CSPRNG für das Sitzungs-Token | MIT oder Apache-2.0 | ungeprüft |
| windows-service (tvpnd) | Windows-Dienst-Wrapper | MIT oder Apache-2.0 | ungeprüft |
| static_vcruntime (tvpnd, Build-Zeit) | Statisch gelinkte VC-Runtime für den Installer | MIT OR Apache-2.0 OR Zlib (Erwartung) | ungeprüft — Prüfprotokoll fehlt: vor dem nächsten Release Lizenzfeld der Crate-Metadaten für 3.0.0 mit Datum und Quelle hier eintragen |
| reqwest, tokio, sha2, futures-util (Tauri-Shell) | Download-Engine | MIT oder Apache-2.0 | ungeprüft |
| Transitive Crates laut `Cargo.lock` (itoa, memchr, zmij, proc-macro2, quote, syn, unicode-ident, serde_core, serde_derive, hyper, native-tls u. a.) | Build und Laufzeit | MIT oder Apache-2.0 (pro Crate) | ungeprüft, `cargo deny` folgt in Phase 2 |
| Windows-/`windows`-Crate | Win32-APIs | MIT oder Apache-2.0 | ungeprüft |

`cargo deny` und `npm audit` werden in Phase 2 Teil der CI; die vollständige Liste entsteht aus `Cargo.lock` und `package-lock.json` und wird hier nachgetragen.

## Eigene Lizenz

Das Projekt steht unter **GPL-3.0-or-later**. Der vollständige Text liegt in [`LICENSE`](../LICENSE). Beiträge werden unter derselben Lizenz eingebracht. Die GPL passt zu den gebündelten GPL-Komponenten (Mullvad-App, aria2, ClamAV) und hält Verbesserungen offen.

## Prüfprotokoll

| Datum | Prüfer | Was geprüft | Ergebnis |
|---|---|---|---|
| – | – | – | – |

## Offene Punkte

1. GPL-Verträglichkeit der Bündelung klären (Mullvad-App, aria2, ClamAV) — vor Phase 6.
2. Quelltextangebot für GPL-Komponenten vorbereiten (Text + Ablageort).
3. Hashes und Signaturen beim Pinnen eintragen (Phase 4/6).
