//! Overlays erkennen und einordnen: Consent-, Newsletter- und andere
//! Dialoge (→ `plan/spezifikation/05`, „Overlay- und Consent-Dialoge“).
//!
//! Grundlage ist die Modalität je Frame aus dem Graph (→
//! [`Graph::is_reachable`]): Ein Overlay ist ein erreichbarer Dialog oder
//! ein benannter Bereich, dessen Name nach Einwilligung klingt; seine
//! Buttons und Links sind alle Bedienelemente darin, auch in einem iframe
//! darunter (Consent-iframes auf spiegel.de, bild.de). Ohne einen solchen
//! Bereich kann die Seite selbst ein Cookie-Hinweis sein (golem.de:
//! Einwilligungsseite ohne Dialog, → [`consent_page_overlay`]).
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
    /// Speichert die Auswahl auf der zweiten Ebene („Auswahl speichern“,
    /// „Einstellungen anwenden“): öffnet keine Einstellungen.
    Save,
    /// Zustimmen oder Ablehnen **je Zweck** auf der zweiten Ebene: derselbe
    /// Name steht mehrfach im Overlay (spiegel.de dreimal „Ablehnen“,
    /// bild.de „Einwilligen“ je Zweck). Weder Ablehnen noch Zustimmen des
    /// Ganzen; Relief wählt ihn nie selbst.
    Purpose,
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
            ButtonKind::Save => "Speichern",
            ButtonKind::Purpose => "vermutlich je Zweck",
            ButtonKind::Pay => "Abo",
            ButtonKind::Close => "Schließen",
            ButtonKind::Other => "weitere",
        }
    }
}

/// Ein erkanntes Overlay.
#[derive(Debug, Clone, Serialize)]
pub struct Overlay {
    /// Index in [`Graph::regions`]; `None`: die Seite selbst ist der
    /// Cookie-Hinweis (Einwilligungsseite ohne Dialog).
    pub region: Option<usize>,
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
/// Zusammen mit einem Einstellungswort: speichert die Auswahl, statt
/// Einstellungen zu öffnen. Allein nicht („Speichern von oder Zugriff auf
/// Informationen“ ist ein Zweck auf bild.de).
const SAVE: &[&str] = &[
    "speicher",
    "anwenden",
    "übernehm",
    "bestätig",
    "save",
    "apply",
    "confirm",
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
            if kind == ButtonKind::Settings {
                if let Some(save) = hit(&n, SAVE) {
                    return (ButtonKind::Save, Some(save));
                }
            }
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
/// Einwilligung benannte Bereiche. Ein solcher Bereich zählt nur mit einem
/// Button, der zustimmt oder ablehnt; sonst ist er Gliederung (Wikipedia:
/// Abschnitt „Session cookie“ als benanntes `section`). Von verschachtelten
/// Cookie-Dialogen zählt der innerste (dort stehen die Buttons). Ist keiner
/// davon ein Cookie-Dialog, kommt eine Einwilligungsseite ohne Dialog hinzu,
/// falls die Seite eine ist ([`consent_page_overlay`]).
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
        .filter(|o| {
            o.region
                .is_some_and(|r| matches!(graph.regions[r].role, Role::Dialog | Role::AlertDialog))
                || o.buttons.iter().any(|(_, k)| {
                    matches!(
                        k.value,
                        Some(ButtonKind::Accept | ButtonKind::Reject | ButtonKind::Purpose)
                    )
                })
        })
        .collect();
    let consent = |o: &Overlay| o.kind.value == Some(OverlayKind::Consent);
    let mut kept: Vec<Overlay> = found
        .iter()
        .filter(|o| {
            !(consent(o)
                && found.iter().any(|inner| {
                    inner.region != o.region
                        && consent(inner)
                        && o.region
                            .is_some_and(|outer| graph.within(inner.region, outer))
                }))
        })
        .cloned()
        .collect();
    if !kept.iter().any(consent) {
        kept.extend(consent_page_overlay(graph));
    }
    kept
}

/// Die Seite selbst als Cookie-Hinweis, ohne `dialog` und ohne nach
/// Einwilligung benannten Bereich (golem.de: ganze Seite mit „Cookies
/// zustimmen“, „Zustimmen und weiter“ und „Zu Golem pur“).
///
/// Bewusst eng, damit gewöhnliche Seiten mit Datenschutz-Links oder einem
/// Artikel über Cookies nicht als Hinweis gelten: Es braucht einen
/// erreichbaren **Button**, der zustimmt, in einem Abschnitt, dessen
/// **Überschrift** ein Einwilligungswort trägt. Ein Link oder Fließtext mit
/// „Datenschutz“ reicht nicht. Die Buttons des Hinweises sind die
/// erreichbaren Buttons und Links im selben Bereich wie der
/// Zustimmen-Button (golem.de: außerhalb von Bereichen, ohne das Logo im
/// `banner`).
pub fn consent_page_overlay(graph: &Graph) -> Option<Overlay> {
    let (accept, heading, word) = graph.controls.iter().find_map(|c| {
        if c.role != Role::Button || !graph.is_reachable(c.region, &c.node) {
            return None;
        }
        if button_kind(c.name.value.as_deref().unwrap_or_default()).0 != ButtonKind::Accept {
            return None;
        }
        let h = &graph.headings[c.heading?];
        if !graph.is_reachable(h.region, &h.node) {
            return None;
        }
        Some((c, h, hit(&normalize(&h.text), CONSENT)?))
    })?;
    let buttons = graph
        .controls
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            matches!(c.role, Role::Button | Role::Link)
                && c.region == accept.region
                && graph.is_reachable(c.region, &c.node)
        })
        .map(|(i, c)| (i, button_fact(graph, c, None)))
        .collect();
    let buttons = mark_purposes(graph, buttons);
    Some(Overlay {
        region: None,
        kind: rule(
            OverlayKind::Consent,
            vec![
                format!("Überschrift „{}“ enthält „{word}“", heading.text),
                format!("Button „{}“ darunter", accept.display_name()),
            ],
            "overlay-cookie-seite",
        ),
        buttons,
    })
}

/// Der Cookie-Dialog der Seite, falls einer erkannt ist: der zuletzt
/// geöffnete modale, sonst der letzte in Dokumentreihenfolge.
pub fn consent(graph: &Graph) -> Option<Overlay> {
    let mut all: Vec<Overlay> = overlays(graph)
        .into_iter()
        .filter(|o| o.kind.value == Some(OverlayKind::Consent))
        .collect();
    let modal = all
        .iter()
        .rposition(|o| o.region.is_some_and(|r| graph.regions[r].modal));
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
        .map(|(i, c)| (i, button_fact(graph, c, Some(region))))
        .collect();
    let buttons = mark_purposes(graph, buttons);

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
        let decision = buttons.iter().find(|(_, k)| {
            matches!(
                k.value,
                Some(ButtonKind::Accept | ButtonKind::Reject | ButtonKind::Purpose)
            )
        });
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
        region: Some(region),
        kind,
        buttons,
    }
}

/// Ein Link, der nach Ablehnen klingt, lehnt den Dialog nicht ab: Er führt
/// woanders hin (bild.de, welt.de: „für Utiq jetzt ablehnen“ öffnet die
/// Seite eines Drittanbieters). Nur Buttons lehnen ab.
///
/// Ein Button ohne Signalwort oder mit „Ablehnen“ gilt als Abo, wenn die
/// Überschrift seines Abschnitts nach Abo klingt (sueddeutsche.de: „Jetzt
/// testen“ unter „Weiter mit SZ Plus-Abo“). Die Überschrift zählt nur, wenn
/// sie im Overlay `region` liegt (`None`: Einwilligungsseite, jede). Das
/// macht einen Button nie wählbar, sondern nimmt ihn höchstens aus dem
/// Ablehnen heraus; deshalb nur `Uncertain`.
fn button_fact(graph: &Graph, c: &Control, region: Option<usize>) -> Fact<ButtonKind> {
    let (mut kind, word) = button_kind(c.name.value.as_deref().unwrap_or_default());
    let mut evidence = match word {
        Some(w) => format!("Name enthält „{w}“"),
        None => "kein Signalwort im Namen".into(),
    };
    let mut certainty = if word.is_some() {
        Certainty::Inferred
    } else {
        Certainty::Uncertain
    };
    if kind == ButtonKind::Reject && c.role == Role::Link {
        kind = ButtonKind::Other;
        evidence.push_str(", aber Link statt Button");
    }
    let section = c
        .heading
        .map(|h| &graph.headings[h])
        .filter(|h| region.is_none_or(|r| graph.within(h.region, r)));
    if let Some((h, w)) = section.and_then(|h| Some((h, hit(&normalize(&h.text), PAY)?))) {
        if c.role == Role::Button && matches!(kind, ButtonKind::Other | ButtonKind::Reject) {
            kind = ButtonKind::Pay;
            certainty = Certainty::Uncertain;
            evidence.push_str(&format!(", aber Abschnitt „{}“ enthält „{w}“", h.text));
        }
    }
    Fact {
        value: Some(kind),
        certainty,
        source: Source::Rule("overlay-button".into()),
        confidence: None,
        evidence: vec![evidence],
    }
}

/// Zweck-Buttons der zweiten Ebene: Ein **Button**, der zustimmt oder
/// ablehnt, gilt als „je Zweck“, wenn ein anderer Button im selben Overlay
/// genauso heißt (spiegel.de: sechsmal „Zustimmen“, dreimal „Ablehnen“
/// unter je einem Zweck; bild.de: „Einwilligen“ und „Ablehnen“ je Zweck).
/// Wer das Ganze ablehnt oder zustimmt, steht einmal da („Allen
/// zustimmen“). Das nimmt Buttons nur aus Ablehnen und Zustimmen heraus und
/// macht nichts wählbar; deshalb `Uncertain`, und lieber einmal zu oft
/// (zwei gleiche „Alle ablehnen“ auf einer Ebene werden ebenfalls nicht
/// von selbst geklickt).
fn mark_purposes(
    graph: &Graph,
    mut buttons: Vec<(usize, Fact<ButtonKind>)>,
) -> Vec<(usize, Fact<ButtonKind>)> {
    let key = |i: usize| {
        let c = &graph.controls[i];
        (c.role == Role::Button)
            .then(|| normalize(c.name.value.as_deref().unwrap_or_default()))
            .filter(|n| !n.is_empty())
    };
    let decisions: Vec<(usize, String)> = buttons
        .iter()
        .filter(|(_, k)| matches!(k.value, Some(ButtonKind::Accept | ButtonKind::Reject)))
        .filter_map(|(i, _)| Some((*i, key(*i)?)))
        .collect();
    for (i, fact) in &mut buttons {
        let Some((_, name)) = decisions.iter().find(|(j, _)| j == i) else {
            continue;
        };
        let count = decisions.iter().filter(|(_, n)| n == name).count();
        if count >= 2 {
            fact.value = Some(ButtonKind::Purpose);
            fact.certainty = Certainty::Uncertain;
            fact.evidence[0].push_str(&format!(
                ", aber {count}-mal gleich benannt: vermutlich je Zweck"
            ));
        }
    }
    buttons
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
        assert_eq!(kind("Auswahl speichern"), Save);
        assert_eq!(kind("Einstellungen anwenden"), Save);
        // Ohne Einstellungswort kein Speichern (Zweck auf bild.de).
        assert_eq!(
            kind("Speichern von oder Zugriff auf Informationen auf einem Endgerät"),
            Other
        );
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
        assert_eq!(
            g.regions[o.region.unwrap()].label(),
            "dialog „Ihre Privatsphäre“"
        );
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
    fn purpose_buttons_on_the_second_level() {
        // Wie spiegel.de: Zustimmen je Zweck, Ablehnen nur bei manchen.
        let g = Graph::build(&consent_page(&[
            "Zustimmen",
            "Zustimmen",
            "Ablehnen",
            "Zustimmen",
            "Ablehnen",
            "Einstellungen anwenden",
            "Allen zustimmen",
        ]));
        let o = consent(&g).unwrap();
        assert_eq!(o.of_kind(ButtonKind::Purpose).count(), 5);
        assert_eq!(o.of_kind(ButtonKind::Reject).count(), 0);
        assert_eq!(o.of_kind(ButtonKind::Accept).count(), 1);
        assert_eq!(o.of_kind(ButtonKind::Save).count(), 1);
        let (_, fact) = &o.buttons[2];
        assert_eq!(fact.value, Some(ButtonKind::Purpose));
        assert_eq!(fact.certainty, Certainty::Uncertain);
        assert_eq!(
            fact.evidence,
            ["Name enthält „ablehnen“, aber 2-mal gleich benannt: vermutlich je Zweck"]
        );
        // Ein einzelnes „Ablehnen“ bleibt Ablehnen.
        let g = Graph::build(&consent_page(&["Zustimmen", "Ablehnen"]));
        let o = consent(&g).unwrap();
        assert_eq!(o.of_kind(ButtonKind::Reject).count(), 1);
        assert_eq!(o.of_kind(ButtonKind::Purpose).count(), 0);
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

    /// Eine Seite ohne Dialog aus (Rolle, Name) unter der Wurzel.
    fn flat_page(items: &[(Role, &str)]) -> relief_model::SemanticGraph {
        use relief_model::{SemanticGraph, SemanticNode, SemanticTree};
        let mut root = SemanticNode::new(NodeId(1), Role::RootWebArea);
        root.name = Fact::known(Some("Seite".into()));
        let mut t = SemanticTree::new(TreeId("main".into()));
        t.root = Some(NodeId(1));
        for (i, (role, name)) in items.iter().enumerate() {
            let id = NodeId(10 + i as i32);
            let mut n = SemanticNode::new(id, role.clone());
            n.name = Fact::known(Some(name.to_string()));
            n.parent = Some(NodeId(1));
            n.dom_node_id = Some(id.0.into());
            root.children.push(id);
            t.nodes.insert(id, n);
        }
        t.nodes.insert(NodeId(1), root);
        SemanticGraph {
            root: Some(TreeId("main".into())),
            trees: [(t.id.clone(), t)].into_iter().collect(),
            ..Default::default()
        }
    }

    #[test]
    fn consent_page_without_dialog() {
        // Wie golem.de: Einwilligungsseite ohne `dialog`.
        let g = Graph::build(&flat_page(&[
            (Role::Heading, "Willkommen!"),
            (Role::Heading, "Cookies zustimmen"),
            (Role::Button, "Zustimmen und weiter"),
            (Role::Link, "Datenschutzerklärung"),
            (Role::Heading, "… oder Magazin pur bestellen"),
            (Role::Link, "Zu Magazin pur"),
            (Role::Link, "Impressum"),
        ]));
        let o = consent(&g).unwrap();
        assert_eq!(o.region, None);
        assert_eq!(o.kind.value, Some(OverlayKind::Consent));
        assert_eq!(o.kind.certainty, Certainty::Inferred);
        assert_eq!(
            o.kind.evidence,
            [
                "Überschrift „Cookies zustimmen“ enthält „cookie“",
                "Button „Zustimmen und weiter“ darunter"
            ]
        );
        assert_eq!(o.of_kind(ButtonKind::Accept).count(), 1);
        assert_eq!(o.of_kind(ButtonKind::Pay).count(), 1);
        assert_eq!(o.of_kind(ButtonKind::Reject).count(), 0);
    }

    #[test]
    fn ordinary_pages_are_no_consent_page() {
        // Datenschutz-Link und Zustimmen-Button, aber keine
        // Einwilligungs-Überschrift darüber.
        let g = Graph::build(&flat_page(&[
            (Role::Heading, "Kontakt"),
            (Role::Button, "Bedingungen akzeptieren"),
            (Role::Link, "Datenschutz"),
            (Role::Link, "Cookie-Richtlinie"),
        ]));
        assert!(overlays(&g).is_empty());
        // Überschrift über Cookies, aber nur Links darunter (Artikel).
        let g = Graph::build(&flat_page(&[
            (Role::Heading, "Cookie-Banner richtig bauen"),
            (Role::Link, "Alle akzeptieren"),
            (Role::Button, "Teilen"),
        ]));
        assert!(overlays(&g).is_empty());
        // Benannter Abschnitt „Session cookie“ (Wikipedia) ohne Button, der
        // zustimmt oder ablehnt: Gliederung, kein Cookie-Hinweis.
        let mut model = flat_page(&[(Role::Region, "Session cookie"), (Role::Link, "Quelle")]);
        let tree = model.trees.get_mut(&TreeId("main".into())).unwrap();
        tree.nodes.get_mut(&NodeId(1)).unwrap().children = vec![NodeId(10)];
        tree.nodes.get_mut(&NodeId(10)).unwrap().children = vec![NodeId(11)];
        tree.nodes.get_mut(&NodeId(11)).unwrap().parent = Some(NodeId(10));
        let g = Graph::build(&model);
        assert_eq!(g.regions.len(), 1);
        assert!(overlays(&g).is_empty());
    }

    #[test]
    fn button_under_subscription_heading_is_pay() {
        // Wie sueddeutsche.de: „Jetzt testen“ unter „Weiter mit SZ Plus-Abo“.
        let mut model = consent_page(&["Ich bin einverstanden", "Jetzt testen"]);
        let frame = model.trees.get_mut(&TreeId("frame".into())).unwrap();
        let mut heading = relief_model::SemanticNode::new(NodeId(12), Role::Heading);
        heading.name = Fact::known(Some("Weiter mit SZ Plus-Abo".into()));
        heading.parent = Some(NodeId(10));
        frame.nodes.insert(NodeId(12), heading);
        let dialog = frame.nodes.get_mut(&NodeId(10)).unwrap();
        dialog.children = vec![NodeId(11), NodeId(20), NodeId(12), NodeId(21)];
        let g = Graph::build(&model);
        let o = consent(&g).unwrap();
        let (_, fact) = &o.buttons[o.buttons.len() - 1];
        assert_eq!(fact.value, Some(ButtonKind::Pay));
        assert_eq!(fact.certainty, Certainty::Uncertain);
        assert_eq!(
            fact.evidence,
            ["kein Signalwort im Namen, aber Abschnitt „Weiter mit SZ Plus-Abo“ enthält „abo“"]
        );
        // Eine Überschrift vor dem Dialog zählt nicht.
        let mut model = consent_page(&["Jetzt testen"]);
        let main = model.trees.get_mut(&TreeId("main".into())).unwrap();
        main.nodes.get_mut(&NodeId(2)).unwrap().name = Fact::known(Some("Unsere Abos".into()));
        let g = Graph::build(&model);
        let o = consent(&g).unwrap();
        let (i, _) = o.buttons[0];
        assert_eq!(
            g.headings[g.controls[i].heading.unwrap()].text,
            "Unsere Abos"
        );
        assert_eq!(o.of_kind(ButtonKind::Pay).count(), 0);
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
