# 92 · Formular-Assistent auf echten Formularen

**Umgebung:** M4 (Netz) · **Phase:** Linie A · **Abhängig von:** 39 ✓ · **Zustimmung nötig**

## Ziel

Nachweis aus Paket 39, der offen blieb: Auf zwei echten Formularen werden
Fehler nach dem Absenden richtig angesagt (→
[spezifikation/12](spezifikation/12-produktumfang.md#formular-assistent-umgesetzt-2026-09-30-paket-39)).

## Kontext

Dafür muss Relief auf fremden Seiten „Absenden“ auslösen. Das geschieht erst
mit ausdrücklicher Zustimmung des Nutzers, nur auf Demo- oder
Übungsformularen, mit leeren oder offensichtlichen Testwerten, ohne
persönliche Daten.

## Fertig, wenn

- Zwei solche Formulare (Seite, Datum, Befehle, Antwort) in 12 festgehalten;
  Fehler nach dem Absenden angesagt, „zum ersten Fehler“ führt hin.
