// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/speech/speech_recognition.h"

#import <AppKit/AppKit.h>
#import <Speech/Speech.h>

#include <memory>
#include <utility>

#include "base/functional/bind.h"
#include "base/no_destructor.h"
#include "base/strings/sys_string_conversions.h"
#include "base/synchronization/lock.h"
#include "base/task/bind_post_task.h"
#include "base/task/sequenced_task_runner.h"

namespace relief::speech {

namespace {

base::Lock& TaskLock() {
  static base::NoDestructor<base::Lock> lock;
  return *lock;
}

// Laufende Erkennung (unter TaskLock).
SFSpeechRecognitionTask* __strong& CurrentTask() {
  static SFSpeechRecognitionTask* __strong task = nil;
  return task;
}

// Ruft das Ergebnis genau einmal (die Handler von Speech.framework können
// mehrfach kommen).
class Once {
 public:
  explicit Once(RecognizedCallback done) : done_(std::move(done)) {}
  void Run(std::optional<std::string> text, std::string note) {
    base::AutoLock hold(lock_);
    if (done_) {
      std::move(done_).Run(std::move(text), std::move(note));
    }
  }

 private:
  base::Lock lock_;
  RecognizedCallback done_;
};

void Start(NSURL* url, std::shared_ptr<Once> once) {
  SFSpeechRecognizer* recognizer = [[SFSpeechRecognizer alloc]
      initWithLocale:[NSLocale localeWithLocaleIdentifier:@"de-DE"]];
  if (!recognizer || !recognizer.available) {
    once->Run(std::nullopt, "Spracherkennung für Deutsch nicht verfügbar");
    return;
  }
  SFSpeechURLRecognitionRequest* request =
      [[SFSpeechURLRecognitionRequest alloc] initWithURL:url];
  request.shouldReportPartialResults = NO;
  // Auf dem Gerät, wo möglich: Die Audiodaten verlassen den Rechner nicht.
  const bool on_device = recognizer.supportsOnDeviceRecognition;
  request.requiresOnDeviceRecognition = on_device;
  const std::string where = on_device ? "auf dem Gerät" : "Apple-Server";
  SFSpeechRecognitionTask* task = [recognizer
      recognitionTaskWithRequest:request
                   resultHandler:^(SFSpeechRecognitionResult* result,
                                   NSError* error) {
                     if (error) {
                       once->Run(std::nullopt,
                                 base::SysNSStringToUTF8(
                                     error.localizedDescription));
                       return;
                     }
                     if (result.final) {
                       once->Run(base::SysNSStringToUTF8(
                                     result.bestTranscription.formattedString),
                                 where);
                     }
                   }];
  base::AutoLock hold(TaskLock());
  CurrentTask() = task;
}

}  // namespace

void RecognizeFile(const base::FilePath& path, RecognizedCallback done) {
  auto once = std::make_shared<Once>(
      base::BindPostTaskToCurrentDefault(std::move(done)));
  NSURL* url = [NSURL
      fileURLWithPath:base::SysUTF8ToNSString(path.AsUTF8Unsafe())];
  [SFSpeechRecognizer requestAuthorization:^(
                          SFSpeechRecognizerAuthorizationStatus status) {
    if (status != SFSpeechRecognizerAuthorizationStatusAuthorized) {
      once->Run(std::nullopt,
                "Keine Freigabe für Spracherkennung (Systemeinstellungen › "
                "Datenschutz & Sicherheit › Spracherkennung)");
      return;
    }
    Start(url, once);
  }];
}

bool CancelRecognition() {
  base::AutoLock hold(TaskLock());
  SFSpeechRecognitionTask* task = CurrentTask();
  CurrentTask() = nil;
  if (!task || task.state == SFSpeechRecognitionTaskStateCompleted) {
    return false;
  }
  [task cancel];
  return true;
}

bool ScreenReaderActive() {
  return NSWorkspace.sharedWorkspace.voiceOverEnabled;
}

}  // namespace relief::speech
