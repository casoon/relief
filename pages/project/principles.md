---
title: Principles
description: Rules every Relief feature has to follow.
order: 3
---

- **Actions only through validated plans.** Every operation is a plan checked
  in Rust against the current page model. A confirmation authorises exactly
  that plan — target, value, destination — and nothing later.
- **A model suggests, never acts.** Relief works without any model. Models may
  propose a missing name or interpret a sentence; they never receive the power
  to trigger browser actions.
- **Every statement carries its origin.** Known from the page, inferred, or
  uncertain. Relief never presents a guess as a fact and never guesses a target:
  ambiguous targets become a numbered choice.
- **Screen readers keep working.** Relief runs alongside VoiceOver and NVDA
  and is tested against them.
- **Relief’s own interface is accessible.** Inspector, command bar, dialogs and
  settings follow a user-agent baseline: keyboard only, clear focus,
  restoreable focus, calm status messages, respect for system settings.
- **Privacy by default.** Nothing leaves the device unless the user chooses a
  cloud model; form values are filtered before anything is sent.
- **Never consent on the user’s behalf.** Cookie and consent dialogs are
  described and, on request, declined — never accepted.
