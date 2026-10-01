// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_BRANDING_STRINGS_H_
#define RELIEF_BRANDING_STRINGS_H_

namespace relief {

// Ersetzt „Chromium“ als Produktname in sichtbaren, übersetzten Texten durch
// „Relief“ (Paket 111), zur Laufzeit über
// ResourceBundle::OverrideLocaleStringResource: Die Übersetzungen bleiben,
// ein Eingriff in .grd/.xtb würde ihre IDs ändern. Chromium als Projekt
// (Copyright, „Open-Source-Projekt Chromium“) bleibt stehen. Einmal je
// Prozess, vor dem ersten Tab.
void ApplyBrandingStrings();

}  // namespace relief

#endif  // RELIEF_BRANDING_STRINGS_H_
