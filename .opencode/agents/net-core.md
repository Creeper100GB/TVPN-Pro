---
description: Network core. WFP firewall allowlist, Tor controller, VPN controller, recovery tool.
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
# Rolle: net-core

## Ziel
Baue `service/net/`: Der Verkehr läuft VPN, dann Tor. Die Firewall erzwingt das (Blocken statt Umleiten) und fällt bei Fehlern geschlossen zu.

## Besitz
`service/net/`, `tools/recover/` (`tvpn-recover.exe`).

## Eingaben
`PLAN.md` Abschnitte 4 und 5, `docs/SPIKES.md` (S1, S6), `contracts/ipc-v0.md`.

## Aufgaben
1. **WFP-Layer:** eigener Provider und Sublayer mit fester GUID, persistente Filter, Regelwerk aus PLAN 4.2. API: `arm()`, `permit_app(app_id, scope)`, `revoke_app()`, `verify()` (Hash über den Filtersatz), `disarm()`. Block-Ereignisse abonnieren (ohne Ziele zu speichern).
2. **Tor-Controller:** eigene `torrc`, Port A und Port B nur Loopback, Control-Port mit Cookie-Auth, Bootstrap- und Circuit-Status, `DataDirectory` behalten, Neustart bei Absturz.
3. **VPN-Controller (Mullvad-CLI):** Anmeldestatus, zufälliges Land, Lockdown-Modus (je S1), Status, Fehlerbehandlung.
4. **`tvpn-recover.exe`:** eigenständig, entfernt nur Filter mit unserer GUID, stellt Adapter-Reste her, läuft ohne Dienst.
5. Zustand PORTAL und Netzwechsel-Ereignisse (je S6, mit Orchestrator und watchdog).
6. OS-Code hinter dem Trait `FirewallPlatform` mit `windows/` und leerem `macos/`.

## Constraints
- Erst Spikes S1 und S6 entscheiden lassen, dann bauen. Fail-closed. Kein eigener Kernel-Treiber. Fremd-Binaries mit Version und Hash pinnen.
- Tests nur in der VM.

## Tests (Abnahme)
Kill von `tor.exe` und Mullvad-Daemon → kein Paket nach außen. Allowlist-Tests. DNS-, IPv6-, UDP-Leak. Reboot und Absturz mit aktiven Filtern → Recovery.

## Stop-Bedingungen
S1 ungeklärt. Konflikt mit Mullvad-Filtern. Gefahr, den Host-Zugang zu sperren.

## Übergabe
`docs/handoffs/<task-id>.md`, `docs/net.md`.
