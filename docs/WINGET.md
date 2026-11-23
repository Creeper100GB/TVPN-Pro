# Winget

## Was jetzt geht

Das Manifest für `0.1.6` liegt unter
`packaging/winget/manifests/c/Creeper100GB/TVPNPro/0.1.6/` und ist mit
`winget validate` geprüft.

Lokal aus dem geklonten Repository installieren:

```powershell
winget install --manifest packaging\winget\manifests\c\Creeper100GB\TVPNPro\0.1.6
```

Winget lädt dabei den Installer aus dem GitHub-Release, prüft die SHA-256 und
startet den NSIS-Installer. Der Installer ist perMachine und verlangt UAC.

## Installationsvalidierung (0.1.6)

Die erste Winget-Validierung scheiterte nicht am Manifest, sondern am
Postinstall-Schritt: `tvpnd.exe --install` benötigte `VCRUNTIME140.dll`, die auf
dem frischen Validierungs-VM fehlte. Windows zeigte einen modalen
Systemfehler-Dialog; der Installer wartete zwei Stunden und Winget meldete
„Installation did not complete within the timeout“.

`tvpnd` linkt die VC-Runtime seit dem Fix statisch (`static_vcruntime`,
`service/tvpnd/build.rs`). Die Import-Tabelle des gebündelten `tvpnd.exe`
nennt nur `api-ms-win-crt-*` (UCRT, Teil von Windows), kein
`VCRUNTIME140.dll`; das gilt auch für `tvpn-pro.exe`. Der korrigierte Installer hat
die SHA-256
`f224318717c1c043bdb18270f6faf116b94d0bdc1b795c1a3e156a46ce2cf24f`;
das Release-Asset und `SHA256SUMS.txt` wurden entsprechend aktualisiert.

Die Microsoft-CLA ist davon unabhängig und im PR bereits bestätigt
(`license/cla` grün, Stand 23.11.2026). Alle zehn Validierungsschritte sind
durchgelaufen; offen ist nur noch das Review durch einen Maintainer.

## Normale Installation und Updates

Damit `winget install --id Creeper100GB.TVPNPro` und
`winget upgrade --id Creeper100GB.TVPNPro` ohne lokale Manifestdatei
funktionieren, muss das Manifest im Community-Repository
`microsoft/winget-pkgs` liegen. Ein Pull Request ist vorbereitet; die Aufnahme
entscheidet das Winget-Team.

Nach der Aufnahme:

```powershell
winget install --id Creeper100GB.TVPNPro -e
winget upgrade --id Creeper100GB.TVPNPro -e
```

Winget aktualisiert nicht im Hintergrund. Der Nutzer stößt das Upgrade an.

### Wenn die App über GitHub installiert wurde

Winget erkennt installierte Programme über den Add/Remove-Programs-Eintrag.
Der Installer schreibt:

- DisplayName: `TVPN Pro`
- Publisher: `TVPN Pro Contributors` (Release `0.1.6`)
- DisplayVersion: `0.1.6`

Der Publisher im ARP-Eintrag kommt aus der beim Build gesetzten Firmenangabe
(`ui/src-tauri/tauri.conf.json`). Das veröffentlichte Release `0.1.6` wurde
noch mit `TVPN Pro Contributors` gebaut; das Manifest für `0.1.6` nennt
deshalb genau diesen Publisher. `ZenSquirrel` wird erst der ARP-Name, wenn
ein Installer neu gebaut und veröffentlicht ist, also ab dem nächsten
Release.

Das Manifest enthält genau diese `AppsAndFeaturesEntries`. Deshalb kann
`winget upgrade` auch eine über GitHub installierte `0.1.5` auf `0.1.6`
aktualisieren, sobald das Paket in der Winget-Quelle liegt. Die Herkunft der
Erstinstallation ist für das Upgrade nicht entscheidend; entscheidend ist der
ARP-Eintrag und die Version.

Prüfen:

```powershell
winget list TVPN
winget upgrade --id Creeper100GB.TVPNPro -e
```

## Neuen Release veröffentlichen

1. Version in `Cargo.toml`, `ui/package.json`,
   `ui/src-tauri/tauri.conf.json`, `ui/src-tauri/Cargo.toml` und
   `ui/src/lib/version.ts` erhöhen.
2. Tag `v<version>-alpha` pushen. Der Release-Workflow baut den Installer.
3. Installer-Hash aus `SHA256SUMS.txt` des Releases übernehmen.
4. Den Ordner `0.1.6` nach `.../TVPNPro/<version>/` kopieren und in allen drei
   YAML-Dateien `PackageVersion`, `InstallerUrl` und `InstallerSha256` anpassen.
5. `winget validate --manifest <ordner>` ausführen.
6. Die drei Dateien in einen Fork von `microsoft/winget-pkgs` legen und einen
   Pull Request öffnen. Pfad:
   `manifests/c/Creeper100GB/TVPNPro/<version>/`.

### Publisher im ARP-Eintrag umstellen

`ui/src-tauri/tauri.conf.json` setzt den Publisher künftiger Builds auf
`ZenSquirrel`. Weil der Publisher im ARP-Eintrag aus dem gebauten Installer
kommt, gilt: Installer neu bauen, den neuen Hash aus `SHA256SUMS.txt`
übernehmen, das Release-Asset ersetzen und danach `InstallerSha256` sowie
`AppsAndFeaturesEntries.Publisher` (plus `Publisher`/`Author`/`Copyright` im
Locale-Manifest) auf den neuen Wert ziehen. Ein Manifest darf nie einen
Publisher nennen, den der ausgelieferte Installer nicht schreibt — sonst
findet `winget upgrade` den ARP-Eintrag nicht.

## Grenzen

- Der Build ist unsigniert. SmartScreen warnt weiterhin.
- Winget ersetzt keine Code-Signierung und keinen eigenen Update-Kanal.
- Winget ist kein Hintergrund-Updater; ohne `winget upgrade` passiert nichts.
- Die App ist Alpha. Die VPN-Kette ist ohne Mullvad-Konto ungeprüft; MAC,
  Hostname und Tor Browser sind nicht fertig. Keine Anonymitätsgarantie.
