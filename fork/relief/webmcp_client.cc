// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/webmcp_client.h"

#include <utility>

#include "base/functional/bind.h"
#include "base/json/json_reader.h"
#include "base/json/json_writer.h"
#include "base/time/time.h"
#include "content/public/browser/devtools_agent_host.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/web_contents.h"
#include "ui/accessibility/ax_tree_id.h"

namespace relief {

namespace {

// So lange wartet ein Aufruf auf `WebMCP.toolResponded`; ein Werkzeug, das
// navigiert, antwortet womöglich nie.
constexpr base::TimeDelta kTimeout = base::Seconds(5);

std::string Json(const base::Value& value) {
  return base::WriteJson(value).value_or("null");
}

}  // namespace

WebMcpClient::WebMcpClient(content::WebContents& contents,
                           ToolsChanged tools_changed)
    : contents_(contents),
      tools_changed_(std::move(tools_changed)),
      host_(content::DevToolsAgentHost::GetOrCreateFor(&contents)) {
  host_->AttachClient(this);
  Send("WebMCP.enable", {});
}

WebMcpClient::~WebMcpClient() {
  if (host_) {
    host_->DetachClient(this);
  }
}

void WebMcpClient::Resync() {
  frames_.clear();
  Send("WebMCP.disable", {});
  Send("WebMCP.enable", {});
}

void WebMcpClient::Invoke(const std::string& tree,
                          const std::string& name,
                          const std::string& input,
                          Responded done) {
  if (pending_) {
    Finish("Error", "Ein anderer Aufruf läuft noch.");
  }
  pending_ = std::move(done);
  std::string frame_id;
  for (const auto& [id, frame] : frames_) {
    if (frame.tree == tree && frame.tools.contains(name)) {
      frame_id = id;
    }
  }
  std::optional<base::DictValue> parsed = base::JSONReader::ReadDict(
      input, base::JSON_PARSE_RFC);
  if (frame_id.empty() || !parsed || !host_) {
    Finish("Error", "Werkzeug nicht mehr erklärt.");
    return;
  }
  base::DictValue params;
  params.Set("frameId", frame_id);
  params.Set("toolName", name);
  params.Set("input", std::move(*parsed));
  pending_call_ = Send("WebMCP.invokeTool", std::move(params));
  timeout_.Start(FROM_HERE, kTimeout,
                 base::BindOnce(&WebMcpClient::OnTimeout,
                                base::Unretained(this)));
}

int WebMcpClient::Send(const std::string& method, base::DictValue params) {
  const int id = ++last_id_;
  if (!host_) {
    return id;
  }
  base::DictValue message;
  message.Set("id", id);
  message.Set("method", method);
  message.Set("params", std::move(params));
  const std::string json = Json(base::Value(std::move(message)));
  host_->DispatchProtocolMessage(this, base::as_byte_span(json));
  return id;
}

void WebMcpClient::DispatchProtocolMessage(
    content::DevToolsAgentHost* agent_host,
    base::span<const uint8_t> message) {
  std::optional<base::DictValue> value = base::JSONReader::ReadDict(
      base::as_string_view(message), base::JSON_PARSE_RFC);
  if (!value) {
    return;
  }
  // Antwort auf invokeTool: Kennung des Aufrufs oder Fehler (etwa bei
  // einem Prozesswechsel abgebrochen).
  if (std::optional<int> id = value->FindInt("id");
      id && pending_ && *id == pending_call_) {
    if (const std::string* invocation =
            value->FindStringByDottedPath("result.invocationId")) {
      pending_invocation_ = *invocation;
    } else {
      const std::string* error =
          value->FindStringByDottedPath("error.message");
      Finish("Error", error ? *error : std::string());
    }
    return;
  }
  const std::string* method = value->FindString("method");
  const base::DictValue* params = value->FindDict("params");
  if (!method || !params) {
    return;
  }
  if (*method == "WebMCP.toolsAdded") {
    if (const base::ListValue* tools = params->FindList("tools")) {
      OnToolsAdded(*tools);
    }
  } else if (*method == "WebMCP.toolsRemoved") {
    if (const base::ListValue* tools = params->FindList("tools")) {
      OnToolsRemoved(*tools);
    }
  } else if (*method == "WebMCP.toolResponded") {
    OnResponded(*params);
  }
}

void WebMcpClient::AgentHostClosed(content::DevToolsAgentHost* agent_host) {
  host_ = nullptr;
  if (pending_) {
    Finish("Error", "Tab geschlossen.");
  }
}

WebMcpClient::FrameTools* WebMcpClient::Current(const std::string& frame_id) {
  content::RenderFrameHost* found = nullptr;
  contents_->ForEachRenderFrameHost([&](content::RenderFrameHost* frame) {
    if (frame->IsActive() &&
        frame->GetDevToolsFrameToken().ToString() == frame_id) {
      found = frame;
    }
  });
  if (!found) {
    frames_.erase(frame_id);
    return nullptr;
  }
  const std::string tree = found->GetAXTreeID().ToString();
  FrameTools& frame = frames_[frame_id];
  if (frame.tree != tree) {
    frame = FrameTools{tree, {}};
  }
  return &frame;
}

void WebMcpClient::Report(const FrameTools& frame) {
  base::ListValue list;
  for (const auto& [name, tool] : frame.tools) {
    list.Append(tool.Clone());
  }
  tools_changed_.Run(frame.tree, Json(base::Value(std::move(list))));
}

void WebMcpClient::OnToolsAdded(const base::ListValue& tools) {
  std::map<std::string, FrameTools*> changed;
  for (const base::Value& entry : tools) {
    const base::DictValue* tool = entry.GetIfDict();
    const std::string* name = tool ? tool->FindString("name") : nullptr;
    const std::string* frame_id = tool ? tool->FindString("frameId") : nullptr;
    if (!name || !frame_id) {
      continue;
    }
    if (FrameTools* frame = Current(*frame_id)) {
      base::DictValue copy = tool->Clone();
      // Aufrufstapel der Registrierung braucht Relief nicht.
      copy.Remove("stackTrace");
      frame->tools[*name] = std::move(copy);
      changed[*frame_id] = frame;
    }
  }
  for (const auto& [id, frame] : changed) {
    Report(*frame);
  }
}

void WebMcpClient::OnToolsRemoved(const base::ListValue& tools) {
  std::map<std::string, FrameTools*> changed;
  for (const base::Value& entry : tools) {
    const base::DictValue* tool = entry.GetIfDict();
    const std::string* name = tool ? tool->FindString("name") : nullptr;
    const std::string* frame_id = tool ? tool->FindString("frameId") : nullptr;
    if (!name || !frame_id) {
      continue;
    }
    if (FrameTools* frame = Current(*frame_id)) {
      frame->tools.erase(*name);
      changed[*frame_id] = frame;
    }
  }
  for (const auto& [id, frame] : changed) {
    Report(*frame);
  }
}

void WebMcpClient::OnResponded(const base::DictValue& params) {
  const std::string* invocation = params.FindString("invocationId");
  if (!pending_ || !invocation || *invocation != pending_invocation_) {
    return;
  }
  const std::string* status = params.FindString("status");
  std::string output;
  if (const base::Value* value = params.Find("output")) {
    output = Json(*value);
  } else if (const std::string* error = params.FindString("errorText")) {
    output = *error;
  }
  Finish(status ? *status : "Error", std::move(output));
}

void WebMcpClient::OnTimeout() {
  Finish("Timeout", std::string());
}

void WebMcpClient::Finish(std::string status, std::string output) {
  timeout_.Stop();
  pending_call_ = 0;
  pending_invocation_.clear();
  if (pending_) {
    std::move(pending_).Run(std::move(status), std::move(output));
  }
}

}  // namespace relief
