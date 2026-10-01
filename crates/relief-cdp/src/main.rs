//! Relief CDP-Spike (Phase 0a): AXTree → Modell → Interaction Graph → Aktion.
//!
//! ```text
//! relief-cdp run <aufgaben.txt>... [--headful]   Aufgabendateien abarbeiten
//! relief-cdp repl <url> [--headful]              interaktiv
//! relief-cdp measure <url>... [--repeat N]       Zeiten und Graph-Stabilität
//! relief-cdp record <aufgaben.txt>... [--out dir]  Aufnahmen als Fixtures speichern
//! relief-cdp palette <url>                       Befehlsleiste im Browser (immer sichtbar)
//! relief-cdp test <aufgaben.txt>... [--junit out.xml] [--report befunde.json] [--fork]
//!                                                ohne Fenster, Bericht für CI; `--fork`: eigener Build (--relief-run)
//! ```
//!
//! Aufgabendatei: `url: …` (Pfad relativ zur Aufgabendatei als `file://`, mit
//! `server:` davor über einen lokalen HTTP-Server → `server.rs`), `do: …`
//! (mit `!` davor bestätigt), `expect: …`
//! (Teilstring der letzten Antwort), `assert: …` (Formular-Zusicherung, Befunde
//! als Antwort → `assertions.rs`), `wait: <ms>` (Seite läuft ohne Eingabe
//! weiter), `#` Kommentar.
//!
//! Security-Log: Mit `RELIEF_LOG=<datei>` hängt jede Eingabe die
//! Entscheidungen der Sitzung als JSON-Zeilen an (`{"t":…,"security":{…}}`,
//! ohne Werte und Namen); `palette` schreibt sie in ihr Protokoll.

mod act;
mod assertions;
mod capture;
mod facts;
mod frames;
mod live;
mod palette;
mod record;
mod report;
mod server;

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use a11y_perception::AXSnapshot;
use anyhow::{bail, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Page;
use futures::StreamExt;
use relief_interaction::{
    command, expectation_met, parse_input, parse_tasks, respond, uses_focus, ActionPlan, Graph,
    Outcome, Pending, Session as Dialog, TaskLine,
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
                && (*i == 0
                    || !matches!(
                        args[i - 1].as_str(),
                        "--repeat" | "--out" | "--junit" | "--report"
                    ))
        })
        .map(|(_, a)| a)
        .collect();
    let Some((mode, rest)) = positional.split_first() else {
        bail!("Aufruf: relief-cdp run|repl|measure …");
    };

    let option = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
    };
    if mode.as_str() == "test" && args.iter().any(|a| a == "--fork") {
        return test_fork(rest, option("--junit"));
    }

    let headful = headful || mode.as_str() == "palette";
    let (mut browser, handler) = launch(headful).await?;
    let result = match mode.as_str() {
        "run" => run_files(&browser, rest).await.map(|_| ()),
        "test" => match run_files(&browser, rest).await {
            Ok((suites, findings)) => {
                write_reports(&suites, &findings, option("--junit"), option("--report"))
            }
            Err(e) => Err(e),
        },
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
    /// Modell vor der letzten Eingabe (`do:`), für `assert: statusmeldung`.
    before_action: Option<SemanticGraph>,
    /// Jeden übersprungenen Neuaufbau gegen einen Vollsnapshot prüfen
    /// (`RELIEF_VERIFY=1`).
    verify: bool,
    /// Messzeile zum letzten Befehl.
    last_stats: Option<String>,
    /// Ruhe nach dem Laden (für `measure`).
    opened: live::Settle,
    /// Befehlszustand (Position) zwischen den Eingaben.
    session: Dialog,
    /// Frames in anderen Renderer-Prozessen (Site Isolation).
    frames: frames::Frames,
    /// Ziel des Security-Logs (`RELIEF_LOG`, in `palette` deren Protokoll).
    security_log: Option<File>,
}

impl Session {
    async fn open(browser: &Browser, url: &str) -> Result<Self> {
        Self::open_page(browser, browser.new_page("about:blank").await?, url).await
    }

    /// Auf einer vorbereiteten Seite (z. B. mit eingefügter Befehlsleiste)
    /// laden. `new_page(url)` + `wait_for_navigation` kehrt teils vor dem
    /// Commit des eigentlichen Dokuments zurück; `goto` wartet auf das Laden.
    async fn open_page(browser: &Browser, page: Page, url: &str) -> Result<Self> {
        let (frames, frame_events) =
            frames::Frames::connect(browser.websocket_address(), page.target_id()).await?;
        // Vor dem Laden starten, damit die Anfragen des Ladens als ausstehend
        // zählen („Seite ruht“ auch über das Netz).
        let mut live = live::Live::start(&page, frame_events).await?;
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
            let snapshot = capture_retry(&page, &frames, "initial").await?;
            attempt += 1;
            if snapshot.tree.len() > MIN_NODES || attempt == 6 {
                break (snapshot, opened);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        };
        // Das geladene Dokument ist Dokument 1, kein ersetztes.
        live.take_new_document();
        let document = document_id(1);
        let mut model = perception::from_snapshot(&snapshot, &document);
        facts::annotate(&page, &frames, &mut model).await?;
        let graph = Graph::build(&model);
        let security_log = match std::env::var_os("RELIEF_LOG") {
            Some(path) => Some(File::options().create(true).append(true).open(path)?),
            None => None,
        };
        Ok(Session {
            page,
            live,
            snapshot,
            document,
            documents: 1,
            model,
            graph,
            before_action: None,
            verify: std::env::var_os("RELIEF_VERIFY").is_some(),
            last_stats: None,
            opened,
            session: Dialog::new(),
            frames,
            security_log,
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
            self.snapshot = capture_retry(&self.page, &self.frames, "current").await?;
            self.model = perception::from_snapshot(&self.snapshot, &self.document);
            facts::annotate(&self.page, &self.frames, &mut self.model).await?;
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
            let full = capture::capture(&self.page, &self.frames, "verify")
                .await?
                .snapshot;
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

    /// Eine Eingabe verarbeiten und die Entscheidungen dazu ins
    /// Security-Log schreiben.
    async fn handle(&mut self, input: &str) -> Result<String> {
        // Nach Fähigkeitsprofil (Paket 41): ggf. gekürzt, Rest für „mehr“.
        let answer = match self.answer(input).await {
            Ok(text) => Ok(self.session.present(text)),
            Err(e) => Err(e),
        };
        let events = self.session.take_security_log();
        if let Some(log) = self.security_log.as_mut() {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis() as u64;
            for event in events {
                writeln!(log, "{}", serde_json::json!({ "t": t, "security": event }))?;
            }
        }
        answer
    }

    /// Eine Eingabe beantworten. `!` am Anfang bestätigt riskante Aktionen.
    async fn answer(&mut self, input: &str) -> Result<String> {
        // Die Seite kann sich seit der letzten Aufnahme geändert haben.
        self.update(false, None).await?;
        self.before_action = Some(self.model.clone());
        // Antwort auf eine offene Rückfrage (Zahl, „ja“, „abbrechen“)?
        let input = match self.session.pending_reply(&self.graph, &self.model, input) {
            Pending::Done(outcome) => return self.run(outcome).await,
            Pending::Command => input.to_string(),
        };
        let (confirmed, cmd) = match parse_input(&input) {
            Ok(c) => c,
            Err(msg) => return Ok(msg),
        };
        let focus = if uses_focus(&cmd) {
            self.focus().await
        } else {
            None
        };
        let outcome = self
            .session
            .handle(&self.graph, &self.model, confirmed, cmd, focus.as_ref());
        self.run(outcome).await
    }

    /// Ergebnis der Sitzung ausführen bzw. beantworten.
    async fn run(&mut self, outcome: Outcome) -> Result<String> {
        match outcome {
            Outcome::Answer(text) => Ok(text),
            Outcome::Perform { plan, label } => self.perform(plan, label).await,
            Outcome::Escape { target, reaches } => self.escape(&target, reaches.as_deref()).await,
            Outcome::Scroll(direction) => {
                let (before, after, max) = act::scroll(&self.page, direction).await?;
                Ok(respond::scrolled(direction, before, after, max))
            }
        }
    }

    /// Validierten Plan ausführen, Ruhe abwarten, neu aufnehmen und die
    /// Wirkung beschreiben.
    async fn perform(&mut self, plan: ActionPlan, label: String) -> Result<String> {
        let before = self.model.clone();
        let before_graph = self.graph.clone();
        if let Err(e) = act::execute(&self.page, &self.frames, &plan).await {
            return Ok(format!("Aktion fehlgeschlagen: {e} ({label})"));
        }
        let settled = self.live.settle().await;
        self.update(true, Some(settled)).await?;
        Ok(self.session.performed(
            &plan,
            &label,
            (&before, &before_graph),
            (&self.model, &self.graph),
        ))
    }

    /// Escape an das fokussierte Element senden und die Wirkung melden
    /// (Risiko niedrig: schließt nur).
    async fn escape(&mut self, target: &str, reaches: Option<&str>) -> Result<String> {
        let before = self.model.clone();
        act::press_escape(&self.page).await?;
        let settled = self.live.settle().await;
        self.update(true, Some(settled)).await?;
        Ok(self.session.escaped(target, reaches, &before, &self.model))
    }

    /// Aktueller Fokus als Knoten des Modells. Eine Fokusänderung ist keine
    /// DOM-Mutation, die Aufnahme kann ihn also veraltet haben; deshalb das
    /// DOM fragen und über die DOM-ID zuordnen.
    async fn focus(&self) -> Option<NodeRef> {
        let dom = capture::focus(&self.page, &self.frames, &self.snapshot.tree)
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
}

/// Tree-ID des `n`-ten Dokuments einer Session.
fn document_id(n: u64) -> TreeId {
    TreeId(format!("dokument-{n}"))
}

/// Weniger Knoten heißt: Dokument noch nicht geladen (nur Wurzel und Body).
const MIN_NODES: usize = 3;

/// Während einer Navigation kann `getFullAXTree` scheitern — kurz nachfassen.
async fn capture_retry(page: &Page, frames: &frames::Frames, label: &str) -> Result<AXSnapshot> {
    let mut last = None;
    for _ in 0..4 {
        match capture::capture(page, frames, label).await {
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

/// Aufgabendateien abarbeiten; Ausgabe auf stdout, dazu die Ergebnisse je
/// Datei und die Befunde der Zusicherungen für den Bericht.
async fn run_files(
    browser: &Browser,
    files: &[&String],
) -> Result<(Vec<report::Suite>, report::Findings)> {
    let (mut passed, mut failed) = (0, 0);
    let mut servers = server::Servers::default();
    let mut suites = Vec::new();
    let mut findings = report::Findings::default();
    for file in files {
        let path = PathBuf::from(file);
        let base = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let text = std::fs::read_to_string(&path)?;
        println!("\n=== {} ===", path.display());
        let started_file = Instant::now();
        let mut suite = report::Suite {
            name: path.display().to_string(),
            ..report::Suite::default()
        };
        let mut session: Option<Session> = None;
        let mut last = String::new();
        // Fähigkeitsprofil der Datei (`profil:`), gilt auch nach `url:`.
        let mut profile = relief_interaction::profile::Store::default();
        // Zustand, auf den sich Antwort und Befunde beziehen.
        let mut state = String::from("Laden");
        let mut page_url = String::new();
        for line in parse_tasks(&text).into_iter().filter_map(spoken_as_typed) {
            match line {
                TaskLine::Url(url) => {
                    let url = match url.strip_prefix(server::PREFIX) {
                        Some(path) => match servers.url(&base.join(path.trim())).await {
                            Ok(url) => url,
                            Err(e) => {
                                println!("\n## {url}\n!! Server nicht startbar: {e}");
                                suite
                                    .errors
                                    .push(format!("Server nicht startbar: {url}: {e}"));
                                session = None;
                                continue;
                            }
                        },
                        None => to_url(&url, &base),
                    };
                    println!("\n## {url}");
                    state = "Laden".into();
                    page_url = url.clone();
                    match Session::open(browser, &url).await {
                        Ok(mut s) => {
                            println!("{}", respond::describe(&s.graph));
                            s.session.set_effects(profile.effective("").effects());
                            session = Some(s);
                        }
                        Err(e) => {
                            println!("!! Seite nicht ladbar: {e}");
                            suite.errors.push(format!("Seite nicht ladbar: {url}: {e}"));
                            session = None;
                        }
                    }
                }
                TaskLine::Do(input) => {
                    let Some(s) = session.as_mut() else { continue };
                    let started = Instant::now();
                    state = input.clone();
                    last = s
                        .handle(&input)
                        .await
                        .unwrap_or_else(|e| format!("Fehler: {e}"));
                    println!(
                        "\n> {input}\n{}\n({} ms{})",
                        indent(&last),
                        started.elapsed().as_millis(),
                        s.last_stats
                            .as_deref()
                            .map(|l| format!("; {l}"))
                            .unwrap_or_default()
                    );
                }
                TaskLine::Profile(line) => {
                    last = match relief_interaction::profile::apply_line(&mut profile, &line) {
                        Ok(text) => text,
                        Err(e) => format!("Fehler: {e}"),
                    };
                    if let Some(s) = session.as_mut() {
                        s.session.set_effects(profile.effective("").effects());
                    }
                    println!("\n§ {line}\n{}", indent(&last));
                }
                TaskLine::Speak(_) | TaskLine::Audio(_) => {
                    unreachable!("spoken_as_typed")
                }
                TaskLine::Assert(text) => {
                    let Some(s) = session.as_mut() else { continue };
                    last = match assertions::run(s, &text).await {
                        Ok((answer, found)) => {
                            findings.add(&page_url, &state, found);
                            answer
                        }
                        Err(e) => format!("Fehler: {e}"),
                    };
                    println!("\n? {text}\n{}", indent(&last));
                }
                TaskLine::Wait(ms) => {
                    if session.is_none() {
                        continue;
                    }
                    println!("\n… {ms} ms warten");
                    tokio::time::sleep(Duration::from_millis(ms)).await;
                }
                TaskLine::Expect(expected) => {
                    if session.is_none() {
                        continue;
                    }
                    let ok = expectation_met(&last, &expected);
                    if ok {
                        passed += 1;
                        println!("  ✓ erwartet „{expected}“");
                    } else {
                        failed += 1;
                        println!("  ✗ erwartet „{expected}“ — FEHLT");
                    }
                    suite.cases.push(report::Case {
                        name: expected,
                        after: state.clone(),
                        passed: ok,
                        answer: last.clone(),
                    });
                }
            }
        }
        suite.seconds = started_file.elapsed().as_secs_f64();
        suites.push(suite);
    }
    println!("\nErwartungen: {passed} erfüllt, {failed} nicht erfüllt");
    Ok((suites, findings))
}

/// JUnit-XML und a11y-report schreiben; Fehler, wenn eine Erwartung nicht
/// erfüllt ist (Rückgabewert für CI).
fn write_reports(
    suites: &[report::Suite],
    findings: &report::Findings,
    junit: Option<PathBuf>,
    befunde: Option<PathBuf>,
) -> Result<()> {
    if let Some(path) = junit {
        std::fs::write(&path, report::junit(suites))?;
        println!("JUnit: {}", path.display());
    }
    if let Some(path) = befunde {
        std::fs::write(&path, serde_json::to_string_pretty(&findings.report())?)?;
        println!(
            "Befunde: {} ({} nach Zusammenfassen)",
            path.display(),
            findings.len()
        );
    }
    let failed = suites
        .iter()
        .flat_map(|s| &s.cases)
        .filter(|c| !c.passed)
        .count();
    let errors: usize = suites.iter().map(|s| s.errors.len()).sum();
    if failed + errors > 0 {
        bail!("{failed} Erwartungen nicht erfüllt, {errors} Läufe unvollständig");
    }
    Ok(())
}

/// Aufgaben im eigenen Build ohne Fenster (`--headless=new`, `--relief-run`,
/// Aktionen über AXActionData). `CHROME` setzt den Build, Standard
/// `~/chromium/src/out/Relief`. Zusicherungen gibt es dort nicht (Feature
/// `assertions` ist im Fork aus), der Bericht enthält nur Erwartungen.
fn test_fork(files: &[&String], junit: Option<PathBuf>) -> Result<()> {
    let chrome = std::env::var_os("CHROME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join("chromium/src/out/Relief/Relief.app/Contents/MacOS/Relief")
        });
    let profile = std::env::temp_dir().join(format!("relief-test-{}", std::process::id()));
    let files: Vec<String> = files
        .iter()
        .map(|f| {
            std::fs::canonicalize(f)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| f.to_string())
        })
        .collect();
    let output = std::process::Command::new(&chrome)
        .args([
            "--headless=new",
            "--enable-relief",
            "--no-first-run",
            "--use-mock-keychain",
        ])
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg(format!("--relief-run={}", files.join(",")))
        .stderr(std::process::Stdio::null())
        .output()?;
    std::fs::remove_dir_all(&profile).ok();
    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");
    let suites = report::parse_fork_output(&stdout);
    if suites.is_empty() {
        bail!("keine Ausgabe vom Fork ({})", chrome.display());
    }
    write_reports(&suites, &report::Findings::default(), junit, None)
}

async fn measure(browser: &Browser, urls: &[&String], repeat: usize) -> Result<()> {
    println!("url | Knoten | Bedienelemente | iframes im Prozess (eingehängt/nicht erreichbar) | iframes eigene Sitzung (eingehängt/nicht erreichbar) | Aufnahme ms (min/max) | Modell+Graph µs (max) | Ruhe ms | stabil");
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
        let mut frames = capture::FrameCount::default();
        for i in 0..repeat {
            let c = capture::capture(&session.page, &session.frames, &format!("m{i}")).await?;
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
            "{url} | {} | {} | {}/{} | {}/{} | {}/{} | {} | {}{} | {}{}",
            signatures[0].0,
            signatures[0].1.len(),
            frames.local.0,
            frames.local.1,
            frames.remote.0,
            frames.remote.1,
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
        for why in &frames.unreachable {
            println!("  nicht erreichbar: {why}");
        }
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

/// `sprich:` ohne Spracherkennung (Paket 26): der Text wie getippt, nach
/// denselben Regeln wie im Fork (Abbruchwörter → „abbrechen“). `audio:`
/// braucht die Spracherkennung des Forks und entfällt hier.
fn spoken_as_typed(line: TaskLine) -> Option<TaskLine> {
    use relief_interaction::speech::{route, Heard};
    match line {
        TaskLine::Speak(text) => match route(&text) {
            Heard::Cancel => Some(TaskLine::Do("abbrechen".into())),
            Heard::Command(c) => Some(TaskLine::Do(c)),
            Heard::Nothing => None,
        },
        TaskLine::Audio(path) => {
            println!("\n… {path}: Audiodatei nur im Fork (Spracherkennung)");
            None
        }
        other => Some(other),
    }
}
