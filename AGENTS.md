# AGENTS.md: Gemeinsame Regeln für alle Agenten (TVPN Pro)

Projekt: Ein-Klick-Privacy-App für Windows (macOS später): Firewall-Allowlist, VPN → Tor, MAC-Wechsel, Tor Browser, Downloader mit Virenscan, Wächter. Master-Plan: `PLAN.md`. Vor jeder Arbeit lesen: diese Datei, deine Rollen-Datei, `contracts/ipc-v0.md`, deine Aufgaben in `docs/TASKS.md`, die zugehörigen Spikes in `docs/SPIKES.md`.

## Sicherheit (nicht verhandelbar)
- **Fail-closed:** Bei jedem Fehler bleibt das Netz gesperrt. Nie still ohne Schutz weiterlaufen.
- **Host-Schutz:** Alles mit Wirkung auf Netzwerk, Firewall, MAC, Hostname, Dienste oder Treiber läuft **nur in der Test-VM** über `tests/vm/run-in-vm.ps1`. Auf dem Host nur bauen, linten und Unit-Tests ohne Systemwirkung. Im Zweifel fragen.
- Privilegierte Aktionen nur im Dienst `tvpnd`. Die UI ist unprivilegiert und schickt nur feste Kommandos aus `contracts/ipc-v0.md`, **nie** Pfade, Programmnamen oder Argumente.
- Der Dienst startet nur Programme aus dem Installationsordner, nach Hash-Prüfung, ohne PATH-Lookup.
- Keine Telemetrie, kein Tracking-SDK, keine unnötigen Netzwerkaufrufe. Die echte IP wird nie abgefragt.
- **Nie loggen oder speichern:** URLs, Fenstertitel, Ziel-Domains/IPs, echte IP, volle MAC, Dateinamen (nur Hash-Präfix). Secrets (Mullvad-Kontonummer) nur im Windows Credential Manager.
- Fremd-Binaries nur mit fester Version und SHA-256. Neue Abhängigkeiten nur mit Eintrag in `docs/LICENSES.md`. Keine Secrets im Repo.
- Kein `unsafe` ohne Begründung im Code und Review durch qa-security.
- Im Tor Browser: keine Add-ons, kein Marionette/WebDriver, nur Prefs, die Webseiten nicht beobachten können.
- Behaupte nie "anonym", "sicher" oder "nicht zurückverfolgbar". Schreibe, was die App tut und wo ihre Grenzen liegen.

## Zusammenarbeit
- Jede Rolle ändert **nur ihre eigenen Ordner** (`PLAN.md` Abschnitt 3). `contracts/` ändert nur der Orchestrator. Änderungswünsche: Eintrag in `docs/contract-requests.md`.
- Ein Branch pro Rolle (`agent/<rolle>`) in eigenem git-Worktree. Kleine, häufige Commits (Conventional Commits, Englisch). Vor dem Merge auf `main` rebasen. Kein Force-Push auf `main`, keine Eingriffe in fremde Branches.
- Sprache: Doku und UI-Texte Deutsch, Code, Kommentare und Commits Englisch.
- Kontext sparen: Lies nur, was du für deine Aufgabe brauchst (zugehörige Plan-Abschnitte statt des ganzen Plans).

## Übergabeformat (`docs/handoffs/<task-id>.md`)
```
## Übergabe: <Rolle>, <Task-ID>
- Fertig:
- Getestet (Host/VM, Befehle, Ergebnis):
- Offen / Risiken:
- Brauche von anderen (Contract-Änderung?):
- Nächster Schritt:
```

## Definition of Done
- Tests grün, Leak-Test der Phase in der VM grün, keine neuen Warnungen.
- Doku in `docs/` aktualisiert, Entscheidungen und Annahmen notiert (`docs/DECISIONS.md`).
- Übergabe-Datei geschrieben, Status in `docs/TASKS.md` vom Orchestrator gesetzt.
- Keine Verletzung der Sicherheitsregeln oben.

## Verhalten
- Bei Unklarheit oder Sicherheitsfragen: **stoppen und den Menschen fragen**, nicht raten.
- Scheiterst du zweimal am selben Problem: Übergabe schreiben und fragen, nicht weiter probieren.
- Annahmen als Annahmen markieren. Unsicheres als Spike behandeln, nicht als fertig melden. Keine API-Details aus dem Gedächtnis erfinden: Docs lesen (falls ein Doku-Tool wie Context7 konfiguriert ist, nutzen).
- Destruktive Aktionen (Löschen, Force-Push, Systemsoftware installieren) nur nach Bestätigung.
- Erkläre dem Menschen Terminal-, Git-, Windows-Dienst- und Firewall-Konzepte, die du nutzt, in ein bis zwei Sätzen.
