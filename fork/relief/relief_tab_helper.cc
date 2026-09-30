// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/relief_tab_helper.h"

#include <optional>
#include <set>
#include <string>
#include <utility>
#include <vector>

#include "base/command_line.h"
#include "base/functional/bind.h"
#include "base/notreached.h"
#include "base/strings/string_split.h"
#include "base/strings/stringprintf.h"
#include "base/task/bind_post_task.h"
#include "base/task/thread_pool.h"
#include "base/time/time.h"
#include "components/tabs/public/tab_interface.h"
#include "content/public/browser/browser_accessibility_state.h"
#include "content/public/browser/host_zoom_map.h"
#include "content/public/browser/navigation_handle.h"
#include "content/public/browser/page.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/render_widget_host_view.h"
#include "content/public/browser/scoped_accessibility_mode.h"
#include "content/public/browser/web_contents.h"
#include "relief/relief_attach.h"
#include "relief/relief_switches.h"
#include "third_party/blink/public/common/page/page_zoom.h"
#include "ui/accessibility/ax_action_data.h"
#include "ui/accessibility/ax_action_handler_base.h"
#include "ui/accessibility/ax_enum_util.h"
#include "ui/accessibility/ax_enums.mojom.h"
#include "ui/accessibility/ax_location_and_scroll_updates.h"
#include "ui/accessibility/ax_mode.h"
#include "ui/accessibility/ax_updates_and_events.h"
#include "url/origin.h"

namespace relief {

namespace {

// Laufende Nummer für das Protokoll (nur UI-Thread).
int g_next_tab = 1;

// Mindestabstand zwischen zwei WebContents::ResetAccessibility. Ein Reset
// serialisiert alle Frames neu (Vollbaum ~8 000 Knoten: 15–23 ms
// Unserialize im UI-Thread, → plan/spezifikation/09); ein Baum, der immer
// wieder aus dem Tritt kommt, soll das nicht in Schleife auslösen.
constexpr base::TimeDelta kResetInterval = base::Seconds(5);

ax::mojom::Action ToAXAction(bridge::Action action) {
  switch (action) {
    case bridge::Action::DoDefault:
      return ax::mojom::Action::kDoDefault;
    case bridge::Action::Focus:
      return ax::mojom::Action::kFocus;
    case bridge::Action::Blur:
      return ax::mojom::Action::kBlur;
    case bridge::Action::SetValue:
      return ax::mojom::Action::kSetValue;
    case bridge::Action::Increment:
      return ax::mojom::Action::kIncrement;
    case bridge::Action::Decrement:
      return ax::mojom::Action::kDecrement;
    case bridge::Action::Expand:
      return ax::mojom::Action::kExpand;
    case bridge::Action::Collapse:
      return ax::mojom::Action::kCollapse;
    case bridge::Action::ScrollToMakeVisible:
      return ax::mojom::Action::kScrollToMakeVisible;
    case bridge::Action::ShowContextMenu:
      return ax::mojom::Action::kShowContextMenu;
  }
  // Die Runtime plant nur gültige Aktionen (plan_action lehnt unbekannte
  // Enum-Werte ab).
  NOTREACHED();
}

}  // namespace

base::CallbackListSubscription AttachToTab(tabs::TabInterface& tab) {
  if (!base::CommandLine::ForCurrentProcess()->HasSwitch(
          switches::kEnableRelief)) {
    return {};
  }
  ReliefTabHelper::CreateForWebContents(tab.GetContents());
  return tab.RegisterWillDiscardContents(base::BindRepeating(
      [](tabs::TabInterface*, content::WebContents*,
         content::WebContents* new_contents) {
        ReliefTabHelper::CreateForWebContents(new_contents);
      }));
}

ReliefTabHelper::ReliefTabHelper(content::WebContents* contents)
    : content::WebContentsObserver(contents),
      content::WebContentsUserData<ReliefTabHelper>(*contents),
      reset_interval_(kResetInterval) {
  const base::CommandLine& command_line =
      *base::CommandLine::ForCurrentProcess();
  std::optional<std::string> activate;
  if (command_line.HasSwitch(switches::kReliefActivate)) {
    activate = command_line.GetSwitchValueUTF8(switches::kReliefActivate);
  }
  std::vector<std::string> log_roles = base::SplitString(
      command_line.GetSwitchValueASCII(switches::kReliefLogNodes), ",",
      base::TRIM_WHITESPACE, base::SPLIT_WANT_NONEMPTY);
  runtime_ = base::SequenceBound<RuntimeHost>(
      base::ThreadPool::CreateSequencedTaskRunner(
          {base::MayBlock(), base::TaskPriority::USER_VISIBLE}),
      g_next_tab++, command_line.GetSwitchValuePath(switches::kReliefLog),
      std::move(activate),
      std::set<std::string>(log_roles.begin(), log_roles.end()),
      base::BindPostTaskToCurrentDefault(base::BindRepeating(
          &ReliefTabHelper::Perform, weak_factory_.GetWeakPtr())));
  registry_observation_.Observe(ui::AXActionHandlerRegistry::GetInstance());

  // Weg B braucht nur kWebContents (kAXModeWebContentsOnly wie
  // chrome.automation); kNativeAPIs bleibt Sache von VoiceOver & Co.
  ui::AXMode mode = ui::kAXModeWebContentsOnly;
  if (command_line.HasSwitch(switches::kReliefScreenReaderMode)) {
    mode |= ui::AXMode::kScreenReader;
  }
  // Wie Reading Mode: Ist kWebContents schon an, bekommt ein neuer
  // Beobachter den vollständigen Baum nur nach einem Reset.
  const bool need_reset =
      contents->GetAccessibilityMode().has_mode(ui::AXMode::kWebContents);
  mode_ = content::BrowserAccessibilityState::GetInstance()
              ->CreateScopedModeForWebContents(contents, mode);
  runtime_.AsyncCall(&RuntimeHost::Log).WithArgs("mode\t" + mode.ToString());
  if (need_reset) {
    Reset("attach");
  }
}

ReliefTabHelper::~ReliefTabHelper() = default;

void ReliefTabHelper::AccessibilityEventReceived(
    const ui::AXUpdatesAndEvents& details) {
  PacketTiming timing;
  timing.received = base::TimeTicks::Now();
  timing.updates = details.updates.size();

  std::unique_ptr<AXTreeMirror>& mirror = trees_[details.ax_tree_id];
  if (!mirror) {
    // Frames im Back-Forward-Cache serialisieren weiter; ihre Bäume gehören
    // nicht zur angezeigten Seite (→ PrimaryPageChanged).
    content::RenderFrameHost* frame =
        content::RenderFrameHost::FromAXTreeID(details.ax_tree_id);
    if (!frame || !frame->IsActive()) {
      trees_.erase(details.ax_tree_id);
      return;
    }
    // Protokoll: welcher Frame (Hauptframe, iframe im selben Prozess oder
    // OOPIF) und von welchem Origin.
    const content::RenderFrameHost* parent = frame->GetParent();
    mirror = std::make_unique<AXTreeMirror>(details.ax_tree_id.ToString(),
                                            /*child=*/parent != nullptr);
    const char* kind = "main";
    if (parent) {
      kind = parent->GetProcess() == frame->GetProcess() ? "iframe" : "oopif";
    }
    runtime_.AsyncCall(&RuntimeHost::Log)
        .WithArgs(base::StringPrintf(
            "tree\t%s\t%s\t%s", kind, details.ax_tree_id.ToString().c_str(),
            frame->GetLastCommittedOrigin().Serialize().c_str()));
  }
  // Ohne Wurzel ist der Baum neu und der Runtime unbekannt; verworfen wird
  // nur er selbst, nicht der Eintrag in hosts_ (der iframe-Knoten im
  // Elternbaum bleibt gültig).
  switch (mirror->CheckStart(details)) {
    case AXTreeMirror::Start::kReady:
      break;
    case AXTreeMirror::Start::kDataOnly:
      // Nur Baumdaten vor dem ersten Baum: übergehen, der Baum folgt (neues
      // Dokument) oder kommt über den Reset nach Back-Forward-Cache.
      runtime_.AsyncCall(&RuntimeHost::Log).WithArgs("skip\tdata-only");
      trees_.erase(details.ax_tree_id);
      return;
    case AXTreeMirror::Start::kNoRoot:
      // Inkrementelles Paket für einen Baum, den Relief nicht hat: verwerfen
      // und den Baum neu serialisieren lassen.
      runtime_.AsyncCall(&RuntimeHost::Log).WithArgs("skip\tno-root");
      trees_.erase(details.ax_tree_id);
      RequestReset("no-root");
      return;
  }
  for (const ui::AXTreeUpdate& update : details.updates) {
    if (!mirror->Unserialize(update)) {
      // Nur in offiziellen Builds ohne DCHECK erreichbar (sonst bricht
      // AXTree selbst ab, → AXTreeMirror::CheckStart). Der Baum ist aus dem
      // Tritt: verwerfen und neu serialisieren lassen.
      runtime_.AsyncCall(&RuntimeHost::Log)
          .WithArgs("error\tunserialize\t" + mirror->error());
      DropTree(details.ax_tree_id);
      RequestReset("unserialize");
      return;
    }
  }
  const base::TimeTicks unserialized = base::TimeTicks::Now();
  timing.unserialize = unserialized - timing.received;

  bridge::Delta delta{};
  const ui::AXTreeID main =
      web_contents()->GetPrimaryMainFrame()->GetAXTreeID();
  if (main != root_ && trees_.contains(main)) {
    root_ = main;
    delta.has_root = true;
    delta.root = rust::String::lossy(main.ToString());
  }
  TakeUpdates(*mirror, delta.trees);
  timing.convert = base::TimeTicks::Now() - unserialized;
  if (delta.trees.empty() && !delta.has_root) {
    return;
  }
  runtime_.AsyncCall(&RuntimeHost::Apply).WithArgs(std::move(delta), timing);
}

void ReliefTabHelper::AccessibilityLocationChangesReceived(
    const ui::AXTreeID& tree_id,
    ui::AXLocationAndScrollUpdates& details) {
  auto it = trees_.find(tree_id);
  if (it == trees_.end()) {
    return;
  }
  PacketTiming timing;
  timing.kind = "location";
  timing.received = base::TimeTicks::Now();
  timing.updates =
      details.location_changes.size() + details.scroll_changes.size();
  it->second->ApplyLocationChanges(details);
  const base::TimeTicks applied = base::TimeTicks::Now();
  timing.unserialize = applied - timing.received;

  bridge::Delta delta{};
  TakeUpdates(*it->second, delta.trees);
  timing.convert = base::TimeTicks::Now() - applied;
  if (delta.trees.empty()) {
    return;
  }
  runtime_.AsyncCall(&RuntimeHost::Apply).WithArgs(std::move(delta), timing);
}

void ReliefTabHelper::PrimaryPageChanged(content::Page& page) {
  std::set<ui::AXTreeID> current;
  page.GetMainDocument().ForEachRenderFrameHost(
      [&current](content::RenderFrameHost* frame) {
        current.insert(frame->GetAXTreeID());
      });
  std::vector<ui::AXTreeID> stale;
  for (const auto& [tree_id, mirror] : trees_) {
    if (!current.contains(tree_id)) {
      stale.push_back(tree_id);
    }
  }
  for (const ui::AXTreeID& tree_id : stale) {
    DropTree(tree_id);
  }
  runtime_.AsyncCall(&RuntimeHost::Log)
      .WithArgs(base::StringPrintf("page\tdropped=%zu\ttrees=%zu\thosts=%zu",
                                   stale.size(), trees_.size(), hosts_.size()));
}

void ReliefTabHelper::DidFinishNavigation(
    content::NavigationHandle* navigation_handle) {
  // Eine Seite aus dem Back-Forward-Cache schickt keinen neuen Baum; ihre
  // Bäume hat PrimaryPageChanged beim Verlassen verworfen.
  if (navigation_handle->IsInPrimaryMainFrame() &&
      navigation_handle->HasCommitted() &&
      navigation_handle->IsServedFromBackForwardCache()) {
    RequestReset("back-forward-cache");
  }
}

void ReliefTabHelper::TreeRemoved(ui::AXTreeID tree_id) {
  DropTree(tree_id);
}

void ReliefTabHelper::SetDeltaObserverForTesting(
    RuntimeHost::DeltaCallback callback) {
  runtime_.AsyncCall(&RuntimeHost::SetDeltaObserverForTesting)
      .WithArgs(std::move(callback));
}

void ReliefTabHelper::RequestReset(std::string_view reason) {
  const base::TimeTicks now = base::TimeTicks::Now();
  if (last_reset_.is_null() || now - last_reset_ >= reset_interval_) {
    Reset(reason);
    return;
  }
  if (reset_timer_.IsRunning()) {
    runtime_.AsyncCall(&RuntimeHost::Log)
        .WithArgs("reset\tpending\t" + std::string(reason));
    return;
  }
  runtime_.AsyncCall(&RuntimeHost::Log)
      .WithArgs("reset\tdeferred\t" + std::string(reason));
  // Unretained: der Timer gehört dem Helfer.
  reset_timer_.Start(FROM_HERE, last_reset_ + reset_interval_ - now,
                     base::BindOnce(&ReliefTabHelper::Reset,
                                    base::Unretained(this), "deferred"));
}

void ReliefTabHelper::Reset(std::string_view reason) {
  last_reset_ = base::TimeTicks::Now();
  ++resets_;
  runtime_.AsyncCall(&RuntimeHost::Log)
      .WithArgs("reset\t" + std::string(reason));
  web_contents()->ResetAccessibility();
}

float ReliefTabHelper::Scale() const {
  const content::RenderWidgetHostView* view =
      web_contents()->GetRenderWidgetHostView();
  const float device = view ? view->GetDeviceScaleFactor() : 1.f;
  // Blink legt den Browser-Zoom wie die Geräte-Skalierung in die Pixel.
  const double zoom = blink::ZoomLevelToZoomFactor(
      content::HostZoomMap::GetZoomLevel(web_contents()));
  return device * static_cast<float>(zoom);
}

void ReliefTabHelper::TakeUpdates(AXTreeMirror& tree,
                                  rust::Vec<bridge::TreeUpdate>& out) {
  const float scale = Scale();
  tree.SetOffset(Offset(tree.id(), scale));
  tree.TakeUpdate(out, hosts_, scale);
  TakeChildOffsets(tree, out, scale);
}

std::optional<gfx::Vector2dF> ReliefTabHelper::Offset(const std::string& tree,
                                                      float scale) const {
  auto host = hosts_.find(tree);
  if (host == hosts_.end()) {
    return std::nullopt;
  }
  auto parent = trees_.find(ui::AXTreeID::FromString(host->second.first));
  if (parent == trees_.end()) {
    return std::nullopt;
  }
  const std::optional<gfx::RectF> bounds =
      parent->second->NodePageBounds(host->second.second, scale);
  return bounds ? std::optional(bounds->OffsetFromOrigin()) : std::nullopt;
}

void ReliefTabHelper::TakeChildOffsets(const AXTreeMirror& parent,
                                       rust::Vec<bridge::TreeUpdate>& out,
                                       float scale) {
  // Erst sammeln: TakeUpdate trägt neue iframe-Knoten in hosts_ ein.
  std::vector<std::string> children;
  for (const auto& [child, host] : hosts_) {
    if (host.first == parent.id()) {
      children.push_back(child);
    }
  }
  for (const std::string& child : children) {
    auto it = trees_.find(ui::AXTreeID::FromString(child));
    if (it == trees_.end() || !it->second->SetOffset(Offset(child, scale))) {
      continue;
    }
    it->second->TakeUpdate(out, hosts_, scale);
    TakeChildOffsets(*it->second, out, scale);
  }
}

void ReliefTabHelper::DropTree(const ui::AXTreeID& tree_id) {
  auto it = trees_.find(tree_id);
  if (it == trees_.end()) {
    return;
  }
  const bool known = it->second->IsKnownToRuntime();
  const std::string id = it->second->id();
  trees_.erase(it);
  // Eintrag des Baums selbst und die seiner iframes (Kindbäume, die noch
  // nicht angekommen sind, blieben sonst stehen).
  hosts_.erase(id);
  std::erase_if(hosts_,
                [&id](const auto& host) { return host.second.first == id; });
  if (tree_id == root_) {
    root_ = ui::AXTreeID();
  }
  if (!known) {
    return;
  }
  bridge::Delta delta{};
  delta.removed_trees.push_back(rust::String::lossy(id));
  PacketTiming timing;
  timing.kind = "drop";
  timing.received = base::TimeTicks::Now();
  runtime_.AsyncCall(&RuntimeHost::Apply).WithArgs(std::move(delta), timing);
}

void ReliefTabHelper::Perform(bridge::ActionPlan plan) {
  // Der Plan wurde gegen einen Graph-Stand geprüft; bis hierher können
  // weitere Pakete angekommen sein. Maßgeblich ist, ob der Knoten im eigenen
  // Baum noch existiert (→ plan/spezifikation/02, „Datenfluss zurück“).
  const ui::AXTreeID tree_id =
      ui::AXTreeID::FromString(std::string(plan.tree));
  auto it = trees_.find(tree_id);
  if (it == trees_.end() || !it->second->HasNode(plan.node)) {
    runtime_.AsyncCall(&RuntimeHost::Log).WithArgs("activate\tstale");
    return;
  }
  ui::AXActionHandlerBase* handler =
      ui::AXActionHandlerRegistry::GetInstance()->GetActionHandler(tree_id);
  if (!handler) {
    runtime_.AsyncCall(&RuntimeHost::Log).WithArgs("activate\tno-handler");
    return;
  }
  ui::AXActionData data;
  data.action = ToAXAction(plan.action);
  data.target_tree_id = tree_id;
  data.target_node_id = plan.node;
  if (plan.has_value) {
    data.value = std::string(plan.value);
  }
  handler->PerformAction(data);
  runtime_.AsyncCall(&RuntimeHost::Log)
      .WithArgs(base::StringPrintf("activate\tsent\taction=%s\tnode=%d",
                                   ui::ToString(data.action), plan.node));
}

WEB_CONTENTS_USER_DATA_KEY_IMPL(ReliefTabHelper);

}  // namespace relief
