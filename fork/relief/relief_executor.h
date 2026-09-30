// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_RELIEF_EXECUTOR_H_
#define RELIEF_RELIEF_EXECUTOR_H_

#include <string>

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

// Führt eine Antwort der Runtime aus (bridge::Reply): Schritte als
// AXActionData bzw. Taste senden, Escape, Scrollen; wartet, bis die Seite
// ruht (keine AX-Pakete mehr), und holt die Antwort (finish_command). Eine
// Ausführung je Tab zugleich; Aufgaben-Runner, Befehlsleiste und Inspector
// teilen sie.
class ReliefExecutor {
 public:
  struct Result {
    std::string text;
    // Auf der Seite wurde etwas gesendet (Schritte, Taste, Scrollen).
    bool acted = false;
  };
  using Done = base::OnceCallback<void(Result)>;

  ReliefExecutor(ReliefTabHelper& helper, content::WebContents& contents);
  ReliefExecutor(const ReliefExecutor&) = delete;
  ReliefExecutor& operator=(const ReliefExecutor&) = delete;
  ~ReliefExecutor();

  void Execute(bridge::Reply reply, Done done);

  // Wartet, bis die Seite ruht: mindestens `min`, danach keine
  // Ladeaktivität und kein AX-Paket für 300 ms, und `ready` (falls gesetzt)
  // gilt; höchstens `max`. Dann `then`. Ersetzt eine laufende Wartezeit.
  void WaitForQuiet(base::TimeDelta min,
                    base::TimeDelta max,
                    base::OnceClosure then,
                    base::RepeatingCallback<bool()> ready = {});

  // Bricht das Warten einer Ausführung ab; `done` bekommt sofort eine
  // Antwort, die sagt, dass gesendete Aktionen trotzdem wirken. false, wenn
  // nichts lief.
  bool Cancel();
  bool busy() const { return !done_.is_null(); }

  // Nach einer Aktion: erstes Paket abwarten, höchstens 3 s.
  static constexpr base::TimeDelta kActionMin = base::Milliseconds(150);
  static constexpr base::TimeDelta kActionMax = base::Seconds(3);

 private:
  void Finish();
  void OnScrolled(bridge::ScrollDirection direction, int before);
  void Complete(std::string text);
  void CheckQuiet();

  const raw_ref<ReliefTabHelper> helper_;
  const raw_ref<content::WebContents> contents_;
  Done done_;
  bool acted_ = false;

  base::RepeatingTimer quiet_timer_;
  base::TimeTicks wait_started_;
  base::TimeDelta wait_min_;
  base::TimeDelta wait_max_;
  base::OnceClosure after_quiet_;
  base::RepeatingCallback<bool()> ready_;

  base::WeakPtrFactory<ReliefExecutor> weak_factory_{this};
};

}  // namespace relief

#endif  // RELIEF_RELIEF_EXECUTOR_H_
