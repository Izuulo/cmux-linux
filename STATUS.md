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
