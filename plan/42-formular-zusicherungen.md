# 42 · Formular-Zusicherungen im Aufgabenformat

**Umgebung:** Cloud · **Phase:** Linie B · **Abhängig von:** —

## Ziel

Das Aufgabenformat (`spike/tasks/*.txt`) um Zusicherungen erweitern, die den
Bedienablauf eines Formulars aus Sicht von Screenreader- und
Tastaturnutzenden prüfen (→
[spezifikation/12](spezifikation/12-produktumfang.md#linie-b-prüfen-im-echten-browser)).

## Zusicherungen

- jedes im AXTree exponierte Feld hat einen nichtleeren zugänglichen Namen;
- wenn DOM-Fakten vorliegen: Chromiums Name stimmt mit der Berechnung durch
  `accname` überein oder die Abweichung wird als eigener Befund erklärt;
- Fehlermeldung ist mit dem Feld verknüpft und nach dem Absenden sichtbar;
- Fokus landet nach fehlerhaftem Absenden auf dem ersten Fehler;
- ein Bestätigungsdialog besitzt zugänglichen Namen, verständliche
  Planparameter, initialen Fokus und Abbruchweg; das Ergebnis wird anschließend
  als gebündelte Statusmeldung ausgegeben;
- eine im Aufgabenfall festgelegte Tab-Folge erreicht alle erwarteten Felder;
  keine allgemeine Gleichheit von Tab- und Lesereihenfolge unterstellen.

## Datenbasis

- AX-/Graph-Fakten: berechneter Name, Rolle, Zustände, Relationen, Fokus und
  Live-Regionen aus `SemanticGraph` und Delta.
- DOM-Fakten: Tag und für die Prüfung nötige Attribute wie `id`, `for`,
  `aria-*` und `tabindex` in einem browserneutralen `a11y-dom`-Modell. Der Host
  erhebt sie; `relief-interaction` erhält Daten, holt sie aber nicht selbst.
- `accname` arbeitet auf diesen DOM-Fakten. Der AX-Graph allein enthält weder
  alle Eingaben der Namensberechnung noch `tabindex`.

## Schritte

1. Browserneutrale Eingabe für die zusätzlichen DOM-Fakten festlegen und im
   CDP-Host befüllen; keine CDP- oder Chromium-Typen im Kern.
2. Zusicherungen browserfrei in `relief-interaction` gegen Graph, Delta und
   – nur wo nötig – DOM-Fakten; `a11y-dom`, `accname` und Diff-Regeln aus
   `a11y-perception` nutzen, nicht nachbauen.
3. Tab-Folgen als beobachtete Schritte mit expliziter Erwartung im
   Aufgabenformat prüfen, nicht aus der Lesereihenfolge ableiten.
4. Testseiten mit je einem richtigen und einem kaputten Formular in
   `spike/fixtures/`.
5. Befunde im Format von `a11y-report`.

## Fertig, wenn

- Kaputte Formulare liefern je Zusicherung einen Befund, richtige keinen;
  Prüfbefehle aus `CLAUDE.md` grün.

## Nicht Teil

Ablegen in barrierlab, solange kein zweiter Konsument da ist (→ 46).
