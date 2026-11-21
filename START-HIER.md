# TVPN Pro Kit: Start hier

Dieses Kit enthält einen Master-Plan, gemeinsame Regeln und acht Agenten für opencode. Du startest **einen** Agenten (den Orchestrator). Der richtet alles ein und sagt dir, wann du weitere Agenten in eigenen Worktrees startest.

## Was du vorher brauchst
- Windows 11, Git, Windows Terminal, opencode
- Eine **Test-VM** (Windows 11 in Hyper-V, VirtualBox oder VMware) mit einem Snapshot "sauber". Alles, was Netzwerk, Firewall oder MAC anfasst, läuft nur dort.
- Ein Mullvad-Konto (nur Kontonummer, anonym bezahlbar) und die Mullvad-App
- Optional, aber sehr hilfreich: ein kleiner **Canary-Endpunkt** auf deinem eigenen Server, der die Quell-IP jeder Verbindung anzeigt (Anleitung schreibt der Orchestrator)

## Schritt für Schritt
1. Das Kit liegt schon im Repo. Klone es (Lernmoment: `git clone` kopiert ein Repo samt Verlauf auf deinen PC):
   ```
   git clone https://github.com/Creeper100GB/TVPN-Pro.git
   cd TVPN-Pro
   ```
2. Kein `git init` nötig, der erste Commit ist schon da (Worktrees brauchen mindestens einen).
3. `opencode --version` ausführen. Notiere die Version, der Orchestrator gleicht die Agent-Dateien damit ab.
4. `opencode --agent orchestrator` starten (oder `opencode` und per Tab-Taste "orchestrator" wählen). Dann den Inhalt von `KICKOFF-PROMPT.md` einfügen.
5. Der Orchestrator macht Phase 0 und **stoppt**. Lies `docs/THREAT_MODEL.md` und `docs/DECISIONS.md` und gib ihm das Okay (Gate G0).
6. Danach läuft es über `docs/TASKS.md`. Der Orchestrator sagt dir, welche Rollen du startest, z. B.:
   ```
   powershell -ExecutionPolicy Bypass -File tools\start-worktrees.ps1 -Roles net-core,privacy-shield,launcher
   ```
   Maximal 3 bis 4 Rollen gleichzeitig. Jede bekommt einen eigenen Tab und Branch.
7. Wenn eine Rolle fertig ist, schreibt sie eine Übergabe in `docs/handoffs/`. Der Orchestrator merged und räumt auf (`tools\cleanup-worktrees.ps1`).

## Deine Regeln
- **Nie auf dem Host testen**, was Netzwerk, Firewall, MAC, Hostname oder Dienste ändert. Immer VM, immer Snapshot vorher.
- Phasen-Abnahmen (Gates) sind deine Entscheidung. Der Orchestrator darf nicht einfach weiterlaufen.
- Rolle hängt oder dreht sich im Kreis? Abbrechen, Übergabe schreiben lassen, neu starten.
- Wenn opencode über die Frontmatter der Agent-Dateien meckert: Die Formate unterscheiden sich je nach Version. Sag dem Orchestrator die Fehlermeldung, er korrigiert die Dateien.
- Die App ist ein Werkzeug, kein Tarnumhang. Nirgends "anonym" oder "sicher" versprechen.
- Anonymität ändert nichts daran, was legal ist.

## Dateiübersicht
| Datei | Wofür |
|---|---|
| `PLAN.md` | Master-Plan, Architektur, Phasen, Tests, Risiken |
| `AGENTS.md` | Gemeinsame Regeln für alle Agenten |
| `KICKOFF-PROMPT.md` | Startnachricht für den Orchestrator |
| `.opencode/agents/` | Orchestrator plus 7 Rollen |
| `contracts/ipc-v0.md` | Schnittstellenvertrag zwischen UI und Dienst |
| `docs/TASKS.md`, `DECISIONS.md`, `SPIKES.md` | Board, Entscheidungen, Machbarkeitsprüfungen |
| `tools/` | Skripte für Worktrees und Aufräumen |
