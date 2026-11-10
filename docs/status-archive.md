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

## 2026-11-02 — M0 merged

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-02
**Current milestone:** M1 terminal — not started
**Next action:** write docs/milestones/m1-terminal/plan.md

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | done (pending CI on master) |
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

## 2026-11-03 — M1 executing (before close-out)

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-02
**Current milestone:** M1 terminal — plan written, executing
**Next action:** execute `docs/milestones/m1-terminal/plan.md` task by task (ledger: `.superpowers/sdd/progress.md`).

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

## 2026-11-03 — M2 tasks 1–10 landed

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-03
**Current milestone:** M2 splits-tabs — not started (plan to write)
**Next action:** write `docs/milestones/m2-splits-tabs/plan.md` from its spec and cmux's Swift split/tab views, then execute it task by task.

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | done (PR #1) |
| M1 terminal | done; exit criterion 1 confirmed by user, 2–3 (Ghostty screenshot pair, latency trace) not yet measured |
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
- gpui `=0.2.2` for UI; libghostty-vt (vendored from cmux @ 21dcc2b, Ghostty fork @ ef4f157) for terminal state.
- Dev setup: Zig 0.16.0 via `scripts/install-zig.sh`; `.tools/zig` on PATH. libc pinned to 0.2.189 in Cargo.lock; never bare `cargo update`. Toolchain pinned in `rust-toolchain.toml`.
- Local app only: no browser, cmux Cloud, Manaflow relays. v1 = M0–M6; M7–M11 add the buildable extras.
- Target: KDE Plasma Wayland first, macOS second. ⌘ → Super by default, all keys remappable.
- Terminal behaviour follows Ghostty source where cmux delegates to Ghostty (M1 journal lists the cases, e.g. Ctrl+[ sends CSI-u).
- Config: reads Ghostty's own config (`~/.config/ghostty/config[.ghostty]`, includes, themes).

## Code map (M1)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/777 scanner, PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME.

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur deferred to M3: KWin 6.8 offers only `ext_background_effect_v1`; gpui 0.2.2 binds only `org_kde_kwin_blur`.
- Real IME (fcitx/ibus) unverified.

## Carry to M2+
- **Put early in the M2 plan:** PTY reader blocks until EOF, so a `setsid` grandchild holding the slave leaves a stuck pane (fix: poll plus wake pipe). Callbacks hold weak handles. Split prepaint before adding panes.
- **Also M2:** bindings become config-remappable, an explicit exit item (`keys.rs` BINDINGS is hard-coded). Ctrl+Shift+C with no selection should pass through to the app (Ghostty `performable`). Add an unsafe multi-line paste prompt (Ghostty `clipboard-paste-protection`).
- **M6:** a panic in `feed` kills the reader without `Exited`; add catch_unwind or a drop guard.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes; prepaint is long — split before adding panes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- M6: licence audit of gpui transitive deps (cargo-deny/about).

## Blockers
None.

## 2026-11-03 — M3 plan written

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-03
**Current milestone:** M2 splits-tabs — in progress on branch `m2-splits-tabs` (plan Tasks 1–10 done, CI green)
**Next action:** at the KDE desktop, run plan Task 11 (exit-criteria checks plus the unticked "Run it" steps), record the results in the journal, then Task 12 close-out (Steps 1, 3, 4; Step 2 done).

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | done (PR #1) |
| M1 terminal | done; exit criterion 1 confirmed by user, 2–3 (Ghostty screenshot pair, latency trace) not yet measured |
| M2 splits-tabs | code done (Tasks 1–10); exit checks (Task 11) pending at the desktop |
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
- gpui `=0.2.2` for UI; libghostty-vt (vendored from cmux @ 21dcc2b, Ghostty fork @ ef4f157) for terminal state.
- Dev setup: Zig 0.16.0 via `scripts/install-zig.sh`; `.tools/zig` on PATH. libc pinned to 0.2.189 in Cargo.lock; never bare `cargo update`. Toolchain pinned in `rust-toolchain.toml`.
- Local app only: no browser, cmux Cloud, Manaflow relays. v1 = M0–M6; M7–M11 add the buildable extras.
- Target: KDE Plasma Wayland first, macOS second. ⌘ → Super by default, all keys remappable.
- Terminal behaviour follows Ghostty source where cmux delegates to Ghostty (M1 journal lists the cases, e.g. Ctrl+[ sends CSI-u).
- Config: reads Ghostty's own config (`~/.config/ghostty/config[.ghostty]`, includes, themes).

## Code map (M1–M2)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/777 scanner, PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`.

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur deferred to M3: KWin 6.8 offers only `ext_background_effect_v1`; gpui 0.2.2 binds only `org_kde_kwin_blur`.
- Real IME (fcitx/ibus) unverified.

## Carry to M3+
- From the M2 plan: container size must come from the workspace area once the sidebar exists (`WorkspaceView::container`); last-surface quit → close workspace; macOS needs a pidfd replacement (M6).
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- M6: licence audit of gpui transitive deps (cargo-deny/about).

## Blockers
None.

## 2026-11-03 — M2 and M3 merged, desktop checks pending

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-03
**Current milestone:** M2 splits-tabs — in progress on branch `m2-splits-tabs` (plan Tasks 1–10 done, CI green)
**Next action:** at the KDE desktop, run plan Task 11 (exit-criteria checks plus the unticked "Run it" steps), record the results in the journal, then Task 12 close-out (Steps 1, 3, 4; Step 2 done). In parallel: M3 plan is written on branch `m3-workspaces-sidebar` (based on M2); execute it from Task 1 after rebasing onto `master` once M2 merges.

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | done (PR #1) |
| M1 terminal | done; exit criterion 1 confirmed by user, 2–3 (Ghostty screenshot pair, latency trace) not yet measured |
| M2 splits-tabs | code done (Tasks 1–10); exit checks (Task 11) pending at the desktop |
| M3 workspaces-sidebar | plan written (`docs/milestones/m3-workspaces-sidebar/plan.md`), not started |
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
- gpui `=0.2.2` for UI; libghostty-vt (vendored from cmux @ 21dcc2b, Ghostty fork @ ef4f157) for terminal state.
- Dev setup: Zig 0.16.0 via `scripts/install-zig.sh`; `.tools/zig` on PATH. libc pinned to 0.2.189 in Cargo.lock; never bare `cargo update`. Toolchain pinned in `rust-toolchain.toml`.
- Local app only: no browser, cmux Cloud, Manaflow relays. v1 = M0–M6; M7–M11 add the buildable extras.
- Target: KDE Plasma Wayland first, macOS second. ⌘ → Super by default, all keys remappable.
- Terminal behaviour follows Ghostty source where cmux delegates to Ghostty (M1 journal lists the cases, e.g. Ctrl+[ sends CSI-u).
- Config: reads Ghostty's own config (`~/.config/ghostty/config[.ghostty]`, includes, themes).

## Code map (M1–M2)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/777 scanner, PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`.

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur deferred to M3: KWin 6.8 offers only `ext_background_effect_v1`; gpui 0.2.2 binds only `org_kde_kwin_blur`.
- Real IME (fcitx/ibus) unverified.

## Carry to M3+
- From the M2 plan: container size must come from the workspace area once the sidebar exists (`WorkspaceView::container`); last-surface quit → close workspace; macOS needs a pidfd replacement (M6).
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- M6: licence audit of gpui transitive deps (cargo-deny/about).

## Blockers
None.

## 2026-11-10 — before M4 plan

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-03
**Current milestone:** M2 and M3 code merged to `master` (PR #3, PR #4); both milestones' desktop exit checks pending
**Next action:** at the KDE desktop run M2 plan Task 11 and M3 plan Task 12 Steps 1–3 (plus every unticked "Run it" step in both plans), record results in each journal, fix what fails on `master`; then write `docs/milestones/m4-notifications/plan.md`.

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | done (PR #1) |
| M1 terminal | done; exit criterion 1 confirmed by user, 2–3 (Ghostty screenshot pair, latency trace) not yet measured |
| M2 splits-tabs | merged (PR #3); exit checks (plan Task 11) pending at the desktop |
| M3 workspaces-sidebar | merged (PR #4); exit checks (plan Task 12 Steps 1–3) pending at the desktop |
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
- gpui `=0.2.2` for UI; libghostty-vt (vendored from cmux @ 21dcc2b, Ghostty fork @ ef4f157) for terminal state.
- Dev setup: Zig 0.16.0 via `scripts/install-zig.sh`; `.tools/zig` on PATH. libc pinned to 0.2.189 in Cargo.lock; never bare `cargo update`. Toolchain pinned in `rust-toolchain.toml`.
- Local app only: no browser, cmux Cloud, Manaflow relays. v1 = M0–M6; M7–M11 add the buildable extras.
- Target: KDE Plasma Wayland first, macOS second. ⌘ → Super by default, all keys remappable.
- Terminal behaviour follows Ghostty source where cmux delegates to Ghostty (M1 journal lists the cases, e.g. Ctrl+[ sends CSI-u).
- Config: reads Ghostty's own config (`~/.config/ghostty/config[.ghostty]`, includes, themes).
- Model: `cmux-core` is the pure model (split tree, workspace list); contextless gpui actions bound from `config.toml`; pidfd-based shell exit (Linux).
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate.

## Code map (M1–M3)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/777 scanner, PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux).

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim never run on a real Wayland session: rg §1.4 risks (borrowed surface proxy, second-queue roundtrip, oversized region) unverified.
- No M2/M3 UI has been seen on screen yet (all built against CI + reviews).
- Real IME (fcitx/ibus) unverified.

## Carry to M4+
- M3 parked: `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
- M4 owns sidebar notification text/badges. M6: macOS ports/cwd (no /proc; sidebar loses start-dir fallback), pidfd replacement, blur on macOS and Plasma ≤6.6.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- M6: licence audit of gpui transitive deps (cargo-deny/about).

## Blockers
None.

## 2026-11-10 — M4 merged, desktop checks pending

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-10
**Current milestone:** M4 notifications merged to `master` (PR #5) before its desktop checks (owner's call, 2026-11-10); M2, M3 and M4 desktop exit checks pending
**Next action:** at the KDE desktop: M4 plan Task 9 Step 1 (exits 1–3, the "Run it" checks and screenshots of Tasks 4–6, blur re-check), M2 plan Task 11, M3 plan Task 12 Steps 1–3; record results in each journal and fix on `master`; then write `docs/milestones/m5-cli-socket/plan.md`.

## Where things are
- Project design and milestone table: `docs/design.md`
- Milestone docs: `docs/milestones/<id>/` (`spec.md`, `plan.md`, `journal.md`)
- Full status history: `docs/status-archive.md`
- Upstream reference: https://github.com/manaflow-ai/cmux (Swift app = behaviour/visual reference)

## Milestones
| ID | State |
|---|---|
| M0 foundations | done (PR #1) |
| M1 terminal | done; exit criterion 1 confirmed by user, 2–3 (Ghostty screenshot pair, latency trace) not yet measured |
| M2 splits-tabs | merged (PR #3); exit checks (plan Task 11) pending at the desktop |
| M3 workspaces-sidebar | merged (PR #4); exit checks (plan Task 12 Steps 1–3) pending at the desktop |
| M4 notifications | merged (PR #5); desktop checks (plan Task 9 Step 1) pending |
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
- gpui `=0.2.2` for UI; libghostty-vt (vendored from cmux @ 21dcc2b, Ghostty fork @ ef4f157) for terminal state.
- Dev setup: Zig 0.16.0 via `scripts/install-zig.sh`; `.tools/zig` on PATH. libc pinned to 0.2.189 in Cargo.lock; never bare `cargo update`. Toolchain pinned in `rust-toolchain.toml`.
- Local app only: no browser, cmux Cloud, Manaflow relays. v1 = M0–M6; M7–M11 add the buildable extras.
- Target: KDE Plasma Wayland first, macOS second. ⌘ → Super by default, all keys remappable.
- Terminal behaviour follows Ghostty source where cmux delegates to Ghostty (M1 journal lists the cases, e.g. Ctrl+[ sends CSI-u).
- Config: reads Ghostty's own config (`~/.config/ghostty/config[.ghostty]`, includes, themes).
- Model: `cmux-core` is the pure model (split tree, workspace list); contextless gpui actions bound from `config.toml`; pidfd-based shell exit (Linux).
- Notifications (M4): cmux read rules in a pure `cmux-core` store; desktop bubbles via `zbus` (already in the tree), click activates with the notification's xdg_activation token (`activation.rs`, sharing `wayland.rs` with blur).
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate.

## Code map (M1–M4)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim never run on a real Wayland session: rg §1.4 risks (borrowed surface proxy, second-queue roundtrip, oversized region) unverified.
- No M2/M3/M4 UI has been seen on screen yet (all built against CI + reviews).
- Real IME (fcitx/ibus) unverified.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
M6: macOS ports/cwd (no /proc; sidebar loses start-dir fallback), pidfd replacement, blur on macOS and Plasma ≤6.6.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- M6: licence audit of gpui transitive deps (cargo-deny/about).

## Blockers
None.
