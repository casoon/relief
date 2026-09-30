---
name: plan-verzeichnis
description: Konvention für das lokale, gitignorete plan/-Verzeichnis eines Projekts (Task-/Ideen-Tracking, siehe globale CLAUDE.md) — wann eine flache Struktur reicht und wann sie in Referenzrahmen (plan/spezifikation/) plus Backlog (plan/-Wurzel, nummerierte Folgepläne, status.md) aufgeteilt wird. Nutzen beim Aufsetzen eines neuen plan/-Verzeichnisses, beim Umstrukturieren eines gewachsenen, oder wenn plan/ so voll ist, dass unklar wird, was noch offen ist vs. was nur Nachschlagewerk ist. NICHT für die committete Projekt-Doku (docs/, README, ADRs) — die ist versioniert und für andere Leser gedacht, plan/ ist lokal und für den Agenten/Nutzer selbst.
---

# Plan-Verzeichnis

`plan/` ist laut globaler CLAUDE.md der Ort für Task-/Ideen-Tracking — gitignored,
lokal, kein Teil der committeten Doku. Die Grundregel von dort gilt immer: einzelne
Dateien statt einer Liste, nummeriert, mit `status.md` daneben, fertige Punkte
promptly entfernen. Dieser Skill ergänzt das um eine Entscheidung, die erst mit
wachsendem Projekt relevant wird: **wann `plan/` selbst eine interne Struktur
braucht.**

## Die Kern-Entscheidung: ein Ordner reicht, oder zwei?

Ein `plan/` mit ein paar Dateien (Überblick, ein, zwei offene Punkte) bleibt flach —
genau die globale CLAUDE.md-Regel, keine Zusatzstruktur nötig. Das Problem entsteht
erst, wenn zwei fachlich verschiedene Dinge im selben flachen Ordner landen:

| Sorte | Beispiel | Lebensdauer |
|---|---|---|
| **Referenzrahmen** | „Wie funktioniert das Kapazitätsmodell", Datenmodell, Scope-Tabelle, Design-Anforderungen | dauerhaft — wird nie „fertig" und gelöscht, auch wenn längst umgesetzt |
| **Backlog** | „Dieser eine offene Punkt muss noch gebaut werden" | zeitlich begrenzt — Zeile in `status.md` weg, sobald umgesetzt und validiert |

Solange beides zusammen in einem flachen Verzeichnis wächst, lässt sich auf einen
Blick nicht mehr sagen, was tatsächlich noch offen ist und was nur Hintergrundwissen
ist — das ist das eigentliche Signal zum Aufteilen, nicht eine feste Dateizahl.

**Wenn das Signal auftritt**, Referenzrahmen in einen Unterordner verschieben (Name
frei, z. B. `plan/spezifikation/` oder `plan/referenz/` — im Projekt konsistent
bleiben), Backlog bleibt flach in der `plan/`-Wurzel:

```
plan/
├── README.md              # erklärt den Split zuerst, bevor irgendwas anderes
├── status.md              # Backlog-Tabelle: Nr, Thema, Status, Abhängig von, Datei
├── 18-irgendein-punkt.md  # offener Folgeplan, nummeriert
├── 19-naechster-punkt.md
└── spezifikation/
    ├── 00-ueberblick.md
    ├── 01-irgendein-fachkapitel.md
    └── …                  # nummeriert, verlinken sich untereinander flach
```

Die `plan/`-Wurzel bleibt dadurch klein genug, um sie mit einem Blick zu
überschauen — wenn dort etwas liegt, das kein offener Backlog-Punkt und keine der
beiden Haushaltsdateien (`README.md`, `status.md`) ist, gehört es nach
`spezifikation/`.

## Lebenszyklus eines Backlog-Eintrags

1. **Anlegen**: nummerierte Datei in der `plan/`-Wurzel, Zeile in `status.md`.
   Größere Punkte folgen dem Ziel/Ansatz/Schritte/Risiken/Offene-Fragen/Nicht-Teil-
   dieses-Plans-Muster; kleine Punkte (eine Randfall-Notiz, eine UI-Inkonsistenz)
   dürfen kürzer sein — die Vollform ist kein Selbstzweck.
2. **Umsetzen und validieren** (Code, Tests, was auch immer der Punkt verlangt).
3. **Aufräumen — der Schritt, der am leichtesten vergessen wird**: Zeile aus
   `status.md` entfernen, dann die Datei entweder ganz löschen (rein
   organisatorisch, keine bleibende fachliche Substanz) oder ihre fachliche
   Substanz in die passende `spezifikation/`-Datei falten, mit einer kurzen
   „Status: umgesetzt, siehe X" statt eines langen Nacherzähl-Absatzes — und
   danach die Backlog-Datei trotzdem löschen. Checkbar erledigt ist dieser Schritt
   erst, wenn kein anderer Datei mehr auf die gelöschte Datei als Link verweist
   (`grep -rl "dateiname.md" plan/` sollte leer sein, `../adr/…`-Pfade mitgeprüft).

## Gotchas

- **Ein reiner Verweis ist kein Grund, eine Datei am Leben zu halten.** Eine
  Backlog-Datei, die nach dem Falten nur noch „siehe X" / „siehe Y" enthält, hat
  ihren Zweck erfüllt — sie gehört gelöscht, nicht als Pointer-Stub liegen
  gelassen. Genau das ist beim ersten Anlauf in Avilo passiert: eine Datei wurde
  auf reine Verweise gekürzt statt gelöscht, und blieb dann noch eine ganze
  Session lang als scheinbar aktiver Eintrag stehen.
- **Nicht vorzeitig aufteilen.** Ein neues Projekt mit drei, vier Dateien braucht
  keinen Unterordner — das ist Overhead ohne Nutzen. Erst aufteilen, wenn beim
  Draufschauen tatsächlich Verwechslungsgefahr zwischen „noch zu tun" und
  „Hintergrundwissen" entsteht.
- **Relative Links brechen beim Verschieben in den Unterordner** — Backlog-Dateien
  verweisen danach mit `spezifikation/NN-datei.md` auf Referenzrahmen-Dateien,
  Referenzrahmen-Dateien untereinander bleiben flach (`NN-datei.md`, sie sind
  Geschwister im selben Unterordner), und alles, was aus `spezifikation/` nach
  draußen zeigt (z. B. `../adr/…`), braucht ein `../` mehr als vorher. Nach jedem
  Verschieben alle `](…\.md)`-Links im ganzen `plan/`-Baum gegenprüfen, nicht nur
  die offensichtlich betroffenen Dateien — mehrere Dateien verweisen typischerweise
  auf dieselbe verschobene Datei.
- **`plan/README.md` erklärt den Split zuerst**, bevor irgendein Fachinhalt kommt —
  wer den Ordner öffnet, muss in den ersten Zeilen verstehen, wo Referenzrahmen und
  wo Backlog ist, nicht erst nach dem Scrollen durch die Leitidee des Projekts.

## Verwandt

- [[agent-config-authoring]] — dieselbe Grundfrage („was gehört wohin") eine Ebene
  höher: CLAUDE.md vs. Skill vs. Memory vs. knowledge-base statt Referenzrahmen vs.
  Backlog innerhalb von `plan/`.
- [[skill-authoring]] — Konventionen für Skills selbst, falls aus einem
  wiederkehrenden `plan/`-Muster mehr als dieser eine Skill werden soll.
