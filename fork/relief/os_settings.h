// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_OS_SETTINGS_H_
#define RELIEF_OS_SETTINGS_H_

namespace relief {

// Systemeinstellungen für einen vorgeschlagenen Startwert des
// Fähigkeitsprofils (Paket 115), nie als Diagnose.
struct OsSettings {
  bool reduce_motion = false;
  bool increase_contrast = false;
};

OsSettings ReadOsSettings();

}  // namespace relief

#endif  // RELIEF_OS_SETTINGS_H_
