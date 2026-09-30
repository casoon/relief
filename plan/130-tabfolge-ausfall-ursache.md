# 130 · CDP-Host: `tabfolge` in einen fremden Frame, Ursache des seltenen Ausfalls

**Umgebung:** Cloud · **Phase:** Linie B · **Abhängig von:** 106 ✓

## Ziel

Tritt der Ausfall von `tabfolge Name, Telefon (Rückfrage)` in
`form-fremd.html` wieder auf, seine Ursache mit Protokoll belegen und
beheben.

## Kontext

- Stand vor Paket 105: 1 Ausfall in 10 960 Tab-Folgen unter Last; mit 105
  und 106: 0 in 15 200 Folgen und 100 Läufen von 06 unter Last
  (→ [spezifikation/12](spezifikation/12-produktumfang.md)) [belegt].
- Die Vermutung aus Paket 85 (Frame bekommt den Fokus zu spät) ist
  widerlegt; beobachtet ist, dass die Seite am Ende einer Folge den Fokus
  ganz abgibt. Ob das auch beim Tab in den Frame vorkommt, ist offen
  [Annahme].

## Schritte

1. Bei einem erneuten Ausfall (Prüfbefehle, CI) die Ausgabe sichern.
2. Die Messung aus Paket 106 wiederholen: in `tab_walk` nach jedem Tab
   `activeElement` und `document.hasFocus()` der Seite und jedes
   angehängten Frames protokollieren (vorübergehend, hinter einer
   Umgebungsvariablen), reine Tab-Folgen über `form-fremd.html` mit 4 bis
   6 Läufen gleichzeitig, bis der Ausfall auftritt.
3. Ursache belegen, beheben.

## Fertig, wenn

- Ursache mit Protokoll belegt und behoben, oder bis zum nächsten Paket an
  den Frames des CDP-Hosts kein Ausfall mehr aufgetreten (dann schließen).

## Nicht Teil

Fork-Host.
