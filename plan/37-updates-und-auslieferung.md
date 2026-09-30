# 37 · Updates und Auslieferung

**Umgebung:** M4 + Entscheidung · **Phase:** Produkt · **Abhängig von:** 36

## Ziel

Relief lässt sich an Dritte weitergeben und bleibt sicher aktuell (→
[spezifikation/12](spezifikation/12-produktumfang.md#grundausstattung-eines-browsers-annahme)).

## Zu entscheiden

- Apple-Developer-Konto für Signatur und Notarisierung (Kosten, Inhaber).
- Update-Mechanismus (z. B. Sparkle oder eigener Dienst) und Hosting.
- Update-Infrastruktur und verantwortliche Person für dringende
  Sicherheitsveröffentlichungen.

## Zwei Betriebsarten [Entscheidung]

- **Forschungsbuild:** darf im dokumentierten Vier-Wochen-Takt bleiben, zeigt
  Basisversion und Alter sichtbar an und wird bei ausstehendem
  Sicherheitsupdate nicht für tägliches Browsen oder unbekannte Live-Seiten
  freigegeben.
- **Verteilbarer Build:** wird erst angeboten, wenn automatische, signierte
  Updates, Rollback/Recovery und eine verbindliche Sicherheitsfrist stehen.
  Chromium veröffentlicht reguläre Stable-Aktualisierungen häufig und bei
  kritischen Lücken zusätzlich ungeplant; daher ist „ein Rebase pro Monat“
  keine ausreichende Produktregel.

## Schritte

1. Signieren und Notarisieren als Skript neben `scripts/chromium-setup.sh`.
2. Selbstaktualisierung einbauen und gegen eine Testversion prüfen.
3. Release-Feed beobachten; neue Stable- und außerplanmäßige
   Sicherheitsupdates bewerten, bauen und in einem kleinen Vorab-Kanal testen.
4. Frische-Gate: Ein verteilbarer Build zeigt Updatefehler und blockiert die
   Freigabe, wenn die festgelegte Sicherheitsfrist überschritten ist.
5. Rollback beziehungsweise Recovery nach einem fehlerhaften Update prüfen;
   Signatur und Herkunft jedes Artefakts verifizieren.
6. Aufwand je Chromium-Version messen (Rebase + Build + Auslieferung) und
   gegen das No-Go-Kriterium „mehr als ~1 Tag pro Monat“ (09) halten.

## Fertig, wenn

- Eine signierte, notarisierte `Relief.app` startet auf einem fremden Mac
  ohne Gatekeeper-Warnung und aktualisiert sich auf eine neuere Testversion.
- Manipulierte oder falsch signierte Updates werden abgelehnt; ein
  fehlgeschlagenes Update lässt sich ohne Profilverlust wiederherstellen.
- Forschungs- und verteilbarer Build sind sichtbar unterschieden; ein
  veralteter Build kann nicht versehentlich als sicher aktuelles Produkt
  veröffentlicht werden.

## Nicht Teil

Windows und Linux (→ 31).
