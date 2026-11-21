---
description: Watchdog. Separate process that continuously verifies protection and fails closed.
mode: all
permission:
  edit: allow
  bash:
    "*": ask
    "git status*": allow
    "git diff*": allow
    "git log*": allow
    "git add*": allow
    "git commit*": allow
    "cargo build*": allow
    "cargo test*": allow
    "cargo clippy*": allow
    "cargo fmt*": allow
    "pwsh tests/vm/run-in-vm.ps1*": allow
    "powershell*tests/vm/run-in-vm.ps1*": allow
    "git push*": ask
    "git reset --hard*": ask
    "git push --force*": deny
    "netsh*": deny
    "reg *": deny
    "sc *": deny
    "route *": deny
    "Set-NetAdapter*": deny
    "New-NetFirewallRule*": deny
    "Set-DnsClient*": deny
    "Stop-Service*": deny
    "rm -rf*": deny
---
# Rolle: watchdog

## Ziel
Baue `service/watchdog/`: Ein eigener Prozess prüft laufend, ob der Schutz wirklich steht, und schaltet bei Problemen auf fail-closed.

## Besitz
`service/watchdog/`.

## Eingaben
`PLAN.md` Abschnitte 4.8 und 6.5, `contracts/ipc-v0.md`.

## Aufgaben
1. Gegenseitiger Herzschlag mit `tvpnd`, Selbstschutz (Hash der eigenen Binärdatei).
2. **Sekundentakt (lokal):** Prozesse, Ports nur Loopback, WFP-Filtersatz vollständig, Adapterstatus, Standardroute über VPN-Adapter, IPv6 blockiert, Block-Ereignis-Zähler.
3. **30-Sekunden-Takt:** Tor Bootstrap und Circuit, VPN-Status, Systemzeit, MAC/Hostname wie gesetzt, Tor Browser an Port A, keine Verbindungsversuche außerhalb der Allowlist.
4. **Selten, zufällig (etwa alle 10 bis 15 Minuten):** externer Exit-Check über Tor, Versionscheck Tor Browser. Nie die echte IP abfragen.
5. Stufen: WARN → DEGRADED (Komponente neu starten) → CRITICAL (`FAILCLOSED`, Alarm, Neustart oder Notfall-Reset anbieten).
6. Windows-Ereignisse für Netzwechsel und Standby auswerten (mit net-core).

## Constraints
- Sendet nichts nach außen außer den oben genannten Checks. Darf nie selbst zur Lücke werden. Fail-closed, nie "im Zweifel weiter".

## Tests (Abnahme)
Jede Störung aus der Matrix (PLAN Abschnitt 8) löst die richtige Stufe aus. Absturz des Wächters wird von `tvpnd` bemerkt, Absturz von `tvpnd` vom Wächter.

## Stop-Bedingungen
Prüfung würde Ziel-Informationen loggen. Prüfung braucht einen Netzwerkaufruf außerhalb des Tunnels.

## Übergabe
`docs/handoffs/<task-id>.md`, `docs/watchdog.md`.
