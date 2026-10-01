//! Sprachschicht (Paket 26): Spracheingabe → Befehl → Antwort →
//! Sprachausgabe (→ `plan/spezifikation/08`, „Speech Assist“). Browserfrei:
//! Erkennung und Ausgabe sind austauschbare Adapter ([`SpeechInput`],
//! [`SpeechOutput`]); im Fork Apples Spracherkennung und Chromiums TTS.
//!
//! „Abbrechen“ ist vorrangig und lokal: Es stoppt die Ausgabe sofort,
//! verwirft eine laufende Erkennung und schließt offene Auswahl und
//! Bestätigung ([`Heard::Cancel`], die Sitzung bekommt „abbrechen“). Der
//! Dialogkontext („nimm das zweite“, „dieses Feld“) kommt aus der Sitzung
//! (Rückfrage, Position), nicht aus einem Modell.

/// Erkennung: liefert Text an den Host; hier nur Abbruch.
pub trait SpeechInput {
    /// Laufende Erkennung verwerfen; true, wenn eine lief.
    fn cancel(&mut self) -> bool;
}

/// Ausgabe.
pub trait SpeechOutput {
    fn speak(&mut self, text: &str);
    /// Sofort verstummen; true, wenn gerade gesprochen wurde.
    fn stop(&mut self) -> bool;
}

/// Was eine erkannte Äußerung bedeutet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heard {
    /// Vorrangiger Abbruch.
    Cancel,
    /// An die Sitzung wie eine getippte Eingabe.
    Command(String),
    /// Nichts verstanden (leer).
    Nothing,
}

/// Wörter, die immer abbrechen, auch mitten in einer Ausgabe.
const CANCEL: &[&str] = &[
    "abbrechen",
    "abbruch",
    "stopp",
    "stop",
    "halt",
    "ruhe",
    "still",
    "nein",
    "cancel",
];

/// Erkannten Text einordnen. Satzzeichen der Erkennung („Stopp.“) zählen
/// nicht.
pub fn route(text: &str) -> Heard {
    let clean = text
        .trim()
        .trim_end_matches(['.', '!', '?'])
        .trim()
        .to_lowercase();
    if clean.is_empty() {
        return Heard::Nothing;
    }
    if CANCEL.contains(&clean.as_str()) {
        return Heard::Cancel;
    }
    Heard::Command(text.trim().trim_end_matches('.').trim().to_string())
}

/// Antwort zum Sprechen: nur die erste Zeile (Listen folgen als Zeilen),
/// ohne Anführungszeichen, höchstens etwa 400 Zeichen bis zum Satzende.
pub fn spoken(answer: &str) -> String {
    let first = answer.lines().next().unwrap_or("").trim();
    let text: String = first
        .chars()
        .filter(|c| !matches!(c, '„' | '“' | '"'))
        .collect();
    if text.chars().count() <= 400 {
        return text;
    }
    let cut: String = text.chars().take(400).collect();
    match cut.rfind(". ") {
        Some(end) => cut[..=end].trim().to_string(),
        None => format!("{cut} …"),
    }
}

/// Ablauf einer Äußerung mit Ein- und Ausgabe; `muted`, solange ein
/// Screenreader spricht (zwei Stimmen sind unbenutzbar).
pub struct SpeechAssist<I, O> {
    pub input: I,
    pub output: O,
    pub muted: bool,
}

impl<I: SpeechInput, O: SpeechOutput> SpeechAssist<I, O> {
    /// Eine Äußerung: Abbruch wirkt sofort auf Aus- und Eingabe; der Text
    /// für die Sitzung kommt zurück (bei Abbruch „abbrechen“) und ob eine
    /// Ausgabe unterbrochen wurde.
    pub fn heard(&mut self, text: &str) -> (Heard, bool) {
        let heard = route(text);
        let stopped = if heard == Heard::Cancel {
            let spoke = self.output.stop();
            let listened = self.input.cancel();
            spoke || listened
        } else {
            // Neue Äußerung: alte Ausgabe nicht weiterreden lassen.
            self.output.stop();
            false
        };
        (heard, stopped)
    }

    /// Antwort der Sitzung ausgeben.
    pub fn answer(&mut self, answer: &str) {
        if !self.muted {
            self.output.speak(&spoken(answer));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        spoken: Vec<String>,
        speaking: bool,
        stops: usize,
    }

    impl SpeechOutput for Recorder {
        fn speak(&mut self, text: &str) {
            self.spoken.push(text.to_string());
            self.speaking = true;
        }
        fn stop(&mut self) -> bool {
            self.stops += 1;
            std::mem::take(&mut self.speaking)
        }
    }

    #[derive(Default)]
    struct Listener {
        running: bool,
    }

    impl SpeechInput for Listener {
        fn cancel(&mut self) -> bool {
            std::mem::take(&mut self.running)
        }
    }

    fn assist() -> SpeechAssist<Listener, Recorder> {
        SpeechAssist {
            input: Listener::default(),
            output: Recorder::default(),
            muted: false,
        }
    }

    #[test]
    fn abbrechen_stoppt_ausgabe_und_erkennung() {
        let mut a = assist();
        assert_eq!(
            a.heard("Was ist auf dieser Seite?").0,
            Heard::Command("Was ist auf dieser Seite?".into())
        );
        a.answer("Seite „Nike Air Max“. Seitentyp Produktseite.\n1. weiter");
        assert_eq!(
            a.output.spoken,
            vec!["Seite Nike Air Max. Seitentyp Produktseite."]
        );
        a.input.running = true;
        assert_eq!(a.heard("Stopp."), (Heard::Cancel, true));
        assert!(!a.output.speaking);
        assert!(!a.input.running);
        // Nichts lief: Abbruch trotzdem, aber nichts unterbrochen.
        assert_eq!(a.heard("abbrechen"), (Heard::Cancel, false));
        assert_eq!(a.heard("  "), (Heard::Nothing, false));
    }

    #[test]
    fn stumm_neben_screenreader() {
        let mut a = assist();
        a.muted = true;
        a.answer("Fertig.");
        assert!(a.output.spoken.is_empty());
    }

    #[test]
    fn lange_antwort_bis_zum_satzende() {
        let long = format!("{}. Rest.", "Wort ".repeat(100).trim());
        let s = spoken(&long);
        assert!(s.chars().count() <= 402, "{}", s.len());
    }
}
