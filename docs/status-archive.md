# Status archive

Append-only. Every time `STATUS.md` is replaced, the previous version is appended
here under a dated heading, newest last. Never edit past entries.

## 2026-11-02 — project created
Repo set up; design approved (`docs/design.md`); milestone specs M0–M6 written.
First STATUS.md written. Next: M0 plan.

## 2026-11-02 — before adding M7–M11

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-02
**Current milestone:** M0 foundations — not started
**Next action:** write `docs/milestones/m0-foundations/plan.md` (writing-plans), then execute it.

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | not started |
| M1 terminal | not started |
| M2 splits-tabs | not started |
| M3 workspaces-sidebar | not started |
| M4 notifications | not started |
| M5 cli-socket | not started |
| M6 parity-release | not started |

## Decisions so far
- Port from source (cmux is GPL-3.0-or-later); REA not needed. Our license: GPL-3.0-or-later.
- Never copy from cmux's BUSL-1.1 server directories.
- gpui for UI; libghostty-vt for terminal state (fallback `alacritty_terminal`, decided in M0).
- Scope: terminal workspace only. No browser, cloud, remote, SSH.
- Target: KDE Plasma Wayland first, macOS second. ⌘ → Super by default, all keys remappable.

## Open risks
- gpui is pre-1.0; API churn. Pin the version.
- libghostty-vt Rust bindings may not expose enough (cells, styles, OSC events).
- KWin blur via gpui unverified on this machine.

## Blockers
None.

## 2026-11-02 — before M0 plan written

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-02
**Current milestone:** M0 foundations — not started
**Next action:** write `docs/milestones/m0-foundations/plan.md` (writing-plans), then execute it.

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | not started |
| M1 terminal | not started |
| M2 splits-tabs | not started |
| M3 workspaces-sidebar | not started |
| M4 notifications | not started |
| M5 cli-socket | not started |
| M6 parity-release | not started |
| M7 feed | not started (post-v1) |
| M8 groups-dock | not started (post-v1) |
| M9 agent-coordination | not started (post-v1) |
| M10 persistence | not started (post-v1) |
| M11 productivity | not started (post-v1) |

## Decisions so far
- Port from source (cmux is GPL-3.0-or-later); REA not needed. Our license: GPL-3.0-or-later.
- Never copy from cmux's BUSL-1.1 server directories.
- gpui for UI; libghostty-vt for terminal state (fallback `alacritty_terminal`, decided in M0).
- Local app only: no browser, cmux Cloud, Manaflow relays. v1 = M0–M6; M7–M11 add the buildable extras.
- macOS/server-only features are "Not planned" in docs/design.md; local alternatives need user OK.
- Target: KDE Plasma Wayland first, macOS second. ⌘ → Super by default, all keys remappable.

## Open risks
- gpui is pre-1.0; API churn. Pin the version.
- libghostty-vt Rust bindings may not expose enough (cells, styles, OSC events).
- KWin blur via gpui not yet verified on KDE Plasma Wayland.

## Blockers
None.

## 2026-11-02 — M0 complete

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-02
**Current milestone:** M0 foundations — plan written, not started
**Next action:** execute `docs/milestones/m0-foundations/plan.md` task by task (Task 1 first).

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | not started |
| M1 terminal | not started |
| M2 splits-tabs | not started |
| M3 workspaces-sidebar | not started |
| M4 notifications | not started |
| M5 cli-socket | not started |
| M6 parity-release | not started |
| M7 feed | not started (post-v1) |
| M8 groups-dock | not started (post-v1) |
| M9 agent-coordination | not started (post-v1) |
| M10 persistence | not started (post-v1) |
| M11 productivity | not started (post-v1) |

## Decisions so far
- Port from source (cmux is GPL-3.0-or-later); REA not needed. Our license: GPL-3.0-or-later.
- Never copy from cmux's BUSL-1.1 server directories.
- gpui for UI; libghostty-vt for terminal state (fallback `alacritty_terminal`, decided in M0).
- Local app only: no browser, cmux Cloud, Manaflow relays. v1 = M0–M6; M7–M11 add the buildable extras.
- macOS/server-only features are "Not planned" in docs/design.md; local alternatives need user OK.
- Target: KDE Plasma Wayland first, macOS second. ⌘ → Super by default, all keys remappable.

## Open risks
- gpui is pre-1.0; API churn. Pin the version.
- libghostty-vt Rust bindings may not expose enough (cells, styles, OSC events).
- KWin blur via gpui not yet verified on KDE Plasma Wayland.

## Blockers
None.
