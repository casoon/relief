# Relief

Deutsch: [README.md](README.md)

> A normal website is a flat surface.
> **RELIEF** makes its structure tangible.

Relief is a research browser built on Chromium. It reads the accessibility
tree in the browser process, builds a semantic model of the page from it and
lets people operate the page by intent: via keyboard, speech or a command bar,
alongside VoiceOver or NVDA rather than in place of them. The same core also
runs as a test mode that checks user flows from the perspective of keyboard and
screen reader users.

Project site: <https://casoon.github.io/relief/>

## Status

Research project under development. Phase 0 (feasibility in our own Chromium
build) is complete; the Phase 1 interfaces run in the fork.

What exists today:

- A Chromium fork (`fork/`) reads the AXTree in the browser process,
  including cross-site iframes, and keeps it as a `SemanticGraph` in the Rust
  runtime. Commands are executed through `AXActionData`, with key presses as a
  fallback. Relief is on by default in its own build (also when started from
  the Dock); `--disable-relief` turns it off.
- The **Relief side panel** (eye icon or Ctrl+Shift+I) holds the command bar
  (Ctrl+Shift+Space) and the Semantic Inspector with findings from barrierlab;
  it switches to the **Semantic View**, where every interaction runs as a
  validated action on the original page.
- **Speech:** a talk button in the panel or Ctrl+Shift+S, recognition on the
  device, spoken answers; “abbrechen” (cancel) interrupts immediately.
- **Keyboard hints** (Ctrl+Shift+M), a **form assistant**, **consent dialogs**
  recognised and declined on request (never accepted).
- **Ability profiles:** output, input and presentation adapted to abilities,
  globally or per site, stored locally only.
- **Testing mode:** form assertions in task files, headless runs with JUnit
  reports, Playwright access through the CDP domain `Relief.*`
  (`examples/playwright/`). A CDP host (`relief-cdp`) runs the same tasks
  against an unmodified Chrome.

The command parser understands German. Commands such as „was ist hier“,
„gehe zu Suche“ or „fülle … mit …“ therefore stay German in the docs and task
files; they are input to Relief, not text to translate.

Details (German): [`docs/project-state.md`](docs/project-state.md).

## Layout

| Path | Contents |
|---|---|
| `crates/` | Rust core: model, interaction graph, AI contract, bridge, CDP host |
| `fork/` | Chromium fork: own code under `fork/relief/`, patch series, base version |
| `spike/` | Test pages, tasks and recordings |
| `docs/` | Current state, architecture, decisions, constraints |
| `plan/` | Specification and work packages, overview in [`plan/status.md`](plan/status.md) |
| `site/`, `pages/` | Project site |

Documentation, code comments and plans are written in German.

## Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Browser tasks against an installed Chrome and against the Chromium build are
described in [`docs/project-state.md`](docs/project-state.md).

## Principles

- Browser actions come only from validated plans; a model can at most propose
  one.
- Every statement carries its origin; uncertain information is never presented
  as fact.
- Screen readers keep working unchanged alongside Relief.

Shared accessibility building blocks come from
[barrierlab](https://github.com/casoon/barrierlab).

## License

MIT, see [`LICENSE`](LICENSE). Chromium code keeps its own licenses.
