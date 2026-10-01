// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_PROFILE_STORE_H_
#define RELIEF_PROFILE_STORE_H_

#include <string>

#include "base/callback_list.h"
#include "base/files/file_path.h"
#include "base/functional/callback.h"
#include "base/memory/scoped_refptr.h"
#include "base/memory/weak_ptr.h"
#include "base/supports_user_data.h"

namespace base {
class SequencedTaskRunner;
}  // namespace base

namespace content {
class BrowserContext;
}  // namespace content

namespace relief {

// Fähigkeitsprofil eines Browserprofils (Paket 41): der Stand als JSON
// (Regeln in crates/relief-interaction/src/profile.rs), lokal in
// <Profil>/Relief/faehigkeiten.json, nie an Seiten oder Dienste. Ein
// Inkognito-Profil hält ihn nur im Speicher.
class ProfileStore : public base::SupportsUserData::Data {
 public:
  static ProfileStore& For(content::BrowserContext* context);

  explicit ProfileStore(content::BrowserContext* context);
  ~ProfileStore() override;

  const std::string& json() const { return json_; }
  // Neuer Stand: speichern und Beobachter benachrichtigen.
  void Set(std::string json);
  // Wird nach dem Laden und nach jeder Änderung gerufen.
  base::CallbackListSubscription Subscribe(base::RepeatingClosure callback);

 private:
  void Loaded(std::string json);

  base::FilePath path_;  // leer: nur im Speicher
  scoped_refptr<base::SequencedTaskRunner> file_runner_;
  std::string json_;
  base::RepeatingClosureList callbacks_;
  base::WeakPtrFactory<ProfileStore> weak_factory_{this};
};

}  // namespace relief

#endif  // RELIEF_PROFILE_STORE_H_
