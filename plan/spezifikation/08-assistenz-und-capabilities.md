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

Offen [validieren]: Rendering als WebUI (HTML in Chromium, eigener
Renderer-Prozess) oder native Views. WebUI ist schneller entwickelt und selbst
barrierefrei testbar.

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
