//! Die Aufgaben aus `spike/tasks/` browserfrei nachgespielt: Eingabe →
//! `parse` → `resolve` → `plan` bzw. Antworttext, Wirkung über die
//! aufgezeichneten Vorher/Nachher-Paare (als Modell über
//! `relief_model::perception`, `respond::describe_diff`). Dazu die Befunde des CDP-Spikes
//! (`plan/spezifikation/09`, Ergebnis) als Tests.
//!
//! Die Erwartungen sind hier festgelegt, nicht aus den aufgezeichneten
//! Antworten in `index.json` übernommen (die stammen aus älterem Code).
//! Aufgaben aus `05-intents.txt` sind nicht aufgenommen; sie decken die
//! Unit-Tests der Module über `graph::sample_tree` ab.

mod common;

use a11y_perception::AXSnapshot;
use common::{eins, felder_mit_option, graph, kurz, modell, ziel, Seite};
use relief_interaction::{
    dismissal, focused, parse, plan, respond, ActionKind, Command, Dismissal, Graph, Resolution,
    Risk,
};
use relief_model::{Certainty, Role};

fn diff(vorher: &AXSnapshot, nachher: &AXSnapshot) -> String {
    respond::describe_diff(&modell(vorher), &modell(nachher))
}

/// Eingabe parsen; ein Parserfehler ist ein Testfehler.
fn befehl(eingabe: &str) -> Command {
    parse(eingabe).unwrap_or_else(|e| panic!("„{eingabe}“: {e}"))
}

/// Schließen-Ziel wie im CDP-Host: mit dem aufgenommenen Fokus.
fn dismissal_text(snap: &AXSnapshot) -> String {
    let m = modell(snap);
    let g = Graph::build(&m);
    match dismissal(&g, &m, focused(&m).as_ref()) {
        Dismissal::Button(c) => kurz(c),
        Dismissal::Escape { target, reaches } => match reaches {
            None => format!("Escape: {target}"),
            Some(r) => format!("Escape: {target}, erreicht aber {r}"),
        },
        Dismissal::NothingOpen => "nichts offen".into(),
    }
}

// ---------------------------------------------------------------------------
// 01-shop-clean: der Known-Pfad

const SHOP: &str = "01-shop-clean/01-shop-clean";

#[test]
fn shop_clean_abfragen() {
    let s = Seite::laden(SHOP);
    let g = graph(&s.aufnahme(&s.schritt("Was ist auf dieser Seite?", 0).vorher));
    assert_eq!(befehl("Was ist auf dieser Seite?"), Command::Describe);
    assert_eq!(
        respond::describe(&g),
        "Seite „Nike Air Max – Testshop“. Seitentyp vermutlich Produktseite (erschlossen: \
         Button „In den Warenkorb“, Preis „129,00 €“, Hauptüberschrift „Nike Air Max“). \
         Hauptüberschrift: „Nike Air Max“. Bereiche: banner, navigation „Hauptmenü“, search, main, region „Technische Daten“, \
         contentinfo. 2 Überschriften, 12 Bedienelemente."
    );
    assert_eq!(befehl("Was kann ich hier tun?"), Command::ListActions);
    assert_eq!(
        respond::list_actions(&g),
        "navigation „Hauptmenü“:\n  - [link] Home\n  - [link] Produkte\n  - [link] Warenkorb (0)\n  \
         - [link] Konto\nsearch:\n  - [searchbox] Suche\n  - [button] Suchen\nmain:\n  \
         - [combobox] Größe Optionen: 39, 40, 41, 42, 43; gewählt: 42 hasPopup=menu\n  \
         - [button] In den Warenkorb\n  - [button] Größentabelle\n  - [button] Jetzt kaufen\n\
         contentinfo:\n  - [link] Impressum\n  - [link] Datenschutz"
    );
    assert_eq!(
        befehl("Welche Überschriften gibt es?"),
        Command::ListHeadings
    );
    assert_eq!(
        respond::list_headings(&g),
        "H1 Nike Air Max\n  H2 Technische Daten"
    );
}

#[test]
fn shop_clean_auswahl_ueber_option() {
    let s = Seite::laden(SHOP);
    let (vorher, nachher) = s.paar("Wähle 43", 0);
    let g = graph(&vorher);
    assert_eq!(befehl("Wähle 43"), Command::Select(None, "43".into()));
    let felder = felder_mit_option(&g, "43");
    assert_eq!(
        felder.iter().map(|c| kurz(c)).collect::<Vec<_>>(),
        ["[combobox] Größe"]
    );
    let p = plan(felder[0], ActionKind::Select("43".into())).unwrap();
    assert_eq!(
        (p.kind, p.risk, p.requires_confirmation),
        (ActionKind::Select("43".into()), Risk::Medium, false)
    );
    assert_eq!(
        diff(&vorher, &nachher),
        "Fokus jetzt auf combobox „Größe“. „42“: selected true → false. „43“: selected false → true."
    );
    let danach = graph(&nachher);
    let groesse = danach
        .controls
        .iter()
        .find(|c| c.role == Role::Combobox)
        .unwrap();
    assert_eq!(groesse.selected_option.as_deref(), Some("43"));
}

/// Befund 6: „Öffne den Warenkorb“ trifft Link und Button gleich gut und
/// fragt nach, statt zu raten.
#[test]
fn befund_mehrdeutigkeit_warenkorb() {
    let s = Seite::laden(SHOP);
    let g = graph(&s.aufnahme(&s.schritt("Öffne den Warenkorb", 0).vorher));
    let cmd = befehl("Öffne den Warenkorb");
    assert_eq!(cmd, Command::Activate("Warenkorb".into()));
    let Some(Resolution::Many(treffer)) = ziel(&g, &cmd) else {
        panic!("„Warenkorb“ muss mehrdeutig sein");
    };
    let treffer: Vec<String> = treffer
        .iter()
        .map(|c| format!("{} in {}", kurz(c), g.region_label(c.region)))
        .collect();
    assert_eq!(
        treffer,
        [
            "[link] Warenkorb (0) in navigation „Hauptmenü“",
            "[button] In den Warenkorb in main"
        ]
    );
    // Mit vollem Namen eindeutig.
    let button = eins(ziel(&g, &befehl("klicke In den Warenkorb")));
    assert_eq!(kurz(button), "[button] In den Warenkorb");
}

#[test]
fn shop_clean_in_den_warenkorb() {
    let s = Seite::laden(SHOP);
    let (vorher, nachher) = s.paar("klicke In den Warenkorb", 0);
    let g = graph(&vorher);
    let c = eins(ziel(&g, &befehl("klicke In den Warenkorb")));
    let p = plan(c, ActionKind::Activate).unwrap();
    assert_eq!((p.risk, p.requires_confirmation), (Risk::Medium, false));
    assert_eq!(
        diff(&vorher, &nachher),
        "Fokus jetzt auf button „In den Warenkorb“. \
         Neuer Text: „Warenkorb (1)“, „Größe 43 in den Warenkorb gelegt.“."
    );
}

/// Befund 7 und Modalität: nativer modaler Dialog öffnen, nur sein Inhalt ist
/// erreichbar, Schließen über den Schließen-Button, danach nichts mehr offen.
#[test]
fn befund_dialog_oeffnen_und_schliessen() {
    let s = Seite::laden(SHOP);
    let (vorher, offen) = s.paar("Öffne die Größentabelle", 0);
    let g = graph(&vorher);
    assert_eq!(dismissal_text(&vorher), "nichts offen");
    let c = eins(ziel(&g, &befehl("Öffne die Größentabelle")));
    assert_eq!(kurz(c), "[button] Größentabelle");
    assert_eq!(
        diff(&vorher, &offen),
        "Fokus jetzt auf button „Schließen“. Dialog „Größentabelle“ geöffnet. \
         Rest der Seite ist gesperrt, solange er offen ist."
    );

    let (offen2, zu) = s.paar("Schließe den Dialog", 0);
    assert_eq!(offen2.tree.len(), offen.tree.len());
    let g = graph(&offen);
    assert_eq!(
        g.active_modal().map(|m| g.regions[m].label()).as_deref(),
        Some("dialog „Größentabelle“")
    );
    assert_eq!(befehl("Schließe den Dialog"), Command::Dismiss);
    let m = modell(&offen);
    let Dismissal::Button(schliessen) = dismissal(&g, &m, focused(&m).as_ref()) else {
        panic!("Schließen-Button erwartet");
    };
    assert_eq!(kurz(schliessen), "[button] Schließen");
    let p = plan(schliessen, ActionKind::Activate).unwrap();
    assert!(!p.requires_confirmation);
    // Der Rest der Seite ist wieder wahrnehmbar; die ersten Texte in
    // Dokumentreihenfolge.
    assert_eq!(
        diff(&offen, &zu),
        "Fokus jetzt auf button „Größentabelle“. Dialog „Größentabelle“ geschlossen. \
         Neuer Text: „Home“, „Produkte“, „Warenkorb (1)“."
    );
    assert_eq!(dismissal_text(&zu), "nichts offen");
}

#[test]
fn shop_clean_suche_fuellen() {
    let s = Seite::laden(SHOP);
    let (vorher, nachher) = s.paar("gehe zu Suche", 0);
    let gv = graph(&vorher);
    let c = eins(ziel(&gv, &befehl("gehe zu Suche")));
    assert_eq!(kurz(c), "[searchbox] Suche");
    let p = plan(c, ActionKind::Focus).unwrap();
    assert_eq!((p.risk, p.requires_confirmation), (Risk::Low, false));
    assert_eq!(
        diff(&vorher, &nachher),
        "Fokus jetzt auf searchbox „Suche“."
    );

    let (vorher, nachher) = s.paar("fülle Suche mit Laufschuhe", 0);
    let cmd = befehl("fülle Suche mit Laufschuhe");
    assert_eq!(cmd, Command::SetValue("Suche".into(), "Laufschuhe".into()));
    let gv = graph(&vorher);
    let c = eins(ziel(&gv, &cmd));
    let p = plan(c, ActionKind::SetValue("Laufschuhe".into())).unwrap();
    assert_eq!((p.risk, p.requires_confirmation), (Risk::Medium, false));
    assert_eq!(diff(&vorher, &nachher), "Neuer Text: „Laufschuhe“.");
    let danach = graph(&nachher);
    let suche = danach
        .controls
        .iter()
        .find(|x| x.role == Role::SearchBox)
        .unwrap();
    assert_eq!(suche.value.as_deref(), Some("Laufschuhe"));
}

#[test]
fn shop_clean_kaufen_nur_mit_bestaetigung() {
    let s = Seite::laden(SHOP);
    assert!(parse("Jetzt kaufen")
        .unwrap_err()
        .starts_with("Nicht verstanden"));
    let (vorher, nachher) = s.paar("!klicke Jetzt kaufen", 0);
    let gv = graph(&vorher);
    let c = eins(ziel(&gv, &befehl("klicke Jetzt kaufen")));
    let p = plan(c, ActionKind::Activate).unwrap();
    assert_eq!(
        (p.risk, p.requires_confirmation, p.notes),
        (Risk::High, true, vec!["Name enthält „kaufen“".to_string()])
    );
    assert_eq!(
        diff(&vorher, &nachher),
        "Fokus jetzt auf button „Jetzt kaufen“. Neuer Text: „Bestellung ausgelöst!“."
    );
}

// ---------------------------------------------------------------------------
// 02-shop-broken: Inferred/Uncertain und Grenzen

const BROKEN: &str = "02-shop-broken/01-shop-broken";

/// Befund 10: Icon-Links ohne Namen werden über die URL erschlossen, mit
/// Evidence; der unbenannte Button bleibt unsicher.
#[test]
fn befund_url_erschliessung() {
    let s = Seite::laden(BROKEN);
    let g = graph(&s.anfang());
    let namen: Vec<(String, Certainty, Vec<String>)> = g
        .controls
        .iter()
        .map(|c| (kurz(c), c.name.certainty, c.name.evidence.clone()))
        .collect();
    assert_eq!(
        namen,
        [
            ("[link] Produkte".into(), Certainty::Known, vec![]),
            (
                "[link] cart".into(),
                Certainty::Inferred,
                vec!["url=file:///cart".to_string()]
            ),
            (
                "[link] mein konto".into(),
                Certainty::Inferred,
                vec!["url=file:///mein-konto".to_string()]
            ),
            (
                "[button] unbenannt (button)".into(),
                Certainty::Uncertain,
                vec!["kein Accessible Name".to_string()]
            ),
        ]
    );
    assert_eq!(
        respond::describe(&g),
        "Seite „Air Max – kaputter Testshop“. Seitentyp möglicherweise Produktseite, unsicher \
         (Hinweise: „In den Warenkorb“ nur als Text, nicht bedienbar, Preis „129,00 €“). \
         0 Überschriften, 4 Bedienelemente, davon 3 ohne gesicherten Namen."
    );
    let aktionen = respond::list_actions(&g);
    assert!(aktionen.contains("[link] cart (erschlossen: url=file:///cart)"));
    assert!(aktionen.contains("[button] unbenannt (button) (Name unbekannt)"));

    // Ein erschlossener Link ist bedienbar, ohne Rückfrage (Risiko LOW).
    let (vorher, nachher) = s.paar("öffne cart", 0);
    let gv = graph(&vorher);
    let cart = eins(ziel(&gv, &befehl("öffne cart")));
    let p = plan(cart, ActionKind::Activate).unwrap();
    assert_eq!((p.risk, p.requires_confirmation), (Risk::Low, false));
    // `/cart` gibt es als Datei nicht: Chrome zeigt seine Fehlerseite.
    assert!(diff(&vorher, &nachher).starts_with("Neue Adresse: chrome-error://chromewebdata/."));
}

#[test]
fn shop_broken_grenzen() {
    let s = Seite::laden("02-shop-broken/02-shop-broken");
    let g = graph(&s.anfang());
    // Unbenannter Button: nur über die Rolle erreichbar, Wirkung unbekannt.
    let button = eins(ziel(&g, &befehl("klicke button")));
    let p = plan(button, ActionKind::Activate).unwrap();
    assert_eq!(
        (p.risk, p.requires_confirmation, p.notes),
        (
            Risk::High,
            true,
            vec![
                "unbenannter Button, Wirkung unbekannt".to_string(),
                "Ziel hat keinen sicheren Namen".to_string()
            ]
        )
    );
    // Befund 9: `div` mit `onclick` und Größen-`span`s existieren im AXTree
    // nicht.
    assert!(matches!(
        ziel(&g, &befehl("klicke In den Warenkorb")),
        Some(Resolution::None)
    ));
    assert_eq!(befehl("wähle 42"), Command::Select(None, "42".into()));
    assert!(felder_mit_option(&g, "42").is_empty());
}

// ---------------------------------------------------------------------------
// 03-form: Formular mit Fehlerbehandlung

const FORM: &str = "03-form/01-form";

#[test]
fn formular_ausfuellen_und_absenden() {
    let s = Seite::laden(FORM);
    let g = graph(&s.anfang());
    assert!(respond::list_actions(&g).contains("[textbox] E-Mail required=true"));

    let (vorher, nachher) = s.paar("fülle Name mit Erika Muster", 0);
    let gv = graph(&vorher);
    let name = eins(ziel(&gv, &befehl("fülle Name mit Erika Muster")));
    assert_eq!(kurz(name), "[textbox] Name");
    assert_eq!(
        diff(&vorher, &nachher),
        "Fokus jetzt auf textbox „Name“. Neuer Text: „Erika Muster“."
    );

    // Absenden: HIGH, Bestätigung nötig.
    let (vorher, nachher) = s.paar("!klicke Nachricht senden", 0);
    let gv = graph(&vorher);
    let senden = eins(ziel(&gv, &befehl("klicke Nachricht senden")));
    let p = plan(senden, ActionKind::Activate).unwrap();
    assert_eq!(
        (p.risk, p.requires_confirmation, p.notes),
        (Risk::High, true, vec!["Name enthält „senden“".to_string()])
    );
    // Validierungsfehler: Fokus auf das Feld, `invalid` wechselt.
    assert_eq!(
        diff(&vorher, &nachher),
        "Fokus jetzt auf textbox „E-Mail“. „E-Mail“: invalid false → true. \
         Neuer Text: „Bitte E-Mail angeben.“."
    );

    let (vorher, nachher) = s.paar("fülle E-Mail mit erika@example.org", 0);
    let cmd = befehl("fülle E-Mail mit erika@example.org");
    assert_eq!(
        cmd,
        Command::SetValue("E-Mail".into(), "erika@example.org".into())
    );
    assert_eq!(kurz(eins(ziel(&graph(&vorher), &cmd))), "[textbox] E-Mail");
    assert_eq!(diff(&vorher, &nachher), "Neuer Text: „erika@example.org“.");

    let (vorher, nachher) = s.paar("!klicke Nachricht senden", 1);
    assert_eq!(
        diff(&vorher, &nachher),
        "Fokus jetzt auf button „Nachricht senden“. „E-Mail“: invalid true → false. \
         Neuer Text: „Danke, Nachricht gesendet.“."
    );
}

/// Befund 5: `describe_diff` meldet `checked` nicht; `target_change` über den
/// Graph schon.
#[test]
fn befund_checkbox_zustand_ueber_graph() {
    let s = Seite::laden(FORM);
    let (vorher, nachher) = s.paar("klicke Datenschutzhinweise", 0);
    let g = graph(&vorher);
    let c = eins(ziel(&g, &befehl("klicke Datenschutzhinweise")));
    assert_eq!(kurz(c), "[checkbox] Datenschutzhinweise gelesen");
    let p = plan(c, ActionKind::Activate).unwrap();
    assert_eq!((p.risk, p.requires_confirmation), (Risk::Medium, false));
    assert_eq!(
        diff(&vorher, &nachher),
        "Fokus jetzt auf checkbox „Datenschutzhinweise gelesen“."
    );
    assert_eq!(
        respond::target_change(&g, &graph(&nachher), &p.target).as_deref(),
        Some("Ziel jetzt: checked=true")
    );
}

// ---------------------------------------------------------------------------
// 04-iframe: Consent-Dialog in einem iframe

/// Befund 8: Der Baum des Frames hängt unter dem `iframe`-Knoten; der
/// Consent-Dialog ist im Graph, seine Buttons sind bedienbar.
#[test]
fn befund_iframe_consent() {
    let s = Seite::laden("04-iframe/01-with-iframe");
    let g = graph(&s.anfang());
    let dialog = g
        .regions
        .iter()
        .position(|r| r.label() == "dialog „Ihre Privatsphäre“")
        .expect("Consent-Dialog aus dem iframe fehlt im Graph");
    assert!(g.regions[dialog].modal);
    let im_dialog: Vec<String> = g
        .controls
        .iter()
        .filter(|c| g.within(c.region, dialog))
        .map(kurz)
        .collect();
    assert_eq!(
        im_dialog,
        ["[button] Alle akzeptieren", "[button] Nur notwendige"]
    );
    // Der modale Dialog sperrt nur das Dokument des Frames: „Zum Artikel“
    // im Elterndokument bleibt erreichbar, wie für Tab im Browser.
    assert_eq!(g.reachable_controls().count(), 3);
    assert_eq!(
        respond::list_actions(&g),
        "main:\n  - [link] Zum Artikel\n\
         dialog „Ihre Privatsphäre“:\n  - [button] Alle akzeptieren\n  - [button] Nur notwendige"
    );
    assert_eq!(
        kurz(eins(ziel(&g, &befehl("klicke Zum Artikel")))),
        "[link] Zum Artikel"
    );
    // Kein Schließen-Button; ohne Fokus ginge Escape ans Dokument.
    assert_eq!(
        dismissal_text(&s.anfang()),
        "Escape: dialog „Ihre Privatsphäre“, erreicht aber das Dokument (kein Element fokussiert)"
    );

    let (vorher, nachher) = s.paar("klicke Nur notwendige", 0);
    let gv = graph(&vorher);
    let c = eins(ziel(&gv, &befehl("klicke Nur notwendige")));
    let p = plan(c, ActionKind::Activate).unwrap();
    assert_eq!((p.risk, p.requires_confirmation), (Risk::Medium, false));
    assert_eq!(
        diff(&vorher, &nachher),
        // Der Fokus steht im Baum des Frames auf dem Button; der Hauptbaum
        // kennt nur den iframe.
        "Fokus jetzt auf button „Nur notwendige“. Neuer Text: „Nur notwendige Cookies.“."
    );
}

// ---------------------------------------------------------------------------
// 10-real: gov.uk, Wikipedia, APG-Beispiele

#[test]
fn govuk_suche_mit_vorschlagsliste() {
    let s = Seite::laden("10-real/01-www-gov-uk");
    let g = graph(&s.anfang());
    assert!(respond::describe(&g).contains(
        "Bereiche: region „Cookies on GOV.UK“, banner, navigation „Navigation menu“, main, \
         search, contentinfo."
    ));
    let (vorher, nachher) = s.paar("gehe zu Search", 0);
    let gv = graph(&vorher);
    let c = eins(ziel(&gv, &befehl("gehe zu Search")));
    assert_eq!(kurz(c), "[combobox] Search");
    assert_eq!(
        diff(&vorher, &nachher),
        "Fokus jetzt auf combobox „Search“."
    );

    let (vorher, nachher) = s.paar("fülle Search mit passport", 0);
    let g = graph(&vorher);
    let c = eins(ziel(&g, &befehl("fülle Search mit passport")));
    let p = plan(c, ActionKind::SetValue("passport".into())).unwrap();
    assert_eq!((p.risk, p.requires_confirmation), (Risk::Medium, false));
    let d = diff(&vorher, &nachher);
    assert!(
        d.starts_with("„Search“: expanded false → true. Neu wahrnehmbar: listbox „passport“"),
        "{d}"
    );
    // Aufgeklappte Combobox mit `hasPopup`: „schließen“ geht per Escape an sie.
    assert_eq!(dismissal_text(&vorher), "nichts offen");
    assert_eq!(dismissal_text(&nachher), "Escape: combobox „Search“");
}

/// Befund 11: benannte Abschnitte werden in der Seitenbeschreibung
/// zusammengefasst; die eingeklappte Suche ist ein Link.
#[test]
fn wikipedia_abschnitte_und_suche() {
    let s = Seite::laden("10-real/02-en-wikipedia-org-wiki-accessibility");
    let g = graph(&s.anfang());
    let text = respond::describe(&g);
    assert!(text.starts_with(
        "Seite „Accessibility - Wikipedia“. Seitentyp vermutlich Artikel \
             (erschlossen: Hauptüberschrift „Accessibility“, 4456 Wörter in längeren Absätzen, \
             0 von 29 Überschriften verlinkt). Hauptüberschrift: „Accessibility“."
    ));
    assert!(text.contains(", contentinfo, 28 benannte Abschnitte. 29 Überschriften"));
    assert_eq!(
        g.regions.iter().filter(|r| r.role == Role::Region).count(),
        28
    );
    assert!(respond::list_headings(&g).contains("\n  H2 "));
    let (vorher, nachher) = s.paar("gehe zu Search", 0);
    assert_eq!(
        kurz(eins(ziel(&graph(&vorher), &befehl("gehe zu Search")))),
        "[link] Search"
    );
    assert_eq!(diff(&vorher, &nachher), "Fokus jetzt auf link „Search“.");
}

#[test]
fn apg_disclosure_aufklappen() {
    let s = Seite::laden(
        "10-real/03-www-w3-org-wai-aria-apg-patterns-disclosure-examples-disclosure-faq",
    );
    let (vorher, nachher) = s.paar("öffne Is there free parking on holidays", 0);
    let g = graph(&vorher);
    assert!(respond::list_actions(&g).contains("[button] Is there free parking on holidays?"));
    let c = eins(ziel(&g, &befehl("öffne Is there free parking on holidays")));
    let p = plan(c, ActionKind::Activate).unwrap();
    assert_eq!((p.risk, p.requires_confirmation), (Risk::Medium, false));
    assert!(diff(&vorher, &nachher)
        .contains("„Is there free parking on holidays?“: expanded false → true."));
    assert_eq!(
        respond::target_change(&g, &graph(&nachher), &p.target).as_deref(),
        Some("Ziel jetzt: expanded=true")
    );
    // Disclosure ohne `hasPopup`: kein Popup, das „schließen“ meint.
    assert_eq!(dismissal_text(&nachher), "nichts offen");
}

const DIALOG: &str = "10-real/06-www-w3-org-wai-aria-apg-patterns-dialog-modal-examples-dialog";

/// Modalität: Chrome blendet bei `aria-modal` den Rest nicht aus (der Baum
/// behält alle Bedienelemente); Relief sperrt ihn selbst.
#[test]
fn befund_modalitaet_apg_dialog() {
    let s = Seite::laden(DIALOG);
    let (vorher, offen) = s.paar("öffne Add Delivery Address", 0);
    let gv = graph(&vorher);
    let c = eins(ziel(&gv, &befehl("öffne Add Delivery Address")));
    assert_eq!(kurz(c), "[button] Add Delivery Address");
    assert_eq!(
        diff(&vorher, &offen),
        "Fokus jetzt auf textbox „Street:“. Dialog „Add Delivery Address“ geöffnet. \
         Rest der Seite ist gesperrt, solange er offen ist."
    );

    let g = graph(&offen);
    let m = g.active_modal().expect("modaler Dialog offen");
    assert_eq!(g.regions[m].label(), "dialog „Add Delivery Address“");
    assert_eq!((g.controls.len(), g.reachable_controls().count()), (67, 8));
    assert!(g.reachable_controls().all(|c| g.within(c.region, m)));
    assert_eq!(befehl("was kann ich tun"), Command::ListActions);
    let aktionen = respond::list_actions(&g);
    assert!(aktionen.starts_with("dialog „Add Delivery Address“:\n  - [textbox] Street:"));
    assert!(aktionen.ends_with(
        "59 weitere Bedienelemente der Seite sind gesperrt, solange „Add Delivery Address“ offen ist."
    ));

    // Hinter dem Dialog: im Graph vorhanden, aber nicht auflösbar.
    let cmd = befehl("öffne Open In CodePen");
    assert!(matches!(ziel(&g, &cmd), Some(Resolution::None)));
    assert!(g
        .controls
        .iter()
        .any(|c| c.name.value.as_deref() == Some("Open In CodePen")
            && !g.is_reachable(c.region, &c.node)));

    // Schließen über „Cancel“ im Dialog.
    let (offen2, zu) = s.paar("schließe den Dialog", 0);
    let g = graph(&offen2);
    let m = modell(&offen2);
    let Dismissal::Button(cancel) = dismissal(&g, &m, focused(&m).as_ref()) else {
        panic!("Cancel-Button erwartet");
    };
    assert_eq!(kurz(cancel), "[button] Cancel");
    assert_eq!(
        diff(&offen2, &zu),
        "Fokus jetzt auf button „Add Delivery Address“. Dialog „Add Delivery Address“ geschlossen."
    );
    assert!(graph(&zu).active_modal().is_none());
}

/// `hasPopup`: Menübutton aufklappen; „schließe das Menü“ findet keinen
/// Dialog und schickt Escape an das aufgeklappte Element.
#[test]
fn befund_haspopup_menuebutton() {
    let s = Seite::laden(
        "10-real/05-www-w3-org-wai-aria-apg-patterns-menu-button-examples-menu-button-links",
    );
    let (vorher, offen) = s.paar("öffne WAI-ARIA Quick Links", 0);
    let g = graph(&vorher);
    let c = eins(ziel(&g, &befehl("öffne WAI-ARIA Quick Links")));
    assert_eq!(
        respond::control_line(c),
        "[button] WAI-ARIA Quick Links hasPopup=menu"
    );
    assert_eq!(dismissal_text(&vorher), "nichts offen");
    let d = diff(&vorher, &offen);
    assert!(
        d.starts_with("Fokus jetzt auf menuitem „W3C Home Page“."),
        "{d}"
    );
    assert!(d.contains(": expanded false → true."), "{d}");

    assert_eq!(befehl("schließe das Menü"), Command::Dismiss);
    // Der Fokus steht auf einem Menüeintrag, das Menü ist das Popup des
    // Buttons (`aria-controls`): Escape erreicht, was es schließen soll.
    assert_eq!(
        dismissal_text(&offen),
        "Escape: button „WAI-ARIA Quick Links“"
    );
    let (offen2, zu) = s.paar("schließe das Menü", 0);
    let d = diff(&offen2, &zu);
    assert!(d.contains(": expanded true → false."), "{d}");
    assert!(d.ends_with("7 Elemente nicht mehr wahrnehmbar."), "{d}");
    assert_eq!(dismissal_text(&zu), "nichts offen");
}

// ---------------------------------------------------------------------------
// 11-korpus: reale Seiten

/// Befund 6: Treffer nur an Wortanfängen — „Suche“ trifft auf casoon.de
/// nicht den Button mit „Besucher“ im Namen.
#[test]
fn befund_suche_trifft_nicht_besucher() {
    let s = Seite::laden("11-korpus/01-www-casoon-de");
    let g = graph(&s.anfang());
    assert!(g.controls.iter().any(|c| c
        .name
        .value
        .as_deref()
        .is_some_and(|n| n.to_lowercase().contains("besucher"))));
    assert!(matches!(
        ziel(&g, &befehl("gehe zu Suche")),
        Some(Resolution::None)
    ));
    assert!(respond::list_actions(&g).contains("navigation „Hauptmenü“:\n"));
}

#[test]
fn korpus_suche_als_link() {
    let s = Seite::laden("11-korpus/02-insights-casoon-de");
    let (vorher, nachher) = s.paar("gehe zu Suche", 0);
    let gv = graph(&vorher);
    let c = eins(ziel(&gv, &befehl("gehe zu Suche")));
    assert_eq!(kurz(c), "[link] Suche");
    assert!(diff(&vorher, &nachher).starts_with("Fokus jetzt auf link „Suche“."));
}

// ---------------------------------------------------------------------------

/// Jede aufgezeichnete Eingabe ist ein Befehl — außer der bewusst
/// unverständlichen aus `01-shop-clean.txt`.
#[test]
fn alle_eingaben_werden_verstanden() {
    for seite in common::seiten() {
        for schritt in &seite.schritte {
            let eingabe = schritt.eingabe.trim_start_matches('!');
            if eingabe == "Jetzt kaufen" {
                continue;
            }
            assert!(
                parse(eingabe).is_ok(),
                "{}: „{}“ nicht verstanden",
                seite.name,
                schritt.eingabe
            );
        }
    }
}
