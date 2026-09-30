//! Overlays erkennen und einordnen: Consent-, Newsletter- und andere
//! Dialoge (→ `plan/spezifikation/05`, „Overlay- und Consent-Dialoge“).
//!
//! Grundlage ist die Modalität je Frame aus dem Graph (→
//! [`Graph::is_reachable`]): Ein Overlay ist ein erreichbarer Dialog oder
//! ein benannter Bereich, dessen Name nach Einwilligung klingt; seine
//! Buttons und Links sind alle Bedienelemente darin, auch in einem iframe
//! darunter (Consent-iframes auf spiegel.de, bild.de).
//!
//! Alles hier ist **Inferenz** aus Beschriftungen, nie
//! [`Certainty::Known`]: Ob ein Dialog ein Cookie-Dialog ist und ob ein
//! Button zustimmt, weiß Relief nur aus Wörtern. Deshalb konservativ:
//! Was nach Zustimmung oder Bezahlen klingt, wählt Relief nie selbst, auch
//! nicht als Schließen-Button ([`blocks_dismissal`]); Ablehnen gibt es nur
//! auf ausdrücklichen Befehl und nur über einen Button, der ablehnt und
//! nicht bezahlt.

use relief_model::{Certainty, Fact, Role, Source};
use serde::Serialize;

use crate::graph::{Control, Graph};

/// Was für ein Overlay es vermutlich ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OverlayKind {
    Consent,
    Newsletter,
    /// Irgendein anderer Dialog.
    Dialog,
}

impl OverlayKind {
    pub fn label(self) -> &'static str {
        match self {
            OverlayKind::Consent => "Cookie-Dialog",
            OverlayKind::Newsletter => "Newsletter-Dialog",
            OverlayKind::Dialog => "Dialog",
        }
    }
}

/// Was ein Button in einem Overlay vermutlich tut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ButtonKind {
    Accept,
    Reject,
    Settings,
    /// Abo oder Bezahlen (z. B. „pur“): kein kostenloses Ablehnen.
    Pay,
    Close,
    Other,
}

impl ButtonKind {
    pub fn label(self) -> &'static str {
        match self {
            ButtonKind::Accept => "Zustimmen",
            ButtonKind::Reject => "Ablehnen",
            ButtonKind::Settings => "Einstellungen",
            ButtonKind::Pay => "Abo",
            ButtonKind::Close => "Schließen",
            ButtonKind::Other => "weitere",
        }
    }
}

/// Ein erkanntes Overlay.
#[derive(Debug, Clone, Serialize)]
pub struct Overlay {
    /// Index in [`Graph::regions`].
    pub region: usize,
    pub kind: Fact<OverlayKind>,
    /// Buttons und Links darin: Index in [`Graph::controls`] und Einordnung,
    /// in Dokumentreihenfolge.
    pub buttons: Vec<(usize, Fact<ButtonKind>)>,
}

impl Overlay {
    /// Erreichbare Buttons dieser Art.
    pub fn of_kind(&self, kind: ButtonKind) -> impl Iterator<Item = usize> + '_ {
        self.buttons
            .iter()
            .filter(move |(_, k)| k.value == Some(kind))
            .map(|(i, _)| *i)
    }
}

/// Wörter, an denen ein Einwilligungs-Dialog zu erkennen ist (Wortanfänge).
const CONSENT: &[&str] = &[
    "cookie",
    "einwilligung",
    "einwilligen",
    "zustimmung",
    "datenschutz",
    "privatsphäre",
    "privacy",
    "consent",
    "tracking",
    "personenbezogen",
];
const NEWSLETTER: &[&str] = &["newsletter"];

/// Einordnung der Buttons, in dieser Vorrangfolge: Wer bezahlen lässt,
/// lehnt nicht kostenlos ab („Ablehnen und abonnieren“ ist ein Abo); wer
/// „nur notwendige … akzeptiert“, lehnt ab; „Einstellungen akzeptieren“
/// öffnet eher Einstellungen. Muster mit `$` am Ende gelten nur als ganzes
/// Wort („pur“, nicht „purpose“).
const PAY: &[&str] = &[
    "abo$",
    "abos$",
    "abonn",
    "pur$",
    "subscri",
    "werbefrei",
    "ad free",
    "bezahl",
    "kostenpflichtig",
];
const REJECT: &[&str] = &[
    "ablehnen",
    "lehne ab",
    "alle ablehnen",
    "nur notwendig",
    "nur erforderlich",
    "nur essenziell",
    "nur essentiell",
    "nur technisch notwendig",
    "verweigern",
    "nicht einwilligen",
    "nicht zustimmen",
    "ohne einwilligung",
    "ohne zustimmung",
    "reject",
    "decline",
    "deny",
    "refuse",
    "necessary only",
    "only necessary",
    "essential only",
    "only essential",
    "do not consent",
    "do not accept",
];
const ACCEPT: &[&str] = &[
    "akzeptier",
    "zustimmen",
    "stimme zu",
    "einwilligen",
    "willige ein",
    "einverstanden",
    "erlauben",
    "zulassen",
    "annehmen",
    "accept",
    "agree",
    "allow",
    // Nicht „Consenthub“ (Link zu Utiq auf bild.de, welt.de).
    "consent$",
];
/// Ganze Namen, die in einem Overlay zustimmen.
const ACCEPT_EXACT: &[&str] = &["ok", "okay", "verstanden", "got it", "alles klar"];
const SETTINGS: &[&str] = &[
    "einstellung",
    "anpassen",
    "optionen",
    "verwalten",
    "details",
    "präferenz",
    "auswahl",
    "settings",
    "preferences",
    "customi",
    "manage",
    "options",
];
const CLOSE: &[&str] = &["schließen", "close", "abbrechen", "cancel", "zurück"];
const CLOSE_EXACT: &[&str] = &["x", "×", "✕"];

/// Einordnung eines Namens mit dem Wort, das sie trägt.
pub fn button_kind(name: &str) -> (ButtonKind, Option<&'static str>) {
    let n = normalize(name);
    if let Some(exact) = CLOSE_EXACT.iter().find(|w| name.trim() == **w || n == **w) {
        return (ButtonKind::Close, Some(exact));
    }
    if let Some(exact) = ACCEPT_EXACT.iter().find(|w| n == **w) {
        return (ButtonKind::Accept, Some(exact));
    }
    for (kind, list) in [
        (ButtonKind::Pay, PAY),
        (ButtonKind::Reject, REJECT),
        (ButtonKind::Settings, SETTINGS),
        (ButtonKind::Accept, ACCEPT),
        (ButtonKind::Close, CLOSE),
    ] {
        if let Some(word) = hit(&n, list) {
            return (kind, Some(word));
        }
    }
    (ButtonKind::Other, None)
}

/// Darf dieser Button nicht als Schließen-Button gewählt werden? Sein Name
/// klingt nach Zustimmung oder Abo („Akzeptieren und schließen“), gleich
/// wie er sonst eingeordnet ist.
pub fn blocks_dismissal(name: &str) -> bool {
    let n = normalize(name);
    ACCEPT_EXACT.contains(&n.as_str())
        || hit(&n, ACCEPT).is_some() && hit(&n, REJECT).is_none()
        || hit(&n, PAY).is_some()
}

/// Erreichbare Overlays in Dokumentreihenfolge: Dialoge und nach
/// Einwilligung benannte Bereiche. Von verschachtelten Cookie-Dialogen zählt
/// der innerste (dort stehen die Buttons).
pub fn overlays(graph: &Graph) -> Vec<Overlay> {
    let found: Vec<Overlay> = graph
        .regions
        .iter()
        .enumerate()
        .filter(|(i, r)| graph.is_reachable(Some(*i), &r.node))
        .filter(|(_, r)| {
            matches!(r.role, Role::Dialog | Role::AlertDialog)
                || r.name
                    .as_deref()
                    .is_some_and(|n| hit(&normalize(n), CONSENT).is_some())
        })
        .map(|(i, _)| classify(graph, i))
        .collect();
    let consent = |o: &Overlay| o.kind.value == Some(OverlayKind::Consent);
    found
        .iter()
        .filter(|o| {
            !(consent(o)
                && found.iter().any(|inner| {
                    inner.region != o.region
                        && consent(inner)
                        && graph.within(Some(inner.region), o.region)
                }))
        })
        .cloned()
        .collect()
}

/// Der Cookie-Dialog der Seite, falls einer erkannt ist: der zuletzt
/// geöffnete modale, sonst der letzte in Dokumentreihenfolge.
pub fn consent(graph: &Graph) -> Option<Overlay> {
    let mut all: Vec<Overlay> = overlays(graph)
        .into_iter()
        .filter(|o| o.kind.value == Some(OverlayKind::Consent))
        .collect();
    let modal = all.iter().rposition(|o| graph.regions[o.region].modal);
    match modal {
        Some(i) => Some(all.swap_remove(i)),
        None => all.pop(),
    }
}

fn classify(graph: &Graph, region: usize) -> Overlay {
    let buttons: Vec<(usize, Fact<ButtonKind>)> = graph
        .controls
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            matches!(c.role, Role::Button | Role::Link)
                && graph.within(c.region, region)
                && graph.is_reachable(c.region, &c.node)
        })
        .map(|(i, c)| (i, button_fact(c)))
        .collect();

    let r = &graph.regions[region];
    let texts = graph
        .texts
        .iter()
        .filter(|t| graph.within(t.region, region))
        .map(|t| t.text.as_str());
    let signal = |list: &[&'static str]| {
        let mut evidence = Vec::new();
        if let Some((name, w)) = r
            .name
            .as_deref()
            .and_then(|n| Some((n, hit(&normalize(n), list)?)))
        {
            evidence.push(format!("Name „{name}“ enthält „{w}“"));
        }
        if let Some((text, w)) = texts
            .clone()
            .find_map(|t| Some((t, hit(&normalize(t), list)?)))
        {
            evidence.push(format!("Text „{}“ enthält „{w}“", shorten(text)));
        }
        evidence
    };

    let mut consent = signal(CONSENT);
    let kind = if !consent.is_empty() {
        let decision = buttons
            .iter()
            .find(|(_, k)| matches!(k.value, Some(ButtonKind::Accept | ButtonKind::Reject)));
        if let Some((i, _)) = decision {
            consent.push(format!("Button „{}“", graph.controls[*i].display_name()));
        }
        rule(OverlayKind::Consent, consent, "overlay-cookie")
    } else {
        let newsletter = signal(NEWSLETTER);
        if !newsletter.is_empty() {
            rule(OverlayKind::Newsletter, newsletter, "overlay-newsletter")
        } else {
            rule(
                OverlayKind::Dialog,
                vec![format!("Rolle {}", r.role)],
                "overlay-dialog",
            )
        }
    };
    Overlay {
        region,
        kind,
        buttons,
    }
}

/// Ein Link, der nach Ablehnen klingt, lehnt den Dialog nicht ab: Er führt
/// woanders hin (bild.de, welt.de: „für Utiq jetzt ablehnen“ öffnet die
/// Seite eines Drittanbieters). Nur Buttons lehnen ab.
fn button_fact(c: &Control) -> Fact<ButtonKind> {
    let (mut kind, word) = button_kind(c.name.value.as_deref().unwrap_or_default());
    let mut evidence = match word {
        Some(w) => format!("Name enthält „{w}“"),
        None => "kein Signalwort im Namen".into(),
    };
    if kind == ButtonKind::Reject && c.role == Role::Link {
        kind = ButtonKind::Other;
        evidence.push_str(", aber Link statt Button");
    }
    Fact {
        value: Some(kind),
        certainty: if word.is_some() {
            Certainty::Inferred
        } else {
            Certainty::Uncertain
        },
        source: Source::Rule("overlay-button".into()),
        confidence: None,
        evidence: vec![evidence],
    }
}

/// Ab zwei Hinweisen erschlossen, mit einem unsicher.
fn rule<T>(value: T, evidence: Vec<String>, id: &str) -> Fact<T> {
    Fact {
        value: Some(value),
        certainty: if evidence.len() >= 2 {
            Certainty::Inferred
        } else {
            Certainty::Uncertain
        },
        source: Source::Rule(id.into()),
        confidence: None,
        evidence,
    }
}

/// Erstes Muster, dessen Wörter in `normalized` aufeinander folgen; jedes
/// Wort des Musters muss einen Wortanfang treffen („nur notwendig“ trifft
/// „Nur notwendige Cookies“, „abo“ nicht „Labor“).
fn hit(normalized: &str, patterns: &[&'static str]) -> Option<&'static str> {
    let words: Vec<&str> = normalized.split(' ').collect();
    patterns
        .iter()
        .copied()
        .find(|p| {
            let pat: Vec<&str> = p.split(' ').collect();
            words.windows(pat.len()).any(|w| {
                w.iter()
                    .zip(&pat)
                    .all(|(word, p)| match p.strip_suffix('$') {
                        Some(exact) => *word == exact,
                        None => word.starts_with(p),
                    })
            })
        })
        .map(|p| p.trim_end_matches('$'))
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Höchstens acht Wörter für die Evidence.
fn shorten(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= 8 {
        words.join(" ")
    } else {
        format!("{} …", words[..8].join(" "))
    }
}

/// Hauptdokument mit Überschrift und Link, darüber ein modaler Dialog
/// „Privacy Center“ mit einem iframe; darin ein modaler Dialog „Ihre
/// Privatsphäre“ mit Text und den Buttons `buttons` (Knoten 20, 21, …).
/// Wie spiegel.de und bild.de (→ `plan/spezifikation/01`, „Live auf echten
/// Seiten“).
#[cfg(test)]
pub(crate) fn consent_page(buttons: &[&str]) -> relief_model::SemanticGraph {
    use relief_model::{NodeId, NodeRef, SemanticGraph, SemanticNode, SemanticTree, TreeId};
    let node = |id: i32, role: Role, name: &str, parent: Option<i32>, children: &[i32]| {
        let mut n = SemanticNode::new(NodeId(id), role);
        n.name = Fact::known(Some(name.to_string()).filter(|s| !s.is_empty()));
        n.parent = parent.map(NodeId);
        n.children = children.iter().copied().map(NodeId).collect();
        n.states.modal = matches!(n.role, Role::Dialog);
        n.dom_node_id = Some(id.into());
        n
    };
    let tree = |id: &str, nodes: Vec<SemanticNode>| {
        let mut t = SemanticTree::new(TreeId(id.into()));
        t.root = Some(nodes[0].id);
        t.nodes = nodes.into_iter().map(|n| (n.id, n)).collect();
        t
    };
    let mut iframe = node(5, Role::Iframe, "Privacy Center", Some(4), &[]);
    iframe.child_tree = Some(TreeId("frame".into()));
    let main = tree(
        "main",
        vec![
            node(1, Role::RootWebArea, "Nachrichten", None, &[2, 3, 4]),
            node(2, Role::Heading, "Nachrichten", Some(1), &[]),
            node(3, Role::Link, "Zum Artikel", Some(1), &[]),
            node(4, Role::Dialog, "Privacy Center", Some(1), &[5]),
            iframe,
        ],
    );
    let ids: Vec<i32> = (20..).take(buttons.len()).collect();
    let mut children = vec![11];
    children.extend(&ids);
    let mut nodes = vec![
        node(1, Role::RootWebArea, "", None, &[10]),
        node(10, Role::Dialog, "Ihre Privatsphäre", Some(1), &children),
        node(
            11,
            Role::StaticText,
            "Wir verwenden Cookies.",
            Some(10),
            &[],
        ),
    ];
    for (id, name) in ids.iter().zip(buttons) {
        nodes.push(node(*id, Role::Button, name, Some(10), &[]));
    }
    let mut frame = tree("frame", nodes);
    frame.data.parent = Some(NodeRef::new(TreeId("main".into()), NodeId(5)));
    SemanticGraph {
        root: Some(TreeId("main".into())),
        trees: [main, frame]
            .into_iter()
            .map(|t| (t.id.clone(), t))
            .collect(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use relief_model::{NodeId, TreeId};

    #[test]
    fn buttons_by_label() {
        use ButtonKind::*;
        let kind = |n| button_kind(n).0;
        assert_eq!(kind("Alle akzeptieren"), Accept);
        assert_eq!(kind("Einwilligen und weiter"), Accept);
        assert_eq!(kind("OK"), Accept);
        assert_eq!(kind("Alle ablehnen"), Reject);
        assert_eq!(kind("Nur notwendige Cookies akzeptieren"), Reject);
        assert_eq!(kind("Weiter ohne Einwilligung"), Reject);
        assert_eq!(kind("Jetzt abonnieren"), Pay);
        assert_eq!(kind("Jetzt BILD PUR abonnieren"), Pay);
        assert_eq!(kind("Ablehnen und Abo abschließen"), Pay);
        assert_eq!(kind("Einstellungen"), Settings);
        assert_eq!(kind("Einwilligung verwalten"), Settings);
        assert_eq!(kind("×"), Close);
        assert_eq!(kind("Purpose details"), Settings);
        assert_eq!(kind("Datenschutzerklärung"), Other);
        assert!(blocks_dismissal("Akzeptieren und schließen"));
        assert!(blocks_dismissal("Einstellungen akzeptieren und schließen"));
        assert!(!blocks_dismissal("Nur notwendige Cookies akzeptieren"));
        assert!(!blocks_dismissal("Schließen"));
    }

    #[test]
    fn innermost_consent_dialog_across_frames() {
        let g = Graph::build(&consent_page(&[
            "Einwilligen und weiter",
            "Jetzt abonnieren",
            "Einstellungen",
        ]));
        let all = overlays(&g);
        assert_eq!(all.len(), 1, "{all:?}");
        let o = consent(&g).unwrap();
        assert_eq!(g.regions[o.region].label(), "dialog „Ihre Privatsphäre“");
        assert_eq!(o.kind.value, Some(OverlayKind::Consent));
        // Text und Button: erschlossen, nie Known.
        assert_eq!(o.kind.certainty, Certainty::Inferred);
        assert_eq!(
            o.kind.evidence,
            [
                "Name „Ihre Privatsphäre“ enthält „privatsphäre“",
                "Text „Wir verwenden Cookies.“ enthält „cookie“",
                "Button „Einwilligen und weiter“"
            ]
        );
        assert_eq!(o.of_kind(ButtonKind::Reject).count(), 0);
        assert_eq!(o.of_kind(ButtonKind::Pay).count(), 1);
    }

    #[test]
    fn a_link_does_not_reject() {
        // bild.de, welt.de: „für Utiq jetzt ablehnen“ ist ein Link.
        let mut model = consent_page(&["Alle akzeptieren", "für Utiq jetzt ablehnen"]);
        let frame = model.trees.get_mut(&TreeId("frame".into())).unwrap();
        frame.nodes.get_mut(&NodeId(21)).unwrap().role = Role::Link;
        let g = Graph::build(&model);
        let o = consent(&g).unwrap();
        assert_eq!(o.of_kind(ButtonKind::Reject).count(), 0);
        assert_eq!(button_kind("Utiq Consenthub").0, ButtonKind::Other);
    }

    #[test]
    fn dialog_without_consent_words_is_just_a_dialog() {
        let mut model = consent_page(&["OK"]);
        let frame = model.trees.get_mut(&TreeId("frame".into())).unwrap();
        frame.nodes.get_mut(&NodeId(11)).unwrap().name =
            Fact::known(Some("Neue Funktionen verfügbar.".into()));
        frame.nodes.get_mut(&NodeId(10)).unwrap().name = Fact::known(Some("Neu".into()));
        let main = model.trees.get_mut(&TreeId("main".into())).unwrap();
        main.nodes.get_mut(&NodeId(4)).unwrap().name = Fact::known(Some("Hinweis".into()));
        let g = Graph::build(&model);
        assert!(consent(&g).is_none());
        let all = overlays(&g);
        assert_eq!(all.len(), 2);
        assert!(all
            .iter()
            .all(|o| o.kind.value == Some(OverlayKind::Dialog)));
    }
}
