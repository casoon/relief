---
title: Architecture
description: How Relief gets from Chromium’s accessibility tree to an action and back.
order: 2
---

## Pipeline

1. **Chromium** computes the accessibility tree of each frame: roles, names,
   states, actions, positions. Relief does not build a second tree from the DOM.
2. **Fork adapter** (C++, in Relief’s own directory) observes the accessibility
   updates of each tab in the browser process and keeps its own copy per frame.
   It does not switch on the operating system’s accessibility bridge; screen
   readers keep their own path.
3. **Bridge** hands every update as a compact delta to the Rust runtime.
4. **Semantic model** (Rust, browser-free): nodes with provenance, the page
   type, functional groups, the primary action and modal state per frame.
5. **Interaction** (Rust, browser-free): parses a command, resolves the target,
   rates the risk and produces a validated action plan.
6. **Back into Chromium**: the plan becomes an accessibility action on the
   node. Relief checks the effect in the next update instead of assuming it.

## Why this way

- The accessibility tree already contains what assistive technology needs.
  Building on it keeps Relief consistent with what screen readers get.
- A small patch set keeps the fork maintainable across Chromium releases.
- A browser-free core can be tested without a browser, against recorded pages,
  and shared with other accessibility tools.

## Shared building blocks

Relief uses the accessibility crates of **barrierlab** — perception of the
tree, accessible-name computation, rules and reports — and contributes back
what a second tool needs as well.
