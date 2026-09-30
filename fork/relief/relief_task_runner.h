// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_RELIEF_TASK_RUNNER_H_
#define RELIEF_RELIEF_TASK_RUNNER_H_

#include <optional>
#include <string>
#include <vector>

#include "base/files/file_path.h"
#include "base/functional/callback.h"
#include "base/memory/raw_ref.h"
#include "base/memory/weak_ptr.h"
#include "base/time/time.h"
#include "base/timer/timer.h"
#include "relief/crates/relief-bridge/src/cxx_bridge.rs.h"

namespace content {
class WebContents;
}  // namespace content

namespace relief {

class ReliefTabHelper;

// Arbeitet Aufgabendateien (spike/tasks/*.txt) im Fork ab wie
// `relief-cdp run`: `url:` laden, `do:` als Befehl an die Runtime, die
// geprüften Schritte als AXActionData senden, Ruhe abwarten (keine
// AX-Pakete mehr), Antwort der Runtime mit `expect:` vergleichen. Ausgabe
// auf stdout; am Ende beendet sich der Browser (--relief-run).
class ReliefTaskRunner {
 public:
  // `files`: Pfade, kommagetrennt.
  ReliefTaskRunner(ReliefTabHelper& helper,
                   content::WebContents& contents,
                   const base::FilePath::StringType& files);
  ReliefTaskRunner(const ReliefTaskRunner&) = delete;
  ReliefTaskRunner& operator=(const ReliefTaskRunner&) = delete;
  ~ReliefTaskRunner();

  // Zeile einer Aufgabendatei; `kFile` trennt die Dateien in der Ausgabe,
  // `kUrl` trägt die aufgelöste URL.
  struct Line {
    enum class Kind { kFile, kUrl, kDo, kExpect };
    Kind kind;
    std::string text;
  };

 private:
  void Loaded(std::vector<Line> lines);
  void Next();
  void Open(const std::string& url);
  void Execute(const std::string& input);
  void OnReply(bridge::Reply reply);
  void OnScrolled(bridge::ScrollDirection direction, int before);
  void Finish();
  void Answer(std::string text);
  // Wartet, bis die Seite ruht: mindestens `min`, danach keine Ladeaktivität
  // und kein AX-Paket für kQuiet, und `ready` (falls gesetzt) gilt;
  // höchstens `max`. Dann `then`.
  void WaitForQuiet(base::TimeDelta min,
                    base::TimeDelta max,
                    base::OnceClosure then,
                    base::RepeatingCallback<bool()> ready = {});
  void CheckQuiet();

  const raw_ref<ReliefTabHelper> helper_;
  const raw_ref<content::WebContents> contents_;
  std::vector<Line> lines_;
  size_t next_ = 0;
  // Letzte Antwort, gegen die `expect:` prüft; ohne geladene Seite werden
  // `do:` und `expect:` übergangen (wie im CDP-Host).
  std::string last_;
  bool page_open_ = false;
  base::TimeTicks command_started_;
  int passed_ = 0;
  int failed_ = 0;

  base::RepeatingTimer quiet_timer_;
  base::TimeTicks wait_started_;
  base::TimeDelta wait_min_;
  base::TimeDelta wait_max_;
  base::OnceClosure after_quiet_;
  base::RepeatingCallback<bool()> ready_;

  base::WeakPtrFactory<ReliefTaskRunner> weak_factory_{this};
};

}  // namespace relief

#endif  // RELIEF_RELIEF_TASK_RUNNER_H_
