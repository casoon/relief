// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/devtools/relief_devtools.h"

#include <set>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "base/functional/bind.h"
#include "base/json/json_writer.h"
#include "base/no_destructor.h"
#include "base/strings/string_number_conversions.h"
#include "base/values.h"
#include "content/public/browser/devtools_agent_host.h"
#include "content/public/browser/devtools_agent_host_client_channel.h"
#include "content/public/browser/web_contents.h"
#include "relief/relief_tab_helper.h"
#include "third_party/inspector_protocol/crdtp/dispatch.h"
#include "third_party/inspector_protocol/crdtp/json.h"

namespace relief {

namespace {

constexpr std::string_view kDomain = "Relief.";

// Clients, an die eine Antwort noch gehen darf.
std::set<content::DevToolsAgentHostClientChannel*>& LiveChannels() {
  static base::NoDestructor<std::set<content::DevToolsAgentHostClientChannel*>>
      channels;
  return *channels;
}

// `body` ist JSON: bei `ok` das Ergebnisobjekt, sonst die Fehlermeldung.
void Respond(content::DevToolsAgentHostClientChannel* channel,
             int call_id,
             bool ok,
             const std::string& body) {
  if (!LiveChannels().contains(channel)) {
    return;
  }
  std::string json;
  if (ok) {
    json = "{\"id\":" + base::NumberToString(call_id) + ",\"result\":" +
           body + "}";
  } else {
    base::DictValue error;
    error.Set("code", -32000);
    error.Set("message", body);
    base::DictValue response;
    response.Set("id", call_id);
    response.Set("error", std::move(error));
    json = base::WriteJson(response).value_or("{}");
  }
  std::vector<uint8_t> cbor;
  if (!crdtp::json::ConvertJSONToCBOR(crdtp::SpanFrom(json), &cbor).ok()) {
    return;
  }
  channel->DispatchProtocolMessageToClient(std::move(cbor));
}

}  // namespace

bool HandleDevToolsCommand(content::DevToolsAgentHostClientChannel* channel,
                           base::span<const uint8_t> message) {
  // Nur lesen: kein Handler, kein Durchreichen über diesen Weg.
  crdtp::Dispatchable dispatchable(crdtp::SpanFrom(message), std::string_view(),
                                   crdtp::FallthroughCallback());
  if (!dispatchable.ok()) {
    return false;
  }
  const std::string method(
      reinterpret_cast<const char*>(dispatchable.Method().data()),
      dispatchable.Method().size());
  if (!method.starts_with(kDomain)) {
    return false;
  }
  LiveChannels().insert(channel);
  const int call_id = dispatchable.CallId();
  std::string params;
  if (!dispatchable.Params().empty() &&
      !crdtp::json::ConvertCBORToJSON(dispatchable.Params(), &params).ok()) {
    Respond(channel, call_id, false, "Parameter nicht lesbar");
    return true;
  }
  content::WebContents* contents =
      channel->GetAgentHost()->GetWebContents();
  ReliefTabHelper* helper =
      contents ? ReliefTabHelper::FromWebContents(contents) : nullptr;
  if (!helper) {
    Respond(channel, call_id, false,
            "Relief ist für dieses Ziel nicht aktiv (--enable-relief, Tab).");
    return true;
  }
  helper->DevToolsCommand(
      method, params,
      base::BindOnce(
          [](content::DevToolsAgentHostClientChannel* channel, int call_id,
             bridge::DevToolsReply reply) {
            Respond(channel, call_id, reply.ok, std::string(reply.json));
          },
          channel, call_id));
  return true;
}

void OnDevToolsClientDetached(
    content::DevToolsAgentHostClientChannel* channel) {
  LiveChannels().erase(channel);
}

}  // namespace relief
