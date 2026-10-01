//! Fähigkeitsprofil im Fork (Paket 41): der Speicherstand als JSON (der
//! Fork hält ihn je Browserprofil in einer Datei), Änderungen und die
//! Wirkungen für einen Tab. Die Regeln stehen in
//! `relief_interaction::profile`.

use relief_interaction::profile::{
    apply_line, describe_effects, Capabilities, Effects, Store, FIELDS, PRESETS,
};
use serde_json::{json, Value};

use crate::runtime::Runtime;

/// Gespeicherter Stand; leer oder unlesbar: Standard.
pub fn store(json: &str) -> Store {
    serde_json::from_str(json).unwrap_or_default()
}

pub fn to_json(store: &Store) -> String {
    serde_json::to_string(store).expect("Store als JSON")
}

/// Stand für die Oberfläche: je Feld Wert, globale und Website-Abweichung,
/// Standard; Voreinstellungen; Wirkung in Worten.
pub fn view(store: &Store, site: &str) -> String {
    let effective = serde_json::to_value(store.effective(site)).expect("JSON");
    let defaults = serde_json::to_value(Capabilities::default()).expect("JSON");
    let site_overrides = store.sites.get(site);
    let fields: Vec<Value> = FIELDS
        .iter()
        .map(|f| {
            json!({
                "field": f,
                "value": effective[*f],
                "default": defaults[*f],
                "global": store.global.get(*f),
                "site": site_overrides.and_then(|o| o.get(*f)),
            })
        })
        .collect();
    json!({
        "site": site,
        "fields": fields,
        "presets": PRESETS.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        "effects": describe_effects(&store.effective(site).effects()),
    })
    .to_string()
}

impl Runtime {
    /// Wirkungen des Profils für diesen Tab übernehmen.
    pub fn set_profile(&mut self, effects: Effects) {
        self.session.set_effects(effects);
    }
}

/// Aufgabenzeile `profil:`.
pub fn line(store: &mut Store, text: &str) -> Result<String, String> {
    apply_line(store, text)
}
