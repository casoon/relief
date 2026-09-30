//! Seitentyp, funktionale Gruppen und primäre Aktion
//! (→ `plan/spezifikation/04`).
//!
//! Deterministische Heuristiken über Rollen, Namen, Überschriften,
//! Formulare, kurze Texte und die Adresse. Jede Aussage ist eine Inferenz
//! mit Evidence ([`Certainty::Inferred`] oder [`Certainty::Uncertain`]),
//! nie [`Certainty::Known`]. Eine Confidence-Zahl fehlt bewusst: Die Regeln
//! sind nicht kalibriert (→ `plan/spezifikation/10`, Inferenz-Kalibrierung);
//! die Stufe ergibt sich aus der Zahl unabhängiger Signale.
//!
//! Signale nur aus dem Modell: Die Aufnahmen tragen weder Positionen noch
//! HTML-`type`/`autocomplete` noch Schema.org. Liefert ein Host sie später,
//! kommen sie als weitere Signale dazu.

use relief_model::{Certainty, Fact, NodeRef, Role, SemanticGraph, Source};
use serde::Serialize;

use crate::graph::{Control, Graph};

/// Seitentyp aus `plan/spezifikation/04` (ohne Dashboard).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PageType {
    Product,
    Article,
    /// Suchergebnisse.
    Search,
    Form,
    Login,
    Checkout,
    Unknown,
}

impl PageType {
    /// Name für Antworttexte.
    pub fn label(self) -> &'static str {
        match self {
            PageType::Product => "Produktseite",
            PageType::Article => "Artikel",
            PageType::Search => "Suchergebnisse",
            PageType::Form => "Formular",
            PageType::Login => "Anmeldung",
            PageType::Checkout => "Kasse",
            PageType::Unknown => "unbekannt",
        }
    }

    /// Seitentypen, auf denen Aktionen folgenreich sind: Risiko HIGH
    /// (→ `plan/spezifikation/05`) und alle Felder sensibel (→ 07).
    pub fn is_sensitive(self) -> bool {
        matches!(self, PageType::Login | PageType::Checkout)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GroupKind {
    /// Ein Produkt mit Auswahl und Warenkorb-/Kauf-Button.
    Product,
    /// Ein `<form>` mit mindestens einem Feld.
    Form,
}

/// Was funktional zusammengehört.
#[derive(Debug, Clone, Serialize)]
pub struct Group {
    pub kind: GroupKind,
    /// Name des Formulars bzw. Überschrift des Produkts.
    pub name: Option<String>,
    /// Indizes in [`Graph::controls`], in Dokumentreihenfolge.
    pub controls: Vec<usize>,
    /// Index in [`Graph::controls`]; immer erschlossen, nie `Known`.
    pub primary: Option<Fact<usize>>,
}

/// Seitentyp, Gruppen und primäre Aktion der Seite.
#[derive(Debug, Clone, Serialize)]
pub struct Page {
    pub kind: Fact<PageType>,
    pub groups: Vec<Group>,
    /// Primäre Aktion der Seite: die der Gruppe, die den Seitentyp trägt.
    pub primary: Option<Fact<usize>>,
}

impl Default for Page {
    fn default() -> Self {
        Page {
            kind: rule(
                PageType::Unknown,
                Certainty::Uncertain,
                "seitentyp",
                vec!["nicht bestimmt".into()],
            ),
            groups: Vec::new(),
            primary: None,
        }
    }
}

/// Button legt ein Produkt in den Warenkorb.
const CART: &[&str] = &[
    "in den warenkorb",
    "zum warenkorb",
    "warenkorb hinzufügen",
    "in den einkaufswagen",
    "add to cart",
    "add to basket",
    "add to bag",
];
/// Button kauft sofort. Nicht „kaufen“ allein: trifft „einkaufen“.
const BUY: &[&str] = &["jetzt kaufen", "sofort kaufen", "buy now"];
/// Ab so vielen Warenkorb-Buttons ist es eine Liste, keine Produktseite.
const PRODUCT_LIST: usize = 3;

/// Beschriftung eines Absende-Buttons.
const SUBMIT: &[&str] = &[
    "senden",
    "abschicken",
    "anmelden",
    "einloggen",
    "registrieren",
    "speichern",
    "bestellen",
    "bestätigen",
    "suchen",
    "weiter",
    "submit",
    "send",
    "log in",
    "login",
    "sign in",
    "sign up",
    "register",
    "save",
    "continue",
    "confirm",
];

const PASSWORD: &[&str] = &["passwort", "password", "kennwort"];
const LOGIN_BUTTON: &[&str] = &[
    "anmelden",
    "einloggen",
    "anmeldung",
    "login",
    "log in",
    "sign in",
];
const LOGIN_PATH: &[&str] = &[
    "login",
    "signin",
    "sign-in",
    "log-in",
    "anmelden",
    "anmeldung",
    "auth",
];
/// Höchstens so viele Textfelder hat ein Anmeldeformular; mehr ist eher
/// eine Registrierung.
const LOGIN_FIELDS: usize = 3;

const CHECKOUT_PATH: &[&str] = &[
    "checkout",
    "kasse",
    "bezahlen",
    "bezahlung",
    "payment",
    "zahlung",
];
const PAYMENT_FIELD: &[&str] = &[
    "kartennummer",
    "card number",
    "kreditkarte",
    "credit card",
    "iban",
    "cvc",
    "cvv",
    "prüfnummer",
    "ablaufdatum",
    "gültig bis",
    "expiry",
    "expiration",
];
const CHECKOUT_STEP: &[&str] = &[
    "lieferadresse",
    "rechnungsadresse",
    "versandadresse",
    "shipping address",
    "billing address",
    "zahlungsart",
    "zahlungsmethode",
    "payment method",
    "versandart",
    "shipping method",
];
const FINAL_BUTTON: &[&str] = &[
    "zahlungspflichtig bestellen",
    "kostenpflichtig bestellen",
    "jetzt bezahlen",
    "bestellung abschicken",
    "bestellung abschließen",
    "kauf abschließen",
    "place order",
    "pay now",
    "complete purchase",
    "complete order",
];

const SEARCH_KEYS: &[&str] = &[
    "q",
    "query",
    "s",
    "search",
    "suche",
    "suchbegriff",
    "searchterm",
    "keyword",
    "keywords",
    "k",
    "term",
];
const SEARCH_PATH: &[&str] = &["search", "suche", "suchergebnisse", "ergebnisse", "results"];
const RESULT_HEADING: &[&str] = &[
    "suchergebnis",
    "ergebnisse für",
    "ergebnisse zu",
    "treffer für",
    "search results",
    "results for",
];

/// Artikel: so viele Wörter in längeren Absätzen mindestens …
const ARTICLE_WORDS: usize = 300;
/// … ein Absatz ist länger ab so vielen Wörtern …
const PARAGRAPH_WORDS: usize = 20;
/// … je Überschrift so viele Wörter mindestens (Startseiten: viele
/// Anrisse unter eigenen Überschriften) …
const WORDS_PER_HEADING: usize = 15;
/// … und höchstens dieser Anteil verlinkter Überschriften (Anrisse sind
/// Links auf Artikel).
const LINKED_HEADINGS: f32 = 0.25;

/// Formularseite: das größte Formular hat so viele Felder mindestens.
const FORM_FIELDS: usize = 3;

pub fn analyze(model: &SemanticGraph, graph: &Graph) -> Page {
    let mut groups = form_groups(model, graph);
    let cart = buttons_with(graph, CART);
    let product = product_group(graph, &cart);
    if let Some(p) = product {
        groups.insert(0, p);
    }
    let signals = Signals::collect(model, graph, &groups, cart.len());
    let (kind, group) = signals.classify();
    let primary = group.and_then(|g| groups[g].primary.clone());
    Page {
        kind,
        groups,
        primary,
    }
}

/// Was die Regeln über die Seite wissen müssen.
struct Signals<'g> {
    graph: &'g Graph,
    url: Option<String>,
    groups: &'g [Group],
    cart_buttons: usize,
    h1: Option<String>,
    /// Kurzer Text mit Ziffer und Währung.
    price: Option<String>,
    /// Warenkorb-Beschriftung als bloßer Text (kein Bedienelement).
    cart_text: Option<String>,
    long_words: usize,
    linked_headings: usize,
}

impl<'g> Signals<'g> {
    fn collect(
        model: &SemanticGraph,
        graph: &'g Graph,
        groups: &'g [Group],
        cart_buttons: usize,
    ) -> Self {
        let url = model
            .root
            .as_ref()
            .and_then(|t| model.trees.get(t))
            .and_then(|t| t.data.url.clone());
        let body = graph.texts.iter().filter(|t| t.level.is_none());
        let price = body
            .clone()
            .map(|t| t.text.as_str())
            .find(|t| is_price(t))
            .map(String::from);
        let cart_text = body
            .clone()
            .map(|t| t.text.as_str())
            .find(|t| matches_any(t, CART).is_some() && words(t) <= 6)
            .map(String::from);
        let long_words = body
            .map(|t| words(&t.text))
            .filter(|w| *w >= PARAGRAPH_WORDS)
            .sum();
        let linked_headings = graph
            .headings
            .iter()
            .filter(|h| near_link(model, &h.node))
            .count();
        Signals {
            graph,
            url,
            groups,
            cart_buttons,
            h1: graph
                .headings
                .iter()
                .find(|h| h.level == 1)
                .map(|h| h.text.clone()),
            price,
            cart_text,
            long_words,
            linked_headings,
        }
    }

    /// Seitentyp und die Gruppe, die ihn trägt (Index in `groups`). Die
    /// Reihenfolge der Regeln ist die Vorrangfolge: Kasse und Anmeldung
    /// enthalten Formulare, Suchergebnisse oft Preise.
    fn classify(&self) -> (Fact<PageType>, Option<usize>) {
        let rules: [(&str, Rule<'g>); 6] = [
            ("seitentyp-kasse", Self::checkout),
            ("seitentyp-anmeldung", Self::login),
            ("seitentyp-suche", Self::search),
            ("seitentyp-produkt", Self::product),
            ("seitentyp-formular", Self::form),
            ("seitentyp-artikel", Self::article),
        ];
        for (id, rule_fn) in rules {
            if let Some(hit) = rule_fn(self) {
                return (rule(hit.kind, hit.certainty, id, hit.evidence), hit.group);
            }
        }
        let mut evidence = vec!["keine Regel trifft zu".to_string()];
        if self.cart_buttons >= PRODUCT_LIST {
            evidence.push(format!(
                "{} Warenkorb-Buttons: Liste mehrerer Produkte",
                self.cart_buttons
            ));
        }
        (
            rule(
                PageType::Unknown,
                Certainty::Uncertain,
                "seitentyp",
                evidence,
            ),
            None,
        )
    }

    fn largest_form(&self, filter: impl Fn(&Group) -> bool) -> Option<usize> {
        self.groups
            .iter()
            .enumerate()
            .filter(|(_, g)| g.kind == GroupKind::Form && filter(g))
            .max_by_key(|(i, g)| (fields(self.graph, g).count(), std::cmp::Reverse(*i)))
            .map(|(i, _)| i)
    }

    /// Kasse: Adresse, Zahlungsfeld, Schritt (Liefer-/Rechnungsadresse,
    /// Zahlungsart) und Bestell-Button sind je ein Signal. Ab zwei
    /// erschlossen, eines allein unsicher — reicht aber, um das Risiko zu
    /// erhöhen.
    fn checkout(&self) -> Option<Hit> {
        let g = self.graph;
        let mut evidence = Vec::new();
        if let Some(seg) = self.path_segment(CHECKOUT_PATH) {
            evidence.push(format!("Adresse enthält „{seg}“"));
        }
        if let Some(c) = g
            .controls
            .iter()
            .find(|c| is_field(&c.role) && matches_any(name(c), PAYMENT_FIELD).is_some())
        {
            evidence.push(format!("Zahlungsfeld „{}“", name(c)));
        }
        // Überschriften nur außerhalb von Kopf, Navigation, Fuß und
        // Randspalte: „Zahlungsarten“ im Fuß steht auf vielen Seiten
        // (bahn.de); Beschriftungen nur an Feldern.
        let step = g
            .headings
            .iter()
            .filter(|h| !in_page_frame(g, h.region))
            .map(|h| h.text.as_str())
            .chain(g.controls.iter().filter(|c| is_field(&c.role)).map(name))
            .find(|t| matches_any(t, CHECKOUT_STEP).is_some());
        if let Some(t) = step {
            evidence.push(format!("Bestellschritt „{t}“"));
        }
        if let Some(c) = g
            .controls
            .iter()
            .find(|c| c.role == Role::Button && matches_any(name(c), FINAL_BUTTON).is_some())
        {
            evidence.push(format!("Button „{}“", name(c)));
        }
        if evidence.is_empty() {
            return None;
        }
        Some(Hit {
            kind: PageType::Checkout,
            certainty: at_least_two(&evidence),
            group: self.largest_form(|_| true),
            evidence,
        })
    }

    /// Anmeldung: Passwortfeld in einem Formular mit wenigen Textfeldern;
    /// dazu Anmelde-Button oder Adresse.
    fn login(&self) -> Option<Hit> {
        let g = self.graph;
        let password = g
            .controls
            .iter()
            .position(|c| c.role == Role::Textbox && matches_any(name(c), PASSWORD).is_some())?;
        let group = self
            .groups
            .iter()
            .position(|gr| gr.kind == GroupKind::Form && gr.controls.contains(&password));
        let scope: Vec<&Control> = match group {
            Some(i) => self.groups[i]
                .controls
                .iter()
                .map(|&c| &g.controls[c])
                .collect(),
            None => g.controls.iter().collect(),
        };
        let text_fields = scope
            .iter()
            .filter(|c| matches!(c.role, Role::Textbox | Role::Combobox))
            .count();
        if text_fields > LOGIN_FIELDS {
            return None;
        }
        let mut evidence = vec![format!("Passwortfeld „{}“", name(&g.controls[password]))];
        if text_fields > 1 {
            evidence.push(format!("{text_fields} Textfelder"));
        }
        let mut more = false;
        if let Some(b) = scope
            .iter()
            .find(|c| c.role == Role::Button && matches_any(name(c), LOGIN_BUTTON).is_some())
        {
            evidence.push(format!("Button „{}“", name(b)));
            more = true;
        }
        if let Some(seg) = self.path_segment(LOGIN_PATH) {
            evidence.push(format!("Adresse enthält „{seg}“"));
            more = true;
        }
        Some(Hit {
            kind: PageType::Login,
            certainty: if more {
                Certainty::Inferred
            } else {
                Certainty::Uncertain
            },
            group,
            evidence,
        })
    }

    /// Suchergebnisse: Suchbegriff in der Adresse (Parameter oder Pfad),
    /// Ergebnis-Überschrift, ausgefülltes Suchfeld. Ein ausgefülltes Feld
    /// allein zählt nicht (Eingabe vor dem Absenden).
    fn search(&self) -> Option<Hit> {
        let g = self.graph;
        let mut evidence = Vec::new();
        if let Some(q) = self.search_query() {
            evidence.push(q);
        }
        if let Some(h) = g
            .headings
            .iter()
            .find(|h| matches_any(&h.text, RESULT_HEADING).is_some())
        {
            evidence.push(format!("Überschrift „{}“", h.text));
        }
        if evidence.is_empty() {
            return None;
        }
        if let Some(c) = g.controls.iter().find(|c| {
            c.value.is_some()
                && (c.role == Role::SearchBox
                    || c.region.is_some_and(|r| g.regions[r].role == Role::Search))
        }) {
            evidence.push(format!(
                "Suchfeld „{}“ = „{}“",
                name(c),
                c.value.as_deref().unwrap_or_default()
            ));
        }
        Some(Hit {
            kind: PageType::Search,
            certainty: at_least_two(&evidence),
            group: None,
            evidence,
        })
    }

    /// Produkt: eine Produktgruppe (Warenkorb-/Kauf-Button, höchstens zwei
    /// Warenkorb-Buttons auf der Seite), dazu Preis und Hauptüberschrift.
    /// Ohne Button, aber mit Preis und Warenkorb-Text: unsicher (Button
    /// nicht bedienbar, z. B. `div` mit `onclick`).
    fn product(&self) -> Option<Hit> {
        // Liste mehrerer Produkte: auch kein Produkt über den Text.
        if self.cart_buttons >= PRODUCT_LIST {
            return None;
        }
        let group = self
            .groups
            .iter()
            .position(|g| g.kind == GroupKind::Product);
        let mut evidence = Vec::new();
        let mut strong = 0;
        if let Some(i) = group {
            let primary = self.groups[i].primary.as_ref().and_then(|p| p.value);
            if let Some(p) = primary {
                evidence.push(format!("Button „{}“", name(&self.graph.controls[p])));
            }
            strong += 1;
        } else if let Some(t) = &self.cart_text {
            evidence.push(format!("„{t}“ nur als Text, nicht bedienbar"));
        } else {
            return None;
        }
        if let Some(p) = &self.price {
            evidence.push(format!("Preis „{p}“"));
            strong += 1;
        }
        if let Some(h) = &self.h1 {
            evidence.push(format!("Hauptüberschrift „{h}“"));
            strong += 1;
        }
        // Warenkorb-Text ohne Preis ist zu wenig.
        if group.is_none() && self.price.is_none() {
            return None;
        }
        Some(Hit {
            kind: PageType::Product,
            certainty: if strong >= 3 {
                Certainty::Inferred
            } else {
                Certainty::Uncertain
            },
            group,
            evidence,
        })
    }

    /// Formular: das größte Formular hat mindestens [`FORM_FIELDS`] Felder,
    /// und die Seite ist kein langer Text mit Formular am Rand.
    fn form(&self) -> Option<Hit> {
        let i = self.largest_form(|_| true)?;
        let group = &self.groups[i];
        let n = fields(self.graph, group).count();
        if n < FORM_FIELDS || self.long_words >= ARTICLE_WORDS {
            return None;
        }
        let mut evidence = vec![match &group.name {
            Some(name) => format!("Formular „{name}“ mit {n} Feldern"),
            None => format!("Formular mit {n} Feldern"),
        }];
        if let Some(h) = &self.h1 {
            evidence.push(format!("Hauptüberschrift „{h}“"));
        }
        Some(Hit {
            kind: PageType::Form,
            certainty: Certainty::Inferred,
            group: Some(i),
            evidence,
        })
    }

    /// Artikel: Hauptüberschrift, viel Fließtext in längeren Absätzen,
    /// wenig Überschriften je Text und kaum verlinkte Überschriften
    /// (Startseiten reihen Anrisse mit verlinkten Überschriften).
    fn article(&self) -> Option<Hit> {
        let h1 = self.h1.as_ref()?;
        let headings = self.graph.headings.len();
        let linked = self.linked_headings as f32 / headings as f32;
        if self.long_words < ARTICLE_WORDS
            || self.long_words < WORDS_PER_HEADING * headings
            || linked >= LINKED_HEADINGS
        {
            return None;
        }
        Some(Hit {
            kind: PageType::Article,
            certainty: Certainty::Inferred,
            group: None,
            evidence: vec![
                format!("Hauptüberschrift „{h1}“"),
                format!("{} Wörter in längeren Absätzen", self.long_words),
                format!(
                    "{} von {headings} Überschriften verlinkt",
                    self.linked_headings
                ),
            ],
        })
    }

    /// Pfadsegmente der Adresse (ohne Endung), klein geschrieben.
    fn segments(&self) -> Vec<String> {
        let Some(url) = &self.url else {
            return Vec::new();
        };
        let path = url.split(['?', '#']).next().unwrap_or_default();
        let path = path.split_once("://").map_or(path, |(_, rest)| {
            rest.split_once('/').map_or("", |(_, p)| p)
        });
        path.split('/')
            .map(|s| s.split('.').next().unwrap_or(s).to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn path_segment(&self, words: &[&str]) -> Option<String> {
        self.segments()
            .into_iter()
            .find(|s| words.contains(&s.as_str()))
    }

    /// Suchbegriff als Parameter (`?q=…`) oder Suchpfad (`/suche/`).
    fn search_query(&self) -> Option<String> {
        let url = self.url.as_deref()?;
        let query = url
            .split_once('?')
            .map(|(_, q)| q.split('#').next().unwrap_or(q));
        for pair in query.into_iter().flat_map(|q| q.split('&')) {
            if let Some((key, value)) = pair.split_once('=') {
                if SEARCH_KEYS.contains(&key.to_lowercase().as_str()) && !value.is_empty() {
                    return Some(format!("Adresse mit Suchbegriff „{key}={value}“"));
                }
            }
        }
        self.path_segment(SEARCH_PATH)
            .map(|seg| format!("Adresse enthält „{seg}“"))
    }
}

/// Eine Seitentyp-Regel.
type Rule<'g> = fn(&Signals<'g>) -> Option<Hit>;

struct Hit {
    kind: PageType,
    certainty: Certainty,
    group: Option<usize>,
    evidence: Vec<String>,
}

fn at_least_two(evidence: &[String]) -> Certainty {
    if evidence.len() >= 2 {
        Certainty::Inferred
    } else {
        Certainty::Uncertain
    }
}

/// Formulare: Bedienelemente mit demselben nächsten `form`-Vorfahren im
/// Modell, auch unbenannte (die im Graph kein Bereich sind). Suchformulare
/// (`role=search`) zählen nicht; ein Formular braucht ein Feld.
fn form_groups(model: &SemanticGraph, graph: &Graph) -> Vec<Group> {
    let mut forms: Vec<(NodeRef, Vec<usize>)> = Vec::new();
    for (i, c) in graph.controls.iter().enumerate() {
        let Some(form) = form_of(model, &c.node) else {
            continue;
        };
        match forms.iter_mut().find(|(f, _)| *f == form) {
            Some((_, controls)) => controls.push(i),
            None => forms.push((form, vec![i])),
        }
    }
    forms
        .into_iter()
        .filter(|(_, controls)| controls.iter().any(|&i| is_field(&graph.controls[i].role)))
        .map(|(form, controls)| {
            let name = model
                .node(&form)
                .and_then(|n| crate::graph::non_empty(n.name.value.as_deref()));
            let primary = form_primary(graph, &controls, name.as_deref());
            Group {
                kind: GroupKind::Form,
                name,
                controls,
                primary,
            }
        })
        .collect()
}

/// Nächster `form`-Vorfahr im selben Baum; `None` in einem Suchbereich.
fn form_of(model: &SemanticGraph, at: &NodeRef) -> Option<NodeRef> {
    let tree = model.trees.get(&at.tree)?;
    let mut current = tree.nodes.get(&at.node)?.parent;
    while let Some(id) = current {
        let node = tree.nodes.get(&id)?;
        match node.role {
            Role::Search => return None,
            Role::Form => return Some(NodeRef::new(at.tree.clone(), id)),
            _ => current = node.parent,
        }
    }
    None
}

/// Absende-Button eines Formulars: Beschriftung mit Absende-Wort
/// (erschlossen), sonst der letzte Button (unsicher).
fn form_primary(graph: &Graph, controls: &[usize], form: Option<&str>) -> Option<Fact<usize>> {
    let buttons: Vec<usize> = controls
        .iter()
        .copied()
        .filter(|&i| graph.controls[i].role == Role::Button && !graph.controls[i].disabled)
        .collect();
    let within = match form {
        Some(f) => format!("Button im Formular „{f}“"),
        None => "Button im Formular".to_string(),
    };
    if let Some((i, word)) = buttons
        .iter()
        .find_map(|&i| Some((i, matches_any(name(&graph.controls[i]), SUBMIT)?)))
    {
        return Some(rule(
            i,
            Certainty::Inferred,
            "primaere-aktion-formular",
            vec![within, format!("Name enthält „{word}“")],
        ));
    }
    let last = *buttons.last()?;
    Some(rule(
        last,
        Certainty::Uncertain,
        "primaere-aktion-formular",
        vec![within, "letzter Button des Formulars".into()],
    ))
}

/// Produktgruppe um den ersten Warenkorb-Button (sonst Kauf-Button): die
/// Bedienelemente außer Links im selben Bereich und Abschnitt. Keine
/// Gruppe bei [`PRODUCT_LIST`] oder mehr Warenkorb-Buttons.
fn product_group(graph: &Graph, cart: &[(usize, &'static str)]) -> Option<Group> {
    if cart.len() >= PRODUCT_LIST {
        return None;
    }
    let &(anchor, word) = cart.first().or(buttons_with(graph, BUY).first())?;
    let a = &graph.controls[anchor];
    let controls: Vec<usize> = graph
        .controls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.role != Role::Link && c.region == a.region && c.heading == a.heading)
        .map(|(i, _)| i)
        .collect();
    let name = a.heading.map(|h| graph.headings[h].text.clone());
    let mut evidence = vec![format!("Name enthält „{word}“")];
    if let Some(n) = &name {
        evidence.push(format!("im Abschnitt „{n}“"));
    }
    if a.region
        .is_some_and(|r| graph.regions[r].role == Role::Main)
    {
        evidence.push("im Hauptbereich".into());
    }
    Some(Group {
        kind: GroupKind::Product,
        name,
        controls,
        primary: Some(rule(
            anchor,
            Certainty::Inferred,
            "primaere-aktion-produkt",
            evidence,
        )),
    })
}

/// Buttons, deren Name eines der Wörter enthält, mit dem Treffer.
fn buttons_with(graph: &Graph, list: &[&'static str]) -> Vec<(usize, &'static str)> {
    graph
        .controls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.role == Role::Button && !c.disabled)
        .filter_map(|(i, c)| Some((i, matches_any(name(c), list)?)))
        .collect()
}

/// Rollen, die als Formularfeld zählen (wie `resolve::step_field`, ohne
/// Suchfeld).
fn is_field(role: &Role) -> bool {
    matches!(
        role,
        Role::Textbox
            | Role::Combobox
            | Role::Listbox
            | Role::Checkbox
            | Role::Radio
            | Role::Switch
            | Role::Slider
            | Role::SpinButton
    )
}

/// Liegt der Bereich im Seitenrahmen (Kopf, Navigation, Fuß, Randspalte)?
fn in_page_frame(graph: &Graph, region: Option<usize>) -> bool {
    let mut current = region;
    while let Some(r) = current {
        if matches!(
            graph.regions[r].role,
            Role::Banner | Role::Navigation | Role::ContentInfo | Role::Complementary
        ) {
            return true;
        }
        current = graph.regions[r].parent;
    }
    false
}

fn fields<'a>(graph: &'a Graph, group: &'a Group) -> impl Iterator<Item = &'a Control> + 'a {
    group
        .controls
        .iter()
        .map(|&i| &graph.controls[i])
        .filter(|c| is_field(&c.role))
}

fn name(c: &Control) -> &str {
    c.name.value.as_deref().unwrap_or_default()
}

/// Erstes Wort der Liste, das im Text vorkommt (ohne Groß-/Kleinschreibung).
fn matches_any(text: &str, list: &[&'static str]) -> Option<&'static str> {
    let text = text.to_lowercase();
    list.iter().copied().find(|w| text.contains(w))
}

fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Kurzer Text (höchstens sechs Wörter) mit Ziffer und Währung.
fn is_price(text: &str) -> bool {
    words(text) <= 6
        && text.chars().any(|c| c.is_ascii_digit())
        && (text.contains(['€', '$', '£']) || text.contains("EUR") || text.contains("CHF"))
}

/// Überschrift in einem Link oder mit einem Link darin (Anriss).
fn near_link(model: &SemanticGraph, at: &NodeRef) -> bool {
    let Some(tree) = model.trees.get(&at.tree) else {
        return false;
    };
    let mut up = tree.nodes.get(&at.node).and_then(|n| n.parent);
    // Anrisse: Link um Überschrift, oft mit einem Bild-/Textcontainer dazwischen.
    for _ in 0..3 {
        let Some(node) = up.and_then(|id| tree.nodes.get(&id)) else {
            break;
        };
        if node.role == Role::Link {
            return true;
        }
        up = node.parent;
    }
    fn contains_link(tree: &relief_model::SemanticTree, id: relief_model::NodeId) -> bool {
        tree.nodes.get(&id).is_some_and(|n| {
            n.role == Role::Link || n.children.iter().any(|c| contains_link(tree, *c))
        })
    }
    tree.nodes
        .get(&at.node)
        .is_some_and(|n| n.children.iter().any(|c| contains_link(tree, *c)))
}

/// Aussage einer Regel dieses Moduls: nie `Known`, ohne Confidence-Zahl.
fn rule<T>(value: T, certainty: Certainty, id: &str, evidence: Vec<String>) -> Fact<T> {
    Fact {
        value: Some(value),
        certainty,
        source: Source::Rule(id.to_string()),
        confidence: None,
        evidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::{plan_on_page, ActionKind, Risk};
    use a11y_perception::{AXNode, AXTree};
    use relief_model::TreeId;

    /// Seite aus `(id, rolle, name, kinder)`, Wurzel zuerst, mit Adresse.
    fn seite(url: &str, nodes: &[(u32, &str, &str, &[u32])]) -> Graph {
        let tree = AXTree::from_nodes(
            nodes
                .iter()
                .map(|&(id, role, name, children)| AXNode {
                    node_id: id.to_string(),
                    role: Some(role.into()),
                    name: (!name.is_empty()).then(|| name.into()),
                    child_ids: children.iter().map(u32::to_string).collect(),
                    parent_id: nodes
                        .iter()
                        .find(|n| n.3.contains(&id))
                        .map(|n| n.0.to_string()),
                    backend_dom_node_id: Some(id.into()),
                    ..Default::default()
                })
                .collect(),
        );
        let mut model = relief_model::perception::from_tree(&tree, &TreeId("t".into()));
        model.trees.values_mut().next().unwrap().data.url = Some(url.into());
        Graph::build(&model)
    }

    fn by_name<'g>(g: &'g Graph, name: &str) -> &'g Control {
        g.controls
            .iter()
            .find(|c| c.name.value.as_deref() == Some(name))
            .unwrap()
    }

    fn primary_name(g: &Graph) -> Option<&str> {
        let p = g.page.primary.as_ref()?;
        g.controls[p.value?].name.value.as_deref()
    }

    fn login() -> Graph {
        seite(
            "https://shop.example/konto",
            &[
                (1, "RootWebArea", "Konto", &[2]),
                (2, "main", "", &[3, 4, 9]),
                (3, "heading", "Willkommen zurück", &[]),
                (4, "form", "", &[5, 6, 7, 8]),
                (5, "textbox", "E-Mail", &[]),
                (6, "textbox", "Passwort", &[]),
                (7, "button", "Passwort anzeigen", &[]),
                (8, "button", "Anmelden", &[]),
                (9, "link", "Passwort vergessen?", &[]),
            ],
        )
    }

    #[test]
    fn anmeldung_erhoeht_risiko_nur() {
        let g = login();
        assert_eq!(g.page.kind.value, Some(PageType::Login));
        assert_eq!(g.page.kind.certainty, Certainty::Inferred);
        assert_eq!(
            g.page.kind.evidence,
            [
                "Passwortfeld „Passwort“",
                "2 Textfelder",
                "Button „Anmelden“"
            ]
        );
        assert_eq!(primary_name(&g), Some("Anmelden"));

        let p = plan_on_page(&g.page, by_name(&g, "Anmelden"), ActionKind::Activate).unwrap();
        assert_eq!((p.risk, p.requires_confirmation), (Risk::High, true));
        assert!(p.notes[0].starts_with("Seitentyp vermutlich Anmeldung ("));
        // Auch ein harmloser Button: auf der Anmeldeseite lieber nachfragen.
        let p = plan_on_page(
            &g.page,
            by_name(&g, "Passwort anzeigen"),
            ActionKind::Activate,
        )
        .unwrap();
        assert_eq!(p.risk, Risk::High);
        // Links, Ausfüllen und Fokus bleiben, wie sie sind.
        let link = by_name(&g, "Passwort vergessen?");
        let p = plan_on_page(&g.page, link, ActionKind::Activate).unwrap();
        assert_eq!((p.risk, p.requires_confirmation), (Risk::Low, false));
        let feld = by_name(&g, "Passwort");
        let p = plan_on_page(&g.page, feld, ActionKind::SetValue("x".into())).unwrap();
        assert_eq!(p.risk, Risk::Medium);
        let p = plan_on_page(&g.page, feld, ActionKind::Focus).unwrap();
        assert_eq!(p.risk, Risk::Low);
    }

    #[test]
    fn registrierung_ist_formular_nicht_anmeldung() {
        let g = seite(
            "https://shop.example/registrieren",
            &[
                (1, "RootWebArea", "Registrieren", &[2]),
                (2, "form", "Konto anlegen", &[3, 4, 5, 6, 7, 8]),
                (3, "textbox", "Vorname", &[]),
                (4, "textbox", "Nachname", &[]),
                (5, "textbox", "E-Mail", &[]),
                (6, "textbox", "Passwort", &[]),
                (7, "textbox", "Passwort wiederholen", &[]),
                (8, "button", "Konto erstellen", &[]),
            ],
        );
        assert_eq!(g.page.kind.value, Some(PageType::Form));
        // Kein Absende-Wort: letzter Button, unsicher.
        let p = g.page.primary.as_ref().unwrap();
        assert_eq!(p.certainty, Certainty::Uncertain);
        assert_eq!(primary_name(&g), Some("Konto erstellen"));
    }

    #[test]
    fn kasse_aus_mehreren_signalen() {
        let g = seite(
            "https://shop.example/checkout/",
            &[
                (1, "RootWebArea", "Bestellung", &[2]),
                (2, "main", "", &[3, 4]),
                (3, "heading", "Zahlungsart", &[]),
                (4, "form", "", &[5, 6, 7, 8]),
                (5, "textbox", "Kartennummer", &[]),
                (6, "textbox", "Name auf der Karte", &[]),
                (7, "checkbox", "AGB gelesen", &[]),
                (8, "button", "Zahlungspflichtig bestellen", &[]),
            ],
        );
        assert_eq!(g.page.kind.value, Some(PageType::Checkout));
        assert_eq!(g.page.kind.certainty, Certainty::Inferred);
        assert_eq!(
            g.page.kind.evidence,
            [
                "Adresse enthält „checkout“",
                "Zahlungsfeld „Kartennummer“",
                "Bestellschritt „Zahlungsart“",
                "Button „Zahlungspflichtig bestellen“"
            ]
        );
        assert_eq!(primary_name(&g), Some("Zahlungspflichtig bestellen"));
        // Zustimmung zu AGB auf der Kasse: HIGH statt MEDIUM.
        let p = plan_on_page(&g.page, by_name(&g, "AGB gelesen"), ActionKind::Activate).unwrap();
        assert_eq!((p.risk, p.requires_confirmation), (Risk::High, true));
    }

    #[test]
    fn unsichere_kasse_erhoeht_trotzdem() {
        let g = seite(
            "https://shop.example/kasse",
            &[
                (1, "RootWebArea", "Shop", &[2]),
                (2, "button", "Weiter", &[]),
            ],
        );
        assert_eq!(g.page.kind.value, Some(PageType::Checkout));
        assert_eq!(g.page.kind.certainty, Certainty::Uncertain);
        let p = plan_on_page(&g.page, by_name(&g, "Weiter"), ActionKind::Activate).unwrap();
        assert_eq!(p.risk, Risk::High);
        assert_eq!(
            p.notes,
            ["Seitentyp möglicherweise Kasse (Adresse enthält „kasse“)"]
        );
    }

    #[test]
    fn suchergebnisse_brauchen_mehr_als_ein_ausgefuelltes_feld() {
        let nodes: &[(u32, &str, &str, &[u32])] = &[
            (1, "RootWebArea", "Shop", &[2, 4]),
            (2, "search", "", &[3]),
            (3, "searchbox", "Suche", &[]),
            (4, "main", "", &[5]),
            (5, "heading", "Suchergebnisse für Laufschuhe", &[]),
        ];
        let g = seite("https://shop.example/suche?q=laufschuhe", nodes);
        assert_eq!(g.page.kind.value, Some(PageType::Search));
        assert_eq!(g.page.kind.certainty, Certainty::Inferred);
        assert_eq!(
            g.page.kind.evidence,
            [
                "Adresse mit Suchbegriff „q=laufschuhe“",
                "Überschrift „Suchergebnisse für Laufschuhe“"
            ]
        );
        assert_eq!(g.page.primary, None);
        // Ohne Adresse und Überschrift: kein Seitentyp aus dem Suchfeld.
        let g = seite("https://shop.example/", &nodes[..3]);
        assert_eq!(g.page.kind.value, Some(PageType::Unknown));
    }

    #[test]
    fn nie_known() {
        for g in [login(), Graph::build(&crate::graph::sample_tree())] {
            assert_ne!(g.page.kind.certainty, Certainty::Known);
            assert!(!g.page.kind.evidence.is_empty());
            for p in g.page.groups.iter().filter_map(|gr| gr.primary.as_ref()) {
                assert_ne!(p.certainty, Certainty::Known);
                assert!(!p.evidence.is_empty());
            }
        }
    }
}
