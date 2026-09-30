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
//! Die Rust-Seite ist in beiden Varianten dieselbe [`Runtime`]; Variante B
//! kostet zusätzlich Mojo-Serialisierung, IPC und eine Kopie im
//! Utility-Prozess. Messungen: `cargo bench -p relief-bridge`.

// `cxx` erzeugt `unsafe`-Code; eigener Code kommt ohne aus.
#![deny(unsafe_code)]

#[allow(unsafe_code)]
mod cxx_bridge;
mod runtime;

pub use cxx_bridge::{delta_from_ffi, delta_to_ffi, ffi};
pub use runtime::{ActionPlan, ActionRequest, Answer, Rejection, Runtime};
