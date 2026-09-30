//! Warum eine Modellausgabe verworfen wurde.

use std::fmt;

use crate::Property;

/// Eine Modellausgabe verletzt den Vertrag. Die Ausgabe wird dann **ganz**
/// verworfen, nicht teilweise übernommen.
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Kein JSON nach Schema: Syntax, fehlendes oder unbekanntes Feld,
    /// falscher Typ, unbekannter Enum-Wert.
    Format(String),
    /// Knoten-ID kommt in der Eingabe nicht vor.
    UnknownNode(String),
    /// Hypothese zu einem Feld, das der Filter als sensibel geschwärzt hat.
    Redacted(String),
    /// Die Eigenschaft hat schon einen Wert von Chromium; KI rekonstruiert
    /// nur Fehlendes (→ `spezifikation/03`, Quellenpriorität).
    NotMissing {
        node: String,
        property: Property,
    },
    /// Zwei Hypothesen zu derselben Eigenschaft desselben Knotens.
    Duplicate {
        node: String,
        property: Property,
    },
    TooMany(usize),
    /// Confidence außerhalb von 0 bis 1.
    Confidence(f32),
    /// Wert leer oder zu lang.
    Value(&'static str),
    /// Evidence fehlt, zu viele Einträge oder ein Eintrag leer/zu lang.
    Evidence(&'static str),
    /// Die Äußerung im Intent ist nicht die der Nutzerin.
    Utterance,
    /// Ziel fehlt, obwohl der Intent eins braucht, oder ist unzulässig.
    Target(&'static str),
    /// Ziel verweist auf einen anderen Stand als die Eingabe.
    GraphVersion {
        expected: u64,
        found: u64,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(e) => write!(f, "Ausgabe entspricht nicht dem Schema: {e}"),
            Self::UnknownNode(id) => write!(f, "Knoten {id} gibt es in der Eingabe nicht"),
            Self::Redacted(id) => write!(f, "Knoten {id} ist geschwärzt"),
            Self::NotMissing { node, property } => {
                write!(f, "{property} von {node} ist schon bekannt")
            }
            Self::Duplicate { node, property } => {
                write!(f, "{property} von {node} mehrfach vorgeschlagen")
            }
            Self::TooMany(n) => write!(f, "{n} Hypothesen sind zu viele"),
            Self::Confidence(c) => write!(f, "Confidence {c} liegt nicht zwischen 0 und 1"),
            Self::Value(why) => write!(f, "Wert unzulässig: {why}"),
            Self::Evidence(why) => write!(f, "Evidence unzulässig: {why}"),
            Self::Utterance => write!(f, "Intent nennt eine andere Äußerung als die Nutzerin"),
            Self::Target(why) => write!(f, "Ziel unzulässig: {why}"),
            Self::GraphVersion { expected, found } => {
                write!(f, "Ziel aus Stand {found}, Eingabe ist Stand {expected}")
            }
        }
    }
}

impl std::error::Error for ValidationError {}
