# 11 · Nutzerstudie Phase 0c

Protokoll für den Nutzennachweis (Backlog 90, zurückgestellt). Formativ: Es soll eine
falsche Produktannahme früh zeigen und den ersten MVP festlegen, keinen
statistischen Wirksamkeitsnachweis liefern.

## Zielgruppe [Entscheidung 2026-09-24]

**Menschen, die das Web ohne präzise Zeigerbedienung nutzen** — motorische
Einschränkungen der Hände/Arme, Tremor, Lähmungen, Ermüdung; Bedienung über
Tastatur, alternative Tastaturen, ggf. Sprachsteuerung. Keine Diagnose als
Einschlusskriterium, sondern die Arbeitsweise: „Ich nutze die Maus nicht oder
nur eingeschränkt.“

Warum diese Gruppe zuerst:

- Sie trifft das, was der Spike schon kann: benannte Aktionen statt Dutzender
  Tab-Stopps, Rückfrage bei Mehrdeutigkeit, Bestätigung riskanter Aktionen,
  Dialoge schließen, iframes erreichen.
- Der Nutzen ist beobachtbar (Tastendrücke, Abbrüche), ohne dass zuerst Sprache
  oder KI gebaut sein müssen.

Ehrliche Gegenposition: Die Baseline ist stark. macOS-/Windows-Sprachsteuerung
kann schon „Klicke Warenkorb“, Tastaturnutzende haben eigene Strategien
(Suchen per Strg+F, Sprunglinks). Relief muss sich dagegen beweisen, nicht
gegen „gar nichts“.

## Hypothese (Job-to-be-done)

> Wenn ich ohne Maus ein Formular, einen Dialog oder ein Menü auf einer
> fremden Seite erledigen muss, will ich direkt sagen können, was ich tun
> will, statt mich durch die Seite zu tabben — damit ich schneller und mit
> weniger Kraft ans Ziel komme und nichts versehentlich auslöse.

Seitenklasse: Formulare, Buchungs-/Suchmasken, Consent-Dialoge, aufklappbare
Menüs auf normalen Websites.

Widerlegt, wenn: Teilnehmende mit ihrer Baseline gleich schnell oder
schneller sind **und** Relief ihnen keine Aufgabe ermöglicht, die sie sonst
abbrechen; oder Relief kritische Fehlaktionen verursacht.

## Ablauf einer Sitzung (90 min)

| Block | Inhalt | min |
|---|---|---|
| 1 | Einwilligung, Ablauf, Vergütung unabhängig vom Ergebnis | 10 |
| 2 | Gespräch: Hilfsmittel, typische Web-Aufgaben, wo es hakt | 15 |
| 3 | Aufgaben A mit eigener Werkzeugkette (Baseline) | 20 |
| 4 | Kurze Einführung Relief (5 min), Aufgaben B mit Relief | 25 |
| 5 | Nachgespräch: Vertrauen, Kontrolle, was fehlt | 15 |
| – | Pausen nach Bedarf, jederzeit Abbruch ohne Nachteil | 5 |

**Wechselnde Reihenfolge:** Aufgaben in zwei gleich schweren Sätzen (A/B);
die Hälfte der Teilnehmenden beginnt mit Relief. Jede Aufgabe einmal mit
Baseline und einmal mit Relief, aber nie dieselbe Aufgabe zweimal
hintereinander auf derselben Seite.

**Setting:** vor Ort mit eigener Hardware der Teilnehmenden, wenn möglich
(Spezialtastaturen, Einstellungen). Relief läuft auf einem mitgebrachten
Rechner oder als Build auf dem Gerät der Teilnehmenden — der Host ist bisher
nur auf macOS erprobt (Linux/Windows offen, Backlog 10, 31). Remote nur, wenn die
Person Relief selbst starten kann; Fernsteuerung eines fremden Rechners
verfälscht die Baseline.

## Aufgaben (Entwurf, vor der Studie gegenprüfen)

Nur echte Seiten ohne Kauf, ohne Absenden realer Formulare an Dritte.
Formularaufgaben auf eigenen Testseiten, die Echtheit nur nachbilden.

| Nr | Aufgabe | Seite | prüft |
|---|---|---|---|
| 1 | Cookie-Hinweis so beantworten, dass nur notwendige Cookies erlaubt sind | Nachrichtenseite mit Consent im iframe (z. B. bild.de) | iframe, Dialog, oft zweite Ebene |
| 2 | Eine Zugverbindung Rostock → Berlin für morgen 9 Uhr suchen (nicht buchen) | bahn.de | Formular mit Autovervollständigung |
| 3 | Kontaktformular vollständig ausfüllen, absenden, Fehlermeldung beheben | eigene Testseite (`spike/fixtures/form.html`, erweitert) | Formular, Fehler, Bestätigung |
| 4 | Über das Hauptmenü eine Unterseite öffnen | dwd.de oder APG-Menü | Menü öffnen/schließen |
| 5 | Ein Regal suchen und das erste Ergebnis öffnen | ikea.com | Suche, Ergebnisliste |

Vorher je Aufgabe festhalten: Erfolgskriterium, als kritisch geltende
Fehlaktion (z. B. „alle akzeptieren“ bei Aufgabe 1), Abbruchregel (10 min).

## Messbogen je Aufgabe

| Größe | Erhebung |
|---|---|
| Erledigt | ja / teilweise / nein |
| Kritische Fehlaktion | ja/nein, welche |
| Hilfen durch Moderation | Anzahl, Art |
| Tastendrücke | Relief: vom Host protokolliert; Baseline: aus Aufzeichnung, sonst geschätzt — Nebenmetrik |
| Zeit | Nebenmetrik |
| Schwierigkeit | SEQ, 1–7 („Wie leicht war die Aufgabe?“) |
| Vertrauen / Kontrolle | je 1–5 („Ich wusste, was passiert“, „Ich hatte die Kontrolle“) |
| Beobachtungen | wörtlich, ohne Deutung |

Relief protokolliert im Prototyp Eingaben, Antworten, Rückfragen und
Bestätigungen mit Zeitstempel, lokal, ohne Seiteninhalte; der Wert eines
Ausfüll- oder Auswahlbefehls steht verdeckt darin (→ 07).

## Auswertung

Je Aufgabe Baseline gegen Relief pro Person, nicht gemittelt über alle.
Ein Befund zählt, wenn er bei mindestens zwei Personen auftritt. Ergebnis:
bestätigte/widerlegte Hypothese, beobachtete Risiken, geänderte Anforderungen,
Empfehlung für Auslieferungsform (→ 09, Go/No-Go).

## Einwilligung (Entwurf, vor Verwendung rechtlich prüfen lassen)

Muss mindestens enthalten:

- Zweck: Erprobung eines Forschungsprototyps, keine Prüfung der Person.
- Welche Daten: Angaben zu Hilfsmitteln und zur Bedienweise (Gesundheitsdaten
  nach Art. 9 DSGVO), Messwerte, Notizen; Bild-/Tonaufzeichnung nur nach
  gesondertem Ja.
- Freiwilligkeit, Abbruch jederzeit ohne Nachteil, Vergütung auch bei Abbruch.
- Pseudonymisierung (Kennung statt Name), Speicherort, Löschfrist, Widerruf.
- Verantwortlicher und Kontakt.
- Keine Weitergabe an Dritte; Ergebnisse nur zusammengefasst und ohne
  Rückschluss auf Personen.

Einwilligung in der für die Person passenden Form (digital ausfüllbar,
barrierefreies PDF oder mündlich mit Protokoll).

## Rekrutierung (Stand: noch keine Kontakte)

Wege [Annahme, Kontakte nicht geprüft]:

- Selbsthilfe- und Fachverbände: z. B. Bundesverband Selbsthilfe
  Körperbehinderter (BSK), Deutsche Gesellschaft für Muskelkranke (DGM),
  Deutsche Multiple Sklerose Gesellschaft (DMSG), Fördergemeinschaft der
  Querschnittgelähmten (FGQ).
- Ergänzende unabhängige Teilhabeberatung (EUTB) in der Region.
- Hochschulen mit Forschung zu Barrierefreiheit/Assistenztechnik, die
  Testpersonen-Pools haben.
- Eigenes Netzwerk (Kunden, barrierlab-Umfeld) — Achtung auf Gefälligkeits-
  antworten; nicht die ersten Teilnehmenden aus dem engsten Kreis.

Anschreiben: kurz, konkret (Dauer, Ort, Vergütung, was getestet wird, dass
nicht die Person getestet wird), in barrierefreier Form, mit Kontakt für
Rückfragen. Vergütung vorab festlegen und unabhängig vom Ergebnis zahlen
[Höhe offen].

## Voraussetzung Prototyp

Befehlsleiste im Browser, per Tastenkürzel, selbst tastaturbedienbar und
mit Screenreader nutzbar; Bestätigungen per „ja“; Protokoll der Eingaben.
Stand → `docs/architecture.md`.
