//! Resolver und Kalibrierung auf der Stichprobe, nur mit Fake-Anbietern
//! (Stufe none und wiedergegebene, ausgedachte Antworten). Kein Netz.

use std::path::Path;

use relief_ai_contract::{Budget, Limits, ModelError, ModelRequest, NoModel, Usage};
use relief_model::Certainty;
use relief_resolver::anthropic::{request_body, user_message, DEFAULT_MODEL, SYSTEM_PROMPT};
use relief_resolver::calibrate::{run, Answer, Outcome, Report};
use relief_resolver::replay::{key, RecordedReply, Recording, Replay, Replies};
use relief_resolver::sample::{local_id, unnamed_controls, Sample};
use relief_resolver::{resolve_node, NEIGHBOURHOOD_NODES};

fn sample() -> Sample {
    Sample::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spike/kalibrierung/fehlende-namen.json"),
    )
    .unwrap()
}

fn reply(value: &str, node: &str, confidence: f32) -> Result<RecordedReply, String> {
    Ok(RecordedReply {
        model: "api:fake@1".into(),
        text: serde_json::json!({"hypotheses": [{
            "node": node, "property": "name", "value": value,
            "confidence": confidence, "evidence": ["test"]
        }]})
        .to_string(),
        usage: Some(Usage {
            input_tokens: 1000,
            output_tokens: 50,
        }),
    })
}

/// Antworten für jeden Stichprobeneintrag, erzeugt von `answer(index, soll)`.
fn replies(
    sample: &Sample,
    answer: impl Fn(usize, &str, &str) -> Result<RecordedReply, String>,
) -> Replies {
    sample
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let input = sample.page(&item.recording).unwrap();
            let id = local_id(&input, item.node).unwrap();
            let excerpt = input.excerpt(&id, NEIGHBOURHOOD_NODES).unwrap();
            (key(&excerpt), answer(i, &item.expected, &id))
        })
        .collect()
}

#[test]
fn stichprobe_passt_zu_den_aufnahmen() {
    let sample = sample();
    assert!(sample.items.len() >= 8);
    // `run` prüft je Eintrag: Knoten existiert, ist unbenannt, Rolle stimmt.
    let (outcomes, pages) = run(&sample, &NoModel).unwrap();
    assert!(outcomes.iter().all(|o| o.answer == Answer::NoReply));
    for page in &pages {
        assert!(page.unnamed_controls >= 1, "{}", page.recording);
    }
    for item in &sample.items {
        assert!(
            item.matches(&item.expected),
            "{}: Soll trifft nicht",
            item.expected
        );
    }
    // Anfragegröße: eng geschnitten, auch auf großen Seiten.
    let largest = outcomes.iter().map(|o| o.request_bytes).max().unwrap();
    assert!(largest < 12_000, "{largest} B");
}

#[test]
fn trefferquote_und_schwelle_aus_wiedergegebenen_antworten() {
    let sample = sample();
    // Die ersten 6 richtig mit 0.95, der Rest falsch mit 0.4.
    let replay = Replay {
        replies: replies(&sample, |i, soll, id| {
            if i < 6 {
                reply(soll, id, 0.95)
            } else {
                reply("Xyzzy", id, 0.4)
            }
        }),
    };
    let (outcomes, pages) = run(&sample, &replay).unwrap();
    // Ergebnisse doppelt, damit ein Band `MIN_SUPPORT` erreicht.
    let outcomes = [outcomes.clone(), outcomes].concat();
    let hits = outcomes
        .iter()
        .filter(|o| matches!(o.answer, Answer::Named { hit: true, .. }))
        .count();
    assert_eq!(hits, 12);
    let report = Report { outcomes, pages };
    assert_eq!(report.suggested_threshold(), Some(0.5));
    let text = report.to_string();
    assert!(text.contains("0.9–1.0: 12/12 (100 %)"), "{text}");
    assert!(text.contains("0.0–0.5: 0/4 (0 %)"), "{text}");
    assert!(
        text.contains("Tokens je Anfrage: Mittel 1000 Eingabe + 50 Ausgabe"),
        "{text}"
    );
}

#[test]
fn zu_wenig_treffer_ergeben_keine_schwelle() {
    let sample = sample();
    let replay = Replay {
        replies: replies(&sample, |i, soll, id| match i % 3 {
            0 => reply(soll, id, 0.9),
            1 => reply("Xyzzy", id, 0.9),
            _ => Err("HTTP 529, overloaded_error: Overloaded".into()),
        }),
    };
    let (outcomes, pages) = run(&sample, &replay).unwrap();
    assert!(outcomes
        .iter()
        .any(|o| matches!(o.answer, Answer::ProviderError(_))));
    assert_eq!(Report { outcomes, pages }.suggested_threshold(), None);
}

#[test]
fn verstoss_verwirft_die_antwort() {
    let sample = sample();
    // Name für einen Knoten, der nicht im Ausschnitt liegt.
    let replay = Replay {
        replies: replies(&sample, |_, soll, _| reply(soll, "t0:999999", 0.9)),
    };
    let (outcomes, _) = run(&sample, &replay).unwrap();
    assert!(outcomes
        .iter()
        .all(|o| matches!(&o.answer, Answer::Invalid(e) if e.contains("t0:999999"))));
}

#[test]
fn resolver_benennt_nur_den_fokus_und_bleibt_unsicher() {
    let sample = sample();
    let input = sample.page("02-shop-broken/01-shop-broken").unwrap();
    assert_eq!(unnamed_controls(&input), ["t0:15", "t0:18", "t0:21"]);
    let replay = Replay {
        replies: replies(&sample, |_, soll, id| reply(soll, id, 0.99)),
    };
    let mut budget = Budget::new(Limits::default());
    let h = resolve_node(&mut budget, &replay, &input, "t0:15")
        .unwrap()
        .unwrap();
    assert_eq!(h.value, "Warenkorb");
    // Keine gemessene Schwelle für das Modell: bleibt Uncertain.
    let fact = h.fact();
    assert_eq!(fact.certainty, Certainty::Uncertain);
    assert_eq!(fact.confidence, Some(0.99));

    assert!(resolve_node(&mut budget, &NoModel, &input, "t0:15")
        .unwrap()
        .is_none());
    assert!(resolve_node(&mut budget, &replay, &input, "t0:9999")
        .unwrap()
        .is_none());

    let broken = Replay {
        replies: replies(&sample, |_, _, id| reply("", id, 0.5)),
    };
    // Dieselbe Anfrage ein drittes Mal: Das Budget der Aufgabe fragt nicht mehr.
    assert!(matches!(
        resolve_node(&mut budget, &broken, &input, "t0:15"),
        Err(ModelError::Limit(_))
    ));
    let mut fresh = Budget::new(Limits::default());
    assert!(matches!(
        resolve_node(&mut fresh, &broken, &input, "t0:15"),
        Err(ModelError::Invalid(_))
    ));
}

#[test]
fn anfrage_an_die_api_enthaelt_nur_den_gefilterten_ausschnitt() {
    let sample = sample();
    let input = sample
        .page("10-real/02-en-wikipedia-org-wiki-accessibility")
        .unwrap();
    let excerpt = input.excerpt("t0:4062", NEIGHBOURHOOD_NODES).unwrap();
    let request = ModelRequest::resolve_missing(excerpt);
    let body = request_body(&request, DEFAULT_MODEL);
    assert_eq!(body["model"], DEFAULT_MODEL);
    assert_eq!(body["system"], SYSTEM_PROMPT);
    assert_eq!(body["output_config"]["format"]["type"], "json_schema");
    assert_eq!(
        body["output_config"]["format"]["schema"]["additionalProperties"],
        false
    );
    let message = body["messages"][0]["content"].as_str().unwrap();
    assert_eq!(message, user_message(&request));
    assert!(message.contains(r#""focus":"t0:4062""#));
    assert!(message.contains(r#""kind":"resolve_missing""#));
    // Ohne Fragment, ohne ganze Seite.
    assert!(!message.contains("#cite_ref"));
    assert!(message.len() * 50 < serde_json::to_string(&input).unwrap().len());
}

#[test]
fn aufzeichnung_gibt_dieselben_antworten_wieder() {
    let sample = sample();
    let original = replies(&sample, |i, soll, id| {
        if i % 2 == 0 {
            reply(soll, id, 0.8)
        } else {
            Err("HTTP 429, rate_limit_error: slow down".into())
        }
    });
    let replay = Replay {
        replies: original.clone(),
    };
    let recording = Recording::new(&replay);
    let (first, _) = run(&sample, &recording).unwrap();
    let recorded = recording.into_replies();
    assert_eq!(recorded, original);
    let (second, _) = run(&sample, &Replay { replies: recorded }).unwrap();
    let answers = |o: &[Outcome]| o.iter().map(|x| x.answer.clone()).collect::<Vec<_>>();
    assert_eq!(answers(&first), answers(&second));
}
