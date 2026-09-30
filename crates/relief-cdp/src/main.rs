//! Relief CDP-Spike (Phase 0a): AXTree → Modell → Interaction Graph → Aktion.
//!
//! ```text
//! relief-cdp run <aufgaben.txt>... [--headful]   Aufgabendateien abarbeiten
//! relief-cdp repl <url> [--headful]              interaktiv
//! relief-cdp measure <url>... [--repeat N]       Zeiten und Graph-Stabilität
//! relief-cdp record <aufgaben.txt>... [--out dir]  Aufnahmen als Fixtures speichern
//! relief-cdp palette <url>                       Befehlsleiste im Browser (immer sichtbar)
//! ```
//!
//! Aufgabendatei: `url: …`, `do: …` (mit `!` davor bestätigt), `expect: …`
//! (Teilstring der letzten Antwort), `assert: …` (Formular-Zusicherung, Befunde
//! als Antwort → `assertions.rs`), `#` Kommentar.

mod act;
mod assertions;
mod capture;
mod live;
mod palette;
mod record;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use a11y_perception::AXSnapshot;
use anyhow::{bail, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Page;
use futures::StreamExt;
use relief_interaction::{
    command, current_place, dismissal, parse, plan_navigation, plan_on_page, resolve,
    resolve_inflected, resolve_place, respond, step_field, step_heading, ActionKind, ActionPlan,
    Command, Control, Dismissal, Graph, Place, PlaceResolution, Resolution, ScrollDirection, Step,
};
use relief_model::{perception, NodeRef, SemanticGraph, TreeId};
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var_os("RUST_LOG").is_some() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .init();
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let headful = args.iter().any(|a| a == "--headful");
    let repeat = args
        .iter()
        .position(|a| a == "--repeat")
        .and_then(|i| args.get(i + 1)?.parse().ok())
        .unwrap_or(3);
    let positional: Vec<&String> = args
        .iter()
        .enumerate()
        .filter(|(i, a)| {
            !a.starts_with("--")
                && (*i == 0 || !matches!(args[i - 1].as_str(), "--repeat" | "--out"))
        })
        .map(|(_, a)| a)
        .collect();
    let Some((mode, rest)) = positional.split_first() else {
        bail!("Aufruf: relief-cdp run|repl|measure …");
    };

    let headful = headful || mode.as_str() == "palette";
    let (mut browser, handler) = launch(headful).await?;
    let result = match mode.as_str() {
        "run" => run_files(&browser, rest).await,
        "repl" => {
            repl(
                &browser,
                rest.first().map(|s| s.as_str()).unwrap_or("about:blank"),
            )
            .await
        }
        "measure" => measure(&browser, rest, repeat).await,
        "record" => {
            let out = args
                .iter()
                .position(|a| a == "--out")
                .and_then(|i| args.get(i + 1))
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("spike/recordings"));
            record::record_files(&browser, rest, &out).await
        }
        "palette" => {
            palette::run(
                &browser,
                rest.first().map(|s| s.as_str()).unwrap_or("about:blank"),
            )
            .await
        }
        "palette-selftest" => match rest.split_first() {
            Some((url, inputs)) => palette::selftest(&browser, url, inputs).await,
            None => Err(anyhow::anyhow!(
                "Aufruf: relief-cdp palette-selftest <url> <eingabe>..."
            )),
        },
        other => Err(anyhow::anyhow!("unbekannter Modus {other}")),
    };
    browser.close().await.ok();
    handler.abort();
    result
}

async fn launch(headful: bool) -> Result<(Browser, tokio::task::JoinHandle<()>)> {
    let mut config = BrowserConfig::builder().window_size(1280, 900);
    config = if headful {
        config.with_head()
    } else {
        config.new_headless_mode()
    };
    // Fremd-Origin-iframes im selben Prozess halten, damit `getFullAXTree`
    // mit `frameId` sie erreicht (nur Spike; der Fork hat alle Frames im
    // Browser-Prozess-AXTree).
    config = config.arg("--disable-site-isolation-trials");
    // Die automatische Erkennung greift ggf. einen verwaisten Wrapper;
    // `CHROME` setzt den Pfad explizit.
    if let Some(path) = std::env::var_os("CHROME").map(PathBuf::from).or_else(|| {
        let mac = PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
        mac.exists().then_some(mac)
    }) {
        config = config.chrome_executable(path);
    }
    let (browser, mut handler) =
        Browser::launch(config.build().map_err(|e| anyhow::anyhow!(e))?).await?;
    let task = tokio::spawn(async move { while handler.next().await.is_some() {} });
    Ok((browser, task))
}

/// Eine geöffnete Seite mit aktueller Aufnahme, Modell und Graph.
struct Session {
    page: Page,
    live: live::Live,
    snapshot: AXSnapshot,
    /// Tree-ID des Hauptdokuments. CDP kennt keine; der Host vergibt eine
    /// neue bei jeder Navigation und jedem `DOM.documentUpdated`, damit wie
    /// in Chromium über Dokumentgrenzen hinweg kein Knoten zugeordnet wird.
    document: TreeId,
    /// Bisher vergebene Tree-IDs.
    documents: u64,
    model: SemanticGraph,
    graph: Graph,
    /// Jeden übersprungenen Neuaufbau gegen einen Vollsnapshot prüfen
    /// (`RELIEF_VERIFY=1`).
    verify: bool,
    /// Messzeile zum letzten Befehl.
    last_stats: Option<String>,
    /// Ruhe nach dem Laden (für `measure`).
    opened: live::Settle,
}

impl Session {
    async fn open(browser: &Browser, url: &str) -> Result<Self> {
        Self::open_page(browser.new_page("about:blank").await?, url).await
    }

    /// Auf einer vorbereiteten Seite (z. B. mit eingefügter Befehlsleiste)
    /// laden. `new_page(url)` + `wait_for_navigation` kehrt teils vor dem
    /// Commit des eigentlichen Dokuments zurück; `goto` wartet auf das Laden.
    async fn open_page(page: Page, url: &str) -> Result<Self> {
        // Vor dem Laden starten, damit die Anfragen des Ladens als ausstehend
        // zählen („Seite ruht“ auch über das Netz).
        let mut live = live::Live::start(&page).await?;
        // Manche Seiten erreichen `load` nie innerhalb des Zeitlimits von
        // chromiumoxide (tagesschau.de): dann mit dem aktuellen Stand weiter.
        if let Err(e) = page.goto(url).await {
            eprintln!("Laden nicht abgeschlossen ({e}), weiter mit dem aktuellen Stand");
        }
        // Kurz nach dem Laden hat manche Seite noch kein Dokument mit Inhalt.
        // Bis dahin neu versuchen und den DOM-Agenten jeweils an das aktuelle
        // Dokument hängen.
        let mut attempt = 0;
        let (snapshot, opened) = loop {
            live.attach().await?;
            let opened = live.settle().await;
            live.take_dirty().await?;
            let snapshot = capture_retry(&page, "initial").await?;
            attempt += 1;
            if snapshot.tree.len() > MIN_NODES || attempt == 6 {
                break (snapshot, opened);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        };
        // Das geladene Dokument ist Dokument 1, kein ersetztes.
        live.take_new_document();
        let document = document_id(1);
        let model = perception::from_snapshot(&snapshot, &document);
        let graph = Graph::build(&model);
        Ok(Session {
            page,
            live,
            snapshot,
            document,
            documents: 1,
            model,
            graph,
            verify: std::env::var_os("RELIEF_VERIFY").is_some(),
            last_stats: None,
            opened,
        })
    }

    /// Stand nachziehen. `force` nach eigenen Aktionen, weil nicht jede
    /// Wirkung eine DOM-Mutation auslöst.
    async fn update(&mut self, force: bool, settle: Option<live::Settle>) -> Result<()> {
        let dirty = self.live.take_dirty().await?;
        let replaced = self.live.take_new_document();
        let navigated = self.page.url().await?.unwrap_or_default() != self.snapshot.url;
        let started = Instant::now();
        let mode = if navigated || dirty || force {
            if navigated {
                self.page.wait_for_navigation().await.ok();
                // Neues Dokument: der DOM-Agent muss es neu übertragen.
                self.live.attach().await?;
                self.live.settle().await;
                self.live.take_dirty().await?;
                self.live.take_new_document();
            }
            if navigated || replaced {
                self.documents += 1;
                self.document = document_id(self.documents);
            }
            self.snapshot = capture_retry(&self.page, "current").await?;
            self.model = perception::from_snapshot(&self.snapshot, &self.document);
            self.graph = Graph::build(&self.model);
            let why = if navigated {
                "Navigation"
            } else if replaced {
                "Dokument ersetzt"
            } else if dirty {
                "DOM geändert"
            } else {
                "eigene Aktion"
            };
            format!(
                "neu aufgenommen ({why}) in {} ms",
                started.elapsed().as_millis()
            )
        } else {
            "unverändert, keine Aufnahme".to_string()
        };

        let mut line = match settle {
            Some(s) => format!(
                "Ruhe nach {} ms, {} Mutationen, {} Anfragen{}; {mode}",
                s.waited_ms,
                s.mutations,
                s.requests,
                if s.hit_max { " (Obergrenze)" } else { "" }
            ),
            None => mode.clone(),
        };
        if self.verify && !(navigated || dirty || force) {
            let full = capture::capture(&self.page, "verify").await?.snapshot;
            let full = perception::from_snapshot(&full, &self.document);
            let same = self.graph.signature() == Graph::build(&full).signature();
            line.push_str(if same {
                "; Prüfung: gleich"
            } else {
                "; Prüfung: ABWEICHEND"
            });
        }
        self.last_stats = Some(line);
        Ok(())
    }

    /// Eine Eingabe verarbeiten. `!` am Anfang bestätigt riskante Aktionen.
    async fn handle(&mut self, input: &str) -> Result<String> {
        let (confirmed, input) = match input.strip_prefix('!') {
            Some(rest) => (true, rest.trim()),
            None => (false, input.trim()),
        };
        // Die Seite kann sich seit der letzten Aufnahme geändert haben.
        self.update(false, None).await?;

        let cmd = match parse(input) {
            Ok(c) => c,
            Err(msg) => return Ok(msg),
        };
        let (target, kind) = match cmd {
            Command::Help => return Ok(command::HELP.into()),
            Command::Describe => return Ok(respond::describe(&self.graph)),
            Command::ListActions => return Ok(respond::list_actions(&self.graph)),
            Command::ListHeadings => return Ok(respond::list_headings(&self.graph)),
            Command::Focus(q) => (self.pick(&q, |_| true), ActionKind::Focus),
            Command::Activate(q) => (self.pick(&q, |_| true), ActionKind::Activate),
            Command::SetValue(q, v) => (self.pick(&q, is_editable), ActionKind::SetValue(v)),
            Command::Select(Some(q), v) => (
                self.pick(&q, |c| !c.options.is_empty()),
                ActionKind::Select(v),
            ),
            Command::Select(None, v) => (self.pick_by_option(&v), ActionKind::Select(v)),
            Command::Increment(q) => (self.pick(&q, is_steppable), ActionKind::Increment),
            Command::Decrement(q) => (self.pick(&q, is_steppable), ActionKind::Decrement),
            Command::FieldStep(step) => {
                let focus = self.focus().await;
                let field = step_field(&self.graph, focus.as_ref(), step)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "Kein {} Formularfeld.",
                            match step {
                                Step::Next => "weiteres",
                                Step::Previous => "vorheriges",
                            }
                        )
                    });
                (field, ActionKind::Focus)
            }
            Command::WhereAmI => {
                let focus = self.focus().await;
                return Ok(respond::where_am_i(&self.graph, focus.as_ref()));
            }
            Command::Scroll(direction) => return self.scroll(direction).await,
            Command::SectionStep(step) => {
                let focus = self.focus().await;
                return match step_heading(&self.graph, focus.as_ref(), step) {
                    Some(h) => self.navigate(Place::Heading(h)).await,
                    None => Ok(format!(
                        "Keine {} Überschrift.",
                        match step {
                            Step::Next => "weitere",
                            Step::Previous => "vorherige",
                        }
                    )),
                };
            }
            Command::GoToPlace(q) => {
                return match self.pick_place(&q) {
                    Ok(place) => self.navigate(place).await,
                    Err(msg) => Ok(msg),
                }
            }
            Command::Read(q) => {
                let place = match q {
                    Some(q) => self.pick_place(&q),
                    None => {
                        current_place(&self.graph, self.focus().await.as_ref()).ok_or_else(|| {
                            "Kein Abschnitt am Fokus. „lies den Abschnitt <Name>“ nennt einen."
                                .into()
                        })
                    }
                };
                return Ok(match place {
                    Ok(place) => respond::read_place(&self.graph, place),
                    Err(msg) => msg,
                });
            }
            Command::Inspect(q) => {
                let found = resolve_inflected(&self.graph, &q, |_| true);
                return Ok(match self.pick_from(&q, found) {
                    Ok(c) => respond::inspect(&c),
                    Err(msg) => msg,
                });
            }
            Command::Dismiss => {
                let focus = self.focus().await;
                match dismissal(&self.graph, &self.model, focus.as_ref()) {
                    Dismissal::Button(c) => (Ok(c.clone()), ActionKind::Activate),
                    Dismissal::NothingOpen => {
                        return Ok("Kein Dialog und kein aufgeklapptes Menü offen.".into())
                    }
                    Dismissal::Escape { target, reaches } => {
                        return self.escape(&target, reaches.as_deref()).await
                    }
                }
            }
        };
        let control = match target {
            Ok(c) => c,
            Err(msg) => return Ok(msg),
        };
        let label = respond::control_line(&control);

        let plan = match plan_on_page(&self.graph.page, &control, kind) {
            Ok(p) => p,
            Err(rejection) => return Ok(format!("Abgelehnt: {rejection} ({label})")),
        };
        if plan.requires_confirmation && !confirmed {
            return Ok(format!(
                "Bestätigung nötig ({:?}): {} — Ziel: {label}. Mit „!“ davor bestätigen.",
                plan.risk,
                plan.notes.join("; ")
            ));
        }
        self.perform(plan, label).await
    }

    /// Validierten Plan ausführen, Ruhe abwarten, neu aufnehmen und die
    /// Wirkung beschreiben.
    async fn perform(&mut self, plan: ActionPlan, label: String) -> Result<String> {
        let before = self.model.clone();
        let before_graph = self.graph.clone();
        if let Err(e) = act::execute(&self.page, &plan).await {
            return Ok(format!("Aktion fehlgeschlagen: {e} ({label})"));
        }
        let settled = self.live.settle().await;
        self.update(true, Some(settled)).await?;
        let target = respond::target_change(&before_graph, &self.graph, &plan.target)
            .map(|t| format!("{t}. "))
            .unwrap_or_default();
        Ok(format!(
            "{:?} auf {label}. {target}{}",
            plan.kind,
            respond::describe_diff(&before, &self.model)
        ))
    }

    /// Escape senden und Wirkung melden (Risiko niedrig: schließt nur).
    /// Escape an das fokussierte Element. `reaches`: Der Fokus liegt nicht
    /// in `target`, Escape erreicht stattdessen dieses Element.
    async fn escape(&mut self, target: &str, reaches: Option<&str>) -> Result<String> {
        let before = self.model.clone();
        act::press_escape(&self.page).await?;
        let settled = self.live.settle().await;
        self.update(true, Some(settled)).await?;
        let focus = match reaches {
            None => String::new(),
            Some(r) => format!(" Der Fokus liegt nicht darin, Escape ging an {r}."),
        };
        Ok(format!(
            "Escape für {target} (kein Schließen-Button gefunden).{focus} {}",
            respond::describe_diff(&before, &self.model)
        ))
    }

    /// Zu einer Überschrift oder einem Bereich (LOW, ohne Rückfrage).
    async fn navigate(&mut self, place: Place) -> Result<String> {
        let (node, dom_node_id) = match place {
            Place::Heading(h) => {
                let h = &self.graph.headings[h];
                (h.node.clone(), h.dom_node_id)
            }
            Place::Region(r) => {
                let r = &self.graph.regions[r];
                (r.node.clone(), r.dom_node_id)
            }
        };
        let label = respond::place_label(&self.graph, place);
        match plan_navigation(&node, dom_node_id) {
            Ok(plan) => self.perform(plan, label).await,
            Err(rejection) => Ok(format!("Abgelehnt: {rejection} ({label})")),
        }
    }

    /// Scrollen hat kein Zielelement und verändert nichts (LOW): wie Escape
    /// ohne `ActionPlan`, Ergebnis aus der Scrollposition.
    async fn scroll(&mut self, direction: ScrollDirection) -> Result<String> {
        let (before, after, max) = act::scroll(&self.page, direction).await?;
        Ok(respond::scrolled(direction, before, after, max))
    }

    /// Aktueller Fokus als Knoten des Modells. Eine Fokusänderung ist keine
    /// DOM-Mutation, die Aufnahme kann ihn also veraltet haben; deshalb das
    /// DOM fragen und über die DOM-ID zuordnen.
    async fn focus(&self) -> Option<NodeRef> {
        let dom = capture::focus(&self.page, &self.snapshot.tree)
            .await
            .ok()?
            .active_backend_node_id?;
        self.model.trees.values().find_map(|t| {
            t.nodes
                .values()
                .find(|n| n.dom_node_id == Some(dom))
                .map(|n| NodeRef::new(t.id.clone(), n.id))
        })
    }

    fn pick_place(&self, query: &str) -> Result<Place, String> {
        match resolve_place(&self.graph, query) {
            PlaceResolution::One(place) => Ok(place),
            PlaceResolution::None => Err(format!(
                "Keine Überschrift und kein Bereich „{query}“ gefunden."
            )),
            PlaceResolution::Many(places) => Err(format!(
                "Mehrdeutig, „{query}“ passt auf:\n{}",
                places
                    .iter()
                    .map(|p| format!("  - {}", respond::place_label(&self.graph, *p)))
                    .collect::<Vec<_>>()
                    .join("\n")
            )),
        }
    }

    fn pick(&self, query: &str, accept: impl Fn(&Control) -> bool) -> Result<Control, String> {
        self.pick_from(query, resolve(&self.graph, query, accept))
    }

    fn pick_from(&self, query: &str, found: Resolution) -> Result<Control, String> {
        match found {
            Resolution::One(c) => Ok(c.clone()),
            Resolution::None => Err(match self.graph.active_modal() {
                Some(m)
                    if self.graph.controls.iter().any(|c| {
                        !self.graph.is_reachable(c.region, &c.node)
                            && c.name
                                .value
                                .as_deref()
                                .is_some_and(|n| n.to_lowercase().contains(&query.to_lowercase()))
                    }) =>
                {
                    format!(
                        "„{query}“ ist gesperrt, solange „{}“ offen ist. Erst den Dialog schließen.",
                        self.graph.regions[m].name.as_deref().unwrap_or("der Dialog")
                    )
                }
                _ => format!("Nichts gefunden für „{query}“."),
            }),
            Resolution::Many(cs) => Err(format!(
                "Mehrdeutig, „{query}“ passt auf:\n{}",
                cs.iter()
                    .map(|c| format!(
                        "  - {} in {}",
                        respond::control_line(c),
                        self.graph.region_label(c.region)
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            )),
        }
    }

    fn pick_by_option(&self, option: &str) -> Result<Control, String> {
        // Wie `resolve`: bei offenem modalem Dialog nur dessen Inhalt.
        let hits: Vec<&Control> = self
            .graph
            .reachable_controls()
            .filter(|c| c.options.iter().any(|o| o.eq_ignore_ascii_case(option)))
            .collect();
        match hits.as_slice() {
            [one] => Ok((*one).clone()),
            [] => Err(format!("Kein Auswahlfeld mit Option „{option}“.")),
            many => Err(format!(
                "Mehrere Auswahlfelder haben „{option}“: {}",
                many.iter()
                    .map(|c| c.display_name())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

fn is_editable(c: &Control) -> bool {
    matches!(
        c.role.as_str(),
        "textbox" | "searchbox" | "combobox" | "spinbutton"
    )
}

fn is_steppable(c: &Control) -> bool {
    matches!(c.role.as_str(), "slider" | "spinbutton")
}

/// Tree-ID des `n`-ten Dokuments einer Session.
fn document_id(n: u64) -> TreeId {
    TreeId(format!("dokument-{n}"))
}

/// Weniger Knoten heißt: Dokument noch nicht geladen (nur Wurzel und Body).
const MIN_NODES: usize = 3;

/// Während einer Navigation kann `getFullAXTree` scheitern — kurz nachfassen.
async fn capture_retry(page: &Page, label: &str) -> Result<AXSnapshot> {
    let mut last = None;
    for _ in 0..4 {
        match capture::capture(page, label).await {
            Ok(c) if !c.snapshot.tree.is_empty() => return Ok(c.snapshot),
            Ok(_) => {}
            Err(e) => last = Some(e),
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("leerer AXTree")))
}

async fn repl(browser: &Browser, url: &str) -> Result<()> {
    let mut session = Session::open(browser, &to_url(url, Path::new("."))).await?;
    println!("{}\n{}", respond::describe(&session.graph), command::HELP);
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(url) = line.strip_prefix("url ") {
            session = Session::open(browser, &to_url(url.trim(), Path::new("."))).await?;
            println!("{}", respond::describe(&session.graph));
            continue;
        }
        println!("{}", session.handle(&line).await?);
    }
    Ok(())
}

async fn run_files(browser: &Browser, files: &[&String]) -> Result<()> {
    let (mut passed, mut failed) = (0, 0);
    for file in files {
        let path = PathBuf::from(file);
        let base = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let text = std::fs::read_to_string(&path)?;
        println!("\n=== {} ===", path.display());
        let mut session: Option<Session> = None;
        let mut last = String::new();
        for line in text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
        {
            if let Some(url) = line.strip_prefix("url:") {
                let url = to_url(url.trim(), &base);
                println!("\n## {url}");
                match Session::open(browser, &url).await {
                    Ok(s) => {
                        println!("{}", respond::describe(&s.graph));
                        session = Some(s);
                    }
                    Err(e) => {
                        println!("!! Seite nicht ladbar: {e}");
                        session = None;
                    }
                }
            } else if let Some(input) = line.strip_prefix("do:") {
                let Some(s) = session.as_mut() else { continue };
                let started = Instant::now();
                last = s
                    .handle(input.trim())
                    .await
                    .unwrap_or_else(|e| format!("Fehler: {e}"));
                println!(
                    "\n> {}\n{}\n({} ms{})",
                    input.trim(),
                    indent(&last),
                    started.elapsed().as_millis(),
                    s.last_stats
                        .as_deref()
                        .map(|l| format!("; {l}"))
                        .unwrap_or_default()
                );
            } else if let Some(text) = line.strip_prefix("assert:") {
                let Some(s) = session.as_mut() else { continue };
                last = assertions::run(s, text.trim())
                    .await
                    .unwrap_or_else(|e| format!("Fehler: {e}"));
                println!("\n? {}\n{}", text.trim(), indent(&last));
            } else if let Some(expected) = line.strip_prefix("expect:") {
                if session.is_none() {
                    continue;
                }
                let expected = expected.trim();
                if last.to_lowercase().contains(&expected.to_lowercase()) {
                    passed += 1;
                    println!("  ✓ erwartet „{expected}“");
                } else {
                    failed += 1;
                    println!("  ✗ erwartet „{expected}“ — FEHLT");
                }
            }
        }
    }
    println!("\nErwartungen: {passed} erfüllt, {failed} nicht erfüllt");
    Ok(())
}

async fn measure(browser: &Browser, urls: &[&String], repeat: usize) -> Result<()> {
    println!("url | Knoten | Bedienelemente | iframes (eingehängt/nicht erreichbar) | Aufnahme ms (min/max) | Modell+Graph µs (max) | Ruhe ms | stabil");
    for url in urls {
        let url = to_url(url, Path::new("."));
        let session = match Session::open(browser, &url).await {
            Ok(s) => s,
            Err(e) => {
                println!("{url} | nicht ladbar: {e}");
                continue;
            }
        };
        let mut tree_ms = Vec::new();
        let mut graph_us = Vec::new();
        let mut signatures = Vec::new();
        let mut frames = (0, 0);
        for i in 0..repeat {
            let c = capture::capture(&session.page, &format!("m{i}")).await?;
            tree_ms.push(c.tree_ms);
            frames = c.frames;
            let started = Instant::now();
            let model = perception::from_snapshot(&c.snapshot, &session.document);
            let graph = Graph::build(&model);
            graph_us.push(started.elapsed().as_micros());
            signatures.push((c.snapshot.tree.len(), graph.signature()));
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        let stable = signatures.windows(2).all(|w| w[0].1 == w[1].1);
        let unstable_detail = if stable {
            String::new()
        } else {
            let a = &signatures[0].1;
            let b = &signatures[signatures.len() - 1].1;
            format!(" ({} vs. {} Einträge)", a.len(), b.len())
        };
        println!(
            "{url} | {} | {} | {}/{} | {}/{} | {} | {}{} | {}{}",
            signatures[0].0,
            signatures[0].1.len(),
            frames.0,
            frames.1,
            tree_ms.iter().min().unwrap(),
            tree_ms.iter().max().unwrap(),
            graph_us.iter().max().unwrap(),
            session.opened.waited_ms,
            if session.opened.hit_max {
                " (Obergrenze)"
            } else {
                ""
            },
            if stable { "ja" } else { "nein" },
            unstable_detail
        );
        session.page.close().await.ok();
    }
    Ok(())
}

fn to_url(s: &str, base: &Path) -> String {
    if s.contains("://") || s.starts_with("about:") {
        return s.to_string();
    }
    let path = base.join(s);
    let abs = std::fs::canonicalize(&path).unwrap_or(path);
    format!("file://{}", abs.display())
}

fn indent(s: &str) -> String {
    s.lines()
        .map(|l| format!("  {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}
