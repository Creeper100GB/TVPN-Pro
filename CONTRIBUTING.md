# Mitmachen

Danke, dass du reinschaust. Das Projekt ist eine Alpha und lebt von Leuten, die Windows, Tor oder Rust wirklich kennen. Du kannst auf Deutsch oder Englisch schreiben.

## Bevor du anfängst

- Lies `PLAN.md` (Architektur und Phasen), `AGENTS.md` (Sicherheitsregeln) und `docs/THREAT_MODEL.md`.
- Alles, was Netzwerk, Firewall, MAC, Hostname, Dienste oder Treiber anfasst, läuft **nur in der Test-VM**. Auf dem Host wird gebaut, gelintet und unit-getestet.
- Keine Sicherheitsversprechen. Nirgends „anonym", „sicher" oder „nicht zurückverfolgbar".
- Fail-closed: Bei Fehlern bleibt das Netz zu.

## Einrichten

```powershell
git clone https://github.com/Creeper100GB/TVPN-Pro.git
cd TVPN-Pro
just verify
cd ui
bun install
bun run check
bun run e2e
```

Für Rust-Tests braucht es die MSVC-Umgebung; `just test` erledigt das über `tools/with-msvc.ps1`.

## Wo du anfangen kannst

Issues mit `help wanted` oder `good first issue`. Besonders gesucht:

1. **WFP-Firewall** (`service/net`): persistente Filter mit eigener GUID, Default-Deny, Interface-Bedingung, Recovery. Erst nach Spike S1.
2. **Tor-Controller** (`service/net`): eigener `tor.exe`, DataDirectory, Control-Port mit Cookie, Bootstrap-Status.
3. **Dienst und IPC** (`service/core`): Named Pipe mit ACL und Token, `tvpnd`, Logging mit Redaktions-Layer.
4. **Launcher** (`service/launcher`): Tor Browser am System-Tor, aria2 über Port B, Block-Erkennung.
5. **QA** (`tests/`): Leak-Matrix in der VM, Canary, EICAR, Fuzzing.
6. **macOS** (Phase 7): NetworkExtension oder `pf`, Notarisierung.

## Arbeitsweise

- Ein Branch pro Rolle (`agent/<rolle>`) in einem eigenen Worktree, kleine Commits.
- Conventional Commits, Englisch. Doku und UI-Texte Deutsch.
- Jede Rolle ändert nur ihre Ordner. `contracts/` ändert nur der Orchestrator; Wünsche in `docs/contract-requests.md`.
- Vor dem Merge: rebase auf `main`, Tests grün, Leak-Test der Phase grün.
- Keine neuen Abhängigkeiten ohne Eintrag in `docs/LICENSES.md`.
- Kein `unsafe` ohne Begründung und Review.

## Pull Request

- Was ändert sich und warum?
- Welche Tests liefen, wo (Host oder VM)?
- Neue Risiken oder Grenzen?
- Screenshots bei UI-Änderungen.

Kleine, prüfbare PRs kommen schneller durch als große Umbauten.

## Sicherheitslücken

Nicht als öffentliches Issue. Siehe `SECURITY.md`.
