// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_INSPECTOR_RELIEF_INSPECTOR_UI_H_
#define RELIEF_INSPECTOR_RELIEF_INSPECTOR_UI_H_

#include <string_view>

#include "base/memory/weak_ptr.h"
#include "chrome/browser/ui/webui/top_chrome/top_chrome_web_ui_controller.h"
#include "chrome/browser/ui/webui/top_chrome/top_chrome_webui_config.h"
#include "content/public/browser/web_contents_user_data.h"

namespace tabs {
class TabInterface;
}  // namespace tabs

namespace relief {

inline constexpr char kInspectorHost[] = "relief-inspector.top-chrome";
inline constexpr char kInspectorUrl[] = "chrome://relief-inspector.top-chrome/";

class ReliefInspectorUI;

class ReliefInspectorUIConfig
    : public DefaultTopChromeWebUIConfig<ReliefInspectorUI> {
 public:
  ReliefInspectorUIConfig();
};

// WebUI des Inspectors. Ressourcen aus inspector/resources/ (eingebettet
// von embed_resources.py), Nachrichten über chrome.send: "ready" (Seite
// geladen), "show" (Schlüssel eines Eintrags im Dokument zeigen); zur Seite
// die Listener "graph" (JSON aus der Runtime) und "status" (Antworttext).
class ReliefInspectorUI : public TopChromeWebUIController {
 public:
  explicit ReliefInspectorUI(content::WebUI* web_ui);
  ReliefInspectorUI(const ReliefInspectorUI&) = delete;
  ReliefInspectorUI& operator=(const ReliefInspectorUI&) = delete;
  ~ReliefInspectorUI() override;

  static constexpr std::string_view GetWebUIName() { return "ReliefInspector"; }

 private:
  WEB_UI_CONTROLLER_TYPE_DECL();
};

// Am WebContents des Inspectors: der Tab, dessen Seite er zeigt.
class InspectorSource : public content::WebContentsUserData<InspectorSource> {
 public:
  ~InspectorSource() override;
  tabs::TabInterface* tab() const { return tab_.get(); }

 private:
  friend class content::WebContentsUserData<InspectorSource>;
  InspectorSource(content::WebContents* contents,
                  base::WeakPtr<tabs::TabInterface> tab);

  base::WeakPtr<tabs::TabInterface> tab_;

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

}  // namespace relief

#endif  // RELIEF_INSPECTOR_RELIEF_INSPECTOR_UI_H_
