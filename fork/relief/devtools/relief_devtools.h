// Relief. MIT-Lizenz wie das Relief-Repository.

#ifndef RELIEF_DEVTOOLS_RELIEF_DEVTOOLS_H_
#define RELIEF_DEVTOOLS_RELIEF_DEVTOOLS_H_

#include <cstdint>

#include "base/containers/span.h"

namespace content {
class DevToolsAgentHostClientChannel;
}  // namespace content

namespace relief {

// Eigene CDP-Domäne `Relief.*` (Paket 45, → crates/relief-bridge/src/
// devtools.rs). Aufgerufen aus ChromeDevToolsManagerDelegate (Patch) für
// jede Nachricht eines Clients: true, wenn sie zur Domäne gehört; die
// Antwort kommt dann asynchron über `channel`, sofern er noch besteht.
bool HandleDevToolsCommand(content::DevToolsAgentHostClientChannel* channel,
                           base::span<const uint8_t> message);

// Der Client hat sich getrennt; ausstehende Antworten an ihn entfallen.
void OnDevToolsClientDetached(
    content::DevToolsAgentHostClientChannel* channel);

}  // namespace relief

#endif  // RELIEF_DEVTOOLS_RELIEF_DEVTOOLS_H_
