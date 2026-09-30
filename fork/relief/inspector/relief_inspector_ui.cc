// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/inspector/relief_inspector_ui.h"

#include <memory>
#include <string>
#include <utility>

#include "base/functional/bind.h"
#include "base/memory/ref_counted_memory.h"
#include "base/memory/weak_ptr.h"
#include "base/scoped_observation.h"
#include "base/time/time.h"
#include "base/timer/timer.h"
#include "base/values.h"
#include "chrome/browser/profiles/profile.h"
#include "components/tabs/public/tab_interface.h"
#include "content/public/browser/web_contents.h"
#include "content/public/browser/web_ui.h"
#include "content/public/browser/web_ui_data_source.h"
#include "content/public/browser/web_ui_message_handler.h"
#include "content/public/common/url_constants.h"
#include "relief/inspector/inspector_resources.h"
#include "relief/relief_tab_helper.h"

namespace relief {

namespace {

// Aktualisierungen bündeln: höchstens so oft neue Daten an die WebUI (bei
// großen Seiten kommen Deltas im Abstand weniger Millisekunden).
constexpr base::TimeDelta kRefreshDelay = base::Milliseconds(250);

const inspector_resources::Resource* FindResource(const std::string& path) {
  const std::string name = path.empty() ? "inspector.html" : path;
  for (const inspector_resources::Resource& resource :
       inspector_resources::kResources) {
    if (name == resource.path) {
      return &resource;
    }
  }
  return nullptr;
}

// Verbindet die WebUI mit dem Tab-Helfer des Tabs, dessen Seite sie zeigt.
class InspectorHandler : public content::WebUIMessageHandler,
                         public ReliefTabHelper::InspectorObserver {
 public:
  InspectorHandler() = default;
  InspectorHandler(const InspectorHandler&) = delete;
  InspectorHandler& operator=(const InspectorHandler&) = delete;
  ~InspectorHandler() override = default;

  // content::WebUIMessageHandler:
  void RegisterMessages() override {
    web_ui()->RegisterMessageCallback(
        "ready", base::BindRepeating(&InspectorHandler::OnReady,
                                     base::Unretained(this)));
    web_ui()->RegisterMessageCallback(
        "show", base::BindRepeating(&InspectorHandler::OnShow,
                                    base::Unretained(this)));
    web_ui()->RegisterMessageCallback(
        "command", base::BindRepeating(&InspectorHandler::OnCommand,
                                       base::Unretained(this)));
    web_ui()->RegisterMessageCallback(
        "cancel", base::BindRepeating(&InspectorHandler::OnCancel,
                                      base::Unretained(this)));
    web_ui()->RegisterMessageCallback(
        "close", base::BindRepeating(&InspectorHandler::OnClose,
                                     base::Unretained(this)));

  }
  void OnJavascriptDisallowed() override {
    observation_.Reset();
    refresh_timer_.Stop();
    weak_factory_.InvalidateWeakPtrs();
  }

  // ReliefTabHelper::InspectorObserver:
  void OnGraphChanged() override {
    if (!refresh_timer_.IsRunning()) {
      refresh_timer_.Start(FROM_HERE, kRefreshDelay, this,
                           &InspectorHandler::Refresh);
    }
  }
  void OnTabHelperDestroyed() override { observation_.Reset(); }
  void OnExternalAnswer(const std::string& input,
                        const ReliefExecutor::Result& result) override {
    if (!IsJavascriptAllowed()) {
      return;
    }
    FireWebUIListener("externalAnswer", base::Value(input),
                      base::Value(result.text), base::Value(result.acted));
  }
  void OnFocusCommandRequested() override {
    if (ReliefTabHelper* helper = Helper()) {
      helper->TakeFocusCommandRequest();
    }
    FocusCommand();
  }

 private:
  // Tab-Helfer der gezeigten Seite; nach einem Discard ein neuer.
  ReliefTabHelper* Helper() {
    InspectorSource* source =
        InspectorSource::FromWebContents(web_ui()->GetWebContents());
    tabs::TabInterface* tab = source ? source->tab() : nullptr;
    ReliefTabHelper* helper =
        tab ? ReliefTabHelper::FromWebContents(tab->GetContents()) : nullptr;
    if (helper && !observation_.IsObservingSource(helper)) {
      observation_.Reset();
      observation_.Observe(helper);
    }
    return helper;
  }

  base::WeakPtr<TopChromeWebUIController::Embedder> Embedder() {
    auto* controller = web_ui()->GetController()->GetAs<ReliefInspectorUI>();
    return controller ? controller->embedder() : nullptr;
  }

  void OnReady(const base::ListValue& args) {
    AllowJavascript();
    // Das Side Panel zeigt den Eintrag erst, wenn die WebUI bereit ist.
    if (auto embedder = Embedder()) {
      embedder->ShowUI();
    }
    Refresh();
    if (ReliefTabHelper* helper = Helper()) {
      if (auto answer = helper->TakeExternalAnswer()) {
        OnExternalAnswer(answer->first, answer->second);
      }
      if (helper->TakeFocusCommandRequest()) {
        FocusCommand();
      }
    }
  }

  // Tastaturfokus ins Panel und dort ins Befehlsfeld.
  void FocusCommand() {
    if (!IsJavascriptAllowed()) {
      return;
    }
    web_ui()->GetWebContents()->Focus();
    FireWebUIListener("focusCommand");
  }

  void OnCommand(const base::ListValue& args) {
    ReliefTabHelper* helper = Helper();
    if (!helper || args.empty() || !args[0].is_string()) {
      return;
    }
    const std::string& input = args[0].GetString();
    helper->Interact(input,
                     base::BindOnce(&InspectorHandler::SendAnswer,
                                    weak_factory_.GetWeakPtr(),
                                    std::string(bridge::redact_input(input))));
  }

  // Nach einer Aktion auf der Seite bekommt die Seite den Tastaturfokus:
  // dort steht das Ziel (Fokus bleibt beim Ziel). Abfragen lassen ihn im
  // Panel. `shown`: die Eingabe fürs Log der Leiste, Wert verdeckt
  // (Paket 76).
  void SendAnswer(std::string shown, ReliefExecutor::Result result) {
    FireWebUIListener("answer", base::Value(std::move(result.text)),
                      base::Value(result.acted), base::Value(std::move(shown)));
    if (result.acted) {
      if (ReliefTabHelper* helper = Helper()) {
        helper->web_contents()->Focus();
      }
    }
  }

  // Abbrechen: eine laufende Ausführung (ihre Antwort meldet das), sonst
  // eine offene Rückfrage in der Runtime.
  void OnCancel(const base::ListValue& args) {
    ReliefTabHelper* helper = Helper();
    if (!helper || helper->executor().Cancel()) {
      return;
    }
    helper->Interact("abbrechen",
                     base::BindOnce(&InspectorHandler::SendAnswer,
                                    weak_factory_.GetWeakPtr(),
                                    std::string("abbrechen")));
  }

  // Schließen gibt den Fokus an die Seite zurück (Side Panel).
  void OnClose(const base::ListValue& args) {
    if (auto embedder = Embedder()) {
      embedder->CloseUI();
    }
  }


  void Refresh() {
    ReliefTabHelper* helper = Helper();
    if (!helper) {
      FireWebUIListener("status", base::Value("Keine Relief-Seite im Tab."));
      return;
    }
    helper->InspectorJson(base::BindOnce(&InspectorHandler::SendGraph,
                                         weak_factory_.GetWeakPtr()));
  }

  void SendGraph(std::string json) {
    FireWebUIListener("graph", base::Value(std::move(json)));
  }

  void OnShow(const base::ListValue& args) {
    ReliefTabHelper* helper = Helper();
    if (!helper || args.empty() || !args[0].is_string()) {
      return;
    }
    helper->Show(args[0].GetString(),
                 base::BindOnce(&InspectorHandler::SendStatus,
                                weak_factory_.GetWeakPtr()));
  }

  void SendStatus(std::string text) {
    FireWebUIListener("status", base::Value(std::move(text)));
  }

  base::ScopedObservation<ReliefTabHelper, ReliefTabHelper::InspectorObserver>
      observation_{this};
  base::OneShotTimer refresh_timer_;
  base::WeakPtrFactory<InspectorHandler> weak_factory_{this};
};

}  // namespace

ReliefInspectorUIConfig::ReliefInspectorUIConfig()
    : DefaultTopChromeWebUIConfig<ReliefInspectorUI>(content::kChromeUIScheme,
                                                     kInspectorHost) {}

ReliefInspectorUI::ReliefInspectorUI(content::WebUI* web_ui)
    : TopChromeWebUIController(web_ui, /*enable_chrome_send=*/true) {
  content::WebUIDataSource* source = content::WebUIDataSource::CreateAndAdd(
      Profile::FromWebUI(web_ui), kInspectorHost);
  source->SetRequestFilter(
      base::BindRepeating(
          [](const std::string& path) { return FindResource(path) != nullptr; }),
      base::BindRepeating(
          [](const std::string& path,
             content::WebUIDataSource::GotDataCallback callback) {
            std::move(callback).Run(
                base::MakeRefCounted<base::RefCountedString>(
                    std::string(FindResource(path)->data)));
          }));
  web_ui->AddMessageHandler(std::make_unique<InspectorHandler>());
}

ReliefInspectorUI::~ReliefInspectorUI() = default;

WEB_UI_CONTROLLER_TYPE_IMPL(ReliefInspectorUI)

InspectorSource::InspectorSource(content::WebContents* contents,
                                 base::WeakPtr<tabs::TabInterface> tab)
    : content::WebContentsUserData<InspectorSource>(*contents),
      tab_(std::move(tab)) {}

InspectorSource::~InspectorSource() = default;

WEB_CONTENTS_USER_DATA_KEY_IMPL(InspectorSource);

}  // namespace relief
