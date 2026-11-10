# STATUS — permanent handoff

<!-- Max 100 lines (CI-enforced from M0). Before replacing this file, append the
     current version to docs/status-archive.md. -->

**Updated:** 2026-11-10
**Current milestone:** M4 notifications: plan written on branch `m4-notifications` (`docs/milestones/m4-notifications/plan.md`); M2 and M3 desktop exit checks still pending
**Next action:** execute the M4 plan from Task 1 (Linux-only Tasks 7–8 are checked by CI). Separately, at the KDE desktop: M2 plan Task 11, M3 plan Task 12 Steps 1–3 and every unticked "Run it" step; record results in each journal.

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
| M4 notifications | plan written (branch `m4-notifications`); not built |
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
- Notifications (M4 plan): cmux read rules in a pure `cmux-core` store; desktop bubbles via `zbus` (already in the tree), click activates with the notification's xdg_activation token.
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
- M3 parked (in M4 plan Task 4): `WindowView::select` sets no inset, so a split right after selecting a workspace hidden through a sidebar/window resize uses a stale width until the next frame (one line: `set_left_inset` in `select`).
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
