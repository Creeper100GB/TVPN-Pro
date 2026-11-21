# Aufgabe: TVPN Pro, Phase 0 (Projekt aufsetzen)

## Ziel
Richte das Projekt gemäß `PLAN.md` ein und bereite die Spikes vor. Stoppe danach und warte auf meine Abnahme (Gate G0).

## Kontext
- Lies in dieser Reihenfolge: `AGENTS.md`, `PLAN.md`, `contracts/ipc-v0.md`, `docs/SPIKES.md`, `docs/TASKS.md`, `docs/DECISIONS.md`.
- Das Projekt ist eine Ein-Klick-Privacy-App für Windows (macOS später). Es ist eine Grüne Wiese, nur dieses Kit existiert.
- Ich bin der Mensch im Projekt. Erkläre Terminal-, Git-, Windows-Dienst- und Firewall-Konzepte, die du nutzt, in ein bis zwei Sätzen.

## Tech-Stack
Rust (Dienst), Tauri (UI), PowerShell (Tools), Windows 11 zuerst, opencode mit git-Worktrees.

## Anforderungen (Must-have)
1. Prüfe `opencode --version` und gleiche die Frontmatter der Agent-Dateien in `.opencode/agents/` mit den Docs dieser Version ab. Korrigiere bei Bedarf und notiere es in `docs/DECISIONS.md`.
2. Arbeite die Aufgaben P0-01 bis P0-07 in `docs/TASKS.md` ab und setze den Status.
3. Schreibe `docs/THREAT_MODEL.md` (aus PLAN Abschnitt 2), `docs/LICENSES.md` (Gerüst) und `docs/canary.md`.
4. Prüfe `contracts/ipc-v0.md` auf Lücken und ergänze sie.
5. Schreibe `tests/vm/README.md` und `tests/vm/run-in-vm.ps1` für meine VM-Software (frage mich zuerst, ob Hyper-V, VirtualBox oder VMware).
6. Gib mir die Toolchain-Liste (Rust, Node LTS, Tauri-CLI, Installer-Tools) und sage, was ich selbst installieren muss.
7. Schlage die Reihenfolge der Spikes S1 bis S6 und die ersten Worktree-Rollen vor.

## Constraints
- Alle Regeln aus `AGENTS.md`. Nichts mit Netzwerk-, Firewall-, MAC-, Hostname- oder Dienstwirkung auf dem Host ausführen.
- Keine Installation von Systemsoftware ohne meine Bestätigung. Kein Force-Push.
- Keine Behauptungen wie "anonym" oder "nicht zurückverfolgbar".

## Deliverable
Committetes Repo-Gerüst, aktuelle `docs/`, Bericht `docs/reports/phase-0.md` (Ergebnis, offene Risiken, was ich tun muss).

## Offene Fragen an mich
Frage mich nur, was du wirklich nicht selbst klären kannst (z. B. VM-Software). Danach: **stoppen und auf G0 warten.**
