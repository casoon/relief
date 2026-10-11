// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_TESTING_UI_BASELINE_H_
#define RELIEF_TESTING_UI_BASELINE_H_

#include <map>
#include <string>
#include <vector>

#include "base/memory/scoped_refptr.h"
#include "base/values.h"
#include "content/public/browser/devtools_agent_host_client.h"

namespace content {
class DevToolsAgentHost;
class WebContents;
}  // namespace content

namespace relief::baseline {

// Prüfungen der User-Agent-Baseline (plan/spezifikation/13, Paket 47) für
// eine Relief-Oberfläche, die als WebContents vorliegt (WebUI im Side
// Panel). Jede Prüfung liefert die Abweichungen als Text; leer heißt
// erfüllt. Wiederverwendbar für jede neue Oberfläche.
using Findings = std::vector<std::string>;

// Name und Rolle aus dem Accessibility-Tree: Jedes nicht ausgeblendete
// Bedienelement hat einen Namen; kein Element ohne Bedienrolle bietet einen
// Klick an.
Findings NamesAndRoles(content::WebContents* ui);

// Statusmeldungen: Die Elemente mit diesen HTML-IDs sind Live-Regionen
// (status, log, alert oder aria-live) im Accessibility-Tree.
Findings LiveRegions(content::WebContents* ui,
                     const std::vector<std::string>& ids);

// Tastatur: echte Tab-Tasten ab dem Dokumentanfang. `expected` erscheint
// in dieser Reihenfolge (Beschreibung wie `Describe` in ui_baseline.cc:
// `#id` oder `tag[Wert oder Text]`); kein Element zweimal hintereinander
// (Tastaturfalle); nach dem letzten verlässt der Fokus die Oberfläche;
// jedes Element zeigt im Fokus einen Umriss von mindestens 2 px.
Findings KeyboardWalk(content::WebContents* ui,
                      const std::vector<std::string>& expected);

// Umbruch: kein waagerechtes Scrollen und kein sichtbares Element ragt
// über den Viewport (200 % Zoom, schmales Panel).
Findings Reflow(content::WebContents* ui);

// Bewegung: keine Übergänge und Animationen (reduzierte Bewegung).
Findings NoMotion(content::WebContents* ui);

// DevTools-Sitzung an einer WebContents, für Emulation in Tests
// (Medienmerkmale) und für CDP-Aufrufe wie Playwright oder der
// MCP-Server.
class CdpClient : public content::DevToolsAgentHostClient {
 public:
  explicit CdpClient(content::WebContents* contents);
  ~CdpClient() override;

  // Ganze Antwort auf den Aufruf: `result` oder `error`.
  base::DictValue Send(const std::string& method, base::DictValue params);

  void DispatchProtocolMessage(content::DevToolsAgentHost*,
                               base::span<const uint8_t> message) override;
  void AgentHostClosed(content::DevToolsAgentHost*) override {}

 private:
  scoped_refptr<content::DevToolsAgentHost> host_;
  int last_id_ = 0;
  std::map<int, base::DictValue> replies_;
};

// Medienmerkmale emulieren, z. B. {{"prefers-reduced-motion", "reduce"},
// {"forced-colors", "active"}}; leer hebt die Emulation auf.
void EmulateMedia(CdpClient& cdp,
                  const std::vector<std::pair<std::string, std::string>>&
                      features);

}  // namespace relief::baseline

#endif  // RELIEF_TESTING_UI_BASELINE_H_
