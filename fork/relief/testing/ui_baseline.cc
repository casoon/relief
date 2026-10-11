// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/testing/ui_baseline.h"

#include <optional>
#include <utility>

#include "base/json/json_reader.h"
#include "base/json/json_writer.h"
#include "base/strings/string_split.h"
#include "base/strings/string_util.h"
#include "base/test/run_until.h"
#include "content/public/browser/devtools_agent_host.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/web_contents.h"
#include "content/public/test/browser_test_utils.h"
#include "content/public/test/scoped_accessibility_mode_override.h"
#include "testing/gtest/include/gtest/gtest.h"
#include "ui/accessibility/ax_enum_util.h"
#include "ui/accessibility/ax_enums.mojom.h"
#include "ui/accessibility/ax_mode.h"
#include "ui/accessibility/ax_node_data.h"
#include "ui/accessibility/ax_role_properties.h"
#include "ui/accessibility/ax_tree_update.h"
#include "ui/events/keycodes/dom/dom_code.h"
#include "ui/events/keycodes/dom/dom_key.h"
#include "ui/events/keycodes/keyboard_codes.h"

namespace relief::baseline {

namespace {

// HTML-ID eines Knotens (nur mit dem Modus kHTML im Snapshot).
std::string HtmlId(const ui::AXNodeData& node) {
  for (const auto& [name, value] : node.html_attributes) {
    if (name == "id") {
      return value;
    }
  }
  return node.GetStringAttribute(ax::mojom::StringAttribute::kHtmlId);
}

// Snapshot des Accessibility-Trees; nach dem Einschalten des Modus erst,
// wenn der Baum da ist (sonst ist er leer und jede Prüfung bestünde).
ui::AXTreeUpdate Snapshot(content::WebContents* ui) {
  // Mit kHTML, damit HTML-IDs im Snapshot stehen.
  content::ScopedAccessibilityModeOverride mode(ui,
                                                ui::kAXModeDefaultForTests);
  ui::AXTreeUpdate tree;
  EXPECT_TRUE(base::test::RunUntil([&] {
    tree = content::GetAccessibilityTreeSnapshot(ui);
    return tree.nodes.size() > 1;
  })) << "Accessibility-Tree bleibt leer";
  return tree;
}

// Beschreibung eines Elements im Dokument, wie KeyboardWalk sie erwartet.
constexpr char kDescribe[] = R"(
    const describe = (e) => {
      if (!e || e === document.body || e === document.documentElement) {
        return '';
      }
      if (e.id) return '#' + e.id;
      const label = e.labels && e.labels.length ? e.labels[0].textContent : '';
      const text = (e.getAttribute('value') || e.getAttribute('aria-label') ||
                    label || e.textContent || '')
          .trim().replace(/\s+/g, ' ').slice(0, 40);
      return e.tagName.toLowerCase() + '[' + text + ']';
    };)";

Findings Lines(const std::string& text) {
  return base::SplitString(text, "\n", base::TRIM_WHITESPACE,
                           base::SPLIT_WANT_NONEMPTY);
}

}  // namespace

Findings NamesAndRoles(content::WebContents* ui) {
  Findings out;
  const ui::AXTreeUpdate tree = Snapshot(ui);
  for (const ui::AXNodeData& node : tree.nodes) {
    if (node.IsInvisibleOrIgnored()) {
      continue;
    }
    const std::string name =
        node.GetStringAttribute(ax::mojom::StringAttribute::kName);
    const std::string id = HtmlId(node);
    if (ui::IsControl(node.role) && name.empty()) {
      out.push_back(std::string(ui::ToString(node.role)) + " ohne Namen" +
                    (id.empty() ? "" : " (#" + id + ")"));
    }
    if (!ui::IsControl(node.role) && !ui::IsLink(node.role) &&
        node.role != ax::mojom::Role::kListBoxOption &&
        node.role != ax::mojom::Role::kDisclosureTriangle &&
        node.GetDefaultActionVerb() == ax::mojom::DefaultActionVerb::kClick) {
      out.push_back(std::string(ui::ToString(node.role)) +
                    " bietet Klick ohne Bedienrolle" +
                    (id.empty() ? "" : " (#" + id + ")"));
    }
  }
  return out;
}

Findings LiveRegions(content::WebContents* ui,
                     const std::vector<std::string>& ids) {
  Findings out;
  const ui::AXTreeUpdate tree = Snapshot(ui);
  for (const std::string& id : ids) {
    const ui::AXNodeData* found = nullptr;
    for (const ui::AXNodeData& node : tree.nodes) {
      if (HtmlId(node) == id) {
        found = &node;
      }
    }
    if (!found) {
      out.push_back("#" + id + " nicht im Accessibility-Tree");
      continue;
    }
    const bool live =
        found->role == ax::mojom::Role::kStatus ||
        found->role == ax::mojom::Role::kLog ||
        found->role == ax::mojom::Role::kAlert ||
        !found->GetStringAttribute(ax::mojom::StringAttribute::kLiveStatus)
             .empty();
    if (!live) {
      out.push_back("#" + id + " ist keine Live-Region (" +
                    ui::ToString(found->role) + ")");
    }
  }
  return out;
}

Findings KeyboardWalk(content::WebContents* ui,
                      const std::vector<std::string>& expected) {
  Findings out;
  ui->Focus();
  EXPECT_TRUE(base::test::RunUntil(
      [&] { return content::EvalJs(ui, "document.hasFocus()").ExtractBool(); }));
  // Startpunkt der Tab-Folge an den Dokumentanfang: Nach blur() setzt
  // Chromium sie an der zuletzt fokussierten Stelle fort.
  EXPECT_TRUE(content::ExecJs(ui, R"(
      document.body.tabIndex = -1;
      document.body.focus();
      document.body.removeAttribute('tabindex');
      window.scrollTo(0, 0);)"));
  std::vector<std::string> seen;
  const size_t limit = expected.size() * 3 + 20;
  for (size_t i = 0; i < limit; ++i) {
    content::SimulateKeyPress(ui, ui::DomKey::TAB, ui::DomCode::TAB,
                              ui::VKEY_TAB, false, false, false, false);
    // Erst nach der Taste nachsehen, wo der Fokus steht.
    content::RunUntilInputProcessed(
        ui->GetPrimaryMainFrame()->GetRenderWidgetHost());
    const std::string step =
        content::EvalJs(ui, std::string(kDescribe) + R"(
            const e = document.activeElement;
            const d = describe(e);
            let outline = '';
            if (d) {
              const s = getComputedStyle(e);
              if (s.outlineStyle === 'none' || parseFloat(s.outlineWidth) < 2) {
                outline = ' ohne sichtbaren Fokus (' + s.outlineStyle + ' ' +
                    s.outlineWidth + ')';
              }
            }
            d + '|' + outline;)")
            .ExtractString();
    const auto [described, outline] =
        std::pair(step.substr(0, step.find('|')), step.substr(step.find('|') + 1));
    if (described.empty()) {
      break;
    }
    if (!seen.empty() && seen.back() == described) {
      out.push_back("Tastaturfalle bei " + described);
      break;
    }
    if (!outline.empty()) {
      out.push_back(described + outline);
    }
    seen.push_back(described);
  }
  if (seen.size() >= limit) {
    out.push_back("Fokus verlässt die Oberfläche nicht");
  }
  size_t next = 0;
  for (const std::string& d : seen) {
    if (next < expected.size() && d == expected[next]) {
      ++next;
    }
  }
  if (next < expected.size()) {
    out.push_back("erwartet „" + expected[next] +
                  "“ in der Tab-Folge: " + base::JoinString(seen, " → "));
  }
  return out;
}

Findings Reflow(content::WebContents* ui) {
  return Lines(content::EvalJs(ui, std::string(kDescribe) + R"(
      const w = document.documentElement.clientWidth;
      const out = [];
      if (document.documentElement.scrollWidth > w + 1) {
        out.push('waagerecht scrollbar: ' +
            document.documentElement.scrollWidth + ' > ' + w);
      }
      for (const e of document.querySelectorAll('body *')) {
        const r = e.getBoundingClientRect();
        if (r.width === 0 || r.height === 0 ||
            getComputedStyle(e).visibility === 'hidden') {
          continue;
        }
        if (r.right > w + 1) {
          out.push((describe(e) || e.tagName) + ' ragt hinaus (' +
              Math.round(r.right) + ' > ' + w + ')');
        }
      }
      out.slice(0, 10).join('\n');)")
                   .ExtractString());
}

Findings NoMotion(content::WebContents* ui) {
  return Lines(content::EvalJs(ui, std::string(kDescribe) + R"(
      const out = [];
      const moves = (list) => list.split(',').some((d) => parseFloat(d) > 0);
      for (const e of document.querySelectorAll('*')) {
        const s = getComputedStyle(e);
        if (moves(s.transitionDuration) ||
            (s.animationName !== 'none' && moves(s.animationDuration))) {
          out.push((describe(e) || e.tagName) + ' bewegt sich (' +
              s.transitionDuration + ', ' + s.animationName + ')');
        }
      }
      out.slice(0, 10).join('\n');)")
                   .ExtractString());
}

CdpClient::CdpClient(content::WebContents* contents)
    : host_(content::DevToolsAgentHost::GetOrCreateFor(contents)) {
  host_->AttachClient(this);
}

CdpClient::~CdpClient() {
  host_->DetachClient(this);
}

base::DictValue CdpClient::Send(const std::string& method,
                                base::DictValue params) {
  const int id = ++last_id_;
  base::DictValue message;
  message.Set("id", id);
  message.Set("method", method);
  message.Set("params", std::move(params));
  const std::string json = base::WriteJson(message).value_or("{}");
  host_->DispatchProtocolMessage(this, base::as_byte_span(json));
  EXPECT_TRUE(base::test::RunUntil([&] { return replies_.contains(id); }));
  return std::move(replies_[id]);
}

void CdpClient::DispatchProtocolMessage(content::DevToolsAgentHost*,
                                        base::span<const uint8_t> message) {
  std::optional<base::DictValue> value = base::JSONReader::ReadDict(
      base::as_string_view(message), base::JSON_PARSE_RFC);
  if (value && value->FindInt("id")) {
    const int id = *value->FindInt("id");
    replies_[id] = std::move(*value);
  }
}

void EmulateMedia(
    CdpClient& cdp,
    const std::vector<std::pair<std::string, std::string>>& features) {
  base::ListValue list;
  for (const auto& [name, value] : features) {
    base::DictValue feature;
    feature.Set("name", name);
    feature.Set("value", value);
    list.Append(std::move(feature));
  }
  base::DictValue params;
  params.Set("features", std::move(list));
  const base::DictValue reply =
      cdp.Send("Emulation.setEmulatedMedia", std::move(params));
  EXPECT_TRUE(reply.FindDict("result")) << reply.DebugString();
}

}  // namespace relief::baseline
