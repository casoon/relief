// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_RELIEF_SWITCHES_H_
#define RELIEF_RELIEF_SWITCHES_H_

namespace relief::switches {

// Schaltet Relief ein. Ohne diesen Schalter hängt sich Relief nirgends ein
// und fordert keinen Accessibility-Modus an.
inline constexpr char kEnableRelief[] = "enable-relief";

// Zusätzlich ui::AXMode::kScreenReader anfordern. Blink serialisiert dann
// auch display-gesperrte Inhalte (content-visibility), wie CDP im Spike
// (→ plan/spezifikation/01, Abschnitt 3). Standard: ohne.
inline constexpr char kReliefScreenReaderMode[] = "relief-screen-reader-mode";

// Messprotokoll (Tab-getrennte Zeilen) in diese Datei anhängen. Ohne den
// Schalter gehen die Zeilen mit LOG(INFO) nach stderr.
inline constexpr char kReliefLog[] = "relief-log";

// Durchstich für den Rückweg: sobald ein Knoten mit genau diesem Namen, der
// DoDefault meldet, im Graphen steht, plant die Runtime `activate` und der
// Tab-Helfer schickt es als AXActionData an den Frame. Einmal je Tab.
inline constexpr char kReliefActivate[] = "relief-activate";

// Nur lesend, für Nachweise auf echten Seiten: Knoten dieser Rollen
// (kommagetrennt, Rollennamen wie im Graphen, z. B. „button,iframe“) mit
// Namen und Position ins Protokoll schreiben, sobald eine Delta sie anlegt,
// ändert oder verschiebt (Zeilen `node`/`bounds`).
inline constexpr char kReliefLogNodes[] = "relief-log-nodes";

// Aufgabendateien (spike/tasks/*.txt, kommagetrennt) im ersten Tab
// abarbeiten wie `relief-cdp run`, Aktionen über AXActionData; Ausgabe auf
// stdout, danach beendet sich der Browser (Rückgabewert 1 bei nicht
// erfüllten Erwartungen). Relative URLs in den Dateien gelten relativ zur
// Datei.
inline constexpr char kReliefRun[] = "relief-run";

// Inspector im Side Panel des ersten Tabs nach dem ersten Laden öffnen.
// Sonst öffnet und schließt ihn Strg+Umschalt+I auf der Seite.
inline constexpr char kReliefInspector[] = "relief-inspector";

// Sprungmarken im ersten Tab nach dem ersten Laden zeigen (Nachweise,
// Vorführung). Sonst Strg+Umschalt+M auf der Seite.
inline constexpr char kReliefMarks[] = "relief-marks";

}  // namespace relief::switches

#endif  // RELIEF_RELIEF_SWITCHES_H_
