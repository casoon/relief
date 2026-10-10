// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_WEBMCP_CLIENT_H_
#define RELIEF_WEBMCP_CLIENT_H_

#include <map>
#include <string>

#include "base/functional/callback.h"
#include "base/memory/raw_ref.h"
#include "base/memory/scoped_refptr.h"
#include "base/timer/timer.h"
#include "base/values.h"
#include "content/public/browser/devtools_agent_host_client.h"

namespace content {
class DevToolsAgentHost;
class WebContents;
}  // namespace content

namespace relief {

// Werkzeuge, die Dokumente des Tabs über WebMCP erklären (Paket 143), über
// eine eigene DevTools-Sitzung je Tab und die Domäne `WebMCP` (Chromium
// 154: Blink-Agent `InspectorWebMCPAgent`, kein Patch in content/). Hält je
// Frame die Liste des aktuellen Dokuments und meldet sie je Dokumentbaum
// (AXTreeID) weiter; ruft Werkzeuge auf und liefert ihre Antwort.
//
// Grenze: Die Sitzung des Tabs sieht Frames im Prozess des Hauptdokuments,
// nicht Werkzeuge in cross-site-iframes (OOPIF).
class WebMcpClient : public content::DevToolsAgentHostClient {
 public:
  // Baum (AXTreeID als Text) und alle Werkzeuge seines Dokuments als
  // JSON-Array von `WebMCP.Tool`-Objekten.
  using ToolsChanged =
      base::RepeatingCallback<void(const std::string& tree,
                                   const std::string& tools_json)>;
  // Status wie `WebMCP.InvocationStatus` oder "Timeout", Ausgabe als JSON
  // (bei Fehlern der Fehlertext).
  using Responded =
      base::OnceCallback<void(std::string status, std::string output)>;

  WebMcpClient(content::WebContents& contents, ToolsChanged tools_changed);
  WebMcpClient(const WebMcpClient&) = delete;
  WebMcpClient& operator=(const WebMcpClient&) = delete;
  ~WebMcpClient() override;

  // Liste neu anfordern (nach Back-Forward-Cache: Blink meldet die
  // Werkzeuge einer wiederhergestellten Seite nicht von selbst).
  void Resync();
  // Werkzeug `name` im Dokument `tree` mit `input` (JSON-Objekt) aufrufen.
  void Invoke(const std::string& tree,
              const std::string& name,
              const std::string& input,
              Responded done);

  // content::DevToolsAgentHostClient:
  void DispatchProtocolMessage(content::DevToolsAgentHost* agent_host,
                               base::span<const uint8_t> message) override;
  void AgentHostClosed(content::DevToolsAgentHost* agent_host) override;

 private:
  struct FrameTools {
    std::string tree;
    // Name → `WebMCP.Tool`.
    std::map<std::string, base::DictValue> tools;
  };

  int Send(const std::string& method, base::DictValue params);
  void OnToolsAdded(const base::ListValue& tools);
  void OnToolsRemoved(const base::ListValue& tools);
  void OnResponded(const base::DictValue& params);
  void OnTimeout();
  void Finish(std::string status, std::string output);
  // Liste des Frames mit dem aktuellen Dokument; ein neues Dokument im
  // selben Frame beginnt leer.
  FrameTools* Current(const std::string& frame_id);
  void Report(const FrameTools& frame);

  const raw_ref<content::WebContents> contents_;
  ToolsChanged tools_changed_;
  scoped_refptr<content::DevToolsAgentHost> host_;
  int last_id_ = 0;
  std::map<std::string, FrameTools> frames_;

  // Ein Aufruf zur Zeit (wie die Befehlsleiste).
  Responded pending_;
  int pending_call_ = 0;
  std::string pending_invocation_;
  base::OneShotTimer timeout_;
};

}  // namespace relief

#endif  // RELIEF_WEBMCP_CLIENT_H_
