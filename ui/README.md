# UI-Shell (Prototyp)

Statische Svelte-5-Oberfläche mit Mock-Daten. Es gibt noch keine IPC-Verbindung zum Dienst; alle Zustände werden lokal simuliert. Das ist Absicht: Phase 0/1 bauen keine Netzwerkfunktionen, und die UI muss gegen einen Mock prüfbar sein.

## Stack

| Baustein | Version | Zweck |
|---|---|---|
| Svelte | 5.59 | Komponenten, Runes |
| SvelteKit | 3.4 | Routing, statischer Build |
| Vite | 8.3 | Build und Dev-Server |
| Tailwind CSS | 4.3 | Utilities, Theme-Tokens |
| TypeScript | 6.0 | Typen (SvelteKit 3 braucht die JS-Compiler-API) |
| Lucide | 1.0 | Icons |
| Inter Variable | 5.3 | Schrift, lokal gebündelt (keine CDN) |
| Playwright | 1.65 | Screenshots und Layout-Prüfung (nur Dev) |

## Befehle

```bash
bun install
bun run dev        # http://localhost:5173
bun run check      # svelte-check, 0 Fehler erwartet
bun run build      # statischer Build nach build/
bun run preview    # baut nicht, serviert build/ auf 4173
bun run shots      # Screenshots nach .artifacts/ (preview muss laufen)
```

## Aufbau

- `src/routes/+page.svelte`: Fenster, Hero, Seitenleiste, Demo-Leiste.
- `src/lib/mock.svelte.ts`: Zustandsautomat und Abläufe (Einschalten, Ausschalten, Panik, Recovery) mit Timern.
- `src/lib/states.ts`: Beschriftungen, Hinweise und Farbton je Zustand.
- `src/lib/components/`: PowerButton, Banner, StatRow, Segmented, Toggle, Sheet, SettingsSheet, SessionSummary.
- `src/lib/theme.svelte.ts`: Auto/Hell/Dunkel, gespeichert in `localStorage`.

## Design-Regeln

- macOS-Anmutung: weiche Rundungen (14 bis 30 px), Blur, Haarlinien, großzügiger Weißraum, Inter mit negativer Laufweite.
- Keine eckigen Windows-Controls, keine Systemfarben, keine Emojis.
- Ein großer Button. Alles andere ist Status oder Einstellung.
- Farbe bedeutet etwas: grün geschützt, amber eingeschränkt, rot gesperrt, blau arbeitet.
- Ehrliche Texte. Nirgends „anonym“, „sicher“ oder „nicht zurückverfolgbar“.
- A11y: `role="switch"`, `aria-pressed`, `aria-live` für den Zustand, Fokusringe, 44-px-Ziele, `prefers-reduced-motion`.

## Demo-Leiste

Der Button unten rechts (`Demo`) schaltet jeden Zustand direkt. „Portal-Schritt simulieren“ hängt den PORTAL-Zustand in den Einschaltablauf. Die Sitzungs-Zusammenfassung lässt sich dort ebenfalls öffnen. Die Leiste verschwindet, sobald echte IPC-Daten da sind.

## Noch nicht enthalten

- Tauri-Fenster und IPC (folgt mit dem Dienst, Phase 2).
- Tray-Icon, Panik-Hotkey, echte Downloads, echte Länderliste.
- Persistenz der Einstellungen (nur Theme wird gespeichert).
