# 08 · Assistenz und Capability Profile

## Fähigkeiten statt Diagnosen [Entscheidung]

Keine Modi wie `blind-mode`, `colorblind-mode`. Stattdessen ein Profil
verfügbarer bzw. benötigter Fähigkeiten, frei kombinierbar:

```rust
pub struct Capabilities {
    pub visual_detail: Level,          // Full | Reduced | None
    pub text_scale: f32,
    pub color_discrimination: Level,
    pub contrast_requirement: Level,
    pub motion_tolerance: Level,
    pub audio_output: Availability,    // Preferred | Available | Unavailable
    pub speech_input: Availability,
    pub keyboard_input: Availability,
    pub pointer_input: Availability,
    pub switch_input: Availability,
    pub text_complexity: Level,        // kognitive Komplexität
}
```

Aus dem Profil berechnet der Browser Darstellung und Bedienangebote. Das
Profil ist lokal, wird nie an Seiten oder Dienste übertragen (es wäre ein
Gesundheitsdatum im Sinne der DSGVO) [Annahme].

Offen [validieren]: Vorbelegung aus OS-Einstellungen (macOS: Reduce Motion,
Increase Contrast, VoiceOver aktiv, Textgröße) — naheliegend, spart Onboarding.

## Visual Assist

Größere Texte · vergrößerte Controls · Kontrastanpassung · reduzierte
Bewegung · Fokusverstärkung · vereinfachte Ansicht · Semantic View.

Einiges davon kann Chromium schon (Zoom, `prefers-reduced-motion`,
`forced-colors`) — erst nutzen, dann ergänzen.

## Semantic View

```
[ Original ] [ Simplified ] [ Semantic ]
```

Eine vom Interaction Graph (→ 04) gerenderte Oberfläche:

```
Nike Air Max
129,00 €
Größe  [ 39 ] [ 40 ] [ 41 ] [ 42 ] [ 43 ]
[ In den Warenkorb ]
Produktbeschreibung …
```

Die Originalseite bleibt geladen und funktional; jede Bedienung im Semantic
View wird als validierte Aktion (→ 05) auf die Originalseite ausgeführt.
Besonders relevant für motorische und visuelle Einschränkungen.

Vorbild/Konkurrenz: Chromium Reading Mode (→ 01). Unterschied: Reading Mode ist
lesend, Semantic View ist **bedienend**.

### Umsetzung im Fork (Paket 29) [belegt]

- **Ort und Form [Entscheidung]:** WebUI im Relief-Side-Panel, wie der
  Inspector (→ 01, „Semantic Inspector“), umschaltbar über „Ansicht:
  Befehle und Inspector · Semantische Ansicht“ (native Radiobuttons). Die
  Originalseite bleibt daneben sichtbar und ist die „Originalansicht“;
  zurück geht jederzeit über denselben Schalter oder Escape (Panel zu).
  Native Controls statt nachgebauter Rollen (→ 13).
- **Daten** (`crates/relief-bridge/src/semantic.rs`, im Inspector-JSON als
  `semantic`): Überschriften, Texte und Bedienelemente des Interaction
  Graph in Dokumentreihenfolge mit ihrem Bereich; Texte in einem
  Bedienelement nur als dessen Name; Herkunft unsicherer Namen und
  „gesperrt“ bei offenem modalem Dialog sichtbar; sensible Werte nie im
  Klartext.
- **Bedienung:** Button/Link → auslösen, Auswahl → `Select`, Textfeld →
  `SetValue` beim Übernehmen (Eingabetaste/Verlassen, nicht je Taste),
  Kontrollkästchen/Radio → auslösen (Zustand kommt von der Seite zurück),
  Zahlenfeld → erhöhen/verringern. Jede Bedienung geht als
  `Runtime::view_act` über `Session::request`: dieselbe Validierung,
  Rückfrage bei Risiko und dasselbe Security-Log wie ein Befehl, samt
  Formularziel aus dem Renderer (→ 07, Paket 75). Rückfragen erscheinen in
  der Ansicht mit „Ja, ausführen“/„Abbrechen“; der Fokus bleibt in der
  Ansicht. Das Protokoll nennt Art und Schlüssel, nie den Wert.
- **Wechsel ohne Seitenaktion:** Umschalten ändert nur das Panel. Die
  Ansicht setzt den Fokus auf den zuletzt gewählten Eintrag bzw. die
  Position der Sitzung; ist er verschwunden, sagt die Statuszeile das und
  die Ansicht beginnt oben. Zurück im Inspector ist derselbe Eintrag
  gewählt. Fokus und angefangene Eingabe bleiben über Aktualisierungen
  erhalten.
- **Darstellung (→ 13):** Größen in `em` (Zoom, Textgröße), Systemfarben
  (`GrayText`, `Highlight`; hoher Kontrast), keine Animationen bei
  reduzierter Bewegung, Fokus immer sichtbar.
- **Belegt:** `relief_browsertests --gtest_filter=*SemantischeAnsicht*`
  (Wechsel ohne Wirkung auf der Seite; Größe wählen, in den Warenkorb,
  Name ausfüllen, Newsletter anhaken, „Jetzt kaufen“ erst nach „Ja,
  ausführen“; Ort über den Wechsel; jedes Bedienelement mit Namen im
  AX-Baum), `tests/inspector.rs`
  `semantische_ansicht_bedient_ueber_validierte_aktionen`; Sichtprüfung
  auf `spike/fixtures/shop-clean.html`.

**Abgrenzung zu Chromium Reading Mode** [belegt: Code, 154.0.8037.58]:
Reading Mode (`chrome/browser/ui/views/side_panel/read_anything/`,
`read_anything_untrusted_page_handler.cc`) zeigt im Side Panel den
**Hauptinhalt** einer Seite zum Lesen (Text, Überschriften, Links, Bilder,
mit Schrift-, Abstands- und Farbeinstellungen und Vorlesen), gewonnen aus
dem AXTree mit HTML-Modus und Inhaltsauswahl. Er bietet keine
Bedienelemente der Seite an und ändert nichts an ihr. Die Semantic View
zeigt **alle** Bereiche samt Navigation, Suche und Formularen und ist
**bedienend**: Jede Aktion wirkt als validierter Plan auf die Originalseite,
mit Rückfrage bei Risiko. Beide leben im Side Panel und lesen den AXTree;
Relief übernimmt von Reading Mode nur das Muster, nicht den Code.

Offen (→ Paket 113): vereinfachte Ansicht (dritte Stufe zwischen Original
und semantisch) und eine Ansicht über die ganze Tab-Breite statt im Panel.

## Speech Assist

```
Sprache → STT → Intent → Semantic Runtime → Antwort/Aktion → TTS
```

Sprache ist eine eigene Ebene: „Was ist auf dieser Seite?" geht nicht an
Chromium, sondern wird zu `{"intent": "describe_page"}`.

Dialogbeispiel:

- „Was ist auf dieser Seite?" → Produktseite Nike Air Max, Informationen,
  Bilder, Größenauswahl, Warenkorb.
- „Welche Größen gibt es?" → `inspect_control(size-selector)` → „39 bis 43,
  42 ist ausgewählt."
- „Nimm 43." → `select(size-selector, 43)` (MEDIUM) → „Größe 43 gewählt."

Randbedingungen [Annahme]:

- Web-Speech-Erkennung fällt im Fork weg (→ 01). Wie bei KI (→ 06) gilt:
  keine Hardware-Voraussetzung, Cloud-STT per API-Key als Option.
- Schicht austauschbar halten (Trait in Rust, Adapter je Plattform im Fork).
- Zwei Stimmen gleichzeitig sind unbenutzbar → bei aktivem Screenreader
  Relief-Ausgabe über dessen Ansagen leiten oder stummschalten [validieren].

| | macOS | Windows | Linux |
|---|---|---|---|
| STT on-device | `SFSpeechRecognizer` | `Windows.Media.SpeechRecognition` | kein brauchbarer Standard → kleines lokales Modell (Vosk/whisper.cpp) oder Cloud |
| TTS | OS-Stimmen über Chromium-TTS | OS-Stimmen über Chromium-TTS | speech-dispatcher (eSpeak NG) |
| Screenreader | VoiceOver | NVDA, JAWS, Narrator | Orca |


## Action Assist

Natürlichsprachliche Aktionen („Öffne den Warenkorb", „Wähle Größe 43",
„Schließe den Dialog") — ausschließlich über validierte Aktionen (→ 05).
Dieselbe Pipeline bedient Sprache, Command-Leiste und Semantic View.
