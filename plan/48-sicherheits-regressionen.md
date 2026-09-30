# 48 · Sicherheits-Regressionsmatrix für Modellgrenzen

**Umgebung:** Cloud · **Phase:** quer · **Abhängig von:** 24; vor Modellintegration

## Ziel

Reliefs Trennung zwischen untrusted Seiten-/Modellausgabe, Rust-Validierung,
Bestätigung und Browseraktion bleibt auch unter gezielten Missbrauchsfällen
belegt (→ spezifikation/07).

## Schritte

1. Abuse-Case-Matrix als browserfreie Fixtures: Prompt-Override,
   Datenabfluss, Werkzeug-/Rechteausweitung, Bestätigungs-Bypass und
   Wiederverwendung, Cache-Vergiftung sowie Schleifen/Kostenüberschreitung.
2. Bestätigungstoken an Aktion, Ziel, Graph-Version, Wert und Zieladresse
   binden; kurze Gültigkeit und einmalige Verwendung testen.
3. Feste Grenzen pro Seite/Aufgabe für Baumgröße, Modellaufrufe,
   Wiederholungen, Zeit und Kosten definieren; verständlichen Abbruch prüfen.
4. Strukturierte Security-Logs auf Entscheidung, Risikoklasse, Plan-ID und
   Abbruchgrund prüfen; sensible Feldwerte dürfen nicht erscheinen.

## Fertig, wenn

- Jeder Abuse Case ohne Browser und mindestens der Bestätigungs-Bypass im Fork
  reproduzierbar getestet ist.
- Keine Modell- oder Cache-Ausgabe kann Risiko senken, eine Freigabe erweitern
  oder selbst eine Browseraktion auslösen.
- Grenzüberschreitungen beenden die Aufgabe ohne Endlosschleife und ohne
  unkontrollierte weitere Modellkosten.
