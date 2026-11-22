# Spike S3: Hostname ohne Neustart

- Datum: 22. November 2026
- VM: `tvpn-test`, Windows 11 Enterprise Evaluation 22621, Checkpoint `sauber`
- Runner: `tests/vm/run-in-vm.ps1`, Gastkonto `tvpn`
- Log: `tests/vm/out/20261122T145119Z-spike-s3.log`

## Vorgehen

1. `Rename-Computer -NewName tvpn-s3-test -Force`
2. Direkt danach `hostname`, `nbtstat -n` und `ipconfig /all` lesen.
3. `ipconfig /release` und `ipconfig /renew`, damit der DHCP-Client den neuen Namen anbietet.
4. Werte vergleichen; keine IP-Adresse und keine MAC-Adresse ausgeben.

## Ergebnis

- `Rename-Computer`: erfolgreich.
- `hostname` danach: `WinDev2407Eval`, unverändert.
- NetBIOS: `WINDEV2407EVAL <00> UNIQUE`, `WORKGROUP <00> GROUP`.
- DHCP Host Name: `WinDev2407Eval`.
- `reboot_required`: `true`.
- Fazit: `REBOOT_REQUIRED`.

Der Checkpoint wurde nach dem Test wiederhergestellt, die Umbenennung ist damit verworfen.

## Folgerung

Ein im Netz sichtbarer Hostname-Wechsel ohne Neustart ist mit Bordmitteln nicht möglich. D8 bleibt gültig: Hostname einmalig im Setup neutralisieren (ein Neustart), pro Sitzung nur die MAC wechseln und in der UI auf den Neustart hinweisen.
