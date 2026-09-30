// Relief. MIT-Lizenz wie das Relief-Repository.

#include "relief/bridge/runtime_host.h"

#include <string_view>
#include <utility>

#include "base/containers/span.h"
#include "base/logging.h"
#include "base/strings/string_number_conversions.h"
#include "base/strings/stringprintf.h"

namespace relief {

namespace {

// Name eines Messknotens: Text „relief-probe:<ms seit Epoche>“, von
// scripts/fork-measure.mjs in die Seite geschrieben.
constexpr std::string_view kProbePrefix = "relief-probe:";

// So lange nach `activate` werden benannte Knoten der Deltas protokolliert.
constexpr base::TimeDelta kDiffWindow = base::Seconds(5);

std::string_view View(const rust::String& text) {
  return std::string_view(text.data(), text.size());
}

}  // namespace

RuntimeHost::RuntimeHost(int tab,
                         const base::FilePath& log,
                         std::optional<std::string> activate,
                         std::set<std::string> log_roles,
                         PerformCallback perform)
    : tab_(tab),
      start_(base::TimeTicks::Now()),
      runtime_(bridge::new_runtime()),
      activate_(std::move(activate)),
      perform_(std::move(perform)),
      log_roles_(std::move(log_roles)) {
  if (!log.empty()) {
    file_.Initialize(log,
                     base::File::FLAG_OPEN_ALWAYS | base::File::FLAG_APPEND);
  }
  Log("start");
}

RuntimeHost::~RuntimeHost() {
  Log(base::StringPrintf(
      "stop\tnodes=%llu",
      static_cast<unsigned long long>(bridge::node_count(*runtime_))));
}

void RuntimeHost::Log(const std::string& line) {
  const std::string out = base::StringPrintf(
      "%.3f\t%d\t%s\n", (base::TimeTicks::Now() - start_).InMillisecondsF(),
      tab_, line.c_str());
  if (file_.IsValid()) {
    file_.WriteAtCurrentPosAndCheck(base::as_byte_span(out));
  } else {
    LOG(INFO) << "relief\t" << tab_ << "\t" << line;
  }
}

bridge::Reply RuntimeHost::RunCommand(const std::string& input,
                                      std::optional<bridge::FormFacts> facts) {
  // Wert eines Ausfüll- oder Auswahlbefehls verdeckt (Paket 76).
  Log("command\t" + std::string(bridge::redact_input(input)));
  if (facts) {
    bridge::apply_form_facts(*runtime_, *facts);
  }
  return bridge::run_command(*runtime_, input);
}

bridge::Found RuntimeHost::ConfirmationTarget() {
  return bridge::confirmation_target(*runtime_);
}

bridge::Reply RuntimeHost::Reconfirm(bridge::FormFacts facts) {
  Log("reconfirm");
  bridge::apply_form_facts(*runtime_, facts);
  return bridge::reconfirm(*runtime_);
}

void RuntimeHost::LogSecurity() {
  for (const rust::String& event : bridge::take_security_log(*runtime_)) {
    Log("security\t" + std::string(event));
  }
}

bridge::DevToolsReply RuntimeHost::DevToolsCommand(const std::string& method,
                                                   const std::string& params) {
  Log("devtools\t" + method);
  return bridge::devtools_command(*runtime_, method, params);
}

std::string RuntimeHost::FinishCommand() {
  return std::string(bridge::finish_command(*runtime_));
}

std::string RuntimeHost::DescribePage() {
  return std::string(bridge::describe_page(*runtime_));
}

std::string RuntimeHost::InspectorJson() {
  return std::string(bridge::inspector_json(*runtime_));
}

rust::Vec<bridge::MarkBox> RuntimeHost::ShowMarks() {
  return bridge::show_marks(*runtime_);
}

bridge::Reply RuntimeHost::ShowNode(const std::string& key) {
  Log("show\t" + key);
  return bridge::show_node(*runtime_, key);
}

uint64_t RuntimeHost::NodeCount() {
  return bridge::node_count(*runtime_);
}

void RuntimeHost::SetDeltaObserverForTesting(DeltaCallback callback) {
  delta_observer_ = std::move(callback);
}

void RuntimeHost::Apply(bridge::Delta delta, PacketTiming timing) {
  const base::TimeTicks dequeued = base::TimeTicks::Now();
  delta.base = version_;
  const bridge::ApplyResult result = bridge::apply_delta(*runtime_, delta);
  const base::TimeTicks applied = base::TimeTicks::Now();
  if (!result.ok) {
    Log("error\tapply\t" + std::string(result.error));
    return;
  }
  version_ = result.version;

  size_t created = 0, changed = 0, removed = 0, bounds = 0;
  for (const bridge::TreeUpdate& tree : delta.trees) {
    created += tree.created.size();
    changed += tree.changed.size();
    removed += tree.removed.size();
    bounds += tree.bounds.size();
  }
  const base::TimeTicks ui_done =
      timing.received + timing.unserialize + timing.convert;
  Log(base::StringPrintf(
      "%s\tversion=%llu\ttrees=%zu\tupdates=%zu\tcreated=%zu\t"
      "changed=%zu\tremoved=%zu\tnodes=%llu\tunserialize_us=%lld\t"
      "convert_us=%lld\tqueue_us=%lld\tapply_us=%lld\tlatency_us=%lld\t"
      "bounds=%zu\tremoved_trees=%zu",
      timing.kind, static_cast<unsigned long long>(version_),
      delta.trees.size(), timing.updates, created, changed, removed,
      static_cast<unsigned long long>(bridge::node_count(*runtime_)),
      timing.unserialize.InMicroseconds(), timing.convert.InMicroseconds(),
      (dequeued - ui_done).InMicroseconds(),
      (applied - dequeued).InMicroseconds(),
      (applied - timing.received).InMicroseconds(), bounds,
      delta.removed_trees.size()));
  if (delta_observer_) {
    delta_observer_.Run(delta);
  }

  // Ende-zu-Ende: Seite schreibt Date.now() in einen Knoten, hier ist der
  // Graph aktualisiert. Beide Uhren sind die Systemzeit.
  const double now_ms = base::Time::Now().InMillisecondsFSinceUnixEpoch();
  bool probe_seen = false;
  for (const bridge::TreeUpdate& tree : delta.trees) {
    for (const auto* nodes : {&tree.created, &tree.changed}) {
      for (const bridge::Node& node : *nodes) {
        const std::string_view name = View(node.name);
        double sent_ms = 0;
        if (!probe_seen && node.has_name && name.starts_with(kProbePrefix) &&
            base::StringToDouble(name.substr(kProbePrefix.size()), &sent_ms)) {
          probe_seen = true;
          Log(base::StringPrintf("probe\tlatency_ms=%.1f\tnodes=%llu",
                                 now_ms - sent_ms,
                                 static_cast<unsigned long long>(
                                     bridge::node_count(*runtime_))));
        }
      }
    }
  }

  LogDiff(delta, applied);
  LogNodes(delta);
  MaybeActivate();
}

void RuntimeHost::LogDiff(const bridge::Delta& delta, base::TimeTicks now) {
  if (!activated_at_ || now - *activated_at_ > kDiffWindow) {
    return;
  }
  const double since_ms = (now - *activated_at_).InMillisecondsF();
  auto log_named = [&](const rust::Vec<bridge::Node>& nodes, char mark) {
    for (const bridge::Node& node : nodes) {
      if (node.has_name && !node.name.empty() &&
          View(node.role) != "inlineTextBox") {
        Log(base::StringPrintf("diff\t+%.1fms\t%c\t%s\t%s", since_ms, mark,
                               std::string(node.role).c_str(),
                               std::string(node.name).c_str()));
      }
    }
  };
  for (const bridge::TreeUpdate& tree : delta.trees) {
    log_named(tree.created, '+');
    log_named(tree.changed, '~');
    if (!tree.removed.empty()) {
      Log(base::StringPrintf("diff\t+%.1fms\t-\t%zu Knoten", since_ms,
                             tree.removed.size()));
    }
  }
}

void RuntimeHost::LogNodes(const bridge::Delta& delta) {
  if (log_roles_.empty()) {
    return;
  }
  auto rect = [](bool has, const bridge::Rect& r) {
    return has ? base::StringPrintf("%.1f\t%.1f\t%.1f\t%.1f", r.x, r.y, r.width,
                                    r.height)
               : std::string("-");
  };
  for (const bridge::TreeUpdate& tree : delta.trees) {
    const std::string tree_id(tree.tree);
    for (const auto* nodes : {&tree.created, &tree.changed}) {
      for (const bridge::Node& node : *nodes) {
        const std::string role(node.role);
        if (!log_roles_.contains(role)) {
          continue;
        }
        logged_nodes_.insert({tree_id, node.id});
        Log(base::StringPrintf("node\t%s\t%d\t%s\t%s\tignored=%d\t%s",
                               tree_id.c_str(), node.id, role.c_str(),
                               std::string(node.name).c_str(), node.ignored,
                               rect(node.has_bounds, node.bounds).c_str()));
      }
    }
    for (const bridge::BoundsChange& change : tree.bounds) {
      if (logged_nodes_.contains({tree_id, change.node})) {
        Log(base::StringPrintf("bounds\t%s\t%d\t%s", tree_id.c_str(),
                               change.node,
                               rect(change.has_bounds, change.bounds).c_str()));
      }
    }
  }
}

void RuntimeHost::MaybeActivate() {
  if (!activate_ || activated_at_) {
    return;
  }
  const bridge::Found found =
      bridge::find_node(*runtime_, *activate_, bridge::Action::DoDefault);
  if (!found.found) {
    return;
  }
  const bridge::ActionRequest request{found.tree,
                                      found.node,
                                      bridge::Action::DoDefault,
                                      /*has_value=*/false,
                                      rust::String(),
                                      found.version};
  bridge::ActionPlan plan = bridge::plan_action(*runtime_, request);
  if (!plan.ok) {
    Log(base::StringPrintf("activate\trejected\trejection=%d",
                           static_cast<int>(plan.rejection)));
    activate_.reset();
    return;
  }
  activated_at_ = base::TimeTicks::Now();
  Log(base::StringPrintf("activate\tplan\ttree=%s\tnode=%d\tversion=%llu",
                         std::string(plan.tree).c_str(), plan.node,
                         static_cast<unsigned long long>(plan.version)));
  perform_.Run(std::move(plan));
}

}  // namespace relief
