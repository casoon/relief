//! Bericht eines Aufgabenlaufs für CI (Paket 44): JUnit-XML je
//! Aufgabendatei und Befunde der Zusicherungen als `a11y-report`, gleiche
//! Befunde über die Zustände einer Aufgabe zusammengefasst.
//!
//! Ein Testfall ist eine `expect:`-Zeile. Befunde tragen einen stabilen
//! Schlüssel (Regel, Seite, Knoten, Rolle, Name, Ergebnis); derselbe Befund
//! nach dem Öffnen eines Dialogs und nach dem nächsten Schritt erscheint
//! einmal, mit den betroffenen Zuständen als Evidence (`source: "state"`).

use a11y_report::{Evidence, Finding, Report};

/// Eine erfüllte oder nicht erfüllte Erwartung.
#[derive(Debug, Clone)]
pub struct Case {
    /// Erwartung, wie in der Aufgabendatei.
    pub name: String,
    /// Eingabe, auf deren Antwort sich die Erwartung bezieht.
    pub after: String,
    pub passed: bool,
    /// Letzte Antwort (bei Fehlschlag die Begründung).
    pub answer: String,
}

/// Eine Aufgabendatei.
#[derive(Debug, Clone, Default)]
pub struct Suite {
    pub name: String,
    pub cases: Vec<Case>,
    /// Laufzeit in Sekunden.
    pub seconds: f64,
    /// Seite nicht ladbar o. Ä.: der Lauf der Datei ist unvollständig.
    pub errors: Vec<String>,
}

/// Befunde aller Zustände, zusammengefasst.
#[derive(Debug, Default)]
pub struct Findings {
    entries: Vec<(String, Finding)>,
}

impl Findings {
    /// Befunde eines Zustands (`state`: z. B. die letzte `do:`-Eingabe)
    /// auf der Seite `url` aufnehmen.
    pub fn add(&mut self, url: &str, state: &str, findings: Vec<Finding>) {
        for mut finding in findings {
            if finding.location.url.is_none() {
                finding.location.url = Some(url.to_string());
            }
            let key = key(&finding);
            let evidence = Evidence {
                source: "state".into(),
                field: Some("nach".into()),
                value: Some(state.to_string()),
            };
            match self.entries.iter_mut().find(|(k, _)| *k == key) {
                Some((_, existing)) => {
                    if !existing.evidence.contains(&evidence) {
                        existing.evidence.push(evidence);
                    }
                }
                None => {
                    finding.evidence.push(evidence);
                    self.entries.push((key, finding));
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn report(&self) -> Report {
        let mut report = Report::new();
        report.extend(self.entries.iter().map(|(_, f)| f.clone()));
        report.finish()
    }
}

/// Stabiler Schlüssel eines Befunds: was, wo, an welchem Element, mit
/// welchem Ergebnis. Die Meldung zählt nicht mit, sie kann Zustandsdetails
/// tragen.
fn key(f: &Finding) -> String {
    [
        f.rule_id.as_str(),
        f.outcome.as_str(),
        f.location.url.as_deref().unwrap_or(""),
        f.location.node.as_deref().unwrap_or(""),
        f.location.selector.as_deref().unwrap_or(""),
        f.role.as_deref().unwrap_or(""),
        f.name.as_deref().unwrap_or(""),
    ]
    .join("\u{1f}")
}

/// JUnit-XML (Jenkins-/GitLab-/GitHub-Format: `testsuites` → `testsuite` →
/// `testcase` mit `failure`).
pub fn junit(suites: &[Suite]) -> String {
    let tests: usize = suites.iter().map(|s| s.cases.len()).sum();
    let failures: usize = suites
        .iter()
        .map(|s| s.cases.iter().filter(|c| !c.passed).count())
        .sum();
    let errors: usize = suites.iter().map(|s| s.errors.len()).sum();
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites name=\"relief\" tests=\"{tests}\" failures=\"{failures}\" errors=\"{errors}\">\n"
    );
    for suite in suites {
        out.push_str(&format!(
            "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"{}\" time=\"{:.3}\">\n",
            escape(&suite.name),
            suite.cases.len(),
            suite.cases.iter().filter(|c| !c.passed).count(),
            suite.errors.len(),
            suite.seconds
        ));
        for error in &suite.errors {
            out.push_str(&format!(
                "    <testcase classname=\"{0}\" name=\"Lauf\">\n      <error message=\"{1}\"/>\n    </testcase>\n",
                escape(&suite.name),
                escape(error)
            ));
        }
        for case in &suite.cases {
            out.push_str(&format!(
                "    <testcase classname=\"{}\" name=\"{}\">",
                escape(&suite.name),
                escape(&format!("{} → {}", case.after, case.name))
            ));
            if case.passed {
                out.push_str("</testcase>\n");
            } else {
                out.push_str(&format!(
                    "\n      <failure message=\"erwartet: {}\">{}</failure>\n    </testcase>\n",
                    escape(&case.name),
                    escape(&case.answer)
                ));
            }
        }
        out.push_str("  </testsuite>\n");
    }
    out.push_str("</testsuites>\n");
    out
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // In XML 1.0 unzulässige Steuerzeichen fallen weg.
            c if (c as u32) < 0x20 && !matches!(c, '\n' | '\r' | '\t') => {}
            c => out.push(c),
        }
    }
    out
}

/// Ausgabe des Fork-Runners (`--relief-run`) in Suites zerlegen. Format:
/// `=== <datei> ===`, `> <eingabe>`, `  ✓ erwartet „…“`,
/// `  ✗ erwartet „…“ — FEHLT`, `!! Seite nicht ladbar: …`; Antwortzeilen
/// sind eingerückt.
pub fn parse_fork_output(output: &str) -> Vec<Suite> {
    let mut suites: Vec<Suite> = Vec::new();
    let mut after = String::from("Laden");
    let mut answer = String::new();
    for line in output.lines() {
        if let Some(name) = line
            .strip_prefix("=== ")
            .and_then(|l| l.strip_suffix(" ==="))
        {
            suites.push(Suite {
                name: name.to_string(),
                ..Suite::default()
            });
            after = "Laden".into();
            answer.clear();
            continue;
        }
        let Some(suite) = suites.last_mut() else {
            continue;
        };
        if let Some(input) = line.strip_prefix("> ") {
            after = input.to_string();
            answer.clear();
        } else if let Some(error) = line.strip_prefix("!! ") {
            suite.errors.push(error.to_string());
        } else if let Some(rest) = line.strip_prefix("  ✓ erwartet „") {
            suite.cases.push(Case {
                name: rest.trim_end_matches('“').to_string(),
                after: after.clone(),
                passed: true,
                answer: answer.clone(),
            });
        } else if let Some(rest) = line.strip_prefix("  ✗ erwartet „") {
            suite.cases.push(Case {
                name: rest.trim_end_matches("“ — FEHLT").to_string(),
                after: after.clone(),
                passed: false,
                answer: answer.clone(),
            });
        } else if let Some(text) = line.strip_prefix("  ") {
            if !answer.is_empty() {
                answer.push('\n');
            }
            answer.push_str(text);
        }
    }
    suites
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gleiche_befunde_ueber_zustaende_einmal() {
        let mut findings = Findings::default();
        let befund = || {
            Finding::fail("form/field-name", "Feld ohne Namen")
                .with_element(Some("textbox".into()), None)
        };
        findings.add("file:///a.html", "Laden", vec![befund()]);
        findings.add("file:///a.html", "öffne Dialog", vec![befund()]);
        findings.add("file:///a.html", "öffne Dialog", vec![befund()]);
        findings.add("file:///b.html", "Laden", vec![befund()]);
        assert_eq!(findings.len(), 2);
        let report = findings.report();
        let zustaende: Vec<_> = report.findings[0]
            .evidence
            .iter()
            .filter_map(|e| e.value.clone())
            .collect();
        assert_eq!(zustaende, vec!["Laden", "öffne Dialog"]);
        assert_eq!(report.summary.fail, 2);
    }

    #[test]
    fn junit_mit_fehlschlag_und_escape() {
        let suites = vec![Suite {
            name: "01 & <x>".into(),
            cases: vec![
                Case {
                    name: "Mehrdeutig".into(),
                    after: "öffne Warenkorb".into(),
                    passed: true,
                    answer: String::new(),
                },
                Case {
                    name: "„Gekauft“".into(),
                    after: "klicke Kaufen".into(),
                    passed: false,
                    answer: "Nichts gefunden für \"Kaufen\".".into(),
                },
            ],
            seconds: 1.5,
            errors: vec![],
        }];
        let xml = junit(&suites);
        assert!(xml.contains("tests=\"2\" failures=\"1\""));
        assert!(xml.contains("name=\"01 &amp; &lt;x&gt;\""));
        assert!(xml.contains("<failure message=\"erwartet: „Gekauft“\">Nichts gefunden für &quot;Kaufen&quot;.</failure>"));
    }

    #[test]
    fn ausgabe_des_fork_runners() {
        let output = "\n=== /x/01.txt ===\n\n## file:///a.html\nSeite „A“.\n\n> Wähle 43\n  Select(\"43\") auf …\n(305 ms)\n  ✓ erwartet „Select(\"43\")“\n\n> klicke X\n  Nichts gefunden für „X“.\n(0 ms)\n  ✗ erwartet „Activate“ — FEHLT\n\n=== /x/02.txt ===\n!! Seite nicht ladbar: kein Baum\n\nErwartungen: 1 erfüllt, 1 nicht erfüllt\n";
        let suites = parse_fork_output(output);
        assert_eq!(suites.len(), 2);
        assert_eq!(suites[0].cases.len(), 2);
        assert!(suites[0].cases[0].passed);
        assert_eq!(suites[0].cases[1].name, "Activate");
        assert_eq!(suites[0].cases[1].after, "klicke X");
        assert_eq!(suites[0].cases[1].answer, "Nichts gefunden für „X“.");
        assert_eq!(suites[1].errors, vec!["Seite nicht ladbar: kein Baum"]);
    }
}
