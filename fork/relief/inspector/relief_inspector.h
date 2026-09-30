// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_INSPECTOR_RELIEF_INSPECTOR_H_
#define RELIEF_INSPECTOR_RELIEF_INSPECTOR_H_

namespace tabs {
class TabInterface;
}  // namespace tabs

namespace relief {

// Semantic Inspector (Paket 20): Side-Panel-Eintrag je Tab mit der WebUI
// chrome://relief-inspector.top-chrome, die den Graph der Seite des Tabs
// live zeigt. Der Eintrag wird beim ersten Öffnen registriert (die
// Side-Panel-Registry des Tabs entsteht in TabFeatures::Init erst nach
// AttachToTab).

// Registriert die WebUI (einmal je Prozess).
void RegisterInspectorWebUI();

// Öffnet den Inspector im Side Panel des Tabs bzw. schließt ihn, wenn er
// dort schon offen ist. Ohne Browserfenster geschieht nichts (Rückgabe
// false).
void ToggleInspector(tabs::TabInterface& tab);
bool ShowInspector(tabs::TabInterface& tab);

}  // namespace relief

#endif  // RELIEF_INSPECTOR_RELIEF_INSPECTOR_H_
