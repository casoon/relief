//! Snapshot-Tests über die Aufnahmen aus `spike/recordings`: je Aufnahme
//! eine Zusammenfassung des Graphs, verglichen mit
//! `tests/erwartungen/<aufgabe>/<seite>.json`, und Graph-Stabilität.
//!
//! Ändert eine Heuristik den Graph, schlägt der Vergleich fehl und zeigt,
//! welche Seiten sich wie ändern. Ist die Änderung gewollt:
//!
//! ```bash
//! RELIEF_ERWARTUNGEN=neu cargo test -p relief-interaction --test recordings
//! git diff crates/relief-interaction/tests/erwartungen
//! ```

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use common::{graph, kurz, seiten, Seite};
use relief_interaction::{respond, PageType};
use relief_model::{Certainty, Fact};
use serde::Serialize;

/// Höchstens so viele Einträge je Liste; der Rest wird gezählt.
const MAX_LISTE: usize = 10;

/// Rollen, die als Formularfeld gelten (wie `resolve::step_field`).
const FELDER: &[&str] = &[
    "textbox",
    "searchbox",
    "combobox",
    "listbox",
    "checkbox",
    "radio",
    "switch",
    "slider",
    "spinbutton",
    "PopUpButton",
];

/// Was an einem Graph für Menschen und Heuristiken zählt.
#[derive(Debug, Serialize)]
struct Zusammenfassung {
    titel: Option<String>,
    /// Bereiche in Dokumentreihenfolge (`Region::label`).
    bereiche: Vec<String>,
    /// Zuletzt geöffneter modaler Dialog; er sperrt den Rest seines
    /// Dokuments (Frames), nicht das Elterndokument.
    modal: Option<String>,
    ueberschriften: usize,
    h1: Vec<String>,
    bedienelemente: usize,
    /// Davon gerade erreichbar (`Graph::reachable_controls`).
    erreichbar: usize,
    /// Herkunft der Namen: Known / Inferred / Uncertain.
    namen: BTreeMap<&'static str, usize>,
    texte: usize,
    /// Bedienelemente ohne gesicherten Namen, mit Evidence.
    ohne_gesicherten_namen: Vec<String>,
    /// Formularfelder mit Wert, Optionen und Zuständen.
    felder: Vec<String>,
    /// Bedienelemente mit `hasPopup`.
    popups: Vec<String>,
    /// Seitentyp mit Herkunft und Evidence.
    seitentyp: String,
    /// Primäre Aktion der Seite mit Herkunft und Evidence.
    primaere_aktion: Option<String>,
    /// Funktionale Gruppen: Art, Name, Größe, primäre Aktion.
    gruppen: Vec<String>,
}

/// `Wert (Certainty): Evidence; …` für erschlossene Aussagen.
fn aussage<T>(fact: &Fact<T>, wert: &str) -> String {
    format!(
        "{wert} ({:?}): {}",
        fact.certainty,
        fact.evidence.join("; ")
    )
}

fn primaer(g: &relief_interaction::Graph, p: &Option<Fact<usize>>) -> Option<String> {
    let p = p.as_ref()?;
    Some(aussage(p, &kurz(&g.controls[p.value?])))
}

fn zusammenfassung(g: &relief_interaction::Graph) -> Zusammenfassung {
    let mut namen = BTreeMap::from([("Known", 0), ("Inferred", 0), ("Uncertain", 0)]);
    for c in &g.controls {
        let key = match c.name.certainty {
            Certainty::Known => "Known",
            Certainty::Inferred => "Inferred",
            Certainty::Uncertain => "Uncertain",
        };
        *namen.get_mut(key).unwrap() += 1;
    }
    let liste = |filter: &dyn Fn(&relief_interaction::Control) -> bool| {
        let all: Vec<String> = g
            .controls
            .iter()
            .filter(|c| filter(c))
            .map(respond::control_line)
            .collect();
        let mut out: Vec<String> = all.iter().take(MAX_LISTE).cloned().collect();
        if all.len() > MAX_LISTE {
            out.push(format!("… und {} weitere", all.len() - MAX_LISTE));
        }
        out
    };
    Zusammenfassung {
        titel: g.title.clone(),
        bereiche: g.regions.iter().map(|r| r.label()).collect(),
        modal: g.active_modal().map(|m| g.regions[m].label()),
        ueberschriften: g.headings.len(),
        h1: g
            .headings
            .iter()
            .filter(|h| h.level == 1)
            .map(|h| h.text.clone())
            .collect(),
        bedienelemente: g.controls.len(),
        erreichbar: g.reachable_controls().count(),
        namen,
        texte: g.texts.len(),
        ohne_gesicherten_namen: liste(&|c| c.name.certainty != Certainty::Known),
        felder: liste(&|c| FELDER.contains(&c.role.as_str())),
        popups: liste(&|c| c.states.iter().any(|(k, _)| k == "hasPopup")),
        seitentyp: aussage(&g.page.kind, &format!("{:?}", g.page.kind.value.unwrap())),
        primaere_aktion: primaer(g, &g.page.primary),
        gruppen: g
            .page
            .groups
            .iter()
            .map(|gr| {
                format!(
                    "{:?} „{}“: {} Bedienelemente, primär {}",
                    gr.kind,
                    gr.name.as_deref().unwrap_or("unbenannt"),
                    gr.controls.len(),
                    primaer(g, &gr.primary).unwrap_or_else(|| "keine".into())
                )
            })
            .collect(),
    }
}

fn erwartungen() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/erwartungen")
}

/// Zusammenfassungen aller Aufnahmen einer Seite als JSON, je Datei ein
/// Schlüssel.
fn seite_als_json(seite: &Seite) -> String {
    let mut all = BTreeMap::new();
    for datei in seite.dateien() {
        let snap = seite.aufnahme(&datei);
        assert!(!snap.tree.is_empty(), "{}/{datei}: leerer Baum", seite.name);
        all.insert(datei, zusammenfassung(&graph(&snap)));
    }
    serde_json::to_string_pretty(&all).unwrap() + "\n"
}

#[test]
fn zusammenfassung_je_aufnahme_wie_erwartet() {
    let neu = std::env::var("RELIEF_ERWARTUNGEN").as_deref() == Ok("neu");
    let seiten = seiten();
    assert!(!seiten.is_empty(), "keine Aufnahmen gefunden");
    let mut abweichend = Vec::new();
    for seite in &seiten {
        // Die Anfangsaufnahme jeder Seite hat Bedienelemente; spätere
        // dürfen leer sein (Fehlerseite nach Navigation in 02-shop-broken).
        assert!(
            !graph(&seite.anfang()).controls.is_empty(),
            "{}: keine Bedienelemente",
            seite.name
        );
        let ist = seite_als_json(seite);
        let pfad = erwartungen().join(format!("{}.json", seite.name));
        if neu {
            std::fs::create_dir_all(pfad.parent().unwrap()).unwrap();
            std::fs::write(&pfad, &ist).unwrap();
            continue;
        }
        let soll = std::fs::read_to_string(&pfad).unwrap_or_default();
        if soll != ist {
            let zeilen: Vec<String> = soll
                .lines()
                .zip(ist.lines())
                .filter(|(s, i)| s != i)
                .take(5)
                .map(|(s, i)| format!("    soll: {s}\n    ist:  {i}"))
                .collect();
            abweichend.push(format!(
                "{} ({} Zeilen soll, {} ist)\n{}",
                seite.name,
                soll.lines().count(),
                ist.lines().count(),
                zeilen.join("\n")
            ));
        }
    }
    assert!(
        abweichend.is_empty(),
        "Graph weicht von der Erwartung ab. Gewollt? RELIEF_ERWARTUNGEN=neu \
         setzt die Erwartungen neu (Diff prüfen).\n{}",
        abweichend.join("\n")
    );
}

/// Keine Erwartungsdatei ohne Aufnahme (umbenannte oder gelöschte Seiten).
#[test]
fn keine_verwaisten_erwartungen() {
    let seiten: Vec<String> = seiten().into_iter().map(|s| s.name).collect();
    for aufgabe in std::fs::read_dir(erwartungen()).unwrap() {
        let aufgabe = aufgabe.unwrap().path();
        for datei in std::fs::read_dir(&aufgabe).unwrap() {
            let datei = datei.unwrap().path();
            let name = format!(
                "{}/{}",
                aufgabe.file_name().unwrap().to_string_lossy(),
                datei.file_stem().unwrap().to_string_lossy()
            );
            assert!(seiten.contains(&name), "verwaist: {}", datei.display());
        }
    }
}

fn signatur(seite: &str, datei: &str) -> Vec<String> {
    graph(&Seite::laden(seite).aufnahme(datei)).signature()
}

const SHOP: &str = "01-shop-clean/01-shop-clean";
const DIALOG_1: &str = "10-real/04-www-w3-org-wai-aria-apg-patterns-dialog-modal-examples-dialog";
const DIALOG_2: &str = "10-real/06-www-w3-org-wai-aria-apg-patterns-dialog-modal-examples-dialog";
const MENU: &str =
    "10-real/05-www-w3-org-wai-aria-apg-patterns-menu-button-examples-menu-button-links";

/// Dieselbe Seite, zweimal geladen und aufgenommen: gleiche Signatur,
/// obwohl Chrome die `nodeId`s neu vergibt.
#[test]
fn gleiche_seite_zweimal_geladen_gleiche_signatur() {
    assert_eq!(
        signatur("02-shop-broken/01-shop-broken", "snapshot-00.json"),
        signatur("02-shop-broken/02-shop-broken", "snapshot-00.json")
    );
    // APG-Dialogseite in zwei Läufen. `06/snapshot-00` ist die
    // Anfangsaufnahme vor dem Nachladen (ohne die zwei „Open In
    // CodePen“-Buttons); verglichen wird der geladene Stand.
    assert_eq!(
        signatur(DIALOG_1, "snapshot-00.json"),
        signatur(DIALOG_2, "snapshot-01.json")
    );
    // Und mit offenem Dialog.
    assert_eq!(
        signatur(DIALOG_1, "snapshot-01.json"),
        signatur(DIALOG_2, "snapshot-02.json")
    );
}

/// Öffnen und wieder schließen führt zur Signatur vorher zurück.
#[test]
fn oeffnen_und_schliessen_gleiche_signatur() {
    // Größentabelle (natives `<dialog>`).
    assert_eq!(
        signatur(SHOP, "snapshot-02.json"),
        signatur(SHOP, "snapshot-04.json")
    );
    // APG-Dialog (`aria-modal`).
    assert_eq!(
        signatur(DIALOG_2, "snapshot-01.json"),
        signatur(DIALOG_2, "snapshot-03.json")
    );
    // APG-Menübutton.
    assert_eq!(
        signatur(MENU, "snapshot-00.json"),
        signatur(MENU, "snapshot-02.json")
    );
}

/// Schritte, die nur Fokus, Wert oder Zustand ändern, lassen die Signatur
/// gleich; Schritte, die Bedienelemente hinzufügen, entfernen oder
/// umbenennen, nicht.
#[test]
fn signatur_je_schritt() {
    let faelle: &[(&str, &str, bool)] = &[
        (SHOP, "Wähle 43", true),
        (SHOP, "gehe zu Suche", true),
        (SHOP, "fülle Suche mit Laufschuhe", true),
        (SHOP, "!klicke Jetzt kaufen", true),
        // „Warenkorb (0)“ → „Warenkorb (1)“: Name geändert.
        (SHOP, "klicke In den Warenkorb", false),
        (SHOP, "Öffne die Größentabelle", false),
        ("03-form/01-form", "fülle Name mit Erika Muster", true),
        ("03-form/01-form", "!klicke Nachricht senden", true),
        ("03-form/01-form", "klicke Datenschutzhinweise", true),
        ("04-iframe/01-with-iframe", "klicke Nur notwendige", true),
        ("10-real/01-www-gov-uk", "gehe zu Search", true),
        // Vorschlagsliste erscheint.
        ("10-real/01-www-gov-uk", "fülle Search mit passport", false),
        (
            "10-real/03-www-w3-org-wai-aria-apg-patterns-disclosure-examples-disclosure-faq",
            "öffne Is there free parking on holidays",
            true,
        ),
        (MENU, "öffne WAI-ARIA Quick Links", false),
        (DIALOG_2, "öffne Add Delivery Address", false),
    ];
    for (seite, eingabe, gleich) in faelle {
        let s = Seite::laden(seite);
        let (vorher, nachher) = s.paar(eingabe, 0);
        assert_eq!(
            graph(&vorher).signature() == graph(&nachher).signature(),
            *gleich,
            "{seite}: „{eingabe}“"
        );
    }
}

/// Von Hand festgelegte Soll-Werte je Seite (Anfangsaufnahme): Seitentyp und
/// Name der primären Aktion der Seite. Startseiten sind kein eigener Typ
/// (→ `plan/spezifikation/04`) und damit `Unknown`; die APG-Beispielseiten
/// sind Dokumentation zum Lesen und gelten als `Article`.
const SOLL: &[(&str, PageType, Option<&str>)] = &[
    (SHOP, PageType::Product, Some("In den Warenkorb")),
    // Produktseite, deren Button ein `div` ist: Typ ja, Aktion nicht
    // bedienbar.
    ("02-shop-broken/01-shop-broken", PageType::Product, None),
    ("02-shop-broken/02-shop-broken", PageType::Product, None),
    ("03-form/01-form", PageType::Form, Some("Nachricht senden")),
    ("04-iframe/01-with-iframe", PageType::Unknown, None),
    ("10-real/01-www-gov-uk", PageType::Unknown, None),
    (
        "10-real/02-en-wikipedia-org-wiki-accessibility",
        PageType::Article,
        None,
    ),
    (
        "10-real/03-www-w3-org-wai-aria-apg-patterns-disclosure-examples-disclosure-faq",
        PageType::Article,
        None,
    ),
    (DIALOG_1, PageType::Article, None),
    (MENU, PageType::Article, None),
    (DIALOG_2, PageType::Article, None),
    ("11-korpus/01-www-casoon-de", PageType::Unknown, None),
    ("11-korpus/02-insights-casoon-de", PageType::Unknown, None),
];

/// Seiten, deren Ist vom Soll abweicht, mit dem Ist: (Seite, Typ, Aktion).
/// Jede Zeile ist eine benannte Fehlklassifikation.
const BEKANNT_FALSCH: &[(&str, PageType, Option<&str>)] = &[];

/// Trefferquote der Heuristiken gegen die Soll-Werte. Ausgabe mit
/// `cargo test -p relief-interaction --test recordings trefferquote -- --nocapture`.
#[test]
fn trefferquote_seitentyp_und_primaere_aktion() {
    let seiten: Vec<String> = seiten().into_iter().map(|s| s.name).collect();
    assert_eq!(SOLL.len(), seiten.len(), "Soll-Wert je Seite");
    let mut falsch = Vec::new();
    let (mut typ_ok, mut aktion_ok) = (0, 0);
    for (seite, typ, aktion) in SOLL {
        assert!(
            seiten.contains(&seite.to_string()),
            "{seite}: keine Aufnahme"
        );
        let g = graph(&Seite::laden(seite).anfang());
        let ist_typ = g.page.kind.value.unwrap();
        assert_ne!(g.page.kind.certainty, Certainty::Known, "{seite}");
        let ist_aktion = g
            .page
            .primary
            .as_ref()
            .map(|p| g.controls[p.value.unwrap()].display_name());
        typ_ok += usize::from(ist_typ == *typ);
        aktion_ok += usize::from(ist_aktion.as_deref() == *aktion);
        println!(
            "{seite}: soll {typ:?}/{aktion:?}, ist {ist_typ:?} ({:?})/{ist_aktion:?}",
            g.page.kind.certainty
        );
        if ist_typ != *typ || ist_aktion.as_deref() != *aktion {
            falsch.push((seite.to_string(), ist_typ, ist_aktion));
        }
    }
    println!(
        "Seitentyp {typ_ok}/{n}, primäre Aktion {aktion_ok}/{n}",
        n = SOLL.len()
    );
    let bekannt: Vec<(String, PageType, Option<String>)> = BEKANNT_FALSCH
        .iter()
        .map(|(s, t, a)| (s.to_string(), *t, a.map(String::from)))
        .collect();
    assert_eq!(falsch, bekannt, "Fehlklassifikationen geändert");
}
