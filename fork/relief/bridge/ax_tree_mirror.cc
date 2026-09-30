// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/bridge/ax_tree_mirror.h"

#include <algorithm>
#include <utility>

#include "base/strings/string_number_conversions.h"
#include "ui/accessibility/ax_enum_util.h"
#include "ui/accessibility/ax_enums.mojom.h"
#include "ui/accessibility/ax_location_and_scroll_updates.h"
#include "ui/accessibility/ax_node.h"
#include "ui/accessibility/ax_node_data.h"
#include "ui/accessibility/ax_tree_data.h"
#include "ui/accessibility/ax_tree_id.h"
#include "ui/accessibility/ax_tree_update.h"
#include "ui/accessibility/ax_updates_and_events.h"

namespace relief {

namespace {

using ax::mojom::BoolAttribute;
using ax::mojom::FloatAttribute;
using ax::mojom::IntAttribute;
using ax::mojom::IntListAttribute;
using ax::mojom::State;
using ax::mojom::StringAttribute;

// AX-Texte kommen aus Blink als UTF-8, können aber ungepaarte Surrogate
// enthalten. rust::String(std::string) bricht dann ab (Chromium baut ohne
// Ausnahmen); lossy ersetzt ungültige Folgen durch U+FFFD.
rust::String Text(const std::string& text) {
  return rust::String::lossy(text);
}

rust::Vec<int32_t> Ids(const std::vector<int32_t>& ids) {
  rust::Vec<int32_t> out;
  out.reserve(ids.size());
  for (int32_t id : ids) {
    out.push_back(id);
  }
  return out;
}

void OptionalText(const ui::AXNodeData& data,
                  StringAttribute attribute,
                  bool& has,
                  rust::String& value) {
  if (data.HasStringAttribute(attribute)) {
    has = true;
    value = Text(data.GetStringAttribute(attribute));
  }
}

bridge::NameFrom ToNameFrom(ax::mojom::NameFrom from) {
  switch (from) {
    case ax::mojom::NameFrom::kAttribute:
    case ax::mojom::NameFrom::kAttributeExplicitlyEmpty:
      return bridge::NameFrom::Attribute;
    case ax::mojom::NameFrom::kRelatedElement:
      return bridge::NameFrom::RelatedElement;
    case ax::mojom::NameFrom::kContents:
      return bridge::NameFrom::Contents;
    case ax::mojom::NameFrom::kPlaceholder:
      return bridge::NameFrom::Placeholder;
    case ax::mojom::NameFrom::kTitle:
      return bridge::NameFrom::Title;
    default:
      return bridge::NameFrom::Unset;
  }
}

bridge::Toggle ToToggle(ax::mojom::CheckedState state) {
  switch (state) {
    case ax::mojom::CheckedState::kFalse:
      return bridge::Toggle::False;
    case ax::mojom::CheckedState::kTrue:
      return bridge::Toggle::True;
    case ax::mojom::CheckedState::kMixed:
      return bridge::Toggle::Mixed;
    case ax::mojom::CheckedState::kNone:
      return bridge::Toggle::Unset;
  }
}

bridge::HasPopup ToHasPopup(ax::mojom::HasPopup popup) {
  switch (popup) {
    case ax::mojom::HasPopup::kFalse:
      return bridge::HasPopup::Unset;
    case ax::mojom::HasPopup::kTrue:
      return bridge::HasPopup::True;
    case ax::mojom::HasPopup::kMenu:
      return bridge::HasPopup::Menu;
    case ax::mojom::HasPopup::kListbox:
      return bridge::HasPopup::Listbox;
    case ax::mojom::HasPopup::kTree:
      return bridge::HasPopup::Tree;
    case ax::mojom::HasPopup::kGrid:
      return bridge::HasPopup::Grid;
    case ax::mojom::HasPopup::kDialog:
      return bridge::HasPopup::Dialog;
  }
}

bridge::Rect ToRect(const gfx::RectF& rect) {
  return bridge::Rect{rect.x(), rect.y(), rect.width(), rect.height()};
}

// AXNodeData → Grenzstruktur. `bounds`: Seitenkoordinaten, von
// AXTreeMirror::PageBounds gerechnet (relative_bounds allein sind relativ
// zum Offset-Container).
bridge::Node ToNode(const ui::AXNode& node,
                    const std::optional<gfx::RectF>& bounds) {
  const ui::AXNodeData& data = node.data();
  bridge::Node out{};
  out.id = data.id;
  if (bounds) {
    out.has_bounds = true;
    out.bounds = ToRect(*bounds);
  }
  const std::string_view role = ReliefRoleName(data.role);
  out.role = rust::String(role.data(), role.size());

  OptionalText(data, StringAttribute::kName, out.has_name, out.name);
  out.name_from = ToNameFrom(data.GetNameFrom());
  OptionalText(data, StringAttribute::kDescription, out.has_description,
               out.description);
  OptionalText(data, StringAttribute::kValue, out.has_value, out.value);
  // Wertebereich wie im CDP-Konverter: Wert = aktueller Zahlenwert, Grenzen
  // und ein abweichender Text (aria-valuetext, bei Chromium in kValue) als
  // Zusatz.
  if (data.HasFloatAttribute(FloatAttribute::kValueForRange)) {
    const std::string number = base::NumberToString(
        data.GetFloatAttribute(FloatAttribute::kValueForRange));
    if (out.has_value && std::string(out.value) != number) {
      out.extra.push_back(bridge::Attribute{"valuetext", out.value});
    }
    out.has_value = true;
    out.value = number;
    for (const auto& [attribute, key] :
         {std::pair(FloatAttribute::kMinValueForRange, "valuemin"),
          std::pair(FloatAttribute::kMaxValueForRange, "valuemax")}) {
      if (data.HasFloatAttribute(attribute)) {
        out.extra.push_back(bridge::Attribute{
            key, base::NumberToString(data.GetFloatAttribute(attribute))});
      }
    }
  }
  OptionalText(data, StringAttribute::kUrl, out.has_url, out.url);
  // HTML-`type` eines <input> (Blink: AXObject::Serialize...Attributes,
  // kInputType), Schlüssel `relief_interaction::security::INPUT_TYPE`: Die
  // Rückfrage verdeckt dann Werte von Passwortfeldern. HTML-`autocomplete`
  // kommt nicht an (kAutoComplete ist aria-autocomplete), das Formularziel
  // auch nicht (AXNodeObject::Url kennt nur Links, Dokument, Bilder).
  if (data.HasStringAttribute(StringAttribute::kInputType)) {
    out.extra.push_back(bridge::Attribute{
        "inputType",
        Text(data.GetStringAttribute(StringAttribute::kInputType))});
  }
  OptionalText(data, StringAttribute::kChildTreeId, out.has_child_tree,
               out.child_tree);

  out.focusable = data.HasState(State::kFocusable);
  out.disabled = data.GetRestriction() == ax::mojom::Restriction::kDisabled;
  out.readonly = data.GetRestriction() == ax::mojom::Restriction::kReadOnly;
  out.required = data.HasState(State::kRequired);
  out.modal = data.GetBoolAttribute(BoolAttribute::kModal);
  out.multiline = data.HasState(State::kMultiline);
  out.multiselectable = data.HasState(State::kMultiselectable);
  out.settable = data.HasAction(ax::mojom::Action::kSetValue);
  out.busy = data.GetBoolAttribute(BoolAttribute::kBusy);

  const bool expanded = data.HasState(State::kExpanded);
  const bool collapsed = data.HasState(State::kCollapsed);
  out.expanded = expanded    ? bridge::OptBool::True
                 : collapsed ? bridge::OptBool::False
                             : bridge::OptBool::Unset;
  if (data.HasBoolAttribute(BoolAttribute::kSelected)) {
    out.selected = data.GetBoolAttribute(BoolAttribute::kSelected)
                       ? bridge::OptBool::True
                       : bridge::OptBool::False;
  }
  // aria-pressed bildet Chromium als Rolle kToggleButton mit CheckedState ab.
  const bridge::Toggle toggle = ToToggle(data.GetCheckedState());
  if (data.role == ax::mojom::Role::kToggleButton) {
    out.pressed = toggle;
  } else {
    out.checked = toggle;
  }
  if (data.GetInvalidState() == ax::mojom::InvalidState::kTrue) {
    out.invalid = bridge::Invalid::True;
  }
  if (data.HasState(State::kRichlyEditable)) {
    out.editable = bridge::Editable::Richtext;
  } else if (data.HasState(State::kEditable)) {
    out.editable = bridge::Editable::Plaintext;
  }
  out.has_popup = ToHasPopup(data.GetHasPopup());
  if (data.HasState(State::kHorizontal)) {
    out.orientation = bridge::Orientation::Horizontal;
  } else if (data.HasState(State::kVertical)) {
    out.orientation = bridge::Orientation::Vertical;
  }

  // Aktionen, die Chromium am Knoten meldet: DoDefault über das
  // Default-Action-Verb (Blink setzt dafür keine Aktion im Bitfeld),
  // Fokus über kFocusable, der Rest aus dem Aktions-Bitfeld bzw. dem
  // Auf-/Zuklapp-Zustand.
  if (data.GetDefaultActionVerb() != ax::mojom::DefaultActionVerb::kNone) {
    out.actions.push_back(bridge::Action::DoDefault);
  }
  if (out.focusable) {
    out.actions.push_back(bridge::Action::Focus);
    out.actions.push_back(bridge::Action::Blur);
  }
  if (out.settable) {
    out.actions.push_back(bridge::Action::SetValue);
  }
  if (data.HasAction(ax::mojom::Action::kIncrement)) {
    out.actions.push_back(bridge::Action::Increment);
  }
  if (data.HasAction(ax::mojom::Action::kDecrement)) {
    out.actions.push_back(bridge::Action::Decrement);
  }
  if (collapsed) {
    out.actions.push_back(bridge::Action::Expand);
  }
  if (expanded) {
    out.actions.push_back(bridge::Action::Collapse);
  }

  out.ignored = data.IsIgnored();
  if (data.HasIntAttribute(IntAttribute::kHierarchicalLevel)) {
    out.has_level = true;
    out.level = data.GetIntAttribute(IntAttribute::kHierarchicalLevel);
  }
  if (node.parent()) {
    out.has_parent = true;
    out.parent = node.parent()->id();
  }
  out.children = Ids(data.child_ids);
  out.labelled_by =
      Ids(data.GetIntListAttribute(IntListAttribute::kLabelledbyIds));
  out.described_by =
      Ids(data.GetIntListAttribute(IntListAttribute::kDescribedbyIds));
  out.controls = Ids(data.GetIntListAttribute(IntListAttribute::kControlsIds));
  out.details = Ids(data.GetIntListAttribute(IntListAttribute::kDetailsIds));
  out.error_message =
      Ids(data.GetIntListAttribute(IntListAttribute::kErrormessageIds));
  out.flow_to = Ids(data.GetIntListAttribute(IntListAttribute::kFlowtoIds));
  if (data.HasIntAttribute(IntAttribute::kActivedescendantId)) {
    out.has_active_descendant = true;
    out.active_descendant =
        data.GetIntAttribute(IntAttribute::kActivedescendantId);
  }
  if (data.GetDOMNodeId() > 0) {
    out.has_dom_node_id = true;
    out.dom_node_id = data.GetDOMNodeId();
  }
  return out;
}

}  // namespace

std::string_view ReliefRoleName(ax::mojom::Role role) {
  using ax::mojom::Role;
  // Nur, wo Chromiums interner Name vom ARIA-Namen abweicht; sonst gilt
  // ui::ToString (lowerCamelCase), wie in relief-model vorgesehen.
  switch (role) {
    case Role::kAlertDialog:
      return "alertdialog";
    case Role::kCheckBox:
      return "checkbox";
    case Role::kColumnHeader:
      return "columnheader";
    case Role::kComboBoxGrouping:
    case Role::kComboBoxMenuButton:
    case Role::kComboBoxSelect:
    case Role::kTextFieldWithComboBox:
      return "combobox";
    case Role::kContentDeletion:
      return "deletion";
    case Role::kContentInfo:
    // Blink vergibt kFooter/kHeader nur außerhalb von Sectioning-Inhalt
    // (sonst kSectionFooter/kSectionHeader): dann Landmark wie bei CDP.
    case Role::kFooter:
      return "contentinfo";
    case Role::kHeader:
      return "banner";
    case Role::kContentInsertion:
      return "insertion";
    case Role::kGenericContainer:
      return "generic";
    case Role::kGridCell:
      return "gridcell";
    case Role::kListBox:
      return "listbox";
    case Role::kListBoxOption:
      return "option";
    case Role::kListItem:
      return "listitem";
    case Role::kMenuBar:
      return "menubar";
    case Role::kMenuItem:
      return "menuitem";
    case Role::kMenuItemCheckBox:
      return "menuitemcheckbox";
    case Role::kMenuItemRadio:
      return "menuitemradio";
    case Role::kProgressIndicator:
      return "progressbar";
    case Role::kRadioButton:
      return "radio";
    case Role::kRadioGroup:
      return "radiogroup";
    case Role::kRowGroup:
      return "rowgroup";
    case Role::kRowHeader:
      return "rowheader";
    case Role::kScrollBar:
      return "scrollbar";
    case Role::kSearchBox:
      return "searchbox";
    case Role::kSectionFooter:
      return "sectionfooter";
    case Role::kSectionHeader:
      return "sectionheader";
    case Role::kSpinButton:
      return "spinbutton";
    case Role::kSplitter:
      return "separator";
    case Role::kTabList:
      return "tablist";
    case Role::kTabPanel:
      return "tabpanel";
    case Role::kTextField:
      return "textbox";
    case Role::kToggleButton:
      return "button";
    case Role::kTreeGrid:
      return "treegrid";
    case Role::kTreeItem:
      return "treeitem";
    default:
      return ui::ToString(role);
  }
}

AXTreeMirror::AXTreeMirror(std::string id, bool child)
    : id_(std::move(id)), child_(child) {
  tree_.AddObserver(this);
}

AXTreeMirror::~AXTreeMirror() {
  tree_.RemoveObserver(this);
}

AXTreeMirror::Start AXTreeMirror::CheckStart(
    const ui::AXUpdatesAndEvents& details) const {
  if (tree_.root() || details.updates.empty()) {
    return Start::kReady;
  }
  const ui::AXTreeUpdate& first = details.updates.front();
  if (first.nodes.empty()) {
    return Start::kDataOnly;
  }
  const bool has_root =
      first.root_id != ui::kInvalidAXNodeID &&
      std::ranges::any_of(first.nodes, [&first](const ui::AXNodeData& node) {
        return node.id == first.root_id;
      });
  return has_root ? Start::kReady : Start::kNoRoot;
}

bool AXTreeMirror::Unserialize(const ui::AXTreeUpdate& update) {
  return tree_.Unserialize(update);
}

void AXTreeMirror::ApplyLocationChanges(
    const ui::AXLocationAndScrollUpdates& changes) {
  for (const ui::AXScrollChange& change : changes.scroll_changes) {
    if (ui::AXNode* node = tree_.GetFromId(change.id)) {
      node->SetScrollInfo(change.scroll_x, change.scroll_y);
      if (!IsRootScroller(change.id)) {
        moved_.insert(change.id);
      }
    }
  }
  for (const ui::AXLocationChange& change : changes.location_changes) {
    if (ui::AXNode* node = tree_.GetFromId(change.id)) {
      node->SetLocation(change.new_location.offset_container_id,
                        change.new_location.bounds,
                        change.new_location.transform.get());
      moved_.insert(change.id);
    }
  }
}

std::optional<gfx::RectF> AXTreeMirror::PageBounds(const ui::AXNode& node,
                                                   float scale) const {
  // Ungeclippt: die Fläche des Elements, nicht der sichtbare Ausschnitt.
  gfx::RectF bounds = tree_.RelativeToTreeBounds(
      &node, gfx::RectF(), /*offscreen=*/nullptr, /*clip_bounds=*/false);
  if (bounds.IsEmpty() || (child_ && !offset_)) {
    return std::nullopt;
  }
  // RelativeToTreeBounds zieht die Scroll-Position des Root-Scrollers ab
  // (Viewport-Koordinaten). Im Hauptdokument zurückgerechnet auf die Seite
  // wie BrowserAccessibility::RelativeToAbsoluteBounds ohne
  // UseRootScrollOffsetsWhenComputingBounds; ein iframe-Viewport liegt am
  // Host-Knoten im Elternbaum (dort setzt Chromium ebenfalls an).
  if (child_) {
    bounds.InvScale(scale);
    bounds.Offset(*offset_);
    return bounds;
  }
  if (const ui::AXNode* scroller =
          tree_.GetFromId(tree_.data().root_scroller_id)) {
    int x = 0;
    int y = 0;
    scroller->GetScrollInfo(&x, &y);
    bounds.Offset(x, y);
  }
  bounds.InvScale(scale);
  return bounds;
}

std::optional<gfx::RectF> AXTreeMirror::NodePageBounds(ui::AXNodeID id,
                                                       float scale) const {
  const ui::AXNode* node = tree_.GetFromId(id);
  return node ? PageBounds(*node, scale) : std::nullopt;
}

std::optional<AXTreeMirror::Scroll> AXTreeMirror::RootScroll() const {
  const ui::AXNode* scroller = tree_.GetFromId(tree_.data().root_scroller_id);
  if (!scroller) {
    return std::nullopt;
  }
  int x = 0;
  int y = 0;
  scroller->GetScrollInfo(&x, &y);
  return Scroll{scroller->id(), x, y,
                scroller->GetIntAttribute(ax::mojom::IntAttribute::kScrollYMax)};
}

bool AXTreeMirror::SetOffset(std::optional<gfx::Vector2dF> offset) {
  if (offset == offset_) {
    return false;
  }
  offset_ = offset;
  if (const ui::AXNode* root = tree_.root()) {
    moved_.insert(root->id());
  }
  return true;
}

void AXTreeMirror::CollectBounds(const ui::AXNode& root,
                                 float scale,
                                 bridge::TreeUpdate& update) {
  std::vector<const ui::AXNode*> stack = {&root};
  while (!stack.empty()) {
    const ui::AXNode* node = stack.back();
    stack.pop_back();
    for (const ui::AXNode* child : node->children()) {
      stack.push_back(child);
    }
    const ui::AXNodeID id = node->id();
    // Angelegte und geänderte Knoten gehen vollständig mit Position hinüber.
    if (touched_.contains(id) || !known_.contains(id)) {
      continue;
    }
    const std::optional<gfx::RectF> bounds = PageBounds(*node, scale);
    auto sent = sent_bounds_.find(id);
    const bool had = sent != sent_bounds_.end();
    if (bounds ? (had && sent->second == *bounds) : !had) {
      continue;
    }
    bridge::BoundsChange change{};
    change.node = id;
    if (bounds) {
      change.has_bounds = true;
      change.bounds = ToRect(*bounds);
      sent_bounds_.insert_or_assign(id, *bounds);
    } else {
      sent_bounds_.erase(sent);
    }
    update.bounds.push_back(std::move(change));
  }
}

void AXTreeMirror::TakeUpdate(rust::Vec<bridge::TreeUpdate>& out,
                              ChildTreeHosts& hosts,
                              float scale) {
  const ui::AXNode* root = tree_.root();
  const std::optional<ui::AXNodeID> root_id =
      root ? std::optional<ui::AXNodeID>(root->id()) : std::nullopt;
  const bool root_changed = root_id != sent_root_;
  if (touched_.empty() && deleted_.empty() && moved_.empty() && !data_dirty_ &&
      !root_changed) {
    return;
  }

  bridge::TreeUpdate update{};
  update.tree = Text(id_);

  // Gelöscht und im selben Paket neu angelegt (Umhängen, Reset) zählt als
  // geändert, nicht als entfernt.
  for (ui::AXNodeID id : deleted_) {
    if (!tree_.GetFromId(id) && known_.erase(id)) {
      update.removed.push_back(id);
    }
  }
  for (ui::AXNodeID id : touched_) {
    const ui::AXNode* node = tree_.GetFromId(id);
    if (!node) {
      continue;
    }
    const std::optional<gfx::RectF> bounds = PageBounds(*node, scale);
    if (bounds) {
      sent_bounds_.insert_or_assign(id, *bounds);
    } else {
      sent_bounds_.erase(id);
    }
    bridge::Node converted = ToNode(*node, bounds);
    if (converted.has_child_tree) {
      hosts[std::string(converted.child_tree)] = {id_, id};
    }
    if (known_.insert(id).second) {
      update.created.push_back(std::move(converted));
    } else {
      update.changed.push_back(std::move(converted));
    }
  }

  // Teilbäume verschobener Knoten; ein Knoten unter einem anderen
  // verschobenen wird nur einmal besucht.
  for (ui::AXNodeID id : moved_) {
    const ui::AXNode* node = tree_.GetFromId(id);
    bool covered = false;
    for (const ui::AXNode* up = node ? node->parent() : nullptr; up && !covered;
         up = up->parent()) {
      covered = moved_.contains(up->id());
    }
    if (node && !covered) {
      CollectBounds(*node, scale, update);
    }
  }

  if (data_dirty_ || !sent_once_) {
    const ui::AXTreeData& data = tree_.data();
    update.has_data = true;
    if (!data.url.empty()) {
      update.data.has_url = true;
      update.data.url = Text(data.url);
    }
    if (!data.title.empty()) {
      update.data.has_title = true;
      update.data.title = Text(data.title);
    }
    if (data.focus_id != ui::kInvalidAXNodeID) {
      update.data.has_focus = true;
      update.data.focus = data.focus_id;
    }
    if (auto host = hosts.find(id_); host != hosts.end()) {
      update.data.has_parent = true;
      update.data.parent_tree = Text(host->second.first);
      update.data.parent_node = host->second.second;
    }
  }
  if (root_changed && root_id) {
    update.has_root = true;
    update.root = *root_id;
  }
  sent_root_ = root_id;

  touched_.clear();
  deleted_.clear();
  moved_.clear();
  data_dirty_ = false;
  sent_once_ = true;
  out.push_back(std::move(update));
}

void AXTreeMirror::OnNodeDataWillChange(ui::AXTree* tree,
                                        const ui::AXNodeData& old_node_data,
                                        const ui::AXNodeData& new_node_data) {
  // Der Knoten selbst geht als geändert hinüber; neu rechnen muss man, wenn
  // seine Position oder Scroll-Position sich ändert, seinen Teilbaum.
  const bool scrolled =
      !IsRootScroller(new_node_data.id) &&
      (old_node_data.GetIntAttribute(IntAttribute::kScrollX) !=
           new_node_data.GetIntAttribute(IntAttribute::kScrollX) ||
       old_node_data.GetIntAttribute(IntAttribute::kScrollY) !=
           new_node_data.GetIntAttribute(IntAttribute::kScrollY));
  if (scrolled ||
      old_node_data.relative_bounds != new_node_data.relative_bounds) {
    moved_.insert(new_node_data.id);
  }
}

void AXTreeMirror::OnNodeWillBeDeleted(ui::AXTree* tree, ui::AXNode* node) {
  deleted_.insert(node->id());
  moved_.erase(node->id());
  sent_bounds_.erase(node->id());
}

void AXTreeMirror::OnTreeDataChanged(ui::AXTree* tree,
                                     const ui::AXTreeData& old_data,
                                     const ui::AXTreeData& new_data) {
  data_dirty_ = true;
}

void AXTreeMirror::OnAtomicUpdateFinished(ui::AXTree* tree,
                                          bool root_changed,
                                          const std::vector<Change>& changes) {
  for (const Change& change : changes) {
    touched_.insert(change.node->id());
  }
}

}  // namespace relief
