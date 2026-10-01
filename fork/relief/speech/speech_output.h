// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_SPEECH_SPEECH_OUTPUT_H_
#define RELIEF_SPEECH_SPEECH_OUTPUT_H_

#include <string>

namespace relief::speech {

// Sprachausgabe über Chromiums TTS (Paket 26; macOS-Stimmen, keine
// Google-Dienste). Stumm, solange ein Screenreader läuft: zwei Stimmen
// sind unbenutzbar (→ spezifikation/08).
class SpeechOutput {
 public:
  SpeechOutput();
  SpeechOutput(const SpeechOutput&) = delete;
  SpeechOutput& operator=(const SpeechOutput&) = delete;
  ~SpeechOutput();

  // Spricht `text` (vorherige Ausgabe verworfen); false, wenn stumm.
  bool Speak(const std::string& text);
  // Sofort verstummen; true, wenn gerade gesprochen wurde.
  bool Stop();

 private:
  double volume_ = 1.0;
};

}  // namespace relief::speech

#endif  // RELIEF_SPEECH_SPEECH_OUTPUT_H_
