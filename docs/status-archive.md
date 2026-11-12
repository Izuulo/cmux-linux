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

## 2026-11-10 — M5 planned, awaiting review

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-10
**Current milestone:** M5 cli-socket: research and plan written (`docs/milestones/m5-cli-socket/`), awaiting the owner's review; M2, M3 and M4 desktop exit checks still pending
**Next action:** owner reviews M5 `plan.md` and picks an execution mode, then branch `m5-cli-socket` and Task 1. At the KDE desktop whenever possible: M4 plan Task 9 Step 1, M2 plan Task 11, M3 plan Task 12 Steps 1–3 (record in each journal, fix on `master`).

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
| M5 cli-socket | planned (research + plan, not yet reviewed); criterion 1 = `cmux notify` hooks (owner's ruling) |
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

## 2026-11-10 — M5 built on branch, before merge

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-10
**Current milestone:** M5 cli-socket built on branch `m5-cli-socket` (Tasks 1–11, final review fixed, Linux CI green); not merged. M2, M3, M4 and now M5 desktop checks pending
**Next action:** owner OK to merge `m5-cli-socket` (PR). At the KDE desktop: M5 plan Task 12 Step 1 (Claude Code hook, `command -v cmux`, tmux denial, second instance, Run-it of Tasks 9–11), M4 plan Task 9 Step 1, M2 plan Task 11, M3 plan Task 12 Steps 1–3; record in each journal.

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
| M5 cli-socket | built on branch (not merged); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; desktop check pending |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate.

## Code map (M1–M5)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`.
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim never run on a real Wayland session: rg §1.4 risks (borrowed surface proxy, second-queue roundtrip, oversized region) unverified.
- No M2–M5 UI has been seen on screen yet (all built against CI + reviews). gpui 0.2.2 hangs in `Application::new` on the macOS host (ActionRegistry insert loop), so the app only runs on Linux for now.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
M6: macOS ports/cwd (no /proc; sidebar loses start-dir fallback), pidfd replacement, blur on macOS and Plasma ≤6.6.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; inherited `CMUX_SOCKET_PATH` passes through when no server; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. M6: `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
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

## 2026-11-10 — M5 merged, before M6 plan

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-10
**Current milestone:** M5 cli-socket merged to `master` (PR #6). Its Run-it steps (Tasks 9–11) passed on the macOS host with a release build; KDE desktop checks for M2–M5 pending
**Next action:** at the KDE desktop: M5 plan Task 12 Step 1 (real Claude Code hook, `command -v cmux`, tmux denial, second instance), M4 plan Task 9 Step 1, M2 plan Task 11, M3 plan Task 12 Steps 1–3; record in each journal. Then write `docs/milestones/m6-parity-release/plan.md`.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate.

## Code map (M1–M5)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`.
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim never run on a real Wayland session: rg §1.4 risks (borrowed surface proxy, second-queue roundtrip, oversized region) unverified.
- No M2–M5 UI has been seen on screen yet (all built against CI + reviews). On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
M6: macOS ports/cwd (no /proc; sidebar loses start-dir fallback), pidfd replacement, blur on macOS and Plasma ≤6.6.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; inherited `CMUX_SOCKET_PATH` passes through when no server; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. M6: `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
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

## 2026-11-11 — M6 plan written, before Tasks 1–7

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-10
**Current milestone:** M6 parity-release, plan written (`docs/milestones/m6-parity-release/plan.md`), branch `m6-parity-release`. KDE desktop checks for M2–M5 pending (folded into M6 Task 8)
**Next action:** owner reviews the M6 plan and picks an execution mode; then M6 Task 1 (macOS Zig rows + macOS CI job). Tasks 1–7 run on the Mac; Task 8 (install, perf, parity spot check, pending M2–M5 desktop checks) at the KDE desktop.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | plan written, not started |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate.

## Code map (M1–M5)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`.
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim never run on a real Wayland session: rg §1.4 risks (borrowed surface proxy, second-queue roundtrip, oversized region) unverified.
- No M2–M5 UI has been seen on screen yet (all built against CI + reviews). On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
M6: macOS ports/cwd (no /proc; sidebar loses start-dir fallback), pidfd replacement, blur on macOS and Plasma ≤6.6.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; inherited `CMUX_SOCKET_PATH` passes through when no server; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. M6: `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
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

## 2026-11-11 — M7 plan written

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-11
**Current milestone:** M6 parity-release on branch `m6-parity-release` (pushed, CI green on linux/macos/arch). Plan Tasks 1–7 done and reviewed on the macOS host; Task 8 (KDE desktop) left.
**Next action:** at the CachyOS desktop, M6 plan Task 8: install with `cd packaging/arch && makepkg -si` (needs `rustup`, not `rust`), Steps 1–4 (installed-app checks, `scripts/perf-panes.sh` on KDE = also M1 exit 3, parity spot check, the pending M2–M5 desktop checks), then Step 5 docs, PR and merge.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | Tasks 1–7 done (macOS); Task 8 at the KDE desktop pending; exit 2 (CachyOS install) and Linux perf not yet run |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate; a `macos` CI job builds/tests release.
- M6: macOS cwd/ports via libproc (`cmux-meta/darwin.rs`); no exit watch on macOS (it revokes the slave on shell exit, so EOF suffices); inherited `CMUX_*` removed in `Session::shell_command()`; package installs to `/usr/lib/cmux-linux/` + `/usr/bin` symlinks; licence gate `deny.toml`; scrollback 10 MB/terminal (cmux ships 50 MB, but our history is uncompressed).

## Code map (M1–M6)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`; M6 `darwin` (macOS libproc).
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim never run on a real Wayland session: rg §1.4 risks (borrowed surface proxy, second-queue roundtrip, oversized region) unverified.
- No M2–M5 UI has been seen on screen yet (all built against CI + reviews). On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.
- M6 parity: panel, 3-tab strip and ring flash were matched from Swift source only, never rendered; owner's cmux.app wasn't a clean reference (non-default config). Remaining gaps (default theme, full-width titlebar band, panel extras, pressed-icon opacity, scroll fades) are in the M6 journal.
- M6 perf on macOS: loaded p95 10.7 ms, CPU 0.1 % after stop; footprint 971 MB at the old 50 MB scrollback, not re-measured at 10 MB. Linux CPU path of `perf-panes.sh` untested.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
Plasma ≤6.6 blur (`org_kde_kwin_blur`): only if Task 8 finds no `ext_background_effect_v1`. M10: map Ghostty `scrollback-limit` and drive libghostty idle compression.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. Not in M6 spec (owner may add): `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- Packaging: crate licence texts (`cargo about`) not shipped yet, before AUR publish; `-git` PKGBUILD only, no release tag; makepkg's Ghostty source still clones from GitHub.
- Shell exit test on Linux now uses perl (`setsid` via fifo marker).

## Blockers
None.

## 2026-11-11 — M7 plan written, awaiting review

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-11
**Current milestone:** M7 feed: plan written on branch `m7-feed` (cut from the unmerged `m6-parity-release`), awaiting the owner's review. M6 Tasks 1–7 done; M6 Task 8 (KDE desktop) still left.
**Next action:** owner reviews `docs/milestones/m7-feed/plan.md` and picks an execution method. Separately, at the CachyOS desktop, M6 plan Task 8: install with `cd packaging/arch && makepkg -si` (needs `rustup`, not `rust`), Steps 1–4 (installed-app checks, `scripts/perf-panes.sh` on KDE = also M1 exit 3, parity spot check, the pending M2–M5 desktop checks), then Step 5 docs, PR and merge.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | Tasks 1–7 done (macOS); Task 8 at the KDE desktop pending; exit 2 (CachyOS install) and Linux perf not yet run |
| M7 feed | plan written (`docs/milestones/m7-feed/plan.md`), not started |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate; a `macos` CI job builds/tests release.
- M6: macOS cwd/ports via libproc (`cmux-meta/darwin.rs`); no exit watch on macOS (it revokes the slave on shell exit, so EOF suffices); inherited `CMUX_*` removed in `Session::shell_command()`; package installs to `/usr/lib/cmux-linux/` + `/usr/bin` symlinks; licence gate `deny.toml`; scrollback 10 MB/terminal (cmux ships 50 MB, but our history is uncompressed).

## Code map (M1–M6)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`; M6 `darwin` (macOS libproc).
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim never run on a real Wayland session: rg §1.4 risks (borrowed surface proxy, second-queue roundtrip, oversized region) unverified.
- No M2–M5 UI has been seen on screen yet (all built against CI + reviews). On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.
- M6 parity: panel, 3-tab strip and ring flash were matched from Swift source only, never rendered; owner's cmux.app wasn't a clean reference (non-default config). Remaining gaps (default theme, full-width titlebar band, panel extras, pressed-icon opacity, scroll fades) are in the M6 journal.
- M6 perf on macOS: loaded p95 10.7 ms, CPU 0.1 % after stop; footprint 971 MB at the old 50 MB scrollback, not re-measured at 10 MB. Linux CPU path of `perf-panes.sh` untested.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
Plasma ≤6.6 blur (`org_kde_kwin_blur`): only if Task 8 finds no `ext_background_effect_v1`. M10: map Ghostty `scrollback-limit` and drive libghostty idle compression.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. Not in M6 spec (owner may add): `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- Packaging: crate licence texts (`cargo about`) not shipped yet, before AUR publish; `-git` PKGBUILD only, no release tag; makepkg's Ghostty source still clones from GitHub.
- Shell exit test on Linux now uses perl (`setsid` via fifo marker).

## Blockers
None.

## 2026-11-11 — before the M6 KDE desktop run

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-11
**Current milestone:** M7 feed on branch `m7-feed` (cut from the unmerged `m6-parity-release`): Tasks 1–8 done and reviewed; Task 9 Step 1 (Mac smoke) partial, Step 3 docs done. M6 Task 8 (KDE desktop) still left.
**Next action:** owner, in this order: (a) manual UI check of the Feed on the Mac (buttons, feedback field typing, question picks, `✓ Submitted` and expired dimming, and the possible bug where opening the right sidebar wipes visible terminal content); (b) M7 Task 9 Step 2, a real Claude Code run (first `type claude` in a cmux-linux terminal started from cmux.app: expect no cmux.app wrapper; `cmux hooks setup` twice, a Bash permission from a background workspace answered in the Feed, a plan approved with Manual, then `cmux hooks uninstall` and a diff of the settings; edits `~/.claude/settings.json` and spends Claude usage); (c) at the CachyOS desktop, M6 plan Task 8: `cd packaging/arch && makepkg -si` (needs `rustup`, not `rust`), Steps 1–4 (installed-app checks, `scripts/perf-panes.sh` on KDE = also M1 exit 3, parity spot check, pending M2–M5 desktop checks), Step 5 docs, PR, merge; then rebase M7 onto `master` and PR it.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | Tasks 1–7 done (macOS); Task 8 at the KDE desktop pending; exit 2 (CachyOS install) and Linux perf not yet run |
| M7 feed | Tasks 1–8 done + reviewed (`m7-feed`); smoke partial (socket side passes, UI unseen); Task 9 Steps 2, 4 pending |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate; a `macos` CI job builds/tests release.
- Feed (M7): cmux's hook protocol (one `PermissionRequest` hook, v2 `feed.push` held until answered, 115 s wait then `{}` so Claude shows its own prompt); `cmux hooks setup|uninstall` edits Claude's settings (no `claude` wrapper); hook acts only with `CMUX_LINUX=1`; one `FeedEnd` exit, wrong-kind replies rejected; store newest first, cap 2000, not persisted; no Ultraplan/Bypass/free-text answers; ctrl+4 = Feed only inside the sidebar.
- M6: macOS cwd/ports via libproc (`cmux-meta/darwin.rs`); no exit watch on macOS (it revokes the slave on shell exit, so EOF suffices); inherited `CMUX_*` removed in `Session::shell_command()`; package installs to `/usr/lib/cmux-linux/` + `/usr/bin` symlinks; licence gate `deny.toml`; scrollback 10 MB/terminal (cmux ships 50 MB, but our history is uncompressed).

## Code map (M1–M7)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`; M6 `darwin` (macOS libproc).
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- M7: `cmux-core::feed` (`FeedStore`, `FeedKind::accepts`, `Decision`); `cmux-ipc::feed` (JSON <-> model, `hook_output`), `cli/hooks.rs` (`feed`, `setup`, `uninstall`); `cmux-config` `Binding.context`; app `window_view/feed_ctl.rs` (waiters, `feed_finish`, timer), `feed.rs` (right sidebar, permission cards, keys), `feed_cards.rs` (plan, question).
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim never run on a real Wayland session: rg §1.4 risks (borrowed surface proxy, second-queue roundtrip, oversized region) unverified.
- No M2–M5 UI has been seen on screen yet (all built against CI + reviews). On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.
- M7 Feed UI never seen drawn (cards, ring, badge, dimming, button clicks, feedback field, question picks all unverified; only socket/hook behaviour was smoke-tested). Possible bug: opening the right sidebar wiped visible terminal content (check `terminal_view` resize/reflow and scrollback). Real Claude Code run (Task 9 Step 2) not done.
- M7 parked (details in its journal): one notification per surface (two cards on a surface share it); stdin read has no timeout; `CMUX_LINUX` would leak into cmux.app launched from our terminal; settings key order relies on workspace `preserve_order` feature unification (check packaging); final-review follow-ups not done: bind `CMUX_LINUX` to our socket path; CLI tests for invalid JSON, uninstall refusal, `HOME` fallback, `--settings-file` precedence; fake-error test for the hook; sidebar-wipe is probably pre-existing resize/reflow (toggle the left sidebar to compare).
- M6 parity: panel, 3-tab strip and ring flash were matched from Swift source only, never rendered; owner's cmux.app wasn't a clean reference (non-default config). Remaining gaps (default theme, full-width titlebar band, panel extras, pressed-icon opacity, scroll fades) are in the M6 journal.
- M6 perf on macOS: loaded p95 10.7 ms, CPU 0.1 % after stop; footprint 971 MB at the old 50 MB scrollback, not re-measured at 10 MB. Linux CPU path of `perf-panes.sh` untested.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
Plasma ≤6.6 blur (`org_kde_kwin_blur`): only if Task 8 finds no `ext_background_effect_v1`. M10: map Ghostty `scrollback-limit` and drive libghostty idle compression.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. Not in M6 spec (owner may add): `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- Packaging: crate licence texts (`cargo about`) not shipped yet, before AUR publish; `-git` PKGBUILD only, no release tag; makepkg's Ghostty source still clones from GitHub.
- Shell exit test on Linux now uses perl (`setsid` via fifo marker).

## Blockers
None.

## 2026-11-11 — before M8 close-out

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-11
**Current milestone:** M6 Task 8 close-out on branch `m6-desktop-checks` (M6 and M7 are merged, PRs #7 and #8): the agent-run part of the KDE desktop run is done and journaled; M8 groups-dock starts on top of it.
**Next action:** agent: M8 (plan, then build). Owner, when convenient: (a) `cd packaging/arch && makepkg -si` (needs the `rustup` package, or `-d` with rustup.rs), start from the KDE launcher, check icon/name and that a notification bubble is attributed to cmux-linux; (b) the mouse and typing checks no script can do: divider, tab and row drag, rename typing, Feed buttons and feedback field, clicking a bubble, IME, `CMUX_TRACE_LATENCY=1` typing (M1 exit 3), and whether `background-blur` really blurs; (c) M7 Task 9 Step 2, a real Claude Code run with `cmux hooks setup`.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | merged (PR #7); KDE run done by script (journal 2026-11-11): perf pass, UI draws, D-Bus notification OK; owner-only checks left (install with sudo, mouse/typing, blur by eye) |
| M7 feed | merged (PR #8); Feed seen drawn on KDE (permission, question, empty state); buttons unclicked; real Claude Code run (Task 9 Step 2) pending |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate; a `macos` CI job builds/tests release.
- Feed (M7): cmux's hook protocol (one `PermissionRequest` hook, v2 `feed.push` held until answered, 115 s wait then `{}` so Claude shows its own prompt); `cmux hooks setup|uninstall` edits Claude's settings (no `claude` wrapper); hook acts only with `CMUX_LINUX=1`; one `FeedEnd` exit, wrong-kind replies rejected; store newest first, cap 2000, not persisted; no Ultraplan/Bypass/free-text answers; ctrl+4 = Feed only inside the sidebar.
- M6: macOS cwd/ports via libproc (`cmux-meta/darwin.rs`); no exit watch on macOS (it revokes the slave on shell exit, so EOF suffices); inherited `CMUX_*` removed in `Session::shell_command()`; package installs to `/usr/lib/cmux-linux/` + `/usr/bin` symlinks; licence gate `deny.toml`; scrollback 10 MB/terminal (cmux ships 50 MB, but our history is uncompressed).

## Code map (M1–M7)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`; M6 `darwin` (macOS libproc).
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- M7: `cmux-core::feed` (`FeedStore`, `FeedKind::accepts`, `Decision`); `cmux-ipc::feed` (JSON <-> model, `hook_output`), `cli/hooks.rs` (`feed`, `setup`, `uninstall`); `cmux-config` `Binding.context`; app `window_view/feed_ctl.rs` (waiters, `feed_finish`, timer), `feed.rs` (right sidebar, permission cards, keys), `feed_cards.rs` (plan, question).
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim runs on Plasma 6.8.2 without errors (KWin offers `ext_background_effect_manager_v1`); that the backdrop is really blurred is not yet confirmed by eye.
- M2–M7 UI has been seen drawn on KDE only through socket-driven screenshots; nothing was clicked, dragged or typed into by hand yet. On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.
- M7 Feed: cards draw; button clicks, feedback field and question picks unverified. The "sidebar wipes the terminal" report is not a bug (readline redraw when the pane is exactly prompt-wide; no OSC 133 shell integration). Real Claude Code run not done.
- M7 parked (details in its journal): one notification per surface (two cards on a surface share it); stdin read has no timeout; `CMUX_LINUX` would leak into cmux.app launched from our terminal; settings key order relies on workspace `preserve_order` feature unification (check packaging); final-review follow-ups not done: bind `CMUX_LINUX` to our socket path; CLI tests for invalid JSON, uninstall refusal, `HOME` fallback, `--settings-file` precedence; fake-error test for the hook.
- M6 parity: panel, 3-tab strip and ring flash were matched from Swift source only, never rendered; owner's cmux.app wasn't a clean reference (non-default config). Remaining gaps (default theme, full-width titlebar band, panel extras, pressed-icon opacity, scroll fades) are in the M6 journal.
- M6 perf on macOS: loaded p95 10.7 ms, CPU 0.1 % after stop; footprint 971 MB at the old 50 MB scrollback, not re-measured at 10 MB. Linux (KDE, release): RSS 483 MB under 19 floods, CPU 0.8 % after stop.

- Scripted UI checks: `debug.shortcut.simulate {combo}` (raw v2 request) presses a chord; needs `CMUX_SOCKET_MODE=allowAll` from outside the app.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
Plasma ≤6.6 blur (`org_kde_kwin_blur`): only if Task 8 finds no `ext_background_effect_v1`. M10: map Ghostty `scrollback-limit` and drive libghostty idle compression.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. Not in M6 spec (owner may add): `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- Packaging: crate licence texts (`cargo about`) not shipped yet, before AUR publish; `-git` PKGBUILD only, no release tag; makepkg's Ghostty source still clones from GitHub.
- Shell exit test on Linux now uses perl (`setsid` via fifo marker).

## Blockers
None.

## 2026-11-11 — before M9 Part 1 close-out

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-11
**Current milestone:** M8 groups-dock on branch `m8-groups-dock` (stacked on `m6-desktop-checks`, PR #9): plan Tasks 1–8 done, PR open. M9 agent-coordination is next.
**Next action:** agent: M9 (research cmux `docs/agent-messages.md` and automations, write `plan.md`, build on top of `m8-groups-dock`). Owner, when convenient: (a) review and merge PR #9 then the M8 PR; (b) the mouse and typing checks no script can do, now including M8's (group header click, `+`, chevron, menus, rename field, Delete prompt, drags, sidebar edge drag, Dock "New Terminal"), plus the older list: divider/tab/row drag, Feed buttons, clicking a bubble, IME, `CMUX_TRACE_LATENCY=1` (M1 exit 3), blur by eye; (c) `makepkg -si` and a launcher start; (d) M7 Task 9 Step 2, a real Claude Code run with `cmux hooks setup`.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | merged (PR #7); KDE run done by script (journal 2026-11-11): perf pass, UI draws, D-Bus notification OK; owner-only checks left (install with sudo, mouse/typing, blur by eye) |
| M7 feed | merged (PR #8); Feed seen drawn on KDE (permission, question, empty state); buttons unclicked; real Claude Code run (Task 9 Step 2) pending |
| M8 groups-dock | built (`m8-groups-dock`); both exit criteria checked on KDE by script and screenshot; mouse paths unverified |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate; a `macos` CI job builds/tests release.
- Feed (M7): cmux's hook protocol (one `PermissionRequest` hook, v2 `feed.push` held until answered, 115 s wait then `{}` so Claude shows its own prompt); `cmux hooks setup|uninstall` edits Claude's settings (no `claude` wrapper); hook acts only with `CMUX_LINUX=1`; one `FeedEnd` exit, wrong-kind replies rejected; store newest first, cap 2000, not persisted; no Ultraplan/Bypass/free-text answers; ctrl+4 = Feed only inside the sidebar.
- M6: macOS cwd/ports via libproc (`cmux-meta/darwin.rs`); no exit watch on macOS (it revokes the slave on shell exit, so EOF suffices); inherited `CMUX_*` removed in `Session::shell_command()`; package installs to `/usr/lib/cmux-linux/` + `/usr/bin` symlinks; licence gate `deny.toml`; scrollback 10 MB/terminal (cmux ships 50 MB, but our history is uncompressed).

## Code map (M1–M7)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`; M6 `darwin` (macOS libproc).
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- M7: `cmux-core::feed` (`FeedStore`, `FeedKind::accepts`, `Decision`); `cmux-ipc::feed` (JSON <-> model, `hook_output`), `cli/hooks.rs` (`feed`, `setup`, `uninstall`); `cmux-config` `Binding.context`; app `window_view/feed_ctl.rs` (waiters, `feed_finish`, timer), `feed.rs` (right sidebar, permission cards, keys), `feed_cards.rs` (plan, question).
- M8: `cmux-core::groups` (`Group`, `Row`, `Placement`, `HeaderClick`); `cmux-ipc` `Kind::WorkspaceGroup`, `GroupSnap`, `cli/groups.rs`; app `window_view/groups.rs` (group actions), `menu.rs` (context menu), `group_ctl.rs` (socket), `dock.rs` (right sidebar frame, Dock, `dock.json`); `WorkspaceView::new_dock`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- M8: nothing in it was clicked or dragged (see Next action b). The Dock is invisible to the socket: `cmux notify`, the Feed hook and OSC notifications don't work from Dock terminals, and CLI commands can't target them. Dock keeps the main area's minimum pane size, so side-by-side splits need a wide sidebar. Groups and Dock are not persisted (M10).
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim runs on Plasma 6.8.2 without errors (KWin offers `ext_background_effect_manager_v1`); that the backdrop is really blurred is not yet confirmed by eye.
- M2–M7 UI has been seen drawn on KDE only through socket-driven screenshots; nothing was clicked, dragged or typed into by hand yet. On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.
- M7 Feed: cards draw; button clicks, feedback field and question picks unverified. The "sidebar wipes the terminal" report is not a bug (readline redraw when the pane is exactly prompt-wide; no OSC 133 shell integration). Real Claude Code run not done.
- M7 parked (details in its journal): one notification per surface (two cards on a surface share it); stdin read has no timeout; `CMUX_LINUX` would leak into cmux.app launched from our terminal; settings key order relies on workspace `preserve_order` feature unification (check packaging); final-review follow-ups not done: bind `CMUX_LINUX` to our socket path; CLI tests for invalid JSON, uninstall refusal, `HOME` fallback, `--settings-file` precedence; fake-error test for the hook.
- M6 parity: panel, 3-tab strip and ring flash were matched from Swift source only, never rendered; owner's cmux.app wasn't a clean reference (non-default config). Remaining gaps (default theme, full-width titlebar band, panel extras, pressed-icon opacity, scroll fades) are in the M6 journal.
- M6 perf on macOS: loaded p95 10.7 ms, CPU 0.1 % after stop; footprint 971 MB at the old 50 MB scrollback, not re-measured at 10 MB. Linux (KDE, release): RSS 483 MB under 19 floods, CPU 0.8 % after stop.

- Scripted UI checks: `debug.shortcut.simulate {combo}` (raw v2 request) presses a chord; needs `CMUX_SOCKET_MODE=allowAll` from outside the app.
- Groups (M8): runs in the workspace list, anchor first, kept by `Workspaces::normalize`; the sidebar draws `rows()`; no empty groups; notification bump stays inside the group; inline group rename; a context menu instead of a palette. Dock (M8): one window-owned `WorkspaceView` in the right sidebar (mode bar Feed/Dock, draggable width), seeded by `~/.config/cmux-linux/dock.json`.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
Plasma ≤6.6 blur (`org_kde_kwin_blur`): only if Task 8 finds no `ext_background_effect_v1`. M10: map Ghostty `scrollback-limit` and drive libghostty idle compression.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. Not in M6 spec (owner may add): `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- Packaging: crate licence texts (`cargo about`) not shipped yet, before AUR publish; `-git` PKGBUILD only, no release tag; makepkg's Ghostty source still clones from GitHub.
- Shell exit test on Linux now uses perl (`setsid` via fifo marker).

## Blockers
None.

## 2026-11-11 — before M9 Parts 2–4

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-11
**Current milestone:** M9 agent-coordination on branch `m9-agent-coordination` (stacked on `m8-groups-dock`, which is stacked on `m6-desktop-checks`): Part 1 of 4, agent messages, is built; Parts 2–4 (auto-naming, automations, Claude Teams) not started.
**Next action:** agent: M9 Part 2 (workspace auto-naming; detail its plan section first), then Parts 3 and 4. Owner, when convenient: (a) review and merge the stack in order: #9 (M6 desktop checks), #10 (M8), then the M9 PR; (b) one real Claude Code run with `cmux hooks setup`: covers M7 Task 9 Step 2 and M9 exit 1 (send `cmux agent message` to a busy Claude and to an idle one); (c) the mouse and typing checks no script can do: M8's group header click, `+`, chevron, menus, rename field, Delete prompt, drags, sidebar edge drag, Dock "New Terminal"; older: divider/tab/row drag, Feed buttons, clicking a bubble, IME, `CMUX_TRACE_LATENCY=1` (M1 exit 3), blur by eye; (d) `makepkg -si` and a launcher start.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | merged (PR #7); KDE run done by script (journal 2026-11-11): perf pass, UI draws, D-Bus notification OK; owner-only checks left (install with sudo, mouse/typing, blur by eye) |
| M7 feed | merged (PR #8); Feed seen drawn on KDE (permission, question, empty state); buttons unclicked; real Claude Code run (Task 9 Step 2) pending |
| M8 groups-dock | built, PR #10; both exit criteria checked on KDE by script and screenshot; mouse paths unverified |
| M9 agent-coordination | Part 1 (agent messages) built and checked against the app with hand-run hooks; real Claude run pending; Parts 2–4 not started |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate; a `macos` CI job builds/tests release.
- Feed (M7): cmux's hook protocol (one `PermissionRequest` hook, v2 `feed.push` held until answered, 115 s wait then `{}` so Claude shows its own prompt); `cmux hooks setup|uninstall` edits Claude's settings (no `claude` wrapper); hook acts only with `CMUX_LINUX=1`; one `FeedEnd` exit, wrong-kind replies rejected; store newest first, cap 2000, not persisted; no Ultraplan/Bypass/free-text answers; ctrl+4 = Feed only inside the sidebar.
- M6: macOS cwd/ports via libproc (`cmux-meta/darwin.rs`); no exit watch on macOS (it revokes the slave on shell exit, so EOF suffices); inherited `CMUX_*` removed in `Session::shell_command()`; package installs to `/usr/lib/cmux-linux/` + `/usr/bin` symlinks; licence gate `deny.toml`; scrollback 10 MB/terminal (cmux ships 50 MB, but our history is uncompressed).

## Code map (M1–M7)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`; M6 `darwin` (macOS libproc).
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- M7: `cmux-core::feed` (`FeedStore`, `FeedKind::accepts`, `Decision`); `cmux-ipc::feed` (JSON <-> model, `hook_output`), `cli/hooks.rs` (`feed`, `setup`, `uninstall`); `cmux-config` `Binding.context`; app `window_view/feed_ctl.rs` (waiters, `feed_finish`, timer), `feed.rs` (right sidebar, permission cards, keys), `feed_cards.rs` (plan, question).
- M9: `cmux-core::messages` (`MessageStore`, `render_messages`); `cmux-ipc` `Snapshot::message_target`, `cli/agent.rs`, `hooks message` in `cli/hooks.rs`; app `window_view/message_ctl.rs`.
- M8: `cmux-core::groups` (`Group`, `Row`, `Placement`, `HeaderClick`); `cmux-ipc` `Kind::WorkspaceGroup`, `GroupSnap`, `cli/groups.rs`; app `window_view/groups.rs` (group actions), `menu.rs` (context menu), `group_ctl.rs` (socket), `dock.rs` (right sidebar frame, Dock, `dock.json`); `WorkspaceView::new_dock`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- M9 messages: the hook's output shapes match Claude Code's hooks reference (checked 2026-11-11) but were never run under Claude. No idle wake, no off switch, not persisted.
- M8: nothing in it was clicked or dragged (see Next action b). The Dock is invisible to the socket: `cmux notify`, the Feed hook and OSC notifications don't work from Dock terminals, and CLI commands can't target them. Dock keeps the main area's minimum pane size, so side-by-side splits need a wide sidebar. Groups and Dock are not persisted (M10).
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim runs on Plasma 6.8.2 without errors (KWin offers `ext_background_effect_manager_v1`); that the backdrop is really blurred is not yet confirmed by eye.
- M2–M7 UI has been seen drawn on KDE only through socket-driven screenshots; nothing was clicked, dragged or typed into by hand yet. On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.
- M7 Feed: cards draw; button clicks, feedback field and question picks unverified. The "sidebar wipes the terminal" report is not a bug (readline redraw when the pane is exactly prompt-wide; no OSC 133 shell integration). Real Claude Code run not done.
- M7 parked (details in its journal): one notification per surface (two cards on a surface share it); stdin read has no timeout; `CMUX_LINUX` would leak into cmux.app launched from our terminal; settings key order relies on workspace `preserve_order` feature unification (check packaging); final-review follow-ups not done: bind `CMUX_LINUX` to our socket path; CLI tests for invalid JSON, uninstall refusal, `HOME` fallback, `--settings-file` precedence; fake-error test for the hook.
- M6 parity: panel, 3-tab strip and ring flash were matched from Swift source only, never rendered; owner's cmux.app wasn't a clean reference (non-default config). Remaining gaps (default theme, full-width titlebar band, panel extras, pressed-icon opacity, scroll fades) are in the M6 journal.
- M6 perf on macOS: loaded p95 10.7 ms, CPU 0.1 % after stop; footprint 971 MB at the old 50 MB scrollback, not re-measured at 10 MB. Linux (KDE, release): RSS 483 MB under 19 floods, CPU 0.8 % after stop.

- Agent messages (M9): app-side store, delivered only through Claude Code hooks (`hooks message` on UserPromptSubmit, PostToolUse, Stop); a workspace's agent is the surface that last claimed.
- Scripted UI checks: `debug.shortcut.simulate {combo}` (raw v2 request) presses a chord; needs `CMUX_SOCKET_MODE=allowAll` from outside the app.
- Groups (M8): runs in the workspace list, anchor first, kept by `Workspaces::normalize`; the sidebar draws `rows()`; no empty groups; notification bump stays inside the group; inline group rename; a context menu instead of a palette. Dock (M8): one window-owned `WorkspaceView` in the right sidebar (mode bar Feed/Dock, draggable width), seeded by `~/.config/cmux-linux/dock.json`.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
Plasma ≤6.6 blur (`org_kde_kwin_blur`): only if Task 8 finds no `ext_background_effect_v1`. M10: map Ghostty `scrollback-limit` and drive libghostty idle compression.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. Not in M6 spec (owner may add): `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- Packaging: crate licence texts (`cargo about`) not shipped yet, before AUR publish; `-git` PKGBUILD only, no release tag; makepkg's Ghostty source still clones from GitHub.
- Shell exit test on Linux now uses perl (`setsid` via fifo marker).

## Blockers
None.

## 2026-11-11 — before merging the M6–M9 stack and CSD

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-11
**Current milestone:** M9 agent-coordination on branch `m9-agent-coordination` (stacked on `m8-groups-dock`, which is stacked on `m6-desktop-checks`): all four parts built (agent messages, auto-naming, automations, Claude Teams), PR #11. None of it has run under a real Claude Code yet.
**Next action:** agent: M10 persistence (write its plan first), on a branch stacked on `m9-agent-coordination`. Owner, when convenient: (a) review and merge the stack in order: #9, #10, #11; (b) done 2026-11-11 with Claude Code 2.1.322 (M9 journal): messages, default auto-naming, automations and a two-teammate `cmux claude-teams` run pass, the last only after adding `respawn-pane`; still open: M7 Task 9 Step 2 (a permission prompt in the Feed) and the `Stop` continue path; (c) the mouse and typing checks no script can do: M8's group header click, `+`, chevron, menus, rename field, Delete prompt, drags, sidebar edge drag, Dock "New Terminal"; older: divider/tab/row drag, Feed buttons, clicking a bubble, IME, `CMUX_TRACE_LATENCY=1` (M1 exit 3), blur by eye; (d) `makepkg -si` and a launcher start.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | merged (PR #7); KDE run done by script (journal 2026-11-11): perf pass, UI draws, D-Bus notification OK; owner-only checks left (install with sudo, mouse/typing, blur by eye) |
| M7 feed | merged (PR #8); Feed seen drawn on KDE (permission, question, empty state); buttons unclicked; real Claude Code run (Task 9 Step 2) pending |
| M8 groups-dock | built, PR #10; both exit criteria checked on KDE by script and screenshot; mouse paths unverified |
| M9 agent-coordination | built, PR #11; each exit criterion checked against the app on KDE with hand-run hooks, a fake naming command and a fake `claude`; real Claude run pending |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate; a `macos` CI job builds/tests release.
- Feed (M7): cmux's hook protocol (one `PermissionRequest` hook, v2 `feed.push` held until answered, 115 s wait then `{}` so Claude shows its own prompt); `cmux hooks setup|uninstall` edits Claude's settings (no `claude` wrapper); hook acts only with `CMUX_LINUX=1`; one `FeedEnd` exit, wrong-kind replies rejected; store newest first, cap 2000, not persisted; no Ultraplan/Bypass/free-text answers; ctrl+4 = Feed only inside the sidebar.
- M6: macOS cwd/ports via libproc (`cmux-meta/darwin.rs`); no exit watch on macOS (it revokes the slave on shell exit, so EOF suffices); inherited `CMUX_*` removed in `Session::shell_command()`; package installs to `/usr/lib/cmux-linux/` + `/usr/bin` symlinks; licence gate `deny.toml`; scrollback 10 MB/terminal (cmux ships 50 MB, but our history is uncompressed).

## Code map (M1–M7)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`; M6 `darwin` (macOS libproc).
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- M7: `cmux-core::feed` (`FeedStore`, `FeedKind::accepts`, `Decision`); `cmux-ipc::feed` (JSON <-> model, `hook_output`), `cli/hooks.rs` (`feed`, `setup`, `uninstall`); `cmux-config` `Binding.context`; app `window_view/feed_ctl.rs` (waiters, `feed_finish`, timer), `feed.rs` (right sidebar, permission cards, keys), `feed_cards.rs` (plan, question).
- M9: `cmux-core::messages` (`MessageStore`, `render_messages`); `cmux-ipc` `Snapshot::message_target`, `cli/agent.rs`, `hooks message` in `cli/hooks.rs`; app `window_view/message_ctl.rs`. Auto-naming: `cmux-config::auto_naming`, `cmux-meta::autoname` (+ `cmux_meta::sh`), `Workspaces::set_auto_title`, app `autoname_ctl.rs`. Automations: `cmux-ipc::automation`, `cli/automation.rs`, app `automation_ctl.rs` (`emit`). Claude Teams: `cmux-ipc` `cli/tmux.rs` (`claude-teams`, `__tmux-compat`), socket `surface.close`.
- M8: `cmux-core::groups` (`Group`, `Row`, `Placement`, `HeaderClick`); `cmux-ipc` `Kind::WorkspaceGroup`, `GroupSnap`, `cli/groups.rs`; app `window_view/groups.rs` (group actions), `menu.rs` (context menu), `group_ctl.rs` (socket), `dock.rs` (right sidebar frame, Dock, `dock.json`); `WorkspaceView::new_dock`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- M9: nothing ran under a real Claude Code. Messages: hook output shapes match the hooks reference; no idle wake, no off switch, not persisted. Claude Teams: the tmux commands a team sends were taken from cmux's shim, not observed; an unhandled one fails loudly; splits can fail for lack of room (no column layout). Auto-naming's default `claude -p` line is untested. Automations: a `run` that calls `cmux` can loop at its rate limit.
- M8: nothing in it was clicked or dragged (see Next action b). The Dock is invisible to the socket: `cmux notify`, the Feed hook and OSC notifications don't work from Dock terminals, and CLI commands can't target them. Dock keeps the main area's minimum pane size, so side-by-side splits need a wide sidebar. Groups and Dock are not persisted (M10).
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim runs on Plasma 6.8.2 without errors (KWin offers `ext_background_effect_manager_v1`); that the backdrop is really blurred is not yet confirmed by eye.
- M2–M7 UI has been seen drawn on KDE only through socket-driven screenshots; nothing was clicked, dragged or typed into by hand yet. On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.
- M7 Feed: cards draw; button clicks, feedback field and question picks unverified. The "sidebar wipes the terminal" report is not a bug (readline redraw when the pane is exactly prompt-wide; no OSC 133 shell integration). Real Claude Code run not done.
- M7 parked (details in its journal): one notification per surface (two cards on a surface share it); stdin read has no timeout; `CMUX_LINUX` would leak into cmux.app launched from our terminal; settings key order relies on workspace `preserve_order` feature unification (check packaging); final-review follow-ups not done: bind `CMUX_LINUX` to our socket path; CLI tests for invalid JSON, uninstall refusal, `HOME` fallback, `--settings-file` precedence; fake-error test for the hook.
- M6 parity: panel, 3-tab strip and ring flash were matched from Swift source only, never rendered; owner's cmux.app wasn't a clean reference (non-default config). Remaining gaps (default theme, full-width titlebar band, panel extras, pressed-icon opacity, scroll fades) are in the M6 journal.
- M6 perf on macOS: loaded p95 10.7 ms, CPU 0.1 % after stop; footprint 971 MB at the old 50 MB scrollback, not re-measured at 10 MB. Linux (KDE, release): RSS 483 MB under 19 floods, CPU 0.8 % after stop.

- Agent messages (M9): app-side store, delivered only through Claude Code hooks (`hooks message` on UserPromptSubmit, PostToolUse, Stop); a workspace's agent is the surface that last claimed.
- M9 Parts 2–4: the app, not a hook, runs the naming command (`[auto_naming]`, off by default; user titles win); automations are cmux's `automations.json` with `notify` and `run` only, over six events from one `emit`; Claude Teams maps tmux pane `%N` to `surface:N` through a private `tmux` script.
- Scripted UI checks: `debug.shortcut.simulate {combo}` (raw v2 request) presses a chord; needs `CMUX_SOCKET_MODE=allowAll` from outside the app.
- Groups (M8): runs in the workspace list, anchor first, kept by `Workspaces::normalize`; the sidebar draws `rows()`; no empty groups; notification bump stays inside the group; inline group rename; a context menu instead of a palette. Dock (M8): one window-owned `WorkspaceView` in the right sidebar (mode bar Feed/Dock, draggable width), seeded by `~/.config/cmux-linux/dock.json`.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
Plasma ≤6.6 blur (`org_kde_kwin_blur`): only if Task 8 finds no `ext_background_effect_v1`. M10: map Ghostty `scrollback-limit` and drive libghostty idle compression.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. Not in M6 spec (owner may add): `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- Packaging: crate licence texts (`cargo about`) not shipped yet, before AUR publish; `-git` PKGBUILD only, no release tag; makepkg's Ghostty source still clones from GitHub.
- Shell exit test on Linux now uses perl (`setsid` via fifo marker).

## Blockers
None.

## 2026-11-11 — before M10 start

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-11
**Current milestone:** none open. M6 desktop checks, M8, M9 and the client-side window buttons are merged to `master` (PRs #9–#12, 2026-11-11); their branches are deleted. PR #13 (2026-11-11): a button in the window's top-left corner that reopens a hidden sidebar; window buttons moved top-right; rounded window corners (window is now always Transparent on Linux); an app icon (`packaging/cmux-linux.png`). All seen drawn on KDE; nothing clicked.
**Next action:** agent: M10 persistence (write its plan first), on a branch from `master`. Owner, when convenient: (a) click-test PR #13's reopen button and window buttons; (b) done 2026-11-11 with Claude Code 2.1.322 (M9 journal): messages, default auto-naming, automations and a two-teammate `cmux claude-teams` run pass, the last only after adding `respawn-pane`; still open: M7 Task 9 Step 2 (a permission prompt in the Feed) and the `Stop` continue path; (c) the mouse and typing checks no script can do: M8's group header click, `+`, chevron, menus, rename field, Delete prompt, drags, sidebar edge drag, Dock "New Terminal"; older: divider/tab/row drag, Feed buttons, clicking a bubble, IME, `CMUX_TRACE_LATENCY=1` (M1 exit 3), blur by eye; (d) `makepkg -si` and a launcher start.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | merged (PR #7); KDE run done by script (journal 2026-11-11): perf pass, UI draws, D-Bus notification OK; owner-only checks left (install with sudo, mouse/typing, blur by eye) |
| M7 feed | merged (PR #8); Feed seen drawn on KDE (permission, question, empty state); buttons unclicked; real Claude Code run (Task 9 Step 2) pending |
| M8 groups-dock | merged (PR #10); both exit criteria checked on KDE by script and screenshot; mouse paths unverified |
| M9 agent-coordination | merged (PR #11); each exit criterion checked against the app on KDE with hand-run hooks, a fake naming command and a fake `claude`; real Claude run pending |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate; a `macos` CI job builds/tests release.
- Feed (M7): cmux's hook protocol (one `PermissionRequest` hook, v2 `feed.push` held until answered, 115 s wait then `{}` so Claude shows its own prompt); `cmux hooks setup|uninstall` edits Claude's settings (no `claude` wrapper); hook acts only with `CMUX_LINUX=1`; one `FeedEnd` exit, wrong-kind replies rejected; store newest first, cap 2000, not persisted; no Ultraplan/Bypass/free-text answers; ctrl+4 = Feed only inside the sidebar.
- M6: macOS cwd/ports via libproc (`cmux-meta/darwin.rs`); no exit watch on macOS (it revokes the slave on shell exit, so EOF suffices); inherited `CMUX_*` removed in `Session::shell_command()`; package installs to `/usr/lib/cmux-linux/` + `/usr/bin` symlinks; licence gate `deny.toml`; scrollback 10 MB/terminal (cmux ships 50 MB, but our history is uncompressed).

## Code map (M1–M7)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`; M6 `darwin` (macOS libproc).
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- M7: `cmux-core::feed` (`FeedStore`, `FeedKind::accepts`, `Decision`); `cmux-ipc::feed` (JSON <-> model, `hook_output`), `cli/hooks.rs` (`feed`, `setup`, `uninstall`); `cmux-config` `Binding.context`; app `window_view/feed_ctl.rs` (waiters, `feed_finish`, timer), `feed.rs` (right sidebar, permission cards, keys), `feed_cards.rs` (plan, question).
- M9: `cmux-core::messages` (`MessageStore`, `render_messages`); `cmux-ipc` `Snapshot::message_target`, `cli/agent.rs`, `hooks message` in `cli/hooks.rs`; app `window_view/message_ctl.rs`. Auto-naming: `cmux-config::auto_naming`, `cmux-meta::autoname` (+ `cmux_meta::sh`), `Workspaces::set_auto_title`, app `autoname_ctl.rs`. Automations: `cmux-ipc::automation`, `cli/automation.rs`, app `automation_ctl.rs` (`emit`). Claude Teams: `cmux-ipc` `cli/tmux.rs` (`claude-teams`, `__tmux-compat`), socket `surface.close`.
- M8: `cmux-core::groups` (`Group`, `Row`, `Placement`, `HeaderClick`); `cmux-ipc` `Kind::WorkspaceGroup`, `GroupSnap`, `cli/groups.rs`; app `window_view/groups.rs` (group actions), `menu.rs` (context menu), `group_ctl.rs` (socket), `dock.rs` (right sidebar frame, Dock, `dock.json`); `WorkspaceView::new_dock`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- M9: nothing ran under a real Claude Code. Messages: hook output shapes match the hooks reference; no idle wake, no off switch, not persisted. Claude Teams: the tmux commands a team sends were taken from cmux's shim, not observed; an unhandled one fails loudly; splits can fail for lack of room (no column layout). Auto-naming's default `claude -p` line is untested. Automations: a `run` that calls `cmux` can loop at its rate limit.
- M8: nothing in it was clicked or dragged (see Next action b). The Dock is invisible to the socket: `cmux notify`, the Feed hook and OSC notifications don't work from Dock terminals, and CLI commands can't target them. Dock keeps the main area's minimum pane size, so side-by-side splits need a wide sidebar. Groups and Dock are not persisted (M10).
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim runs on Plasma 6.8.2 without errors (KWin offers `ext_background_effect_manager_v1`); that the backdrop is really blurred is not yet confirmed by eye.
- M2–M7 UI has been seen drawn on KDE only through socket-driven screenshots; nothing was clicked, dragged or typed into by hand yet. On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.
- M7 Feed: cards draw; button clicks, feedback field and question picks unverified. The "sidebar wipes the terminal" report is not a bug (readline redraw when the pane is exactly prompt-wide; no OSC 133 shell integration). Real Claude Code run not done.
- M7 parked (details in its journal): one notification per surface (two cards on a surface share it); stdin read has no timeout; `CMUX_LINUX` would leak into cmux.app launched from our terminal; settings key order relies on workspace `preserve_order` feature unification (check packaging); final-review follow-ups not done: bind `CMUX_LINUX` to our socket path; CLI tests for invalid JSON, uninstall refusal, `HOME` fallback, `--settings-file` precedence; fake-error test for the hook.
- M6 parity: panel, 3-tab strip and ring flash were matched from Swift source only, never rendered; owner's cmux.app wasn't a clean reference (non-default config). Remaining gaps (default theme, full-width titlebar band, panel extras, pressed-icon opacity, scroll fades) are in the M6 journal.
- M6 perf on macOS: loaded p95 10.7 ms, CPU 0.1 % after stop; footprint 971 MB at the old 50 MB scrollback, not re-measured at 10 MB. Linux (KDE, release): RSS 483 MB under 19 floods, CPU 0.8 % after stop.

- Agent messages (M9): app-side store, delivered only through Claude Code hooks (`hooks message` on UserPromptSubmit, PostToolUse, Stop); a workspace's agent is the surface that last claimed.
- M9 Parts 2–4: the app, not a hook, runs the naming command (`[auto_naming]`, off by default; user titles win); automations are cmux's `automations.json` with `notify` and `run` only, over six events from one `emit`; Claude Teams maps tmux pane `%N` to `surface:N` through a private `tmux` script.
- Scripted UI checks: `debug.shortcut.simulate {combo}` (raw v2 request) presses a chord; needs `CMUX_SOCKET_MODE=allowAll` from outside the app.
- Groups (M8): runs in the workspace list, anchor first, kept by `Workspaces::normalize`; the sidebar draws `rows()`; no empty groups; notification bump stays inside the group; inline group rename; a context menu instead of a palette. Dock (M8): one window-owned `WorkspaceView` in the right sidebar (mode bar Feed/Dock, draggable width), seeded by `~/.config/cmux-linux/dock.json`.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
Plasma ≤6.6 blur (`org_kde_kwin_blur`): only if Task 8 finds no `ext_background_effect_v1`. M10: map Ghostty `scrollback-limit` and drive libghostty idle compression.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. Not in M6 spec (owner may add): `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- Packaging: crate licence texts (`cargo about`) not shipped yet, before AUR publish; `-git` PKGBUILD only, no release tag; makepkg's Ghostty source still clones from GitHub.
- Shell exit test on Linux now uses perl (`setsid` via fifo marker).

## Blockers
None.

## 2026-11-11 — M10 plan written

# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-11
**Current milestone:** M10 persistence, branch `m10-persistence` (PR #14). Plan written 2026-11-11 (`docs/milestones/m10-persistence/plan.md`): Part 1 layout restore, Part 2 Claude/custom-agent resume, Part 3 local-tmux. No code yet. Earlier: M6–M9, the client-side window buttons and PR #13 (sidebar reopen button, rounded corners, app icon) are merged to `master`.
**Next action:** agent: M10 Part 1 Task 1 (`cmux-core::session`: `Layout::shape()`/`from_shape()` with tests), after the owner OKs the plan's deviations (auto-resume of interrupted Claude, no scrollback persistence, local-tmux without a registry). Owner, when convenient: (a) click-test PR #13's reopen button and window buttons; (b) done 2026-11-11 with Claude Code 2.1.322 (M9 journal): messages, default auto-naming, automations and a two-teammate `cmux claude-teams` run pass, the last only after adding `respawn-pane`; still open: M7 Task 9 Step 2 (a permission prompt in the Feed) and the `Stop` continue path; (c) the mouse and typing checks no script can do: M8's group header click, `+`, chevron, menus, rename field, Delete prompt, drags, sidebar edge drag, Dock "New Terminal"; older: divider/tab/row drag, Feed buttons, clicking a bubble, IME, `CMUX_TRACE_LATENCY=1` (M1 exit 3), blur by eye; (d) `makepkg -si` and a launcher start.

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
| M5 cli-socket | merged (PR #6); criterion 1 = `cmux notify` hooks (owner's ruling), automated by `hook_commands_work_unchanged`; KDE desktop check pending |
| M6 parity-release | merged (PR #7); KDE run done by script (journal 2026-11-11): perf pass, UI draws, D-Bus notification OK; owner-only checks left (install with sudo, mouse/typing, blur by eye) |
| M7 feed | merged (PR #8); Feed seen drawn on KDE (permission, question, empty state); buttons unclicked; real Claude Code run (Task 9 Step 2) pending |
| M8 groups-dock | merged (PR #10); both exit criteria checked on KDE by script and screenshot; mouse paths unverified |
| M9 agent-coordination | merged (PR #11); each exit criterion checked against the app on KDE with hand-run hooks, a fake naming command and a fake `claude`; real Claude run pending |
| M10 persistence | in progress: plan written, Part 1 next |
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
- Socket (M5): cmux's wire format and access modes; own path `~/.local/state/cmux-linux/cmux.sock`; refs from 1 per run; the `cmux` CLI is a library (`cmux_ipc::cli::run`) tested against the real server.
- Sidebar metadata: `.git/HEAD` + `/proc` polled every 1.5 s, PR via `gh pr list --json` (cmux cadence), all on the background executor.
- Blur: own `ext_background_effect_v1` shim on gpui 0.2.2's surface (no gpui upgrade); whole window, only with Ghostty `background-blur`.
- macOS dev builds: `--features gpui/runtime_shaders` (no Metal Toolchain); Linux CI is the gate; a `macos` CI job builds/tests release.
- Feed (M7): cmux's hook protocol (one `PermissionRequest` hook, v2 `feed.push` held until answered, 115 s wait then `{}` so Claude shows its own prompt); `cmux hooks setup|uninstall` edits Claude's settings (no `claude` wrapper); hook acts only with `CMUX_LINUX=1`; one `FeedEnd` exit, wrong-kind replies rejected; store newest first, cap 2000, not persisted; no Ultraplan/Bypass/free-text answers; ctrl+4 = Feed only inside the sidebar.
- M6: macOS cwd/ports via libproc (`cmux-meta/darwin.rs`); no exit watch on macOS (it revokes the slave on shell exit, so EOF suffices); inherited `CMUX_*` removed in `Session::shell_command()`; package installs to `/usr/lib/cmux-linux/` + `/usr/bin` symlinks; licence gate `deny.toml`; scrollback 10 MB/terminal (cmux ships 50 MB, but our history is uncompressed).

## Code map (M1–M7)
- `cmux-config`: Ghostty config parser/loader.
- `cmux-term`: `Term` (libghostty-vt wrapper, epoch for selection safety), OSC 7/9/99/777 scanner (Ghostty's ConEmu and kitty rules), PTY `Session` (non-blocking drop), `Selection`, paste/focus encoding.
- `cmux-core`: split tree (`tree`: panes, tabs, focus, zoom) and `geometry` (rects, admission, divider drag, resize, equalize). No UI.
- `cmux-config` also: `keybindings` (cmux defaults with Super, `[keybindings]` overrides in `~/.config/cmux-linux/config.toml`).
- `cmux-core` also: `workspaces` (list, selection, rename, reorder); M4 `notifications` (store, read rules).
- `cmux-meta`: `git`, `path`, `ports`, `pr` (`PrPoller`), `collect`; M6 `darwin` (macOS libproc).
- `cmux-ipc` (M5, no gpui): `wire`, `handles`, `json`, `snapshot` (reads + target resolution), `server`, `paths`, `env`, `keys`, `cli/*`; binary `cmux`. App side: `window_view/control.rs`.
- M7: `cmux-core::feed` (`FeedStore`, `FeedKind::accepts`, `Decision`); `cmux-ipc::feed` (JSON <-> model, `hook_output`), `cli/hooks.rs` (`feed`, `setup`, `uninstall`); `cmux-config` `Binding.context`; app `window_view/feed_ctl.rs` (waiters, `feed_finish`, timer), `feed.rs` (right sidebar, permission cards, keys), `feed_cards.rs` (plan, question).
- M9: `cmux-core::messages` (`MessageStore`, `render_messages`); `cmux-ipc` `Snapshot::message_target`, `cli/agent.rs`, `hooks message` in `cli/hooks.rs`; app `window_view/message_ctl.rs`. Auto-naming: `cmux-config::auto_naming`, `cmux-meta::autoname` (+ `cmux_meta::sh`), `Workspaces::set_auto_title`, app `autoname_ctl.rs`. Automations: `cmux-ipc::automation`, `cli/automation.rs`, app `automation_ctl.rs` (`emit`). Claude Teams: `cmux-ipc` `cli/tmux.rs` (`claude-teams`, `__tmux-compat`), socket `surface.close`.
- M8: `cmux-core::groups` (`Group`, `Row`, `Placement`, `HeaderClick`); `cmux-ipc` `Kind::WorkspaceGroup`, `GroupSnap`, `cli/groups.rs`; app `window_view/groups.rs` (group actions), `menu.rs` (context menu), `group_ctl.rs` (socket), `dock.rs` (right sidebar frame, Dock, `dock.json`); `WorkspaceView::new_dock`.
- `cmux-app`: gpui `TerminalView` + `TerminalElement`, keys, colours, style, mouse, IME; M2 adds `actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`; M3 adds `window_view` (root, pollers), `window_view/sidebar`, `window_view/rename`, `blur` (Linux); M4 adds `attention` (ring, flash), `window_view/notify` (rules wiring, jump), `window_view/panel`, `desktop_notify` (zbus, Linux), `activation` + `wayland` (Linux).

## Open risks
- M9: nothing ran under a real Claude Code. Messages: hook output shapes match the hooks reference; no idle wake, no off switch, not persisted. Claude Teams: the tmux commands a team sends were taken from cmux's shim, not observed; an unhandled one fails loudly; splits can fail for lack of room (no column layout). Auto-naming's default `claude -p` line is untested. Automations: a `run` that calls `cmux` can loop at its rate limit.
- M8: nothing in it was clicked or dragged (see Next action b). The Dock is invisible to the socket: `cmux notify`, the Feed hook and OSC notifications don't work from Dock terminals, and CLI commands can't target them. Dock keeps the main area's minimum pane size, so side-by-side splits need a wide sidebar. Groups and Dock are not persisted (M10).
- gpui is pre-1.0 (pinned). No public FontMetrics → `line_gap` 0.
- Blur shim runs on Plasma 6.8.2 without errors (KWin offers `ext_background_effect_manager_v1`); that the backdrop is really blurred is not yet confirmed by eye.
- M2–M7 UI has been seen drawn on KDE only through socket-driven screenshots; nothing was clicked, dragged or typed into by hand yet. On macOS a **debug** build hangs in gpui's `Application::new` (ActionRegistry insert loop); a **release** build (`cargo build --release --features gpui/runtime_shaders`) runs.
- M5 hooks fail silently if a login shell's startup files hide our `cmux` on `PATH`, or if the agent runs outside the app's process tree (tmux) in cmuxOnly mode (README documents both; cmux's shell integration and capability tokens are not ported).
- Real IME (fcitx/ibus) unverified.
- M7 Feed: cards draw; button clicks, feedback field and question picks unverified. The "sidebar wipes the terminal" report is not a bug (readline redraw when the pane is exactly prompt-wide; no OSC 133 shell integration). Real Claude Code run not done.
- M7 parked (details in its journal): one notification per surface (two cards on a surface share it); stdin read has no timeout; `CMUX_LINUX` would leak into cmux.app launched from our terminal; settings key order relies on workspace `preserve_order` feature unification (check packaging); final-review follow-ups not done: bind `CMUX_LINUX` to our socket path; CLI tests for invalid JSON, uninstall refusal, `HOME` fallback, `--settings-file` precedence; fake-error test for the hook.
- M6 parity: panel, 3-tab strip and ring flash were matched from Swift source only, never rendered; owner's cmux.app wasn't a clean reference (non-default config). Remaining gaps (default theme, full-width titlebar band, panel extras, pressed-icon opacity, scroll fades) are in the M6 journal.
- M6 perf on macOS: loaded p95 10.7 ms, CPU 0.1 % after stop; footprint 971 MB at the old 50 MB scrollback, not re-measured at 10 MB. Linux (KDE, release): RSS 483 MB under 19 floods, CPU 0.8 % after stop.

- Agent messages (M9): app-side store, delivered only through Claude Code hooks (`hooks message` on UserPromptSubmit, PostToolUse, Stop); a workspace's agent is the surface that last claimed.
- M9 Parts 2–4: the app, not a hook, runs the naming command (`[auto_naming]`, off by default; user titles win); automations are cmux's `automations.json` with `notify` and `run` only, over six events from one `emit`; Claude Teams maps tmux pane `%N` to `surface:N` through a private `tmux` script.
- Scripted UI checks: `debug.shortcut.simulate {combo}` (raw v2 request) presses a chord; needs `CMUX_SOCKET_MODE=allowAll` from outside the app.
- Groups (M8): runs in the workspace list, anchor first, kept by `Workspaces::normalize`; the sidebar draws `rows()`; no empty groups; notification bump stays inside the group; inline group rename; a context menu instead of a palette. Dock (M8): one window-owned `WorkspaceView` in the right sidebar (mode bar Feed/Dock, draggable width), seeded by `~/.config/cmux-linux/dock.json`.

## Carry to M4+
- M3 parked (fixed in M4 Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
- M3 deferred: keyboard workspace move doesn't scroll into view; drop line 1 px off/unclipped when scrolled; long rename hides caret; `run_with_deadline` drain join unbounded if a grandchild holds the pipe; no git dirty `*`; no persistence (M10); shell-integration kicks (M5).
Plasma ≤6.6 blur (`org_kde_kwin_blur`): only if Task 8 finds no `ext_background_effect_v1`. M10: map Ghostty `scrollback-limit` and drive libghostty idle compression.
- M5 deferred minors: timed-out requests still run later; denial text can become "Socket closed before reply"; CLI panics on `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC=inf` and non-UTF-8 args; no flock against simultaneous starts; hidden new workspace splits measured at full width; no CLI tests for denial/timeout texts. Not in M6 spec (owner may add): `identify`, `rpc`, `list-notifications`, `clear-notifications`, capability tokens, socket settings UI.
- **M1 exit criteria 2–3:** take the screenshot pair and run `CMUX_TRACE_LATENCY=1` next time the user is at the desktop.
- Mouse: touchpad may scroll ~3× Ghostty's rate; no drag threshold, shift-click extend, autoscroll, horizontal wheel; mouse glue untested.
- Render: underline styles simplified; per-cell Font clone in prepaint; blink task always wakes.
- IME: preedit has no background and isn't clamped; `bounds_for_range` approximate.
- PTY: `resize` locks on GUI thread.
- Term: epoch over-invalidates on any ESC[3J; title/bell coalesced per feed.
- Config: quoted `?` path handling; theme files' own `theme`/`config-file` lines ignored.
- CI: fmt `-p` list hand-maintained; clippy excludes vendored crates (`--no-deps`).
- Packaging: crate licence texts (`cargo about`) not shipped yet, before AUR publish; `-git` PKGBUILD only, no release tag; makepkg's Ghostty source still clones from GitHub.
- Shell exit test on Linux now uses perl (`setsid` via fifo marker).

## Blockers
None.
