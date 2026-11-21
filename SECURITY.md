# Sicherheit

## Status

Alpha. Es gibt keine unterstützte Version, und die Schutzfunktionen (Firewall, VPN, Tor-Integration) sind noch nicht implementiert. Ein „Security-Bug" kann also auch einfach ein fehlendes Feature sein. Bitte prüfe zuerst `README.md` und `docs/THREAT_MODEL.md`.

## Lücke melden

- Nutze **GitHub Security Advisories**: Security-Tab → „Report a vulnerability". Nicht als öffentliches Issue.
- Beschreibe: betroffene Komponente, Version/Commit, Reproduktion, Auswirkung, ob die VM nötig ist.
- Keine echten IPs, MACs, URLs oder Dateinamen in Berichten. Hashes und Codes genügen.

## Was erwartet werden kann

- Best effort, keine Bug-Bounty, keine Garantie auf eine Frist.
- Wir melden zurück, ob wir das Problem reproduzieren können, und nennen eine Einschätzung.
- Bitte keine Details veröffentlichen, bevor ein Fix oder eine gemeinsame Ankündigung da ist.

## Im Scope (Beispiele)

- Umgehung der IPC-Regeln, Rechteausweitung über den Dienst.
- Filter, die sich ohne `tvpn-recover` entfernen lassen, oder Recovery, das fremde Filter löscht.
- Hash-Prüfung, die sich umgehen lässt, oder Prozessstart außerhalb des Installationsordners.
- Logs oder UI, die echte IPs, MACs, URLs oder Dateinamen preisgeben.
- Pfad-Ausbrüche beim Download (`..`, absolute Pfade, NTFS-Streams).
- Torrent-Route, die ohne VPN Verbindungen zulässt.

## Außerhalb des Scopes

- Fehlende Features (Firewall, VPN, Tor, MAC, Wächter).
- Grenzen von Tor, VPN-Anbietern oder Browsern, die im Threat Model stehen.
- Angriffe, die Administrator- oder Kernel-Rechte voraussetzen.
- Social Engineering.

## Sichere Tests

Alles mit Netzwerk-, Firewall-, MAC- oder Dienstwirkung nur in der Test-VM. Auf dem Host: bauen, linten, Unit-Tests. Siehe `tests/vm/README.md`.
