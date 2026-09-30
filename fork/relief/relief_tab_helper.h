// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_RELIEF_TAB_HELPER_H_
#define RELIEF_RELIEF_TAB_HELPER_H_

#include <map>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <utility>

#include "base/functional/callback.h"
#include "base/memory/weak_ptr.h"
#include "base/observer_list.h"
#include "base/observer_list_types.h"
#include "base/scoped_observation.h"
#include "base/threading/sequence_bound.h"
#include "base/time/time.h"
#include "base/timer/timer.h"
#include "content/public/browser/render_widget_host.h"
#include "content/public/browser/web_contents_observer.h"
#include "content/public/browser/web_contents_user_data.h"
#include "relief/bridge/ax_tree_mirror.h"
#include "relief/bridge/runtime_host.h"
#include "relief/relief_executor.h"
#include "ui/accessibility/ax_action_handler_registry.h"
#include "ui/accessibility/ax_tree_id.h"

namespace content {
class ScopedAccessibilityMode;
}  // namespace content

namespace relief {

class ReliefMarksOverlay;
class ReliefTaskRunner;

// Relief je Tab, erzeugt von AttachToTab (relief_attach.h). Weg B (→
// plan/spezifikation/01): fordert den AXMode per ScopedAccessibilityMode an,
// baut aus AccessibilityEventReceived je Frame einen eigenen ui::AXTree
// (Positionen aus AccessibilityLocationChangesReceived), übersetzt die
// Änderungen in eine Delta und gibt sie an die Rust-Runtime auf deren
// Sequenz. Rückweg: geprüfte ActionPlans als AXActionData an den Frame des
// Zielknotens.
//
// Lebenszyklus der Bäume: Die Runtime hält nur Bäume von Frames der
// aktuellen Seite. Ein Baum fällt weg über TreeRemoved (Frame gelöscht oder
// neues Dokument im selben Frame) und beim Seitenwechsel (PrimaryPageChanged;
// Seiten im Back-Forward-Cache melden kein TreeRemoved). Pakete inaktiver
// Frames (Back-Forward-Cache) werden übergangen.
class ReliefTabHelper
    : public content::WebContentsObserver,
      public content::WebContentsUserData<ReliefTabHelper>,
      public ui::AXActionHandlerObserver {
 public:
  ReliefTabHelper(const ReliefTabHelper&) = delete;
  ReliefTabHelper& operator=(const ReliefTabHelper&) = delete;
  ~ReliefTabHelper() override;

  // content::WebContentsObserver:
  void AccessibilityEventReceived(
      const ui::AXUpdatesAndEvents& details) override;
  void AccessibilityLocationChangesReceived(
      const ui::AXTreeID& tree_id,
      ui::AXLocationAndScrollUpdates& details) override;
  void PrimaryPageChanged(content::Page& page) override;
  void DidFinishLoad(content::RenderFrameHost* render_frame_host,
                     const GURL& validated_url) override;
  void DidFinishNavigation(
      content::NavigationHandle* navigation_handle) override;

  // ui::AXActionHandlerObserver:
  void TreeRemoved(ui::AXTreeID tree_id) override;

  // Für den Inspector (inspector/): Benachrichtigung auf dem UI-Thread,
  // sobald eine Delta an die Runtime ging.
  class InspectorObserver : public base::CheckedObserver {
   public:
    virtual void OnGraphChanged() = 0;
    virtual void OnTabHelperDestroyed() = 0;
    // Die Befehlsleiste soll den Fokus bekommen (Strg+Umschalt+Leertaste).
    virtual void OnFocusCommandRequested() {}
    // Eine Eingabe außerhalb der Leiste (Sprungmarke) wurde beantwortet.
    virtual void OnExternalAnswer(const std::string& input,
                                  const ReliefExecutor::Result& result) {}
  };
  void AddObserver(InspectorObserver* observer);
  void RemoveObserver(InspectorObserver* observer);
  // Graph als JSON (crates/relief-bridge/src/inspector.rs).
  void InspectorJson(base::OnceCallback<void(std::string)> done);
  // Eingabe der Befehlsleiste: an die Runtime, Antwort ausführen, Wirkung
  // abwarten; `done` bekommt Antwort und ob auf der Seite gehandelt wurde.
  void Interact(const std::string& input,
                base::OnceCallback<void(ReliefExecutor::Result)> done);
  // Antwort einer Sprungmarke, die das Panel beim Start noch nicht sah.
  std::optional<std::pair<std::string, ReliefExecutor::Result>>
  TakeExternalAnswer() {
    return std::exchange(external_answer_, std::nullopt);
  }
  // Wurde die Befehlsleiste angefordert, bevor ihr Panel bereit war?
  bool TakeFocusCommandRequest() {
    return std::exchange(focus_command_requested_, false);
  }
  // Eintrag im Dokument zeigen (Fokus bzw. Hinbewegen, nichts auslösen);
  // `done` bekommt die Antwort nach kurzer Ruhe.
  void Show(const std::string& key,
            base::OnceCallback<void(std::string)> done);

  // Für den Aufgaben-Runner (relief_task_runner.h). Antworten der Runtime
  // kommen auf dem UI-Thread an, nach allen bis zum Aufruf gesendeten
  // Deltas.
  void RunCommand(const std::string& input,
                  base::OnceCallback<void(bridge::Reply)> done);
  void FinishCommand(base::OnceCallback<void(std::string)> done);
  void DescribePage(base::OnceCallback<void(std::string)> done);
  void CountNodes(base::OnceCallback<void(uint64_t)> done);
  // Ein Schritt: Aktion als AXActionData an den Frame des Knotens (false,
  // wenn der Knoten nicht mehr im eigenen Baum steht oder der Frame keinen
  // Handler hat) oder eine Taste.
  bool PerformStep(const bridge::Step& step);
  // Taste als echtes Tastaturereignis an das fokussierte Widget:
  // AXActionData kennt keine Tasten (Ersatzweg, → spezifikation/05).
  void PressKey(bridge::Key key);
  // Scroll-Position des Hauptdokuments in Blink-Pixeln und die Höhe einer
  // Bildschirmseite (80 % des Viewports, wie im CDP-Host).
  struct MainScroll {
    int x = 0;
    int y = 0;
    int y_max = 0;
    int page = 0;
  };
  std::optional<MainScroll> GetMainScroll() const;
  bool ScrollMainTo(int y);
  // Führt Antworten der Runtime aus (Schritte, Ruhe, Antwort); eine je Tab.
  ReliefExecutor& executor() { return *executor_; }
  // Letztes AX-Paket (Baum oder Positionen); Ruhe = keines seit einer Weile.
  base::TimeTicks last_packet() const { return last_packet_; }
  // Der Baum des aktuellen Hauptdokuments ist in der Runtime.
  bool has_main_tree() const;

  // Für Browser-Tests (//relief/testing).
  void SetDeltaObserverForTesting(RuntimeHost::DeltaCallback callback);
  void SetResetIntervalForTesting(base::TimeDelta interval) {
    reset_interval_ = interval;
  }
  int resets_for_testing() const { return resets_; }
  bool marks_visible_for_testing() const;
  size_t trees_for_testing() const { return trees_.size(); }
  size_t hosts_for_testing() const { return hosts_.size(); }

 private:
  friend class content::WebContentsUserData<ReliefTabHelper>;
  explicit ReliefTabHelper(content::WebContents* contents);

  // Schickt den Plan als AXActionData; UI-Thread (--relief-activate).
  void Perform(bridge::ActionPlan plan);
  bool Send(const std::string& tree,
            int32_t node,
            bridge::Action action,
            std::optional<std::string> value);
  // Entfernt einen Baum hier und in der Runtime.
  void DropTree(const ui::AXTreeID& tree_id);
  // Lässt alle Frames neu serialisieren, höchstens einmal je
  // `reset_interval_`; eine Anfrage innerhalb der Spanne wird bis zu ihrem
  // Ende aufgeschoben (eine offene genügt).
  void RequestReset(std::string_view reason);
  void Reset(std::string_view reason);
  // Tastenkürzel für den Inspector am Widget des Hauptframes (vor der
  // Seite, → inspector/relief_inspector.h).
  void WatchKeys(content::RenderFrameHost* frame);
  bool OnKeyPress(const input::NativeWebKeyboardEvent& event);
  // Sprungmarken (Paket 38): Strg+Umschalt+M zeigt sie, Buchstaben wählen.
  void ShowMarks();
  void OnMarks(rust::Vec<bridge::MarkBox> marks);
  void HideMarks();
  bool OnMarksKey(const input::NativeWebKeyboardEvent& event);
  void ChooseMark(std::string label);
  void NotifyGraphChanged();
  // Blink-Pixel je CSS-Pixel für die Positionen: Geräte-Skalierung ×
  // Browser-Zoom.
  float Scale() const;
  // Holt die Änderungen eines Baums ab, mit Offset, wenn er ein iframe ist,
  // und reicht einen neuen Offset an seine iframe-Bäume weiter (ganzer Baum
  // als BoundsChange), rekursiv.
  void TakeUpdates(AXTreeMirror& tree, rust::Vec<bridge::TreeUpdate>& out);
  void TakeChildOffsets(const AXTreeMirror& parent,
                        rust::Vec<bridge::TreeUpdate>& out,
                        float scale);
  // Position des Viewports eines iframe-Baums im Hauptdokument: Position
  // seines Host-Knotens im Elternbaum; nullopt, solange einer fehlt.
  std::optional<gfx::Vector2dF> Offset(const std::string& tree,
                                       float scale) const;

  std::unique_ptr<content::ScopedAccessibilityMode> mode_;
  std::map<ui::AXTreeID, std::unique_ptr<AXTreeMirror>> trees_;
  ChildTreeHosts hosts_;
  // Hauptbaum, wie zuletzt an die Runtime gemeldet.
  ui::AXTreeID root_;
  base::SequenceBound<RuntimeHost> runtime_;
  base::TimeDelta reset_interval_;
  base::TimeTicks last_reset_;
  base::OneShotTimer reset_timer_;
  int resets_ = 0;
  base::TimeTicks last_packet_;
  std::unique_ptr<ReliefExecutor> executor_;
  // --relief-run, nur im ersten Tab.
  std::unique_ptr<ReliefTaskRunner> task_runner_;
  base::ObserverList<InspectorObserver> observers_;
  // Widget (Prozess, Routing-ID), an dem das Tastenkürzel hängt; ein
  // Widget kann einen Frame überleben.
  std::optional<std::pair<int32_t, int32_t>> keys_widget_;
  content::RenderWidgetHost::KeyPressEventCallback key_callback_;
  // --relief-inspector: nach dem ersten Laden öffnen (einmal).
  bool open_inspector_ = false;
  bool focus_command_requested_ = false;
  // --relief-marks: nach dem ersten Laden zeigen (einmal).
  bool show_marks_on_load_ = false;
  std::unique_ptr<ReliefMarksOverlay> marks_overlay_;
  // Getippte Buchstaben und Länge der Marken des gezeigten Stands.
  std::string typed_;
  size_t label_length_ = 0;
  base::OneShotTimer marks_refresh_;
  std::optional<std::pair<std::string, ReliefExecutor::Result>>
      external_answer_;
  base::ScopedObservation<ui::AXActionHandlerRegistry,
                          ui::AXActionHandlerObserver>
      registry_observation_{this};
  base::WeakPtrFactory<ReliefTabHelper> weak_factory_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

}  // namespace relief

#endif  // RELIEF_RELIEF_TAB_HELPER_H_
