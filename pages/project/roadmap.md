---
title: Roadmap
description: Phases of the project, what has been shown and what is planned.
order: 1
---

Relief is developed in phases. Each phase ends with something that runs and
has been measured, not with a document.

## Phase 0 — Feasibility (done)

- A spike against a normal Chrome showed that the accessibility tree is enough
  to understand and operate typical pages.
- A Chromium 154 fork builds on Apple silicon from a script. Incremental builds
  of Relief’s own code take seconds.
- Relief receives the accessibility tree of every tab in the browser process,
  including cross-site iframes, keeps a semantic page model in a Rust runtime
  and can trigger an action on a node; the result shows up in the next update.
- Updating the model takes at most 1.1 ms at the 95th percentile on large
  real pages (Wikipedia, spiegel.de).
- The fork touched Chromium in two places outside its own directory.
- VoiceOver works unchanged with Relief running, checked step by step with and
  without Relief.

Decision on 2026-09-30: **go** for phase 1.

## Phase 1 and later — Assistance

| Step | What it brings | Status |
|---|---|---|
| Actions into the page | Every operation as a validated plan through Chromium’s accessibility actions | done |
| Inspector | See what Relief understands about a page, with findings from shared accessibility rules | done |
| Command bar | “What can I do here?”, “open the menu”, numbered choices when a target is ambiguous | done |
| Speech | Speech input and output on the device, no cloud service required | spoken commands and answers done; live microphone next |
| Keyboard hints | Every actionable element reachable with a few keys, also inside iframes | done |
| Form assistant | Required fields, errors and focus after submit, filling fields by command | done |
| Consent and overlays | Recognise, describe, decline on request — never accept on the user’s behalf | done |
| Semantic view | A simplified view generated from the model; the original page stays available | done in the side panel; full tab width deferred |
| Ability profiles | Combinable settings for output, input and presentation — no diagnosis modes | next |
| Missing names | Suggestions for unlabelled controls, on-device where possible, always marked as uncertain | resolver built; calibration run pending |
| Accessible own interface | Inspector, command bar and dialogs usable by keyboard and screen reader | next |

## Testing mode

| Step | What it brings | Status |
|---|---|---|
| Form assertions | Names, error association, focus after errors, status messages, tab sequence | done |
| Real screen-reader output | A driver that records what VoiceOver says; NVDA later | planned |
| Headless runs | Test files run without a window and report as JUnit | done |
| Playwright | Existing Playwright tests can drive Relief and query its page model | done |

## Product basics

- The build is branded as Relief with its own profile directory (done).
- Before anyone else gets a build: signed, notarised, self-updating releases
  that keep up with Chromium’s security updates.
- Windows and Linux follow once build and test hosts exist; a platform only
  counts as supported when its screen readers and input methods pass the same
  core tasks.

## Deferred

A user study with people who rely on assistive technology is planned once the
contacts exist. Until then the project claims technical results, not benefit.
