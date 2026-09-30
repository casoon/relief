// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_RELIEF_TAB_HELPER_H_
#define RELIEF_RELIEF_TAB_HELPER_H_

#include <map>
#include <memory>
#include <optional>
#include <string>
#include <string_view>

#include "base/memory/weak_ptr.h"
#include "base/scoped_observation.h"
#include "base/threading/sequence_bound.h"
#include "base/time/time.h"
#include "base/timer/timer.h"
#include "content/public/browser/web_contents_observer.h"
#include "content/public/browser/web_contents_user_data.h"
#include "relief/bridge/ax_tree_mirror.h"
#include "relief/bridge/runtime_host.h"
#include "ui/accessibility/ax_action_handler_registry.h"
#include "ui/accessibility/ax_tree_id.h"

namespace content {
class ScopedAccessibilityMode;
}  // namespace content

namespace relief {

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
  void DidFinishNavigation(
      content::NavigationHandle* navigation_handle) override;

  // ui::AXActionHandlerObserver:
  void TreeRemoved(ui::AXTreeID tree_id) override;

  // Für Browser-Tests (//relief/testing).
  void SetDeltaObserverForTesting(RuntimeHost::DeltaCallback callback);
  void SetResetIntervalForTesting(base::TimeDelta interval) {
    reset_interval_ = interval;
  }
  int resets_for_testing() const { return resets_; }
  size_t trees_for_testing() const { return trees_.size(); }
  size_t hosts_for_testing() const { return hosts_.size(); }

 private:
  friend class content::WebContentsUserData<ReliefTabHelper>;
  explicit ReliefTabHelper(content::WebContents* contents);

  // Schickt den Plan als AXActionData; UI-Thread.
  void Perform(bridge::ActionPlan plan);
  // Entfernt einen Baum hier und in der Runtime.
  void DropTree(const ui::AXTreeID& tree_id);
  // Lässt alle Frames neu serialisieren, höchstens einmal je
  // `reset_interval_`; eine Anfrage innerhalb der Spanne wird bis zu ihrem
  // Ende aufgeschoben (eine offene genügt).
  void RequestReset(std::string_view reason);
  void Reset(std::string_view reason);
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
  base::ScopedObservation<ui::AXActionHandlerRegistry,
                          ui::AXActionHandlerObserver>
      registry_observation_{this};
  base::WeakPtrFactory<ReliefTabHelper> weak_factory_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

}  // namespace relief

#endif  // RELIEF_RELIEF_TAB_HELPER_H_
