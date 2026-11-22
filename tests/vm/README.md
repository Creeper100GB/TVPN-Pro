# Test-VM (Windows 11, Hyper-V)

Alles, was Netzwerk, Firewall, MAC, Hostname, Dienste oder Treiber anfasst, laeuft **nur** in dieser VM. Der Runner startet die VM, spielt ein Skript ein, sammelt die Ausgabe ein und stellt den sauberen Zustand wieder her.

## Begriffe in einem Satz

- **Hyper-V**: Microsofts Hypervisor in Windows; eine VM ist ein kompletter Rechner in einer Datei.
- **Checkpoint**: Hyper-V-Snapshot einer VM (Festplatten- und Speicherzustand); ein Restore verwirft alles, was seit dem Checkpoint in der VM passiert ist.
- **PowerShell Direct**: Kanal vom Host in den Gast ueber den Hyper-V-Bus (VMBus), ohne Netzwerk, ohne Freigaben, ohne offene Ports.
- **VMBus**: der interne Nachrichtenkanal zwischen Host und Gast, der auch ohne Netzwerkkarte funktioniert.

## Voraussetzungen

- Windows 11 Pro/Enterprise mit aktiviertem Hyper-V (`Get-VM` liefert eine Liste, kein Fehler).
- Eine PowerShell **als Administrator** (Hyper-V-Cmdlets und PowerShell Direct brauchen das).
- Eine Windows-11-VM mit:
  - aktivierten Integrationsdiensten (Standard),
  - einem **lokalen** Administratorkonto, z. B. `tvpn` (PowerShell Direct akzeptiert keine reinen Microsoft-Konto-Logins),
  - Netzwerkadapter an einem Switch, der fuer die Tests gebraucht wird (bei Leak-Tests: der normale Internet-Switch).
- Ein Checkpoint im sauberen Zustand, Standardname `sauber`:
  ```powershell
  Checkpoint-VM -Name tvpn-test -SnapshotName sauber
  ```

## Automatische Einrichtung

Mit einem Windows-11-ISO legt dieses Skript die VM `tvpn-test` an, installiert Windows 11 Pro unbeaufsichtigt, erstellt das lokale Admin-Konto `tvpn`, setzt den Checkpoint `sauber` und führt den Smoke-Test aus:

```powershell
powershell -ExecutionPolicy Bypass -File tests\vm\create-vm.ps1 -IsoPath C:\Users\Jonah\VMs\Win11_Client_x64_en-us.iso
```

Das Skript braucht eine Admin-Shell, Hyper-V und `oscdimg.exe` aus den Windows-ADK-Deployment-Tools (Default: `C:\Program Files (x86)\Windows Kits\10\Assessment and Deployment Kit\Deployment Tools\amd64\Oscdimg\oscdimg.exe`). Die Antwortdatei liegt auf einer zweiten DVD, damit Setup die OS-Platte als einzige Festplatte sieht. Das Skript fasst nur die Dateien unter `-VMPath` an und verweigert jeden Datenträger, der kein file-backed virtuelles Laufwerk ist. Das Standard-Passwort des Testkontos steht im Skript; es gilt nur für die wegwerfbare VM.

## Ersteinrichtung (manuell)

1. Hyper-V-Manager oeffnen, neue VM anlegen (Generation 2), Windows 11 installieren.
2. Im Gast ein lokales Konto `tvpn` mit Adminrechten anlegen (`net user tvpn * /add` und `net localgroup administrators tvpn /add`).
3. Im Gast alle Testwerkzeuge installieren, die die Tests brauchen (Mullvad-App, Tor, Browser, aria2, ClamAV).
4. VM herunterfahren, dann den Checkpoint setzen:
   ```powershell
   Checkpoint-VM -Name tvpn-test -SnapshotName sauber
   ```
5. Checkpoint pruefen:
   ```powershell
   powershell -ExecutionPolicy Bypass -File tests\vm\run-in-vm.ps1 -ListCheckpoints
   ```

## Benutzung

```powershell
# 1) Runner selbst testen (aendert nichts im Gast)
powershell -ExecutionPolicy Bypass -File tests\vm\run-in-vm.ps1 -Script tests\vm\smoke.ps1

# 2) Eigener Test mit Argumenten und Beispieldateien
powershell -ExecutionPolicy Bypass -File tests\vm\run-in-vm.ps1 `
    -Script tests\vm\leak-test.ps1 `
    -ScriptArguments '-Case','ipv6' `
    -Include tests\fixtures

# 3) VM nach dem Lauf offen lassen (Debugging), kein Revert
powershell -ExecutionPolicy Bypass -File tests\vm\run-in-vm.ps1 -Script tests\vm\smoke.ps1 -KeepRunning -NoRevert
```

Der Exitcode des Skripts ist der Exitcode des Gasts. `0` heisst bestanden, alles andere nicht. Jeder Lauf schreibt ein Protokoll nach `tests/vm/out/` (per `.gitignore` ausgeschlossen).

## Was der Runner tut

1. Prueft, dass `-VMName` nicht der Host ist.
2. Stoppt die VM hart und stellt den Checkpoint `sauber` wieder her (ausser `-NoRevert`).
3. Startet die VM und wartet auf Heartbeat.
4. Fragt einmalig nach dem lokalen Admin-Konto des Gasts.
5. Kopiert das Skript (und `-Include`) nach `C:\tvpn-test` im Gast.
6. Fuehrt es in einem frischen `powershell.exe` aus, mit `-NonInteractive`.
7. Gibt Ausgabe und Exitcode zurueck.
8. Schaltet die VM aus (ausser `-KeepRunning`). Der naechste Lauf beginnt wieder beim Checkpoint.

## Was der Runner **nicht** tut

- Er aendert auf dem Host nichts: keine Firewall-Regel, keine Route, kein Adapter, kein Dienst.
- Er laedt nichts aus dem Netz.
- Er speichert keine Zugangsdaten. Das Konto wird pro Lauf abgefragt.
- Er ist **keine** Sandbox: ein Gast-Skript laeuft mit Adminrechten in der VM und kann dort Schaden anrichten. Das ist gewollt und der Grund fuer den Checkpoint.

## Regeln fuer Tests

- Vor jedem Test gilt der Checkpoint, nicht der Zustand von gestern.
- Ein Test darf den Gast nicht so hinterlassen, dass der naechste Test davon abhaengt.
- Protokolle aus der VM duerfen keine echten IPs, MACs, URLs oder Dateinamen enthalten, die im Produkt verboten sind. Wenn ein Test solche Werte braucht, schreibt er sie als Hash oder gar nicht.
- Tests, die einen Neustart brauchen, machen ihn im Gast selbst und melden das Ergebnis danach.

## Fehlersuche

| Symptom | Ursache |
|---|---|
| `Get-VM` nicht gefunden | Hyper-V-Feature fehlt oder keine Admin-Shell. |
| `Checkpoint 'sauber' fehlt` | Checkpoint nicht angelegt oder anderer Name. |
| `Wait-VM ... Heartbeat` laeuft in Timeout | Gast haengt beim Boot, Integrationsdienste aus, oder der Checkpoint ist kaputt. |
| `PowerShell Direct ... fehlgeschlagen` | Kein lokales Admin-Konto, falsches Passwort, oder Gast noch nicht bereit. |
| Skript laeuft, aber Exitcode 0 trotz Fehler | Das Gast-Skript muss `exit 1` setzen; `Write-Error` allein zaehlt nicht. |

## Andere VM-Software

Der Runner ist auf Hyper-V ausgelegt. VirtualBox (`VBoxManage guestcontrol`, `snapshot restore`) und VMware (`vmrun`) koennten spaeter als eigene Adapter dazukommen; solange es nur einen Pfad gibt, bleibt es bei Hyper-V.
