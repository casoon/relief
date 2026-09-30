# 106 · CDP-Host: `tabfolge` in einen fremden Frame sporadisch verfehlt

**Umgebung:** Cloud · **Phase:** Linie B · **Abhängig von:** 85 ✓

## Ziel

Die Ursache des seltenen Ausfalls von `tabfolge` in `form-fremd.html`
finden und beheben, oder belegen, dass er nicht mehr auftritt.

## Kontext

- Der in Paket 85 genannte sporadische Ausfall (154/155 über 01–09) ist
  einmal wieder aufgetreten und gesichert (2026-09-30, Stand vor Paket 85,
  drei Läufe von 01–09 gleichzeitig), in `spike/tasks/06-form-assertions.txt`
  bei `url: server:../fixtures/form-fremd.html` [belegt]:

  ```text
  ? tabfolge Name, Telefon (Rückfrage)
    1 Befund:
    - fail form/tab-order [high]: „Telefon (Rückfrage)“ wird per Tab nicht erreicht.
    ✗ erwartet „Keine Befunde“ — FEHLT
  ```

  Die übrigen Zeilen des Abschnitts (Namensvergleich, `fülle Telefon`,
  „wo bin ich“) waren im selben Lauf erfüllt.
- Läufe ohne Ausfall: vor Paket 85 15 Läufe 01–09 einzeln und 9 weitere
  unter Last (1 Ausfall in 10 Läufen unter Last); mit Paket 85 10 Läufe
  01–09 und 15 einzeln, 5 unter Last, dazu 64 Durchgänge von 06 unter Last
  (je 2 bis 3 Läufe gleichzeitig) [belegt].
- Nicht die Ruhe-Erkennung: `tabfolge` wartet nicht auf Ruhe, sondern
  fragt nach jedem Tab den Fokus ab (`assertions.rs`, `tab_walk`) [belegt
  im Code].
- Vermutung: Springt der Fokus in den Frame eines anderen Prozesses, meldet
  die Seite sofort das `iframe`, der Frame bekommt den Fokus erst danach
  (IPC); fragt `Frames::active_element` dazwischen, antwortet der Frame mit
  `body`, und das Feld fehlt in der Folge [Annahme]. Ein Nachfragen bei
  `body` im Frame (bis 25 × 20 ms) wurde in 24 Durchgängen unter Last nie
  gebraucht, deshalb nicht übernommen.

## Schritte

1. `tab_walk` mit Protokoll je Schritt (Fokus als Frame-Nummer und
   Backend-ID, `body` der Seite) unter Last laufen lassen, bis der Ausfall
   auftritt.
2. Ursache belegen, dann beheben (Fokus im Frame abwarten o. ä.).

## Fertig, wenn

- Ursache mit Protokoll belegt und behoben, oder 100 Durchgänge von 06
  unter Last ohne Ausfall.

## Nicht Teil

Fork-Host.
