// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/os_settings.h"

#import <AppKit/AppKit.h>

namespace relief {

OsSettings ReadOsSettings() {
  NSWorkspace* workspace = NSWorkspace.sharedWorkspace;
  return OsSettings{workspace.accessibilityDisplayShouldReduceMotion,
                    workspace.accessibilityDisplayShouldIncreaseContrast};
}

}  // namespace relief
