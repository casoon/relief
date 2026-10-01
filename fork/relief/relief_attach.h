// Relief. MIT-Lizenz wie das Relief-Repository.
//
// Einziger Header, den Chromium-Code außerhalb von //relief/ einbindet
// (chrome/browser/ui/tabs/tab_features.cc, Patch
// fork/patches/Relief-Tab-Helfer-je-Tab-erzeugen.patch). Klein gehalten,
// damit Änderungen in //relief/ tab_features.cc nicht neu übersetzen.

#ifndef RELIEF_RELIEF_ATTACH_H_
#define RELIEF_RELIEF_ATTACH_H_

#include "base/callback_list.h"

namespace tabs {
class TabInterface;
}  // namespace tabs

namespace relief {

// Mit --disable-relief passiert nichts (leere Subscription). Sonst hängt
// Relief an den WebContents des Tabs und nach einem Discard an den neuen;
// die Subscription lebt so lange wie die TabFeatures des Tabs.
base::CallbackListSubscription AttachToTab(tabs::TabInterface& tab);

}  // namespace relief

#endif  // RELIEF_RELIEF_ATTACH_H_
