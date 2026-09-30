//! Kalibrierlauf für den Resolver fehlender Namen.
//!
//! ```text
//! relief-resolver kalibrieren [--stichprobe DATEI] [--aufzeichnen DATEI] [--wiedergeben DATEI]
//! ```
//!
//! Anbieter, in dieser Reihenfolge:
//! - `--wiedergeben`: gespeicherte Antworten, ohne Key und Netz;
//! - mit Feature `anthropic`: Anthropic Messages API, Key aus
//!   `ANTHROPIC_API_KEY`, Modell aus `RELIEF_ANTHROPIC_MODEL`;
//! - sonst Stufe `none`: nur Anfragegrößen.

use std::path::PathBuf;
use std::process::ExitCode;

use relief_ai_contract::ModelProvider;
use relief_resolver::calibrate::{run, Report};
use relief_resolver::replay::{Recording, Replay, Replies};
use relief_resolver::sample::Sample;

const USAGE: &str =
    "relief-resolver kalibrieren [--stichprobe DATEI] [--aufzeichnen DATEI] [--wiedergeben DATEI]";

fn main() -> ExitCode {
    match cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Fehler: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cli() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("kalibrieren") {
        return Err(format!("Aufruf: {USAGE}"));
    }
    let mut sample = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spike/kalibrierung/fehlende-namen.json"
    ));
    let (mut record, mut replay) = (None, None);
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .map(PathBuf::from)
                .ok_or(format!("{arg}: Datei fehlt"))
        };
        match arg.as_str() {
            "--stichprobe" => sample = value()?,
            "--aufzeichnen" => record = Some(value()?),
            "--wiedergeben" => replay = Some(value()?),
            other => return Err(format!("unbekannt: {other}\nAufruf: {USAGE}")),
        }
    }
    let sample = Sample::load(&sample)?;

    let provider: Box<dyn ModelProvider> = match replay {
        Some(path) => {
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let replies: Replies =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            Box::new(Replay { replies })
        }
        None => live_provider()?,
    };
    let recording = Recording::new(provider.as_ref());
    let (outcomes, pages) = run(&sample, &recording)?;
    print!("{}", Report { outcomes, pages });

    if let Some(path) = record {
        let json = serde_json::to_string_pretty(&recording.into_replies()).unwrap();
        std::fs::write(&path, json + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
        println!("Antworten gespeichert: {}", path.display());
    }
    Ok(())
}

#[cfg(feature = "anthropic")]
fn live_provider() -> Result<Box<dyn ModelProvider>, String> {
    let provider = relief_resolver::anthropic::AnthropicProvider::from_env().map_err(|e| {
        format!(
            "{e}\nKey setzen: export {}=…  (Modell wählen: export {}=claude-sonnet-5)",
            relief_resolver::anthropic::KEY_ENV,
            relief_resolver::anthropic::MODEL_ENV
        )
    })?;
    eprintln!("Anbieter: Anthropic, Modell {}", provider.model());
    Ok(Box::new(provider))
}

#[cfg(not(feature = "anthropic"))]
fn live_provider() -> Result<Box<dyn ModelProvider>, String> {
    eprintln!(
        "Ohne Feature `anthropic`: Stufe none, nur Anfragegrößen. Mit Modell: \
         cargo run -p relief-resolver --features anthropic -- kalibrieren"
    );
    Ok(Box::new(relief_ai_contract::NoModel))
}
