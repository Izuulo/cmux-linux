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

This is the **local** app. Anything web-based beyond the M13 in-app browser,
cmux Cloud/VMs, Manaflow relays and phone sync, browser cookie import, iOS. The
Swift CLI's cloud/VM commands are out of scope. See "Not planned" for local
alternatives.

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
| UI toolkit | **gpui** (Zed's), pinned version | Draws everything itself on the GPU → identical look on Linux and macOS; Wayland; KWin blur needs `ext_background_effect_v1`, planned for M3; Zed's GPL-3 terminal view is a reference. |
| Terminal engine | **libghostty-vt** via vendored cmux crates (decided M0, see journal) | Same engine as cmux/Ghostty. |
| Terminal rendering | Own gpui element drawing the cell grid | Ghostty's full embedding API is macOS/iOS-only, so on Linux we render ourselves. |
| Agent integration | `cmux` CLI + Unix socket, same command names, flags, env vars as cmux | Existing Claude Code hooks work unchanged. |
| Config | Read `~/.config/ghostty/config` (fonts, theme, colours) + `~/.config/cmux-linux/config.toml` (shortcuts) | Matches cmux's "Ghostty compatible" behaviour. |

## Architecture

One Cargo workspace. Each crate has one job and is testable alone.

| Crate | Job | Depends on |
|---|---|---|
| `cmux-core` | Model: workspaces, split tree, panes, tabs (surfaces), notification store. No UI. | — |
| `cmux-term` | PTY + libghostty-vt wrapper; emits title, cwd (OSC 7), bell, notification (OSC 9/99/777) events. | `ghostty-vt` (vendored) |
| `cmux-meta` | Sidebar metadata: git branch (`.git/HEAD`), listening ports (`/proc`, by controlling TTY), PR via `gh`. | `serde_json` |
| `cmux-config` | Ghostty config parser + keybinding config. | `toml` |
| `cmux-ipc` | Socket server, cmux wire format and handles, model snapshot for socket reads, the `cmux` CLI binary. No gpui. | `serde_json`, `uuid`, `libc`, `futures` |
| `cmux-app` | gpui app: window, sidebar, split view, terminal element, shortcuts, notification UI; desktop notifications over D-Bus (`zbus`, Linux). | all |

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
| M0 foundations | Toolchain and risky choices proven | Empty translucent gpui window on KDE Wayland (blur → M3); libghostty-vt builds and parses bytes; engine decision recorded; CI green. |
| M1 terminal | One fully usable terminal | Run Claude Code in a single window: correct colours/fonts from Ghostty config, input, mouse, selection, clipboard, scrollback, resize. |
| M2 splits-tabs | Panes and tabs | Split H/V, tab strip per pane, focus moves, drag-resize dividers, Super shortcuts, remap via config. |
| M3 workspaces-sidebar | Workspaces + sidebar | Multiple workspaces in a vertical sidebar showing cwd, git branch, ports, PR; Super+1–9 switching. |
| M4 notifications | Agents get your attention | OSC 9/99/777 → pane ring, tab highlight, sidebar text + badge, notification panel, Super+Shift+U, KDE desktop notification. |
| M5 cli-socket | Scriptable like cmux | `cmux notify`, `dismiss-notification`, new workspace, split, send keys, list, focus over the socket; Claude Code hooks work unchanged. |
| M6 parity-release | Feels like the Mac app, installable | Visual parity pass vs Swift app, macOS build, Linux package (AUR PKGBUILD), performance check with many panes. |
| M7 feed | Approve agents without switching panes | Claude Code permission and plan approvals answered from the right sidebar Feed. |
| M8 groups-dock | Organise many agents | Collapsible workspace groups in the sidebar; right-side Dock of terminals. |
| M9 agent-coordination | Agents work together | `cmux agent message`, Claude Teams as native splits, workspace auto-naming, automations. |
| M10 persistence | Nothing lost on restart | Layout restore, Vault agent-session resume, opt-in `local-tmux` persistence. |
| M11 productivity | Faster everyday use | Command palette, `cmux.json` custom commands, SSH workspaces. |
| M12 settings | Configure without editing files | Settings window for cmux-linux options and the Ghostty config (themes, blur, opacity, fonts), applied live. |
| M13 browser | Web pages next to terminals | A browser surface in a pane or tab with URL bar and navigation; links and `cmux browser open` land there. |
| M14 mac-look | Looks native to macOS Tahoe | A shared `mac` UI kit; animated glass popovers with arrows, menus, palette, title bar, System Settings–style Settings and browser chrome built on it. |

M0–M6 are v1. M7–M14 come after, in order, unless STATUS.md records a change.

## Not planned

These cmux features depend on macOS or Manaflow's servers. Local alternatives
are welcome but unscheduled; add one as a milestone only with the user's OK.

| cmux feature | Why not | Possible local alternative |
|---|---|---|
| iOS Simulator pane | Needs Xcode | Works only in the macOS build, if ever added there |
| cmux Cloud, VMs | Paid Manaflow service | SSH workspaces (M11) to your own machines |
| iOS app, phone sync, push | Manaflow relays (BUSL-1.1) | KDE Connect forwards desktop notifications to a phone |
| Custom SwiftUI sidebars | Needs a SwiftUI interpreter | Sidebar fields driven by a script or config file |
| Computer use (`cmux-cua`) | macOS backend | A Linux MCP backend, if one exists |
| Sparkle auto-update | macOS framework | AUR / pacman updates (M6) |
| Privileged `sudo` helper | macOS helper | `pkexec`/polkit prompt |
