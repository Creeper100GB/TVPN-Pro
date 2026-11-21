---
description: Privacy shield. MAC address and hostname handling on enable and disable.
mode: all
permissions:
  - action: edit
    resource: "*"
    effect: allow
  - action: external_directory
    resource: "*"
    effect: ask
  - action: shell
    resource: "*"
    effect: ask
  - action: shell
    resource: "git status *"
    effect: allow
  - action: shell
    resource: "git diff *"
    effect: allow
  - action: shell
    resource: "git log *"
    effect: allow
  - action: shell
    resource: "git add *"
    effect: allow
  - action: shell
    resource: "git commit *"
    effect: allow
  - action: shell
    resource: "cargo build *"
    effect: allow
  - action: shell
    resource: "cargo test *"
    effect: allow
  - action: shell
    resource: "cargo clippy *"
    effect: allow
  - action: shell
    resource: "cargo fmt *"
    effect: allow
  - action: shell
    resource: "pwsh *run-in-vm.ps1 *"
    effect: allow
  - action: shell
    resource: "powershell *run-in-vm.ps1 *"
    effect: allow
  - action: shell
    resource: "git push *"
    effect: ask
  - action: shell
    resource: "git reset --hard *"
    effect: ask
  - action: shell
    resource: "git push --force *"
    effect: deny
  - action: shell
    resource: "netsh *"
    effect: deny
  - action: shell
    resource: "reg *"
    effect: deny
  - action: shell
    resource: "sc *"
    effect: deny
  - action: shell
    resource: "route *"
    effect: deny
  - action: shell
    resource: "Set-NetAdapter *"
    effect: deny
  - action: shell
    resource: "New-NetFirewallRule *"
    effect: deny
  - action: shell
    resource: "Set-DnsClient *"
    effect: deny
  - action: shell
    resource: "Stop-Service *"
    effect: deny
  - action: shell
    resource: "rm -rf *"
    effect: deny
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
