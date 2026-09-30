// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_BRIDGE_RUNTIME_HOST_H_
#define RELIEF_BRIDGE_RUNTIME_HOST_H_

#include <optional>
#include <set>
#include <string>
#include <utility>

#include "base/files/file.h"
#include "base/files/file_path.h"
#include "base/functional/callback.h"
#include "base/time/time.h"
#include "relief/crates/relief-bridge/src/cxx_bridge.rs.h"

namespace relief {

// Zeiten eines AX-Pakets auf dem UI-Thread, für das Messprotokoll.
struct PacketTiming {
  // Protokollzeile: "packet" (AccessibilityEventReceived) oder "location"
  // (AccessibilityLocationChangesReceived).
  const char* kind = "packet";
  // Eingang des Pakets im Tab-Helfer.
  base::TimeTicks received;
  // Eigener Baum: AXTree::Unserialize aller Updates des Pakets bzw.
  // Übernahme der Positionen.
  base::TimeDelta unserialize;
  // AXNodeData → cxx-Strukturen, inklusive Positionen.
  base::TimeDelta convert;
  // AXTreeUpdates bzw. Positions- und Scroll-Änderungen im Paket.
  size_t updates = 0;
};

// Die Rust-Runtime eines Tabs auf ihrer eigenen Sequenz (nicht im
// AX-Callback des UI-Threads, → plan/spezifikation/02). Nimmt Deltas an,
// schreibt das Messprotokoll und plant beim Durchstich `activate`.
class RuntimeHost {
 public:
  using PerformCallback = base::RepeatingCallback<void(bridge::ActionPlan)>;
  // Für Browser-Tests (//relief/testing): jede angewandte Delta, auf der
  // Sequenz der Runtime.
  using DeltaCallback = base::RepeatingCallback<void(const bridge::Delta&)>;

  // `log`: Datei für das Protokoll, leer = LOG(INFO). `activate`: Name aus
  // --relief-activate. `log_roles`: Rollen aus --relief-log-nodes.
  // `perform` läuft auf dem UI-Thread.
  RuntimeHost(int tab,
              const base::FilePath& log,
              std::optional<std::string> activate,
              std::set<std::string> log_roles,
              PerformCallback perform);
  RuntimeHost(const RuntimeHost&) = delete;
  RuntimeHost& operator=(const RuntimeHost&) = delete;
  ~RuntimeHost();

  void Apply(bridge::Delta delta, PacketTiming timing);
  // Zeile ins Protokoll (auch für Ereignisse vom UI-Thread).
  void Log(const std::string& line);

  // Befehle in Sprache (→ crates/relief-bridge/src/command.rs), gegen den
  // Graphen nach allen bis dahin angewandten Deltas. `facts`: Angaben des
  // Renderers zum Ziel einer offenen Rückfrage, vorher ins Modell (Paket 75).
  // Die Entscheidungen der Sitzung schreibt erst LogSecurity() ins
  // Protokoll, damit eine neu gestellte Rückfrage (Reconfirm) dazugehört.
  bridge::Reply RunCommand(const std::string& input,
                           std::optional<bridge::FormFacts> facts);
  // Ziel der offenen Rückfrage (`found` false: keine).
  bridge::Found ConfirmationTarget();
  // Angaben zum Ziel ins Modell und die offene Rückfrage damit neu stellen.
  bridge::Reply Reconfirm(bridge::FormFacts facts);
  // Security-Log der Sitzung abholen und je Eintrag eine Zeile
  // "security\t<JSON>" schreiben (Entscheidung, Plan-ID, Aktionsart, Risiko,
  // Grund; keine Werte, keine Namen).
  void LogSecurity();
  std::string FinishCommand();
  // CDP-Domäne `Relief.*` (Paket 45); protokolliert die Methode.
  bridge::DevToolsReply DevToolsCommand(const std::string& method,
                                        const std::string& params);
  std::string DescribePage();
  std::string InspectorJson();
  rust::Vec<bridge::MarkBox> ShowMarks();
  bridge::Reply ShowNode(const std::string& key);
  uint64_t NodeCount();
  void SetDeltaObserverForTesting(DeltaCallback callback);

 private:
  void MaybeActivate();
  void LogDiff(const bridge::Delta& delta, base::TimeTicks now);
  void LogNodes(const bridge::Delta& delta);

  const int tab_;
  const base::TimeTicks start_;
  base::File file_;
  rust::Box<bridge::Runtime> runtime_;
  uint64_t version_ = 0;

  std::optional<std::string> activate_;
  PerformCallback perform_;
  DeltaCallback delta_observer_;
  // Zeitpunkt, zu dem `activate` geplant wurde; danach werden die Deltas
  // mit Namen protokolliert, um die Wirkung zu sehen.
  std::optional<base::TimeTicks> activated_at_;
  // --relief-log-nodes: protokollierte Rollen und die Knoten (Baum, ID),
  // deren Positionsänderungen deshalb mitgeschrieben werden.
  const std::set<std::string> log_roles_;
  std::set<std::pair<std::string, int>> logged_nodes_;
};

}  // namespace relief

#endif  // RELIEF_BRIDGE_RUNTIME_HOST_H_
