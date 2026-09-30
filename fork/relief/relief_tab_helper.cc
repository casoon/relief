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
#include "base/strings/string_util.h"
#include "base/strings/stringprintf.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/bind_post_task.h"
#include "base/task/sequenced_task_runner.h"
#include "base/task/thread_pool.h"
#include "base/time/time.h"
#include "components/input/native_web_keyboard_event.h"
#include "components/tabs/public/tab_interface.h"
#include "content/public/browser/browser_accessibility_state.h"
#include "content/public/browser/host_zoom_map.h"
#include "content/public/browser/navigation_handle.h"
#include "content/public/browser/page.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/render_process_host.h"
#include "content/public/browser/render_widget_host.h"
#include "content/public/browser/render_widget_host_view.h"
#include "content/public/browser/scoped_accessibility_mode.h"
#include "content/public/browser/web_contents.h"
#include "mojo/public/cpp/bindings/associated_remote.h"
#include "mojo/public/cpp/bindings/callback_helpers.h"
#include "relief/common/form_facts.mojom.h"
#include "third_party/blink/public/common/associated_interfaces/associated_interface_provider.h"
#include "relief/branding_strings.h"
#include "relief/inspector/relief_inspector.h"
#include "relief/relief_attach.h"
#include "relief/marks_overlay.h"
#include "relief/relief_executor.h"
#include "relief/relief_switches.h"
#include "relief/relief_task_runner.h"
#include "third_party/blink/public/common/page/page_zoom.h"
#include "ui/accessibility/ax_action_data.h"
#include "ui/accessibility/ax_action_handler_base.h"
#include "ui/accessibility/ax_enum_util.h"
#include "ui/accessibility/ax_enums.mojom.h"
#include "ui/accessibility/ax_location_and_scroll_updates.h"
#include "ui/accessibility/ax_mode.h"
#include "ui/accessibility/ax_updates_and_events.h"
#include "ui/events/keycodes/dom/dom_code.h"
#include "ui/events/keycodes/dom/dom_key.h"
#include "ui/events/keycodes/keyboard_codes.h"
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
    case bridge::Action::SetSequentialFocusNavigationStartingPoint:
      return ax::mojom::Action::kSetSequentialFocusNavigationStartingPoint;
  }
  // Die Runtime plant nur gültige Aktionen (plan_action lehnt unbekannte
  // Enum-Werte ab).
  NOTREACHED();
}

}  // namespace

base::CallbackListSubscription AttachToTab(tabs::TabInterface& tab) {
  // Produktname in übersetzten Texten, unabhängig von --enable-relief.
  ApplyBrandingStrings();
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
  const int tab = g_next_tab++;
  runtime_ = base::SequenceBound<RuntimeHost>(
      base::ThreadPool::CreateSequencedTaskRunner(
          {base::MayBlock(), base::TaskPriority::USER_VISIBLE}),
      tab, command_line.GetSwitchValuePath(switches::kReliefLog),
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
  executor_ = std::make_unique<ReliefExecutor>(*this, *contents);
  key_callback_ = base::BindRepeating(&ReliefTabHelper::OnKeyPress,
                                      base::Unretained(this));
  WatchKeys(contents->GetPrimaryMainFrame());
  open_inspector_ =
      tab == 1 && command_line.HasSwitch(switches::kReliefInspector);
  show_marks_on_load_ =
      tab == 1 && command_line.HasSwitch(switches::kReliefMarks);
  if (tab == 1 && command_line.HasSwitch(switches::kReliefRun)) {
    task_runner_ = std::make_unique<ReliefTaskRunner>(
        *this, *contents,
        command_line.GetSwitchValueNative(switches::kReliefRun));
  }
}

ReliefTabHelper::~ReliefTabHelper() {
  WatchKeys(nullptr);
  HideMarks();
  for (InspectorObserver& observer : observers_) {
    observer.OnTabHelperDestroyed();
  }
}

void ReliefTabHelper::AddObserver(InspectorObserver* observer) {
  observers_.AddObserver(observer);
}

void ReliefTabHelper::RemoveObserver(InspectorObserver* observer) {
  observers_.RemoveObserver(observer);
}

void ReliefTabHelper::NotifyGraphChanged() {
  for (InspectorObserver& observer : observers_) {
    observer.OnGraphChanged();
  }
  // Gezeigte Marken folgen der Seite (Scrollen, neue Elemente), gebündelt.
  if (marks_overlay_ && marks_overlay_->visible() &&
      !marks_refresh_.IsRunning()) {
    marks_refresh_.Start(FROM_HERE, base::Milliseconds(200),
                         base::BindOnce(&ReliefTabHelper::ShowMarks,
                                        base::Unretained(this)));
  }
}

void ReliefTabHelper::ShowMarks() {
  runtime_.AsyncCall(&RuntimeHost::ShowMarks)
      .Then(base::BindOnce(&ReliefTabHelper::OnMarks,
                           weak_factory_.GetWeakPtr()));
}

void ReliefTabHelper::OnMarks(rust::Vec<bridge::MarkBox> marks) {
  const std::optional<MainScroll> scroll = GetMainScroll();
  const float scale = Scale();
  const float zoom = static_cast<float>(blink::ZoomLevelToZoomFactor(
      content::HostZoomMap::GetZoomLevel(web_contents())));
  const float sx = scroll ? scroll->x / scale : 0;
  const float sy = scroll ? scroll->y / scale : 0;
  const gfx::Size viewport = web_contents()->GetContainerBounds().size();
  std::vector<ReliefMarksOverlay::Box> boxes;
  label_length_ = 0;
  for (const bridge::MarkBox& mark : marks) {
    label_length_ = mark.label.size();
    const gfx::Rect rect(static_cast<int>((mark.x - sx) * zoom),
                         static_cast<int>((mark.y - sy) * zoom),
                         static_cast<int>(mark.width * zoom),
                         static_cast<int>(mark.height * zoom));
    if (!rect.Intersects(gfx::Rect(viewport))) {
      continue;
    }
    boxes.push_back({base::UTF8ToUTF16(std::string(mark.label)), rect,
                     mark.uncertain});
  }
  if (!marks_overlay_) {
    marks_overlay_ = std::make_unique<ReliefMarksOverlay>(*web_contents());
  }
  runtime_.AsyncCall(&RuntimeHost::Log)
      .WithArgs(base::StringPrintf("marken\tgezeigt=%zu\tgesamt=%zu",
                                   boxes.size(), marks.size()));
  marks_overlay_->Show(std::move(boxes));
}

bool ReliefTabHelper::marks_visible_for_testing() const {
  return marks_overlay_ && marks_overlay_->visible();
}

void ReliefTabHelper::HideMarks() {
  marks_refresh_.Stop();
  typed_.clear();
  if (marks_overlay_) {
    marks_overlay_->Hide();
  }
}

bool ReliefTabHelper::OnMarksKey(const input::NativeWebKeyboardEvent& event) {
  if (!marks_overlay_ || !marks_overlay_->visible() ||
      event.GetType() != blink::WebInputEvent::Type::kRawKeyDown) {
    return false;
  }
  if (event.windows_key_code == ui::VKEY_ESCAPE) {
    HideMarks();
    return true;
  }
  // Buchstaben der Marken (Grundreihe), ohne Umschalttasten.
  constexpr std::string_view kLetters = "asdfghjkl";
  const char letter =
      static_cast<char>(base::ToLowerASCII(static_cast<char>(
          event.windows_key_code)));
  if ((event.GetModifiers() & blink::WebInputEvent::kKeyModifiers) != 0 ||
      kLetters.find(letter) == std::string_view::npos) {
    // Jede andere Taste beendet den Markenmodus und geht an die Seite.
    HideMarks();
    return false;
  }
  typed_.push_back(letter);
  if (typed_.size() >= label_length_) {
    std::string label = typed_;
    HideMarks();
    ChooseMark(std::move(label));
  }
  return true;
}

void ReliefTabHelper::ChooseMark(std::string label) {
  const std::string input = "marke " + label;
  Interact(input,
           base::BindOnce(
               [](base::WeakPtr<ReliefTabHelper> self, std::string input,
                  ReliefExecutor::Result result) {
                 if (!self) {
                   return;
                 }
                 // Rückfragen und Fehler gehören in die Leiste: dort lassen
                 // sie sich beantworten.
                 const bool needs_panel = !result.acted;
                 self->external_answer_ = std::pair(input, result);
                 for (InspectorObserver& observer : self->observers_) {
                   observer.OnExternalAnswer(input, result);
                   self->external_answer_.reset();
                 }
                 if (needs_panel) {
                   if (tabs::TabInterface* tab =
                           tabs::TabInterface::MaybeGetFromContents(
                               self->web_contents())) {
                     ShowInspector(*tab);
                   }
                 }
               },
               weak_factory_.GetWeakPtr(), input));
}

void ReliefTabHelper::WatchKeys(content::RenderFrameHost* frame) {
  content::RenderWidgetHost* widget =
      frame ? frame->GetRenderWidgetHost() : nullptr;
  content::RenderWidgetHost* old =
      keys_widget_ ? content::RenderWidgetHost::FromID(keys_widget_->first,
                                                       keys_widget_->second)
                   : nullptr;
  if (old == widget && widget) {
    return;
  }
  if (old) {
    old->RemoveKeyPressEventCallback(key_callback_);
  }
  keys_widget_.reset();
  if (widget) {
    widget->AddKeyPressEventCallback(key_callback_);
    keys_widget_ = std::pair(widget->GetProcess()->GetDeprecatedID(),
                             widget->GetRoutingID());
  }
}

bool ReliefTabHelper::OnKeyPress(const input::NativeWebKeyboardEvent& event) {
  // Strg+Umschalt+I (Inspector) und Strg+Umschalt+Leertaste
  // (Befehlsleiste), ohne weitere Umschalttasten (macOS: die
  // Entwicklertools liegen auf Befehl+Wahl+I).
  constexpr int kModifiers = blink::WebInputEvent::kControlKey |
                             blink::WebInputEvent::kShiftKey |
                             blink::WebInputEvent::kAltKey |
                             blink::WebInputEvent::kMetaKey;
  if (OnMarksKey(event)) {
    return true;
  }
  const bool command = event.windows_key_code == ui::VKEY_SPACE;
  const bool marks = event.windows_key_code == ui::VKEY_M;
  if (event.GetType() != blink::WebInputEvent::Type::kRawKeyDown ||
      (event.windows_key_code != ui::VKEY_I && !command && !marks) ||
      (event.GetModifiers() & kModifiers) !=
          (blink::WebInputEvent::kControlKey |
           blink::WebInputEvent::kShiftKey)) {
    return false;
  }
  if (marks) {
    runtime_.AsyncCall(&RuntimeHost::Log).WithArgs("marken\ttaste");
    if (marks_overlay_ && marks_overlay_->visible()) {
      HideMarks();
    } else {
      ShowMarks();
    }
    return true;
  }
  runtime_.AsyncCall(&RuntimeHost::Log)
      .WithArgs(command ? "leiste\ttaste" : "inspector\ttaste");
  tabs::TabInterface* tab =
      tabs::TabInterface::MaybeGetFromContents(web_contents());
  if (!tab) {
    return true;
  }
  if (!command) {
    ToggleInspector(*tab);
    return true;
  }
  // Leiste öffnen und fokussieren; ist das Panel noch nicht bereit, holt
  // es sich die Anforderung beim Start (TakeFocusCommandRequest).
  focus_command_requested_ = true;
  ShowInspector(*tab);
  for (InspectorObserver& observer : observers_) {
    observer.OnFocusCommandRequested();
  }
  return true;
}

void ReliefTabHelper::Interact(
    const std::string& input,
    base::OnceCallback<void(ReliefExecutor::Result)> done) {
  RunCommand(input, base::BindOnce(
                        [](base::WeakPtr<ReliefTabHelper> self,
                           base::OnceCallback<void(ReliefExecutor::Result)> done,
                           bridge::Reply reply) {
                          if (self) {
                            self->executor().Execute(std::move(reply),
                                                     std::move(done));
                          }
                        },
                        weak_factory_.GetWeakPtr(), std::move(done)));
}

void ReliefTabHelper::DidFinishLoad(content::RenderFrameHost* render_frame_host,
                                    const GURL& validated_url) {
  if (!render_frame_host->IsInPrimaryMainFrame()) {
    return;
  }
  if (show_marks_on_load_) {
    show_marks_on_load_ = false;
    // Der Baum kommt kurz nach dem Laden; dann zeigen.
    base::SequencedTaskRunner::GetCurrentDefault()->PostDelayedTask(
        FROM_HERE,
        base::BindOnce(&ReliefTabHelper::ShowMarks,
                       weak_factory_.GetWeakPtr()),
        base::Seconds(1));
  }
  if (!open_inspector_) {
    return;
  }
  // Beim Start kann das Laden fertig sein, bevor der Tab in einem Fenster
  // hängt oder das Fenster sein Side Panel angelegt hat (Show wird dann
  // übergangen): kurz danach öffnen.
  open_inspector_ = false;
  base::SequencedTaskRunner::GetCurrentDefault()->PostDelayedTask(
      FROM_HERE,
      base::BindOnce(
          [](base::WeakPtr<ReliefTabHelper> self) {
            if (!self) {
              return;
            }
            tabs::TabInterface* tab =
                tabs::TabInterface::MaybeGetFromContents(self->web_contents());
            const bool shown = tab && ShowInspector(*tab);
            self->runtime_.AsyncCall(&RuntimeHost::Log)
                .WithArgs(std::string("inspector\t") +
                          (shown ? "geöffnet" : "kein Fenster"));
          },
          weak_factory_.GetWeakPtr()),
      base::Seconds(1));
}

void ReliefTabHelper::InspectorJson(
    base::OnceCallback<void(std::string)> done) {
  runtime_.AsyncCall(&RuntimeHost::InspectorJson).Then(std::move(done));
}

void ReliefTabHelper::Show(const std::string& key,
                           base::OnceCallback<void(std::string)> done) {
  runtime_.AsyncCall(&RuntimeHost::ShowNode)
      .WithArgs(key)
      .Then(base::BindOnce(
          [](base::WeakPtr<ReliefTabHelper> self,
             base::OnceCallback<void(std::string)> done, bridge::Reply reply) {
            if (!self) {
              return;
            }
            self->executor().Execute(
                std::move(reply),
                base::BindOnce(
                    [](base::OnceCallback<void(std::string)> done,
                       ReliefExecutor::Result result) {
                      std::move(done).Run(std::move(result.text));
                    },
                    std::move(done)));
          },
          weak_factory_.GetWeakPtr(), std::move(done)));
}

void ReliefTabHelper::AccessibilityEventReceived(
    const ui::AXUpdatesAndEvents& details) {
  PacketTiming timing;
  timing.received = base::TimeTicks::Now();
  timing.updates = details.updates.size();
  last_packet_ = timing.received;

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
  NotifyGraphChanged();
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
  last_packet_ = timing.received;
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
  NotifyGraphChanged();
}

void ReliefTabHelper::PrimaryPageChanged(content::Page& page) {
  // Neues Hauptdokument, womöglich mit neuem Widget: Kürzel umhängen.
  WatchKeys(&page.GetMainDocument());
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
  NotifyGraphChanged();
}

void ReliefTabHelper::Perform(bridge::ActionPlan plan) {
  Send(std::string(plan.tree), plan.node, plan.action,
       plan.has_value ? std::optional(std::string(plan.value)) : std::nullopt);
}

bool ReliefTabHelper::PerformStep(const bridge::Step& step) {
  if (step.key != bridge::Key::None) {
    PressKey(step.key);
    return true;
  }
  return Send(std::string(step.tree), step.node, step.action,
              step.has_value ? std::optional(std::string(step.value))
                             : std::nullopt);
}

bool ReliefTabHelper::Send(const std::string& tree,
                           int32_t node,
                           bridge::Action action,
                           std::optional<std::string> value) {
  // Der Plan wurde gegen einen Graph-Stand geprüft; bis hierher können
  // weitere Pakete angekommen sein. Maßgeblich ist, ob der Knoten im eigenen
  // Baum noch existiert (→ plan/spezifikation/02, „Datenfluss zurück“).
  const ui::AXTreeID tree_id = ui::AXTreeID::FromString(tree);
  auto it = trees_.find(tree_id);
  if (it == trees_.end() || !it->second->HasNode(node)) {
    runtime_.AsyncCall(&RuntimeHost::Log).WithArgs("activate\tstale");
    return false;
  }
  ui::AXActionHandlerBase* handler =
      ui::AXActionHandlerRegistry::GetInstance()->GetActionHandler(tree_id);
  if (!handler) {
    runtime_.AsyncCall(&RuntimeHost::Log).WithArgs("activate\tno-handler");
    return false;
  }
  ui::AXActionData data;
  data.action = ToAXAction(action);
  data.target_tree_id = tree_id;
  data.target_node_id = node;
  if (value) {
    data.value = std::move(*value);
  }
  handler->PerformAction(data);
  runtime_.AsyncCall(&RuntimeHost::Log)
      .WithArgs(base::StringPrintf("activate\tsent\taction=%s\tnode=%d",
                                   ui::ToString(data.action), node));
  return true;
}

bool ReliefTabHelper::has_main_tree() const {
  return root_ != ui::AXTreeIDUnknown() &&
         root_ == web_contents()->GetPrimaryMainFrame()->GetAXTreeID();
}

// Formularziel und `autocomplete` trägt der AXTree nicht (Paket 75). Der
// Browser fragt sie beim Renderer an, nur solange eine Rückfrage offen ist
// oder entsteht: vor jeder Eingabe für das Ziel der offenen Rückfrage (damit
// „ja“ gegen den aktuellen Stand bindet) und nach einer Eingabe, die eine
// neue Rückfrage stellt; die stellt die Runtime dann mit den Angaben neu.
void ReliefTabHelper::RunCommand(
    const std::string& input,
    base::OnceCallback<void(bridge::Reply)> done) {
  RunWithFormFacts(
      base::BindOnce(
          [](base::WeakPtr<ReliefTabHelper> self, const std::string& input,
             std::optional<bridge::FormFacts> facts,
             base::OnceCallback<void(bridge::Reply)> replied) {
            self->runtime_.AsyncCall(&RuntimeHost::RunCommand)
                .WithArgs(input, std::move(facts))
                .Then(std::move(replied));
          },
          weak_factory_.GetWeakPtr(), input),
      std::move(done));
}

void ReliefTabHelper::ViewAct(const std::string& key,
                              const std::string& kind,
                              const std::string& value,
                              base::OnceCallback<void(bridge::Reply)> done) {
  RunWithFormFacts(
      base::BindOnce(
          [](base::WeakPtr<ReliefTabHelper> self, const std::string& key,
             const std::string& kind, const std::string& value,
             std::optional<bridge::FormFacts> facts,
             base::OnceCallback<void(bridge::Reply)> replied) {
            self->runtime_.AsyncCall(&RuntimeHost::ViewAct)
                .WithArgs(key, kind, value, std::move(facts))
                .Then(std::move(replied));
          },
          weak_factory_.GetWeakPtr(), key, kind, value),
      std::move(done));
}

void ReliefTabHelper::ViewInteract(
    const std::string& key,
    const std::string& kind,
    const std::string& value,
    base::OnceCallback<void(ReliefExecutor::Result)> done) {
  ViewAct(key, kind, value,
          base::BindOnce(
              [](base::WeakPtr<ReliefTabHelper> self,
                 base::OnceCallback<void(ReliefExecutor::Result)> done,
                 bridge::Reply reply) {
                if (self) {
                  self->executor().Execute(std::move(reply), std::move(done));
                }
              },
              weak_factory_.GetWeakPtr(), std::move(done)));
}

void ReliefTabHelper::RunWithFormFacts(
    Runner run,
    base::OnceCallback<void(bridge::Reply)> done) {
  runtime_.AsyncCall(&RuntimeHost::ConfirmationTarget)
      .Then(base::BindOnce(&ReliefTabHelper::OnTargetBeforeCommand,
                           weak_factory_.GetWeakPtr(), std::move(run),
                           std::move(done)));
}

void ReliefTabHelper::OnTargetBeforeCommand(
    Runner run,
    base::OnceCallback<void(bridge::Reply)> done,
    bridge::Found open) {
  auto go = base::BindOnce(
      [](base::WeakPtr<ReliefTabHelper> self, Runner run,
         base::OnceCallback<void(bridge::Reply)> done,
         std::optional<bridge::FormFacts> facts) {
        if (!self) {
          return;
        }
        std::optional<std::pair<std::string, int32_t>> asked;
        if (facts) {
          asked.emplace(std::string(facts->tree), facts->node);
        }
        std::move(run).Run(
            std::move(facts),
            base::BindOnce(&ReliefTabHelper::OnCommandReplied, self,
                           std::move(asked), std::move(done)));
      },
      weak_factory_.GetWeakPtr(), std::move(run), std::move(done));
  if (!open.found) {
    std::move(go).Run(std::nullopt);
    return;
  }
  FetchFormFacts(open, std::move(go));
}

void ReliefTabHelper::OnCommandReplied(
    std::optional<std::pair<std::string, int32_t>> asked,
    base::OnceCallback<void(bridge::Reply)> done,
    bridge::Reply reply) {
  runtime_.AsyncCall(&RuntimeHost::ConfirmationTarget)
      .Then(base::BindOnce(
          [](base::WeakPtr<ReliefTabHelper> self,
             std::optional<std::pair<std::string, int32_t>> asked,
             base::OnceCallback<void(bridge::Reply)> done, bridge::Reply reply,
             bridge::Found open) {
            if (!self) {
              return;
            }
            const bool fresh =
                !open.found ||
                (asked && asked->first == std::string(open.tree) &&
                 asked->second == open.node);
            if (fresh) {
              self->FinishCommandReply(std::move(done), std::move(reply));
              return;
            }
            self->FetchFormFacts(
                open,
                base::BindOnce(
                    [](base::WeakPtr<ReliefTabHelper> self,
                       base::OnceCallback<void(bridge::Reply)> done,
                       bridge::Reply reply,
                       std::optional<bridge::FormFacts> facts) {
                      if (!self) {
                        return;
                      }
                      if (!facts) {
                        self->FinishCommandReply(std::move(done),
                                                 std::move(reply));
                        return;
                      }
                      self->runtime_.AsyncCall(&RuntimeHost::Reconfirm)
                          .WithArgs(std::move(*facts))
                          .Then(base::BindOnce(
                              &ReliefTabHelper::FinishCommandReply, self,
                              std::move(done)));
                    },
                    self, std::move(done), std::move(reply)));
          },
          weak_factory_.GetWeakPtr(), std::move(asked), std::move(done),
          std::move(reply)));
}

void ReliefTabHelper::FinishCommandReply(
    base::OnceCallback<void(bridge::Reply)> done,
    bridge::Reply reply) {
  runtime_.AsyncCall(&RuntimeHost::LogSecurity);
  std::move(done).Run(std::move(reply));
}

void ReliefTabHelper::FetchFormFacts(
    const bridge::Found& target,
    base::OnceCallback<void(std::optional<bridge::FormFacts>)> done) {
  content::RenderFrameHost* frame = content::RenderFrameHost::FromAXTreeID(
      ui::AXTreeID::FromString(std::string(target.tree)));
  if (!frame || content::WebContents::FromRenderFrameHost(frame) !=
                    web_contents()) {
    std::move(done).Run(std::nullopt);
    return;
  }
  auto remote =
      std::make_unique<mojo::AssociatedRemote<relief::mojom::FormFacts>>();
  frame->GetRemoteAssociatedInterfaces()->GetInterface(remote.get());
  mojo::AssociatedRemote<relief::mojom::FormFacts>* raw = remote.get();
  // Ohne Antwort (Frame weg, Renderer ohne Agent) geht es ohne Angaben
  // weiter.
  (*raw)->Get(
      target.node,
      mojo::WrapCallbackWithDefaultInvokeIfNotRun(
          base::BindOnce(
              [](std::unique_ptr<
                     mojo::AssociatedRemote<relief::mojom::FormFacts>>,
                 std::string tree, int32_t node,
                 base::OnceCallback<void(std::optional<bridge::FormFacts>)>
                     done,
                 relief::mojom::FormInfoPtr info) {
                if (!info) {
                  std::move(done).Run(std::nullopt);
                  return;
                }
                bridge::FormFacts facts;
                facts.tree = tree;
                facts.node = node;
                facts.has_action = info->form_action.has_value();
                facts.action = info->form_action.value_or("");
                for (const relief::mojom::FieldFactPtr& field : info->fields) {
                  facts.fields.push_back(
                      bridge::FieldFact{field->ax_id, field->autocomplete});
                }
                std::move(done).Run(std::move(facts));
              },
              std::move(remote), std::string(target.tree), target.node,
              std::move(done)),
          relief::mojom::FormInfoPtr()));
}

void ReliefTabHelper::DevToolsCommand(
    const std::string& method,
    const std::string& params,
    base::OnceCallback<void(bridge::DevToolsReply)> done) {
  runtime_.AsyncCall(&RuntimeHost::DevToolsCommand)
      .WithArgs(method, params)
      .Then(std::move(done));
}

void ReliefTabHelper::FinishCommand(
    base::OnceCallback<void(std::string)> done) {
  runtime_.AsyncCall(&RuntimeHost::FinishCommand).Then(std::move(done));
}

void ReliefTabHelper::DescribePage(base::OnceCallback<void(std::string)> done) {
  runtime_.AsyncCall(&RuntimeHost::DescribePage).Then(std::move(done));
}

void ReliefTabHelper::CountNodes(base::OnceCallback<void(uint64_t)> done) {
  runtime_.AsyncCall(&RuntimeHost::NodeCount).Then(std::move(done));
}

void ReliefTabHelper::PressKey(bridge::Key key) {
  ui::KeyboardCode code = ui::VKEY_ESCAPE;
  ui::DomKey dom_key = ui::DomKey::ESCAPE;
  ui::DomCode dom_code = ui::DomCode::ESCAPE;
  const char* name = "Escape";
  switch (key) {
    case bridge::Key::None:
    case bridge::Key::Escape:
      break;
    case bridge::Key::ArrowUp:
      code = ui::VKEY_UP;
      dom_key = ui::DomKey::ARROW_UP;
      dom_code = ui::DomCode::ARROW_UP;
      name = "ArrowUp";
      break;
    case bridge::Key::ArrowDown:
      code = ui::VKEY_DOWN;
      dom_key = ui::DomKey::ARROW_DOWN;
      dom_code = ui::DomCode::ARROW_DOWN;
      name = "ArrowDown";
      break;
  }
  content::RenderFrameHost* frame = web_contents()->GetFocusedFrame();
  if (!frame) {
    frame = web_contents()->GetPrimaryMainFrame();
  }
  content::RenderWidgetHost* widget = frame->GetRenderWidgetHost();
  for (const blink::WebInputEvent::Type type :
       {blink::WebInputEvent::Type::kRawKeyDown,
        blink::WebInputEvent::Type::kKeyUp}) {
    input::NativeWebKeyboardEvent event(
        type, blink::WebInputEvent::kNoModifiers, base::TimeTicks::Now());
    event.windows_key_code = code;
    event.native_key_code = code;
    event.dom_key = dom_key;
    event.dom_code = static_cast<int>(dom_code);
    widget->ForwardKeyboardEvent(event);
  }
  runtime_.AsyncCall(&RuntimeHost::Log).WithArgs(std::string("key\t") + name);
}

std::optional<ReliefTabHelper::MainScroll> ReliefTabHelper::GetMainScroll()
    const {
  auto it = trees_.find(root_);
  content::RenderWidgetHostView* view =
      web_contents()->GetRenderWidgetHostView();
  if (it == trees_.end() || !view) {
    return std::nullopt;
  }
  const std::optional<AXTreeMirror::Scroll> scroll = it->second->RootScroll();
  if (!scroll) {
    return std::nullopt;
  }
  return MainScroll{scroll->x, scroll->y, scroll->y_max,
                    static_cast<int>(view->GetVisibleViewportSize().height() *
                                     Scale() * 0.8f)};
}

bool ReliefTabHelper::ScrollMainTo(int y) {
  auto it = trees_.find(root_);
  if (it == trees_.end()) {
    return false;
  }
  const std::optional<AXTreeMirror::Scroll> scroll = it->second->RootScroll();
  ui::AXActionHandlerBase* handler =
      ui::AXActionHandlerRegistry::GetInstance()->GetActionHandler(root_);
  if (!scroll || !handler) {
    return false;
  }
  ui::AXActionData data;
  data.action = ax::mojom::Action::kSetScrollOffset;
  data.target_tree_id = root_;
  data.target_node_id = scroll->node;
  data.target_point = gfx::Point(0, y);
  handler->PerformAction(data);
  runtime_.AsyncCall(&RuntimeHost::Log)
      .WithArgs(base::StringPrintf("scroll\tsent\ty=%d", y));
  return true;
}

WEB_CONTENTS_USER_DATA_KEY_IMPL(ReliefTabHelper);

}  // namespace relief
