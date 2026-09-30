// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/relief_task_runner.h"

#include <unistd.h>

#include <algorithm>
#include <utility>

#include "base/files/file_util.h"
#include "base/functional/bind.h"
#include "base/process/process.h"
#include "base/strings/string_split.h"
#include "base/strings/string_util.h"
#include "base/strings/stringprintf.h"
#include "base/task/thread_pool.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/web_contents.h"
#include "net/base/filename_util.h"
#include "relief/relief_tab_helper.h"
#include "ui/base/page_transition_types.h"
#include "url/gurl.h"

namespace relief {

namespace {

// Ruhe: so lange kein AX-Paket (Baum oder Positionen). Blink serialisiert
// Änderungen gebündelt mit dem nächsten Frame; 300 ms reichen auf den
// Testseiten auch für Wirkungen aus Event-Handlern mit kurzer Verzögerung.
constexpr base::TimeDelta kQuiet = base::Milliseconds(300);
constexpr base::TimeDelta kPoll = base::Milliseconds(50);
// Nach einer Aktion: erstes Paket abwarten, höchstens 3 s (wie der
// CDP-Host); Laden einer Seite höchstens 10 s.
constexpr base::TimeDelta kActionMin = base::Milliseconds(150);
constexpr base::TimeDelta kActionMax = base::Seconds(3);
constexpr base::TimeDelta kLoadMin = base::Milliseconds(300);
constexpr base::TimeDelta kLoadMax = base::Seconds(10);

void Print(const std::string& text) {
  base::WriteFileDescriptor(STDOUT_FILENO, text + "\n");
}

std::string Indent(const std::string& text) {
  std::vector<std::string> lines = base::SplitString(
      text, "\n", base::KEEP_WHITESPACE, base::SPLIT_WANT_ALL);
  for (std::string& line : lines) {
    line = "  " + line;
  }
  return base::JoinString(lines, "\n");
}

// URL einer `url:`-Zeile: absolut, oder ein Pfad relativ zur Aufgabendatei.
std::string ResolveUrl(const std::string& url, const base::FilePath& dir) {
  if (url.find("://") != std::string::npos || url.starts_with("about:")) {
    return url;
  }
  const base::FilePath path = dir.AppendASCII(url);
  const base::FilePath absolute = base::MakeAbsoluteFilePath(path);
  return net::FilePathToFileURL(absolute.empty() ? path : absolute).spec();
}

// Liest und zerlegt die Dateien (blockierend, im Thread-Pool).
std::vector<ReliefTaskRunner::Line> ReadTasks(
    base::FilePath::StringType files) {
  using Line = ReliefTaskRunner::Line;
  std::vector<Line> out;
  for (const base::FilePath::StringType& name :
       base::SplitString(files, FILE_PATH_LITERAL(","), base::TRIM_WHITESPACE,
                         base::SPLIT_WANT_NONEMPTY)) {
    base::FilePath file = base::MakeAbsoluteFilePath(base::FilePath(name));
    std::string text;
    if (file.empty() || !base::ReadFileToString(file, &text)) {
      out.push_back(
          {Line::Kind::kFile, base::FilePath(name).AsUTF8Unsafe() + " (nicht lesbar)"});
      continue;
    }
    out.push_back({Line::Kind::kFile, file.AsUTF8Unsafe()});
    for (const bridge::Task& task : bridge::parse_task_file(text)) {
      const std::string value(task.text);
      switch (task.kind) {
        case bridge::TaskKind::Url:
          out.push_back({Line::Kind::kUrl, ResolveUrl(value, file.DirName())});
          break;
        case bridge::TaskKind::Do:
          out.push_back({Line::Kind::kDo, value});
          break;
        case bridge::TaskKind::Expect:
          out.push_back({Line::Kind::kExpect, value});
          break;
      }
    }
  }
  return out;
}

}  // namespace

ReliefTaskRunner::ReliefTaskRunner(ReliefTabHelper& helper,
                                   content::WebContents& contents,
                                   const base::FilePath::StringType& files)
    : helper_(helper), contents_(contents) {
  base::ThreadPool::PostTaskAndReplyWithResult(
      FROM_HERE, {base::MayBlock(), base::TaskPriority::USER_VISIBLE},
      base::BindOnce(&ReadTasks, files),
      base::BindOnce(&ReliefTaskRunner::Loaded, weak_factory_.GetWeakPtr()));
}

ReliefTaskRunner::~ReliefTaskRunner() = default;

void ReliefTaskRunner::Loaded(std::vector<Line> lines) {
  lines_ = std::move(lines);
  Next();
}

void ReliefTaskRunner::Next() {
  while (next_ < lines_.size()) {
    const Line& line = lines_[next_++];
    switch (line.kind) {
      case Line::Kind::kFile:
        Print("\n=== " + line.text + " ===");
        page_open_ = false;
        continue;
      case Line::Kind::kUrl:
        Open(line.text);
        return;
      case Line::Kind::kDo:
        if (!page_open_) {
          continue;
        }
        Execute(line.text);
        return;
      case Line::Kind::kExpect:
        if (!page_open_) {
          continue;
        }
        if (bridge::expectation_met(last_, line.text)) {
          ++passed_;
          Print("  ✓ erwartet „" + line.text + "“");
        } else {
          ++failed_;
          Print("  ✗ erwartet „" + line.text + "“ — FEHLT");
        }
        continue;
    }
  }
  Finish();
}

void ReliefTaskRunner::Open(const std::string& url) {
  Print("\n## " + url);
  page_open_ = false;
  content::NavigationController::LoadURLParams params{GURL(url)};
  params.transition_type = ui::PAGE_TRANSITION_TYPED;
  contents_->GetController().LoadURLWithParams(params);
  // Bis die Navigation committet ist und der neue Hauptbaum angekommen
  // ist; vorher ruht die Seite womöglich nur, weil sie noch nicht begonnen
  // hat (erster Tab beim Start).
  auto loaded = base::BindRepeating(
      [](base::WeakPtr<ReliefTaskRunner> self, GURL url) {
        return self && self->helper_->has_main_tree() &&
               self->contents_->GetLastCommittedURL() == url;
      },
      weak_factory_.GetWeakPtr(), GURL(url));
  WaitForQuiet(kLoadMin, kLoadMax,
               base::BindOnce(
                   [](base::WeakPtr<ReliefTaskRunner> self) {
                     if (!self) {
                       return;
                     }
                     if (!self->helper_->has_main_tree()) {
                       Print("!! Seite nicht ladbar: kein Baum");
                       self->Next();
                       return;
                     }
                     self->helper_->DescribePage(base::BindOnce(
                         [](base::WeakPtr<ReliefTaskRunner> self,
                            std::string text) {
                           if (!self) {
                             return;
                           }
                           Print(text);
                           self->page_open_ = true;
                           self->Next();
                         },
                         self));
                   },
                   weak_factory_.GetWeakPtr()),
               std::move(loaded));
}

void ReliefTaskRunner::Execute(const std::string& input) {
  Print("\n> " + input);
  command_started_ = base::TimeTicks::Now();
  helper_->RunCommand(input, base::BindOnce(&ReliefTaskRunner::OnReply,
                                            weak_factory_.GetWeakPtr()));
}

void ReliefTaskRunner::OnReply(bridge::Reply reply) {
  auto finish = base::BindOnce(
      [](base::WeakPtr<ReliefTaskRunner> self) {
        if (!self) {
          return;
        }
        self->helper_->FinishCommand(
            base::BindOnce(&ReliefTaskRunner::Answer, self));
      },
      weak_factory_.GetWeakPtr());
  switch (reply.kind) {
    case bridge::ReplyKind::Answer:
      Answer(std::string(reply.text));
      return;
    case bridge::ReplyKind::Perform:
      for (const bridge::Step& step : reply.steps) {
        if (!helper_->PerformStep(step)) {
          Answer("Aktion fehlgeschlagen: Knoten nicht mehr im Baum.");
          return;
        }
      }
      WaitForQuiet(kActionMin, kActionMax, std::move(finish));
      return;
    case bridge::ReplyKind::Escape:
      helper_->PressKey(bridge::Key::Escape);
      WaitForQuiet(kActionMin, kActionMax, std::move(finish));
      return;
    case bridge::ReplyKind::Scroll: {
      const std::optional<ReliefTabHelper::MainScroll> scroll =
          helper_->GetMainScroll();
      if (!scroll) {
        Answer("Scroll: Position des Dokuments unbekannt.");
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
      if (target != scroll->y && !helper_->ScrollMainTo(target)) {
        Answer("Scroll: Dokument lässt sich nicht scrollen.");
        return;
      }
      // Chromium meldet die neue Scroll-Position gebündelt mit den
      // Positionen, teils erst nach der Ruhe-Spanne: auf sie warten.
      auto moved = base::BindRepeating(
          [](base::WeakPtr<ReliefTaskRunner> self, int before, int target) {
            if (!self || target == before) {
              return true;
            }
            const std::optional<ReliefTabHelper::MainScroll> now =
                self->helper_->GetMainScroll();
            return now && now->y != before;
          },
          weak_factory_.GetWeakPtr(), scroll->y, target);
      WaitForQuiet(kActionMin, kActionMax,
                   base::BindOnce(&ReliefTaskRunner::OnScrolled,
                                  weak_factory_.GetWeakPtr(), reply.scroll,
                                  scroll->y),
                   std::move(moved));
      return;
    }
  }
}

void ReliefTaskRunner::OnScrolled(bridge::ScrollDirection direction,
                                  int before) {
  const std::optional<ReliefTabHelper::MainScroll> scroll =
      helper_->GetMainScroll();
  if (!scroll) {
    Answer("Scroll: Position des Dokuments unbekannt.");
    return;
  }
  Answer(std::string(
      bridge::scrolled_text(direction, before, scroll->y, scroll->y_max)));
}

void ReliefTaskRunner::Answer(std::string text) {
  last_ = std::move(text);
  Print(Indent(last_));
  Print(base::StringPrintf(
      "(%lld ms)",
      (base::TimeTicks::Now() - command_started_).InMilliseconds()));
  Next();
}

void ReliefTaskRunner::Finish() {
  Print(base::StringPrintf("\nErwartungen: %d erfüllt, %d nicht erfüllt",
                           passed_, failed_));
  // Ein Lauf ohne Fenster-Schließen: der Browser hat seine Aufgabe erledigt.
  base::Process::TerminateCurrentProcessImmediately(failed_ ? 1 : 0);
}

void ReliefTaskRunner::WaitForQuiet(base::TimeDelta min,
                                    base::TimeDelta max,
                                    base::OnceClosure then,
                                    base::RepeatingCallback<bool()> ready) {
  wait_started_ = base::TimeTicks::Now();
  wait_min_ = min;
  wait_max_ = max;
  after_quiet_ = std::move(then);
  ready_ = std::move(ready);
  quiet_timer_.Start(FROM_HERE, kPoll, this, &ReliefTaskRunner::CheckQuiet);
}

void ReliefTaskRunner::CheckQuiet() {
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
