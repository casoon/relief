// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/relief_task_runner.h"

#include <unistd.h>

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
#include "relief/relief_executor.h"
#include "relief/relief_tab_helper.h"
#include "ui/base/page_transition_types.h"
#include "url/gurl.h"

namespace relief {

namespace {

// Laden einer Seite: mindestens so lange, höchstens 10 s.
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
        case bridge::TaskKind::Assert:
          out.push_back({Line::Kind::kAssert, value});
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
      case Line::Kind::kAssert:
        if (!page_open_) {
          continue;
        }
        // Die Zusicherungen brauchen DOM-Fakten, die der Fork nicht erhebt
        // (→ relief-cdp, Paket 42).
        last_ = "Zusicherung im Fork nicht verfügbar: " + line.text;
        Print("\n? " + line.text + "\n" + Indent(last_));
        continue;
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
  helper_->executor().WaitForQuiet(
      kLoadMin, kLoadMax,
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
  helper_->Interact(input, base::BindOnce(
                               [](base::WeakPtr<ReliefTaskRunner> self,
                                  ReliefExecutor::Result result) {
                                 if (self) {
                                   self->Answer(std::move(result.text));
                                 }
                               },
                               weak_factory_.GetWeakPtr()));
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

}  // namespace relief
