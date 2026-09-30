# Aufnahmen

Accessibility-Tree-Aufnahmen echter und eigener Seiten als Fixtures für
browserfreie Tests (`crates/relief-interaction/tests/`). Entstanden mit

```bash
cargo run -p relief-cdp -- record spike/tasks/0*.txt spike/tasks/10-real.txt
cargo run -p relief-cdp -- record spike/tasks/11-korpus.txt --headful
```

am 2026-09-24, Google Chrome 154.0.8037.58, macOS (M4).

## Aufbau

`<aufgabendatei>/<NN>-<seite>/`:

- `index.json`: URL, Titel, Anfangsaufnahme, und je `do:`-Schritt die
  Eingabe, die Aufnahme vorher und nachher sowie die Antwort von Relief zum
  Aufnahmezeitpunkt.
- `snapshot-NN.json`: ein `a11y_perception::AXSnapshot` (AXTree, Fokus, URL,
  Titel). Eine unveränderte Aufnahme wird nicht doppelt gespeichert; mehrere
  Schritte verweisen dann auf dieselbe Datei.

Lokale Pfade sind als `file:///REPO/…` gespeichert.

## Grenzen

- Momentaufnahmen. Sie sagen nichts darüber, wie die Seiten heute aussehen.
- Die Antworten in `index.json` stammen aus dem damaligen Code und sind keine
  Soll-Werte. Tests legen ihre Erwartungen selbst fest.
- `11-korpus` enthält nur eigene Websites; Aufnahmen fremder, nicht offen
  lizenzierter Seiten liegen nicht im Repo.
- `10-real` stammt von Seiten mit offener Lizenz: gov.uk (Open Government
  Licence), Wikipedia (CC BY-SA), W3C-APG-Beispiele (W3C-Lizenz).
- Unkomprimiertes JSON (~13 MB im Arbeitsverzeichnis). Git
  speichert Objekte ohnehin komprimiert, und so bleiben Änderungen diffbar.
