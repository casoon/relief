# 41 · Fähigkeitsprofile

**Umgebung:** Cloud + M4 · **Phase:** Linie A · **Abhängig von:** 25, 26, 29

## Ziel

Oberfläche und Ausgabe nach Fähigkeiten statt Diagnosen umschalten (→
[spezifikation/08](spezifikation/08-assistenz-und-capabilities.md)). Ein Profil
besteht aus frei kombinierbaren Angaben etwa zu visuellem Detail,
Kontrastbedarf, Ein- und Ausgabekanälen, Bewegungstoleranz und Textkomplexität;
es gibt keine Diagnosemodi.

## Schritte

1. Profil als Datensatz im Kern (Ausgabekanäle, Detailgrad, Bestätigungen,
   Darstellung), browserfrei.
2. Einzelne Fähigkeiten im Fork ändern; neutrale Start-Presets dürfen mehrere
   Werte vorbelegen, bleiben aber vollständig editierbar.
3. Profil lokal und optional pro Website merken; nie an Seite oder Dienst
   übertragen.
4. Jede Änderung ist sofort nachvollziehbar, einzeln rücksetzbar und über
   „Standard wiederherstellen“ vollständig reversibel. Globale und
   websitespezifische Werte sind sichtbar getrennt; Betriebssystemeinstellungen
   dienen höchstens als vorgeschlagener Startwert, nie als Diagnose.

## Fertig, wenn

- Dieselbe Aufgabe im Testshop läuft mit mindestens vier repräsentativen
  Kombinationen von Fähigkeitswerten; Unterschiede in Ansage, Eingabe und
  Darstellung sind dokumentiert, ohne die Kombinationen als Diagnosen zu
  bezeichnen.
- Keine Kombination entfernt Inhalt, Bedeutung oder notwendige Bedienwege;
  Zurücksetzen stellt den Ausgangszustand reproduzierbar wieder her.

## Hinweis

Welche Fähigkeitskombinationen zuerst zählen, entscheidet der Nutzennachweis
(→ 90).
