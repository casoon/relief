//! Deterministischer Befehlsparser (Phase 2 ohne LLM).
//!
//! Versteht eine kleine feste Menge deutscher und englischer Formulierungen.
//! Alles andere wird abgelehnt statt geraten.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Command {
    Describe,
    ListActions,
    ListHeadings,
    Focus(String),
    Activate(String),
    /// Ziel, Wert.
    SetValue(String, String),
    /// Ziel (optional, wird sonst über die Option gesucht), Option.
    Select(Option<String>, String),
    /// Offenen Dialog oder aufgeklapptes Menü schließen.
    Dismiss,
    /// Wo steht der Fokus: Element, Bereich, Abschnitt.
    WhereAmI,
    Scroll(ScrollDirection),
    /// Zur nächsten/vorigen Überschrift.
    SectionStep(Step),
    /// Zum nächsten/vorigen Formularfeld.
    FieldStep(Step),
    /// Zu einer Überschrift oder einem Bereich mit diesem Namen.
    GoToPlace(String),
    /// Abschnitt oder Bereich vorlesen; ohne Ziel der am Fokus.
    Read(Option<String>),
    /// Wert, Optionen und Grenzen eines Bedienelements.
    Inspect(String),
    Increment(String),
    Decrement(String),
    /// Offene Dialoge und Cookie-Hinweise mit eingeordneten Buttons.
    Overlays,
    /// Im Cookie-Dialog den Button wählen, der ablehnt (nur auf diesen
    /// ausdrücklichen Befehl; nie zustimmen).
    RejectConsent,
    /// Im Cookie-Dialog die Einstellungen öffnen, hinter denen ein Ablehnen
    /// liegen kann (nur auf diesen ausdrücklichen Befehl).
    ConsentSettings,
    /// Was hinter dem modalen Dialog liegt, nur Auskunft.
    Background,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Step {
    Next,
    Previous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ScrollDirection {
    Down,
    Up,
    Top,
    Bottom,
}

pub fn parse(input: &str) -> Result<Command, String> {
    let text = input.trim().trim_end_matches(['.', '?', '!']).trim();
    let lower = text.to_lowercase();

    const DESCRIBE: &[&str] = &[
        "describe",
        "was ist hier",
        "was ist auf dieser seite",
        "beschreibe die seite",
    ];
    const ACTIONS: &[&str] = &[
        "list actions",
        "was kann ich tun",
        "was kann ich hier tun",
        "was kann ich hier machen",
        "aktionen",
    ];
    const HEADINGS: &[&str] = &[
        "list headings",
        "überschriften",
        "welche überschriften gibt es",
    ];
    if DESCRIBE.contains(&lower.as_str()) {
        return Ok(Command::Describe);
    }
    if ACTIONS.contains(&lower.as_str()) {
        return Ok(Command::ListActions);
    }
    if HEADINGS.contains(&lower.as_str()) {
        return Ok(Command::ListHeadings);
    }
    const DISMISS: &[&str] = &[
        "dismiss",
        "close dialog",
        "escape",
        "schließe den dialog",
        "schließe dialog",
        "dialog schließen",
        "schließe das fenster",
        "schließe das popup",
        "schließe das menü",
        "menü schließen",
        "schließen",
    ];
    if DISMISS.contains(&lower.as_str()) {
        return Ok(Command::Dismiss);
    }
    if lower == "help" || lower == "hilfe" {
        return Ok(Command::Help);
    }
    if let Some(cmd) = parse_fixed(&lower) {
        return Ok(cmd);
    }

    // „gehe zur Überschrift Technische Daten“, „zum Bereich Hauptmenü“ —
    // vor „gehe zu …“, das Bedienelemente sucht.
    if let Some(rest) = strip_any(
        text,
        &[
            "gehe zur überschrift ",
            "zur überschrift ",
            "gehe zum abschnitt ",
            "zum abschnitt ",
            "gehe zum bereich ",
            "zum bereich ",
            "go to heading ",
            "go to section ",
            "go to region ",
        ],
    ) {
        return Ok(Command::GoToPlace(clean(rest)));
    }
    // „lies den Abschnitt Technische Daten“, „lies Versand“
    if let Some(rest) = strip_any(
        text,
        &[
            "lies den abschnitt ",
            "lies abschnitt ",
            "lies den bereich ",
            "lies bereich ",
            "lies ",
            "read section ",
            "read ",
        ],
    ) {
        return Ok(Command::Read(Some(clean(rest))));
    }
    // „welche Größen gibt es“, „details zu Menge“
    if let Some(rest) = strip_any(text, &["welche ", "which ", "what "]) {
        if let Some(target) = strip_suffix_any(
            rest,
            &[
                " gibt es",
                " sind verfügbar",
                " stehen zur auswahl",
                " are there",
                " are available",
            ],
        ) {
            return Ok(Command::Inspect(clean(target)));
        }
    }
    if let Some(rest) = strip_any(text, &["details zu ", "details von ", "inspect "]) {
        return Ok(Command::Inspect(clean(rest)));
    }
    // „erhöhe Menge“, „Menge erhöhen“, „verringere Menge“
    if let Some(rest) = strip_any(text, &["erhöhe ", "increase ", "increment "]) {
        return Ok(Command::Increment(clean(rest)));
    }
    if let Some(target) = strip_suffix_any(text, &[" erhöhen"]) {
        return Ok(Command::Increment(clean(target)));
    }
    if let Some(rest) = strip_any(
        text,
        &[
            "verringere ",
            "reduziere ",
            "senke ",
            "decrease ",
            "decrement ",
        ],
    ) {
        return Ok(Command::Decrement(clean(rest)));
    }
    if let Some(target) = strip_suffix_any(text, &[" verringern", " reduzieren", " senken"]) {
        return Ok(Command::Decrement(clean(target)));
    }

    // „set Suche = Schuhe“, „fülle Suche mit Schuhe“
    if let Some(rest) = strip_any(text, &["set ", "setze "]) {
        if let Some((target, value)) = rest.split_once('=') {
            return Ok(Command::SetValue(clean(target), clean(value)));
        }
    }
    if let Some(rest) = strip_any(text, &["fülle ", "fill "]) {
        if let Some((target, value)) = split_word(rest, &[" mit ", " with "]) {
            return Ok(Command::SetValue(clean(target), clean(value)));
        }
    }

    // „select Größe = 43“, „wähle 43“, „wähle 43 bei Größe“, „nimm 43“
    if let Some(rest) = strip_any(text, &["select "]) {
        return Ok(match rest.split_once('=') {
            Some((target, value)) => Command::Select(Some(clean(target)), clean(value)),
            None => Command::Select(None, clean(rest)),
        });
    }
    if let Some(rest) = strip_any(text, &["wähle ", "nimm "]) {
        let rest = rest
            .trim_start_matches("größe ")
            .trim_start_matches("Größe ");
        return Ok(match split_word(rest, &[" bei ", " in ", " im "]) {
            Some((value, target)) => Command::Select(Some(clean(target)), clean(value)),
            None => Command::Select(None, clean(rest)),
        });
    }

    if let Some(rest) = strip_any(
        text,
        &[
            "focus ",
            "fokus ",
            "gehe zu ",
            "gehe zum ",
            "gehe zur ",
            "springe zu ",
        ],
    ) {
        return Ok(Command::Focus(clean(rest)));
    }
    if let Some(rest) = strip_any(
        text,
        &[
            "activate ",
            "click ",
            "open ",
            "öffne ",
            "klicke ",
            "klicke auf ",
            "drücke ",
            "aktiviere ",
            "schließe ",
        ],
    ) {
        let rest = strip_any(rest, &["auf "]).unwrap_or(rest);
        return Ok(Command::Activate(clean(rest)));
    }

    Err(format!(
        "Nicht verstanden: „{text}“. „hilfe“ zeigt die Befehle."
    ))
}

/// Feste Sätze ohne Ziel (bereits klein geschrieben, ohne Satzzeichen).
fn parse_fixed(lower: &str) -> Option<Command> {
    const FIXED: &[(&[&str], Command)] = &[
        (
            &["wo bin ich", "where am i", "wo ist der fokus"],
            Command::WhereAmI,
        ),
        (
            &[
                "scrolle nach unten",
                "scrolle runter",
                "nach unten scrollen",
                "scroll down",
            ],
            Command::Scroll(ScrollDirection::Down),
        ),
        (
            &[
                "scrolle nach oben",
                "scrolle hoch",
                "nach oben scrollen",
                "scroll up",
            ],
            Command::Scroll(ScrollDirection::Up),
        ),
        (
            &[
                "scrolle zum anfang",
                "scrolle ganz nach oben",
                "zum seitenanfang",
                "scroll to top",
            ],
            Command::Scroll(ScrollDirection::Top),
        ),
        (
            &[
                "scrolle zum ende",
                "scrolle ganz nach unten",
                "zum seitenende",
                "scroll to bottom",
            ],
            Command::Scroll(ScrollDirection::Bottom),
        ),
        (
            &[
                "gehe zum nächsten abschnitt",
                "zum nächsten abschnitt",
                "nächster abschnitt",
                "gehe zur nächsten überschrift",
                "nächste überschrift",
                "next section",
                "next heading",
            ],
            Command::SectionStep(Step::Next),
        ),
        (
            &[
                "gehe zum vorherigen abschnitt",
                "zum vorherigen abschnitt",
                "vorheriger abschnitt",
                "gehe zur vorherigen überschrift",
                "vorherige überschrift",
                "previous section",
                "previous heading",
            ],
            Command::SectionStep(Step::Previous),
        ),
        (
            &[
                "gehe zum nächsten formularfeld",
                "zum nächsten formularfeld",
                "nächstes formularfeld",
                "gehe zum nächsten feld",
                "nächstes feld",
                "next field",
                "next form field",
            ],
            Command::FieldStep(Step::Next),
        ),
        (
            &[
                "gehe zum vorherigen formularfeld",
                "zum vorherigen formularfeld",
                "vorheriges formularfeld",
                "gehe zum vorherigen feld",
                "vorheriges feld",
                "previous field",
                "previous form field",
            ],
            Command::FieldStep(Step::Previous),
        ),
        (
            &[
                "welcher dialog ist offen",
                "was ist das für ein dialog",
                "beschreibe den dialog",
                "cookie-dialog",
                "cookie dialog",
                "cookies",
                "which dialog is open",
                "describe dialog",
            ],
            Command::Overlays,
        ),
        (
            &[
                "cookies ablehnen",
                "lehne cookies ab",
                "lehne die cookies ab",
                "lehne ab",
                "ablehnen",
                "alle ablehnen",
                "einwilligung ablehnen",
                "lehne die einwilligung ab",
                "reject cookies",
                "reject all",
                "decline cookies",
            ],
            Command::RejectConsent,
        ),
        (
            &[
                "cookie-einstellungen öffnen",
                "cookie-einstellungen",
                "öffne die cookie-einstellungen",
                "zu den cookie-einstellungen",
                "gehe zu den cookie-einstellungen",
                "cookie einstellungen öffnen",
                "cookie einstellungen",
                "öffne die cookie einstellungen",
                "open cookie settings",
                "cookie settings",
            ],
            Command::ConsentSettings,
        ),
        (
            &[
                "was ist hinter dem dialog",
                "was liegt hinter dem dialog",
                "was steht hinter dem dialog",
                "was ist im hintergrund",
                "beschreibe den hintergrund",
                "hintergrund",
                "what is behind the dialog",
            ],
            Command::Background,
        ),
        (
            &[
                "lies den abschnitt",
                "lies diesen abschnitt",
                "lies vor",
                "read section",
                "read this section",
            ],
            Command::Read(None),
        ),
    ];
    FIXED
        .iter()
        .find(|(phrases, _)| phrases.contains(&lower))
        .map(|(_, cmd)| cmd.clone())
}

pub const HELP: &str = "Befehle: was ist hier · wo bin ich · was kann ich tun · überschriften · \
gehe zu <Ziel> · gehe zur Überschrift <Name> · nächster/vorheriger Abschnitt · \
nächstes/vorheriges Formularfeld · lies den Abschnitt [<Name>] · welche <Optionen> gibt es · \
öffne <Ziel> · schließe den Dialog · fülle <Feld> mit <Wert> · wähle <Option> [bei <Feld>] · \
erhöhe/verringere <Feld> · scrolle nach unten/oben/zum Anfang/zum Ende · \
welcher Dialog ist offen · cookies ablehnen · cookie-einstellungen öffnen · \
was ist hinter dem Dialog";

fn strip_any<'a>(text: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    let lower = text.to_lowercase();
    prefixes
        .iter()
        .filter(|p| lower.starts_with(*p))
        .max_by_key(|p| p.len())
        // Präfixe sind ASCII oder bleiben in Klein-/Großschreibung gleich lang.
        .map(|p| &text[p.len()..])
}

fn strip_suffix_any<'a>(text: &'a str, suffixes: &[&str]) -> Option<&'a str> {
    let lower = text.to_lowercase();
    suffixes
        .iter()
        .find(|s| lower.ends_with(*s))
        // wie bei `strip_any`: gleiche Länge in Klein- und Großschreibung.
        .map(|s| &text[..text.len() - s.len()])
}

fn split_word<'a>(text: &'a str, words: &[&str]) -> Option<(&'a str, &'a str)> {
    let lower = text.to_lowercase();
    words
        .iter()
        .find_map(|w| lower.find(w).map(|i| (&text[..i], &text[i + w.len()..])))
}

/// Artikel und Anführungszeichen weg: „den Warenkorb“ → „Warenkorb“.
fn clean(s: &str) -> String {
    let s = s.trim().trim_matches(['"', '„', '“', '\'']);
    let lower = s.to_lowercase();
    for article in ["den ", "die ", "das ", "dem ", "der ", "the "] {
        if lower.starts_with(article) {
            return s[article.len()..].trim().to_string();
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_phrases() {
        assert_eq!(parse("Was kann ich hier tun?"), Ok(Command::ListActions));
        assert_eq!(
            parse("Öffne den Warenkorb."),
            Ok(Command::Activate("Warenkorb".into()))
        );
        assert_eq!(
            parse("Wähle Größe 43"),
            Ok(Command::Select(None, "43".into()))
        );
        assert_eq!(
            parse("wähle M bei Größe"),
            Ok(Command::Select(Some("Größe".into()), "M".into()))
        );
        assert_eq!(
            parse("fülle E-Mail mit a@b.de"),
            Ok(Command::SetValue("E-Mail".into(), "a@b.de".into()))
        );
        assert_eq!(
            parse("klicke auf Senden"),
            Ok(Command::Activate("Senden".into()))
        );
        assert_eq!(parse("Schließe den Dialog."), Ok(Command::Dismiss));
        assert_eq!(
            parse("schließe Größentabelle"),
            Ok(Command::Activate("Größentabelle".into()))
        );
        assert!(parse("überweise 500 euro").is_err());
    }

    #[test]
    fn parses_where_am_i() {
        assert_eq!(parse("Wo bin ich?"), Ok(Command::WhereAmI));
        assert_eq!(parse("where am I"), Ok(Command::WhereAmI));
        assert_eq!(parse("Wo ist der Fokus?"), Ok(Command::WhereAmI));
    }

    #[test]
    fn parses_scroll() {
        use ScrollDirection::*;
        assert_eq!(parse("Scrolle nach unten."), Ok(Command::Scroll(Down)));
        assert_eq!(parse("nach unten scrollen"), Ok(Command::Scroll(Down)));
        assert_eq!(parse("scroll down"), Ok(Command::Scroll(Down)));
        assert_eq!(parse("Scrolle nach oben"), Ok(Command::Scroll(Up)));
        assert_eq!(parse("scroll up"), Ok(Command::Scroll(Up)));
        assert_eq!(parse("Scrolle zum Anfang"), Ok(Command::Scroll(Top)));
        assert_eq!(parse("zum Seitenanfang"), Ok(Command::Scroll(Top)));
        assert_eq!(parse("Scrolle zum Ende"), Ok(Command::Scroll(Bottom)));
        assert_eq!(parse("scroll to bottom"), Ok(Command::Scroll(Bottom)));
    }

    #[test]
    fn parses_section_navigation() {
        assert_eq!(
            parse("Gehe zum nächsten Abschnitt."),
            Ok(Command::SectionStep(Step::Next))
        );
        assert_eq!(
            parse("nächste Überschrift"),
            Ok(Command::SectionStep(Step::Next))
        );
        assert_eq!(parse("next heading"), Ok(Command::SectionStep(Step::Next)));
        assert_eq!(
            parse("vorheriger Abschnitt"),
            Ok(Command::SectionStep(Step::Previous))
        );
        assert_eq!(
            parse("previous section"),
            Ok(Command::SectionStep(Step::Previous))
        );
        assert_eq!(
            parse("Gehe zur Überschrift Technische Daten"),
            Ok(Command::GoToPlace("Technische Daten".into()))
        );
        assert_eq!(
            parse("zum Bereich Hauptmenü"),
            Ok(Command::GoToPlace("Hauptmenü".into()))
        );
        assert_eq!(
            parse("go to heading Shipping"),
            Ok(Command::GoToPlace("Shipping".into()))
        );
        // „gehe zu …“ ohne Überschrift/Bereich bleibt Fokus auf ein Bedienelement.
        assert_eq!(parse("gehe zu Suche"), Ok(Command::Focus("Suche".into())));
    }

    #[test]
    fn parses_field_navigation() {
        assert_eq!(
            parse("Gehe zum nächsten Formularfeld."),
            Ok(Command::FieldStep(Step::Next))
        );
        assert_eq!(parse("nächstes Feld"), Ok(Command::FieldStep(Step::Next)));
        assert_eq!(parse("next field"), Ok(Command::FieldStep(Step::Next)));
        assert_eq!(
            parse("vorheriges Formularfeld"),
            Ok(Command::FieldStep(Step::Previous))
        );
        assert_eq!(
            parse("previous form field"),
            Ok(Command::FieldStep(Step::Previous))
        );
    }

    #[test]
    fn parses_read_region() {
        assert_eq!(
            parse("Lies den Abschnitt technische Daten."),
            Ok(Command::Read(Some("technische Daten".into())))
        );
        assert_eq!(
            parse("lies den Bereich Hauptmenü"),
            Ok(Command::Read(Some("Hauptmenü".into())))
        );
        assert_eq!(
            parse("lies Versand"),
            Ok(Command::Read(Some("Versand".into())))
        );
        assert_eq!(
            parse("read section Shipping"),
            Ok(Command::Read(Some("Shipping".into())))
        );
        assert_eq!(parse("Lies den Abschnitt"), Ok(Command::Read(None)));
        assert_eq!(parse("lies vor"), Ok(Command::Read(None)));
        assert_eq!(parse("read this section"), Ok(Command::Read(None)));
    }

    #[test]
    fn parses_inspect_control() {
        assert_eq!(
            parse("Welche Größen gibt es?"),
            Ok(Command::Inspect("Größen".into()))
        );
        assert_eq!(
            parse("welche Farben sind verfügbar"),
            Ok(Command::Inspect("Farben".into()))
        );
        assert_eq!(
            parse("which sizes are available"),
            Ok(Command::Inspect("sizes".into()))
        );
        assert_eq!(
            parse("Details zu Menge"),
            Ok(Command::Inspect("Menge".into()))
        );
        assert_eq!(
            parse("inspect quantity"),
            Ok(Command::Inspect("quantity".into()))
        );
        // Die feste Abfrage nach Überschriften hat Vorrang.
        assert_eq!(
            parse("Welche Überschriften gibt es?"),
            Ok(Command::ListHeadings)
        );
    }

    #[test]
    fn parses_overlay_commands() {
        assert_eq!(parse("Welcher Dialog ist offen?"), Ok(Command::Overlays));
        assert_eq!(parse("Cookie-Dialog"), Ok(Command::Overlays));
        assert_eq!(parse("Cookies ablehnen."), Ok(Command::RejectConsent));
        assert_eq!(parse("Lehne die Cookies ab"), Ok(Command::RejectConsent));
        assert_eq!(parse("reject all"), Ok(Command::RejectConsent));
        assert_eq!(
            parse("Cookie-Einstellungen öffnen"),
            Ok(Command::ConsentSettings)
        );
        assert_eq!(parse("cookie settings"), Ok(Command::ConsentSettings));
        assert_eq!(parse("Was ist hinter dem Dialog?"), Ok(Command::Background));
        // Zustimmen ist kein eigener Befehl; „klicke …“ bleibt ein Klick der
        // Nutzerin auf einen genannten Button.
        assert!(parse("cookies akzeptieren").is_err());
    }

    #[test]
    fn parses_increment_decrement() {
        assert_eq!(
            parse("Erhöhe die Menge"),
            Ok(Command::Increment("Menge".into()))
        );
        assert_eq!(
            parse("Menge erhöhen."),
            Ok(Command::Increment("Menge".into()))
        );
        assert_eq!(
            parse("increase quantity"),
            Ok(Command::Increment("quantity".into()))
        );
        assert_eq!(
            parse("verringere Menge"),
            Ok(Command::Decrement("Menge".into()))
        );
        assert_eq!(
            parse("Menge verringern"),
            Ok(Command::Decrement("Menge".into()))
        );
        assert_eq!(
            parse("reduziere die Sohlenhärte"),
            Ok(Command::Decrement("Sohlenhärte".into()))
        );
        assert_eq!(
            parse("decrease volume"),
            Ok(Command::Decrement("volume".into()))
        );
    }
}
