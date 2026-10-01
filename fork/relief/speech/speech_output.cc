// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/speech/speech_output.h"

#include <algorithm>
#include <memory>

#include "base/command_line.h"
#include "base/strings/string_number_conversions.h"
#include "content/public/browser/tts_controller.h"
#include "content/public/browser/tts_utterance.h"
#include "relief/relief_switches.h"
#include "relief/speech/speech_recognition.h"

namespace relief::speech {

SpeechOutput::SpeechOutput() {
  const base::CommandLine& command_line =
      *base::CommandLine::ForCurrentProcess();
  double volume = 1.0;
  if (command_line.HasSwitch(switches::kReliefSpeechVolume) &&
      base::StringToDouble(
          command_line.GetSwitchValueASCII(switches::kReliefSpeechVolume),
          &volume)) {
    volume_ = std::clamp(volume, 0.0, 1.0);
  }
}

SpeechOutput::~SpeechOutput() = default;

bool SpeechOutput::Speak(const std::string& text) {
  if (text.empty() || ScreenReaderActive()) {
    return false;
  }
  std::unique_ptr<content::TtsUtterance> utterance =
      content::TtsUtterance::Create();
  utterance->SetText(text);
  utterance->SetLang("de-DE");
  utterance->SetContinuousParameters(/*rate=*/1.0, /*pitch=*/1.0, volume_);
  utterance->SetShouldClearQueue(true);
  content::TtsController::GetInstance()->SpeakOrEnqueue(std::move(utterance));
  return true;
}

bool SpeechOutput::Stop() {
  content::TtsController* tts = content::TtsController::GetInstance();
  const bool speaking = tts->IsSpeaking();
  tts->Stop();
  return speaking;
}

}  // namespace relief::speech
