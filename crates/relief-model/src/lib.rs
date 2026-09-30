//! Relief: semantisches Datenmodell und Delta-Format.
//!
//! Browserfrei. Das Modell hat die Form, in der Chromium den
//! Accessibility-Tree im Browser-Prozess liefert (ein Baum je Frame und
//! Dokument, Integer-IDs je Baum, inkrementelle Updates), ohne einen
//! Chromium-Typ zu verwenden. Befüllt wird es heute aus
//! `a11y_perception::AXTree` (CDP-Host, Aufnahmen) und vom Fork-Adapter
//! (→ `plan/spezifikation/02`, `03`).
//!
//! | Baustein | Inhalt |
//! |---|---|
//! | [`SemanticGraph`], [`SemanticTree`], [`TreeData`] | alle Bäume einer Seite, Verknüpfung über Child-Trees |
//! | [`SemanticNode`], [`States`], [`Relations`] | ein Knoten mit typisierten Eigenschaften |
//! | [`Fact`], [`Certainty`], [`Source`] | Herkunft einer Aussage |
//! | [`TreeId`], [`NodeId`], [`NodeRef`], [`GraphVersion`] | Identität und Version |
//! | [`TreeDelta`], [`TreeUpdate`] | inkrementelle Änderung; [`TreeDelta::between`] und [`SemanticGraph::apply`] |
//! | `perception` | Konverter `a11y_perception::AXTree` → Modell (Feature `perception`, Standard; im Fork-Build aus) |
//!
//! # Identität
//!
//! Ein Knoten ist durch [`NodeRef`] = ([`TreeId`], [`NodeId`]) bestimmt.
//! Chromium vergibt je Frame **und Dokument** eine eigene Tree-ID; nach einer
//! Navigation ist ein Knoten ein anderer Knoten, auch wenn die Node-ID gleich
//! bleibt. Das Modell rät keine Identität über Dokumentgrenzen hinweg.

#![forbid(unsafe_code)]

mod delta;
mod fact;
mod graph;
mod id;
mod node;
#[cfg(feature = "perception")]
pub mod perception;
mod role;

pub use delta::{ApplyError, BoundsChange, TreeDelta, TreeUpdate};
pub use fact::{Certainty, Fact, Source};
pub use graph::{SemanticGraph, SemanticTree, TreeData};
pub use id::{GraphVersion, NodeId, NodeRef, TreeId};
pub use node::{
    Action, Editable, HasPopup, IgnoredReason, Invalid, NameFrom, Orientation, Rect, Relations,
    SemanticNode, States, Toggle,
};
pub use role::Role;
