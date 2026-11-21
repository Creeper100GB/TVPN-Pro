---
description: Network core. WFP firewall allowlist, Tor controller, VPN controller, recovery tool.
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
