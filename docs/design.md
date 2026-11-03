# cmux-linux — Design

Approved 2026-11-02. This is the project-level spec. Each milestone refines it in
`docs/milestones/<id>/spec.md`.

## Goal

A native Rust port of the **terminal-workspace part** of [cmux](https://github.com/manaflow-ai/cmux)
that runs on Linux (primary target: KDE Plasma on Wayland) and macOS, and feels the
same as the macOS app: same layout, sidebar, notification rings, splits, and
shortcuts (⌘ mapped to Super).

cmux is open source, so this is a **port from source**, not reverse engineering.
The Swift app (`Sources/`) is the behavioural and visual reference.

## Non-goals

In-app browser and anything web-based, cloud/VMs, remote machines, SSH workspaces,
browser cookie import, Sparkle auto-update, iOS. The Swift CLI's hundreds of
cloud/browser/VM commands are out of scope.

## Constraints

- **License:** GPL-3.0-or-later (cmux's app license). We may vendor cmux's
  GPL code. We never copy from cmux's BUSL-1.1 directories (the server
  software; each has its own `LICENSE`). Check a directory's licence before
  reading code to port from it.
- **Fidelity over invention:** when unsure how something should look or behave,
  read the Swift source and match it. Record deviations in the milestone journal.
- **Linux has no ⌘:** every ⌘ shortcut maps to Super by default; every binding is
  remappable in config.

## Key decisions

| Decision | Choice | Why |
|---|---|---|
| UI toolkit | **gpui** (Zed's), pinned version | Draws everything itself on the GPU → identical look on Linux and macOS; Wayland + KWin blur; Zed's GPL-3 terminal view is a reference. |
| Terminal engine | **libghostty-vt** via cmux's `ghostty-vt` crates | Same engine as cmux/Ghostty. Fallback: `alacritty_terminal` if the bindings are too thin (decided in M0). |
| Terminal rendering | Own gpui element drawing the cell grid | Ghostty's full embedding API is macOS/iOS-only, so on Linux we render ourselves. |
| Agent integration | `cmux` CLI + Unix socket, same command names, flags, env vars as cmux | Existing Claude Code hooks work unchanged. |
| Config | Read `~/.config/ghostty/config` (fonts, theme, colours) + `~/.config/cmux-linux/config.toml` (shortcuts) | Matches cmux's "Ghostty compatible" behaviour. |

## Architecture

One Cargo workspace. Each crate has one job and is testable alone.

| Crate | Job | Depends on |
|---|---|---|
| `cmux-core` | Model: workspaces, split tree, panes, tabs (surfaces), notification store. No UI. | — |
| `cmux-term` | PTY + libghostty-vt wrapper; emits title, cwd (OSC 7), bell, notification (OSC 9/99/777) events. | `ghostty-vt` (vendored) |
| `cmux-meta` | Sidebar metadata: git branch (watch `.git/HEAD`), listening ports (`/proc`, by process tree), PR via `gh`. | `cmux-core` |
| `cmux-config` | Ghostty config parser + keybinding config. | — |
| `cmux-ipc` | Socket server + the `cmux` CLI binary. | `cmux-core` |
| `cmux-app` | gpui app: window, sidebar, split view, terminal element, shortcuts. | all |

**Data flow:** PTY bytes → `cmux-term` (libghostty-vt state + events) →
`cmux-core` model updates (title, cwd, notifications) → gpui re-renders the
affected views. CLI → socket → `cmux-ipc` → same model mutations. `cmux-meta`
polls/watches per workspace cwd and writes into the model.

**Errors:** a crashed shell closes only its surface; socket errors return
non-zero exit + message (as cmux); malformed config falls back to defaults and
shows a one-line warning, never a crash.

**Testing:** model, config parser, socket protocol and metadata collectors are
unit-tested without a window. Terminal correctness is checked by feeding
recorded byte streams to `cmux-term`. UI parity is checked by side-by-side
screenshots against the macOS app, recorded in the milestone journal.

## Milestones

Each milestone has one goal and a demo that proves it. Docs live in
`docs/milestones/<id>/` (spec, plan, journal). A milestone is done when its exit
criteria pass and STATUS.md says so.

| ID | Goal | Exit demo |
|---|---|---|
| M0 foundations | Toolchain and risky choices proven | Empty blurred gpui window on KDE Wayland; libghostty-vt builds and parses bytes; engine decision recorded; CI green. |
| M1 terminal | One fully usable terminal | Run Claude Code in a single window: correct colours/fonts from Ghostty config, input, mouse, selection, clipboard, scrollback, resize. |
| M2 splits-tabs | Panes and tabs | Split H/V, tab strip per pane, focus moves, drag-resize dividers, Super shortcuts, remap via config. |
| M3 workspaces-sidebar | Workspaces + sidebar | Multiple workspaces in a vertical sidebar showing cwd, git branch, ports, PR; Super+1–9 switching. |
| M4 notifications | Agents get your attention | OSC 9/99/777 → pane ring, tab highlight, sidebar text + badge, notification panel, Super+Shift+U, KDE desktop notification. |
| M5 cli-socket | Scriptable like cmux | `cmux notify`, `dismiss-notification`, new workspace, split, send keys, list, focus over the socket; Claude Code hooks work unchanged. |
| M6 parity-release | Feels like the Mac app, installable | Visual parity pass vs Swift app, macOS build, Linux package (AUR PKGBUILD), performance check with many panes. |

**Later (unscheduled):** session/layout restore, command palette, `cmux.json`
custom commands, SSH workspaces, Claude Teams.
