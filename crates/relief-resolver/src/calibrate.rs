//! Kalibrierung: Trefferquote je Confidence-Band und Kosten je Anfrage und
//! Seite, auf der Stichprobe aus [`crate::sample`].
//!
//! Der Lauf fragt je Stichprobeneintrag den Anbieter mit genau der Eingabe,
//! die auch [`crate::resolve_node`] schickt, und prüft die Antwort streng.
//! Aus den Ergebnissen entsteht ein [`Report`]; eine Schwelle wird nur
//! vorgeschlagen, wenn genug Hypothesen darüber liegen
//! ([`MIN_SUPPORT`], [`TARGET_PRECISION`]).

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

use relief_ai_contract::{validate_hypotheses, ModelProvider, ModelRequest, Property, Usage};

use crate::anthropic::{price_per_mtok, user_message};
use crate::sample::{local_id, unnamed_controls, Sample};
use crate::NEIGHBOURHOOD_NODES;

/// Confidence-Bänder, untere Grenzen; das letzte reicht bis 1.
pub const BANDS: &[f32] = &[0.0, 0.5, 0.7, 0.8, 0.9];
/// Anteil richtiger Namen, den eine Schwelle mindestens erreichen muss
/// [Annahme: festzulegen mit der ersten Messung].
pub const TARGET_PRECISION: f64 = 0.9;
/// So viele Hypothesen müssen mindestens über einer Schwelle liegen, damit
/// sie vorgeschlagen wird.
pub const MIN_SUPPORT: usize = 10;

/// Was bei einem Stichprobeneintrag herauskam.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// Stufe ohne Modell oder Aufzeichnung ohne Eintrag.
    NoReply,
    /// Anbieterfehler (Netz, Key, Ablehnung, abgeschnitten).
    ProviderError(String),
    /// Ausgabe verletzt den Vertrag und wurde ganz verworfen.
    Invalid(String),
    /// Gültige Ausgabe, aber kein Name für den Knoten.
    NoHypothesis,
    Named {
        value: String,
        confidence: f32,
        hit: bool,
    },
}

/// Ergebnis eines Eintrags.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub recording: String,
    pub node: String,
    pub expected: String,
    /// Größe der Nutzernachricht (Auftrag + Ausschnitt) in Bytes.
    pub request_bytes: usize,
    /// Knoten im Ausschnitt.
    pub excerpt_nodes: usize,
    pub usage: Option<Usage>,
    pub model: Option<String>,
    pub answer: Answer,
}

/// Seite der Stichprobe: wie viele Controls dort ohne Namen sind.
#[derive(Debug, Clone)]
pub struct Page {
    pub recording: String,
    pub unnamed_controls: usize,
}

/// Alle Einträge der Stichprobe durchlaufen.
pub fn run(
    sample: &Sample,
    provider: &dyn ModelProvider,
) -> Result<(Vec<Outcome>, Vec<Page>), String> {
    let mut outcomes = Vec::new();
    let mut pages = BTreeMap::new();
    for item in &sample.items {
        if !pages.contains_key(&item.recording) {
            pages.insert(item.recording.clone(), sample.page(&item.recording)?);
        }
        let input = &pages[&item.recording];
        let id = local_id(input, item.node)
            .ok_or_else(|| format!("{}: Knoten {} fehlt", item.recording, item.node))?;
        let found = &input.nodes().iter().find(|n| n.id == id).unwrap();
        if found.name.is_some() || found.role != item.role {
            return Err(format!(
                "{} {id}: erwartet unbenannt mit Rolle {}, gefunden {} {:?}",
                item.recording, item.role, found.role, found.name
            ));
        }
        let excerpt = input.excerpt(&id, NEIGHBOURHOOD_NODES).unwrap();
        let excerpt_nodes = excerpt.nodes().len();
        let request = ModelRequest::resolve_missing(excerpt);
        let request_bytes = user_message(&request).len();
        let mut outcome = Outcome {
            recording: item.recording.clone(),
            node: id.clone(),
            expected: item.expected.clone(),
            request_bytes,
            excerpt_nodes,
            usage: None,
            model: None,
            answer: Answer::NoReply,
        };
        outcome.answer = match provider.complete(&request) {
            Ok(None) => Answer::NoReply,
            Err(e) => Answer::ProviderError(e.0),
            Ok(Some(reply)) => {
                outcome.usage = reply.usage;
                outcome.model = Some(reply.model.to_string());
                match validate_hypotheses(&reply.text, request.input(), &reply.model) {
                    Err(e) => Answer::Invalid(e.to_string()),
                    Ok(hs) => match hs.into_iter().find(|h| {
                        request.input().node_ref(&id) == Some(&h.node)
                            && h.property == Property::Name
                    }) {
                        None => Answer::NoHypothesis,
                        Some(h) => Answer::Named {
                            hit: item.matches(&h.value),
                            value: h.value,
                            confidence: h.confidence,
                        },
                    },
                }
            }
        };
        outcomes.push(outcome);
    }
    let pages = pages
        .into_iter()
        .map(|(recording, input)| Page {
            recording,
            unnamed_controls: unnamed_controls(&input).len(),
        })
        .collect();
    Ok((outcomes, pages))
}

/// Auswertung eines Laufs.
#[derive(Debug, Clone)]
pub struct Report {
    pub outcomes: Vec<Outcome>,
    pub pages: Vec<Page>,
}

/// Hypothesen mit Confidence in `[from, to)` (bis 1 einschließlich beim
/// letzten Band): (Anzahl, Treffer).
fn count(outcomes: &[Outcome], from: f32, to: f32) -> (usize, usize) {
    outcomes
        .iter()
        .filter_map(|o| match o.answer {
            Answer::Named {
                confidence, hit, ..
            } if confidence >= from && (confidence < to || to > 1.0) => Some(hit),
            _ => None,
        })
        .fold((0, 0), |(n, h), hit| (n + 1, h + usize::from(hit)))
}

impl Report {
    /// Kleinste Bandgrenze, ab der mindestens [`MIN_SUPPORT`] Hypothesen
    /// liegen und davon mindestens [`TARGET_PRECISION`] richtig sind.
    pub fn suggested_threshold(&self) -> Option<f32> {
        BANDS.iter().copied().find(|&t| {
            let (n, hits) = count(&self.outcomes, t, 2.0);
            n >= MIN_SUPPORT && hits as f64 / n as f64 >= TARGET_PRECISION
        })
    }
}

fn rate(hits: usize, n: usize) -> String {
    if n == 0 {
        "–".into()
    } else {
        format!("{:.0} %", 100.0 * hits as f64 / n as f64)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = &self.outcomes;
        let models: Vec<&str> = {
            let mut m: Vec<&str> = o.iter().filter_map(|x| x.model.as_deref()).collect();
            m.sort();
            m.dedup();
            m
        };
        writeln!(
            f,
            "Stichprobe: {} Einträge auf {} Seiten",
            o.len(),
            self.pages.len()
        )?;
        writeln!(
            f,
            "Modell: {}",
            if models.is_empty() {
                "keine Antwort mit Modellangabe".to_string()
            } else {
                models.join(", ")
            }
        )?;

        writeln!(f, "\nEinträge:")?;
        for x in o {
            let answer = match &x.answer {
                Answer::NoReply => "keine Antwort".to_string(),
                Answer::ProviderError(e) => format!("Anbieterfehler: {e}"),
                Answer::Invalid(e) => format!("verworfen: {e}"),
                Answer::NoHypothesis => "kein Name vorgeschlagen".to_string(),
                Answer::Named {
                    value,
                    confidence,
                    hit,
                } => format!(
                    "{} „{value}“ ({confidence:.2})",
                    if *hit { "Treffer" } else { "falsch" }
                ),
            };
            let tokens = x
                .usage
                .map(|u| format!(", {}+{} Tokens", u.input_tokens, u.output_tokens))
                .unwrap_or_default();
            writeln!(
                f,
                "  {} {} soll „{}“: {answer} [{} B, {} Knoten{tokens}]",
                x.recording, x.node, x.expected, x.request_bytes, x.excerpt_nodes
            )?;
        }

        let named = o
            .iter()
            .filter(|x| matches!(x.answer, Answer::Named { .. }))
            .count();
        let other = |p: fn(&Answer) -> bool| o.iter().filter(|x| p(&x.answer)).count();
        writeln!(
            f,
            "\nAntworten: {named} mit Namen, {} ohne Namen, {} verworfen, {} Anbieterfehler, {} ohne Antwort",
            other(|a| matches!(a, Answer::NoHypothesis)),
            other(|a| matches!(a, Answer::Invalid(_))),
            other(|a| matches!(a, Answer::ProviderError(_))),
            other(|a| matches!(a, Answer::NoReply)),
        )?;

        writeln!(f, "\nTrefferquote je Confidence-Band:")?;
        for (i, &from) in BANDS.iter().enumerate() {
            let to = BANDS.get(i + 1).copied().unwrap_or(2.0);
            let (n, hits) = count(o, from, to);
            let label = if to > 1.0 {
                format!("{from:.1}–1.0")
            } else {
                format!("{from:.1}–{to:.1}")
            };
            writeln!(f, "  {label}: {hits}/{n} ({})", rate(hits, n))?;
        }
        writeln!(
            f,
            "Ab Schwelle (Anteil richtig / Abdeckung der Stichprobe):"
        )?;
        for &t in BANDS {
            let (n, hits) = count(o, t, 2.0);
            writeln!(
                f,
                "  ≥ {t:.1}: {hits}/{n} ({}), {}",
                rate(hits, n),
                rate(n, o.len())
            )?;
        }
        match self.suggested_threshold() {
            Some(t) => writeln!(f, "Vorschlag Schwelle: {t:.1} (≥ {:.0} % richtig bei ≥ {MIN_SUPPORT} Hypothesen)", TARGET_PRECISION * 100.0)?,
            None => writeln!(
                f,
                "Vorschlag Schwelle: keiner (keine Grenze mit ≥ {MIN_SUPPORT} Hypothesen und ≥ {:.0} % richtig)",
                TARGET_PRECISION * 100.0
            )?,
        }

        let bytes: Vec<usize> = o.iter().map(|x| x.request_bytes).collect();
        writeln!(
            f,
            "\nAnfragegröße (Nutzernachricht): Mittel {} B, größte {} B; dazu Systemtext {} B und Schema",
            bytes.iter().sum::<usize>() / bytes.len().max(1),
            bytes.iter().max().copied().unwrap_or(0),
            crate::anthropic::SYSTEM_PROMPT.len(),
        )?;

        let usages: Vec<Usage> = o.iter().filter_map(|x| x.usage).collect();
        if usages.is_empty() {
            writeln!(f, "Tokens: keine gemeldet")?;
            return Ok(());
        }
        let (input, output) = usages.iter().fold((0, 0), |(i, out), u| {
            (i + u.input_tokens, out + u.output_tokens)
        });
        let n = usages.len() as u64;
        writeln!(
            f,
            "Tokens je Anfrage: Mittel {} Eingabe + {} Ausgabe; größte {} + {}",
            input / n,
            output / n,
            usages.iter().map(|u| u.input_tokens).max().unwrap_or(0),
            usages.iter().map(|u| u.output_tokens).max().unwrap_or(0),
        )?;
        let price = models
            .first()
            .and_then(|m| price_per_mtok(m.rsplit('@').next().unwrap_or(m)));
        let dollars = |i: u64, out: u64| {
            price
                .map(|(pi, po)| format!(", ≈ {:.4} $", (i as f64 * pi + out as f64 * po) / 1e6))
                .unwrap_or_default()
        };
        writeln!(
            f,
            "Kosten je Anfrage im Mittel{}",
            dollars(input / n, output / n).replacen(", ", ": ", 1)
        )?;
        writeln!(
            f,
            "Je Seite (Stichprobe; hochgerechnet auf alle unbenannten Controls der Seite):"
        )?;
        for page in &self.pages {
            let mine: Vec<Usage> = o
                .iter()
                .filter(|x| x.recording == page.recording)
                .filter_map(|x| x.usage)
                .collect();
            let (pi, po) = mine.iter().fold((0, 0), |(i, out), u| {
                (i + u.input_tokens, out + u.output_tokens)
            });
            let k = page.unnamed_controls as u64;
            let mut line = format!(
                "  {}: {} Anfragen, {pi}+{po} Tokens{}",
                page.recording,
                mine.len(),
                dollars(pi, po)
            );
            if !mine.is_empty() {
                let m = mine.len() as u64;
                let _ = write!(
                    line,
                    "; {k} unbenannte Controls ≈ {}+{} Tokens{}",
                    pi / m * k,
                    po / m * k,
                    dollars(pi / m * k, po / m * k)
                );
            }
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}
