---
description: Privacy shield. MAC address and hostname handling on enable and disable.
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
# Rolle: privacy-shield

## Ziel
Baue `service/privacy/`: Die MAC wechselt beim Einschalten und beim Ausschalten, der Hostname wird neutralisiert (je S3), das Original lässt sich wiederherstellen.

## Besitz
`service/privacy/`.

## Eingaben
`PLAN.md` Abschnitte 5 und 6.1, `docs/SPIKES.md` (S3), `contracts/ipc-v0.md`.

## Aufgaben
1. **Spike S3 zuerst:** Geht ein Hostname-Wechsel pro Sitzung ohne Neustart? Sonst Plan A (einmalig im Setup neutralisieren).
2. MAC pro physischem Adapter: lokal administriert, unicast, zufällig. Registry `NetworkAddress` plus Adapter-Neustart, Netz abwarten.
3. Original sicher merken. Modi "neu würfeln" (Default) und "Original zurück" (Heimnetz).
4. Treiber ohne Unterstützung erkennen, Banner `mac_unsupported` melden, Weiterlaufen nur mit Bestätigung.
5. Neue Adapter erkennen: bekommt eine neue MAC, bevor er genutzt wird.
6. OS-Code hinter dem Trait `MacChanger` mit `windows/` und leerem `macos/`.

## Constraints
- Nie während eines laufenden Tunnels wechseln: Reihenfolge laut PLAN Abschnitt 5 (nach Firewall-Armierung, vor VPN; beim Ausschalten nach VPN-Trennung, vor Entwaffnung).
- Keine virtuellen oder VPN-Adapter anfassen. Nur VM-Tests.

## Tests (Abnahme)
`ipconfig /all` vorher und nachher, mehrere Adapter, Wiederherstellung, Adapter ohne Unterstützung, Hostname laut S3.

## Stop-Bedingungen
Gefahr, dass der Adapter nach dem Wechsel nicht wieder hochkommt. Unklarer Neustart-Bedarf für den Menschen.

## Übergabe
`docs/handoffs/<task-id>.md`, `docs/privacy.md`.
