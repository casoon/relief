// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/inspector/relief_inspector.h"

#include <memory>

#include "base/functional/bind.h"
#include "base/no_destructor.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/ui/actions/chrome_action_id.h"
#include "chrome/browser/ui/browser_actions.h"
#include "ui/actions/actions.h"
#include "components/vector_icons/vector_icons.h"
#include "ui/base/models/image_model.h"
#include "ui/color/color_id.h"
#include "chrome/browser/ui/browser_window/public/browser_window_interface.h"
#include "chrome/browser/ui/side_panel/side_panel_entry.h"
#include "chrome/browser/ui/side_panel/side_panel_entry_scope.h"
#include "chrome/browser/ui/side_panel/side_panel_registry.h"
#include "chrome/browser/ui/side_panel/side_panel_ui.h"
#include "chrome/browser/ui/views/side_panel/side_panel_web_ui_view.h"
#include "chrome/browser/ui/webui/top_chrome/webui_contents_wrapper.h"
#include "chrome/grit/generated_resources.h"
#include "components/tabs/public/tab_interface.h"
#include "content/public/browser/webui_config_map.h"
#include "relief/inspector/relief_inspector_ui.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/web_contents.h"
#include "content/public/browser/web_ui.h"
#include "content/public/browser/web_ui_controller.h"
#include "ui/base/page_transition_types.h"
#include "ui/views/view.h"
#include "url/gurl.h"

namespace relief {

namespace {

// Eintrag im Side Panel des Tabs (Patch: SidePanelEntryId::kRelief).
SidePanelEntry::Key InspectorKey() {
  return SidePanelEntry::Key(SidePanelEntry::Id::kRelief);
}

// Wie WebUIContentsWrapperT, aber ohne dessen static_assert auf Chromiums
// Liste von WebUI-Namen für Metriken (tools/metrics/histograms, sonst ein
// weiterer Patch). Der Name taucht nur in Messwerten auf.
class InspectorContentsWrapper : public WebUIContentsWrapper {
 public:
  explicit InspectorContentsWrapper(Profile* profile)
      : WebUIContentsWrapper(GURL(kInspectorUrl),
                             profile,
                             IDS_SETTINGS_ACCESSIBILITY,
                             /*webui_resizes_host=*/false,
                             /*esc_closes_ui=*/false,
                             /*supports_draggable_regions=*/false,
                             ReliefInspectorUI::GetWebUIName()) {
    SetEmbedder();
  }

  void ReloadWebContents() override {
    web_contents()->GetController().LoadURL(
        GURL(kInspectorUrl), content::Referrer(),
        ui::PAGE_TRANSITION_AUTO_TOPLEVEL, std::string());
    SetEmbedder();
  }

  base::WeakPtr<WebUIContentsWrapper> GetWeakPtr() override {
    return weak_factory_.GetWeakPtr();
  }

 private:
  void SetEmbedder() {
    content::WebUI* webui = web_contents()->GetWebUI();
    if (webui && webui->GetController()) {
      if (auto* controller =
              webui->GetController()->GetAs<ReliefInspectorUI>()) {
        controller->set_embedder(weak_factory_.GetWeakPtr());
      }
    }
  }

  base::WeakPtrFactory<WebUIContentsWrapper> weak_factory_{this};
};

// Side-Panel-Ansicht, die ihren Wrapper besitzt (wie SidePanelWebUIViewT).
class InspectorView : public SidePanelWebUIView {
 public:
  InspectorView(SidePanelEntryScope& scope,
                std::unique_ptr<InspectorContentsWrapper> wrapper)
      : SidePanelWebUIView(
            scope,
            base::RepeatingClosure(),
            // Schließen aus der WebUI (Escape): das Side Panel des Fensters.
            base::BindRepeating(
                [](BrowserWindowInterface* browser) {
                  SidePanelUI::From(browser)->Close();
                },
                &scope.GetBrowserWindowInterface()),
            wrapper.get()),
        wrapper_(std::move(wrapper)) {}

 private:
  std::unique_ptr<InspectorContentsWrapper> wrapper_;
};

std::unique_ptr<views::View> CreateInspectorView(
    base::WeakPtr<tabs::TabInterface> tab,
    SidePanelEntryScope& scope) {
  auto wrapper = std::make_unique<InspectorContentsWrapper>(
      scope.GetBrowserWindowInterface().GetProfile());
  InspectorSource::CreateForWebContents(wrapper->web_contents(), tab);
  return std::make_unique<InspectorView>(scope, std::move(wrapper));
}

// Aktions-Element des Eintrags (Titel in der Kopfzeile des Side Panels),
// einmal je Fenster; Chromium legt seine eigenen in BrowserActions an.
void RegisterAction(BrowserWindowInterface& browser) {
  actions::ActionItem* root = BrowserActions::From(&browser)->root_action_item();
  if (actions::ActionManager::Get().FindAction(kActionSidePanelShowRelief,
                                               root)) {
    return;
  }
  root->AddChild(
      actions::ActionItem::Builder(
          base::BindRepeating(
              [](BrowserWindowInterface* bwi, actions::ActionItem*,
                 actions::ActionInvocationContext) {
                if (tabs::TabInterface* tab = bwi->GetActiveTabInterface()) {
                  ToggleInspector(*tab);
                }
              },
              &browser))
          .SetActionId(kActionSidePanelShowRelief)
          .SetText(u"Relief")
          .SetTooltipText(
              u"Relief: Befehl (Strg+Umschalt+Leertaste), Inspector "
              u"(Strg+Umschalt+I)")
          // Platzhalter bis zum Branding (Paket 36).
          .SetImage(ui::ImageModel::FromVectorIcon(vector_icons::kVisibilityIcon,
                                                   ui::kColorIcon))
          .Build());
}

// Registriert den Eintrag, falls der Tab ihn noch nicht hat; nullptr ohne
// Browserfenster.
SidePanelUI* PrepareInspector(tabs::TabInterface& tab) {
  RegisterInspectorWebUI();
  BrowserWindowInterface* browser = tab.GetBrowserWindowInterface();
  SidePanelRegistry* registry = SidePanelRegistry::From(&tab);
  if (!browser || !registry) {
    return nullptr;
  }
  RegisterAction(*browser);
  if (!registry->GetEntryForKey(InspectorKey())) {
    registry->Register(std::make_unique<SidePanelEntry>(
        InspectorKey(),
        base::BindRepeating(&CreateInspectorView, tab.GetWeakPtr()),
        /*default_content_width_callback=*/base::NullCallback()));
  }
  return SidePanelUI::From(browser);
}

}  // namespace

void RegisterInspectorWebUI() {
  static bool registered = false;
  if (registered) {
    return;
  }
  registered = true;
  content::WebUIConfigMap::GetInstance().AddWebUIConfig(
      std::make_unique<ReliefInspectorUIConfig>());
}

bool ShowInspector(tabs::TabInterface& tab) {
  SidePanelUI* side_panel = PrepareInspector(tab);
  if (!side_panel) {
    return false;
  }
  side_panel->Show(InspectorKey());
  return true;
}

void ToggleInspector(tabs::TabInterface& tab) {
  SidePanelUI* side_panel = PrepareInspector(tab);
  if (!side_panel) {
    return;
  }
  if (side_panel->IsSidePanelEntryShowing(InspectorKey())) {
    side_panel->Close();
  } else {
    side_panel->Show(InspectorKey());
  }
}

}  // namespace relief
