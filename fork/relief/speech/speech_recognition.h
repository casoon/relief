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

// Spracherkennung einer Audiodatei, Deutsch (Paket 26; macOS:
// SFSpeechRecognizer, auf dem Gerät, wenn verfügbar). Fragt beim ersten
// Mal nach der Freigabe. `done` läuft auf der aufrufenden Sequenz.
void RecognizeFile(const base::FilePath& path, RecognizedCallback done);

// Laufende Erkennung verwerfen; true, wenn eine lief.
bool CancelRecognition();

// Ein Screenreader (VoiceOver) ist an: Relief spricht dann nicht selbst.
bool ScreenReaderActive();

}  // namespace relief::speech

#endif  // RELIEF_SPEECH_SPEECH_RECOGNITION_H_
