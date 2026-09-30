//! Ein Knoten des Modells und seine typisierten Eigenschaften.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Fact, NodeId, Role, TreeId};

/// Ein Knoten eines Accessibility-Baums.
///
/// Struktur (`parent`, `children`) und Relationen verweisen nur auf Knoten
/// **desselben** Baums, wie in Chromium. Der Übergang in einen anderen Baum
/// (iframe) läuft ausschließlich über [`SemanticNode::child_tree`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticNode {
    pub id: NodeId,
    pub role: Role,
    pub name: Fact<String>,
    /// Woher Chromium den Namen hat (HTML-/ARIA-Mechanismus).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_from: Option<NameFrom>,
    pub description: Fact<String>,
    pub value: Fact<String>,
    #[serde(default)]
    pub states: States,
    /// Aktionen, die Chromium für den Knoten meldet. CDP meldet keine; der
    /// CDP-Konverter lässt die Liste leer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    /// Seitenkoordinaten in CSS-Pixeln. Kommt in Chromium über einen eigenen
    /// Kanal (→ [`crate::BoundsChange`]); CDP liefert keine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Rect>,
    /// Chromium nimmt den Knoten aus der Barrierefreiheits-Sicht heraus; er
    /// bleibt im Baum (z. B. verborgen oder uninteressant).
    #[serde(default)]
    pub ignored: bool,
    /// Gründe für `ignored`. Nur CDP meldet sie; Chromium intern kennt nur
    /// den Zustand.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignored_reasons: Vec<IgnoredReason>,
    /// Überschriften-, Listen- oder Baumebene.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    /// Ziel eines Links bzw. Adresse eines Dokuments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub parent: Option<NodeId>,
    #[serde(default)]
    pub children: Vec<NodeId>,
    /// Eingebetteter Baum (iframe). Chromium: `kChildTreeId` am Knoten.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_tree: Option<TreeId>,
    #[serde(default, skip_serializing_if = "Relations::is_empty")]
    pub relations: Relations,
    /// DOM-Knoten (Chromium `kDOMNodeId`, CDP `backendDOMNodeId`) für
    /// DOM-/Layout-Kontext bei Bedarf. Keine Identität des AX-Knotens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dom_node_id: Option<i64>,
    /// Gemeldete Eigenschaften, die das Modell (noch) nicht typisiert, als
    /// Text (z. B. `roledescription`, `live`, `valuemin`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
}

impl SemanticNode {
    /// Ein leerer Knoten mit Rolle; alle Aussagen `Known` ohne Wert.
    pub fn new(id: NodeId, role: Role) -> Self {
        Self {
            id,
            role,
            name: Fact::default(),
            name_from: None,
            description: Fact::default(),
            value: Fact::default(),
            states: States::default(),
            actions: Vec::new(),
            bounds: None,
            ignored: false,
            ignored_reasons: Vec::new(),
            level: None,
            url: None,
            parent: None,
            children: Vec::new(),
            child_tree: None,
            relations: Relations::default(),
            dom_node_id: None,
            extra: BTreeMap::new(),
        }
    }
}

/// Zustände eines Knotens.
///
/// Fokus steht nicht hier: Chromium führt ihn je Baum (`AXTreeData`), das
/// Modell in [`crate::TreeData::focus`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct States {
    pub focusable: bool,
    pub disabled: bool,
    pub readonly: bool,
    pub required: bool,
    pub modal: bool,
    pub multiline: bool,
    pub multiselectable: bool,
    /// Wert ist über `SetValue` änderbar.
    pub settable: bool,
    pub busy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<Toggle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressed: Option<Toggle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<Invalid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editable: Option<Editable>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_popup: Option<HasPopup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<Orientation>,
}

/// `checked`/`pressed`: dreiwertig.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Toggle {
    False,
    True,
    Mixed,
}

/// `aria-invalid` ungleich `false`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Invalid {
    True,
    Spelling,
    Grammar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Editable {
    Plaintext,
    Richtext,
}

/// `aria-haspopup` ungleich `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HasPopup {
    True,
    Menu,
    Listbox,
    Tree,
    Grid,
    Dialog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Woher der Name kommt (Auswahl aus Chromiums `ax::mojom::NameFrom`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NameFrom {
    /// `aria-label`, `alt` u. ä.
    Attribute,
    /// `aria-labelledby`, `<label for>`.
    RelatedElement,
    /// Inhalt des Elements.
    Contents,
    Placeholder,
    Title,
}

/// Warum Chromium einen Knoten ignoriert (CDP-Kennungen).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IgnoredReason {
    Uninteresting,
    NotRendered,
    NotVisible,
    AriaHiddenElement,
    AriaHiddenSubtree,
    PresentationalRole,
    EmptyAlt,
    ActiveModalDialog,
    LabelFor,
    InertElement,
    InertSubtree,
    /// Grund ohne eigene Variante, mit der Kennung des Adapters.
    Other(String),
}

impl IgnoredReason {
    /// Aus der CDP-Kennung (`ignoredReasons[].name`).
    pub fn from_cdp(name: &str) -> Self {
        match name {
            "uninteresting" => Self::Uninteresting,
            "notRendered" => Self::NotRendered,
            "notVisible" => Self::NotVisible,
            "ariaHiddenElement" => Self::AriaHiddenElement,
            "ariaHiddenSubtree" => Self::AriaHiddenSubtree,
            "presentationalRole" => Self::PresentationalRole,
            "emptyAlt" => Self::EmptyAlt,
            "activeModalDialog" => Self::ActiveModalDialog,
            "labelFor" => Self::LabelFor,
            "inertElement" => Self::InertElement,
            "inertSubtree" => Self::InertSubtree,
            other => Self::Other(other.to_string()),
        }
    }
}

/// Aktionen, die Chromium für einen Knoten anbieten kann (Auswahl aus
/// `ax::mojom::Action`, die für Relief relevant ist → `spezifikation/01`, 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    DoDefault,
    Focus,
    Blur,
    SetValue,
    Increment,
    Decrement,
    Expand,
    Collapse,
    ScrollToMakeVisible,
    ShowContextMenu,
}

/// Relationen zu anderen Knoten desselben Baums.
///
/// `aria-owns` fehlt absichtlich: Chromium bildet es als Umhängen im Baum ab,
/// nicht als Relation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Relations {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labelled_by: Vec<NodeId>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub described_by: Vec<NodeId>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub controls: Vec<NodeId>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<NodeId>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub error_message: Vec<NodeId>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flow_to: Vec<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_descendant: Option<NodeId>,
}

impl Relations {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Rechteck in CSS-Pixeln, Seitenkoordinaten.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
