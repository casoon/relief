# 114 · Sprache: Live-Mikrofon, Cloud-STT, „dieses Feld“

**Umgebung:** Cloud schreibt, M4 baut und prüft · **Phase:** 3 · **Abhängig von:** 26 ✓

## Ziel

Die Sprachschicht aus Paket 26 (→ spezifikation/08, „Umsetzung (Paket
26)“) live bedienbar machen.

## Schritte

1. Sprechtaste im Relief-Panel (und Kürzel): Mikrofon über
   `SFSpeechAudioBufferRecognitionRequest`, Freigabe Mikrofon; Ende der
   Äußerung erkennen; „abbrechen“ während der Erkennung.
2. Cloud-STT per eigenem Key als Option hinter derselben Schnittstelle
   (Privacy-Grenze → spezifikation/07).
3. „dieses Feld“, „hier“ als Bezug auf Fokus bzw. Position in Befehlen.
4. Koexistenz mit VoiceOver manuell prüfen (mit 47).

## Fertig, wenn

- Die Dialogbeispiele aus spezifikation/08 funktionieren live über das
  Mikrofon; Ausgabe und offene Aktion lassen sich per Sprache abbrechen.
