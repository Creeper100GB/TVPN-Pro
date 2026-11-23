# Contract-Änderungswünsche

Rollen ändern `contracts/` nicht selbst. Sie schreiben hier einen Eintrag und arbeiten mit dem aktuellen Vertrag weiter.

Format:

```
## <Task-ID>, <Rolle>, <Datum>
- Betrifft: contracts/ipc-v0.md, Abschnitt …
- Wunsch:
- Warum:
- Dringlichkeit: blockiert / nicht blockierend
- Ohne die Änderung mache ich:
```

## CR-01, Service, 2026-11-22

- Betrifft: contracts/ipc-v0.md, Abschnitt 7 (URL-Regeln)
- Wunsch: `.localdomain` und der bloße Host `localdomain` in die Liste der
  lokalen Suffixe aufnehmen.
- Warum: `service/core/src/url.rs` lehnt beides bereits ab (`LOCAL_SUFFIXES`
  enthält `.localdomain`, `validate_host` prüft `localdomain`). Der Vertrag
  nennt nur `localhost`, `.local`, `.internal`, `.lan` und `.home.arpa`; Code
  und Vertrag sind damit nicht deckungsgleich.
- Dringlichkeit: nicht blockierend
- Ohne die Änderung mache ich: weiter ablehnen (strenger als der Vertrag) und
  die Abweichung hier dokumentieren.

## CR-02, Service, 2026-11-22

- Betrifft: contracts/ipc-v0.md, Abschnitt 5 (`disable`) und Abschnitt 8
  (Übergänge)
- Wunsch: festhalten, dass `disable` aus einem hängenden `TEARDOWN` den Abbau
  erneut versucht (shutdown + disarm) und bei Erfolg `accepted: true` mit
  `state: OFF` antwortet; bei erneutem Fehler bleibt es fail-closed in
  `TEARDOWN` mit `failclosed`-Banner.
- Warum: Im hängenden `TEARDOWN` bietet die UI den Notfall-Reset an; der
  Ausschalt-Knopf ist von dort gar nicht erreichbar (die UI ignoriert ihn in
  `TEARDOWN`). `disable` kommt nur über einen Desync an diesen Punkt, also wenn
  die UI einen anderen Zustand zeigt als der Dienst. Für genau diesen Desync ist
  der zweite Versuch das Sicherheitsnetz — nicht der Normalfall, und nicht der
  Grund, den Vertrag zu ändern.
- Dringlichkeit: nicht blockierend
- Ohne die Änderung mache ich: den Abbau wie beschrieben wiederholen und die
  Abweichung dokumentieren.

## CR-03, UI, 2026-11-22

- Betrifft: contracts/ipc-v0.md, Abschnitt 5 (`panic`) und Abschnitt 8
  (Übergänge)
- Wunsch: bei `panic` festhalten, dass es das Netz **öffnet** (Tor und VPN
  beendet, Filter entfernt), nicht „Netz zu".
- Warum: `contracts/ipc-v0.md` beschreibt `panic` als „Apps zu, Netz zu,
  Teardown". Die ausgelieferte App macht das Gegenteil und sagt es auch:
  `README.md` (Zeile „Panik") und `docs/STATE.md` beschreiben „das Netz
  ist danach **offen**", und der Knopf heißt „Panik: Schutz beenden, Netz
  öffnen". Ein Vertrag, der hier „Netz zu" verspricht, behauptet einen
  Kill-Switch, den es nicht gibt.
- Dringlichkeit: nicht blockierend (Doku-Widerspruch, kein Codefehler)
- Ohne die Änderung mache ich: weiter das ehrliche Verhalten zeigen und die
  Abweichung hier dokumentieren.

## Status

Drei offene Wünsche (CR-01, CR-02, CR-03), alle nicht blockierend.
