# Entscheidungen (ADR-Log)

Neue Einträge hängt nur der Orchestrator an. Status: `Vorschlag` oder `Abgenommen` (vom Menschen).

| ID | Entscheidung | Begründung | Überdenken, wenn | Status |
|---|---|---|---|---|
| D1 | Rust (Dienst) plus Tauri (UI) | Sicher, ein Binary, Web-Skills des Menschen nutzbar | Plattformgrenzen bei macOS-Erweiterungen | Vorschlag |
| D2 | **Fokus-Modus:** Blocken statt Umleiten, kein TUN in v1 | Weniger Angriffsfläche, kein Loop, kein DNS-Leak durch Umleitung | Nutzer will wirklich alle Apps über Tor (System-Modus, S8) | Vorschlag |
| D3 | Reihenfolge VPN → Tor | Provider sieht Tor-Nutzung nicht | Tor-Projekt oder Anbieter raten klar ab | Vorschlag |
| D4 | Mullvad als VPN (Kontonummer, anonyme Zahlung) | Kein Personenbezug nötig, CLI vorhanden | Konflikte mit WFP (S1) | Vorschlag |
| D5 | aria2 als Standard-Downloader, FDM als Adapter | Open Source, per RPC steuerbar, Proxy per HTTP-CONNECT | S4 zeigt Probleme | Vorschlag |
| D6 | Mullvad Browser als Fallback-Browser | Für VPN-Nutzung gebaut, gleiche Fingerprint-Schutz-Basis | S5 zeigt Probleme | Vorschlag |
| D7 | Scanner orchestriert Defender und ClamAV, kein eigener Engine-Bau | Ehrlicher und wartbar | Defender nicht verfügbar | Vorschlag |
| D8 | Hostname einmalig neutralisieren (Plan A) | Neustart pro Sitzung unpraktisch | S3 findet Weg ohne Neustart | Vorschlag |
| D9 | Block-Erkennung per Fenstertitel, kein Add-on, kein WebDriver | Fingerprint bleibt unverändert | S5 findet bessere Methode | Vorschlag |
| D10 | Logging: Dev-Logs ohne sensible Daten, Release aus | Debugbar ohne Spuren | Datenschutzprobleme in Tests | Vorschlag |
| D11 | Dienstkonto: so wenig Rechte wie möglich (offen, LocalSystem oder eigenes Konto) | Angriffsfläche | Technisch nicht machbar | Offen, Orchestrator klärt in Phase 1 |
| D12 | Mullvad-Lockdown als Basis oder eigene WFP-Basis | Eine Quelle der Wahrheit | S1 | Offen, entscheidet S1 |
