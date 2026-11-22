# VM-Test: Dienst, WFP und Tor

Datum: 22. November 2026
VM: `tvpn-test`, Windows 11 Enterprise Evaluation 22621, Checkpoint `sauber`
Gastkonto: `tvpn` (lokaler Admin)
Runner: `tests/vm/run-in-vm.ps1`

## Aufbau

- `tvpnd.exe` als Konsolenprozess und als installierter Windows-Dienst.
- Tor Expert Bundle 15.0.26, SHA-256 `c1988c701de2f24f7d098d1d6d7f507b58f27740d097f29c246e0524d639b252`, stimmt mit `sha256sums-signed-build.txt` von torproject.org überein.
- `vpn_enabled=false` für den Test, weil kein Mullvad-Konto vorhanden ist. Der Dienst geht damit bewusst in `DEGRADED`, nicht in `READY`.

## Ergebnisse

1. **Service-Smoke** (`service-smoke.ps1`): Pipe, Token, `hello`, `get_status` funktionieren. `enable` ohne Tor/Mullvad liefert `E_PREFLIGHT_MISSING_COMPONENT`/`tor`, der Zustand ist `FAILCLOSED`, `panic` geht nach `OFF`. Log: `tests/vm/out/20261122T150846Z-service-smoke.log`.
2. **Enable-Smoke** (`enable-smoke.ps1`): Mit Tor und `vpn_enabled=false` liefert `enable` `DEGRADED`. Tor-Bootstrap 100, Circuit steht, `shield.firewall_armed=true`, `firewall_filters=12` (8 Default-/Loopback, 4 Tor-Permit). `Test-NetConnection 1.1.1.1:443` ist blockiert. `panic` setzt `firewall_filters=0`, Zustand `OFF`, danach ist das Netz wieder erreichbar. Log: `tests/vm/out/20261122T151301Z-enable-smoke.log`.
3. **Dienstinstallation** (`service-install.ps1`): `tvpnd --install` legt den Dienst an, schreibt `owner.sid` und startet ihn als SYSTEM. Der Benutzer `tvpn` liest das Token und bekommt über die Pipe `state=OFF`. `--uninstall` entfernt den Dienst. Log: `tests/vm/out/20261122T152426Z-service-install.log`.
4. **UI** (Host, Mock-Dienst): Die Release-App liest Status und Einstellungen über die Tauri-Befehle; nach einem externen `enable` zeigt sie `Geschützt` und die Verbindungsleiste. Der Klick auf den Power-Knopf wurde per synthetischer Maus nicht ausgelöst; das ist eine Grenze des Testharnischs, nicht des IPC-Pfads. Screenshot: `C:\Users\Jonah\VMs\ui-rel2-ready.png`.

## Grenzen

- Keine Mullvad-App und kein Konto: VPN-zu-Tor-Kette und S1-Interface-Bedingung sind nicht geprüft.
- Der WFP-Permit ist App-ID-basiert. Fällt das VPN aus, kann `tor.exe` theoretisch über den physischen Adapter weiterlaufen, bis der Status-Poll-Wächter `FAILCLOSED` auslöst. Das ist kein vollständiger Leak-Schutz.
- MAC-Wechsel: Der Hyper-V-Adapter meldet `unsupported`; der echte Adapter-Write fehlt.
- Hostname: S3 `REBOOT_REQUIRED`, im Betrieb unverändert; Banner `hostname_unchanged`.
