# 116 · Sprache: Cloud-STT per eigenem Key

**Umgebung:** Cloud schreibt, M4 baut und prüft · **Phase:** 3 · **Abhängig von:** 114 ✓

## Ziel

Spracherkennung über einen Cloud-Dienst mit eigenem Key als Option neben
Apples Erkennung auf dem Gerät (→ spezifikation/08, „Speech Assist“), etwa
für Linux oder bessere Erkennung.

## Offen vor dem Start

- Anbieter wählen (Entscheidung Nutzer); Kosten und Datenschutz.

## Schritte

1. Adapter hinter derselben Schnittstelle (`speech::Listen`), Audio nur
   über die Privacy-Grenze (→ spezifikation/07, `Budget`/`Permit`).
2. Ausdrücklich einschalten (Profil oder Einstellung), nie Standard.

## Fertig, wenn

- `17-sprache.txt` läuft mit dem Cloud-Adapter; ohne Key bleibt alles
  lokal.
