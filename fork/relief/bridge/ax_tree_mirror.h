// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_BRIDGE_AX_TREE_MIRROR_H_
#define RELIEF_BRIDGE_AX_TREE_MIRROR_H_

#include <map>
#include <optional>
#include <string>
#include <vector>

#include "relief/crates/relief-bridge/src/cxx_bridge.rs.h"
#include "third_party/abseil-cpp/absl/container/flat_hash_map.h"
#include "third_party/abseil-cpp/absl/container/flat_hash_set.h"
#include "ui/accessibility/ax_node_id_forward.h"
#include "ui/accessibility/ax_tree.h"
#include "ui/accessibility/ax_tree_observer.h"
#include "ui/gfx/geometry/rect_f.h"
#include "ui/gfx/geometry/vector2d_f.h"

namespace ui {
struct AXLocationAndScrollUpdates;
struct AXTreeUpdate;
struct AXUpdatesAndEvents;
}  // namespace ui

namespace relief {

// iframe-Knoten, in dem ein Child-Tree hängt: Tree-ID (Text) → (Elternbaum,
// Knoten). Füllt TreeData::parent im Modell.
using ChildTreeHosts = std::map<std::string, std::pair<std::string, int32_t>>;

// Eigener Accessibility-Baum eines Frames und Dokuments (Weg B, →
// plan/spezifikation/01, Abschnitt 2): befüllt aus dem Datenstrom von
// WebContentsObserver::AccessibilityEventReceived, unabhängig vom
// BrowserAccessibilityManager (der nur mit kNativeAPIs existiert).
//
// Merkt sich über AXTreeObserver, welche Knoten seit der letzten Delta
// angelegt, geändert oder gelöscht wurden, und fasst das als
// relief::bridge::TreeUpdate zusammen. Geänderte Knoten gehen vollständig
// hinüber (wie im Modell).
//
// Positionen: Seitenkoordinaten des Hauptdokuments in CSS-Pixeln, aus
// relative_bounds über die Offset-Container (ui::AXTree::RelativeToTreeBounds,
// ungeclippt), geteilt durch `scale` (Blink-Pixel je CSS-Pixel). Im
// Hauptdokument kommt die Scroll-Position des Root-Scrollers dazu (Seite
// statt Viewport). Ein iframe-Baum rechnet in seinem Viewport und verschiebt
// um `offset` = Position des Host-Knotens im Elternbaum (SetOffset); ohne
// Offset hat er keine Positionen. Ändert sich die Position oder
// Scroll-Position eines Knotens, werden er und sein Teilbaum neu gerechnet,
// bei neuem Offset der ganze Baum; Knoten, deren Position sich dabei ändert,
// gehen als BoundsChange hinüber. Läuft auf dem UI-Thread.
class AXTreeMirror : public ui::AXTreeObserver {
 public:
  // `child`: Baum eines iframes (Positionen erst mit SetOffset).
  AXTreeMirror(std::string id, bool child);
  AXTreeMirror(const AXTreeMirror&) = delete;
  AXTreeMirror& operator=(const AXTreeMirror&) = delete;
  ~AXTreeMirror() override;

  // Kann der Baum das Paket anwenden? Ohne Wurzel muss das erste Update die
  // Wurzel samt Knoten mitbringen. Sonst schlägt Unserialize fehl, und das
  // bricht in Builds mit DCHECK oder ohne is_official_build den Browser ab
  // (ui/accessibility/ax_common.h, AXTree::RecordError) — auch bei einem
  // Update nur mit Baumdaten („Tree has no root“). Typischer Fall: Paket
  // eines Frames, dessen Baum Relief verworfen hat (Rückkehr aus dem
  // Back-Forward-Cache).
  enum class Start {
    kReady,
    // Erstes Update nur mit Baumdaten (z. B. Fokuswechsel beim Commit).
    kDataOnly,
    // Erstes Update mit Knoten, aber ohne Wurzel (inkrementell).
    kNoRoot,
  };
  Start CheckStart(const ui::AXUpdatesAndEvents& details) const;

  // Wendet ein AXTreeUpdate an; bei false steht der Grund in error().
  bool Unserialize(const ui::AXTreeUpdate& update);
  const std::string& error() const { return tree_.error(); }

  // Übernimmt Positionen und Scroll-Positionen aus
  // AccessibilityLocationChangesReceived in den eigenen Baum. Unbekannte
  // Knoten werden übergangen (wie im BrowserAccessibilityManager).
  void ApplyLocationChanges(const ui::AXLocationAndScrollUpdates& changes);

  // Hängt die seit dem letzten Aufruf gesammelten Änderungen als TreeUpdate
  // an `out` an; nichts, wenn sich nichts geändert hat. `hosts` liefert den
  // iframe-Knoten im Elternbaum, neue Child-Tree-Verweise trägt die Funktion
  // dort ein. `scale`: Blink-Pixel je CSS-Pixel (Geräte-Skalierung ×
  // Browser-Zoom).
  void TakeUpdate(rust::Vec<bridge::TreeUpdate>& out,
                  ChildTreeHosts& hosts,
                  float scale);

  // Position des Viewports dieses iframe-Baums im Hauptdokument in
  // CSS-Pixeln (nullopt: unbekannt). Bei Änderung wird der ganze Baum in der
  // nächsten TakeUpdate neu gerechnet. Gibt zurück, ob sich etwas änderte.
  bool SetOffset(std::optional<gfx::Vector2dF> offset);

  // Seitenkoordinaten eines Knotens (Host-Knoten eines iframes); nullopt,
  // wenn der Knoten fehlt oder keine Position hat.
  std::optional<gfx::RectF> NodePageBounds(ui::AXNodeID id, float scale) const;

  // Scroll-Position des Root-Scrollers in Blink-Pixeln (Hauptdokument;
  // nullopt ohne Root-Scroller).
  struct Scroll {
    ui::AXNodeID node;
    int x;
    int y;
    int y_max;
  };
  std::optional<Scroll> RootScroll() const;

  // Die Runtime hat den Baum (mindestens eine Delta ging hinüber); nur dann
  // meldet das Entfernen ihn dort als removed_trees.
  bool IsKnownToRuntime() const { return sent_once_; }
  bool HasNode(ui::AXNodeID id) const { return tree_.GetFromId(id); }
  const std::string& id() const { return id_; }

 private:
  // Seitenkoordinaten in CSS-Pixeln; nullopt bei leerer Fläche oder
  // fehlendem Offset eines iframe-Baums.
  std::optional<gfx::RectF> PageBounds(const ui::AXNode& node,
                                       float scale) const;
  // Position eines Knotens, den die Runtime hat, neu rechnen; bei Änderung
  // an `update.bounds` anhängen.
  void CollectBounds(const ui::AXNode& root,
                     float scale,
                     bridge::TreeUpdate& update);

  // ui::AXTreeObserver:
  void OnNodeDataWillChange(ui::AXTree* tree,
                            const ui::AXNodeData& old_node_data,
                            const ui::AXNodeData& new_node_data) override;
  void OnNodeWillBeDeleted(ui::AXTree* tree, ui::AXNode* node) override;
  void OnTreeDataChanged(ui::AXTree* tree,
                         const ui::AXTreeData& old_data,
                         const ui::AXTreeData& new_data) override;
  void OnAtomicUpdateFinished(ui::AXTree* tree,
                              bool root_changed,
                              const std::vector<Change>& changes) override;

  // Scrollt der Root-Scroller, bleiben Seitenkoordinaten des
  // Hauptdokuments gleich; in einem iframe verschiebt er dessen Inhalt.
  bool IsRootScroller(ui::AXNodeID id) const {
    return !child_ && id == tree_.data().root_scroller_id;
  }

  const std::string id_;
  const bool child_;
  std::optional<gfx::Vector2dF> offset_;
  ui::AXTree tree_;
  // Knoten, die die Runtime hat (Stand der zuletzt abgegebenen Delta).
  absl::flat_hash_set<ui::AXNodeID> known_;
  // Seit der letzten Delta angelegt oder geändert bzw. gelöscht.
  absl::flat_hash_set<ui::AXNodeID> touched_;
  absl::flat_hash_set<ui::AXNodeID> deleted_;
  // Knoten, deren eigene Position oder Scroll-Position sich geändert hat;
  // ihr Teilbaum wird in TakeUpdate neu gerechnet.
  absl::flat_hash_set<ui::AXNodeID> moved_;
  // Zuletzt an die Runtime gegebene Position je Knoten (fehlt = keine).
  absl::flat_hash_map<ui::AXNodeID, gfx::RectF> sent_bounds_;
  std::optional<ui::AXNodeID> sent_root_;
  bool data_dirty_ = true;
  bool sent_once_ = false;
};

// Relief-Rollenname: ARIA-Name, wo es einen gibt, sonst Chromiums interner
// Name in lowerCamelCase (→ crates/relief-model/src/role.rs).
std::string_view ReliefRoleName(ax::mojom::Role role);

}  // namespace relief

#endif  // RELIEF_BRIDGE_AX_TREE_MIRROR_H_
