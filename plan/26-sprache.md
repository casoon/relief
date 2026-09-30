# 26 · Sprachschicht

**Umgebung:** Cloud (Traits) + M4 (Integration) · **Phase:** 3 · **Abhängig von:** 25

## Ziel

STT → Intent → Runtime → Antwort → TTS nach
[spezifikation/08](spezifikation/08-assistenz-und-capabilities.md#speech-assist),
austauschbar, ohne Google-Dienste.

## Schritte

1. Rust-Traits `SpeechInput`/`SpeechOutput` (browserfrei), Testdoubles.
2. TTS über Chromium-TTS; STT macOS (`SFSpeechRecognizer`), Cloud-STT per
   eigenem Key als Option.
3. „Abbrechen“ als vorrangiger lokaler Befehl: TTS sofort stoppen, laufende
   Erkennung verwerfen und offene Auswahl/Bestätigung aus 25 schließen.
4. Kurzer Dialogkontext nur aus explizitem Fokus und letzter Relief-Auswahl,
   damit „dieses Feld“/„nimm das zweite“ ohne Modell auflösbar bleibt.
5. Koexistenz mit VoiceOver.

## Fertig, wenn

- Die Dialogbeispiele aus 08 („Was ist auf dieser Seite?“, „Welche Größen …“,
  „Nimm 43“) funktionieren per Sprache auf der Testseite; Ausgabe und offene
  Aktion lassen sich per Sprache abbrechen.
