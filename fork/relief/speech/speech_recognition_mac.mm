// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/speech/speech_recognition.h"

#import <AVFoundation/AVFoundation.h>
#import <AppKit/AppKit.h>
#import <Speech/Speech.h>

#include <memory>
#include <utility>

#include "base/functional/bind.h"
#include "base/no_destructor.h"
#include "base/strings/sys_string_conversions.h"
#include "base/synchronization/lock.h"
#include "base/task/bind_post_task.h"

namespace relief::speech {

namespace {

// Stille nach dem letzten erkannten Wort, die eine Äußerung beendet.
constexpr int64_t kSilenceNs = 1500 * NSEC_PER_MSEC;
// Zeit bis zum ersten Wort (Nachdenken, Freigabe bestätigen).
constexpr int64_t kFirstWordNs = 6 * NSEC_PER_SEC;
// Längste Äußerung.
constexpr int64_t kMaxNs = 15 * NSEC_PER_SEC;

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

// Eine laufende Äußerung. Zustand nur auf der Hauptwarteschlange.
struct Utterance {
  SFSpeechAudioBufferRecognitionRequest* __strong request = nil;
  SFSpeechRecognitionTask* __strong task = nil;
  AVAudioEngine* __strong engine = nil;
  std::shared_ptr<Once> once;
  std::string best;
  std::string where;
  uint64_t generation = 0;
  // Zwischenergebnisse bisher (jedes Wort setzt die Stille zurück).
  uint64_t results = 0;
};

Utterance& Current() {
  static base::NoDestructor<Utterance> utterance;
  return *utterance;
}

void StopAudio(Utterance& u) {
  if (u.engine) {
    [u.engine.inputNode removeTapOnBus:0];
    [u.engine stop];
    u.engine = nil;
  }
}

// Äußerung abschließen: kein weiterer Ton, das Endergebnis kommt.
void EndAudio(uint64_t generation) {
  Utterance& u = Current();
  if (u.generation != generation || !u.request) {
    return;
  }
  StopAudio(u);
  [u.request endAudio];
}

void Finish(std::optional<std::string> text, std::string note) {
  Utterance& u = Current();
  StopAudio(u);
  std::shared_ptr<Once> once = std::move(u.once);
  u.request = nil;
  u.task = nil;
  u.generation++;
  if (once) {
    once->Run(std::move(text), std::move(note));
  }
}

// Beendet die Äußerung, wenn seit `heard` (Zahl der Ergebnisse) nichts
// Neues kam.
void EndAfterSilence(uint64_t generation, int64_t wait_ns) {
  const uint64_t heard = Current().results;
  dispatch_after(dispatch_time(DISPATCH_TIME_NOW, wait_ns),
                 dispatch_get_main_queue(), ^{
                   if (Current().results == heard) {
                     EndAudio(generation);
                   }
                 });
}

// Audiodatei in den Strom geben (für Tests derselbe Weg wie das Mikrofon).
bool FeedFile(const base::FilePath& path,
              SFSpeechAudioBufferRecognitionRequest* request) {
  NSError* error = nil;
  AVAudioFile* file = [[AVAudioFile alloc]
      initForReading:[NSURL fileURLWithPath:base::SysUTF8ToNSString(
                                                path.AsUTF8Unsafe())]
               error:&error];
  if (!file) {
    return false;
  }
  AVAudioFormat* format = file.processingFormat;
  while (true) {
    AVAudioPCMBuffer* buffer =
        [[AVAudioPCMBuffer alloc] initWithPCMFormat:format frameCapacity:4096];
    if (![file readIntoBuffer:buffer error:&error] ||
        buffer.frameLength == 0) {
      break;
    }
    [request appendAudioPCMBuffer:buffer];
  }
  return true;
}

void Start(std::optional<base::FilePath> file) {
  Utterance& u = Current();
  SFSpeechRecognizer* recognizer = [[SFSpeechRecognizer alloc]
      initWithLocale:[NSLocale localeWithLocaleIdentifier:@"de-DE"]];
  if (!recognizer || !recognizer.available) {
    Finish(std::nullopt, "Spracherkennung für Deutsch nicht verfügbar");
    return;
  }
  SFSpeechAudioBufferRecognitionRequest* request =
      [[SFSpeechAudioBufferRecognitionRequest alloc] init];
  request.shouldReportPartialResults = YES;
  // Auf dem Gerät, wo möglich: Die Audiodaten verlassen den Rechner nicht.
  const bool on_device = recognizer.supportsOnDeviceRecognition;
  request.requiresOnDeviceRecognition = on_device;
  u.where = on_device ? "auf dem Gerät" : "Apple-Server";
  u.request = request;
  const uint64_t generation = ++u.generation;
  u.task = [recognizer
      recognitionTaskWithRequest:request
                   resultHandler:^(SFSpeechRecognitionResult* result,
                                   NSError* error) {
                     dispatch_async(dispatch_get_main_queue(), ^{
                       Utterance& now = Current();
                       if (now.generation != generation) {
                         return;
                       }
                       if (result) {
                         now.best = base::SysNSStringToUTF8(
                             result.bestTranscription.formattedString);
                         now.results++;
                       }
                       if (result.final || error) {
                         if (now.best.empty()) {
                           Finish(std::nullopt,
                                  error ? base::SysNSStringToUTF8(
                                              error.localizedDescription)
                                        : "nichts verstanden");
                         } else {
                           Finish(now.best, now.where);
                         }
                         return;
                       }
                       // Neues Wort: Stille neu messen.
                       EndAfterSilence(generation, kSilenceNs);
                     });
                   }];
  if (file) {
    if (!FeedFile(*file, request)) {
      Finish(std::nullopt, "Audiodatei nicht lesbar");
      return;
    }
    [request endAudio];
    return;
  }
  AVAudioEngine* engine = [[AVAudioEngine alloc] init];
  AVAudioInputNode* input = engine.inputNode;
  [input installTapOnBus:0
              bufferSize:1024
                  format:[input outputFormatForBus:0]
                   block:^(AVAudioPCMBuffer* buffer, AVAudioTime* when) {
                     [request appendAudioPCMBuffer:buffer];
                   }];
  [engine prepare];
  NSError* error = nil;
  if (![engine startAndReturnError:&error]) {
    [input removeTapOnBus:0];
    Finish(std::nullopt, "Mikrofon nicht verfügbar");
    return;
  }
  u.engine = engine;
  u.results = 0;
  EndAfterSilence(generation, kFirstWordNs);
  dispatch_after(dispatch_time(DISPATCH_TIME_NOW, kMaxNs),
                 dispatch_get_main_queue(), ^{
                   EndAudio(generation);
                 });
}

}  // namespace

void Listen(std::optional<base::FilePath> file, RecognizedCallback done) {
  // Eine laufende Äußerung endet ohne Ergebnis.
  CancelRecognition();
  Current().once = std::make_shared<Once>(
      base::BindPostTaskToCurrentDefault(std::move(done)));
  Current().best.clear();
  const uint64_t generation = Current().generation;
  [SFSpeechRecognizer requestAuthorization:^(
                          SFSpeechRecognizerAuthorizationStatus status) {
    dispatch_async(dispatch_get_main_queue(), ^{
      if (Current().generation != generation) {
        return;
      }
      if (status != SFSpeechRecognizerAuthorizationStatusAuthorized) {
        Finish(std::nullopt,
               "Keine Freigabe für Spracherkennung (Systemeinstellungen › "
               "Datenschutz & Sicherheit › Spracherkennung)");
        return;
      }
      if (file) {
        Start(file);
        return;
      }
      [AVCaptureDevice
          requestAccessForMediaType:AVMediaTypeAudio
                  completionHandler:^(BOOL granted) {
                    dispatch_async(dispatch_get_main_queue(), ^{
                      if (Current().generation != generation) {
                        return;
                      }
                      if (!granted) {
                        Finish(std::nullopt,
                               "Keine Freigabe für das Mikrofon "
                               "(Systemeinstellungen › Datenschutz & "
                               "Sicherheit › Mikrofon)");
                        return;
                      }
                      Start(std::nullopt);
                    });
                  }];
    });
  }];
}

bool StopListening() {
  Utterance& u = Current();
  if (!u.request) {
    return false;
  }
  EndAudio(u.generation);
  return true;
}

bool IsListening() {
  return Current().once != nullptr;
}

bool CancelRecognition() {
  Utterance& u = Current();
  const bool running = u.once != nullptr;
  if (u.task) {
    [u.task cancel];
  }
  StopAudio(u);
  u.request = nil;
  u.task = nil;
  u.once = nullptr;
  u.generation++;
  return running;
}

bool ScreenReaderActive() {
  return NSWorkspace.sharedWorkspace.voiceOverEnabled;
}

}  // namespace relief::speech
