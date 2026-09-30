# 28 · Resolver für fehlende Namen: Messlauf und Schwellen

**Umgebung:** lokal oder Cloud mit API-Key (`ANTHROPIC_API_KEY`) · **Phase:** 4 · **Abhängig von:** 27 ✓, 12 ✓; Rest wartet auf einen API-Key

## Ziel

Die Confidence-Schwellen für Modellnamen **empirisch** festlegen. Gebaut ist
alles bis auf die Messung: Ausschnitt, Anthropic-Adapter, Stichprobe,
Kalibrierwerkzeug (→ `spezifikation/06`, „Resolver fehlender Namen“).

## Schritte

1. Messlauf mit Key, je Modell:

   ```bash
   export ANTHROPIC_API_KEY=…            # eigener Key, nie ins Repo
   cargo run -p relief-resolver --features anthropic -- kalibrieren \
     --aufzeichnen spike/kalibrierung/antworten-haiku-4-5.json
   RELIEF_ANTHROPIC_MODEL=claude-sonnet-5 \
   cargo run -p relief-resolver --features anthropic -- kalibrieren \
     --aufzeichnen spike/kalibrierung/antworten-sonnet-5.json
   ```

   Erneut auswerten ohne Kosten: `cargo run -p relief-resolver --
   kalibrieren --wiedergeben <datei>`.
2. Prüfen, ob die API das Schema ohne die entfernten Grenzen annimmt
   (Annahme in `anthropic::UNSUPPORTED_KEYWORDS`); sonst anpassen.
3. Zielwert für die Trefferquote über der Schwelle festlegen (Werkzeug:
   ≥ 90 % bei ≥ 10 Hypothesen, Annahme). Die Stichprobe hat 8 Einträge und
   reicht dafür nicht (`MIN_SUPPORT` = 10): vorher aus eigenen Testseiten
   oder offen lizenzierten Seiten auf mindestens 20 erweitern.
4. Schwelle je Modell in `CALIBRATED_THRESHOLDS`
   (`crates/relief-ai-contract/src/hypothesis.rs`) eintragen, Messung
   (Trefferquote je Band, Tokens und Kosten je Anfrage und Seite) in
   `spezifikation/06`.

## Fertig, wenn

- Schwellen mit Messung in `spezifikation/06` und `CALIBRATED_THRESHOLDS`;
  `none` bleibt Standard.

## Nicht Teil

- Aufruf des Resolvers aus der Runtime (Lazy, Cache, Rückfall auf `none`
  mit Ansage) und die Frage aus `spezifikation/03`, ob eine Hypothese beim
  Ändern des Knotens verworfen wird.
- Icon-Font-Namen aus Private-Use-Zeichen (für Chromium vorhanden, für
  Menschen bedeutungslos): eigene Frage, ob sie als „fehlt“ gelten.
