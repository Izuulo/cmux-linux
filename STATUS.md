# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-02
**Current milestone:** M1 terminal — planning
**Next action:** write docs/milestones/m1-terminal/plan.md

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | done (PR #1, CI green on master) |
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
- gpui for UI; libghostty-vt for terminal state.
- Engine decision (M0): **libghostty-vt**, vendored from cmux @ 21dcc2b (Ghostty fork @ ef4f157). 191/191 vendored tests and the spike test passed. OSC 7/9/777 come from our `cmux-term::osc` scanner.
- Dev setup: Zig 0.16.0 via `scripts/install-zig.sh`; `.tools/zig` must be on PATH. libc pinned to 0.2.189 in Cargo.lock (xattr 0.2.3 needs `libc::ENOATTR`); don't `cargo update` it.
- Local app only: no browser, cmux Cloud, Manaflow relays. v1 = M0–M6; M7–M11 add the buildable extras.
- macOS/server-only features are "Not planned" in docs/design.md; local alternatives need user OK.
- Target: KDE Plasma Wayland first, macOS second. ⌘ → Super by default, all keys remappable.

## Open risks
- gpui is pre-1.0; API churn. Pin the version.
- Resolved (M0): libghostty-vt bindings expose cells, styles, cursor and title; spike passed. Gaps are listed under "Carry to M1".
- Blur open, deferred to M3: window is translucent but not blurred on Plasma 6.8.2 Wayland. KWin 6.8 offers only `ext_background_effect_v1`; gpui 0.2.2 binds only `org_kde_kwin_blur`.

## Carry to M1
- Wire the `on_pty_write` callback so DA/DSR query replies go back to the PTY.
- Surface bell and title-change events.
- Add a resize/scroll API.
- Avoid `Snapshot`'s per-frame cell clones and double row walk.
- `Snapshot`: add dims (cols/rows) and `#[derive(Debug)]`.
- Pin the toolchain with `rust-toolchain.toml` (floating stable + `-D warnings` can break CI).
- Lint tests too: `clippy --all-targets`.
- OSC scanner misses 8-bit C1 OSC (0x9d), which libghostty-vt does normalize.
- rustfmt isn't installed locally (`rustup component add rustfmt`).
- Clippy in CI uses `--no-deps --exclude ghostty-vt --exclude ghostty-vt-sys` (vendored lint); drop once clean.

## Blockers
None.
