// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_SPEECH_SPEECH_RECOGNITION_H_
#define RELIEF_SPEECH_SPEECH_RECOGNITION_H_

#include <optional>
#include <string>

#include "base/files/file_path.h"
#include "base/functional/callback.h"

namespace relief::speech {

// Ergebnis: erkannter Text (oder nichts) und wie erkannt bzw. warum nicht.
using RecognizedCallback =
    base::OnceCallback<void(std::optional<std::string> text,
                            std::string note)>;

// Spracherkennung, Deutsch (Pakete 26, 114; macOS: SFSpeechRecognizer als
// Datenstrom, auf dem Gerät, wenn verfügbar). Ohne `file` vom Mikrofon,
// sonst aus der Audiodatei über denselben Weg (für Tests). Die Äußerung
// endet nach kurzer Stille, mit StopListening oder nach 15 s. Fragt beim
// ersten Mal nach den Freigaben. `done` läuft auf der aufrufenden Sequenz.
void Listen(std::optional<base::FilePath> file, RecognizedCallback done);

// Äußerung beenden und das bisher Erkannte liefern; false, wenn keine lief.
bool StopListening();

// Hört gerade zu.
bool IsListening();

// Laufende Erkennung verwerfen (ohne Ergebnis); true, wenn eine lief.
bool CancelRecognition();

// Ein Screenreader (VoiceOver) ist an: Relief spricht dann nicht selbst.
bool ScreenReaderActive();

}  // namespace relief::speech

#endif  // RELIEF_SPEECH_SPEECH_RECOGNITION_H_
