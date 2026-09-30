// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/branding_strings.h"

#include <string>

#include "base/strings/string_util.h"
#include "chrome/grit/branded_strings.h"
#include "components/strings/grit/components_branded_strings.h"
#include "ui/base/l10n/l10n_util.h"
#include "ui/base/resource/resource_bundle.h"

namespace relief {

namespace {

// Texte mit „Chromium“ nur als Produktname: Über-Seite, Menüs,
// Standardbrowser, Neustart und Aktualisierung.
constexpr int kProductOnly[] = {
    IDS_ABOUT,
    IDS_APP_MENU_PRODUCT_NAME,
    IDS_APPMENU_TOOLTIP_UPDATE_AVAILABLE,
    IDS_DEFAULT_BROWSER_CHANGED_MESSAGE,
    IDS_DEFAULT_BROWSER_INFOBAR_TEXT,
    IDS_FIRSTRUN_DLG_MAC_SET_DEFAULT_BROWSER_LABEL,
    IDS_MACOS_OBSOLETE,
    IDS_QUIT_WITH_APPS_TITLE,
    IDS_RELAUNCH_CONFIRMATION_DIALOG_TITLE,
    IDS_RELAUNCH_TO_UPDATE,
    IDS_SETTINGS_ABOUT_PROGRAM,
    IDS_SETTINGS_DEFAULT_BROWSER_DEFAULT,
    IDS_SETTINGS_DEFAULT_BROWSER_ERROR,
    IDS_SETTINGS_DEFAULT_BROWSER_MAKE_DEFAULT,
    IDS_SETTINGS_DEFAULT_BROWSER_SECONDARY,
    IDS_SETTINGS_GET_HELP_USING_CHROME,
    IDS_SETTINGS_UPGRADE_SUCCESSFUL_RELAUNCH,
    IDS_SETTINGS_UPGRADE_UP_TO_DATE,
    IDS_SETTINGS_UPGRADE_UPDATING,
    IDS_SETTINGS_UPGRADE_UPDATING_PERCENT,
    IDS_UPDATE_RECOMMENDED_DIALOG_TITLE,
    IDS_UPGRADE_BUBBLE_MENU_ITEM,
    IDS_UPGRADE_BUBBLE_TEXT,
    IDS_UPGRADE_BUBBLE_TITLE,
};

// „Relief wird durch das Open-Source-Projekt Chromium … ermöglicht“: nur
// das erste „Chromium“ ist das Produkt.
constexpr int kProductFirst[] = {
    IDS_VERSION_UI_LICENSE,
};

void Override(int id, bool all) {
  std::u16string text = l10n_util::GetStringUTF16(id);
  if (all) {
    base::ReplaceSubstringsAfterOffset(&text, 0, u"Chromium", u"Relief");
  } else {
    base::ReplaceFirstSubstringAfterOffset(&text, 0, u"Chromium", u"Relief");
  }
  ui::ResourceBundle::GetSharedInstance().OverrideLocaleStringResource(id,
                                                                       text);
}

}  // namespace

void ApplyBrandingStrings() {
  static bool applied = false;
  if (applied) {
    return;
  }
  applied = true;
  for (int id : kProductOnly) {
    Override(id, /*all=*/true);
  }
  for (int id : kProductFirst) {
    Override(id, /*all=*/false);
  }
}

}  // namespace relief
