// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/relief_executor.h"

#include <algorithm>
#include <optional>
#include <utility>

#include "base/functional/bind.h"
#include "content/public/browser/web_contents.h"
#include "relief/relief_tab_helper.h"

namespace relief {

namespace {

// Ruhe: so lange kein AX-Paket (Baum oder Positionen). Blink serialisiert
// Änderungen gebündelt mit dem nächsten Frame; 300 ms reichen auf den
// Testseiten auch für Wirkungen aus Event-Handlern mit kurzer Verzögerung.
constexpr base::TimeDelta kQuiet = base::Milliseconds(300);
constexpr base::TimeDelta kPoll = base::Milliseconds(50);

}  // namespace

ReliefExecutor::ReliefExecutor(ReliefTabHelper& helper,
                               content::WebContents& contents)
    : helper_(helper), contents_(contents) {}

ReliefExecutor::~ReliefExecutor() = default;

void ReliefExecutor::Execute(bridge::Reply reply, Done done) {
  // Eine laufende Ausführung verliert ihre Antwort nicht stillschweigend.
  Cancel();
  done_ = std::move(done);
  acted_ = false;
  switch (reply.kind) {
    case bridge::ReplyKind::Answer:
      Complete(std::string(reply.text));
      return;
    case bridge::ReplyKind::Perform:
      for (const bridge::Step& step : reply.steps) {
        if (!helper_->PerformStep(step)) {
          Complete("Aktion fehlgeschlagen: Knoten nicht mehr im Baum.");
          return;
        }
        acted_ = true;
      }
      WaitForQuiet(kActionMin, kActionMax,
                   base::BindOnce(&ReliefExecutor::Finish,
                                  weak_factory_.GetWeakPtr()));
      return;
    case bridge::ReplyKind::Escape:
      helper_->PressKey(bridge::Key::Escape);
      acted_ = true;
      WaitForQuiet(kActionMin, kActionMax,
                   base::BindOnce(&ReliefExecutor::Finish,
                                  weak_factory_.GetWeakPtr()));
      return;
    case bridge::ReplyKind::Scroll: {
      const std::optional<ReliefTabHelper::MainScroll> scroll =
          helper_->GetMainScroll();
      if (!scroll) {
        Complete("Scroll: Position des Dokuments unbekannt.");
        return;
      }
      int target = scroll->y;
      switch (reply.scroll) {
        case bridge::ScrollDirection::Down:
          target += scroll->page;
          break;
        case bridge::ScrollDirection::Up:
          target -= scroll->page;
          break;
        case bridge::ScrollDirection::Top:
          target = 0;
          break;
        case bridge::ScrollDirection::Bottom:
          target = scroll->y_max;
          break;
      }
      target = std::clamp(target, 0, std::max(scroll->y_max, 0));
      if (target != scroll->y) {
        if (!helper_->ScrollMainTo(target)) {
          Complete("Scroll: Dokument lässt sich nicht scrollen.");
          return;
        }
        acted_ = true;
      }
      // Chromium meldet die neue Scroll-Position gebündelt mit den
      // Positionen, teils erst nach der Ruhe-Spanne: auf sie warten.
      auto moved = base::BindRepeating(
          [](base::WeakPtr<ReliefExecutor> self, int before, int target) {
            if (!self || target == before) {
              return true;
            }
            const std::optional<ReliefTabHelper::MainScroll> now =
                self->helper_->GetMainScroll();
            return now && now->y != before;
          },
          weak_factory_.GetWeakPtr(), scroll->y, target);
      WaitForQuiet(kActionMin, kActionMax,
                   base::BindOnce(&ReliefExecutor::OnScrolled,
                                  weak_factory_.GetWeakPtr(), reply.scroll,
                                  scroll->y),
                   std::move(moved));
      return;
    }
  }
}

void ReliefExecutor::Finish() {
  helper_->FinishCommand(
      base::BindOnce(&ReliefExecutor::Complete, weak_factory_.GetWeakPtr()));
}

void ReliefExecutor::OnScrolled(bridge::ScrollDirection direction,
                                int before) {
  const std::optional<ReliefTabHelper::MainScroll> scroll =
      helper_->GetMainScroll();
  if (!scroll) {
    Complete("Scroll: Position des Dokuments unbekannt.");
    return;
  }
  Complete(std::string(
      bridge::scrolled_text(direction, before, scroll->y, scroll->y_max)));
}

void ReliefExecutor::Complete(std::string text) {
  if (done_) {
    std::move(done_).Run(Result{std::move(text), acted_});
  }
}

bool ReliefExecutor::Cancel() {
  if (!done_) {
    return false;
  }
  quiet_timer_.Stop();
  after_quiet_.Reset();
  ready_.Reset();
  Complete(acted_ ? "Abgebrochen. Bereits gesendete Aktionen wirken; ihre "
                    "Wirkung wird nicht mehr gemeldet."
                  : "Abgebrochen. Nichts ausgeführt.");
  return true;
}

void ReliefExecutor::WaitForQuiet(base::TimeDelta min,
                                  base::TimeDelta max,
                                  base::OnceClosure then,
                                  base::RepeatingCallback<bool()> ready) {
  wait_started_ = base::TimeTicks::Now();
  wait_min_ = min;
  wait_max_ = max;
  after_quiet_ = std::move(then);
  ready_ = std::move(ready);
  quiet_timer_.Start(FROM_HERE, kPoll, this, &ReliefExecutor::CheckQuiet);
}

void ReliefExecutor::CheckQuiet() {
  const base::TimeTicks now = base::TimeTicks::Now();
  const base::TimeTicks last = std::max(helper_->last_packet(), wait_started_);
  const bool quiet = now - wait_started_ >= wait_min_ &&
                     !contents_->IsLoading() && now - last >= kQuiet &&
                     (!ready_ || ready_.Run());
  if (!quiet && now - wait_started_ < wait_max_) {
    return;
  }
  quiet_timer_.Stop();
  std::move(after_quiet_).Run();
}

}  // namespace relief
