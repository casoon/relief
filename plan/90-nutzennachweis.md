# 90 · Nutzennachweis (zurückgestellt)

## Ziel

Parallel zum CDP-Spike prüfen, ob Relief für eine klar benannte Nutzergruppe
eine konkrete Web-Aufgabe besser löst als deren heutige Werkzeugkette.

**Zurückgestellt (2026-09-24):** Das Projekt konzentriert sich auf die
technische Lösung. Der Nutzennachweis wird nachgeholt, sobald Kontakte zu
Teilnehmenden bestehen. Vorbereitet bleibt, was schon da ist.

## Stand 2026-09-24

- Zielgruppe entschieden: ohne präzise Zeigerbedienung (→ `docs/decisions.md`).
- Protokoll, Aufgaben, Messbogen, Einwilligungsinhalte, Rekrutierungswege:
  [spezifikation/11-nutzerstudie.md](spezifikation/11-nutzerstudie.md).
- Prototyp: Befehlsleiste im Browser (`relief-cdp palette <url>`), per
  Selbsttest mit echten Tastenereignissen geprüft.

Als Nächstes, in dieser Reihenfolge:

1. Einwilligungstext ausformulieren und rechtlich prüfen lassen; Vergütung festlegen.
2. Aufgaben 1–5 gegen die Live-Seiten prüfen, Testseite für Aufgabe 3 erweitern.
3. Pilotsitzung mit einer Person außerhalb der Zielgruppe (Ablauf, Zeiten, Protokoll).
4. Rekrutierung über die Wege in 11 starten (noch keine Kontakte).
5. Sitzungen, Auswertung, Ergebnis nach 09 (Go/No-Go).

## Schritte (ursprünglich)

1. 5–8 potenzielle Nutzerinnen und Nutzer aus einer zunächst gemeinsamen
   Bedarfsgruppe gewinnen und angemessen vergüten.
2. Aufgaben, heutige Hilfsmittel, Abbrüche, Fehlaktionen und Hilfebedarf
   erfassen; daraus einen primären Job-to-be-done und eine Seitenklasse wählen.
3. Aus dem CDP-Spike den kleinsten bedienbaren Prototyp für 3–5 dieser Aufgaben
   bereitstellen.
4. Baseline und Prototyp mit wechselnder Reihenfolge vergleichen.
5. Ergebnis dokumentieren: belegter Nutzen, beobachtete Risiken, geänderte
   Anforderungen und Empfehlung für Fork, CDP-Host oder Component Extension.

## Voraussetzungen

- **Der Prototyp muss für die gewählte Gruppe selbst bedienbar sein.** Erfüllt
  durch die Befehlsleiste im Browser (statt der Terminal-REPL). Offen: Läuft
  bisher nur auf macOS erprobt; für Sitzungen auf Geräten der Teilnehmenden
  braucht es Windows (→ 31).
- **Datenschutz:** Die Sitzungen erheben Angaben zu Einschränkungen und
  Hilfsmitteln, also Gesundheitsdaten (Art. 9 DSGVO). Ausdrückliche
  Einwilligung, Datensparsamkeit, keine Aufzeichnung ohne Zustimmung,
  Pseudonymisierung der Auswertung.

## Entscheidungskriterium

Go für den gewählten MVP, wenn mindestens ein wichtiger wiederkehrender
Anwendungsfall gegenüber der Baseline klar verbessert oder erstmals ermöglicht
wird und keine neue kritische Fehlaktion entsteht.

Das Ergebnis ist formativ. Es belegt eine tragfähige Richtung, aber noch keine
Wirksamkeit für alle Menschen mit ähnlichen Bedarfen.
