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
build) is complete; Phase 1 is building the first user interfaces.

What exists today:

- A Chromium fork (`fork/`) reads the AXTree (on by default, `--disable-relief` turns it off),
  including cross-site iframes, in the browser process and keeps it as a
  `SemanticGraph` in the Rust runtime. Commands are executed through
  `AXActionData`, with key presses as a fallback.
- The Relief side panel is a WebUI with the command bar at the top
  (Ctrl+Shift+Space) and the Semantic Inspector below it (Ctrl+Shift+I or
  `--relief-inspector`): landmarks, headings and controls, live, with the
  origin of each accessible name.
- A CDP spike (`relief-cdp`) drives an unmodified Chrome through the same
  commands. Task files under `spike/tasks/` combine commands with expected
  results; `assert:` lines check the current state of a form (field names,
  error associations, focus, status messages, tab order) and report findings
  with `form/…` rule IDs.

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
