# Spikes (kurze Untersuchungen vor dem Bauen, risk-first)

Jeder Spike: Zeitbox, Test nur in der VM, Ergebnis als Eintrag in `docs/DECISIONS.md` und kurzer Bericht `docs/reports/spike-<id>.md`.

## S1: WFP-Filter neben Mullvad-Lockdown (net-core)
- **Frage:** Koexistieren unsere Filter (Default-Deny, `tor.exe` nur über den VPN-Adapter, Allowlist-Apps nur zu Loopback-Tor-Ports) mit dem Mullvad-Daemon im Lockdown-Modus? Gibt es die Interface-Bedingung auf der gewählten Filterebene, sonst Quell-IP?
- **Vorgehen:** Mullvad in der VM, Lockdown an, eigene Filter mit Provider-GUID, Abbruch, Reconnect und Serverwechsel simulieren, Filterliste auswerten.
- **Erfolg:** Kein Paket außerhalb von VPN und Tor, kein Aussperren, Recovery entfernt nur eigene Filter.
- **Plan B:** Mullvad-Lockdown als alleiniger Kill-Switch, unsere Filter nur für die App-Allowlist.
- **Stand 22.11.2026:** App-ID-Permit für `tor.exe` und den Mullvad-Daemon ist implementiert und in der VM mit echtem Tor geprüft (12 Filter). Die Interface-Bedingung fehlt weiter; ohne Mullvad-Konto ist die Koexistenz nicht testbar.

## S2: Tor Browser am System-Tor (launcher)
- **Frage:** Startet Tor Browser sauber mit externem Tor (Umgebungsvariablen, Control-Port-Auth)? Gehen "Neue Identität" und der Updater? Ändert sich etwas Web-Sichtbares?
- **Erfolg:** Kein zweites Tor, Fingerprint identisch zum Standard-Tor-Browser, "Neue Identität" funktioniert.
- **Plan B:** Tor Browser mit eigenem Tor hinter dem VPN, nur der Downloader am System-Tor.

## S3: Hostname ohne Neustart (privacy-shield)
- **Frage:** Gibt es einen Weg, den im Netz sichtbaren Rechnernamen (DHCP, NetBIOS, mDNS) pro Sitzung ohne Neustart zu ändern?
- **Erfolg:** Wechsel sichtbar im Netz (DHCP-Lease prüfen) ohne Neustart.
- **Plan A:** Einmaliges Neutralisieren im Setup (ein Neustart), pro Sitzung nur die MAC, UI-Hinweis.
- **Ergebnis (22.11.2026, VM `tvpn-test`):** `Rename-Computer` meldet Erfolg, aber `hostname`, NetBIOS und der DHCP-Hostname bleiben bis zum Neustart alt. Ergebnis `REBOOT_REQUIRED`. Bericht: `docs/reports/spike-s3.md`.

## S4: Downloader und Scanner-Engines (launcher, scanner)
- **Fragen:** (1) aria2 über den Tor-HTTP-CONNECT-Port, Resume nach Tor-Neustart. (2) Kann FDM über Proxy, ist Torrent aus? (3) ClamAV: RAM und Startzeit mit `clamd` und mit `clamscan`. (4) Defender-CLI vorhanden oder durch Dritt-AV ersetzt?
- **Erfolg:** Download über Port B, Resume klappt, Scan-Pfad für jedes Ergebnis klar.
- **Plan B:** Nur Defender oder nur ClamAV, FDM weglassen.

## S5: Block-Titel, URL, Fallback-Browser (launcher)
- **Fragen:** (1) Titel-Muster zuverlässig? (2) URL per UI Automation lesbar, ohne dass sich etwas Web-Sichtbares ändert? (3) Fallback-Browser nur über den VPN-Adapter erreichbar?
- **Erfolg:** Block erkannt, URL im Fallback-Browser geöffnet, Banner erscheint.
- **Plan B:** Manuelle Taste mit URL-Eingabe oder Zwischenablage.

## S6: Captive Portal unter Default-Deny (net-core)
- **Frage:** Wie lässt sich ein Portal-Login erledigen, ohne die Firewall aufzugeben? Wie erkennt man ein Portal zuverlässig?
- **Erfolg:** Simuliertes Portal in der VM: Login nur im Fallback-Browser, danach wieder voll scharf.
- **Plan B:** Portal-Login nur manuell vor dem Einschalten.

## S7: macOS (später, Phase 7)
- **Fragen:** Firewall-Ansatz (NetworkExtension-Filter vs. `pf` plus Tor-TransPort), MAC-Wechsel unter neueren Versionen, Berechtigungen für Fenstertitel, Notarisierung.

## S8: System-Modus mit TUN (optional, Phase 9)
- **Fragen:** TUN-Umleitung für Apps außerhalb der Allowlist: Routing-Loop, DNS, UDP, Leaks. Erst nach dem Audit.
