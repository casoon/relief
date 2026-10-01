//! Fähigkeitsprofile (Paket 41, → `plan/spezifikation/08`): verfügbare
//! bzw. benötigte Fähigkeiten, frei kombinierbar, keine Diagnosemodi.
//! Browserfrei: Datensatz, globale und websitespezifische Werte, neutrale
//! Voreinstellungen und die daraus folgenden Wirkungen ([`Effects`]). Das
//! Profil bleibt lokal; nichts davon geht an Seiten oder Dienste.
//!
//! Wirkungen nehmen nie Inhalt, Bedeutung oder Bedienwege weg: Kurze
//! Antworten heben den Rest für „mehr“ auf, Rückfragen werden höchstens
//! mehr, nie weniger.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Wie viel einer Fähigkeit verfügbar ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Full,
    Reduced,
    None,
}

/// Ein- oder Ausgabekanal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Availability {
    Preferred,
    Available,
    Unavailable,
}

/// Wie viel Kontrast gebraucht wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Need {
    Standard,
    Increased,
    Maximum,
}

/// Das Profil. Abweichend von der Skizze in spezifikation/08 ist Kontrast
/// ein Bedarf ([`Need`]) statt einer verfügbaren Stufe: „volle
/// Kontrastanforderung“ wäre mehrdeutig.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Capabilities {
    pub visual_detail: Level,
    /// Faktor für Text und Bedienelemente (1,0–3,0).
    pub text_scale: f32,
    pub color_discrimination: Level,
    pub contrast: Need,
    pub motion_tolerance: Level,
    pub audio_output: Availability,
    pub speech_input: Availability,
    pub keyboard_input: Availability,
    pub pointer_input: Availability,
    pub switch_input: Availability,
    /// Wie viel Text auf einmal gut verarbeitbar ist.
    pub text_complexity: Level,
}

impl Default for Capabilities {
    fn default() -> Self {
        Capabilities {
            visual_detail: Level::Full,
            text_scale: 1.0,
            color_discrimination: Level::Full,
            contrast: Need::Standard,
            motion_tolerance: Level::Full,
            audio_output: Availability::Available,
            speech_input: Availability::Available,
            keyboard_input: Availability::Available,
            pointer_input: Availability::Available,
            switch_input: Availability::Unavailable,
            text_complexity: Level::Full,
        }
    }
}

/// Feldnamen in Reihenfolge der Oberfläche.
pub const FIELDS: &[&str] = &[
    "visual_detail",
    "text_scale",
    "color_discrimination",
    "contrast",
    "motion_tolerance",
    "audio_output",
    "speech_input",
    "keyboard_input",
    "pointer_input",
    "switch_input",
    "text_complexity",
];

/// Abweichungen von einem Grundprofil: nur gesetzte Felder (global
/// gegenüber dem Standard, je Website gegenüber global).
pub type Overrides = BTreeMap<String, serde_json::Value>;

/// Gespeicherter Stand: globale Abweichungen und je Website (Host).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    pub global: Overrides,
    pub sites: BTreeMap<String, Overrides>,
}

fn apply(base: &Capabilities, overrides: &Overrides) -> Capabilities {
    let mut value = serde_json::to_value(base).expect("Capabilities als JSON");
    for (field, v) in overrides {
        value[field.as_str()] = v.clone();
    }
    serde_json::from_value(value).unwrap_or_else(|_| base.clone())
}

impl Store {
    /// Profil für `site` (leer: global).
    pub fn effective(&self, site: &str) -> Capabilities {
        let global = apply(&Capabilities::default(), &self.global);
        match self.sites.get(site) {
            Some(o) if !site.is_empty() => apply(&global, o),
            _ => global,
        }
    }

    /// Ein Feld setzen (global bei leerem `site`); Fehler bei unbekanntem
    /// Feld oder ungültigem Wert. Ein Wert gleich dem Grundprofil entfernt
    /// die Abweichung.
    pub fn set(&mut self, site: &str, field: &str, value: serde_json::Value) -> Result<(), String> {
        if !FIELDS.contains(&field) {
            return Err(format!("Unbekannte Fähigkeit „{field}“."));
        }
        let base = if site.is_empty() {
            Capabilities::default()
        } else {
            self.effective("")
        };
        let mut probe = Overrides::new();
        probe.insert(field.to_string(), value.clone());
        let applied = apply(&base, &probe);
        if serde_json::to_value(&applied).expect("JSON")[field] != value {
            return Err(format!("Ungültiger Wert für „{field}“: {value}."));
        }
        if field == "text_scale" && !(1.0..=3.0).contains(&applied.text_scale) {
            return Err("Textgröße zwischen 1,0 und 3,0.".into());
        }
        let same = serde_json::to_value(&base).expect("JSON")[field] == value;
        let target = if site.is_empty() {
            &mut self.global
        } else {
            self.sites.entry(site.to_string()).or_default()
        };
        if same {
            target.remove(field);
        } else {
            target.insert(field.to_string(), value);
        }
        self.sites.retain(|_, o| !o.is_empty());
        Ok(())
    }

    /// Ein Feld zurücksetzen (global bzw. nur für `site`).
    pub fn reset_field(&mut self, site: &str, field: &str) {
        if site.is_empty() {
            self.global.remove(field);
        } else if let Some(o) = self.sites.get_mut(site) {
            o.remove(field);
        }
        self.sites.retain(|_, o| !o.is_empty());
    }

    /// „Standard wiederherstellen“: alles global und je Website.
    pub fn reset_all(&mut self) {
        *self = Store::default();
    }

    /// Voreinstellung als Ausgangspunkt (global): setzt ihre Werte,
    /// alles bleibt einzeln änderbar.
    pub fn apply_preset(&mut self, name: &str) -> Result<(), String> {
        let (_, values) = PRESETS
            .iter()
            .find(|(n, _)| *n == name)
            .ok_or_else(|| format!("Unbekannte Voreinstellung „{name}“."))?;
        for (field, value) in *values {
            self.set("", field, serde_json::from_str(value).expect("Preset-Wert"))?;
        }
        Ok(())
    }
}

/// Neutrale Startvoreinstellungen: beschreiben, was sie tun, nicht wen sie
/// meinen.
pub const PRESETS: &[(&str, &[(&str, &str)])] = &[
    (
        "Ausgabe vor allem gesprochen",
        &[
            ("audio_output", "\"preferred\""),
            ("visual_detail", "\"reduced\""),
        ],
    ),
    (
        "Große, ruhige Darstellung",
        &[
            ("text_scale", "1.5"),
            ("contrast", "\"increased\""),
            ("motion_tolerance", "\"reduced\""),
        ],
    ),
    (
        "Bedienung ohne Zeigegerät",
        &[("pointer_input", "\"unavailable\"")],
    ),
    (
        "Kurze Antworten, mehr Rückfragen",
        &[("text_complexity", "\"reduced\"")],
    ),
];

/// Was das Profil an Ausgabe, Eingabe und Darstellung ändert.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Effects {
    /// Antworten sprechen.
    pub speak_answers: bool,
    /// Nur den ersten Satz bzw. die erste Zeile, Rest über „mehr“.
    pub short_answers: bool,
    /// Auch Aktionen mittleren Risikos (Auswählen, Ausfüllen, Auslösen)
    /// erst nach Rückfrage.
    pub confirm_changes: bool,
    /// Semantic View als Startansicht im Panel.
    pub semantic_view: bool,
    /// Sprungmarken nach dem Laden zeigen.
    pub marks_on_load: bool,
    /// Zoom für Seite und Panel.
    pub zoom: f32,
    pub increased_contrast: bool,
    pub reduced_motion: bool,
}

impl Default for Effects {
    fn default() -> Self {
        Capabilities::default().effects()
    }
}

impl Capabilities {
    pub fn effects(&self) -> Effects {
        let no_pointer = self.pointer_input == Availability::Unavailable;
        Effects {
            speak_answers: self.audio_output == Availability::Preferred
                || self.visual_detail == Level::None,
            short_answers: self.text_complexity != Level::Full,
            confirm_changes: self.text_complexity != Level::Full,
            semantic_view: self.visual_detail == Level::Reduced || no_pointer,
            marks_on_load: no_pointer && self.keyboard_input != Availability::Unavailable,
            zoom: self.text_scale,
            increased_contrast: self.contrast != Need::Standard,
            reduced_motion: self.motion_tolerance != Level::Full,
        }
    }
}

/// Antwort nach Profil: bei kurzen Antworten erste Zeile bzw. erster Satz,
/// der Rest kommt zurück (für „mehr“).
pub fn present(answer: &str, effects: &Effects) -> (String, Option<String>) {
    if !effects.short_answers {
        return (answer.to_string(), None);
    }
    let (first, rest) = match answer.split_once('\n') {
        Some((first, rest)) => (first.trim_end(), rest.trim()),
        None => (answer, ""),
    };
    // Erster Satz der ersten Zeile; „(…)“-Erklärungen gehören zum Rest.
    let (head, tail) = match first.find(". ") {
        Some(i) => (&first[..=i], first[i + 2..].trim()),
        None => (first, ""),
    };
    let rest = [tail, rest]
        .iter()
        .filter(|t| !t.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    if rest.is_empty() {
        (head.to_string(), None)
    } else {
        (format!("{head} („mehr“ für Details)"), Some(rest))
    }
}

/// Wirkungen in Worten.
pub fn describe_effects(e: &Effects) -> String {
    let mut parts = Vec::new();
    if e.speak_answers {
        parts.push("Antworten gesprochen".to_string());
    }
    if e.short_answers {
        parts.push("kurze Antworten („mehr“ für den Rest)".into());
    }
    if e.confirm_changes {
        parts.push("Rückfrage auch vor Änderungen".into());
    }
    if e.semantic_view {
        parts.push("Semantic View zuerst".into());
    }
    if e.marks_on_load {
        parts.push("Sprungmarken nach dem Laden".into());
    }
    if e.zoom != 1.0 {
        parts.push(format!("Zoom {} %", (e.zoom * 100.0).round()));
    }
    if e.increased_contrast {
        parts.push("erhöhter Kontrast".into());
    }
    if e.reduced_motion {
        parts.push("wenig Bewegung".into());
    }
    if parts.is_empty() {
        "keine (Standard)".into()
    } else {
        parts.join(", ")
    }
}

/// Aufgabenzeile `profil: …` (global): „standard“, der Name einer
/// Voreinstellung oder `feld=wert, …`. Antwort: Abweichungen und Wirkung.
pub fn apply_line(store: &mut Store, line: &str) -> Result<String, String> {
    let line = line.trim();
    if line.eq_ignore_ascii_case("standard") {
        store.reset_all();
    } else if PRESETS.iter().any(|(n, _)| *n == line) {
        store.apply_preset(line)?;
    } else {
        for pair in line.split(',') {
            let (field, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("Erwartet feld=wert, war „{}“.", pair.trim()))?;
            let value = value.trim();
            let json = value
                .parse::<f64>()
                .map(serde_json::Value::from)
                .unwrap_or_else(|_| serde_json::Value::from(value));
            store.set("", field.trim(), json)?;
        }
    }
    let changed: Vec<String> = store
        .global
        .iter()
        .map(|(k, v)| format!("{k}={}", v.to_string().trim_matches('"')))
        .collect();
    Ok(format!(
        "Profil: {}. Wirkung: {}.",
        if changed.is_empty() {
            "Standard".into()
        } else {
            changed.join(", ")
        },
        describe_effects(&store.effective("").effects())
    ))
}

/// Felder als Tabelle für Ausgaben: (Feld, Wert als JSON).
pub fn describe(c: &Capabilities) -> Vec<(String, String)> {
    let value = serde_json::to_value(c).expect("JSON");
    FIELDS
        .iter()
        .map(|f| (f.to_string(), value[*f].to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn standard_ohne_wirkung() {
        let e = Capabilities::default().effects();
        assert!(!e.speak_answers && !e.short_answers && !e.confirm_changes);
        assert!(!e.semantic_view && !e.marks_on_load);
        assert_eq!(e.zoom, 1.0);
    }

    #[test]
    fn global_und_je_website_getrennt_und_ruecksetzbar() {
        let mut s = Store::default();
        s.set("", "text_scale", json!(1.5)).unwrap();
        s.set("shop.test", "audio_output", json!("preferred"))
            .unwrap();
        assert_eq!(s.effective("").text_scale, 1.5);
        assert_eq!(s.effective("").audio_output, Availability::Available);
        let site = s.effective("shop.test");
        assert_eq!(
            (site.text_scale, site.audio_output),
            (1.5, Availability::Preferred)
        );
        // Wert gleich dem Grundprofil: keine Abweichung.
        s.set("shop.test", "audio_output", json!("available"))
            .unwrap();
        assert!(s.sites.is_empty());
        s.reset_field("", "text_scale");
        assert_eq!(s, Store::default());
        assert!(s.set("", "gibt_es_nicht", json!(1)).is_err());
        assert!(s.set("", "contrast", json!("viel")).is_err());
        assert!(s.set("", "text_scale", json!(9.0)).is_err());
    }

    #[test]
    fn voreinstellung_ist_ausgangspunkt() {
        let mut s = Store::default();
        s.apply_preset("Große, ruhige Darstellung").unwrap();
        s.set("", "motion_tolerance", json!("full")).unwrap();
        let c = s.effective("");
        assert_eq!(c.text_scale, 1.5);
        assert_eq!(c.motion_tolerance, Level::Full);
        s.reset_all();
        assert_eq!(s.effective(""), Capabilities::default());
    }

    #[test]
    fn aufgabenzeile() {
        let mut s = Store::default();
        let t = apply_line(&mut s, "text_complexity=reduced, text_scale=1.5").unwrap();
        assert!(
            t.contains("Rückfrage auch vor Änderungen") && t.contains("Zoom 150 %"),
            "{t}"
        );
        let t = apply_line(&mut s, "standard").unwrap();
        assert_eq!(t, "Profil: Standard. Wirkung: keine (Standard).");
        assert!(apply_line(&mut s, "Bedienung ohne Zeigegerät")
            .unwrap()
            .contains("Sprungmarken"));
        assert!(apply_line(&mut s, "quatsch").is_err());
    }

    #[test]
    fn kurze_antwort_behaelt_den_rest() {
        let e = Capabilities {
            text_complexity: Level::Reduced,
            ..Capabilities::default()
        }
        .effects();
        let (head, rest) = present("Seite „X“. Seitentyp Produktseite.\n1. eins", &e);
        assert_eq!(head, "Seite „X“. („mehr“ für Details)");
        assert_eq!(rest.as_deref(), Some("Seitentyp Produktseite.\n1. eins"));
        assert_eq!(present("Fertig.", &e), ("Fertig.".into(), None));
    }
}
