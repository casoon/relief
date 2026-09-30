//! Relief: Grenze zwischen Browser und Runtime.
//!
//! Browserfrei. Stabile Schnittstelle: **Delta rein** ([`relief_model::TreeDelta`]),
//! **Antworten und ActionPlans raus** ([`Answer`], [`ActionPlan`]). Zwei
//! Varianten, wie Chromium die Runtime aufruft (→ `plan/spezifikation/02`):
//!
//! | Variante | Ort | Weg über die Grenze |
//! |---|---|---|
//! | A: in-process | Browser-Prozess | [`ffi`]: `cxx`-Bridge, flache geteilte Strukturen, keine Serialisierung |
//! | B: Utility-Prozess | eigener, sandboxbarer Prozess | `mojom/relief_runtime.mojom`: dieselben Strukturen als Mojo-Typen; im Utility-Prozess weiter über [`ffi`] |
//!
//! Befehle in Sprache laufen über [`Runtime::command`]: dieselbe
//! Befehlslogik wie im CDP-Host (`relief_interaction::Session`), der Plan wird
//! hier in AX-Schritte übersetzt ([`ax_steps`]), die der Fork als
//! `AXActionData` schickt.
//!
//! Die Rust-Seite ist in beiden Varianten dieselbe [`Runtime`]; Variante B
//! kostet zusätzlich Mojo-Serialisierung, IPC und eine Kopie im
//! Utility-Prozess. Messungen: `cargo bench -p relief-bridge`.

// `cxx` erzeugt `unsafe`-Code; eigener Code kommt ohne aus.
#![deny(unsafe_code)]

mod command;
#[allow(unsafe_code)]
mod cxx_bridge;
pub mod devtools;
mod inspector;
mod runtime;
mod semantic;

pub use command::{ax_steps, AxStep, Key, Reply, Step};
pub use cxx_bridge::{delta_from_ffi, delta_to_ffi, ffi};
pub use inspector::inspector_json;
pub use runtime::{ActionPlan, ActionRequest, Answer, Rejection, Runtime};
