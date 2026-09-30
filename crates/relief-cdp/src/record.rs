//! AX-Aufnahmen als Fixtures (Backlog 11).
//!
//! Spielt Aufgabendateien ab wie `run` und speichert je Seite die
//! Anfangsaufnahme sowie je `do:` die Aufnahme vorher und nachher, dazu die
//! Antwort. So können browserfreie Tests echte Seiten nachspielen.
//!
//! Ablage: `<out>/<aufgabendatei>/<NN>-<seite>/`, darin `snapshot-NN.json`
//! (`a11y_perception::AXSnapshot`) und `index.json`. Eine Aufnahme, die sich
//! gegenüber der vorigen nicht geändert hat, wird nicht erneut gespeichert —
//! der Schritt verweist auf dieselbe Datei.
//!
//! Lokale Pfade (`file://<Arbeitsverzeichnis>/…`) werden durch
//! `file:///REPO/…` ersetzt, damit keine Rechnerpfade ins Repo gelangen.

use std::path::{Path, PathBuf};

use a11y_perception::AXSnapshot;
use anyhow::Result;
use chromiumoxide::browser::Browser;
use serde_json::{json, Value};

use crate::{to_url, Session};

pub async fn record_files(browser: &Browser, files: &[&String], out: &Path) -> Result<()> {
    for file in files {
        let path = PathBuf::from(file);
        let base = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("aufgaben")
            .to_string();
        let text = std::fs::read_to_string(&path)?;
        println!("\n=== {} ===", path.display());

        let mut page: Option<PageRecorder> = None;
        let mut count = 0;
        for line in text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
        {
            if let Some(url) = line.strip_prefix("url:") {
                if let Some(p) = page.take() {
                    p.finish()?;
                }
                let url = to_url(url.trim(), &base);
                count += 1;
                let dir = out.join(&stem).join(format!("{count:02}-{}", slug(&url)));
                match Session::open(browser, &url).await {
                    Ok(session) => {
                        println!("## {url} → {}", dir.display());
                        page = Some(PageRecorder::new(session, dir, url)?);
                    }
                    Err(e) => println!("!! Seite nicht ladbar, übersprungen: {url}: {e}"),
                }
            } else if let Some(input) = line.strip_prefix("do:") {
                if let Some(p) = page.as_mut() {
                    p.step(input.trim()).await?;
                }
            }
        }
        if let Some(p) = page.take() {
            p.finish()?;
        }
    }
    Ok(())
}

struct PageRecorder {
    session: Session,
    dir: PathBuf,
    url: String,
    steps: Vec<Value>,
    initial: String,
    last_json: String,
    last_file: String,
    files: usize,
}

impl PageRecorder {
    fn new(session: Session, dir: PathBuf, url: String) -> Result<Self> {
        std::fs::create_dir_all(&dir)?;
        let mut rec = PageRecorder {
            session,
            dir,
            url,
            steps: Vec::new(),
            initial: String::new(),
            last_json: String::new(),
            last_file: String::new(),
            files: 0,
        };
        let snapshot = rec.session.snapshot.clone();
        rec.initial = rec.save(&snapshot)?;
        Ok(rec)
    }

    async fn step(&mut self, input: &str) -> Result<()> {
        // Stand unmittelbar vor dem Befehl, wie `handle` ihn sieht.
        self.session.update(false, None).await?;
        let before = self.session.snapshot.clone();
        let before_file = self.save(&before)?;
        let response = self
            .session
            .handle(input)
            .await
            .unwrap_or_else(|e| format!("Fehler: {e}"));
        let after = self.session.snapshot.clone();
        let after_file = self.save(&after)?;
        println!("  > {input} ({before_file} → {after_file})");
        self.steps.push(json!({
            "eingabe": input,
            "vorher": before_file,
            "nachher": after_file,
            "antwort": response,
        }));
        Ok(())
    }

    /// Speichert die Aufnahme, wenn sie sich von der zuletzt gespeicherten
    /// unterscheidet; der Zeitstempel zählt dabei nicht.
    fn save(&mut self, snapshot: &AXSnapshot) -> Result<String> {
        let mut comparable = snapshot.clone();
        comparable.timestamp_ms = 0;
        let json = neutral_paths(&serde_json::to_string(&comparable)?);
        if json == self.last_json {
            return Ok(self.last_file.clone());
        }
        let name = format!("snapshot-{:02}.json", self.files);
        self.files += 1;
        std::fs::write(self.dir.join(&name), &json)?;
        self.last_json = json;
        self.last_file = name.clone();
        Ok(name)
    }

    fn finish(self) -> Result<()> {
        let url = neutral_paths(&self.url);
        let index = json!({
            "url": url,
            "titel": self.session.snapshot.document_title,
            "anfang": self.initial,
            "schritte": self.steps,
        });
        std::fs::write(
            self.dir.join("index.json"),
            neutral_paths(&serde_json::to_string_pretty(&index)?),
        )?;
        Ok(())
    }
}

fn neutral_paths(text: &str) -> String {
    match std::env::current_dir().and_then(|d| d.canonicalize()) {
        Ok(dir) => text.replace(&format!("file://{}/", dir.display()), "file:///REPO/"),
        Err(_) => text.to_string(),
    }
}

/// `https://www.gov.uk/` → `www-gov-uk`, `file:///…/shop-clean.html` → `shop-clean`.
fn slug(url: &str) -> String {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let raw = if url.starts_with("file://") {
        rest.rsplit('/')
            .next()
            .unwrap_or(rest)
            .trim_end_matches(".html")
            .to_string()
    } else {
        rest.trim_end_matches('/').to_string()
    };
    let s: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    s.split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
