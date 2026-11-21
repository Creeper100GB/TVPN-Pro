# Threat Model: TVPN Pro

Stand: Phase 0. Grundlage: `PLAN.md` Abschnitt 2. Dieses Dokument beschreibt, **wovor** die App schützt, **wie**, und **was übrig bleibt**. Es ist kein Versprechen.

## 1. Was die App ist

Ein Werkzeug, das auf einem Windows-Rechner die Spuren im Netz reduziert: ausgehender Verkehr nur über VPN und Tor, MAC-Wechsel, unveränderter Tor Browser, Downloads mit Scan, laufende Selbstprüfung. Sie ersetzt kein vorsichtiges Verhalten und keine Betriebssicherheit des Geräts.

**Sprachregel:** Nirgends steht „anonym“, „sicher“ oder „nicht zurückverfolgbar“. Formulierung: „reduziert Spuren, ersetzt kein vorsichtiges Verhalten.“

## 2. Assets

| Asset | Warum es zählt | Wo es liegt |
|---|---|---|
| Echte IP-Adresse | verknüpft Person mit Aktivität | beim ISP, beim Ziel |
| MAC-Adresse | verknüpft Gerät mit Netz | im lokalen Netz, in DHCP-Leases |
| Hostname | verknüpft Sitzungen im selben Netz | DHCP, NetBIOS, mDNS |
| Ziele und Zeiten | verraten Interessen und Gewohnheiten | ISP, VPN-Anbieter, DNS |
| Heruntergeladene Dateien | Schadcode, Metadaten, Wasserzeichen | Datenträger, Download-Ordner |
| Mullvad-Kontonummer | Zugang und Bezahlhistorie | Credential Manager |
| Logs | können alles oben rekonstruieren, wenn sie es enthalten | `%ProgramData%\TVPN\logs` |

## 3. Vertrauensgrenzen

```
[Mensch] → [UI, unprivilegiert] --IPC (Pipe, ACL, Token)--> [Dienst tvpnd, privilegiert]
                                                              ├── Mullvad-Daemon
                                                              ├── tor.exe
                                                              ├── WFP-Filter (eigene GUID)
                                                              ├── Scanner (Defender, ClamAV)
                                                              └── Launcher (Hash-geprüft)
[Wächter, eigener Prozess] <--Herzschlag--> [tvpnd]
```

- Die UI ist **nicht** vertrauenswürdig für Privilegien: sie darf nur feste Kommandos senden, keine Pfade, Programmnamen oder Argumente.
- Der Dienst ist die einzige Stelle mit Systemwirkung.
- Der Wächter ist ein zweiter Zeuge; fällt einer aus, schlägt der andere Alarm.
- Alles außerhalb dieser Kette (Browser, Downloads, fremde Apps) ist **nicht** vertrauenswürdig.

## 4. Annahmen

- Windows 11 ist nicht kompromittiert. Ein Kernel-Angreifer oder Administrator-Schadcode hebt jede Schutzmaßnahme auf.
- Der Mensch benutzt die App bestimmungsgemäß: keine Logins mit Klarnamen im Tor-Fenster, keine persönlichen Dateien in Downloads, keine Add-ons im Tor Browser.
- Der VPN-Anbieter (Mullvad) ist ehrlich genug, nicht zu loggen. Das ist eine Annahme, kein Beweis.
- Tor funktioniert wie spezifiziert; das Tor-Netz ist nicht global beobachtet.
- Die Test-VM ist wegwerfbar; auf dem Host läuft keine dieser Maßnahmen.

## 5. Beobachter

| Beobachter | Sieht ohne App | Maßnahme der App | Restrisiko |
|---|---|---|---|
| Website / Tracker | IP, Fingerprint, Cookies | Tor-Exit-IP, Tor Browser unverändert, getrennte Konten | Verhalten, Browser-0-Days, Logins |
| Lokales Netz / Café | MAC, Hostname, WLAN-Aktivität | MAC-Wechsel, Firewall blockt alles Fremde | Hostname (S3), Portal sieht das Gerät, Anwesenheit |
| ISP | Ziele, Tor-Nutzung, Mengen | VPN zuerst | sieht VPN-Nutzung, Mengen, Zeiten |
| VPN-Anbieter | echte IP, Tor-Nutzung | Konto ohne Personenbezug, anonyme Zahlung | „No-log“ ist ein Versprechen, kein Beweis |
| Tor-Relays | Entry: VPN-IP; Exit: Klartext | VPN vor Tor, HTTPS-only | Korrelation bei beobachteten Enden |
| Andere Software, Downloads | alles, was Apps melden | Firewall-Allowlist, Scanner | 0-Days, IP-verratende Dokumente nach dem Ausschalten |
| Torrent-Peers (nur VPN-Bulk-Route) | IP, Port, Client-Kennung, übertragene Blöcke | ausschließlich VPN-Adresse, DHT/LPD/PEX/UPnP aus, keine eingehenden Verbindungen | Peer sieht die VPN-IP und das Image, der VPN-Anbieter sieht den Traffic |
| Zugriff aufs Gerät | lokale Spuren | Aufräumen, Pflicht-Hinweis Plattenverschlüsselung | Beschlagnahme, Forensik, Boot-Spuren |

### 5.1 Website und Tracker

Die App zwingt den Tor Browser durch Tor und blockt direkte Wege. Der Browser bleibt unverändert: gleiche Prefs, keine Add-ons, kein Marionette/WebDriver, gleiche Fenstergröße. Was die Seite sieht, ist die Exit-IP und der Standard-Fingerprint. **Nicht** verhindert: Verhalten (Schreibstil, Uhrzeiten, Muster), Logins, die eine Person zuordnen, und Fehler im Browser selbst.

### 5.2 Lokales Netz und öffentliche WLANs

MAC-Wechsel pro Sitzung und Adapter, Firewall blockt alles außer Loopback, VPN und Tor. Der Hostname bleibt die schwächste Stelle: Windows übernimmt einen neuen Namen meist erst nach einem Neustart. Plan A neutralisiert ihn einmalig im Setup; S3 klärt, ob mehr geht. Ein Captive Portal sieht das Gerät zwangsläufig, bevor irgendetwas geschützt ist; der Zustand PORTAL macht das sichtbar und zeitlich begrenzt.

### 5.3 ISP

Ohne VPN sieht der ISP Ziele und Zeiten. Mit VPN sieht er nur eine verschlüsselte Verbindung zu einem VPN-Endpunkt und Mengen/Zeiten. Das ist eine Verbesserung, keine Unsichtbarkeit: VPN-Nutzung selbst ist sichtbar, und Zeitkorrelation bleibt möglich.

### 5.4 VPN-Anbieter

Der Anbieter sieht die echte IP und die Tatsache, dass Tor benutzt wird. Die App verlangt keine Identität, sondern nur eine Kontonummer, die anonym bezahlt werden kann. Ob der Anbieter loggt, entscheidet er, nicht die App. Ein kompromittierter oder kooperierender Anbieter bricht die Kette.

### 5.5 Tor

VPN vor Tor verhindert, dass ein Tor-Guard die echte IP sieht. Der Exit sieht den Klartext, wenn keine Ende-zu-Ende-Verschlüsselung (HTTPS) besteht. Wer beide Enden beobachtet, kann korrelieren; dagegen hilft die App nicht.

### 5.6 Fremde Software und Downloads

Die Firewall lässt nur Allowlist-Apps zu Tor-Ports auf Loopback. Alles andere bleibt offline, solange die App an ist. Downloads werden geprüft, aber ein Scanner findet keine 0-Days. Mark-of-the-Web und Quarantäne reduzieren Risiko, sie beseitigen es nicht. Nach dem Ausschalten schützt die Firewall nicht mehr: ein Dokument, das beim Öffnen nachlädt, verrät dann die echte IP. Das steht in der UI.

### 5.7 Zugriff auf das Gerät

Wer das Gerät in die Hand bekommt, findet Spuren: Browser-Profile, Download-Ordner, Windows-Ereignisprotokolle, Prefetch, Volume Shadow Copies. Die App räumt definierte Dinge auf (Browser-Session, temporäre Dateien, Logs im Release). Sie ist kein Forensik-Werkzeug und keine Festplattenverschlüsselung. Plattenverschlüsselung ist Pflicht, nicht Option.

### 5.8 Torrents und die VPN-Bulk-Route

Torrents laufen ausschließlich über den VPN-Adapter (`torrent_mode = vpn_only`, Standard `blocked`). Über Tor wäre es Netzmissbrauch und ein Leak-Risiko: DHT, Local Peer Discovery, Peer Exchange und UPnP würden versuchen, an Tor vorbei zu gehen. Auf der VPN-Route sind sie abgeschaltet, eingehende Verbindungen ebenfalls.

Was das bedeutet, ehrlich:
- Peers sehen die VPN-IP, nicht die echte. Sie sehen auch, welches Image geladen wird.
- Der VPN-Anbieter sieht den Torrent-Verkehr und kann ihn mit deinem Konto verknüpfen, wenn du kein anonymes Konto nutzt.
- Der ISP sieht nur VPN-Traffic, aber Menge und Zeit.
- Es gibt **keinen** Tor-Schutz auf dieser Route. Das ist VPN-Privatsphäre, keine Anonymität. Der Banner `vpn_route` sagt das, solange die Route aktiv ist.

## 6. Angriffe auf die App selbst

| Angriff | Gegenmaßnahme | Status |
|---|---|---|
| Fremder Prozess sendet IPC-Kommandos | Pipe-ACL pro Benutzer, Sitzungs-Token, feste Kommandoliste, Schema-Validierung, Rate-Limit | geplant Phase 2 |
| UI wird kompromittiert | UI hat keine Privilegien; Dienst startet nur Hash-geprüfte Programme aus dem Installationsordner, kein PATH-Lookup | geplant Phase 2 |
| Manipulierte Fremd-Binärdatei | feste Version, SHA-256, Signaturprüfung (Tor Browser per GPG) | geplant |
| Lieferkette (Crates, npm) | `Cargo.lock`, `package-lock.json`, `cargo audit`, `cargo deny`, `npm audit`, wenige Abhängigkeiten | CI-Skelett vorhanden |
| Logs verraten Ziele | Redaktions-Regeln, CI-Gate gegen IP/MAC/URL, Release-Logs aus | Gate-Stub vorhanden |
| Dienst läuft als zu mächtiges Konto | D11: minimales Konto prüfen | offen |
| Absturz hinterlässt offene Filter | persistente Filter, `tvpn-recover.exe`, Start-Recovery | geplant Phase 2 |
| Wächter wird abgeschaltet | eigener Prozess, gegenseitiger Herzschlag, Hash-Schutz | geplant Phase 5 |

## 7. Was die App ausdrücklich nicht leistet

- Kein Schutz vor Gegnern, die beide Enden des Tor-Netzes beobachten.
- Kein Schutz vor Browser-0-Days oder kompromittierten Downloads.
- Kein Schutz vor eigenen Fehlern: Logins, Schreibstil, Dateien öffnen, Metadaten.
- Kein Schutz vor einem kompromittierten Windows oder Administrator-Schadcode.
- Keine Unsichtbarkeit im lokalen Netz (Hostname, Anwesenheit, Portal).
- Keine Garantie, dass ein VPN-Anbieter nicht loggt.
- Keine Rechtsberatung. Anonymität ändert nicht, was legal ist.

## 8. Offene Punkte

| Punkt | Klärung |
|---|---|
| Koexistenz unserer WFP-Filter mit Mullvad-Lockdown | Spike S1 |
| Tor Browser am System-Tor ohne Fingerprint-Änderung | Spike S2 |
| Hostname ohne Neustart | Spike S3 |
| Downloader und Scanner unter Tor | Spike S4 |
| Block-Erkennung und URL-Beschaffung | Spike S5 |
| Captive Portal unter Default-Deny | Spike S6 |
| Dienstkonto | D11, Phase 1 |
| macOS | Spike S7, Phase 7 |

Pflege: Dieses Dokument wird am Ende jeder Phase und spätestens beim Abschluss-Audit (Phase 8) gegen den tatsächlichen Code geprüft. Änderungen nur mit Eintrag in `docs/DECISIONS.md`.
