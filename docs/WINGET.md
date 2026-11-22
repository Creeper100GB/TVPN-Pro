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
- Publisher: `TVPN Pro Contributors`
- DisplayVersion: `0.1.5` oder `0.1.6`

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

## Grenzen

- Der Build ist unsigniert. SmartScreen warnt weiterhin.
- Winget ersetzt keine Code-Signierung und keinen eigenen Update-Kanal.
- Winget ist kein Hintergrund-Updater; ohne `winget upgrade` passiert nichts.
- Die App ist Alpha. Die VPN-Kette ist ohne Mullvad-Konto ungeprüft; MAC,
  Hostname und Tor Browser sind nicht fertig. Keine Anonymitätsgarantie.
