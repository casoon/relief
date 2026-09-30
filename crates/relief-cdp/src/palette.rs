//! Befehlsleiste im Browser für die Nutzerstudie (Backlog 01b).
//!
//! `palette.js` wird in jedes Dokument eingefügt; Strg+Umschalt+Leertaste
//! öffnet sie. Eingaben kommen über die Binding `reliefCommand`, die Antwort
//! geht über `window.__reliefShow` zurück und wird dort in einer Live-Region
//! angezeigt. Riskante Aktionen werden mit „ja“ bestätigt.
//!
//! Protokoll: eine JSON-Zeile je Eingabe in `RELIEF_LOG` (Standard
//! `relief-protokoll.jsonl`) mit Zeit, Eingabe, Art des Ergebnisses und
//! Tastendrücken — ohne Seiteninhalte und ohne Antworttexte; dazu die Zeilen
//! des Security-Logs (`security`, ohne Werte und Namen).

use std::io::Write;

use anyhow::Result;
use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType, InsertTextParams,
};
use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
use chromiumoxide::cdp::js_protocol::runtime::{AddBindingParams, EventBindingCalled};
use chromiumoxide::listeners::EventStream;
use chromiumoxide::Page;
use futures::StreamExt;
use serde_json::{json, Value};

use crate::Session;

const SCRIPT: &str = include_str!("palette.js");
const BINDING: &str = "reliefCommand";

pub async fn run(browser: &Browser, url: &str) -> Result<()> {
    let (session, calls) = open(browser, url).await?;
    println!(
        "Relief-Befehlsleiste aktiv: Strg+Umschalt+Leertaste im Browserfenster. Beenden: Strg+C."
    );
    serve(session, calls).await
}

/// Wie ein Mensch bedienen: Tastenkürzel, Text, Enter — und die Antwort aus
/// der Leiste zurücklesen. Prüft den ganzen Weg über Binding und Live-Region.
pub async fn selftest(browser: &Browser, url: &str, inputs: &[&String]) -> Result<()> {
    let (session, calls) = open(browser, url).await?;
    let page = session.page.clone();
    let server = tokio::spawn(serve(session, calls));
    for input in inputs {
        key(&page, " ", "Space", 32, 2 | 8).await?;
        page.execute(InsertTextParams::new(input.as_str())).await?;
        key(&page, "Enter", "Enter", 13, 0).await?;
        let mut answer = String::new();
        for _ in 0..60 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            // Ergebnis steht in der Leiste oder, nach einer Aktion, im Hinweis.
            let r = page
                .evaluate("(() => { const h = document.querySelector('relief-palette'); if (!h) return ''; const r = h.shadowRoot; const t = r.querySelector('.toast'); return t.matches(':popover-open') ? t.textContent : r.querySelector('.out').textContent; })()")
                .await?;
            answer = r.into_value::<String>().unwrap_or_default();
            if !answer.is_empty() && answer != "Einen Moment …" {
                break;
            }
        }
        let active = page
            .evaluate("(() => { const a = document.activeElement; return a ? a.tagName + (a.id ? '#' + a.id : '') : ''; })()")
            .await?
            .into_value::<String>()
            .unwrap_or_default();
        println!("\n> {input}\n{answer}\n(Fokus: {active})");
    }
    server.abort();
    Ok(())
}

async fn open(browser: &Browser, url: &str) -> Result<(Session, EventStream<EventBindingCalled>)> {
    let page = browser.new_page("about:blank").await?;
    let calls = page.event_listener::<EventBindingCalled>().await?;
    page.execute(AddBindingParams::new(BINDING)).await?;
    page.execute(AddScriptToEvaluateOnNewDocumentParams::new(SCRIPT))
        .await?;
    let session = Session::open_page(
        browser,
        page,
        &crate::to_url(url, std::path::Path::new(".")),
    )
    .await?;
    Ok((session, calls))
}

async fn key(page: &Page, key: &str, code: &str, vk: i64, modifiers: i64) -> Result<()> {
    for kind in [
        DispatchKeyEventType::RawKeyDown,
        DispatchKeyEventType::Char,
        DispatchKeyEventType::KeyUp,
    ] {
        if kind == DispatchKeyEventType::Char && key != "Enter" {
            continue;
        }
        let mut b = DispatchKeyEventParams::builder()
            .r#type(kind.clone())
            .key(key)
            .code(code)
            .windows_virtual_key_code(vk)
            .modifiers(modifiers);
        if kind == DispatchKeyEventType::Char {
            b = b.text("\r");
        }
        page.execute(b.build().map_err(|e| anyhow::anyhow!(e))?)
            .await?;
    }
    Ok(())
}

async fn serve(mut session: Session, mut calls: EventStream<EventBindingCalled>) -> Result<()> {
    let log_path = std::env::var("RELIEF_LOG").unwrap_or_else(|_| "relief-protokoll.jsonl".into());
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    session.security_log = Some(log.try_clone()?);

    let mut last_keys = 0u64;
    while let Some(call) = calls.next().await {
        if call.name != BINDING {
            continue;
        }
        let payload: Value = serde_json::from_str(&call.payload).unwrap_or_default();
        let text = payload["text"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string();
        let keys = payload["keys"].as_u64().unwrap_or(0);

        // Die offene Leiste macht als modaler Dialog die Seite inert — ihr
        // Accessibility-Baum wäre leer. Deshalb vor jedem Befehl schließen und
        // Ruhe abwarten; der Browser gibt den Fokus dorthin zurück, wo er vor
        // der Leiste war, sodass auch Escape und Fokusbefehle die Seite treffen.
        session
            .page
            .evaluate("window.__reliefHide && window.__reliefHide()")
            .await
            .ok();
        session.live.settle().await;
        // „ja“, Zahl und „abbrechen“ beantwortet die Sitzung selbst
        // (`Session::pending_reply`).
        let (response, kind) = respond(&mut session, &text).await;
        let pending = matches!(kind, "bestätigung nötig" | "rückfrage");

        let js = if kind == "ausgeführt" {
            format!(
                "window.__reliefToast && window.__reliefToast({})",
                json!(response)
            )
        } else {
            format!(
                "window.__reliefShow && window.__reliefShow({}, {})",
                json!(response),
                pending
            )
        };
        session.page.evaluate(js).await.ok();

        writeln!(
            log,
            "{}",
            json!({
                "t": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() as u64,
                "eingabe": text,
                "ergebnis": kind,
                "tasten_seit_letzter_eingabe": keys.saturating_sub(last_keys),
                "tasten_gesamt": keys,
            })
        )?;
        last_keys = keys;
    }
    Ok(())
}

/// Befehl ausführen und die Art des Ergebnisses für das Protokoll bestimmen.
async fn respond(session: &mut Session, input: &str) -> (String, &'static str) {
    let response = session
        .handle(input)
        .await
        .unwrap_or_else(|e| format!("Fehler: {e}"));
    let kind = classify(&response);
    (response, kind)
}

fn classify(response: &str) -> &'static str {
    const KINDS: &[(&str, &str)] = &[
        ("Bestätigung nötig", "bestätigung nötig"),
        ("Mehrdeutig", "rückfrage"),
        ("Nichts gefunden", "nicht gefunden"),
        ("Kein ", "nicht gefunden"),
        ("Nicht verstanden", "nicht verstanden"),
        ("Abgelehnt", "abgelehnt"),
        ("Aktion fehlgeschlagen", "fehlgeschlagen"),
        ("Fehler", "fehler"),
        ("Focus", "ausgeführt"),
        ("Activate", "ausgeführt"),
        ("SetValue", "ausgeführt"),
        ("Select", "ausgeführt"),
        ("Escape", "ausgeführt"),
        ("NavigateTo", "ausgeführt"),
        ("Increment", "ausgeführt"),
        ("Decrement", "ausgeführt"),
        ("Scroll", "ausgeführt"),
    ];
    KINDS
        .iter()
        .find(|(prefix, _)| response.starts_with(prefix))
        .map(|(_, kind)| *kind)
        .unwrap_or("abfrage")
}
