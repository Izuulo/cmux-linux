# M2 splits-tabs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ] `) syntax for tracking.

**Goal:** Split panes and per-pane tabs that behave like cmux: a binary split tree with draggable 1 px dividers, a 28 px tab strip per pane, geometric focus movement, zoom, and cmux's shortcut table with ⌘ as Super, remappable from `~/.config/cmux-linux/config.toml`.

**Architecture:** `cmux-core` holds the whole model as pure data (Bonsplit's binary tree: panes with ordered surfaces, splits with a first-child ratio) plus every rule that needs maths: split admission, pixel rects, divider drag, keyboard resize, equalize, directional focus. `cmux-config` gains the six Ghostty split/paste keys and a `[keybindings]` table that it turns into gpui keystroke strings. `cmux-app` gets gpui actions bound from that table, a `WorkspaceView` that owns the `Layout` and one `TerminalView` per surface and draws panes, dividers and tab strips as absolutely positioned divs from `Layout::rects`, and `TerminalView` reports exit/title instead of quitting.

**Tech Stack:** Rust 2024 (toolchain 1.99.0), gpui `=0.2.2`, libghostty-vt (vendored), portable-pty 0.9, libc 0.2 (`poll`, `pidfd_open`), toml 0.8.23 (already in `Cargo.lock`), Lucide icons (ISC), Zig 0.16.0 for the Ghostty build.

## Global Constraints

Carried from M0/M1:
- License GPL-3.0-or-later for our crates. Every added dependency, vendored file or asset goes in `NOTICE` with its licence.
- Never copy from cmux's BUSL-1.1 directories.
- Zig must be exactly 0.16.0 (`scripts/install-zig.sh`); every cargo command needs `export PATH="$PWD/.tools/zig:$PATH"` from the repo root first.
- Use `--locked` for build/test/clippy in CI. libc stays pinned at 0.2.189 in `Cargo.lock` (xattr 0.2.3 needs `libc::ENOATTR`): never run a bare `cargo update`; adding a dependency must not move libc (check with `grep -A1 'name = "libc"' Cargo.lock`).
- gpui pinned with `=0.2.2`.
- No personal info (names, emails, home paths with usernames, machine details, the user's config contents) in tracked files.
- `STATUS.md` max 100 lines; append it to `docs/status-archive.md` before replacing it.
- Toolchain pinned by `rust-toolchain.toml` (1.99.0 with clippy and rustfmt).
- Before each commit: `cargo fmt -p <crates you touched>` and `cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings`. Vendored crates are not formatted or linted (we don't edit them).
- `cmux-term` and `cmux-config` never depend on gpui. `cmux-config` does not depend on ghostty-vt. `cmux-core` depends on nothing.
- Linux-only APIs (`write_to_primary`, `read_from_primary`, `pidfd_open`) stay behind `cfg` for the later macOS port.

New for M2:
- Work happens on branch `m2-splits-tabs`; it merges to `master` at the end (Task 12).
- Every commit message ends with a blank line and the attribution lines the executing session is given (at least `Co-Authored-By: <model name> <noreply@anthropic.com>`). The commands below show `-m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"`; use your own model name and add any session line you were given.
- Working in a `git worktree`: `.cargo/config.toml` resolves `ZIG` and `CMUX_GHOSTTY_SRC` relative to the checkout cargo runs in, so a fresh worktree needs `git submodule update --init --recursive` and either `scripts/install-zig.sh` there or `export ZIG=<main checkout>/.tools/zig/zig`.
- gpui code uses only APIs confirmed in `docs/milestones/m2-splits-tabs/research-gpui.md` or the gpui 0.2.2 source (`~/.cargo/registry/src/*/gpui-0.2.2/src/`). All code in this plan was compiled, formatted, clippy-checked and its tests run against gpui 0.2.2 and the vendored ghostty-vt before the plan was written (177 tests across the workspace at the end of Task 10). The gpui UI code (Tasks 6–10) was built and linted but not run on a desktop; Task 11 is that run.
- Behaviour and numbers come from `research-cmux.md` (cited as `rc §n`) and gpui patterns from `research-gpui.md` (`rg §n`). When something is unclear, read the Swift/Bonsplit source named there and match it; record any deviation in the journal.
- UI sizes are cmux's points used as logical pixels (1 pt = 1 px). The terminal font keeps Ghostty's 96 dpi point conversion from M1.
- Screenshots are taken with spectacle and stored outside the repo in `~/Pictures/cmux-m2/`; the journal names them by file name only.

## Decisions made in this plan

| Decision | Why |
|---|---|
| D1 Model in `cmux-core`, pure, modelled on Bonsplit: strictly binary `Node::Pane \| Node::Split{id, orientation, first, second, ratio}`; ratio = first child's fraction, default 0.5, clamp 0.1..0.9; `PaneId`/`SurfaceId`/`SplitId` are `u64` newtypes from one counter; a pane is `Vec<SurfaceId>` + selected index | rc §1. Unit-testable without a window; one place for every rule. |
| D1 ops follow cmux exactly: split = new pane second (right/down), focused, 0.5, clears zoom, admission ≥160 wide / ≥100 tall else equalize the same-orientation run else refuse; close = sibling takes the slot, focus → most recently focused pane (else the absorbing sibling), unrelated close keeps focus; tab rules (after selected, next-else-previous on close, wrap, 9 = last, move without wrap); geometric `adjacentPane` with MRU → overlap → order tie-break; tree-order cycle; zoom no-op with one pane; resize moves the innermost matching divider by 20 px; equalize by span count; divider double-click equalizes its run | rc §1, §2, §3 notes, §5, §6; cmux `CmuxPanes/Geometry/*.swift` and `bonsplit/.../SplitViewController.swift` read while writing Tasks 3–4. |
| Directional focus and pane cycling clear zoom | Bonsplit's behaviour here is UNVERIFIED (rc §2); Ghostty unzooms on any focus change unless `split-preserve-zoom = navigation` (vendor/ghostty `Config.zig` "split-preserve-zoom"), and focusing a hidden pane would be wrong. |
| Directional focus uses normalized unit-square rects (no dividers); pixel layout uses 1 px dividers snapped to device pixels | Bonsplit's `computePaneBounds` is normalized; layout needs real pixels. Both come from the same tree. |
| Pane rects include the 28 px tab strip; admission minimums (160 × 100) include it | cmux's minimum height is max(100, 28 + 3 × 17) (rc §1). |
| D2 `WorkspaceView` owns the `Layout` and `HashMap<SurfaceId, Entity<TerminalView>>`; panes and dividers are absolute divs placed from `Layout::rects`; dividers: 1 px line in a 17 px band (8 each side), `occlude()`, ResizeLeftRight/UpDown cursors, gpui `on_drag`/`on_drag_move` (rg §1.2 route B), double-click = equalize run; terminal mouse handlers return early while `cx.has_active_drag()` | One maths source for layout, focus and drag (rg §2 note). Drag payloads hold ids only, never entities. |
| The container is the window viewport | The workspace fills the window until M3 adds the sidebar; `window.viewport_size()` is exact at render time, so no stale bounds. M3 must pass the real container size. |
| Hidden tabs are not rendered; their terminals keep running | cmux `.keepAllAlive` (rc §1). gpui only paints rendered views, so a hidden tab costs no paint. |
| D3 gpui actions in namespace `cmux` (`actions.rs`), one per cmux action rawValue, bound from cmux-config's table; `[keybindings]` keys are cmux rawValues, values a chord or array in cmux syntax; `""`/`"none"` unbind; unknown action / bad chord warns and keeps that action's defaults; `KeyBinding::load` validates without panicking; bound at startup only | rc §3 "Remapping", rg §1.5. |
| All bindings are contextless; `Workspace`, `Pane` and `Terminal` key contexts exist on the divs for later use | rg §1.5: a contextless binding outranks contextual ones, so mixing schemes surprises; handlers sit on the right divs anyway (rg §1.6). |
| Shift+symbol chords are translated to the US shifted key (`super+shift+]` → `super-}`) | gpui's Linux backend reports Shift+] as key `}` with Shift cleared (`platform/linux/platform.rs` `from_xkb`); a test pins it. `shortcut:` comment — non-US layouts need the real keymap (M6). |
| `selectSurfaceByNumber` takes one chord ending in a digit; its modifiers are bound with each of 1–9 | cmux's numbered-digit family (`KeyboardShortcutSettings.swift` `usesNumberedDigitMatching`). |
| Ctrl+Tab / Ctrl+Shift+Tab are ordinary (remappable) defaults of `nextSurface` / `prevSurface` | cmux hard-wires them (rc §3); remappable is the project rule. |
| `copy` / `paste` are action names of our own (Super+C/V, Ctrl+Shift+C/V); Copy with no selection calls `cx.propagate()` so the key reaches `key_down` (Super: swallowed; Ctrl+Shift+C: sent to the PTY) | cmux has no copy/paste shortcut entries (menu items); Ghostty's `performable:` semantics (STATUS carry list, rg §1.6). |
| Key releases go to the PTY iff the press did, or the key has no binding in the current context (keymap lookup) | The keymap replaces M1's `bound_keys`; a press that a binding let through (Copy) still gets its release (rg §1.7 item 3). |
| D4 `TerminalView` emits `TerminalEvent::{Exited, TitleChanged}`; shell exit closes that tab; closing the last surface quits; no close prompts; window title = active tab's title, tab title = OSC title else "Terminal" | rc §1 close rules, rc §4 title source. rc §1 suggests a replacement terminal; the controller chose quit until M3 adds workspaces. |
| D5 New splits/tabs start in the active terminal's last OSC 7 directory when `split-`/`tab-inherit-working-directory` allow (default true), else `$HOME`; a missing directory falls back to `$HOME` | rc §1 cwd. `CommandBuilder::cwd` on a missing dir would fail the spawn. |
| D6 visuals: strip 28, always shown; tabs content-sized 48..220, padding 6, spacing 0, icon slot 14 (glyph 11.5), gaps 6, title 11 px tail-truncated, close slot 16 (glyph 9); Bonsplit colour formulas from the terminal bg; 1 px right separator per tab; bottom separator except under the selected tab; 1.5 px accent indicator inset 1 px; close button on selected/hovered tab; middle-click closes; double-click strip = new tab, tab = zoom; buttons new terminal / split right / split down (lane 6 + 3 × 22 + 2 × 4 gaps + 8); unfocused strip greyscale; dim overlay `1 − clamp(unfocused-split-opacity, 0.15, 1)` only with >1 pane; divider = `split-divider-color` else separator colour | rc §4, Bonsplit `TabBarMetrics.swift`, `TabBarColors.swift`, cmux `WindowChromeColorResolver.swift`. |
| Accent = Apple systemBlue: `#007AFF` on light bars, `#0A84FF` on dark | cmux uses `controlAccentColor`, whose default is systemBlue; Linux has no portable accent colour. |
| Chrome text uses gpui's default UI font (`.SystemUIFont`, which falls back to Noto Sans on KDE) | cmux uses the system font; no new font to bundle. |
| The close button is always laid out; its glyph is transparent unless the tab is selected or hovered (`group_hover`) | gpui 0.2.2 has no conditional-on-hover child; the slot is inside the tab so it can only be clicked while hovered. |
| Icons: Lucide `terminal`, `square-terminal`, `x`, `columns-2`, `rows-2` as SVG files served by an `AssetSource` | ISC licence; closest shapes to cmux's SF Symbols (`terminal.fill`, `terminal`, `xmark`, `square.split.2x1`, `square.split.1x2`). |
| A click that focuses an unfocused pane is not sent to the PTY unless the app tracks the mouse | cmux `TerminalPointerFocusActivationPolicy` (rc §2). |
| D6 tab drag: gpui `on_drag` per tab with a ghost view; each strip listens to `on_drag_move` and claims the drop when the pointer is inside it; insertion index = first tab whose mid-x is right of the pointer; 2 × 20 accent bar; drop = `Layout::move_surface` (last tab out closes the pane); tab bounds come from `on_children_prepainted` | rc §4 "Drag and drop", rg §1.3. |
| D7 PTY reader `poll`s the master dup, a `UnixStream` wake pair and (Linux) a `pidfd` of the shell; dropping `Session` wakes it; shell exit ends the session after draining buffered output even if a `setsid -f` grandchild holds the slave | rg §3 design. The pidfd was added while writing the plan: a test showed that only a forked `setsid` grandchild keeps EOF away, and the wake fd alone fixes the drop leak but not "shell exited, pane stays". `shortcut:` macOS needs kqueue (M6). |
| D7 `feed` runs under `catch_unwind`; on a panic the poison is cleared, the reader stops and `Exited` is still sent | STATUS M6 carry item; clearing poison keeps the GUI thread's `lock().unwrap()` from panicking before the tab closes. |
| D7 prepaint split at the rg §4 seams (`measure`, `collect_cells` + `push_rect`/`push_run`, `plan_cursor`, `paint_cursor`, `paint_marked`) with no behaviour change, before panes exist | STATUS carry item; merge helpers get unit tests. |
| D7 paste protection: confirm through `window.prompt` (gpui draws its own prompt on Linux) when `clipboard-paste-protection` is on and `ghostty_paste_is_safe` fails, unless bracketed paste is on and the text has no `ESC[201~` | Ghostty `Surface.zig` `completeClipboardPaste`; `clipboard-paste-bracketed-safe` is hard-wired to its default `true`. Applies to clipboard and middle-click PRIMARY pastes, as in Ghostty. |
| D8 cmux-config parses `unfocused-split-opacity` (0.7, clamped 0.15..1), `unfocused-split-fill`, `split-divider-color` (unset; empty resets), `split-inherit-working-directory`, `tab-inherit-working-directory`, `clipboard-paste-protection` (true) | vendor/ghostty `src/config/Config.zig`. |
| `toml` 0.8.23 with `default-features = false, features = ["parse"]`, parsed as `toml::Table` | Already in `Cargo.lock` (cbindgen build dep), so only the `cmux-config` dependency list changes in the lock; no serde derive. |
| A refused split does nothing visible | cmux beeps; gpui 0.2.2 has no beep API. `shortcut:` comment. |
| D9 exit criteria are checked by the user at the desktop against cmux's published screenshots and docs (no Mac at hand) | Task 11. |

## Deferred (with reason)

- Edge-drop-to-split (dragging a tab onto a pane body) and focus-follows-mouse: listed as deferred by the controller; rc §4, rc §2.
- Tab context menu, rename (`renameTab`), pinned tabs, dirty/unread badges, the zoom accessory on the zoomed tab, Ctrl digit hints, tooltips on the strip buttons: not in the M2 spec; tooltips need a tooltip view (gpui `tooltip` takes an `AnyView`).
- `newPaneAutoLayout`, `moveSurfaceToPane*`, `focusPaneByNumber`, workspace actions: out of M2 (controller).
- Live config reload, in-window config warnings: startup only, stderr (M6).
- Multi-stroke chords: gpui holds the first stroke up to 1 s (rg §1.5); cmux's defaults have none.
- Escape to cancel a tab drag, scroll-to-active-tab on overflow, divider grab offset (the divider jumps up to 8 px to the pointer when a drag starts): `shortcut:` comments where relevant.
- `app.equalizeSplitsOnCreate` (default false in cmux), `paneBorderColor` / `activePaneBorderColor`, `split-preserve-zoom`, `clipboard-paste-bracketed-safe`, `surface-tab-bar-font-size`: defaults are hard-wired.
- Beep on refused split / no-op resize: no gpui API.

## File structure

| Path | Responsibility |
|---|---|
| `crates/cmux-term/src/pty.rs` | Reader loop with `poll` (master, wake socket, pidfd), panic guard |
| `crates/cmux-term/src/input.rs` | + `paste_needs_confirmation` |
| `crates/cmux-core/src/lib.rs` | Module list |
| `crates/cmux-core/src/tree.rs` | Ids, `Node`/`Pane`/`Split`, `Layout`: split, tabs, close, focus (directional, cycle), zoom |
| `crates/cmux-core/src/geometry.rs` | `Layout::rects`, `try_split` (admission), `drag_divider`, `resize_pane`, `equalize_all`, `equalize_run` |
| `crates/cmux-config/src/ghostty/mod.rs` | + six Ghostty keys |
| `crates/cmux-config/src/ghostty/load.rs` | + `Env::cmux_config_file` |
| `crates/cmux-config/src/keybindings.rs` | Default table, chord translation, `[keybindings]` parsing |
| `crates/cmux-app/src/actions.rs` | gpui actions, config binding → `KeyBinding` |
| `crates/cmux-app/src/terminal_element.rs` | Prepaint split into helpers |
| `crates/cmux-app/src/terminal_view.rs` | Events, cwd/title accessors, Copy/Paste actions, key-up rule, paste prompt |
| `crates/cmux-app/src/keys.rs` | Binding table removed |
| `crates/cmux-app/src/mouse.rs` | Drag guard, focus-click policy |
| `crates/cmux-app/src/chrome.rs` | Separator/divider/dim colours, tab palette |
| `crates/cmux-app/src/workspace.rs` | `WorkspaceView`: terminals per surface, panes, dividers, actions, window title |
| `crates/cmux-app/src/workspace/tab_strip.rs` | Tab strip, tabs, buttons, tab drag and drop |
| `crates/cmux-app/src/assets.rs` | `AssetSource` for the icons |
| `crates/cmux-app/assets/icons/` | Lucide SVGs and `LICENSE-lucide.txt` |
| `crates/cmux-app/src/main.rs` | Load config + keybindings, bind keys, open the workspace window |

---

### Task 1: PTY reader that can always stop, and a feed panic guard

Covers STATUS "Carry to M2+": the reader blocks until EOF (a `setsid` grandchild keeps a pane alive and leaks the reader, `Term` and writer); `feed` panics kill the reader without `Exited`. Design: rg §3.

**Files:**
- Modify: `crates/cmux-term/src/pty.rs` (replace), `crates/cmux-term/tests/pty.rs` (append)

**Interfaces:**
- Consumes: `Term::feed` (M1).
- Produces: no API change. `Session::spawn` now also opens a pidfd and a wake socket; `Drop for Session` wakes the reader; `PtyEvent::Exited` is sent when the shell exits (Linux), on EOF/EIO, or after a `feed` panic. Private: `fn guarded<T, R>(m: &Mutex<T>, f: impl FnOnce(&mut T) -> R) -> Option<R>`.

- [x] **Step 1: Write the failing tests.** Append to `crates/cmux-term/tests/pty.rs`:

```rust
#[cfg(target_os = "linux")]
#[test]
fn shell_exit_ends_the_session_even_if_a_setsid_grandchild_holds_the_pty() {
    // Before M2 the reader waited for EOF, which never came while `sleep` kept the slave open.
    let t = Instant::now();
    let out = run("setsid -f sleep 5; echo started", None);
    assert!(out.contains("started"), "{out}");
    assert!(
        t.elapsed() < Duration::from_secs(2),
        "took {:?}",
        t.elapsed()
    );
}

#[test]
fn drop_releases_the_reader_even_if_a_setsid_grandchild_holds_the_pty() {
    let token = Arc::new(());
    let held = token.clone();
    let mut cmd = CommandBuilder::new("sh");
    cmd.args(["-c", "setsid -f sleep 5; sleep 30"]);
    let session = Session::spawn(
        cmd,
        80,
        24,
        |_| {},
        move |_| {
            let _keep = &held;
        },
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    drop(session);
    // The reader drops its event closure (and the `held` clone) when it returns.
    let deadline = Instant::now() + Duration::from_secs(2);
    while Arc::strong_count(&token) > 1 {
        assert!(Instant::now() < deadline, "reader thread still running");
        std::thread::sleep(Duration::from_millis(10));
    }
}
```

- [x] **Step 2: Run tests to verify they fail.** Run: `cargo test -p cmux-term --test pty setsid`. Expected: both FAIL after ~2 s and 5 s (`took 5.0…s` and `reader thread still running`). `setsid -f` (util-linux) forks, so `sleep` is a grandchild in its own session holding the slave.

- [x] **Step 3: Implement.** Replace `crates/cmux-term/src/pty.rs` with:

```rust
//! Runs a command on a PTY and pumps bytes between it and a [`Term`].
//!
//! Threads: a reader feeds PTY output into the `Term` (under its mutex) and reports events;
//! a writer owns the PTY writer and drains one channel that carries both user input and the
//! terminal's own replies, so nothing ever writes from inside `Term::feed`.
//!
//! The reader `poll`s the master, a wake socket and (Linux) a pidfd of the shell, so it never
//! blocks forever: dropping the `Session` wakes it, and the shell exiting ends the session even
//! when a `setsid` grandchild still holds the PTY slave open.

use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

pub use portable_pty::CommandBuilder;
use portable_pty::{Child, MasterPty, PtySize, native_pty_system};

use crate::{Term, TermEvent};

#[derive(Debug)]
pub enum PtyEvent {
    /// A chunk of output was parsed; repaint.
    Output(Vec<TermEvent>),
    /// The child closed the PTY (EOF, or EIO on Linux once it exits).
    Exited,
}

pub struct Session {
    pub term: Arc<Mutex<Term>>,
    writer: mpsc::Sender<Vec<u8>>,
    master: Box<dyn MasterPty + Send>,
    child: ChildGuard,
    /// Set on drop so the reader stops reporting events.
    closed: Arc<AtomicBool>,
    /// Dropped with the session; the reader sees its peer hang up and returns.
    _wake: UnixStream,
}

/// Owns the child; dropping it tears the child down without blocking the caller.
struct ChildGuard(Option<Box<dyn Child + Send + Sync>>);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let Some(mut child) = self.0.take() else {
            return;
        };
        #[cfg(unix)]
        let pgid = child.process_id().and_then(|p| i32::try_from(p).ok());
        // Like a terminal closing: hang up the shell's whole process group (the shell is the
        // session leader, so pgid == pid). portable-pty's own `kill` would sleep ~200 ms
        // before escalating, which would stall the GUI thread.
        #[cfg(unix)]
        if let Some(pgid) = pgid {
            // SAFETY: plain signal syscall; a stale pgid at worst gives ESRCH.
            unsafe { libc::killpg(pgid, libc::SIGHUP) };
        }
        // Escalate and reap off-thread so a child that ignores SIGHUP can't block us.
        let _ = thread::Builder::new()
            .name("pty-reaper".into())
            .spawn(move || {
                let deadline = Instant::now() + Duration::from_millis(500);
                while Instant::now() < deadline {
                    if !matches!(child.try_wait(), Ok(None)) {
                        return; // exited and reaped (or unwaitable)
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                #[cfg(unix)]
                if let Some(pgid) = pgid {
                    // SAFETY: as above; the child is unreaped, so the pgid cannot have been reused.
                    unsafe { libc::killpg(pgid, libc::SIGKILL) };
                }
                #[cfg(not(unix))]
                let _ = child.kill();
                let _ = child.wait();
            });
    }
}

impl Session {
    /// The user's login shell (`$SHELL`, else the passwd entry) in `$HOME`.
    pub fn shell_command() -> CommandBuilder {
        let mut cmd = CommandBuilder::new_default_prog();
        // shortcut: xterm-256color, not xterm-ghostty, since that terminfo may be missing; kitty
        // keyboard is negotiated by escape sequence either way. Ship terminfo in M6.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Some(home) = std::env::var_os("HOME") {
            cmd.cwd(home);
        }
        cmd
    }

    /// Spawns `cmd`. `init` runs on the new `Term` before any output is read (set default
    /// colours there so the first OSC 10/11 query already answers). `on_event` runs on the
    /// reader thread. Once the returned `Session` is dropped `on_event` is not called again,
    /// except for at most one event already in flight.
    pub fn spawn(
        cmd: CommandBuilder,
        cols: u16,
        rows: u16,
        init: impl FnOnce(&mut Term),
        mut on_event: impl FnMut(PtyEvent) + Send + 'static,
    ) -> io::Result<Session> {
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = native_pty_system()
            .openpty(size)
            .map_err(io::Error::other)?;
        // Declared first so any `?` below kills and reaps the child.
        let child = ChildGuard(Some(
            pair.slave.spawn_command(cmd).map_err(io::Error::other)?,
        ));
        // Our copy of the slave must close, or the reader never sees EOF when the child exits.
        drop(pair.slave);
        let master = pair.master;
        let mut reader = master.try_clone_reader().map_err(io::Error::other)?;
        let mut pty_writer = master.take_writer().map_err(io::Error::other)?;
        let raw = master
            .as_raw_fd()
            .ok_or_else(|| io::Error::other("PTY master has no fd"))?;
        // SAFETY: `raw` is the master's fd and `master` is alive for this borrow; the clone
        // is an independent fd the reader polls.
        let master_fd = unsafe { BorrowedFd::borrow_raw(raw) }.try_clone_to_owned()?;
        let exit_fd = child
            .0
            .as_ref()
            .and_then(|c| c.process_id())
            .and_then(pidfd);
        let (wake, wake_peer) = UnixStream::pair()?;

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let mut term = Term::new(cols, rows, tx.clone()).map_err(io::Error::other)?;
        init(&mut term);
        let term = Arc::new(Mutex::new(term));

        thread::Builder::new()
            .name("pty-writer".into())
            .spawn(move || {
                for bytes in rx {
                    if pty_writer
                        .write_all(&bytes)
                        .and_then(|()| pty_writer.flush())
                        .is_err()
                    {
                        break;
                    }
                }
            })?;

        let shared = term.clone();
        let closed = Arc::new(AtomicBool::new(false));
        let reader_closed = closed.clone();
        thread::Builder::new()
            .name("pty-reader".into())
            .spawn(move || {
                let mut buf = vec![0u8; 64 * 1024];
                let mut child_gone = false;
                loop {
                    let ready = match wait(&master_fd, &wake_peer, exit_fd.as_ref(), child_gone) {
                        Ok(ready) => ready,
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    };
                    if ready.wake {
                        return; // the Session was dropped
                    }
                    if ready.exited {
                        // Drain what is buffered, then end even if a grandchild keeps the slave.
                        child_gone = true;
                    }
                    if !ready.master {
                        if child_gone {
                            break;
                        }
                        continue;
                    }
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let Some(events) = guarded(&shared, |t| t.feed(&buf[..n])) else {
                                eprintln!("cmux-linux: terminal parser panicked; closing it");
                                break;
                            };
                            if reader_closed.load(Ordering::Acquire) {
                                return;
                            }
                            on_event(PtyEvent::Output(events));
                        }
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                if !reader_closed.load(Ordering::Acquire) {
                    on_event(PtyEvent::Exited);
                }
            })?;

        Ok(Session {
            term,
            writer: tx,
            master,
            child,
            closed,
            _wake: wake,
        })
    }

    /// The child's pid, if the platform reports one.
    pub fn process_id(&self) -> Option<u32> {
        self.child.0.as_ref().and_then(|c| c.process_id())
    }

    /// Queues bytes for the child. Never blocks.
    pub fn write(&self, bytes: Vec<u8>) {
        let _ = self.writer.send(bytes);
    }

    /// Resizes the terminal, then tells the kernel (which sends SIGWINCH).
    pub fn resize(&self, cols: u16, rows: u16, cell_w_px: u32, cell_h_px: u32) -> io::Result<()> {
        self.term
            .lock()
            .unwrap()
            .resize(cols, rows, cell_w_px, cell_h_px)
            .map_err(io::Error::other)?;
        let px = |cells: u16, cell: u32| (u32::from(cells) * cell).min(u32::from(u16::MAX)) as u16;
        let size = PtySize {
            rows,
            cols,
            pixel_width: px(cols, cell_w_px),
            pixel_height: px(rows, cell_h_px),
        };
        self.master.resize(size).map_err(io::Error::other)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Release);
        // Fields drop after this: `_wake` closing wakes the reader, which returns and drops its
        // `Term` clone; `writer` closing then ends the writer thread, so every master fd closes
        // and the kernel hangs up the PTY. `child` SIGHUPs the process group and reaps it.
    }
}

/// Runs `f` on the locked value. A panic inside `f` is caught and reported as `None`, and the
/// poison is cleared so the GUI thread's later `lock().unwrap()` does not panic too.
fn guarded<T, R>(m: &Mutex<T>, f: impl FnOnce(&mut T) -> R) -> Option<R> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        f(&mut m.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
    }));
    m.clear_poison();
    result.ok()
}

struct Ready {
    master: bool,
    wake: bool,
    exited: bool,
}

/// Blocks until the master is readable (or hung up), the wake socket's peer is gone, or the
/// child exited. Once `child_gone`, only checks whether output is still buffered.
fn wait(
    master: &OwnedFd,
    wake: &UnixStream,
    exit: Option<&OwnedFd>,
    child_gone: bool,
) -> io::Result<Ready> {
    let fd = |fd: RawFd| libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // A negative fd is ignored by poll.
    let exit_fd = exit.filter(|_| !child_gone).map_or(-1, AsRawFd::as_raw_fd);
    let mut fds = [fd(master.as_raw_fd()), fd(wake.as_raw_fd()), fd(exit_fd)];
    let timeout = if child_gone { 0 } else { -1 };
    // SAFETY: `fds` is a valid array of 3 pollfds for the duration of the call.
    if unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) } < 0 {
        return Err(io::Error::last_os_error());
    }
    let hit = |p: &libc::pollfd| p.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0;
    Ok(Ready {
        master: hit(&fds[0]),
        wake: hit(&fds[1]),
        exited: hit(&fds[2]),
    })
}

/// A pidfd that becomes readable when `pid` exits.
// shortcut: Linux only (pidfd_open, 5.3+); elsewhere the session ends on PTY EOF as in M1, so a
// setsid grandchild can keep a pane open. Use kqueue EVFILT_PROC in the M6 macOS build.
fn pidfd(pid: u32) -> Option<OwnedFd> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::FromRawFd;
        // SAFETY: plain syscall; on success the kernel returns a new fd that nothing else owns.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0) };
        // SAFETY: `fd` is that new, open fd.
        (fd >= 0).then(|| unsafe { OwnedFd::from_raw_fd(fd as RawFd) })
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_under_the_lock_is_caught_and_unpoisoned() {
        let m = Mutex::new(1);
        assert_eq!(guarded(&m, |v| *v + 1), Some(2));
        let r: Option<()> = guarded(&m, |_| panic!("boom"));
        assert_eq!(r, None);
        assert!(!m.is_poisoned());
        assert_eq!(*m.lock().unwrap(), 1);
    }
}
```

- [x] **Step 4: Run tests to verify they pass.** Run: `cargo test -p cmux-term`. Expected: lib `test result: ok. 35 passed` (includes `a_panic_under_the_lock_is_caught_and_unpoisoned`), `pty` `test result: ok. 9 passed` in well under a second each.

- [x] **Step 5: Format, lint, commit.**

```bash
cargo fmt -p cmux-term
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-term
git commit -m "fix(cmux-term): poll-based PTY reader with wake socket and pidfd; feed panic guard" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Split `TerminalElement::prepaint` (no behaviour change)

Covers STATUS "split prepaint before adding panes" at the seams in rg §4.

**Files:**
- Modify: `crates/cmux-app/src/terminal_element.rs` (replace)

**Interfaces:**
- Consumes: `TermStyle`, `cell_size` (`style.rs`), `Geometry`, `snap_to_device` (`terminal_view.rs`), `cmux_term::{Term, FrameInfo}`, `colors::*` (M1).
- Produces (private to the module): `fn measure(&mut Window, &TermStyle, Bounds<Pixels>) -> Geometry`; `fn push_rect(&mut Vec<Rect>, row: u16, col: u16, span: u16, Hsla)`; `fn push_run(&mut Vec<Run>, Run)`; `fn collect_cells(&mut Term, &FrameInfo, &TermStyle, Option<SelectionRange>) -> Cells`; `fn plan_cursor(&FrameInfo, Option<UnderCursor>, &TermStyle, CursorState) -> (Option<CursorPaint>, Option<Marked>)`; `Frame::paint_cursor`, `Frame::paint_marked`. `TerminalElement::new(Entity<TerminalView>)` unchanged.

- [x] **Step 1: Replace the file.** Replace `crates/cmux-app/src/terminal_element.rs` with (the two new tests are at the bottom):

```rust
//! Paints the terminal grid.
//!
//! Adapted from Zed's terminal element (zed-industries/zed@69e2130,
//! crates/terminal_view/src/terminal_element.rs, GPL-3.0-or-later, Copyright Zed Industries):
//! batching column-contiguous same-style cells into one shaped run (`BatchedTextRun`), merging
//! same-colour backgrounds into rects (`BackgroundRegion`), measuring the grid and resizing the
//! terminal in prepaint, and the paint order. Rewritten for libghostty-vt cells.

use cmux_term::ghostty_vt::{
    CellWidth, CursorShape, SelectionRange, UnderlineStyle as VtUnderline,
};
use cmux_term::selection::range_contains;
use cmux_term::{FrameInfo, Term};
use gpui::{
    App, BorderStyle, Bounds, Element, ElementId, ElementInputHandler, Entity, Font, FontStyle,
    FontWeight, GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels, Point,
    StrikethroughStyle, Style, TextRun, UnderlineStyle, Window, fill, outline, point, px, relative,
    size,
};

use crate::colors::{self, CellColors};
use crate::style::{TermStyle, cell_size};
use crate::terminal_view::{Geometry, TerminalView, snap_to_device};

pub struct TerminalElement {
    view: Entity<TerminalView>,
}

impl TerminalElement {
    pub fn new(view: Entity<TerminalView>) -> Self {
        Self { view }
    }
}

impl IntoElement for TerminalElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

/// A run of same-colour background cells on one row.
struct Rect {
    row: u16,
    col: u16,
    cells: u16,
    color: Hsla,
}

/// Text shaped and painted as one line, starting at a cell.
struct Run {
    row: u16,
    col: u16,
    cells: u16,
    text: String,
    style: TextRun,
    /// Narrow single-codepoint cells only. gpui's `force_width` moves glyph n to n * width, so
    /// wide and combining cells stand alone and get no `force_width` (a combining mark would
    /// otherwise land in the next column); each is painted at its own cell origin instead.
    batchable: bool,
}

struct CursorPaint {
    row: u16,
    col: u16,
    cells: u16,
    shape: CursorShape,
    color: Hsla,
    /// The glyph repainted in the cursor-text colour, for a solid block.
    glyph: Option<Run>,
}

pub struct Frame {
    origin: Point<Pixels>,
    cell_w: f32,
    cell_h: f32,
    font_size: Pixels,
    rects: Vec<Rect>,
    runs: Vec<Run>,
    cursor: Option<CursorPaint>,
    marked: Option<Marked>,
}

fn same_style(a: &TextRun, b: &TextRun) -> bool {
    a.font == b.font
        && a.color == b.color
        && a.underline == b.underline
        && a.strikethrough == b.strikethrough
}

/// gpui's `force_width` snaps glyph n to n * width, which is right for a batch of narrow cells
/// and wrong for a cell holding a base plus combining marks or a wide glyph.
fn run_force_width(batchable: bool, cell_w: f32) -> Option<Pixels> {
    batchable.then(|| px(cell_w))
}

fn face(base: &Font, bold: bool, italic: bool) -> Font {
    let mut font = base.clone();
    if bold {
        font.weight = FontWeight::BOLD;
    }
    if italic {
        font.style = FontStyle::Italic;
    }
    font
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = Frame;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Frame {
        let view = self.view.read(cx);
        let style = view.style.clone();
        let term = view.session.term.clone();
        let focused = view.focus.is_focused(window);
        let blink_on = view.blink_on;
        let marked_text = view.marked_text.clone();

        // Measure the grid; resize the terminal and PTY when it changes (Zed does this in prepaint too).
        let geometry = measure(window, &style, bounds);
        self.view.update(cx, |view, _| view.set_geometry(geometry));

        let mut term = term.lock().unwrap();
        // Resolved once per frame: each `Selection::contains` would be two FFI lookups.
        let selection = self
            .view
            .read(cx)
            .selection
            .as_ref()
            .map(|s| s.range(&term));
        let selection_lost = matches!(selection, Some(None));
        let selection = selection.flatten();
        let mut cells = Cells::default();
        let mut cursor = (None, None);
        let mut cursor_cell = None;
        let mut cursor_blinks = false;
        if let Ok(info) = term.update_frame() {
            cells = collect_cells(&mut term, &info, &style, selection);
            cursor_cell = info.cursor.map(|c| (c.x, c.y));
            cursor_blinks = info.cursor.is_some_and(|c| c.blinking);
            let state = CursorState {
                focused,
                blink_on,
                marked_text,
            };
            cursor = plan_cursor(&info, cells.under_cursor.take(), &style, state);
        }
        drop(term);
        self.view.update(cx, |view, _| {
            if selection_lost {
                // Lost rows: drop it to release its tracked points (selection.rs `range`).
                view.selection = None;
            }
            view.cursor_cell = cursor_cell;
            view.cursor_blinks = cursor_blinks;
        });

        Frame {
            origin: geometry.origin,
            cell_w: geometry.cell_w,
            cell_h: geometry.cell_h,
            font_size: px(style.font_px),
            rects: cells.rects,
            runs: cells.runs,
            cursor: cursor.0,
            marked: cursor.1,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        frame: &mut Frame,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.view.read(cx).focus.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.view.clone()),
            cx,
        );
        let frame = &*frame;
        for r in &frame.rects {
            window.paint_quad(fill(frame.cell_bounds(r.col, r.row, r.cells), r.color));
        }
        for run in &frame.runs {
            frame.paint_run(run, window, cx);
        }
        if let Some(c) = &frame.cursor {
            frame.paint_cursor(c, window, cx);
        }
        if let Some(marked) = &frame.marked {
            let font = self.view.read(cx).style.font.clone();
            frame.paint_marked(marked, font, window, cx);
        }
        self.view.update(cx, |view, _| view.painted());
    }
}

/// Grid geometry for `bounds`: cell size snapped to device pixels, whole cols/rows, and the
/// origin on a device pixel.
fn measure(window: &mut Window, style: &TermStyle, bounds: Bounds<Pixels>) -> Geometry {
    let font_size = px(style.font_px);
    let scale = window.scale_factor();
    let ts = window.text_system();
    let font_id = ts.resolve_font(&style.font);
    let advance = ts
        .advance(font_id, font_size, 'm')
        .map_or(style.font_px * 0.6, |s| f32::from(s.width));
    let ascent = f32::from(ts.ascent(font_id, font_size));
    let descent = f32::from(ts.descent(font_id, font_size));
    // shortcut: gpui 0.2.2 keeps FontMetrics.line_gap pub(crate) (text_system.rs:846, no
    // accessor; research.md §1), so the leading is taken as 0, which matches JetBrains Mono
    // and most monospace fonts. Pass the real value once gpui exposes it.
    let line_gap = 0.0;
    let (cell_w, cell_h) = cell_size(
        advance,
        ascent,
        descent,
        line_gap,
        scale,
        style.cfg.adjust_cell_height,
    );
    let cols = ((f32::from(bounds.size.width) / cell_w) as u16).max(1);
    let rows = ((f32::from(bounds.size.height) / cell_h) as u16).max(1);
    Geometry {
        origin: point(
            px(snap_to_device(f32::from(bounds.origin.x), scale)),
            px(snap_to_device(f32::from(bounds.origin.y), scale)),
        ),
        cell_w,
        cell_h,
        cols,
        rows,
        scale,
    }
}

/// The cell under the cursor, as the cursor needs it: text, face, pre-selection colours, width.
struct UnderCursor {
    text: String,
    font: Font,
    colors: CellColors,
    span: u16,
}

#[derive(Default)]
struct Cells {
    rects: Vec<Rect>,
    runs: Vec<Run>,
    under_cursor: Option<UnderCursor>,
}

/// Extends the last rect when `color` continues it on the same row, else starts a new one.
fn push_rect(rects: &mut Vec<Rect>, row: u16, col: u16, span: u16, color: Hsla) {
    match rects.last_mut() {
        Some(r) if r.row == row && r.col + r.cells == col && r.color == color => r.cells += span,
        _ => rects.push(Rect {
            row,
            col,
            cells: span,
            color,
        }),
    }
}

/// Appends `run` to the last run when both are batchable, adjacent and the same style.
fn push_run(runs: &mut Vec<Run>, run: Run) {
    match runs.last_mut() {
        Some(last)
            if run.batchable
                && last.batchable
                && last.row == run.row
                && last.col + last.cells == run.col
                && same_style(&last.style, &run.style) =>
        {
            last.text.push_str(&run.text);
            last.style.len += run.style.len;
            last.cells += run.cells;
        }
        _ => runs.push(run),
    }
}

/// Walks the viewport once: merged background rects, batched text runs, and the cursor cell.
fn collect_cells(
    term: &mut Term,
    info: &FrameInfo,
    style: &TermStyle,
    selection: Option<SelectionRange>,
) -> Cells {
    let cfg = &style.cfg;
    let (dfg, dbg) = (info.default_fg, info.default_bg);
    let offset = info.scrollbar.map_or(0, |s| s.offset) as u32;
    let bg_alpha = if cfg.background_opacity_cells {
        cfg.background_opacity
    } else {
        1.0
    };
    let at_cursor = info.cursor.map(|c| (c.x, c.y));
    let mut out = Cells::default();
    let _ = term.walk_rows(|row, cells| {
        let row = row as u16;
        for (i, cell) in cells.iter().enumerate() {
            let col = i as u16;
            if matches!(cell.width, CellWidth::SpacerTail | CellWidth::SpacerHead) {
                continue;
            }
            let span = if cell.width == CellWidth::Wide { 2 } else { 1 };
            let pre = colors::resolve(cell, dfg, dbg, cfg.faint_opacity, bg_alpha);
            let abs_row = offset + u32::from(row);
            // A range can end on a wide char's lead or start on its spacer tail.
            let selected = selection.is_some_and(|r| {
                range_contains(r, col, abs_row)
                    || (span == 2 && range_contains(r, col + 1, abs_row))
            });
            let c = if selected {
                colors::selected(
                    pre,
                    dfg,
                    dbg,
                    cfg.selection_foreground,
                    cfg.selection_background,
                )
            } else {
                pre
            };
            let font = face(&style.font, cell.bold, cell.italic);
            if at_cursor == Some((col, row)) {
                // The cursor picks its colours from the cell before selection, as Ghostty does.
                // SGR 8 text stays hidden under a block cursor.
                let text = if c.invisible {
                    String::new()
                } else {
                    cell.text.clone()
                };
                out.under_cursor = Some(UnderCursor {
                    text,
                    font: font.clone(),
                    colors: pre,
                    span,
                });
            }
            if let Some(bg) = c.bg {
                push_rect(&mut out.rects, row, col, span, colors::hsla(bg, c.bg_alpha));
            }
            // Invisible (SGR 8) hides the glyph and its decorations (Ghostty generic.zig).
            if cell.text.is_empty() || c.invisible {
                continue;
            }
            let fg = colors::hsla(c.fg, c.fg_alpha);
            let text_run = TextRun {
                len: cell.text.len(),
                font,
                color: fg,
                background_color: None,
                underline: cell.underline.then_some(UnderlineStyle {
                    thickness: px(1.),
                    color: Some(fg),
                    wavy: cell.underline_style == Some(VtUnderline::Curly),
                }),
                strikethrough: cell.strikethrough.then_some(StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(fg),
                }),
            };
            let batchable = span == 1 && cell.text.chars().count() == 1;
            push_run(
                &mut out.runs,
                Run {
                    row,
                    col,
                    cells: span,
                    text: cell.text.clone(),
                    style: text_run,
                    batchable,
                },
            );
        }
    });
    out
}

/// What the cursor depends on besides the terminal.
struct CursorState {
    focused: bool,
    blink_on: bool,
    marked_text: String,
}

type Marked = (u16, u16, String, Hsla);

/// The cursor to paint (none while blinked off or composing) and the IME preedit, if any.
fn plan_cursor(
    info: &FrameInfo,
    under: Option<UnderCursor>,
    style: &TermStyle,
    state: CursorState,
) -> (Option<CursorPaint>, Option<Marked>) {
    let Some(c) = info.cursor else {
        return (None, None);
    };
    let cfg = &style.cfg;
    let (dfg, dbg) = (info.default_fg, info.default_bg);
    let under = under.unwrap_or_else(|| UnderCursor {
        text: String::new(),
        font: style.font.clone(),
        colors: CellColors {
            fg: dfg,
            bg: None,
            bg_alpha: 1.0,
            fg_alpha: 1.0,
            invisible: false,
        },
        span: 1,
    });
    if !state.marked_text.is_empty() {
        // While composing, the preedit replaces the cursor (Zed does the same).
        let color = colors::hsla(under.colors.fg, 1.0);
        return (None, Some((c.x, c.y, state.marked_text, color)));
    }
    if c.blinking && !state.blink_on && state.focused {
        return (None, None);
    }
    let (fill_rgb, glyph_rgb) = colors::cursor(
        under.colors,
        dfg,
        dbg,
        info.cursor_color,
        cfg.cursor_color,
        cfg.cursor_text,
    );
    // Ghostty draws an unfocused cursor as a hollow block.
    let shape = if state.focused {
        c.shape
    } else {
        CursorShape::BlockHollow
    };
    let glyph = (shape == CursorShape::Block && !under.text.is_empty()).then(|| Run {
        row: c.y,
        col: c.x,
        cells: under.span,
        style: TextRun {
            len: under.text.len(),
            font: under.font,
            color: colors::hsla(glyph_rgb, 1.0),
            background_color: None,
            underline: None,
            strikethrough: None,
        },
        text: under.text,
        batchable: false,
    });
    let paint = CursorPaint {
        row: c.y,
        col: c.x,
        cells: under.span,
        shape,
        color: colors::hsla(fill_rgb, 1.0),
        glyph,
    };
    (Some(paint), None)
}

impl Frame {
    fn cell_bounds(&self, col: u16, row: u16, cells: u16) -> Bounds<Pixels> {
        let origin = point(
            self.origin.x + px(f32::from(col) * self.cell_w),
            self.origin.y + px(f32::from(row) * self.cell_h),
        );
        Bounds::new(
            origin,
            size(px(f32::from(cells) * self.cell_w), px(self.cell_h)),
        )
    }

    fn paint_cursor(&self, c: &CursorPaint, window: &mut Window, cx: &mut App) {
        let cell = self.cell_bounds(c.col, c.row, c.cells);
        let thickness = px(2.);
        match c.shape {
            CursorShape::Block => window.paint_quad(fill(cell, c.color)),
            CursorShape::BlockHollow => {
                window.paint_quad(outline(cell, c.color, BorderStyle::Solid))
            }
            CursorShape::Bar => window.paint_quad(fill(
                Bounds::new(cell.origin, size(thickness, cell.size.height)),
                c.color,
            )),
            CursorShape::Underline => {
                let origin = point(cell.origin.x, cell.origin.y + cell.size.height - thickness);
                window.paint_quad(fill(
                    Bounds::new(origin, size(cell.size.width, thickness)),
                    c.color,
                ));
            }
        }
        if let Some(glyph) = &c.glyph {
            self.paint_run(glyph, window, cx);
        }
    }

    /// The IME preedit at the cursor, underlined in the cell's foreground.
    fn paint_marked(&self, marked: &Marked, font: Font, window: &mut Window, cx: &mut App) {
        let (col, row, text, color) = marked;
        let style = TextRun {
            len: text.len(),
            font,
            color: *color,
            background_color: None,
            underline: Some(UnderlineStyle {
                thickness: px(1.),
                color: Some(*color),
                wavy: false,
            }),
            strikethrough: None,
        };
        let run = Run {
            row: *row,
            col: *col,
            cells: 1,
            text: text.clone(),
            style,
            batchable: false,
        };
        self.paint_run(&run, window, cx);
    }

    fn paint_run(&self, run: &Run, window: &mut Window, cx: &mut App) {
        let origin = self.cell_bounds(run.col, run.row, run.cells).origin;
        let line = window.text_system().shape_line(
            run.text.clone().into(),
            self.font_size,
            std::slice::from_ref(&run.style),
            run_force_width(run.batchable, self.cell_w),
        );
        let _ = line.paint(origin, px(self.cell_h), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_batchable_runs_force_the_cell_width() {
        assert_eq!(run_force_width(true, 9.5), Some(px(9.5)));
        // `e` + U+0301 must keep its accent in the same cell.
        assert_eq!(run_force_width(false, 9.5), None);
    }

    fn run(col: u16, text: &str, batchable: bool) -> Run {
        Run {
            row: 0,
            col,
            cells: 1,
            text: text.into(),
            style: TextRun {
                len: text.len(),
                font: gpui::font("JetBrains Mono"),
                color: gpui::black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            },
            batchable,
        }
    }

    #[test]
    fn adjacent_same_colour_backgrounds_merge() {
        let mut rects = Vec::new();
        push_rect(&mut rects, 0, 0, 1, gpui::red());
        push_rect(&mut rects, 0, 1, 2, gpui::red());
        push_rect(&mut rects, 0, 3, 1, gpui::blue());
        push_rect(&mut rects, 1, 4, 1, gpui::blue());
        let got: Vec<_> = rects.iter().map(|r| (r.row, r.col, r.cells)).collect();
        assert_eq!(got, vec![(0, 0, 3), (0, 3, 1), (1, 4, 1)]);
    }

    #[test]
    fn batchable_neighbours_join_one_run() {
        let mut runs = Vec::new();
        push_run(&mut runs, run(0, "a", true));
        push_run(&mut runs, run(1, "b", true));
        push_run(&mut runs, run(2, "é", false));
        push_run(&mut runs, run(3, "c", true));
        push_run(&mut runs, run(5, "d", true));
        let got: Vec<_> = runs
            .iter()
            .map(|r| (r.col, r.text.as_str(), r.style.len))
            .collect();
        assert_eq!(
            got,
            vec![(0, "ab", 2), (2, "é", 2), (3, "c", 1), (5, "d", 1)]
        );
    }
}
```

The cursor condition changed shape only: M1 painted when `!blinking || blink_on || !focused`; `plan_cursor` returns no cursor when `blinking && !blink_on && focused`.

- [x] **Step 2: Run tests.** Run: `cargo test -p cmux-app --lib terminal_element`. Expected: `test result: ok. 3 passed` (`only_batchable_runs_force_the_cell_width`, `adjacent_same_colour_backgrounds_merge`, `batchable_neighbours_join_one_run`). Then `cargo test -p cmux-app`: `ok. 49 passed`.

- [ ] **Step 3: Run it.** `cargo run -p cmux-app`. Expected: identical to M1 — prompt, colours, blinking cursor, hollow cursor when the window loses focus, `vim` and an IME preedit (if you have one) look the same.

- [x] **Step 4: Format, lint, commit.**

```bash
cargo fmt -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-app/src/terminal_element.rs
git commit -m "refactor(cmux-app): split TerminalElement::prepaint into measure/collect/plan helpers" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: cmux-core split tree: panes, tabs, focus, zoom

rc §1 (model, tab rules, close rules, new split), §2 (focus), §6 (zoom). Bonsplit sources: `Internal/Models/{SplitNode,SplitState,PaneState}.swift`, `Internal/Controllers/SplitViewController.swift` (`closePane` 640-710, `adjacentPane` 797-866).

**Files:**
- Create: `crates/cmux-core/src/tree.rs`
- Modify: `crates/cmux-core/src/lib.rs`

**Interfaces:**
- Produces (crate root re-exports everything in `tree`):
  - `struct PaneId(pub u64)`, `struct SurfaceId(pub u64)`, `struct SplitId(pub u64)` (Copy, Eq, Hash, Ord, Debug)
  - `enum Orientation { Horizontal /* side by side */, Vertical /* stacked */ }`, `enum Direction { Left, Right, Up, Down }` with `Direction::orientation(self) -> Orientation`, `Direction::forward(self) -> bool` (Right/Down)
  - `const DEFAULT_RATIO: f32 = 0.5; MIN_RATIO: f32 = 0.1; MAX_RATIO: f32 = 0.9`
  - `struct Pane { pub id, pub tabs: Vec<SurfaceId>, pub selected: usize }`, `Pane::selected_surface(&self) -> SurfaceId`
  - `struct Split { pub id, pub orientation, pub first: Box<Node>, pub second: Box<Node>, pub ratio: f32 }`, `enum Node { Pane(Pane), Split(Split) }`
  - `struct Rect { pub x, pub y, pub w, pub h: f32 }` with `max_x`, `max_y`, `along(Orientation) -> (start, len)`, `contains(x, y)`
  - `enum Closed { Tab, Pane, LastSurface, NotFound }`
  - `struct Layout` (`pub(crate) root`): `new()`, `root()`, `focused() -> PaneId`, `zoomed() -> Option<PaneId>`, `pane(PaneId) -> Option<&Pane>`, `focused_pane() -> &Pane`, `active_surface() -> SurfaceId`, `panes() -> Vec<&Pane>` (tree order), `pane_count()`, `surfaces()`, `pane_of(SurfaceId) -> Option<PaneId>`, `focus_pane(PaneId) -> bool`, `split(Orientation) -> SurfaceId` (no size check), `new_tab() -> SurfaceId`, `select_surface(SurfaceId) -> bool`, `cycle_tab(forward: bool)`, `select_tab_number(usize) -> bool`, `move_tab(isize) -> bool`, `move_surface(SurfaceId, to: PaneId, index: usize) -> bool`, `close_surface(SurfaceId) -> Closed`, `close_other_tabs() -> Vec<SurfaceId>`, `toggle_zoom() -> bool`, `focus_cycle(forward: bool) -> bool`, `neighbor(Direction) -> Option<PaneId>`, `focus_direction(Direction) -> bool`
  - `pub(crate)` for Task 4: `Node::contains_pane`, field `Layout::root`.

- [x] **Step 1: Write the failing tests.** Change `crates/cmux-core/src/lib.rs` to:

```rust
//! cmux-core: the UI-free model. M2: one workspace's split tree, panes and tabs.

mod tree;

pub use tree::*;
```

Create `crates/cmux-core/src/tree.rs` with its header and tests module only:

```rust
//! The split tree of one workspace: a strictly binary tree of panes, each pane an ordered
//! list of surfaces (tabs). Semantics follow Bonsplit (MIT, github.com/manaflow-ai/bonsplit
//! @58b73fd) as used by cmux; see docs/milestones/m2-splits-tabs/research-cmux.md §1, §2, §6.

#[cfg(test)]
mod tests {
    use super::*;
    use Direction::*;
    use Orientation::*;

    fn tabs(l: &Layout, p: PaneId) -> (Vec<SurfaceId>, usize) {
        let pane = l.pane(p).unwrap();
        (pane.tabs.clone(), pane.selected)
    }

    #[test]
    fn new_layout_has_one_focused_pane_with_one_tab() {
        let l = Layout::new();
        assert_eq!(l.pane_count(), 1);
        assert_eq!(l.focused_pane().tabs.len(), 1);
        assert_eq!(l.active_surface(), l.focused_pane().tabs[0]);
    }

    #[test]
    fn split_puts_the_new_pane_second_focused_at_half() {
        let mut l = Layout::new();
        let old = l.focused();
        let s = l.split(Horizontal);
        let Node::Split(split) = l.root() else {
            panic!("root is a split")
        };
        assert_eq!(split.ratio, 0.5);
        assert_eq!(split.orientation, Horizontal);
        assert!(matches!(&*split.first, Node::Pane(p) if p.id == old));
        assert!(matches!(&*split.second, Node::Pane(p) if p.tabs == vec![s]));
        assert_eq!(l.active_surface(), s);
        assert_ne!(l.focused(), old);
    }

    #[test]
    fn split_clears_zoom() {
        let mut l = Layout::new();
        l.split(Horizontal);
        assert!(l.toggle_zoom());
        l.split(Vertical);
        assert_eq!(l.zoomed(), None);
    }

    #[test]
    fn new_tab_goes_after_the_selected_tab() {
        let mut l = Layout::new();
        let p = l.focused();
        let a = l.active_surface();
        let b = l.new_tab();
        assert!(l.select_surface(a));
        let c = l.new_tab();
        assert_eq!(tabs(&l, p), (vec![a, c, b], 1));
    }

    #[test]
    fn closing_the_selected_tab_selects_the_next_else_the_previous() {
        let mut l = Layout::new();
        let p = l.focused();
        let a = l.active_surface();
        let b = l.new_tab();
        let c = l.new_tab();
        l.select_surface(b);
        assert_eq!(l.close_surface(b), Closed::Tab);
        assert_eq!(tabs(&l, p), (vec![a, c], 1), "c slid into b's index");
        assert_eq!(l.close_surface(c), Closed::Tab);
        assert_eq!(tabs(&l, p), (vec![a], 0), "last removed: previous");
    }

    #[test]
    fn closing_an_unselected_tab_keeps_the_selection() {
        let mut l = Layout::new();
        let p = l.focused();
        let a = l.active_surface();
        let b = l.new_tab();
        let c = l.new_tab();
        assert_eq!(l.close_surface(a), Closed::Tab);
        assert_eq!(tabs(&l, p), (vec![b, c], 1));
    }

    #[test]
    fn tab_cycling_wraps_and_numbers_pick_with_nine_last() {
        let mut l = Layout::new();
        let p = l.focused();
        l.new_tab();
        l.new_tab();
        l.cycle_tab(true);
        assert_eq!(l.pane(p).unwrap().selected, 0);
        l.cycle_tab(false);
        assert_eq!(l.pane(p).unwrap().selected, 2);
        assert!(l.select_tab_number(1));
        assert_eq!(l.pane(p).unwrap().selected, 0);
        assert!(!l.select_tab_number(5));
        assert!(l.select_tab_number(9));
        assert_eq!(l.pane(p).unwrap().selected, 2);
    }

    #[test]
    fn move_tab_does_not_wrap() {
        let mut l = Layout::new();
        let p = l.focused();
        let a = l.active_surface();
        let b = l.new_tab();
        assert!(!l.move_tab(1));
        assert!(l.move_tab(-1));
        assert_eq!(tabs(&l, p), (vec![b, a], 0));
        assert!(!l.move_tab(-1));
    }

    #[test]
    fn closing_the_last_tab_of_a_pane_gives_the_sibling_its_slot() {
        let mut l = Layout::new();
        let left = l.focused();
        let right_tab = l.split(Horizontal);
        let bottom_tab = l.split(Vertical);
        let bottom = l.focused();
        assert_eq!(l.close_surface(bottom_tab), Closed::Pane);
        let Node::Split(s) = l.root() else {
            panic!("still a split")
        };
        assert!(matches!(&*s.first, Node::Pane(p) if p.id == left));
        assert!(matches!(&*s.second, Node::Pane(p) if p.tabs == vec![right_tab]));
        assert!(l.pane(bottom).is_none());
    }

    #[test]
    fn closing_the_focused_pane_focuses_the_most_recent_one() {
        let mut l = Layout::new();
        let a = l.focused();
        l.split(Horizontal);
        let b = l.focused();
        let c_tab = l.split(Horizontal);
        // Focus history: a, b, c, a, c. Closing c must go back to a, not to its sibling b.
        l.focus_pane(a);
        l.select_surface(c_tab);
        assert_eq!(l.close_surface(c_tab), Closed::Pane);
        assert_eq!(l.focused(), a);
        assert!(l.pane(b).is_some());
    }

    #[test]
    fn closing_an_unfocused_pane_keeps_focus_and_closing_the_zoomed_pane_unzooms() {
        let mut l = Layout::new();
        let a = l.focused();
        let a_tab = l.active_surface();
        let b_tab = l.split(Horizontal);
        let b = l.focused();
        l.split(Vertical);
        l.focus_pane(b);
        l.toggle_zoom();
        assert_eq!(l.close_surface(a_tab), Closed::Pane);
        assert_eq!(l.focused(), b);
        assert_eq!(l.zoomed(), Some(b));
        assert!(l.pane(a).is_none());
        assert_eq!(l.close_surface(b_tab), Closed::Pane);
        assert_eq!(l.zoomed(), None);
    }

    #[test]
    fn the_last_surface_is_never_closed() {
        let mut l = Layout::new();
        let s = l.active_surface();
        assert_eq!(l.close_surface(s), Closed::LastSurface);
        assert_eq!(l.surfaces(), vec![s]);
        assert_eq!(l.close_surface(SurfaceId(999)), Closed::NotFound);
    }

    #[test]
    fn zoom_is_a_no_op_with_one_pane_and_toggles_otherwise() {
        let mut l = Layout::new();
        assert!(!l.toggle_zoom());
        l.split(Horizontal);
        assert!(l.toggle_zoom());
        assert_eq!(l.zoomed(), Some(l.focused()));
        assert!(l.toggle_zoom());
        assert_eq!(l.zoomed(), None);
    }

    #[test]
    fn close_other_tabs_keeps_only_the_selected_one() {
        let mut l = Layout::new();
        let a = l.active_surface();
        let b = l.new_tab();
        let c = l.new_tab();
        l.select_surface(b);
        assert_eq!(l.close_other_tabs(), vec![a, c]);
        assert_eq!(l.surfaces(), vec![b]);
    }

    #[test]
    fn move_surface_within_a_pane_uses_insertion_indices() {
        let mut l = Layout::new();
        let p = l.focused();
        let a = l.active_surface();
        let b = l.new_tab();
        let c = l.new_tab();
        assert!(!l.move_surface(a, p, 0), "onto itself");
        assert!(!l.move_surface(a, p, 1), "right after itself");
        assert!(l.move_surface(a, p, 3));
        assert_eq!(tabs(&l, p), (vec![b, c, a], 2));
        assert!(l.move_surface(a, p, 0));
        assert_eq!(tabs(&l, p), (vec![a, b, c], 0));
    }

    #[test]
    fn moving_the_last_tab_out_closes_its_pane_and_focuses_the_target() {
        let mut l = Layout::new();
        let left = l.focused();
        let a = l.active_surface();
        let b = l.split(Horizontal);
        let right = l.focused();
        l.toggle_zoom();
        assert!(l.move_surface(b, left, 0));
        assert_eq!(l.pane_count(), 1);
        assert!(l.pane(right).is_none());
        assert_eq!(tabs(&l, left), (vec![b, a], 0));
        assert_eq!(l.focused(), left);
        assert_eq!(l.zoomed(), None);
        assert!(!l.move_surface(b, left, 1), "right after itself");
    }

    /// a | (b / c): a on the left, b top right, c bottom right.
    fn three() -> (Layout, PaneId, PaneId, PaneId) {
        let mut l = Layout::new();
        let a = l.focused();
        l.split(Horizontal);
        let b = l.focused();
        l.split(Vertical);
        let c = l.focused();
        (l, a, b, c)
    }

    #[test]
    fn directional_focus_is_geometric_and_does_not_wrap() {
        let (mut l, a, b, c) = three();
        assert!(l.focus_direction(Up));
        assert_eq!(l.focused(), b);
        assert!(!l.focus_direction(Up));
        assert!(!l.focus_direction(Right));
        assert!(l.focus_direction(Left));
        assert_eq!(l.focused(), a);
        assert!(!l.focus_direction(Left));
        assert!(!l.focus_direction(Down));
        let _ = c;
    }

    #[test]
    fn directional_focus_prefers_the_most_recently_focused_neighbour() {
        let (mut l, a, b, c) = three();
        // From a, both b and c touch the right edge. c was focused last.
        l.focus_pane(a);
        assert!(l.focus_direction(Right));
        assert_eq!(l.focused(), c);
        l.focus_pane(b);
        l.focus_pane(a);
        assert!(l.focus_direction(Right));
        assert_eq!(l.focused(), b);
    }

    #[test]
    fn never_focused_neighbours_rank_by_overlap_then_order() {
        let mut l = Layout::new();
        let a = l.focused();
        l.split(Horizontal);
        let b = l.focused();
        l.split(Vertical);
        let c = l.focused();
        // Make b larger than c and forget both focus histories.
        if let Node::Split(root) = &mut l.root
            && let Node::Split(right) = &mut *root.second
        {
            right.ratio = 0.7;
        }
        l.focus_seq.retain(|id, _| *id == a);
        l.focus_pane(a);
        assert_eq!(l.neighbor(Right), Some(b));
        let _ = c;
    }

    #[test]
    fn focus_cycles_in_tree_order_and_wraps() {
        let (mut l, a, b, c) = three();
        assert!(l.focus_cycle(true));
        assert_eq!(l.focused(), a);
        assert!(l.focus_cycle(true));
        assert_eq!(l.focused(), b);
        assert!(l.focus_cycle(false));
        assert!(l.focus_cycle(false));
        assert_eq!(l.focused(), c);
    }

    #[test]
    fn focus_moves_clear_zoom() {
        let (mut l, _, _, _) = three();
        l.toggle_zoom();
        assert!(l.focus_direction(Up));
        assert_eq!(l.zoomed(), None);
    }
}
```

- [x] **Step 2: Run tests to verify they fail.** Run: `cargo test -p cmux-core`. Expected: compile errors (`cannot find type Layout`, `cannot find type PaneId`).

- [x] **Step 3: Implement.** Insert above the tests module in `tree.rs`:

```rust
use std::collections::HashMap;

/// A leaf of the split tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneId(pub u64);

/// A tab inside a pane (cmux calls it a surface).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfaceId(pub u64);

/// An inner node of the split tree; identifies a divider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SplitId(pub u64);

/// `Horizontal` = side by side (first left), `Vertical` = stacked (first on top), as in Bonsplit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    /// The orientation of the splits whose divider sits across this direction.
    pub fn orientation(self) -> Orientation {
        match self {
            Direction::Left | Direction::Right => Orientation::Horizontal,
            Direction::Up | Direction::Down => Orientation::Vertical,
        }
    }

    /// Right and Down point from a split's first child to its second.
    pub fn forward(self) -> bool {
        matches!(self, Direction::Right | Direction::Down)
    }
}

/// Bonsplit's `dividerPosition` default and clamp range (`BonsplitConfiguration.swift:121`).
pub const DEFAULT_RATIO: f32 = 0.5;
pub const MIN_RATIO: f32 = 0.1;
pub const MAX_RATIO: f32 = 0.9;

#[derive(Debug, Clone, PartialEq)]
pub struct Pane {
    pub id: PaneId,
    /// Never empty while the pane is in the tree.
    pub tabs: Vec<SurfaceId>,
    /// Index into `tabs`.
    pub selected: usize,
}

impl Pane {
    pub fn selected_surface(&self) -> SurfaceId {
        self.tabs[self.selected]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Split {
    pub id: SplitId,
    pub orientation: Orientation,
    pub first: Box<Node>,
    pub second: Box<Node>,
    /// Fraction of the space (minus the divider) given to `first`, in `MIN_RATIO..=MAX_RATIO`.
    pub ratio: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Pane(Pane),
    Split(Split),
}

/// A rectangle in some coordinate space (unit square for focus, logical pixels for layout).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn max_x(&self) -> f32 {
        self.x + self.w
    }

    pub fn max_y(&self) -> f32 {
        self.y + self.h
    }

    /// (start, length) along the axis an `orientation` split divides.
    pub fn along(&self, orientation: Orientation) -> (f32, f32) {
        match orientation {
            Orientation::Horizontal => (self.x, self.w),
            Orientation::Vertical => (self.y, self.h),
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.max_x() && y >= self.y && y < self.max_y()
    }
}

/// What `close_surface` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Closed {
    /// The tab went away; its pane stays.
    Tab,
    /// It was the pane's last tab, so the pane went away too.
    Pane,
    /// It is the only surface left; nothing changed (the app quits instead, M2).
    LastSurface,
    /// No such surface.
    NotFound,
}

impl Node {
    fn pane(&self, id: PaneId) -> Option<&Pane> {
        match self {
            Node::Pane(p) => (p.id == id).then_some(p),
            Node::Split(s) => s.first.pane(id).or_else(|| s.second.pane(id)),
        }
    }

    fn pane_mut(&mut self, id: PaneId) -> Option<&mut Pane> {
        match self {
            Node::Pane(p) => (p.id == id).then_some(p),
            Node::Split(s) => match s.first.pane_mut(id) {
                Some(p) => Some(p),
                None => s.second.pane_mut(id),
            },
        }
    }

    fn collect_panes<'a>(&'a self, out: &mut Vec<&'a Pane>) {
        match self {
            Node::Pane(p) => out.push(p),
            Node::Split(s) => {
                s.first.collect_panes(out);
                s.second.collect_panes(out);
            }
        }
    }

    fn first_pane(&self) -> PaneId {
        match self {
            Node::Pane(p) => p.id,
            Node::Split(s) => s.first.first_pane(),
        }
    }

    /// Replaces the leaf `id` with `make(leaf)`. Returns false when `id` is not here.
    fn replace_pane(&mut self, id: PaneId, make: &mut dyn FnMut(Node) -> Node) -> bool {
        match self {
            Node::Pane(p) if p.id == id => {
                let leaf = std::mem::replace(self, placeholder());
                *self = make(leaf);
                true
            }
            Node::Pane(_) => false,
            Node::Split(s) => s.first.replace_pane(id, make) || s.second.replace_pane(id, make),
        }
    }

    /// Removes leaf `id`; its sibling takes the parent's slot. Returns the first pane of that
    /// sibling, or `None` when `id` is not below this node (or is this node).
    fn remove_pane(&mut self, id: PaneId) -> Option<PaneId> {
        let Node::Split(s) = self else { return None };
        let survivor = if matches!(&*s.first, Node::Pane(p) if p.id == id) {
            std::mem::replace(&mut *s.second, placeholder())
        } else if matches!(&*s.second, Node::Pane(p) if p.id == id) {
            std::mem::replace(&mut *s.first, placeholder())
        } else {
            return s.first.remove_pane(id).or_else(|| s.second.remove_pane(id));
        };
        let absorbed = survivor.first_pane();
        *self = survivor;
        Some(absorbed)
    }

    pub(crate) fn contains_pane(&self, id: PaneId) -> bool {
        self.pane(id).is_some()
    }

    /// Unit-square bounds of every pane, ignoring dividers (Bonsplit `computePaneBounds`).
    fn normalized(&self, r: Rect, out: &mut Vec<(PaneId, Rect)>) {
        match self {
            Node::Pane(p) => out.push((p.id, r)),
            Node::Split(s) => {
                let (a, b) = match s.orientation {
                    Orientation::Horizontal => {
                        let w = r.w * s.ratio;
                        (
                            Rect { w, ..r },
                            Rect {
                                x: r.x + w,
                                w: r.w - w,
                                ..r
                            },
                        )
                    }
                    Orientation::Vertical => {
                        let h = r.h * s.ratio;
                        (
                            Rect { h, ..r },
                            Rect {
                                y: r.y + h,
                                h: r.h - h,
                                ..r
                            },
                        )
                    }
                };
                s.first.normalized(a, out);
                s.second.normalized(b, out);
            }
        }
    }
}

fn placeholder() -> Node {
    Node::Pane(Pane {
        id: PaneId(0),
        tabs: Vec::new(),
        selected: 0,
    })
}

/// Bonsplit's directional-navigation epsilon (`SplitViewController.swift:823`).
const NAV_EPSILON: f32 = 0.001;

/// One workspace's panes and tabs: the tree, the focused pane, focus recency and zoom.
#[derive(Debug, Clone)]
pub struct Layout {
    pub(crate) root: Node,
    focused: PaneId,
    zoomed: Option<PaneId>,
    /// Monotonic focus sequence per pane; higher = focused more recently. Never focused = absent.
    focus_seq: HashMap<PaneId, u64>,
    seq: u64,
    next_id: u64,
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout {
    /// One pane holding one surface, focused.
    pub fn new() -> Self {
        let mut layout = Layout {
            root: placeholder(),
            focused: PaneId(0),
            zoomed: None,
            focus_seq: HashMap::new(),
            seq: 0,
            next_id: 1,
        };
        let pane = layout.new_pane();
        let id = pane.id;
        layout.root = Node::Pane(pane);
        layout.focus_pane(id);
        layout
    }

    fn next(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn new_pane(&mut self) -> Pane {
        let id = PaneId(self.next());
        let surface = SurfaceId(self.next());
        Pane {
            id,
            tabs: vec![surface],
            selected: 0,
        }
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    pub fn focused(&self) -> PaneId {
        self.focused
    }

    pub fn zoomed(&self) -> Option<PaneId> {
        self.zoomed
    }

    pub fn pane(&self, id: PaneId) -> Option<&Pane> {
        self.root.pane(id)
    }

    pub fn focused_pane(&self) -> &Pane {
        self.root
            .pane(self.focused)
            .expect("focused pane is in the tree")
    }

    /// The selected surface of the focused pane: the one keyboard input goes to.
    pub fn active_surface(&self) -> SurfaceId {
        self.focused_pane().selected_surface()
    }

    /// Panes in depth-first order (first child before second).
    pub fn panes(&self) -> Vec<&Pane> {
        let mut out = Vec::new();
        self.root.collect_panes(&mut out);
        out
    }

    pub fn pane_count(&self) -> usize {
        self.panes().len()
    }

    /// Every surface, pane by pane in tree order.
    pub fn surfaces(&self) -> Vec<SurfaceId> {
        self.panes()
            .iter()
            .flat_map(|p| p.tabs.iter().copied())
            .collect()
    }

    pub fn pane_of(&self, surface: SurfaceId) -> Option<PaneId> {
        self.panes()
            .iter()
            .find(|p| p.tabs.contains(&surface))
            .map(|p| p.id)
    }

    /// Focuses `id` and bumps its recency. False if `id` is not in the tree.
    pub fn focus_pane(&mut self, id: PaneId) -> bool {
        if !self.root.contains_pane(id) {
            return false;
        }
        self.seq += 1;
        self.focus_seq.insert(id, self.seq);
        self.focused = id;
        true
    }

    /// Splits the focused pane: the existing pane stays `first`, a new one-tab pane becomes
    /// `second` (right or below), gets focus, and the ratio is 0.5. Clears zoom. No size check;
    /// the app uses `Layout::try_split`, which checks the minimum pane size first.
    pub fn split(&mut self, orientation: Orientation) -> SurfaceId {
        self.zoomed = None;
        let pane = self.new_pane();
        let (pane_id, surface) = (pane.id, pane.tabs[0]);
        let split_id = SplitId(self.next());
        let mut pane = Some(pane);
        self.root.replace_pane(self.focused, &mut |old| {
            Node::Split(Split {
                id: split_id,
                orientation,
                first: Box::new(old),
                second: Box::new(Node::Pane(pane.take().expect("replaced once"))),
                ratio: DEFAULT_RATIO,
            })
        });
        self.focus_pane(pane_id);
        surface
    }

    /// A new surface in the focused pane, right after the selected tab, and selected.
    pub fn new_tab(&mut self) -> SurfaceId {
        let surface = SurfaceId(self.next());
        let focused = self.focused;
        let pane = self.root.pane_mut(focused).expect("focused pane");
        pane.selected += 1;
        pane.tabs.insert(pane.selected, surface);
        surface
    }

    /// Selects `surface` and focuses its pane (a click on a tab).
    pub fn select_surface(&mut self, surface: SurfaceId) -> bool {
        let Some(pane_id) = self.pane_of(surface) else {
            return false;
        };
        let pane = self.root.pane_mut(pane_id).expect("pane_of");
        pane.selected = pane
            .tabs
            .iter()
            .position(|s| *s == surface)
            .expect("pane_of");
        self.focus_pane(pane_id);
        true
    }

    /// Next (`forward`) or previous tab in the focused pane, wrapping.
    pub fn cycle_tab(&mut self, forward: bool) {
        let focused = self.focused;
        let pane = self.root.pane_mut(focused).expect("focused pane");
        let n = pane.tabs.len();
        pane.selected = if forward {
            (pane.selected + 1) % n
        } else {
            (pane.selected + n - 1) % n
        };
    }

    /// cmux `selectSurfaceByNumber`: 1-8 pick that tab if it exists, 9 always picks the last.
    pub fn select_tab_number(&mut self, number: usize) -> bool {
        let focused = self.focused;
        let pane = self.root.pane_mut(focused).expect("focused pane");
        let index = match number {
            9 => pane.tabs.len() - 1,
            1..=8 if number <= pane.tabs.len() => number - 1,
            _ => return false,
        };
        pane.selected = index;
        true
    }

    /// Moves the focused pane's selected tab one place left (`-1`) or right (`+1`), no wrap.
    pub fn move_tab(&mut self, delta: isize) -> bool {
        let focused = self.focused;
        let pane = self.root.pane_mut(focused).expect("focused pane");
        let to = pane.selected as isize + delta;
        if to < 0 || to as usize >= pane.tabs.len() {
            return false;
        }
        pane.tabs.swap(pane.selected, to as usize);
        pane.selected = to as usize;
        true
    }

    /// Moves `surface` into `to` before tab `index` (an index into `to`'s current tabs; `len`
    /// appends). Dropping a tab on itself or right after itself is a no-op (Bonsplit
    /// `PaneState.swift:89`). The moved tab is selected and its pane focused; a pane left
    /// empty closes. Moving to another pane clears zoom. Returns false when nothing moved.
    pub fn move_surface(&mut self, surface: SurfaceId, to: PaneId, index: usize) -> bool {
        let Some(from) = self.pane_of(surface) else {
            return false;
        };
        if !self.root.contains_pane(to) {
            return false;
        }
        let last_surface = self.pane_count() == 1 && self.focused_pane().tabs.len() == 1;
        let src = self.root.pane_mut(from).expect("pane_of");
        let at = src
            .tabs
            .iter()
            .position(|s| *s == surface)
            .expect("pane_of");
        if from == to {
            if index == at || index == at + 1 {
                return false;
            }
            src.tabs.remove(at);
            let index = if index > at { index - 1 } else { index }.min(src.tabs.len());
            src.tabs.insert(index, surface);
            src.selected = index;
            self.focus_pane(to);
            return true;
        }
        if last_surface {
            return false;
        }
        Self::remove_tab_at(src, at);
        let emptied = src.tabs.is_empty();
        let dst = self.root.pane_mut(to).expect("checked");
        let index = index.min(dst.tabs.len());
        dst.tabs.insert(index, surface);
        dst.selected = index;
        self.zoomed = None;
        if emptied {
            self.root.remove_pane(from);
            self.focus_seq.remove(&from);
        }
        self.focus_pane(to);
        true
    }

    /// Removes tab `at`, keeping the selection on the same surface, or (when the selected tab
    /// goes) on the tab that slid into its index, else the previous one (`PaneState.swift:63-80`).
    fn remove_tab_at(pane: &mut Pane, at: usize) {
        pane.tabs.remove(at);
        if at < pane.selected || pane.selected >= pane.tabs.len() {
            pane.selected = pane.selected.saturating_sub(1);
        }
    }

    /// Closes one surface. Closing a pane's last tab removes the pane; its sibling takes the
    /// space. Focus then goes to the most recently focused remaining pane (else the sibling);
    /// closing an unfocused pane keeps focus. Closing the zoomed pane clears zoom.
    pub fn close_surface(&mut self, surface: SurfaceId) -> Closed {
        let Some(pane_id) = self.pane_of(surface) else {
            return Closed::NotFound;
        };
        let only_pane = self.pane_count() == 1;
        let pane = self.root.pane_mut(pane_id).expect("pane_of");
        if pane.tabs.len() > 1 {
            let at = pane
                .tabs
                .iter()
                .position(|s| *s == surface)
                .expect("pane_of");
            Self::remove_tab_at(pane, at);
            return Closed::Tab;
        }
        if only_pane {
            return Closed::LastSurface;
        }
        let absorbed = self.root.remove_pane(pane_id).expect("not the root");
        self.focus_seq.remove(&pane_id);
        if self.zoomed == Some(pane_id) {
            self.zoomed = None;
        }
        if self.focused == pane_id {
            let next = self
                .focus_seq
                .iter()
                .max_by_key(|(_, seq)| **seq)
                .map_or(absorbed, |(id, _)| *id);
            self.focus_pane(next);
        }
        Closed::Pane
    }

    /// cmux `closeOtherTabsInPane`: closes every tab of the focused pane but the selected one.
    /// Returns the closed surfaces.
    pub fn close_other_tabs(&mut self) -> Vec<SurfaceId> {
        let focused = self.focused;
        let pane = self.root.pane_mut(focused).expect("focused pane");
        let keep = pane.selected_surface();
        let closed = pane.tabs.iter().copied().filter(|s| *s != keep).collect();
        pane.tabs = vec![keep];
        pane.selected = 0;
        closed
    }

    /// Zoom the focused pane, or unzoom. No-op with one pane (Bonsplit matches Ghostty here).
    pub fn toggle_zoom(&mut self) -> bool {
        if self.zoomed.is_some() {
            self.zoomed = None;
            return true;
        }
        if self.pane_count() < 2 {
            return false;
        }
        self.zoomed = Some(self.focused);
        true
    }

    /// Next/previous pane in tree order, wrapping (cmux `focusNextPane`/`focusPreviousPane`).
    /// Clears zoom, since the target may be hidden.
    pub fn focus_cycle(&mut self, forward: bool) -> bool {
        let ids: Vec<PaneId> = self.panes().iter().map(|p| p.id).collect();
        if ids.len() < 2 {
            return false;
        }
        let at = ids
            .iter()
            .position(|id| *id == self.focused)
            .expect("focused");
        let n = ids.len();
        let next = if forward {
            ids[(at + 1) % n]
        } else {
            ids[(at + n - 1) % n]
        };
        self.zoomed = None;
        self.focus_pane(next)
    }

    /// The pane directional navigation from the focused pane reaches (Bonsplit `adjacentPane`):
    /// panes whose facing edge touches the focused pane's edge with positive overlap; the most
    /// recently focused wins, then larger overlap, then earlier layout order. `None` at an edge.
    pub fn neighbor(&self, dir: Direction) -> Option<PaneId> {
        let mut rects = Vec::new();
        let unit = Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        };
        self.root.normalized(unit, &mut rects);
        let cur = rects.iter().find(|(id, _)| *id == self.focused)?.1;
        let recency = |id: &PaneId| self.focus_seq.get(id).copied().unwrap_or(0);
        rects
            .iter()
            .enumerate()
            .filter(|(_, (id, b))| *id != self.focused && b.w > NAV_EPSILON && b.h > NAV_EPSILON)
            .filter_map(|(order, (id, b))| {
                let (gap, overlap) = match dir {
                    Direction::Left => (cur.x - b.max_x(), overlap_y(&cur, b)),
                    Direction::Right => (b.x - cur.max_x(), overlap_y(&cur, b)),
                    Direction::Up => (cur.y - b.max_y(), overlap_x(&cur, b)),
                    Direction::Down => (b.y - cur.max_y(), overlap_x(&cur, b)),
                };
                (gap.abs() <= NAV_EPSILON && overlap > NAV_EPSILON).then_some((*id, overlap, order))
            })
            .max_by(|a, b| {
                recency(&a.0)
                    .cmp(&recency(&b.0))
                    .then_with(|| {
                        if (a.1 - b.1).abs() > NAV_EPSILON {
                            a.1.total_cmp(&b.1)
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    })
                    .then_with(|| b.2.cmp(&a.2))
            })
            .map(|(id, _, _)| id)
    }

    /// Moves focus to the neighbour in `dir`. Clears zoom when it moves.
    // Bonsplit leaves zoom alone here (research-cmux.md §2, UNVERIFIED); Ghostty's goto_split
    // unzooms, and a hidden pane must not get focus, so we unzoom.
    pub fn focus_direction(&mut self, dir: Direction) -> bool {
        let Some(id) = self.neighbor(dir) else {
            return false;
        };
        self.zoomed = None;
        self.focus_pane(id)
    }
}

fn overlap_x(a: &Rect, b: &Rect) -> f32 {
    a.max_x().min(b.max_x()) - a.x.max(b.x)
}

fn overlap_y(a: &Rect, b: &Rect) -> f32 {
    a.max_y().min(b.max_y()) - a.y.max(b.y)
}
```

- [x] **Step 4: Run tests to verify they pass.** Run: `cargo test -p cmux-core`. Expected: `test result: ok. 21 passed`.

- [x] **Step 5: Format, lint, commit.**

```bash
cargo fmt -p cmux-core
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-core
git commit -m "feat(cmux-core): split tree with panes, tabs, focus and zoom" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: cmux-core geometry: rects, admission, divider drag, keyboard resize, equalize

rc §1 "Minimum size admission" and "Equalize", §3 resize notes, §4 "Dividers", §5. cmux sources: `Packages/macOS/CmuxPanes/Sources/CmuxPanes/Geometry/ExternalTreeNode+SplitGeometry.swift`, `ExternalTreeNode+SplitSpace.swift`, `Sources/Workspace+SplitSpace.swift`.

**Files:**
- Create: `crates/cmux-core/src/geometry.rs`
- Modify: `crates/cmux-core/src/lib.rs`

**Interfaces:**
- Consumes: Task 3's `Layout`, `Node`, `Split`, `Rect`, ids, `Direction`, ratio constants.
- Produces:
  - `const DIVIDER: f32 = 1.0; MIN_DRAG_SIZE: f32 = 100.0; MIN_SPLIT_WIDTH: f32 = 160.0; MIN_SPLIT_HEIGHT: f32 = 100.0; RESIZE_STEP: f32 = 20.0`
  - `struct DividerRect { pub split: SplitId, pub orientation: Orientation, pub rect: Rect }`, `struct Rects { pub panes: Vec<(PaneId, Rect)>, pub dividers: Vec<DividerRect> }`, `struct NoSpace`
  - `Layout::rects(&self, width: f32, height: f32, scale: f32) -> Rects` (zoomed pane alone; divider positions snapped to `scale`)
  - `Layout::ratio(&self, SplitId) -> Option<f32>`
  - `Layout::try_split(&mut self, Orientation, width, height, scale) -> Result<SurfaceId, NoSpace>`
  - `Layout::drag_divider(&mut self, SplitId, pos: f32, width, height, scale) -> bool` (`pos` along the split's axis in container coordinates)
  - `Layout::resize_pane(&mut self, Direction, step: f32, width, height, scale) -> bool`
  - `Layout::equalize_all(&mut self)`, `Layout::equalize_run(&mut self, SplitId) -> bool`

- [ ] **Step 1: Write the failing tests.** Change `crates/cmux-core/src/lib.rs` to:

```rust
//! cmux-core: the UI-free model. M2: one workspace's split tree, panes and tabs.

mod geometry;
mod tree;

pub use geometry::*;
pub use tree::*;
```

Create `crates/cmux-core/src/geometry.rs` with its header and tests module only:

```rust
//! Pixel layout of the split tree, divider drag, keyboard resize, equalize and split admission.
//! Numbers and rules follow Bonsplit/cmux (research-cmux.md §1 "Minimum size admission",
//! "Equalize", §3 resize notes, §4 "Dividers", §5); cmux's own versions live in
//! `cmux/Packages/macOS/CmuxPanes/Sources/CmuxPanes/Geometry/` (GPL-3.0-or-later).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{Direction::*, Orientation::*};

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn root_split(l: &Layout) -> &Split {
        let Node::Split(s) = l.root() else {
            panic!("root is a split")
        };
        s
    }

    #[test]
    fn two_panes_side_by_side_with_a_one_pixel_divider() {
        let mut l = Layout::new();
        let a = l.focused();
        l.split(Horizontal);
        let b = l.focused();
        let rects = l.rects(1001.0, 500.0, 1.0);
        assert_eq!(
            rects.panes,
            vec![
                (a, r(0.0, 0.0, 500.0, 500.0)),
                (b, r(501.0, 0.0, 500.0, 500.0))
            ]
        );
        assert_eq!(rects.dividers.len(), 1);
        assert_eq!(rects.dividers[0].rect, r(500.0, 0.0, 1.0, 500.0));
        assert_eq!(rects.dividers[0].orientation, Horizontal);
    }

    #[test]
    fn divider_positions_snap_to_device_pixels() {
        let mut l = Layout::new();
        l.split(Vertical);
        // 799 px of space at 0.5 is 399.5; at scale 1 that rounds to 400, at scale 2 it stays.
        assert_eq!(l.rects(100.0, 800.0, 1.0).dividers[0].rect.y, 400.0);
        assert_eq!(l.rects(100.0, 800.0, 2.0).dividers[0].rect.y, 399.5);
        let panes = l.rects(100.0, 800.0, 1.0).panes;
        assert_eq!(panes[1].1, r(0.0, 401.0, 100.0, 399.0));
    }

    #[test]
    fn a_zoomed_pane_fills_the_container_alone() {
        let mut l = Layout::new();
        l.split(Horizontal);
        let b = l.focused();
        l.toggle_zoom();
        let rects = l.rects(800.0, 600.0, 1.0);
        assert_eq!(rects.panes, vec![(b, r(0.0, 0.0, 800.0, 600.0))]);
        assert!(rects.dividers.is_empty());
    }

    #[test]
    fn drag_follows_the_pointer_but_keeps_100_px_and_the_ratio_range() {
        let mut l = Layout::new();
        l.split(Horizontal);
        let id = root_split(&l).id;
        assert!(l.drag_divider(id, 300.0, 1001.0, 500.0, 1.0));
        assert_eq!(l.ratio(id), Some(0.3));
        l.drag_divider(id, 20.0, 1001.0, 500.0, 1.0);
        assert_eq!(
            l.ratio(id),
            Some(0.1),
            "100 px of 1000 is also the 0.1 floor"
        );
        l.drag_divider(id, 5000.0, 1001.0, 500.0, 1.0);
        assert_eq!(l.ratio(id), Some(0.9));
        l.drag_divider(id, 20.0, 201.0, 500.0, 1.0);
        assert_eq!(l.ratio(id), Some(0.5), "too small for 100 px each: half");
        assert!(!l.drag_divider(id, 20.0, 201.0, 500.0, 1.0), "no change");
    }

    #[test]
    fn keyboard_resize_moves_the_divider_by_side() {
        let mut l = Layout::new();
        let left = l.focused();
        l.split(Horizontal);
        let id = root_split(&l).id;
        l.focus_pane(left);
        assert!(
            !l.resize_pane(Left, RESIZE_STEP, 1000.0, 500.0, 1.0),
            "left pane, left: nothing"
        );
        assert!(
            !l.resize_pane(Up, RESIZE_STEP, 1000.0, 500.0, 1.0),
            "no vertical split"
        );
        assert!(l.resize_pane(Right, RESIZE_STEP, 1000.0, 500.0, 1.0));
        assert!((l.ratio(id).unwrap() - 0.52).abs() < 1e-6);
        l.focus_cycle(true);
        assert!(l.resize_pane(Left, RESIZE_STEP, 1000.0, 500.0, 1.0));
        assert!((l.ratio(id).unwrap() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn keyboard_resize_uses_the_innermost_matching_split_and_stops_at_the_clamp() {
        // a | (b | c), focus b: Right moves the inner divider, Left the outer one.
        let mut l = Layout::new();
        l.split(Horizontal);
        let b = l.focused();
        l.split(Horizontal);
        l.focus_pane(b);
        let outer = root_split(&l).id;
        let Node::Split(inner) = &*root_split(&l).second else {
            panic!()
        };
        let inner = inner.id;
        assert!(l.resize_pane(Right, 50.0, 1001.0, 500.0, 1.0));
        assert_eq!(l.ratio(outer), Some(0.5));
        assert!(l.ratio(inner).unwrap() > 0.5);
        assert!(l.resize_pane(Left, 50.0, 1001.0, 500.0, 1.0));
        assert!(l.ratio(outer).unwrap() < 0.5);
        for _ in 0..100 {
            l.resize_pane(Left, 50.0, 1001.0, 500.0, 1.0);
        }
        assert_eq!(l.ratio(outer), Some(0.1));
        assert!(!l.resize_pane(Left, 50.0, 1001.0, 500.0, 1.0));
    }

    #[test]
    fn equalize_all_counts_leaves_along_each_axis() {
        // a | (b | (c / d)): three columns, the last one split in two rows.
        let mut l = Layout::new();
        l.split(Horizontal);
        l.split(Horizontal);
        l.split(Vertical);
        l.equalize_all();
        let root = root_split(&l);
        assert!((root.ratio - 1.0 / 3.0).abs() < 1e-6);
        let Node::Split(inner) = &*root.second else {
            panic!()
        };
        assert_eq!(inner.ratio, 0.5);
        let Node::Split(rows) = &*inner.second else {
            panic!()
        };
        assert_eq!(rows.ratio, 0.5);
    }

    #[test]
    fn equalize_run_stops_at_a_cross_orientation_split() {
        // a | (b / (c | d)): the c|d run is only that split.
        let mut l = Layout::new();
        l.split(Horizontal);
        l.split(Vertical);
        l.split(Horizontal);
        let root = root_split(&l).id;
        let Node::Split(col) = &*root_split(&l).second else {
            panic!()
        };
        let Node::Split(cd) = &*col.second else {
            panic!()
        };
        let (col, cd) = (col.id, cd.id);
        l.drag_divider(root, 700.0, 1001.0, 800.0, 1.0);
        l.drag_divider(cd, 900.0, 1001.0, 800.0, 1.0);
        l.drag_divider(col, 300.0, 1001.0, 800.0, 1.0);
        let before = (l.ratio(root), l.ratio(col));
        assert!(l.equalize_run(cd));
        assert_eq!(l.ratio(cd), Some(0.5));
        assert_eq!((l.ratio(root), l.ratio(col)), before);
    }

    #[test]
    fn equalize_run_climbs_same_orientation_ancestors() {
        // a | (b | c) with skewed dividers: double-clicking the inner one equalizes both.
        let mut l = Layout::new();
        l.split(Horizontal);
        l.split(Horizontal);
        let outer = root_split(&l).id;
        let Node::Split(inner) = &*root_split(&l).second else {
            panic!()
        };
        let inner = inner.id;
        l.drag_divider(outer, 800.0, 1001.0, 500.0, 1.0);
        assert!(l.equalize_run(inner));
        assert!((l.ratio(outer).unwrap() - 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(l.ratio(inner), Some(0.5));
    }

    #[test]
    fn a_split_that_leaves_less_than_160_px_is_refused() {
        let mut l = Layout::new();
        assert_eq!(l.try_split(Horizontal, 300.0, 500.0, 1.0), Err(NoSpace));
        assert_eq!(l.pane_count(), 1);
        assert!(l.try_split(Horizontal, 400.0, 500.0, 1.0).is_ok());
        assert_eq!(l.try_split(Vertical, 400.0, 150.0, 1.0), Err(NoSpace));
    }

    #[test]
    fn a_tight_split_borrows_room_by_equalizing_its_run() {
        let mut l = Layout::new();
        l.split(Horizontal);
        let outer = root_split(&l).id;
        l.drag_divider(outer, 400.0, 501.0, 500.0, 1.0);
        // b is 100 wide; splitting it fits only once a, b, b' get a third each.
        assert!(l.try_split(Horizontal, 501.0, 500.0, 1.0).is_ok());
        assert!((l.ratio(outer).unwrap() - 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(l.pane_count(), 3);
        // A fourth column would be 124 px: refused.
        assert_eq!(l.try_split(Horizontal, 501.0, 500.0, 1.0), Err(NoSpace));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail.** Run: `cargo test -p cmux-core`. Expected: compile errors (`no method named rects`, `cannot find value RESIZE_STEP`).

- [ ] **Step 3: Implement.** Insert above the tests module in `geometry.rs`:

```rust
use crate::tree::{
    Direction, Layout, MAX_RATIO, MIN_RATIO, Node, Orientation, PaneId, Rect, Split, SplitId,
    SurfaceId,
};

/// Divider thickness in logical pixels (Bonsplit `dividerThickness` default).
pub const DIVIDER: f32 = 1.0;
/// Smallest pane side a divider drag leaves (Bonsplit `minimumPaneWidth/Height`).
pub const MIN_DRAG_SIZE: f32 = 100.0;
/// Smallest pane a new split may create (cmux `Workspace+SplitSpace.swift`): 160 wide, and
/// max(100, tab strip 28 + 3 rows × 17) = 100 tall.
pub const MIN_SPLIT_WIDTH: f32 = 160.0;
pub const MIN_SPLIT_HEIGHT: f32 = 100.0;
/// cmux `app.paneResizeStepPixels` default.
pub const RESIZE_STEP: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DividerRect {
    pub split: SplitId,
    /// The split's orientation: a `Horizontal` split has a vertical divider line.
    pub orientation: Orientation,
    /// The 1 px line.
    pub rect: Rect,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rects {
    pub panes: Vec<(PaneId, Rect)>,
    pub dividers: Vec<DividerRect>,
}

/// A split refused because a pane would end up below the minimum size (cmux beeps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoSpace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Fits,
    FitsAfterEqualizingRun,
    NoSpace,
}

fn snap(v: f32, scale: f32) -> f32 {
    (v * scale).round() / scale
}

fn clamp_ratio(r: f32) -> f32 {
    r.clamp(MIN_RATIO, MAX_RATIO)
}

/// Splits `r` along `o`: `first` gets `ratio` of the space left after the divider, snapped
/// to a device pixel. Returns (first, divider, second).
fn divide(r: Rect, o: Orientation, ratio: f32, scale: f32) -> (Rect, Rect, Rect) {
    let (_, len) = r.along(o);
    let avail = (len - DIVIDER).max(0.0);
    let first = snap(avail * ratio, scale).clamp(0.0, avail);
    let second = avail - first;
    let line = DIVIDER.min(len);
    match o {
        Orientation::Horizontal => (
            Rect { w: first, ..r },
            Rect {
                x: r.x + first,
                w: line,
                ..r
            },
            Rect {
                x: r.x + first + line,
                w: second,
                ..r
            },
        ),
        Orientation::Vertical => (
            Rect { h: first, ..r },
            Rect {
                y: r.y + first,
                h: line,
                ..r
            },
            Rect {
                y: r.y + first + line,
                h: second,
                ..r
            },
        ),
    }
}

impl Node {
    fn layout(&self, r: Rect, scale: f32, out: &mut Rects) {
        match self {
            Node::Pane(p) => out.panes.push((p.id, r)),
            Node::Split(s) => {
                let (a, line, b) = divide(r, s.orientation, s.ratio, scale);
                s.first.layout(a, scale, out);
                out.dividers.push(DividerRect {
                    split: s.id,
                    orientation: s.orientation,
                    rect: line,
                });
                s.second.layout(b, scale, out);
            }
        }
    }

    /// The rect a split occupies (both children and its divider).
    fn split_rect(&self, r: Rect, id: SplitId, scale: f32) -> Option<Rect> {
        let Node::Split(s) = self else { return None };
        if s.id == id {
            return Some(r);
        }
        let (a, _, b) = divide(r, s.orientation, s.ratio, scale);
        s.first
            .split_rect(a, id, scale)
            .or_else(|| s.second.split_rect(b, id, scale))
    }

    fn find_split(&self, id: SplitId) -> Option<&Split> {
        let Node::Split(s) = self else { return None };
        if s.id == id {
            return Some(s);
        }
        s.first.find_split(id).or_else(|| s.second.find_split(id))
    }

    fn find_split_mut(&mut self, id: SplitId) -> Option<&mut Split> {
        let Node::Split(s) = self else { return None };
        if s.id == id {
            return Some(s);
        }
        // Checked first: returning the borrow from a failed `if let` trips the borrow checker.
        if s.first.find_split(id).is_some() {
            return s.first.find_split_mut(id);
        }
        s.second.find_split_mut(id)
    }

    /// Splits from here down to pane `id`'s parent, each with "the pane is in its first child".
    fn path_to_pane(&self, id: PaneId) -> Option<Vec<(SplitId, bool)>> {
        match self {
            Node::Pane(p) => (p.id == id).then(Vec::new),
            Node::Split(s) => {
                let (in_first, rest) = match s.first.path_to_pane(id) {
                    Some(rest) => (true, rest),
                    None => (false, s.second.path_to_pane(id)?),
                };
                let mut path = vec![(s.id, in_first)];
                path.extend(rest);
                Some(path)
            }
        }
    }

    /// Splits from here down to split `id`, inclusive.
    fn path_to_split(&self, id: SplitId) -> Option<Vec<SplitId>> {
        let Node::Split(s) = self else { return None };
        if s.id == id {
            return Some(vec![s.id]);
        }
        let rest = s
            .first
            .path_to_split(id)
            .or_else(|| s.second.path_to_split(id))?;
        let mut path = vec![s.id];
        path.extend(rest);
        Some(path)
    }

    /// Leaves along `o`; a child split of the other orientation counts as one.
    fn spans(&self, o: Orientation) -> usize {
        match self {
            Node::Split(s) if s.orientation == o => s.first.spans(o) + s.second.spans(o),
            _ => 1,
        }
    }

    /// Post-order: every split gets `spans(first) / spans(both)` along its own axis.
    fn equalize_all(&mut self) {
        if let Node::Split(s) = self {
            s.first.equalize_all();
            s.second.equalize_all();
            let (a, b) = (s.first.spans(s.orientation), s.second.spans(s.orientation));
            s.ratio = clamp_ratio(a as f32 / (a + b) as f32);
        }
    }

    /// Like `equalize_all`, but only through splits of orientation `o`.
    fn equalize_run(&mut self, o: Orientation) {
        if let Node::Split(s) = self
            && s.orientation == o
        {
            s.first.equalize_run(o);
            s.second.equalize_run(o);
            let (a, b) = (s.first.spans(o), s.second.spans(o));
            s.ratio = clamp_ratio(a as f32 / (a + b) as f32);
        }
    }

    /// Leaves along `o` counting `splitting` as two (the split about to happen).
    fn prospective_spans(&self, o: Orientation, splitting: PaneId) -> usize {
        match self {
            Node::Pane(p) if p.id == splitting => 2,
            Node::Split(s) if s.orientation == o => {
                s.first.prospective_spans(o, splitting) + s.second.prospective_spans(o, splitting)
            }
            _ => 1,
        }
    }

    /// cmux `fitsAfterEqualizingRun`: whether every slot of the run still gets `min` once the
    /// run is equalized with `splitting` split in two.
    fn fits_after_equalizing(
        &self,
        extent: f32,
        o: Orientation,
        splitting: PaneId,
        min: f32,
    ) -> bool {
        match self {
            Node::Pane(p) if p.id == splitting => (extent - DIVIDER) / 2.0 >= min,
            Node::Pane(_) => extent >= min,
            Node::Split(s) if s.orientation != o => self.required_extent(o, min) <= extent,
            Node::Split(s) => {
                let a = s.first.prospective_spans(o, splitting);
                let b = s.second.prospective_spans(o, splitting);
                let avail = extent - DIVIDER;
                if avail < 0.0 {
                    return false;
                }
                let pos = clamp_ratio(a as f32 / (a + b) as f32);
                s.first
                    .fits_after_equalizing(avail * pos, o, splitting, min)
                    && s.second
                        .fits_after_equalizing(avail * (1.0 - pos), o, splitting, min)
            }
        }
    }

    /// The smallest extent along `o` that keeps every pane here at `min`, at current ratios.
    fn required_extent(&self, o: Orientation, min: f32) -> f32 {
        match self {
            Node::Pane(_) => min,
            Node::Split(s) => {
                let a = s.first.required_extent(o, min);
                let b = s.second.required_extent(o, min);
                if s.orientation != o {
                    return a.max(b);
                }
                let pos = clamp_ratio(s.ratio);
                (a / pos).max(b / (1.0 - pos)) + DIVIDER
            }
        }
    }
}

fn full(width: f32, height: f32) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: width,
        h: height,
    }
}

impl Layout {
    /// Pane and divider rects for a `width` × `height` container (logical pixels; divider
    /// positions snapped to `scale`). A zoomed pane fills the container alone.
    pub fn rects(&self, width: f32, height: f32, scale: f32) -> Rects {
        if let Some(z) = self.zoomed() {
            return Rects {
                panes: vec![(z, full(width, height))],
                dividers: Vec::new(),
            };
        }
        self.unzoomed_rects(width, height, scale)
    }

    fn unzoomed_rects(&self, width: f32, height: f32, scale: f32) -> Rects {
        let mut out = Rects::default();
        self.root.layout(full(width, height), scale, &mut out);
        out
    }

    pub fn ratio(&self, split: SplitId) -> Option<f32> {
        self.root.find_split(split).map(|s| s.ratio)
    }

    /// Splits the focused pane if both halves keep the minimum size, borrowing room by
    /// equalizing the same-orientation run when needed (cmux #15371); else `Err(NoSpace)`.
    pub fn try_split(
        &mut self,
        o: Orientation,
        width: f32,
        height: f32,
        scale: f32,
    ) -> Result<SurfaceId, NoSpace> {
        let verdict = self.split_verdict(o, width, height, scale);
        if verdict == Verdict::NoSpace {
            return Err(NoSpace);
        }
        let surface = self.split(o);
        if verdict == Verdict::FitsAfterEqualizingRun
            && let Some(path) = self.root.path_to_pane(self.focused())
            && let Some((parent, _)) = path.last()
        {
            self.equalize_run(*parent);
        }
        Ok(surface)
    }

    /// cmux `splitSpaceVerdict` for splitting the focused pane along `o`.
    fn split_verdict(&self, o: Orientation, width: f32, height: f32, scale: f32) -> Verdict {
        let pane = self.focused();
        let min = match o {
            Orientation::Horizontal => MIN_SPLIT_WIDTH,
            Orientation::Vertical => MIN_SPLIT_HEIGHT,
        };
        let rects = self.unzoomed_rects(width, height, scale);
        let Some((_, rect)) = rects.panes.iter().find(|(id, _)| *id == pane) else {
            return Verdict::Fits;
        };
        let extent = rect.along(o).1;
        if extent <= 0.0 {
            // Not measured yet: allow rather than refuse on unknown geometry.
            return Verdict::Fits;
        }
        if (extent - DIVIDER) / 2.0 >= min {
            return Verdict::Fits;
        }
        let path = self.root.path_to_pane(pane).unwrap_or_default();
        let same = |id: SplitId| self.root.find_split(id).is_some_and(|s| s.orientation == o);
        let Some(&(parent, _)) = path.last() else {
            return Verdict::NoSpace;
        };
        if !same(parent) {
            return Verdict::NoSpace;
        }
        let mut run_root = parent;
        for &(ancestor, _) in path.iter().rev().skip(1) {
            if !same(ancestor) {
                break;
            }
            run_root = ancestor;
        }
        let root_rect = full(width, height);
        let run_extent = self
            .root
            .split_rect(root_rect, run_root, scale)
            .map_or(0.0, |r| r.along(o).1);
        let run = Node::Split(self.root.find_split(run_root).expect("on the path").clone());
        if run.fits_after_equalizing(run_extent, o, pane, min) {
            Verdict::FitsAfterEqualizingRun
        } else {
            Verdict::NoSpace
        }
    }

    /// Divider drag: puts `split`'s divider under `pos` (container coordinates along the
    /// split's axis), keeping each side at least 100 px (or half the space when smaller) and
    /// the ratio in 0.1..0.9. Returns whether the ratio changed.
    pub fn drag_divider(
        &mut self,
        split: SplitId,
        pos: f32,
        width: f32,
        height: f32,
        scale: f32,
    ) -> bool {
        let Some(rect) = self.root.split_rect(full(width, height), split, scale) else {
            return false;
        };
        let s = self
            .root
            .find_split_mut(split)
            .expect("split_rect found it");
        let (start, len) = rect.along(s.orientation);
        let avail = len - DIVIDER;
        if avail <= 0.0 {
            return false;
        }
        let min_side = MIN_DRAG_SIZE.min(avail / 2.0);
        let first = (pos - start).clamp(min_side, avail - min_side);
        let ratio = clamp_ratio(first / avail);
        let changed = ratio != s.ratio;
        s.ratio = ratio;
        changed
    }

    /// cmux keyboard resize (`resizeDividerAdjustment`): moves the divider of the innermost
    /// enclosing split of `dir`'s orientation that has the focused pane on the matching side
    /// (first child for Right/Down, second for Left/Up) by `step` px, clamped to 0.1..0.9.
    /// Returns false when no divider moves (cmux beeps).
    pub fn resize_pane(
        &mut self,
        dir: Direction,
        step: f32,
        width: f32,
        height: f32,
        scale: f32,
    ) -> bool {
        let o = dir.orientation();
        let Some(path) = self.root.path_to_pane(self.focused()) else {
            return false;
        };
        let Some(&(split, _)) = path.iter().rev().find(|(id, in_first)| {
            *in_first == dir.forward()
                && self
                    .root
                    .find_split(*id)
                    .is_some_and(|s| s.orientation == o)
        }) else {
            return false;
        };
        let axis = self
            .root
            .split_rect(full(width, height), split, scale)
            .map_or(1.0, |r| r.along(o).1.max(1.0));
        let s = self.root.find_split_mut(split).expect("on the path");
        let sign = if dir.forward() { 1.0 } else { -1.0 };
        let ratio = clamp_ratio(s.ratio + sign * step / axis);
        let changed = ratio != s.ratio;
        s.ratio = ratio;
        changed
    }

    /// cmux `equalizeSplits`: every split, by leaf count along its axis.
    pub fn equalize_all(&mut self) {
        self.root.equalize_all();
    }

    /// Divider double-click: equalizes the run of same-orientation splits containing `split`
    /// (up through same-orientation ancestors, down through same-orientation descendants).
    pub fn equalize_run(&mut self, split: SplitId) -> bool {
        let Some(path) = self.root.path_to_split(split) else {
            return false;
        };
        let o = self
            .root
            .find_split(split)
            .expect("on the path")
            .orientation;
        let mut run_root = split;
        for &ancestor in path.iter().rev().skip(1) {
            if self
                .root
                .find_split(ancestor)
                .is_none_or(|s| s.orientation != o)
            {
                break;
            }
            run_root = ancestor;
        }
        let Some(root) = self.root.find_split_mut(run_root) else {
            return false;
        };
        let mut node = Node::Split(root.clone());
        node.equalize_run(o);
        let Node::Split(done) = node else {
            unreachable!()
        };
        *root = done;
        true
    }
}
```

- [ ] **Step 4: Run tests to verify they pass.** Run: `cargo test -p cmux-core`. Expected: `test result: ok. 32 passed`.

- [ ] **Step 5: Format, lint, commit.**

```bash
cargo fmt -p cmux-core
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-core
git commit -m "feat(cmux-core): layout rects, split admission, divider drag, keyboard resize, equalize" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: cmux-config: Ghostty split keys and the keybinding table

D3, D8. rc §3 (table and remapping syntax); cmux `Sources/KeyboardShortcutSettings.swift` (`defaultShortcut` 430-700, `parseConfig`/`parseConfigKeyToken` 2417-2548); vendor/ghostty `src/config/Config.zig` (1058-1082, 2045-2056, 2396-2399).

**Files:**
- Create: `crates/cmux-config/src/keybindings.rs`
- Modify: `Cargo.toml`, `crates/cmux-config/Cargo.toml`, `crates/cmux-config/src/lib.rs`, `crates/cmux-config/src/ghostty/mod.rs`, `crates/cmux-config/src/ghostty/load.rs`, `NOTICE`, `Cargo.lock`

**Interfaces:**
- Produces:
  - `Config` fields: `unfocused_split_opacity: f32` (0.7, clamped 0.15..=1), `unfocused_split_fill: Option<Rgb>`, `split_divider_color: Option<Rgb>`, `split_inherit_working_directory: bool`, `tab_inherit_working_directory: bool`, `clipboard_paste_protection: bool` (all `true` by default)
  - `Env::cmux_config_file(&self) -> PathBuf` (`$XDG_CONFIG_HOME/cmux-linux/config.toml`)
  - module `cmux_config::keybindings`: `struct Binding { pub action: &'static str, pub keystroke: String, pub number: Option<u8> }`, `struct Keybindings { pub bindings: Vec<Binding>, pub warnings: Vec<String> }`, `const DEFAULTS: &[(&str, &[&str])]`, `fn chord_to_gpui(&str) -> Result<String, String>`, `fn keybindings(user: Option<&str>, origin: &str) -> Keybindings`, `fn load(&Path) -> Keybindings`

- [ ] **Step 1: Add the dependency.** In the root `Cargo.toml` `[workspace.dependencies]`, add after `futures = "0.3"`:

```toml
# Already in Cargo.lock (cbindgen); `parse` only, no serde derive.
toml = { version = "0.8.23", default-features = false, features = ["parse"] }
```

In `crates/cmux-config/Cargo.toml` `[dependencies]` add `toml.workspace = true`. Change `crates/cmux-config/src/lib.rs` to:

```rust
//! cmux-config: Ghostty config (M1) and keybindings (M2).

pub mod ghostty;
pub mod keybindings;
```

In `NOTICE`, insert before the "Further components" paragraph:

```text
toml (crates.io 0.8, https://github.com/toml-rs/toml), used by cmux-config, with its
  toml_edit, toml_datetime, toml_write, serde_spanned and winnow dependencies
  MIT OR Apache-2.0 (winnow: MIT).

```

Run once without `--locked` so cargo records the dependency: `cargo build -p cmux-config`. Then `git diff Cargo.lock`. Expected: only `dependencies = [ "toml 0.8.23" ]` added under `cmux-config`; `grep -A1 'name = "libc"' Cargo.lock` still says `0.2.189`.

- [ ] **Step 2: Write the failing tests.** Create `crates/cmux-config/src/keybindings.rs` with its header and tests module only:

```rust
//! Key bindings: cmux's default shortcut table with ⌘ as Super, overridable from the
//! `[keybindings]` table of `~/.config/cmux-linux/config.toml`.
//!
//! Keys are cmux action names (`KeyboardShortcutSettings.swift` `Action` rawValues), values a
//! chord or a list of chords in cmux's syntax (`KeyboardShortcutSettings.swift`
//! `parseConfig`): modifiers `cmd`/`command`/`super`, `shift`, `opt`/`option`/`alt`,
//! `ctrl`/`control`, then one key, joined by `+`. `""` or `"none"` unbinds. Output is gpui
//! keystroke syntax (`super-shift-d`); the app validates it with gpui's own parser.

#[cfg(test)]
mod tests {
    use super::*;

    fn keys_for(k: &Keybindings, action: &str) -> Vec<String> {
        k.bindings
            .iter()
            .filter(|b| b.action == action)
            .map(|b| b.keystroke.clone())
            .collect()
    }

    #[test]
    fn chords_translate_to_gpui_keystrokes() {
        assert_eq!(chord_to_gpui("super+d").unwrap(), "super-d");
        assert_eq!(chord_to_gpui("cmd+shift+d").unwrap(), "shift-super-d");
        assert_eq!(
            chord_to_gpui("super+shift+return").unwrap(),
            "shift-super-enter"
        );
        assert_eq!(chord_to_gpui("super+opt+left").unwrap(), "alt-super-left");
        assert_eq!(chord_to_gpui("ctrl+shift+tab").unwrap(), "ctrl-shift-tab");
        assert_eq!(chord_to_gpui("Ctrl+Shift+H").unwrap(), "ctrl-shift-h");
        assert_eq!(chord_to_gpui("ctrl+F5").unwrap(), "ctrl-f5");
        assert_eq!(chord_to_gpui("super+comma").unwrap(), "super-,");
    }

    #[test]
    fn shifted_symbols_become_the_shifted_key() {
        assert_eq!(chord_to_gpui("super+shift+]").unwrap(), "super-}");
        assert_eq!(chord_to_gpui("super+shift+opt+[").unwrap(), "alt-super-{");
        assert_eq!(chord_to_gpui("super+ctrl+shift+=").unwrap(), "ctrl-super-+");
        assert_eq!(chord_to_gpui("shift+plus").unwrap(), "+");
    }

    #[test]
    fn bad_chords_are_errors() {
        assert!(chord_to_gpui("hyper+d").is_err());
        assert!(chord_to_gpui("super+").is_err());
        assert!(chord_to_gpui("super+notakey").is_err());
    }

    #[test]
    fn defaults_cover_cmux_table() {
        let k = keybindings(None, "test");
        assert_eq!(k.warnings, Vec::<String>::new());
        assert_eq!(keys_for(&k, "splitRight"), vec!["super-d"]);
        assert_eq!(keys_for(&k, "nextSurface"), vec!["super-}", "ctrl-tab"]);
        assert_eq!(keys_for(&k, "resizePaneDown"), vec!["ctrl-shift-j"]);
        assert!(keys_for(&k, "focusNextPane").is_empty());
        let numbered: Vec<_> = k
            .bindings
            .iter()
            .filter(|b| b.action == "selectSurfaceByNumber")
            .map(|b| (b.keystroke.as_str(), b.number))
            .collect();
        assert_eq!(numbered.len(), 9);
        assert_eq!(numbered[0], ("ctrl-1", Some(1)));
        assert_eq!(numbered[8], ("ctrl-9", Some(9)));
    }

    #[test]
    fn user_entries_replace_or_unbind_an_action() {
        let k = keybindings(
            Some(
                "[keybindings]\nsplitRight = \"ctrl+alt+d\"\nsplitDown = [\"super+shift+d\", \"ctrl+alt+shift+d\"]\ncloseTab = \"none\"\nnewSurface = \"\"\nselectSurfaceByNumber = \"alt+1\"\n",
            ),
            "test",
        );
        assert_eq!(k.warnings, Vec::<String>::new());
        assert_eq!(keys_for(&k, "splitRight"), vec!["ctrl-alt-d"]);
        assert_eq!(
            keys_for(&k, "splitDown"),
            vec!["shift-super-d", "ctrl-alt-shift-d"]
        );
        assert!(keys_for(&k, "closeTab").is_empty());
        assert!(keys_for(&k, "newSurface").is_empty());
        assert_eq!(keys_for(&k, "selectSurfaceByNumber")[4], "alt-5");
    }

    #[test]
    fn bad_entries_warn_and_keep_defaults() {
        let k = keybindings(
            Some(
                "[keybindings]\nsplitRight = \"hyper+d\"\nnoSuchAction = \"super+x\"\nsplitDown = 3\nselectSurfaceByNumber = \"ctrl+x\"\n",
            ),
            "cfg",
        );
        assert_eq!(k.warnings.len(), 4, "{:?}", k.warnings);
        assert!(k.warnings.iter().any(|w| w.contains("noSuchAction")));
        assert_eq!(keys_for(&k, "splitRight"), vec!["super-d"]);
        assert_eq!(keys_for(&k, "splitDown"), vec!["shift-super-d"]);
        assert_eq!(keys_for(&k, "selectSurfaceByNumber")[0], "ctrl-1");
    }

    #[test]
    fn malformed_toml_warns_and_uses_defaults() {
        let k = keybindings(Some("[keybindings\nsplitRight = "), "cfg");
        assert_eq!(k.warnings.len(), 1, "{:?}", k.warnings);
        assert!(k.warnings[0].starts_with("cfg: "));
        assert_eq!(keys_for(&k, "splitRight"), vec!["super-d"]);
    }

    #[test]
    fn a_missing_file_is_not_a_warning() {
        let k = load(Path::new("/nonexistent/cmux-linux/config.toml"));
        assert_eq!(k.warnings, Vec::<String>::new());
        assert!(!k.bindings.is_empty());
    }
}
```

Apply this patch to the Ghostty config tests (the test half of the `mod.rs` and `load.rs` changes; save it and run `git apply`, or edit by hand):

```diff
diff --git a/crates/cmux-config/src/ghostty/load.rs b/crates/cmux-config/src/ghostty/load.rs
index e6fcdfd..2ba900a 100644
--- a/crates/cmux-config/src/ghostty/load.rs
+++ b/crates/cmux-config/src/ghostty/load.rs
@@ -466,6 +472,10 @@ mod tests {
         let env = Env::from_vars(Some("/x".into()), Some("/h".into()), None).unwrap();
         assert_eq!(env.config_dir, PathBuf::from("/x/ghostty"));
         assert_eq!(env.home, Some(PathBuf::from("/h")));
+        assert_eq!(
+            env.cmux_config_file(),
+            PathBuf::from("/x/cmux-linux/config.toml")
+        );
     }
 
     #[test]
diff --git a/crates/cmux-config/src/ghostty/mod.rs b/crates/cmux-config/src/ghostty/mod.rs
index 4173ba2..da88b0a 100644
--- a/crates/cmux-config/src/ghostty/mod.rs
+++ b/crates/cmux-config/src/ghostty/mod.rs
@@ -390,6 +426,49 @@ mod tests {
         assert_eq!(cfg.faint_opacity, 0.4);
     }
 
+    #[test]
+    fn split_keys_apply() {
+        let (cfg, warnings) = parse(
+            "unfocused-split-opacity = 0.05\nunfocused-split-fill = #102030\n\
+             split-divider-color = 445566\nsplit-inherit-working-directory = false\n\
+             tab-inherit-working-directory = false\nclipboard-paste-protection = false\n",
+        );
+        assert_eq!(warnings, Vec::<String>::new());
+        assert_eq!(cfg.unfocused_split_opacity, 0.15, "clamped like Ghostty");
+        assert_eq!(
+            cfg.unfocused_split_fill,
+            Some(Rgb {
+                r: 0x10,
+                g: 0x20,
+                b: 0x30
+            })
+        );
+        assert_eq!(
+            cfg.split_divider_color,
+            Some(Rgb {
+                r: 0x44,
+                g: 0x55,
+                b: 0x66
+            })
+        );
+        assert!(!cfg.split_inherit_working_directory);
+        assert!(!cfg.tab_inherit_working_directory);
+        assert!(!cfg.clipboard_paste_protection);
+        let (cfg, _) = parse("split-divider-color = #445566\nsplit-divider-color =\n");
+        assert_eq!(cfg.split_divider_color, None, "empty resets");
+    }
+
+    #[test]
+    fn split_key_defaults_match_ghostty() {
+        let cfg = Config::default();
+        assert_eq!(cfg.unfocused_split_opacity, 0.7);
+        assert_eq!(cfg.unfocused_split_fill, None);
+        assert_eq!(cfg.split_divider_color, None);
+        assert!(cfg.split_inherit_working_directory);
+        assert!(cfg.tab_inherit_working_directory);
+        assert!(cfg.clipboard_paste_protection);
+    }
+
     #[test]
     fn empty_font_family_resets_the_list() {
         let (cfg, _) = parse("font-family = A\nfont-family = \"\"\nfont-family = B\n");
```

- [ ] **Step 3: Run tests to verify they fail.** Run: `cargo test -p cmux-config`. Expected: compile errors (`cannot find function keybindings`, `no field unfocused_split_opacity`, `no method named cmux_config_file`).

- [ ] **Step 4: Implement the Ghostty keys.** Apply:

```diff
diff --git a/crates/cmux-config/src/ghostty/load.rs b/crates/cmux-config/src/ghostty/load.rs
index e6fcdfd..2ba900a 100644
--- a/crates/cmux-config/src/ghostty/load.rs
+++ b/crates/cmux-config/src/ghostty/load.rs
@@ -30,6 +30,12 @@ impl Env {
         Some(Env { dark, ..env })
     }
 
+    /// `$XDG_CONFIG_HOME/cmux-linux/config.toml`: cmux-linux's own settings (key bindings).
+    pub fn cmux_config_file(&self) -> PathBuf {
+        let config_home = self.config_dir.parent().unwrap_or(&self.config_dir);
+        config_home.join("cmux-linux").join("config.toml")
+    }
+
     /// Takes the variables as values so tests can cover the rules without the process env.
     /// `None` when neither a usable `XDG_CONFIG_HOME` nor `HOME` exists.
     fn from_vars(
diff --git a/crates/cmux-config/src/ghostty/mod.rs b/crates/cmux-config/src/ghostty/mod.rs
index 4173ba2..da88b0a 100644
--- a/crates/cmux-config/src/ghostty/mod.rs
+++ b/crates/cmux-config/src/ghostty/mod.rs
@@ -88,6 +88,17 @@ pub struct Config {
     pub selection_background: Option<DynColor>,
     pub selection_foreground: Option<DynColor>,
     pub faint_opacity: f32,
+    /// Clamped to 0.15..=1 as Ghostty does; the dim overlay alpha is `1 - this`.
+    pub unfocused_split_opacity: f32,
+    /// `None` dims with the background colour.
+    pub unfocused_split_fill: Option<Rgb>,
+    /// `None` uses the chrome separator colour (cmux).
+    pub split_divider_color: Option<Rgb>,
+    pub split_inherit_working_directory: bool,
+    pub tab_inherit_working_directory: bool,
+    /// Confirm before pasting text with a newline (or a bracketed-paste end) unless the
+    /// program enabled bracketed paste.
+    pub clipboard_paste_protection: bool,
 }
 
 impl Default for Config {
@@ -116,6 +127,12 @@ impl Default for Config {
             selection_background: None,
             selection_foreground: None,
             faint_opacity: 0.5,
+            unfocused_split_opacity: 0.7,
+            unfocused_split_fill: None,
+            split_divider_color: None,
+            split_inherit_working_directory: true,
+            tab_inherit_working_directory: true,
+            clipboard_paste_protection: true,
         }
     }
 }
@@ -161,6 +178,16 @@ impl Config {
             "selection-background" => self.selection_background = dyn_color(value)?,
             "selection-foreground" => self.selection_foreground = dyn_color(value)?,
             "faint-opacity" => self.faint_opacity = number(value)?.clamp(0.0, 1.0),
+            "unfocused-split-opacity" => {
+                self.unfocused_split_opacity = number(value)?.clamp(0.15, 1.0)
+            }
+            "unfocused-split-fill" => self.unfocused_split_fill = optional_rgb(value)?,
+            "split-divider-color" => self.split_divider_color = optional_rgb(value)?,
+            "split-inherit-working-directory" => {
+                self.split_inherit_working_directory = boolean(value)?
+            }
+            "tab-inherit-working-directory" => self.tab_inherit_working_directory = boolean(value)?,
+            "clipboard-paste-protection" => self.clipboard_paste_protection = boolean(value)?,
             _ => {}
         }
         Ok(())
@@ -246,6 +273,15 @@ fn dyn_color(s: &str) -> Result<Option<DynColor>, String> {
     }))
 }
 
+/// An optional colour; an empty value resets it to unset.
+fn optional_rgb(s: &str) -> Result<Option<Rgb>, String> {
+    if s.is_empty() {
+        Ok(None)
+    } else {
+        parse_rgb(s).map(Some)
+    }
+}
+
 /// `f32::from_str` also accepts `nan` and `inf`, which no config value means.
 fn number(s: &str) -> Result<f32, String> {
     s.parse()
```

- [ ] **Step 5: Implement the keybindings.** Insert above the tests module in `keybindings.rs`:

```rust
use std::path::Path;

/// One gpui binding: `keystroke` runs `action`; `number` is the digit for
/// `selectSurfaceByNumber`, whose chord binds the same modifiers with each of 1-9.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub action: &'static str,
    pub keystroke: String,
    pub number: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Keybindings {
    pub bindings: Vec<Binding>,
    /// One line each, `path: problem`. Never fatal.
    pub warnings: Vec<String>,
}

/// Every action M2 handles with cmux's default chords (research-cmux.md §3; Cmd → super).
/// Ctrl+Tab / Ctrl+Shift+Tab are cmux's hard-wired legacy tab keys; here they are ordinary
/// remappable defaults. Copy and paste are the M1 bindings.
pub const DEFAULTS: &[(&str, &[&str])] = &[
    ("copy", &["super+c", "ctrl+shift+c"]),
    ("paste", &["super+v", "ctrl+shift+v"]),
    ("splitRight", &["super+d"]),
    ("splitDown", &["super+shift+d"]),
    ("toggleSplitZoom", &["super+shift+return"]),
    ("equalizeSplits", &["super+ctrl+shift+="]),
    ("resizePaneLeft", &["ctrl+shift+h"]),
    ("resizePaneRight", &["ctrl+shift+l"]),
    ("resizePaneUp", &["ctrl+shift+k"]),
    ("resizePaneDown", &["ctrl+shift+j"]),
    ("focusLeft", &["super+opt+left"]),
    ("focusRight", &["super+opt+right"]),
    ("focusUp", &["super+opt+up"]),
    ("focusDown", &["super+opt+down"]),
    ("focusPreviousPane", &[]),
    ("focusNextPane", &[]),
    ("newSurface", &["super+t"]),
    ("closeTab", &["super+w"]),
    ("closeOtherTabsInPane", &["super+opt+t"]),
    ("nextSurface", &["super+shift+]", "ctrl+tab"]),
    ("prevSurface", &["super+shift+[", "ctrl+shift+tab"]),
    ("selectSurfaceByNumber", &["ctrl+1"]),
    ("moveSurfaceLeft", &["super+shift+opt+["]),
    ("moveSurfaceRight", &["super+shift+opt+]"]),
];

const NUMBERED: &str = "selectSurfaceByNumber";

/// What a US-layout Shift turns a key into. gpui on Linux reports Shift+] as key `}` with
/// Shift cleared (platform/linux/platform.rs `from_xkb`), so the binding must say `}` too.
// shortcut: US layout only; other layouts need the real keymap. Revisit with M6 layout work.
fn shifted(key: &str) -> Option<&'static str> {
    Some(match key {
        "1" => "!",
        "2" => "@",
        "3" => "#",
        "4" => "$",
        "5" => "%",
        "6" => "^",
        "7" => "&",
        "8" => "*",
        "9" => "(",
        "0" => ")",
        "-" => "_",
        "=" => "+",
        "[" => "{",
        "]" => "}",
        "\\" => "|",
        ";" => ":",
        "'" => "\"",
        "," => "<",
        "." => ">",
        "/" => "?",
        "`" => "~",
        _ => return None,
    })
}

/// cmux's key tokens (`parseConfigKeyToken`) in gpui's key names.
fn key_name(token: &str) -> Option<String> {
    let t = token.trim().to_lowercase();
    let named = match t.as_str() {
        "left" | "arrowleft" | "leftarrow" | "←" => "left",
        "right" | "arrowright" | "rightarrow" | "→" => "right",
        "up" | "arrowup" | "uparrow" | "↑" => "up",
        "down" | "arrowdown" | "downarrow" | "↓" => "down",
        "tab" => "tab",
        "return" | "enter" | "↩" => "enter",
        "space" | "spacebar" | "<space>" => "space",
        "escape" | "esc" => "escape",
        "backspace" => "backspace",
        "delete" => "delete",
        "home" => "home",
        "end" => "end",
        "pageup" => "pageup",
        "pagedown" => "pagedown",
        "comma" => ",",
        "period" | "dot" => ".",
        "slash" => "/",
        "backslash" => "\\",
        "semicolon" => ";",
        "quote" | "apostrophe" => "'",
        "backtick" | "grave" => "`",
        "minus" | "hyphen" => "-",
        "plus" | "equals" => "=",
        "leftbracket" | "openbracket" => "[",
        "rightbracket" | "closebracket" => "]",
        f if f.len() > 1
            && f.starts_with('f')
            && f[1..].parse::<u8>().is_ok_and(|n| (1..=20).contains(&n)) =>
        {
            return Some(f.to_string());
        }
        c if c.chars().count() == 1 && !c.chars().any(char::is_whitespace) => {
            return Some(c.to_string());
        }
        _ => return None,
    };
    Some(named.to_string())
}

/// One cmux chord (`super+shift+]`) as a gpui keystroke (`super-}`).
pub fn chord_to_gpui(chord: &str) -> Result<String, String> {
    let parts: Vec<&str> = chord.split('+').collect();
    let (key, mods) = parts.split_last().expect("split yields one part");
    let (mut ctrl, mut alt, mut shift, mut sup) = (false, false, false, false);
    for m in mods {
        match m.trim().to_lowercase().as_str() {
            "cmd" | "command" | "super" | "⌘" => sup = true,
            "shift" | "⇧" => shift = true,
            "opt" | "option" | "alt" | "⌥" => alt = true,
            "ctrl" | "control" | "ctl" | "⌃" => ctrl = true,
            other => return Err(format!("unknown modifier `{other}` in `{chord}`")),
        }
    }
    let mut key = key_name(key).ok_or_else(|| format!("unknown key in `{chord}`"))?;
    if shift && let Some(s) = shifted(&key) {
        key = s.to_string();
        shift = false;
    }
    let mut out = String::new();
    for (on, name) in [
        (ctrl, "ctrl-"),
        (alt, "alt-"),
        (shift, "shift-"),
        (sup, "super-"),
    ] {
        if on {
            out.push_str(name);
        }
    }
    out.push_str(&key);
    Ok(out)
}

/// The chords for one action as gpui bindings. `selectSurfaceByNumber` chords must end in a
/// digit; their modifiers are bound with each of 1-9.
fn expand(action: &'static str, chords: &[String]) -> Result<Vec<Binding>, String> {
    let mut out = Vec::new();
    for chord in chords {
        if action == NUMBERED {
            let last = chord.rsplit('+').next().unwrap_or_default().trim();
            if !last.chars().all(|c| c.is_ascii_digit()) || last.is_empty() {
                return Err(format!("`{chord}` must end in a digit (it binds 1-9)"));
            }
            let mods = &chord[..chord.len() - last.len()];
            for n in 1..=9u8 {
                out.push(Binding {
                    action,
                    keystroke: chord_to_gpui(&format!("{mods}{n}"))?,
                    number: Some(n),
                });
            }
        } else {
            out.push(Binding {
                action,
                keystroke: chord_to_gpui(chord)?,
                number: None,
            });
        }
    }
    Ok(out)
}

/// A TOML value as a chord list: a string, or an array of strings. `""` / `"none"` = none.
fn chords(value: &toml::Value) -> Result<Vec<String>, String> {
    let list = match value {
        toml::Value::String(s) => vec![s.clone()],
        toml::Value::Array(items) => items
            .iter()
            .map(|v| v.as_str().map(str::to_string).ok_or("expected a string"))
            .collect::<Result<_, _>>()?,
        _ => return Err("expected a chord string or a list of them".into()),
    };
    Ok(list
        .into_iter()
        .filter(|c| !c.trim().is_empty() && !c.trim().eq_ignore_ascii_case("none"))
        .collect())
}

/// Defaults, with each action named in `user`'s `[keybindings]` replaced by the user's chords.
/// `origin` prefixes warnings. A bad entry is reported and its action keeps its defaults.
pub fn keybindings(user: Option<&str>, origin: &str) -> Keybindings {
    let mut warnings = Vec::new();
    let mut table = toml::Table::new();
    if let Some(text) = user {
        match text.parse::<toml::Table>() {
            Ok(t) => table = t,
            Err(e) => warnings.push(format!("{origin}: {}", e.message())),
        }
    }
    let user = match table.get("keybindings") {
        None => toml::Table::new(),
        Some(toml::Value::Table(t)) => t.clone(),
        Some(_) => {
            warnings.push(format!("{origin}: keybindings: expected a table"));
            toml::Table::new()
        }
    };
    for name in user.keys() {
        if !DEFAULTS.iter().any(|(a, _)| a == name) {
            warnings.push(format!("{origin}: keybindings: unknown action `{name}`"));
        }
    }
    let mut bindings = Vec::new();
    for (action, defaults) in DEFAULTS {
        let defaults: Vec<String> = defaults.iter().map(|c| c.to_string()).collect();
        let expanded = match user.get(*action).map(chords) {
            None => expand(action, &defaults),
            Some(Ok(list)) => expand(action, &list).or_else(|e| {
                warnings.push(format!("{origin}: keybindings.{action}: {e}"));
                expand(action, &defaults)
            }),
            Some(Err(e)) => {
                warnings.push(format!("{origin}: keybindings.{action}: {e}"));
                expand(action, &defaults)
            }
        };
        bindings.extend(expanded.expect("the default table is valid"));
    }
    Keybindings { bindings, warnings }
}

/// Reads `path` (a missing file is fine) and builds the bindings.
pub fn load(path: &Path) -> Keybindings {
    let origin = path.display().to_string();
    match std::fs::read_to_string(path) {
        Ok(text) => keybindings(Some(&text), &origin),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => keybindings(None, &origin),
        Err(e) => {
            let mut k = keybindings(None, &origin);
            k.warnings.push(format!("{origin}: {e}"));
            k
        }
    }
}
```

- [ ] **Step 6: Run tests to verify they pass.** Run: `cargo test -p cmux-config --locked`. Expected: `test result: ok. 41 passed`.

- [ ] **Step 7: Format, lint, commit.**

```bash
cargo fmt -p cmux-config
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add Cargo.toml Cargo.lock NOTICE crates/cmux-config
git commit -m "feat(cmux-config): Ghostty split/paste keys and cmux keybinding table with TOML overrides" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Actions and keymap; terminal events; Copy passthrough

D3, D4 (events only). rg §1.5–1.7.

**Files:**
- Create: `crates/cmux-app/src/actions.rs`
- Modify: `crates/cmux-app/src/lib.rs`, `crates/cmux-app/src/keys.rs`, `crates/cmux-app/src/terminal_view.rs`, `crates/cmux-app/src/main.rs`

**Interfaces:**
- Consumes: `cmux_config::keybindings::{Binding, Keybindings, DEFAULTS, keybindings, load}`, `Env::cmux_config_file` (Task 5).
- Produces:
  - `cmux_app::actions`: unit actions `Copy, Paste, SplitRight, SplitDown, ToggleSplitZoom, EqualizeSplits, ResizePaneLeft/Right/Up/Down, FocusLeft/Right/Up/Down, FocusPreviousPane, FocusNextPane, NewSurface, CloseTab, CloseOtherTabsInPane, NextSurface, PrevSurface, MoveSurfaceLeft, MoveSurfaceRight`; `struct SelectSurfaceByNumber(pub u8)`; `fn action_for(&Binding) -> Option<Box<dyn Action>>`; `fn key_binding(&Binding, &dyn PlatformKeyboardMapper) -> Result<KeyBinding, String>`; `fn bind(&mut App, &Keybindings) -> Vec<String>` (warnings)
  - `terminal_view`: `enum TerminalEvent { Exited, TitleChanged }`, `impl EventEmitter<TerminalEvent> for TerminalView`, `TerminalView::new(style: Rc<TermStyle>, cwd: Option<String>, &mut Window, &mut Context<Self>)`, `TerminalView::title(&self) -> Option<&str>`, `TerminalView::cwd(&self) -> Option<&str>`. `TerminalView` no longer sets the window title.
  - Removed: `keys::{AppAction, BINDINGS, chord, binding}`.

- [ ] **Step 1: Write the failing tests.** Change `crates/cmux-app/src/lib.rs` to:

```rust
//! cmux-app: the gpui app. Everything but `main` lives here so it can be unit-tested.

pub mod actions;
pub mod colors;
pub mod ime;
pub mod keys;
pub mod mouse;
pub mod style;
pub mod terminal_element;
pub mod terminal_view;
```

Create `crates/cmux-app/src/actions.rs` with its header and tests module only:

```rust
//! gpui actions for cmux's shortcuts and binding them from cmux-config's table.

#[cfg(test)]
mod tests {
    use super::*;
    use cmux_config::keybindings::{DEFAULTS, keybindings};
    use gpui::{DummyKeyboardMapper, Keystroke, Modifiers};

    #[test]
    fn every_config_action_has_a_gpui_action() {
        for (name, _) in DEFAULTS {
            let b = Binding {
                action: name,
                keystroke: String::new(),
                number: Some(1),
            };
            assert!(action_for(&b).is_some(), "{name}");
        }
    }

    fn binding_for(action: &str) -> KeyBinding {
        let k = keybindings(None, "test");
        let b = k.bindings.iter().find(|b| b.action == action).unwrap();
        key_binding(b, &DummyKeyboardMapper).unwrap()
    }

    /// What gpui's Linux backend reports (platform/linux/platform.rs `from_xkb`).
    fn typed(
        platform: bool,
        control: bool,
        shift: bool,
        key: &str,
        key_char: Option<&str>,
    ) -> Keystroke {
        Keystroke {
            modifiers: Modifiers {
                platform,
                control,
                shift,
                ..Default::default()
            },
            key: key.into(),
            key_char: key_char.map(Into::into),
        }
    }

    #[test]
    fn the_default_table_parses_in_gpui() {
        for b in &keybindings(None, "test").bindings {
            assert!(key_binding(b, &DummyKeyboardMapper).is_ok(), "{b:?}");
        }
    }

    #[test]
    fn defaults_match_what_linux_reports() {
        // Super+Shift+] arrives as `}` with Shift cleared; Super+Shift+D keeps Shift.
        let next = binding_for("nextSurface");
        assert_eq!(
            next.match_keystrokes(&[typed(true, false, false, "}", Some("}"))]),
            Some(false)
        );
        let down = binding_for("splitDown");
        assert_eq!(
            down.match_keystrokes(&[typed(true, false, true, "d", Some("D"))]),
            Some(false)
        );
        let copy = binding_for("copy");
        assert_eq!(
            copy.match_keystrokes(&[typed(true, false, false, "c", Some("c"))]),
            Some(false)
        );
        assert_eq!(
            copy.match_keystrokes(&[typed(false, true, false, "c", None)]),
            None
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail.** Run: `cargo test -p cmux-app --lib actions`. Expected: compile errors (`cannot find function action_for`, `cannot find function key_binding`).

- [ ] **Step 3: Implement the actions.** Insert above the tests module in `actions.rs`:

```rust
use cmux_config::keybindings::{Binding, Keybindings};
use gpui::{Action, App, KeyBinding, PlatformKeyboardMapper};

gpui::actions!(
    cmux,
    [
        Copy,
        Paste,
        SplitRight,
        SplitDown,
        ToggleSplitZoom,
        EqualizeSplits,
        ResizePaneLeft,
        ResizePaneRight,
        ResizePaneUp,
        ResizePaneDown,
        FocusLeft,
        FocusRight,
        FocusUp,
        FocusDown,
        FocusPreviousPane,
        FocusNextPane,
        NewSurface,
        CloseTab,
        CloseOtherTabsInPane,
        NextSurface,
        PrevSurface,
        MoveSurfaceLeft,
        MoveSurfaceRight,
    ]
);

/// cmux `selectSurfaceByNumber`: 1-8 pick that tab, 9 the last one.
#[derive(Clone, PartialEq, Debug, gpui::Action)]
#[action(namespace = cmux, no_json)]
pub struct SelectSurfaceByNumber(pub u8);

/// The gpui action for one config binding. `None` only if cmux-config lists an action this
/// table lacks (a test keeps them in sync).
pub fn action_for(b: &Binding) -> Option<Box<dyn Action>> {
    Some(match b.action {
        "copy" => Box::new(Copy),
        "paste" => Box::new(Paste),
        "splitRight" => Box::new(SplitRight),
        "splitDown" => Box::new(SplitDown),
        "toggleSplitZoom" => Box::new(ToggleSplitZoom),
        "equalizeSplits" => Box::new(EqualizeSplits),
        "resizePaneLeft" => Box::new(ResizePaneLeft),
        "resizePaneRight" => Box::new(ResizePaneRight),
        "resizePaneUp" => Box::new(ResizePaneUp),
        "resizePaneDown" => Box::new(ResizePaneDown),
        "focusLeft" => Box::new(FocusLeft),
        "focusRight" => Box::new(FocusRight),
        "focusUp" => Box::new(FocusUp),
        "focusDown" => Box::new(FocusDown),
        "focusPreviousPane" => Box::new(FocusPreviousPane),
        "focusNextPane" => Box::new(FocusNextPane),
        "newSurface" => Box::new(NewSurface),
        "closeTab" => Box::new(CloseTab),
        "closeOtherTabsInPane" => Box::new(CloseOtherTabsInPane),
        "nextSurface" => Box::new(NextSurface),
        "prevSurface" => Box::new(PrevSurface),
        "moveSurfaceLeft" => Box::new(MoveSurfaceLeft),
        "moveSurfaceRight" => Box::new(MoveSurfaceRight),
        "selectSurfaceByNumber" => Box::new(SelectSurfaceByNumber(b.number?)),
        _ => return None,
    })
}

/// One config binding as a gpui `KeyBinding` (contextless, so one scheme decides precedence;
/// research-gpui.md §1.5). Never panics: a bad keystroke is an `Err`.
pub fn key_binding(b: &Binding, mapper: &dyn PlatformKeyboardMapper) -> Result<KeyBinding, String> {
    let action = action_for(b).ok_or_else(|| format!("no handler for `{}`", b.action))?;
    KeyBinding::load(&b.keystroke, action, None, false, None, mapper)
        .map_err(|e| format!("keybindings.{}: {e}", b.action))
}

/// Binds every entry; returns a warning per binding gpui rejected (the rest still bind).
pub fn bind(cx: &mut App, keybindings: &Keybindings) -> Vec<String> {
    let mapper = cx.keyboard_mapper().clone();
    let (ok, bad): (Vec<_>, Vec<_>) = keybindings
        .bindings
        .iter()
        .map(|b| key_binding(b, mapper.as_ref()))
        .partition(Result::is_ok);
    cx.bind_keys(ok.into_iter().map(Result::unwrap));
    bad.into_iter().map(|e| e.unwrap_err()).collect()
}
```

- [ ] **Step 4: Remove the M1 binding table.** Apply:

```diff
diff --git a/crates/cmux-app/src/keys.rs b/crates/cmux-app/src/keys.rs
index c664bdd..a158c6f 100644
--- a/crates/cmux-app/src/keys.rs
+++ b/crates/cmux-app/src/keys.rs
@@ -1,49 +1,8 @@
-//! gpui keystrokes to libghostty-vt key events, and the app's default key bindings.
+//! gpui keystrokes to libghostty-vt key events. Bindings live in cmux-config and `actions`.
 
 use cmux_term::ghostty_vt::{KeyAction, KeyInput, Mods, sys};
 use gpui::{Keystroke, Modifiers};
 
-/// Things a key can do besides typing.
-#[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub enum AppAction {
-    Copy,
-    Paste,
-}
-
-/// Default bindings, as `ctrl-alt-shift-super-key` chords (⌘ maps to Super).
-/// Remapping through config arrives in M2; until then this table is the one place to change.
-pub const BINDINGS: &[(&str, AppAction)] = &[
-    ("super-c", AppAction::Copy),
-    ("super-v", AppAction::Paste),
-    ("ctrl-shift-c", AppAction::Copy),
-    ("ctrl-shift-v", AppAction::Paste),
-];
-
-pub fn chord(ks: &Keystroke) -> String {
-    let m = &ks.modifiers;
-    let mut s = String::new();
-    for (on, name) in [
-        (m.control, "ctrl-"),
-        (m.alt, "alt-"),
-        (m.shift, "shift-"),
-        (m.platform, "super-"),
-    ] {
-        if on {
-            s.push_str(name);
-        }
-    }
-    s.push_str(&ks.key);
-    s
-}
-
-pub fn binding(ks: &Keystroke) -> Option<AppAction> {
-    let chord = chord(ks);
-    BINDINGS
-        .iter()
-        .find(|(b, _)| *b == chord)
-        .map(|(_, action)| *action)
-}
-
 pub fn mods(m: &Modifiers) -> Mods {
     let mut out = Mods::default();
     for (on, bit) in [
@@ -354,14 +313,4 @@ mod tests {
         );
         assert_eq!(shifted.action, Some(KeyAction::Repeat));
     }
-
-    #[test]
-    fn default_bindings() {
-        assert_eq!(binding(&ks("p", "c", None)), Some(AppAction::Copy));
-        assert_eq!(binding(&ks("p", "v", None)), Some(AppAction::Paste));
-        assert_eq!(binding(&ks("cs", "c", Some("C"))), Some(AppAction::Copy));
-        assert_eq!(binding(&ks("cs", "v", Some("V"))), Some(AppAction::Paste));
-        assert_eq!(binding(&ks("c", "c", None)), None);
-        assert_eq!(chord(&ks("csap", "x", None)), "ctrl-alt-shift-super-x");
-    }
 }
```

- [ ] **Step 5: Terminal events, Copy/Paste actions, key-up rule, cwd.** Apply:

```diff
diff --git a/crates/cmux-app/src/terminal_view.rs b/crates/cmux-app/src/terminal_view.rs
index e49025d..fde47b8 100644
--- a/crates/cmux-app/src/terminal_view.rs
+++ b/crates/cmux-app/src/terminal_view.rs
@@ -15,12 +15,13 @@ use cmux_term::selection::Selection;
 use futures::StreamExt;
 use futures::channel::mpsc;
 use gpui::{
-    App, ClipboardItem, Context, FocusHandle, Focusable, IntoElement, KeyDownEvent, KeyUpEvent,
-    Keystroke, MouseButton, ParentElement, Pixels, Point, Render, Styled, Subscription, Task,
-    Window, div, prelude::*, px,
+    App, ClipboardItem, Context, EventEmitter, FocusHandle, Focusable, IntoElement, KeyDownEvent,
+    KeyUpEvent, Keystroke, MouseButton, ParentElement, Pixels, Point, Render, Styled, Subscription,
+    Task, Window, div, prelude::*, px,
 };
 
-use crate::keys::{self, AppAction};
+use crate::actions::{Copy, Paste};
+use crate::keys;
 use crate::style::TermStyle;
 use crate::terminal_element::TerminalElement;
 
@@ -70,6 +71,15 @@ impl Geometry {
     }
 }
 
+/// What the workspace hears from a terminal.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum TerminalEvent {
+    /// The shell exited; the workspace closes this tab.
+    Exited,
+    /// The OSC 0/2 title changed.
+    TitleChanged,
+}
+
 pub struct TerminalView {
     pub(crate) session: Session,
     pub(crate) style: Rc<TermStyle>,
@@ -88,10 +98,12 @@ pub struct TerminalView {
     pub(crate) dragging: bool,
     /// Wheel pixels not yet turned into whole lines.
     pub(crate) scroll_px: f32,
-    /// `Keystroke.key` of presses consumed by an app binding, whose release must not reach the PTY.
-    bound_keys: HashSet<String>,
+    /// `Keystroke.key` of presses that went to the PTY, so their release goes there too even
+    /// when a binding (a Copy with nothing selected) let the press through.
+    forwarded_keys: HashSet<String>,
     title: Option<String>,
-    title_dirty: bool,
+    /// Last OSC 7 directory, for new splits and tabs.
+    cwd: Option<String>,
     /// `CMUX_TRACE_LATENCY=1`: key press time, waiting for its echo.
     trace_latency: bool,
     key_at: Option<Instant>,
@@ -101,10 +113,20 @@ pub struct TerminalView {
 }
 
 impl TerminalView {
-    pub fn new(style: Rc<TermStyle>, window: &mut Window, cx: &mut Context<Self>) -> Self {
+    /// Starts the user's shell, in `cwd` when it is an existing directory (else `$HOME`).
+    pub fn new(
+        style: Rc<TermStyle>,
+        cwd: Option<String>,
+        window: &mut Window,
+        cx: &mut Context<Self>,
+    ) -> Self {
         let (tx, mut rx) = mpsc::unbounded::<PtyEvent>();
+        let mut cmd = Session::shell_command();
+        if let Some(dir) = cwd.as_deref().filter(|d| std::path::Path::new(d).is_dir()) {
+            cmd.cwd(dir);
+        }
         let session = Session::spawn(
-            Session::shell_command(),
+            cmd,
             80,
             24,
             |term| style.apply_to(term),
@@ -173,9 +195,9 @@ impl TerminalView {
             mouse: MouseEncoders::new().expect("libghostty-vt mouse encoders"),
             dragging: false,
             scroll_px: 0.0,
-            bound_keys: HashSet::new(),
+            forwarded_keys: HashSet::new(),
             title: None,
-            title_dirty: false,
+            cwd,
             trace_latency: std::env::var_os("CMUX_TRACE_LATENCY").is_some(),
             key_at: None,
             echo_at: None,
@@ -187,8 +209,8 @@ impl TerminalView {
     fn on_pty_events(&mut self, events: Vec<PtyEvent>, cx: &mut Context<Self>) {
         for event in events {
             match event {
-                // Ghostty closes the surface when its shell exits; M1 has one surface.
-                PtyEvent::Exited => return cx.quit(),
+                // Ghostty closes the surface when its shell exits; the workspace does that.
+                PtyEvent::Exited => return cx.emit(TerminalEvent::Exited),
                 PtyEvent::Output(events) => {
                     if let Some(at) = self.key_at.take() {
                         self.echo_at = Some(at);
@@ -197,10 +219,11 @@ impl TerminalView {
                         match e {
                             TermEvent::Title(t) => {
                                 self.title = Some(t);
-                                self.title_dirty = true;
+                                cx.emit(TerminalEvent::TitleChanged);
                             }
-                            // Bell, cwd and notifications feed the sidebar and attention UI (M3/M4).
-                            TermEvent::Bell | TermEvent::Cwd(_) | TermEvent::Notify { .. } => {}
+                            TermEvent::Cwd(dir) => self.cwd = Some(dir),
+                            // Bell and notifications feed the sidebar and attention UI (M3/M4).
+                            TermEvent::Bell | TermEvent::Notify { .. } => {}
                         }
                     }
                 }
@@ -209,6 +232,16 @@ impl TerminalView {
         cx.notify();
     }
 
+    /// The OSC 0/2 title, if the shell or program set one.
+    pub fn title(&self) -> Option<&str> {
+        self.title.as_deref()
+    }
+
+    /// The last OSC 7 working directory.
+    pub fn cwd(&self) -> Option<&str> {
+        self.cwd.as_deref()
+    }
+
     /// Called by the element after painting: logs key-to-echo-to-paint latency when tracing.
     pub(crate) fn painted(&mut self) {
         if let Some(at) = self.echo_at.take()
@@ -235,8 +268,8 @@ impl TerminalView {
     }
 
     fn focus_changed(&mut self, focused: bool, cx: &mut Context<Self>) {
-        // A binding key released while unfocused never sends key_up here; forget it.
-        self.bound_keys.clear();
+        // A key released while unfocused never sends key_up here; forget it.
+        self.forwarded_keys.clear();
         // A release lost while unfocused would otherwise keep clicks local on a tracking TUI.
         self.dragging = false;
         // The IME is disabled on leave and never ends the preedit itself.
@@ -269,15 +302,9 @@ impl TerminalView {
         out
     }
 
+    /// Runs only for keys no action consumed: gpui dispatches bound chords to their action
+    /// handlers first (research-gpui.md §1.6).
     fn key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
-        if let Some(action) = keys::binding(&ev.keystroke) {
-            self.bound_keys.insert(ev.keystroke.key.clone());
-            match action {
-                AppAction::Copy => self.copy(cx),
-                AppAction::Paste => self.paste(cx),
-            }
-            return cx.stop_propagation();
-        }
         // Super combos are desktop and app shortcuts, never terminal input. gpui's Linux text
         // fallback already skips them (wayland/window.rs:725-727, x11/window.rs:1004 only insert
         // `key_char` for shift-or-no modifiers), but replay_pending_input (window.rs:3969-3978)
@@ -296,14 +323,27 @@ impl TerminalView {
             // ends the dead-key preedit; gpui sends no unmark for it (wayland/client.rs:1320).
             self.selection = None;
             self.marked_text.clear();
+            self.forwarded_keys.insert(ev.keystroke.key.clone());
             self.send_input(bytes);
             cx.stop_propagation();
             cx.notify();
         }
     }
 
-    fn key_up(&mut self, ev: &KeyUpEvent, _: &mut Window, _: &mut Context<Self>) {
-        if self.bound_keys.remove(&ev.keystroke.key) || ev.keystroke.modifiers.platform {
+    fn key_up(&mut self, ev: &KeyUpEvent, window: &mut Window, cx: &mut Context<Self>) {
+        let forwarded = self.forwarded_keys.remove(&ev.keystroke.key);
+        if ev.keystroke.modifiers.platform {
+            return;
+        }
+        // A release whose press ran an app action must not reach the PTY either.
+        let bound = || {
+            !cx.key_bindings()
+                .borrow()
+                .bindings_for_input(std::slice::from_ref(&ev.keystroke), &window.context_stack())
+                .0
+                .is_empty()
+        };
+        if !forwarded && bound() {
             return;
         }
         // Empty unless the app asked for kitty's report-event-types flag.
@@ -313,6 +353,19 @@ impl TerminalView {
         }
     }
 
+    /// Copy action. With nothing selected the key falls through to `key_down` (and so to the
+    /// PTY for Ctrl+Shift+C), like Ghostty's `performable:` bindings.
+    fn copy_action(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
+        if self.selection.is_none() {
+            return cx.propagate();
+        }
+        self.copy(cx);
+    }
+
+    fn paste_action(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
+        self.paste(cx);
+    }
+
     pub(crate) fn copy(&mut self, cx: &mut Context<Self>) {
         let Some(selection) = &self.selection else {
             return;
@@ -342,20 +395,18 @@ impl Focusable for TerminalView {
     }
 }
 
+impl EventEmitter<TerminalEvent> for TerminalView {}
+
 impl Render for TerminalView {
     fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
-        if self.title_dirty {
-            self.title_dirty = false;
-            if let Some(title) = &self.title {
-                window.set_window_title(title);
-            }
-        }
         div()
             .size_full()
             .p(px(snap_padding(PADDING_PX, window.scale_factor())))
             .bg(self.style.window_background())
             .track_focus(&self.focus)
             .key_context("Terminal")
+            .on_action(cx.listener(Self::copy_action))
+            .on_action(cx.listener(Self::paste_action))
             .on_key_down(cx.listener(Self::key_down))
             .on_key_up(cx.listener(Self::key_up))
             .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
```

- [ ] **Step 6: Bind keys at startup.** Replace `crates/cmux-app/src/main.rs` with (the subscription keeps M1's quit-on-exit until Task 7 replaces this window content):

```rust
use std::rc::Rc;

use cmux_app::actions;
use cmux_app::style::{TermStyle, installed_families, load_bundled_fonts};
use cmux_app::terminal_view::{TerminalEvent, TerminalView};
use cmux_config::ghostty::{Env, load};
use cmux_config::keybindings;
use gpui::{
    App, AppContext, Application, Bounds, Focusable, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, px, size,
};

fn main() {
    Application::new().run(|cx: &mut App| {
        let dark = matches!(
            cx.window_appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let env = Env::from_system(dark);
        let loaded = env.as_ref().map(load).unwrap_or_default();
        let keys = match &env {
            Some(env) => keybindings::load(&env.cmux_config_file()),
            None => keybindings::keybindings(None, "config.toml"),
        };
        let bind_warnings = actions::bind(cx, &keys);
        // shortcut: warnings go to stderr; an in-window warning line is not planned before M6.
        for warning in loaded
            .warnings
            .iter()
            .chain(&keys.warnings)
            .chain(&bind_warnings)
        {
            eprintln!("cmux-linux: config: {warning}");
        }
        // Order matters: gpui caches failed font lookups, so the bundled font is registered and
        // the family picked before anything resolves a font_id.
        load_bundled_fonts(cx);
        let installed = installed_families(cx);
        let style = Rc::new(TermStyle::new(&loaded.config, &installed));
        // Translucent only when asked. KWin blur needs ext_background_effect_v1, which gpui 0.2.2
        // lacks (M0 journal); Blurred is kept so it works once gpui supports it.
        let window_background = if loaded.config.background_opacity < 1.0 {
            WindowBackgroundAppearance::Blurred
        } else {
            WindowBackgroundAppearance::Opaque
        };
        let bounds = Bounds::centered(None, size(px(1100.), px(700.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background,
                app_id: Some("cmux-linux".into()),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| TerminalView::new(style, None, window, cx));
                let focus = view.read(cx).focus_handle(cx);
                window.focus(&focus);
                // M1 behaviour until the workspace (Task 7) owns terminals: shell exit quits.
                cx.subscribe(&view, |_, ev: &TerminalEvent, cx| {
                    if *ev == TerminalEvent::Exited {
                        cx.quit();
                    }
                })
                .detach();
                view
            },
        )
        .expect("open window");
        cx.activate(true);
    });
}
```

- [ ] **Step 7: Run tests and lints.** Run: `cargo test -p cmux-app --locked` then the clippy line. Expected: `test result: ok. 51 passed` (`every_config_action_has_a_gpui_action`, `the_default_table_parses_in_gpui`, `defaults_match_what_linux_reports` new; `keys::tests::default_bindings` gone); clippy clean.

- [ ] **Step 8: Run it.** `cargo run -p cmux-app`. Check: Super+C/V and Ctrl+Shift+C/V copy and paste as in M1; with nothing selected, `sleep 100` then Ctrl+Shift+C interrupts it (the key reached the PTY); Super+C with nothing selected does nothing; `kitten show-key -m kitty` (if kitty is installed) shows a release for every press it shows; `exit` still quits.

- [ ] **Step 9: Format, lint, commit.**

```bash
cargo fmt -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-app
git commit -m "feat(cmux-app): gpui actions bound from config; terminal events; copy passthrough" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: WorkspaceView: panes, dividers, split/close/focus/zoom/resize/equalize

D2, D4, D5, D6 (dimming, divider colour, focus click). rc §1, §2, §4 "Dividers" and "Unfocused-pane dimming", §5, §6; rg §1.2 B, §1.4, §1.6.

**Files:**
- Create: `crates/cmux-app/src/chrome.rs`, `crates/cmux-app/src/workspace.rs`
- Modify: `Cargo.toml`, `crates/cmux-app/Cargo.toml`, `Cargo.lock`, `crates/cmux-app/src/lib.rs`, `crates/cmux-app/src/mouse.rs`, `crates/cmux-app/src/main.rs`

**Interfaces:**
- Consumes: Tasks 3–4 `Layout` API, `RESIZE_STEP`, `Rects`/`DividerRect`; Task 6 actions, `TerminalView::{new, title, cwd}`, `TerminalEvent`; Task 5 `Config` split keys.
- Produces:
  - `chrome`: `fn rgba(Rgb, f32) -> Rgba`, `fn shift(Rgba, amount: f32, alpha: f32) -> Rgba`, `fn separator(bg: Rgba) -> Rgba`, `fn divider(&Config) -> Rgba`, `fn dim_overlay(&Config) -> Option<Rgba>`
  - `workspace::WorkspaceView::new(Rc<TermStyle>, &mut Window, &mut Context<Self>)`, `layout(&self) -> &Layout`, `terminal(&self, SurfaceId) -> Option<&Entity<TerminalView>>`, `close_surface(&mut self, SurfaceId, &mut Window, &mut Context<Self>)`; `impl Focusable`, `impl Render`.
  - Private, used by Tasks 8–9: fields `layout`, `surfaces: HashMap<SurfaceId, Surface>` (`Surface { view, .. }`), `style`; methods `focus_active`, `split(Orientation, ..)`, `new_tab(..)`, `render_pane(PaneId, Rect, Option<Rgba>)`.

- [ ] **Step 1: Wire the crate.** Root `Cargo.toml` `[workspace.dependencies]` add `cmux-core = { path = "crates/cmux-core" }`; `crates/cmux-app/Cargo.toml` `[dependencies]` add `cmux-core.workspace = true`. Change `crates/cmux-app/src/lib.rs` to:

```rust
//! cmux-app: the gpui app. Everything but `main` lives here so it can be unit-tested.

pub mod actions;
pub mod chrome;
pub mod colors;
pub mod ime;
pub mod keys;
pub mod mouse;
pub mod style;
pub mod terminal_element;
pub mod terminal_view;
pub mod workspace;
```

Run `cargo build -p cmux-core` (without `--locked`), then `git diff Cargo.lock`. Expected: only `"cmux-core",` added to `cmux-app`'s dependency list; libc unchanged.

- [ ] **Step 2: Write the failing tests.** Create `crates/cmux-app/src/chrome.rs` with its header and tests module only:

```rust
//! Chrome colours derived from the terminal background with cmux's formulas
//! (research-cmux.md §4): separators and dividers (`WindowChromeColorResolver.separatorColor`),
//! the unfocused-pane dim (Ghostty `unfocused-split-*`).

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Rgba, b: Rgba) -> bool {
        [a.r - b.r, a.g - b.g, a.b - b.b, a.a - b.a]
            .iter()
            .all(|d| d.abs() < 1e-4)
    }

    #[test]
    fn separator_steps_away_from_the_background() {
        let dark = rgba(
            Rgb {
                r: 0x28,
                g: 0x2c,
                b: 0x34,
            },
            1.0,
        );
        let want = Rgba {
            r: 0x28 as f32 / 255.0 + 0.16,
            g: 0x2c as f32 / 255.0 + 0.16,
            b: 0x34 as f32 / 255.0 + 0.16,
            a: 0.36,
        };
        assert!(close(separator(dark), want));
        let light = rgba(
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            1.0,
        );
        assert!(close(
            separator(light),
            Rgba {
                r: 0.7,
                g: 0.7,
                b: 0.7,
                a: 0.26
            }
        ));
    }

    #[test]
    fn divider_prefers_the_ghostty_colour() {
        let mut cfg = Config::default();
        assert!(close(divider(&cfg), separator(rgba(cfg.background, 1.0))));
        cfg.split_divider_color = Some(Rgb { r: 255, g: 0, b: 0 });
        assert!(close(
            divider(&cfg),
            Rgba {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0
            }
        ));
    }

    #[test]
    fn dim_overlay_uses_ghostty_opacity_and_fill() {
        let mut cfg = Config::default();
        let dim = dim_overlay(&cfg).unwrap();
        assert!((dim.a - 0.3).abs() < 1e-6);
        assert!(close(Rgba { a: 1.0, ..dim }, rgba(cfg.background, 1.0)));
        cfg.unfocused_split_fill = Some(Rgb { r: 0, g: 0, b: 0 });
        assert!(close(
            dim_overlay(&cfg).unwrap(),
            Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.3
            }
        ));
        cfg.unfocused_split_opacity = 1.0;
        assert_eq!(dim_overlay(&cfg), None);
    }
}
```

Create `crates/cmux-app/src/workspace.rs` containing only `//! The workspace (filled in below).` so the module list compiles.

- [ ] **Step 3: Run tests to verify they fail.** Run: `cargo test -p cmux-app --lib chrome`. Expected: compile errors (`cannot find function separator`, `cannot find function dim_overlay`).

- [ ] **Step 4: Implement the colours.** Insert above the tests module in `chrome.rs`:

```rust
use cmux_config::ghostty::{Config, Rgb};
use gpui::Rgba;

pub fn rgba(c: Rgb, a: f32) -> Rgba {
    Rgba {
        r: f32::from(c.r) / 255.0,
        g: f32::from(c.g) / 255.0,
        b: f32::from(c.b) / 255.0,
        a,
    }
}

/// Each RGB channel moved by `amount` and clamped; alpha replaced.
pub fn shift(c: Rgba, amount: f32, alpha: f32) -> Rgba {
    let f = |v: f32| (v + amount).clamp(0.0, 1.0);
    Rgba {
        r: f(c.r),
        g: f(c.g),
        b: f(c.b),
        a: alpha,
    }
}

/// cmux's separator test: Rec. 601 luma above 0.5.
fn luma_is_light(c: Rgba) -> bool {
    0.299 * c.r + 0.587 * c.g + 0.114 * c.b > 0.5
}

/// Tab-bar separators and the default divider: light bg -0.30 @ 0.26, dark bg +0.16 @ 0.36.
pub fn separator(bg: Rgba) -> Rgba {
    if luma_is_light(bg) {
        shift(bg, -0.30, 0.26)
    } else {
        shift(bg, 0.16, 0.36)
    }
}

/// Ghostty `split-divider-color` if set, else the separator colour (cmux `Workspace.swift:3963`).
pub fn divider(cfg: &Config) -> Rgba {
    cfg.split_divider_color
        .map_or_else(|| separator(rgba(cfg.background, 1.0)), |c| rgba(c, 1.0))
}

/// The overlay on unfocused panes: `unfocused-split-fill` (else the background) at alpha
/// `1 - unfocused-split-opacity`. `None` when that is fully transparent.
pub fn dim_overlay(cfg: &Config) -> Option<Rgba> {
    let alpha = 1.0 - cfg.unfocused_split_opacity.clamp(0.15, 1.0);
    (alpha > 0.0).then(|| rgba(cfg.unfocused_split_fill.unwrap_or(cfg.background), alpha))
}
```

Run: `cargo test -p cmux-app --lib chrome`. Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: Implement the workspace.** Replace `crates/cmux-app/src/workspace.rs` with:

```rust
//! The workspace: one cmux-core `Layout` drawn as absolutely positioned panes and dividers,
//! each pane showing its selected tab's terminal. All geometry comes from `Layout::rects`, so
//! layout, focus and divider drag share one maths source (plan D2).

use std::collections::HashMap;
use std::rc::Rc;

use cmux_core::{
    Closed, Direction, DividerRect, Layout, Orientation, PaneId, RESIZE_STEP, Rect, SurfaceId,
};
use gpui::{
    AppContext, ClickEvent, Context, CursorStyle, DragMoveEvent, EmptyView, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, ParentElement, Render, Rgba,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};

use crate::actions::*;
use crate::chrome;
use crate::style::TermStyle;
use crate::terminal_view::{TerminalEvent, TerminalView};

/// Hit band each side of a 1 px divider (cmux `dividerHitExpansion`).
const DIVIDER_HIT: f32 = 8.0;

/// Drag payload for a divider. Plain ids: nothing here keeps a view alive.
#[derive(Debug, Clone, Copy)]
struct DividerDrag {
    split: cmux_core::SplitId,
    orientation: Orientation,
}

struct Surface {
    view: Entity<TerminalView>,
    _subscriptions: [Subscription; 2],
}

pub struct WorkspaceView {
    layout: Layout,
    surfaces: HashMap<SurfaceId, Surface>,
    style: Rc<TermStyle>,
    focus: FocusHandle,
    window_title: String,
    _subscriptions: Vec<Subscription>,
}

impl WorkspaceView {
    pub fn new(style: Rc<TermStyle>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Nothing focused (a closed tab's handle is gone) means keys reach only the root:
        // put focus back on the active terminal (research-gpui.md §1.4).
        let lost = cx.on_focus_lost(window, |this, window, cx| this.focus_active(window, cx));
        let mut this = Self {
            layout: Layout::new(),
            surfaces: HashMap::new(),
            style,
            focus: cx.focus_handle(),
            window_title: String::new(),
            _subscriptions: vec![lost],
        };
        let first = this.layout.active_surface();
        this.add_terminal(first, None, window, cx);
        this.focus_active(window, cx);
        this
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn terminal(&self, surface: SurfaceId) -> Option<&Entity<TerminalView>> {
        self.surfaces.get(&surface).map(|s| &s.view)
    }

    fn add_terminal(
        &mut self,
        surface: SurfaceId,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let style = self.style.clone();
        let view = cx.new(|cx| TerminalView::new(style, cwd, window, cx));
        let events = cx.subscribe_in(&view, window, move |this, _, ev, window, cx| match ev {
            TerminalEvent::Exited => this.close_surface(surface, window, cx),
            TerminalEvent::TitleChanged => cx.notify(),
        });
        let handle = view.read(cx).focus_handle(cx);
        // A click focuses the terminal itself; follow it in the model.
        let focused = cx.on_focus_in(&handle, window, move |this, _, cx| {
            if this.layout.active_surface() != surface && this.layout.select_surface(surface) {
                cx.notify();
            }
        });
        self.surfaces.insert(
            surface,
            Surface {
                view,
                _subscriptions: [events, focused],
            },
        );
    }

    /// Focuses the active surface's terminal (after any change to focus or tabs).
    fn focus_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(s) = self.surfaces.get(&self.layout.active_surface()) {
            let handle = s.view.read(cx).focus_handle(cx);
            window.focus(&handle);
        }
        cx.notify();
    }

    /// The directory a new terminal starts in: the active terminal's OSC 7 cwd, if allowed.
    fn inherited_cwd(&self, inherit: bool, cx: &Context<Self>) -> Option<String> {
        let s = self.surfaces.get(&self.layout.active_surface())?;
        inherit
            .then(|| s.view.read(cx).cwd().map(str::to_string))
            .flatten()
    }

    fn container(window: &Window) -> (f32, f32, f32) {
        let size = window.viewport_size();
        (
            f32::from(size.width),
            f32::from(size.height),
            window.scale_factor(),
        )
    }

    pub fn close_surface(
        &mut self,
        surface: SurfaceId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.layout.close_surface(surface) {
            // M2 has no workspaces: the last surface closing quits, as M1 did (M3 revisits).
            Closed::LastSurface => cx.quit(),
            Closed::NotFound => {}
            Closed::Tab | Closed::Pane => {
                self.surfaces.remove(&surface);
                self.focus_active(window, cx);
            }
        }
    }

    fn split(&mut self, o: Orientation, window: &mut Window, cx: &mut Context<Self>) {
        let (w, h, scale) = Self::container(window);
        let cwd = self.inherited_cwd(self.style.cfg.split_inherit_working_directory, cx);
        // shortcut: cmux beeps on a refused split; gpui 0.2.2 has no beep API, so nothing happens.
        if let Ok(surface) = self.layout.try_split(o, w, h, scale) {
            self.add_terminal(surface, cwd, window, cx);
            self.focus_active(window, cx);
        }
    }

    fn new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self.inherited_cwd(self.style.cfg.tab_inherit_working_directory, cx);
        let surface = self.layout.new_tab();
        self.add_terminal(surface, cwd, window, cx);
        self.focus_active(window, cx);
    }

    fn focus_dir(&mut self, dir: Direction, window: &mut Window, cx: &mut Context<Self>) {
        if self.layout.focus_direction(dir) {
            self.focus_active(window, cx);
        }
    }

    fn resize(&mut self, dir: Direction, window: &mut Window, cx: &mut Context<Self>) {
        let (w, h, scale) = Self::container(window);
        if self.layout.resize_pane(dir, RESIZE_STEP, w, h, scale) {
            cx.notify();
        }
    }

    fn close_other_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for s in self.layout.close_other_tabs() {
            self.surfaces.remove(&s);
        }
        self.focus_active(window, cx);
    }

    fn on_divider_drag(
        &mut self,
        ev: &DragMoveEvent<DividerDrag>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let d = *ev.drag(cx);
        let pos = match d.orientation {
            Orientation::Horizontal => ev.event.position.x - ev.bounds.origin.x,
            Orientation::Vertical => ev.event.position.y - ev.bounds.origin.y,
        };
        let (w, h, scale) = Self::container(window);
        // shortcut: the divider jumps to the pointer (at most the 8 px band half) when the drag
        // starts; NSSplitView keeps the grab offset. Store it in DividerDrag if it bothers anyone.
        if self
            .layout
            .drag_divider(d.split, f32::from(pos), w, h, scale)
        {
            cx.notify();
        }
    }

    fn update_window_title(&mut self, window: &mut Window, cx: &Context<Self>) {
        let title = self
            .surfaces
            .get(&self.layout.active_surface())
            .and_then(|s| s.view.read(cx).title().map(str::to_string))
            .unwrap_or_else(|| "Terminal".into());
        if title != self.window_title {
            window.set_window_title(&title);
            self.window_title = title;
        }
    }

    fn render_pane(&self, pane: PaneId, r: Rect, dim: Option<Rgba>) -> impl IntoElement {
        let surface = self
            .layout
            .pane(pane)
            .map(|p| p.selected_surface())
            .and_then(|s| self.surfaces.get(&s))
            .map(|s| s.view.clone());
        div()
            .absolute()
            .left(px(r.x))
            .top(px(r.y))
            .w(px(r.w))
            .h(px(r.h))
            .overflow_hidden()
            .key_context("Pane")
            .children(surface)
            // No mouse handlers, so no hitbox: clicks go through to the terminal.
            .children(dim.map(|c| div().absolute().inset_0().bg(c)))
    }

    fn render_divider(&self, d: DividerRect, color: Rgba, cx: &Context<Self>) -> impl IntoElement {
        let r = d.rect;
        let (band, line, cursor) = match d.orientation {
            Orientation::Horizontal => (
                Rect {
                    x: r.x - DIVIDER_HIT,
                    w: r.w + 2.0 * DIVIDER_HIT,
                    ..r
                },
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(DIVIDER_HIT))
                    .w(px(r.w)),
                CursorStyle::ResizeLeftRight,
            ),
            Orientation::Vertical => (
                Rect {
                    y: r.y - DIVIDER_HIT,
                    h: r.h + 2.0 * DIVIDER_HIT,
                    ..r
                },
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(DIVIDER_HIT))
                    .h(px(r.h)),
                CursorStyle::ResizeUpDown,
            ),
        };
        let drag = DividerDrag {
            split: d.split,
            orientation: d.orientation,
        };
        div()
            .id(("divider", d.split.0 as usize))
            .absolute()
            .left(px(band.x))
            .top(px(band.y))
            .w(px(band.w))
            .h(px(band.h))
            .cursor(cursor)
            // Above the terminals: the band must win the hover test over them.
            .occlude()
            .on_drag(drag, |_, _, _, cx| cx.new(|_| EmptyView))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                // cmux equalizeSplitsOnDividerDoubleClick (default on): the run, not all splits.
                if ev.click_count() == 2 && this.layout.equalize_run(drag.split) {
                    cx.notify();
                }
            }))
            .child(line.bg(color))
    }
}

impl Focusable for WorkspaceView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.update_window_title(window, cx);
        let (w, h, scale) = Self::container(window);
        let rects = self.layout.rects(w, h, scale);
        let focused = self.layout.focused();
        let dim = chrome::dim_overlay(&self.style.cfg).filter(|_| self.layout.pane_count() > 1);
        let divider = chrome::divider(&self.style.cfg);
        let panes: Vec<_> = rects
            .panes
            .iter()
            .map(|(p, r)| self.render_pane(*p, *r, dim.filter(|_| *p != focused)))
            .collect();
        let dividers: Vec<_> = rects
            .dividers
            .iter()
            .map(|d| self.render_divider(*d, divider, cx))
            .collect();
        div()
            .id("workspace")
            .relative()
            .size_full()
            .bg(self.style.window_background())
            .key_context("Workspace")
            .track_focus(&self.focus)
            .on_drag_move(cx.listener(Self::on_divider_drag))
            .on_action(
                cx.listener(|this, _: &SplitRight, w, cx| {
                    this.split(Orientation::Horizontal, w, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &SplitDown, w, cx| this.split(Orientation::Vertical, w, cx)),
            )
            .on_action(cx.listener(|this, _: &ToggleSplitZoom, w, cx| {
                if this.layout.toggle_zoom() {
                    this.focus_active(w, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &EqualizeSplits, _, cx| {
                this.layout.equalize_all();
                cx.notify();
            }))
            .on_action(
                cx.listener(|this, _: &ResizePaneLeft, w, cx| this.resize(Direction::Left, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ResizePaneRight, w, cx| {
                    this.resize(Direction::Right, w, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &ResizePaneUp, w, cx| this.resize(Direction::Up, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ResizePaneDown, w, cx| this.resize(Direction::Down, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &FocusLeft, w, cx| this.focus_dir(Direction::Left, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &FocusRight, w, cx| this.focus_dir(Direction::Right, w, cx)),
            )
            .on_action(cx.listener(|this, _: &FocusUp, w, cx| this.focus_dir(Direction::Up, w, cx)))
            .on_action(
                cx.listener(|this, _: &FocusDown, w, cx| this.focus_dir(Direction::Down, w, cx)),
            )
            .on_action(cx.listener(|this, _: &FocusNextPane, w, cx| {
                if this.layout.focus_cycle(true) {
                    this.focus_active(w, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &FocusPreviousPane, w, cx| {
                if this.layout.focus_cycle(false) {
                    this.focus_active(w, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &NewSurface, w, cx| this.new_tab(w, cx)))
            .on_action(cx.listener(|this, _: &CloseTab, w, cx| {
                let s = this.layout.active_surface();
                this.close_surface(s, w, cx);
            }))
            .on_action(
                cx.listener(|this, _: &CloseOtherTabsInPane, w, cx| this.close_other_tabs(w, cx)),
            )
            .on_action(cx.listener(|this, _: &NextSurface, w, cx| {
                this.layout.cycle_tab(true);
                this.focus_active(w, cx);
            }))
            .on_action(cx.listener(|this, _: &PrevSurface, w, cx| {
                this.layout.cycle_tab(false);
                this.focus_active(w, cx);
            }))
            .on_action(cx.listener(|this, a: &SelectSurfaceByNumber, w, cx| {
                if this.layout.select_tab_number(usize::from(a.0)) {
                    this.focus_active(w, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &MoveSurfaceLeft, _, cx| {
                if this.layout.move_tab(-1) {
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &MoveSurfaceRight, _, cx| {
                if this.layout.move_tab(1) {
                    cx.notify();
                }
            }))
            .children(panes)
            .children(dividers)
    }
}
```

- [ ] **Step 6: Drag guard and focus-click policy.** Apply to `mouse.rs`:

```diff
diff --git a/crates/cmux-app/src/mouse.rs b/crates/cmux-app/src/mouse.rs
index 6cafb24..e80c109 100644
--- a/crates/cmux-app/src/mouse.rs
+++ b/crates/cmux-app/src/mouse.rs
@@ -118,7 +118,17 @@ impl TerminalView {
         window: &mut Window,
         cx: &mut Context<Self>,
     ) {
+        // A divider or tab drag owns the pointer (research-gpui.md §1.2 route B).
+        if cx.has_active_drag() {
+            return;
+        }
+        let was_focused = self.focus.is_focused(window);
         window.focus(&self.focus);
+        // The click that focuses another pane is not terminal input, unless the app tracks the
+        // mouse (cmux TerminalPointerFocusActivationPolicy).
+        if !was_focused && !self.tracking() {
+            return;
+        }
         if self.reporting(&ev.modifiers) {
             if let Some(b) = vt_button(ev.button) {
                 self.report(MouseAction::Press, Some(b), ev.position, &ev.modifiers);
@@ -158,6 +168,9 @@ impl TerminalView {
         _: &mut Window,
         cx: &mut Context<Self>,
     ) {
+        if cx.has_active_drag() {
+            return;
+        }
         if self.reporting(&ev.modifiers) {
             let button = ev.pressed_button.and_then(vt_button);
             self.report(MouseAction::Motion, button, ev.position, &ev.modifiers);
@@ -174,6 +187,9 @@ impl TerminalView {
     }
 
     pub(crate) fn mouse_up(&mut self, ev: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
+        if cx.has_active_drag() {
+            return;
+        }
         if self.reporting(&ev.modifiers) {
             if let Some(b) = vt_button(ev.button) {
                 self.report(MouseAction::Release, Some(b), ev.position, &ev.modifiers);
```

- [ ] **Step 7: Open the workspace.** Replace `crates/cmux-app/src/main.rs` with:

```rust
use std::rc::Rc;

use cmux_app::actions;
use cmux_app::style::{TermStyle, installed_families, load_bundled_fonts};
use cmux_app::workspace::WorkspaceView;
use cmux_config::ghostty::{Env, load};
use cmux_config::keybindings;
use gpui::{
    App, AppContext, Application, Bounds, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, px, size,
};

fn main() {
    Application::new().run(|cx: &mut App| {
        let dark = matches!(
            cx.window_appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let env = Env::from_system(dark);
        let loaded = env.as_ref().map(load).unwrap_or_default();
        let keys = match &env {
            Some(env) => keybindings::load(&env.cmux_config_file()),
            None => keybindings::keybindings(None, "config.toml"),
        };
        let bind_warnings = actions::bind(cx, &keys);
        // shortcut: warnings go to stderr; an in-window warning line is not planned before M6.
        for warning in loaded
            .warnings
            .iter()
            .chain(&keys.warnings)
            .chain(&bind_warnings)
        {
            eprintln!("cmux-linux: config: {warning}");
        }
        // Order matters: gpui caches failed font lookups, so the bundled font is registered and
        // the family picked before anything resolves a font_id.
        load_bundled_fonts(cx);
        let installed = installed_families(cx);
        let style = Rc::new(TermStyle::new(&loaded.config, &installed));
        // Translucent only when asked. KWin blur needs ext_background_effect_v1, which gpui 0.2.2
        // lacks (M0 journal); Blurred is kept so it works once gpui supports it.
        let window_background = if loaded.config.background_opacity < 1.0 {
            WindowBackgroundAppearance::Blurred
        } else {
            WindowBackgroundAppearance::Opaque
        };
        let bounds = Bounds::centered(None, size(px(1100.), px(700.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background,
                app_id: Some("cmux-linux".into()),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| WorkspaceView::new(style, window, cx)),
        )
        .expect("open window");
        cx.activate(true);
    });
}
```

- [ ] **Step 8: Run tests and lints.** Run: `cargo test --workspace --locked --exclude ghostty-vt --exclude ghostty-vt-sys` then the clippy line. Expected: cmux-app lib `test result: ok. 54 passed`; clippy clean.

- [ ] **Step 9: Run it.** `cargo run -p cmux-app` and check, fixing before committing:
  - Super+D splits right, Super+Shift+D splits down; the new pane is focused, gets a prompt in the source pane's directory (`cd /tmp` first, then split: `pwd` says `/tmp`), and the window title follows the focused pane (`printf '\e]2;hello\a'`).
  - The unfocused pane is dimmed (30 % of the background colour); with one pane nothing is dimmed.
  - Super+Alt+arrows move focus geometrically; at an edge nothing happens. Clicking an unfocused pane at a shell prompt focuses it without starting a selection; in an unfocused `htop` (mouse tracking on) the same click focuses and also reaches `htop`, as in cmux.
  - Dragging a divider resizes live and stops 100 px from either side; double-clicking a divider equalizes its row/column; Ctrl+Shift+H/J/K/L move the divider by 20 px (from the left pane of a left|right split, Ctrl+Shift+L widens it, Ctrl+Shift+H does nothing); Super+Ctrl+Shift+= equalizes all.
  - Super+Shift+Return zooms the focused pane and back; it does nothing with one pane.
  - Super+T opens a tab (the pane shows the new terminal; tabs are not drawn until Task 8), Super+Shift+] / [ and Ctrl+Tab cycle, Ctrl+1 / Ctrl+9 select, Super+W closes; `exit` in a pane closes it and its sibling takes the space; closing the last terminal quits.
  - While dragging a divider over `htop`, `htop` sees no mouse events.

- [ ] **Step 10: Format, lint, commit.**

```bash
cargo fmt -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add Cargo.toml Cargo.lock crates/cmux-app
git commit -m "feat(cmux-app): workspace with split panes, dividers, focus, zoom and resize" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Tab strip: visuals, tab mouse actions, split buttons

D6. rc §4 "Pane tab strip"; Bonsplit `Internal/Styling/{TabBarMetrics,TabBarColors}.swift`, `Internal/Views/{TabItemView,TabBarView}.swift`.

**Files:**
- Create: `crates/cmux-app/src/workspace/tab_strip.rs`, `crates/cmux-app/src/assets.rs`, `crates/cmux-app/assets/icons/{terminal,square-terminal,x,columns-2,rows-2}.svg`, `crates/cmux-app/assets/icons/LICENSE-lucide.txt`
- Modify: `crates/cmux-app/src/chrome.rs`, `crates/cmux-app/src/workspace.rs`, `crates/cmux-app/src/lib.rs`, `crates/cmux-app/src/main.rs`, `NOTICE`

**Interfaces:**
- Consumes: Task 7 `WorkspaceView` private fields and `focus_active`, `split`, `new_tab`, `close_surface`; `Layout::{select_surface, toggle_zoom, focus_pane}`.
- Produces:
  - `chrome`: `fn wcag_is_light(Rgba) -> bool`, `fn desaturate(Rgba) -> Rgba`, `fn accent(light_bar: bool) -> Rgba`, `struct TabPalette { pub bar, active_fill, hover_fill, active_text, inactive_text, separator, accent: Rgba }`, `fn tab_palette(bg: Rgba, focused: bool) -> TabPalette`
  - `assets::Assets` (`AssetSource`) serving `icons/*.svg`
  - `workspace::tab_strip::TAB_BAR_HEIGHT: f32 = 28.0`; `WorkspaceView::render_tab_strip(&self, &Pane, focused: bool, &Context<Self>)` (Task 9 adds a `strip_x` parameter)

- [ ] **Step 1: Add the icons.** Create the five SVGs (Lucide, lucide.dev; 24 × 24, `stroke="currentColor"`, which gpui renders as a mask in `text_color`). Each file is one line:

```text
crates/cmux-app/assets/icons/terminal.svg:
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/></svg>

crates/cmux-app/assets/icons/square-terminal.svg:
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 11 2-2-2-2"/><path d="M11 13h4"/><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/></svg>

crates/cmux-app/assets/icons/x.svg:
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>

crates/cmux-app/assets/icons/columns-2.svg:
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M12 3v18"/></svg>

crates/cmux-app/assets/icons/rows-2.svg:
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 12h18"/></svg>
```

Create `crates/cmux-app/assets/icons/LICENSE-lucide.txt`:

```text
ISC License

Copyright (c) for portions of Lucide are held by Cole Bemis 2013-2022 as part of Feather (MIT).
All other copyright (c) for Lucide are held by Lucide Contributors 2022.

Permission to use, copy, modify, and/or distribute this software for any purpose with or
without fee is hereby granted, provided that the above copyright notice and this permission
notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS
SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL
THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT,
NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.
```

In `NOTICE`, insert before the "Further components" paragraph:

```text
Lucide icons (crates/cmux-app/assets/icons/*.svg, https://lucide.dev)
  terminal, square-terminal, x, columns-2, rows-2. Copyright Lucide Contributors and
  Cole Bemis (Feather). ISC License; see crates/cmux-app/assets/icons/LICENSE-lucide.txt.

Bonsplit (https://github.com/manaflow-ai/bonsplit @ 58b73fd), cmux's split/tab library
  Copyright (c) 2026 Alasdair Monk. MIT. crates/cmux-core and crates/cmux-app/src/workspace*
  follow its split, focus, tab and tab-bar behaviour and metrics; no code is copied.

```

- [ ] **Step 2: Write the failing tests.** Change `crates/cmux-app/src/lib.rs` to:

```rust
//! cmux-app: the gpui app. Everything but `main` lives here so it can be unit-tested.

pub mod actions;
pub mod assets;
pub mod chrome;
pub mod colors;
pub mod ime;
pub mod keys;
pub mod mouse;
pub mod style;
pub mod terminal_element;
pub mod terminal_view;
pub mod workspace;
```

Create `crates/cmux-app/src/assets.rs` with its header and tests module only:

```rust
//! Bundled assets served to gpui (`svg().path(..)`). Icons are Lucide (ISC, see
//! assets/icons/LICENSE-lucide.txt).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_loads_and_is_svg() {
        for (path, _) in FILES {
            let bytes = Assets.load(path).unwrap().unwrap();
            assert!(bytes.starts_with(b"<svg"), "{path}");
        }
        assert!(Assets.load("icons/missing.svg").unwrap().is_none());
        assert_eq!(Assets.list("icons/").unwrap().len(), FILES.len());
    }
}
```

Apply the test half of the palette change to `chrome.rs`:

```diff
diff --git a/crates/cmux-app/src/chrome.rs b/crates/cmux-app/src/chrome.rs
index 3cf1a99..f607138 100644
--- a/crates/cmux-app/src/chrome.rs
+++ b/crates/cmux-app/src/chrome.rs
@@ -98,6 +204,77 @@ mod tests {
         ));
     }
 
+    #[test]
+    fn tab_palette_follows_bonsplit_on_dark_and_light_bars() {
+        let dark = rgba(
+            Rgb {
+                r: 0x28,
+                g: 0x2c,
+                b: 0x34,
+            },
+            1.0,
+        );
+        let p = tab_palette(dark, true);
+        assert!(close(p.active_fill, shift(dark, 0.12, 1.0)));
+        assert!(close(p.hover_fill, shift(dark, 0.07, 0.78)));
+        assert!(close(
+            p.active_text,
+            Rgba {
+                r: 1.0,
+                g: 1.0,
+                b: 1.0,
+                a: 0.82
+            }
+        ));
+        assert!(close(
+            p.inactive_text,
+            Rgba {
+                r: 1.0,
+                g: 1.0,
+                b: 1.0,
+                a: 0.68
+            }
+        ));
+        assert!(close(p.accent, accent(false)));
+        let light = rgba(
+            Rgb {
+                r: 0xfa,
+                g: 0xfa,
+                b: 0xfa,
+            },
+            1.0,
+        );
+        let p = tab_palette(light, true);
+        assert!(close(p.active_fill, shift(light, -0.065, 1.0)));
+        assert!(close(
+            p.inactive_text,
+            Rgba {
+                r: 0.0,
+                g: 0.0,
+                b: 0.0,
+                a: 0.62
+            }
+        ));
+    }
+
+    #[test]
+    fn an_unfocused_strip_is_greyscale() {
+        let p = tab_palette(
+            rgba(
+                Rgb {
+                    r: 0x28,
+                    g: 0x2c,
+                    b: 0x34,
+                },
+                1.0,
+            ),
+            false,
+        );
+        assert_eq!(p.accent.r, p.accent.g);
+        assert_eq!(p.accent.g, p.accent.b);
+        assert_eq!(p.active_fill.r, p.active_fill.b);
+    }
+
     #[test]
     fn divider_prefers_the_ghostty_colour() {
         let mut cfg = Config::default();
```

- [ ] **Step 3: Run tests to verify they fail.** Run: `cargo test -p cmux-app --lib`. Expected: compile errors (`cannot find value FILES`, `cannot find function tab_palette`).

- [ ] **Step 4: Implement assets and palette.** Insert above the tests module in `assets.rs`:

```rust
use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

const FILES: &[(&str, &[u8])] = &[
    (
        "icons/terminal.svg",
        include_bytes!("../assets/icons/terminal.svg"),
    ),
    (
        "icons/square-terminal.svg",
        include_bytes!("../assets/icons/square-terminal.svg"),
    ),
    ("icons/x.svg", include_bytes!("../assets/icons/x.svg")),
    (
        "icons/columns-2.svg",
        include_bytes!("../assets/icons/columns-2.svg"),
    ),
    (
        "icons/rows-2.svg",
        include_bytes!("../assets/icons/rows-2.svg"),
    ),
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(FILES
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(FILES
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}
```

Apply the rest of the `chrome.rs` change:

```diff
diff --git a/crates/cmux-app/src/chrome.rs b/crates/cmux-app/src/chrome.rs
index 3cf1a99..f607138 100644
--- a/crates/cmux-app/src/chrome.rs
+++ b/crates/cmux-app/src/chrome.rs
@@ -1,6 +1,7 @@
 //! Chrome colours derived from the terminal background with cmux's formulas
 //! (research-cmux.md §4): separators and dividers (`WindowChromeColorResolver.separatorColor`),
-//! the unfocused-pane dim (Ghostty `unfocused-split-*`).
+//! the unfocused-pane dim (Ghostty `unfocused-split-*`), and the pane tab strip
+//! (Bonsplit `TabBarColors.swift`).
 
 use cmux_config::ghostty::{Config, Rgb};
 use gpui::Rgba;
@@ -52,6 +53,111 @@ pub fn dim_overlay(cfg: &Config) -> Option<Rgba> {
     (alpha > 0.0).then(|| rgba(cfg.unfocused_split_fill.unwrap_or(cfg.background), alpha))
 }
 
+/// Bonsplit `isBonsplitLightColor`: WCAG relative luminance, black text contrasts more.
+pub fn wcag_is_light(c: Rgba) -> bool {
+    let lin = |v: f32| {
+        if v <= 0.03928 {
+            v / 12.92
+        } else {
+            ((v + 0.055) / 1.055).powf(2.4)
+        }
+    };
+    let l = 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
+    (l + 0.05) / 0.05 > 1.05 / (l + 0.05)
+}
+
+/// Saturation 0 (Bonsplit `bonsplitSaturating(by: 0)`): every channel becomes the Rec. 601 luma.
+pub fn desaturate(c: Rgba) -> Rgba {
+    let l = 0.299 * c.r + 0.587 * c.g + 0.114 * c.b;
+    Rgba {
+        r: l,
+        g: l,
+        b: l,
+        a: c.a,
+    }
+}
+
+/// macOS's default `controlAccentColor` (systemBlue): #007AFF in light, #0A84FF in dark
+/// appearance. cmux draws the selected-tab indicator in it; Linux has no portable accent, so
+/// we use Apple's value matched to the bar's lightness.
+pub fn accent(light_bar: bool) -> Rgba {
+    if light_bar {
+        rgba(
+            Rgb {
+                r: 0x00,
+                g: 0x7a,
+                b: 0xff,
+            },
+            1.0,
+        )
+    } else {
+        rgba(
+            Rgb {
+                r: 0x0a,
+                g: 0x84,
+                b: 0xff,
+            },
+            1.0,
+        )
+    }
+}
+
+/// Colours of one pane's tab strip.
+#[derive(Debug, Clone, Copy, PartialEq)]
+pub struct TabPalette {
+    pub bar: Rgba,
+    pub active_fill: Rgba,
+    pub hover_fill: Rgba,
+    pub active_text: Rgba,
+    pub inactive_text: Rgba,
+    pub separator: Rgba,
+    pub accent: Rgba,
+}
+
+/// The strip palette for a bar drawn on `bg`; an unfocused pane's strip is greyscale
+/// (Bonsplit `TabBarView.swift:923`).
+pub fn tab_palette(bg: Rgba, focused: bool) -> TabPalette {
+    let light = wcag_is_light(bg);
+    let white_or_black = |a: f32| {
+        let v = if light { 0.0 } else { 1.0 };
+        Rgba {
+            r: v,
+            g: v,
+            b: v,
+            a,
+        }
+    };
+    let palette = TabPalette {
+        bar: bg,
+        active_fill: if light {
+            shift(bg, -0.065, bg.a)
+        } else {
+            shift(bg, 0.12, bg.a)
+        },
+        hover_fill: if light {
+            shift(bg, -0.03, 0.78)
+        } else {
+            shift(bg, 0.07, 0.78)
+        },
+        active_text: white_or_black(0.82),
+        inactive_text: white_or_black(if light { 0.62 } else { 0.68 }),
+        separator: separator(bg),
+        accent: accent(light),
+    };
+    if focused {
+        return palette;
+    }
+    TabPalette {
+        bar: desaturate(palette.bar),
+        active_fill: desaturate(palette.active_fill),
+        hover_fill: desaturate(palette.hover_fill),
+        active_text: palette.active_text,
+        inactive_text: palette.inactive_text,
+        separator: desaturate(palette.separator),
+        accent: desaturate(palette.accent),
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
```

Run: `cargo test -p cmux-app --lib`. Expected: `test result: ok. 57 passed`.

- [ ] **Step 5: Implement the strip.** Create `crates/cmux-app/src/workspace/tab_strip.rs`:

```rust
//! A pane's tab strip, sized and coloured like Bonsplit's tab bar as cmux configures it
//! (research-cmux.md §4 "Pane tab strip"; numbers from `TabBarMetrics.swift`, logical px).

use cmux_core::{Orientation, Pane, PaneId};
use gpui::{
    ClickEvent, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
    svg,
};

use super::WorkspaceView;
use crate::chrome::{self, TabPalette};

/// cmux `sharedChromeBarHeight`.
pub const TAB_BAR_HEIGHT: f32 = 28.0;
const TAB_MIN_W: f32 = 48.0;
const TAB_MAX_W: f32 = 220.0;
const TAB_PAD: f32 = 6.0;
const ICON_SLOT: f32 = 14.0;
/// The terminal glyph is drawn at max(10, 14 - 2.5) (`TabItemView.swift:975`).
const ICON_GLYPH: f32 = 11.5;
const CONTENT_GAP: f32 = 6.0;
const TITLE_PX: f32 = 11.0;
const CLOSE_SLOT: f32 = 16.0;
const CLOSE_GLYPH: f32 = 9.0;
const INDICATOR_H: f32 = 1.5;
const LANE_LEAD: f32 = 6.0;
const LANE_TRAIL: f32 = 8.0;
const BUTTON_W: f32 = 22.0;
const BUTTON_GAP: f32 = 4.0;
const BUTTON_GLYPH: f32 = 12.0;

/// The right-hand split-action buttons (cmux's default set minus the browser).
#[derive(Debug, Clone, Copy)]
enum StripButton {
    NewTerminal,
    SplitRight,
    SplitDown,
}

impl WorkspaceView {
    pub(super) fn palette(&self, focused: bool) -> TabPalette {
        let cfg = &self.style.cfg;
        chrome::tab_palette(
            chrome::rgba(cfg.background, cfg.background_opacity),
            focused,
        )
    }

    fn tab_title(&self, surface: cmux_core::SurfaceId, cx: &Context<Self>) -> SharedString {
        self.surfaces
            .get(&surface)
            .and_then(|s| {
                s.view
                    .read(cx)
                    .title()
                    .map(|t| SharedString::from(t.to_string()))
            })
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "Terminal".into())
    }

    pub(super) fn render_tab_strip(
        &self,
        pane: &Pane,
        focused: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let p = self.palette(focused);
        let pane_id = pane.id;
        let tabs: Vec<_> = pane
            .tabs
            .iter()
            .enumerate()
            .map(|(ix, s)| self.render_tab(*s, ix == pane.selected, p, cx))
            .collect();
        let buttons = [
            (StripButton::NewTerminal, "icons/square-terminal.svg"),
            (StripButton::SplitRight, "icons/columns-2.svg"),
            (StripButton::SplitDown, "icons/rows-2.svg"),
        ]
        .map(|(b, icon)| {
            div()
                .id(SharedString::from(format!("strip-{}-{:?}", pane_id.0, b)))
                .flex()
                .items_center()
                .justify_center()
                .w(px(BUTTON_W))
                .h_full()
                .child(
                    svg()
                        .path(icon)
                        .size(px(BUTTON_GLYPH))
                        .text_color(p.inactive_text),
                )
                .active(|s| s.opacity(0.72))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.strip_button(pane_id, b, window, cx)
                }))
        });
        div()
            .flex()
            .flex_row()
            .flex_none()
            .w_full()
            .h(px(TAB_BAR_HEIGHT))
            .bg(p.bar)
            .child(
                div()
                    .id(SharedString::from(format!("tabs-{}", pane_id.0)))
                    .flex()
                    .flex_row()
                    .flex_shrink()
                    .min_w(px(0.))
                    .h_full()
                    .overflow_x_scroll()
                    .children(tabs),
            )
            .child(
                // Empty strip area: the bottom separator continues; double-click opens a tab.
                div()
                    .id(SharedString::from(format!("strip-empty-{}", pane_id.0)))
                    .flex_1()
                    .h_full()
                    .border_b_1()
                    .border_color(p.separator)
                    .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                        if ev.click_count() == 2 {
                            this.layout.focus_pane(pane_id);
                            this.new_tab(window, cx);
                        } else if this.layout.focus_pane(pane_id) {
                            this.focus_active(window, cx);
                        }
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_none()
                    .items_center()
                    .gap(px(BUTTON_GAP))
                    .pl(px(LANE_LEAD))
                    .pr(px(LANE_TRAIL))
                    .h_full()
                    .border_b_1()
                    .border_color(p.separator)
                    .children(buttons),
            )
    }

    fn render_tab(
        &self,
        surface: cmux_core::SurfaceId,
        selected: bool,
        p: TabPalette,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let group = SharedString::from(format!("tab-{}", surface.0));
        let text = if selected {
            p.active_text
        } else {
            p.inactive_text
        };
        let close_text = if selected {
            p.inactive_text
        } else {
            gpui::transparent_black().into()
        };
        let close = div()
            .id(SharedString::from(format!("tab-close-{}", surface.0)))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .ml(px(CONTENT_GAP))
            .size(px(CLOSE_SLOT))
            .rounded_full()
            .text_color(close_text)
            // Shown on the selected or hovered tab; brighter with a hover circle over the button.
            .group_hover(group.clone(), |s| s.text_color(p.inactive_text))
            .hover(|s| s.bg(p.hover_fill).text_color(p.active_text))
            .child(
                svg()
                    .path("icons/x.svg")
                    .size(px(CLOSE_GLYPH))
                    .text_color(close_text),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.close_surface(surface, window, cx);
            }));
        div()
            .id(SharedString::from(format!("tab-{}", surface.0)))
            .group(group)
            .relative()
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .h_full()
            .min_w(px(TAB_MIN_W))
            .max_w(px(TAB_MAX_W))
            .px(px(TAB_PAD))
            .border_r_1()
            .border_color(p.separator)
            .when(selected, |d| d.bg(p.active_fill))
            // The selected tab merges with the content below: no bottom separator under it.
            .when(!selected, |d| d.border_b_1().hover(|s| s.bg(p.hover_fill)))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(ICON_SLOT))
                    .child(
                        svg()
                            .path("icons/terminal.svg")
                            .size(px(ICON_GLYPH))
                            .text_color(text),
                    ),
            )
            .child(
                div()
                    .ml(px(CONTENT_GAP))
                    .flex_shrink()
                    .min_w(px(0.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(TITLE_PX))
                    .text_color(text)
                    .child(self.tab_title(surface, cx)),
            )
            .child(close)
            .when(selected, |d| {
                d.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right(px(1.))
                        .h(px(INDICATOR_H))
                        .bg(p.accent),
                )
            })
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                this.layout.select_surface(surface);
                // Double-click on a tab toggles pane zoom (`TabItemView.swift:424`).
                if ev.click_count() == 2 {
                    this.layout.toggle_zoom();
                }
                this.focus_active(window, cx);
            }))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    this.close_surface(surface, window, cx)
                }),
            )
    }

    fn strip_button(
        &mut self,
        pane: PaneId,
        b: StripButton,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.layout.focus_pane(pane);
        match b {
            StripButton::NewTerminal => self.new_tab(window, cx),
            StripButton::SplitRight => self.split(Orientation::Horizontal, window, cx),
            StripButton::SplitDown => self.split(Orientation::Vertical, window, cx),
        }
    }
}
```

Apply to `workspace.rs` (declares the module and puts the strip above each pane's terminal):

```diff
diff --git a/crates/cmux-app/src/workspace.rs b/crates/cmux-app/src/workspace.rs
index 36b6999..7fa440f 100644
--- a/crates/cmux-app/src/workspace.rs
+++ b/crates/cmux-app/src/workspace.rs
@@ -14,6 +14,8 @@ use gpui::{
     StatefulInteractiveElement, Styled, Subscription, Window, div, px,
 };
 
+mod tab_strip;
+
 use crate::actions::*;
 use crate::chrome;
 use crate::style::TermStyle;
@@ -213,24 +215,39 @@ impl WorkspaceView {
         }
     }
 
-    fn render_pane(&self, pane: PaneId, r: Rect, dim: Option<Rgba>) -> impl IntoElement {
-        let surface = self
-            .layout
-            .pane(pane)
-            .map(|p| p.selected_surface())
-            .and_then(|s| self.surfaces.get(&s))
+    fn render_pane(
+        &self,
+        pane: PaneId,
+        r: Rect,
+        dim: Option<Rgba>,
+        cx: &Context<Self>,
+    ) -> impl IntoElement + use<> {
+        let pane = self.layout.pane(pane).expect("rects come from the layout");
+        let terminal = self
+            .surfaces
+            .get(&pane.selected_surface())
             .map(|s| s.view.clone());
+        let focused = pane.id == self.layout.focused();
         div()
             .absolute()
             .left(px(r.x))
             .top(px(r.y))
             .w(px(r.w))
             .h(px(r.h))
+            .flex()
+            .flex_col()
             .overflow_hidden()
             .key_context("Pane")
-            .children(surface)
-            // No mouse handlers, so no hitbox: clicks go through to the terminal.
-            .children(dim.map(|c| div().absolute().inset_0().bg(c)))
+            .child(self.render_tab_strip(pane, focused, cx))
+            .child(
+                div()
+                    .relative()
+                    .flex_1()
+                    .min_h(px(0.))
+                    .children(terminal)
+                    // No mouse handlers, so no hitbox: clicks go through to the terminal.
+                    .children(dim.map(|c| div().absolute().inset_0().bg(c))),
+            )
     }
 
     fn render_divider(&self, d: DividerRect, color: Rgba, cx: &Context<Self>) -> impl IntoElement {
@@ -307,7 +324,7 @@ impl Render for WorkspaceView {
         let panes: Vec<_> = rects
             .panes
             .iter()
-            .map(|(p, r)| self.render_pane(*p, *r, dim.filter(|_| *p != focused)))
+            .map(|(p, r)| self.render_pane(*p, *r, dim.filter(|_| *p != focused), cx))
             .collect();
         let dividers: Vec<_> = rects
             .dividers
```

- [ ] **Step 6: Serve the assets.** Replace `crates/cmux-app/src/main.rs` with:

```rust
use std::rc::Rc;

use cmux_app::actions;
use cmux_app::assets::Assets;
use cmux_app::style::{TermStyle, installed_families, load_bundled_fonts};
use cmux_app::workspace::WorkspaceView;
use cmux_config::ghostty::{Env, load};
use cmux_config::keybindings;
use gpui::{
    App, AppContext, Application, Bounds, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, px, size,
};

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        let dark = matches!(
            cx.window_appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let env = Env::from_system(dark);
        let loaded = env.as_ref().map(load).unwrap_or_default();
        let keys = match &env {
            Some(env) => keybindings::load(&env.cmux_config_file()),
            None => keybindings::keybindings(None, "config.toml"),
        };
        let bind_warnings = actions::bind(cx, &keys);
        // shortcut: warnings go to stderr; an in-window warning line is not planned before M6.
        for warning in loaded
            .warnings
            .iter()
            .chain(&keys.warnings)
            .chain(&bind_warnings)
        {
            eprintln!("cmux-linux: config: {warning}");
        }
        // Order matters: gpui caches failed font lookups, so the bundled font is registered and
        // the family picked before anything resolves a font_id.
        load_bundled_fonts(cx);
        let installed = installed_families(cx);
        let style = Rc::new(TermStyle::new(&loaded.config, &installed));
        // Translucent only when asked. KWin blur needs ext_background_effect_v1, which gpui 0.2.2
        // lacks (M0 journal); Blurred is kept so it works once gpui supports it.
        let window_background = if loaded.config.background_opacity < 1.0 {
            WindowBackgroundAppearance::Blurred
        } else {
            WindowBackgroundAppearance::Opaque
        };
        let bounds = Bounds::centered(None, size(px(1100.), px(700.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background,
                app_id: Some("cmux-linux".into()),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| WorkspaceView::new(style, window, cx)),
        )
        .expect("open window");
        cx.activate(true);
    });
}
```

- [ ] **Step 7: Run tests and lints.** Run: `cargo test -p cmux-app --locked` and the clippy line. Expected: `ok. 57 passed`; clippy clean.

- [ ] **Step 8: Run it.** `cargo run -p cmux-app` and check against rc §4, fixing before committing:
  - Every pane has a 28 px strip, also with one tab; the selected tab has the lighter fill, a 1.5 px blue line on top and no bottom line; other tabs show a bottom line and a 1 px right line; hovering a tab lightens it and shows its ×.
  - Titles are 11 px, truncate with … beyond 220 px, and tabs never get narrower than 48 px; the strip scrolls sideways when full.
  - Clicking a tab selects it and focuses its pane; double-clicking a tab zooms; middle-click or × closes; double-clicking empty strip space opens a tab in that pane; the three buttons open a terminal tab, split right, split down for that pane.
  - The unfocused pane's strip is grey (its accent line too).

- [ ] **Step 9: Format, lint, commit.**

```bash
cargo fmt -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add NOTICE crates/cmux-app
git commit -m "feat(cmux-app): pane tab strips with cmux metrics, colours and split buttons" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Tab drag: reorder within a pane and move to another pane

D6. rc §4 "Drag and drop of tabs" (insertion by mid-x, 2 × 20 bar, last tab out closes the pane); rg §1.3.

**Files:**
- Modify: `crates/cmux-app/src/workspace/tab_strip.rs`, `crates/cmux-app/src/workspace.rs`

**Interfaces:**
- Consumes: `Layout::move_surface` (Task 3), Task 8 strip.
- Produces: `tab_strip::insertion_index(mid_xs: &[f32], x: f32) -> usize`; `DraggedTab` (ids + title + palette, `impl Render` as the ghost); `WorkspaceView` fields `tab_bounds: Rc<RefCell<HashMap<PaneId, Vec<Bounds<Pixels>>>>>`, `drop_target: Option<(PaneId, usize)>`; `render_tab_strip(&self, &Pane, focused, strip_x: f32, &Context<Self>)`.

- [ ] **Step 1: Write the failing test.** Append to the end of `tab_strip.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_index_is_before_the_first_tab_whose_middle_is_right_of_the_pointer() {
        let mids = [30.0, 90.0, 150.0];
        assert_eq!(insertion_index(&mids, 0.0), 0);
        assert_eq!(insertion_index(&mids, 31.0), 1);
        assert_eq!(insertion_index(&mids, 120.0), 2);
        assert_eq!(insertion_index(&mids, 400.0), 3);
        assert_eq!(insertion_index(&[], 10.0), 0);
    }
}
```

- [ ] **Step 2: Run test to verify it fails.** Run: `cargo test -p cmux-app --lib tab_strip`. Expected: compile error (`cannot find function insertion_index`).

- [ ] **Step 3: Implement.** Apply (the test hunk from Step 1 is left out):

```diff
diff --git a/crates/cmux-app/src/workspace/tab_strip.rs b/crates/cmux-app/src/workspace/tab_strip.rs
index 1c73729..35a3b20 100644
--- a/crates/cmux-app/src/workspace/tab_strip.rs
+++ b/crates/cmux-app/src/workspace/tab_strip.rs
@@ -1,11 +1,11 @@
 //! A pane's tab strip, sized and coloured like Bonsplit's tab bar as cmux configures it
 //! (research-cmux.md §4 "Pane tab strip"; numbers from `TabBarMetrics.swift`, logical px).
 
-use cmux_core::{Orientation, Pane, PaneId};
+use cmux_core::{Orientation, Pane, PaneId, SurfaceId};
 use gpui::{
-    ClickEvent, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
-    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
-    svg,
+    ClickEvent, Context, DragMoveEvent, InteractiveElement, IntoElement, MouseButton,
+    MouseDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled,
+    Window, div, prelude::*, px, svg,
 };
 
 use super::WorkspaceView;
@@ -29,6 +29,42 @@ const LANE_TRAIL: f32 = 8.0;
 const BUTTON_W: f32 = 22.0;
 const BUTTON_GAP: f32 = 4.0;
 const BUTTON_GLYPH: f32 = 12.0;
+/// Tab drop insertion bar (`TabBarMetrics.dropIndicatorWidth/Height`).
+const DROP_BAR_W: f32 = 2.0;
+const DROP_BAR_H: f32 = 20.0;
+
+/// A tab being dragged; also the ghost drawn under the pointer. Ids only, so a drag never
+/// keeps a closed pane or terminal alive.
+#[derive(Clone)]
+pub(super) struct DraggedTab {
+    surface: SurfaceId,
+    title: SharedString,
+    palette: TabPalette,
+}
+
+impl Render for DraggedTab {
+    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
+        div()
+            .flex()
+            .items_center()
+            .h(px(TAB_BAR_HEIGHT))
+            .max_w(px(TAB_MAX_W))
+            .px(px(TAB_PAD))
+            .bg(self.palette.active_fill)
+            .text_size(px(TITLE_PX))
+            .text_color(self.palette.active_text)
+            .whitespace_nowrap()
+            .overflow_hidden()
+            .text_ellipsis()
+            .child(self.title.clone())
+    }
+}
+
+/// Bonsplit `TabDropIndexResolver`: before the first tab whose midpoint is right of `x`,
+/// else after the last.
+pub fn insertion_index(mid_xs: &[f32], x: f32) -> usize {
+    mid_xs.iter().position(|m| *m > x).unwrap_or(mid_xs.len())
+}
 
 /// The right-hand split-action buttons (cmux's default set minus the browser).
 #[derive(Debug, Clone, Copy)]
@@ -47,7 +83,7 @@ impl WorkspaceView {
         )
     }
 
-    fn tab_title(&self, surface: cmux_core::SurfaceId, cx: &Context<Self>) -> SharedString {
+    fn tab_title(&self, surface: SurfaceId, cx: &Context<Self>) -> SharedString {
         self.surfaces
             .get(&surface)
             .and_then(|s| {
@@ -64,10 +100,21 @@ impl WorkspaceView {
         &self,
         pane: &Pane,
         focused: bool,
+        strip_x: f32,
         cx: &Context<Self>,
     ) -> impl IntoElement + use<> {
         let p = self.palette(focused);
         let pane_id = pane.id;
+        let bounds = self.tab_bounds.clone();
+        let drop_bar = self.drop_bar_x(pane_id, strip_x).map(|x| {
+            div()
+                .absolute()
+                .left(px(x - DROP_BAR_W / 2.0))
+                .top(px((TAB_BAR_HEIGHT - DROP_BAR_H) / 2.0))
+                .w(px(DROP_BAR_W))
+                .h(px(DROP_BAR_H))
+                .bg(p.accent)
+        });
         let tabs: Vec<_> = pane
             .tabs
             .iter()
@@ -99,14 +146,27 @@ impl WorkspaceView {
                 }))
         });
         div()
+            .relative()
             .flex()
             .flex_row()
             .flex_none()
             .w_full()
             .h(px(TAB_BAR_HEIGHT))
             .bg(p.bar)
+            .on_drag_move(
+                cx.listener(move |this, ev: &DragMoveEvent<DraggedTab>, _, cx| {
+                    this.tab_drag_moved(pane_id, ev, cx)
+                }),
+            )
+            .on_drop(cx.listener(move |this, d: &DraggedTab, window, cx| {
+                this.drop_tab(pane_id, d.surface, window, cx)
+            }))
             .child(
                 div()
+                    // Tab bounds for drop-index maths (they are content-sized).
+                    .on_children_prepainted(move |b, _, _| {
+                        bounds.borrow_mut().insert(pane_id, b);
+                    })
                     .id(SharedString::from(format!("tabs-{}", pane_id.0)))
                     .flex()
                     .flex_row()
@@ -147,11 +207,69 @@ impl WorkspaceView {
                     .border_color(p.separator)
                     .children(buttons),
             )
+            .children(drop_bar)
+    }
+
+    /// Where the insertion bar goes while a tab is dragged over this strip (strip-relative x).
+    fn drop_bar_x(&self, pane: PaneId, strip_x: f32) -> Option<f32> {
+        let (target, index) = self.drop_target?;
+        if target != pane {
+            return None;
+        }
+        let bounds = self.tab_bounds.borrow();
+        let tabs = bounds.get(&pane)?;
+        let x = match tabs.get(index) {
+            Some(b) => b.origin.x,
+            None => tabs.last()?.right(),
+        };
+        Some(f32::from(x) - strip_x)
+    }
+
+    fn tab_drag_moved(
+        &mut self,
+        pane: PaneId,
+        ev: &DragMoveEvent<DraggedTab>,
+        cx: &mut Context<Self>,
+    ) {
+        // Every strip hears every move; only the one under the pointer claims the drop.
+        let inside = ev.bounds.contains(&ev.event.position);
+        let target = if inside {
+            let mids: Vec<f32> = self
+                .tab_bounds
+                .borrow()
+                .get(&pane)
+                .map(|b| b.iter().map(|b| f32::from(b.center().x)).collect())
+                .unwrap_or_default();
+            Some((pane, insertion_index(&mids, f32::from(ev.event.position.x))))
+        } else if self.drop_target.is_some_and(|(p, _)| p == pane) {
+            None
+        } else {
+            return;
+        };
+        if self.drop_target != target {
+            self.drop_target = target;
+            cx.notify();
+        }
+    }
+
+    fn drop_tab(
+        &mut self,
+        pane: PaneId,
+        surface: SurfaceId,
+        window: &mut Window,
+        cx: &mut Context<Self>,
+    ) {
+        let index = match self.drop_target.take() {
+            Some((p, ix)) if p == pane => ix,
+            _ => self.layout.pane(pane).map_or(0, |p| p.tabs.len()),
+        };
+        self.layout.move_surface(surface, pane, index);
+        self.focus_active(window, cx);
     }
 
     fn render_tab(
         &self,
-        surface: cmux_core::SurfaceId,
+        surface: SurfaceId,
         selected: bool,
         p: TabPalette,
         cx: &Context<Self>,
@@ -167,6 +285,11 @@ impl WorkspaceView {
         } else {
             gpui::transparent_black().into()
         };
+        let ghost = DraggedTab {
+            surface,
+            title: self.tab_title(surface, cx),
+            palette: p,
+        };
         let close = div()
             .id(SharedString::from(format!("tab-close-{}", surface.0)))
             .flex()
@@ -253,6 +376,7 @@ impl WorkspaceView {
                 }
                 this.focus_active(window, cx);
             }))
+            .on_drag(ghost, |tab, _, _, cx| cx.new(|_| tab.clone()))
             .on_mouse_down(
                 MouseButton::Middle,
                 cx.listener(move |this, _: &MouseDownEvent, window, cx| {
```

and:

```diff
diff --git a/crates/cmux-app/src/workspace.rs b/crates/cmux-app/src/workspace.rs
index 7fa440f..53d5925 100644
--- a/crates/cmux-app/src/workspace.rs
+++ b/crates/cmux-app/src/workspace.rs
@@ -2,6 +2,7 @@
 //! each pane showing its selected tab's terminal. All geometry comes from `Layout::rects`, so
 //! layout, focus and divider drag share one maths source (plan D2).
 
+use std::cell::RefCell;
 use std::collections::HashMap;
 use std::rc::Rc;
 
@@ -9,8 +10,8 @@ use cmux_core::{
     Closed, Direction, DividerRect, Layout, Orientation, PaneId, RESIZE_STEP, Rect, SurfaceId,
 };
 use gpui::{
-    AppContext, ClickEvent, Context, CursorStyle, DragMoveEvent, EmptyView, Entity, FocusHandle,
-    Focusable, InteractiveElement, IntoElement, ParentElement, Render, Rgba,
+    AppContext, Bounds, ClickEvent, Context, CursorStyle, DragMoveEvent, EmptyView, Entity,
+    FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Pixels, Render, Rgba,
     StatefulInteractiveElement, Styled, Subscription, Window, div, px,
 };
 
@@ -42,6 +43,10 @@ pub struct WorkspaceView {
     style: Rc<TermStyle>,
     focus: FocusHandle,
     window_title: String,
+    /// Each pane's tab bounds from the last prepaint (window coordinates).
+    tab_bounds: Rc<RefCell<HashMap<PaneId, Vec<Bounds<Pixels>>>>>,
+    /// Where a dragged tab would land: pane and insertion index.
+    drop_target: Option<(PaneId, usize)>,
     _subscriptions: Vec<Subscription>,
 }
 
@@ -56,6 +61,8 @@ impl WorkspaceView {
             style,
             focus: cx.focus_handle(),
             window_title: String::new(),
+            tab_bounds: Rc::default(),
+            drop_target: None,
             _subscriptions: vec![lost],
         };
         let first = this.layout.active_surface();
@@ -238,7 +245,7 @@ impl WorkspaceView {
             .flex_col()
             .overflow_hidden()
             .key_context("Pane")
-            .child(self.render_tab_strip(pane, focused, cx))
+            .child(self.render_tab_strip(pane, focused, r.x, cx))
             .child(
                 div()
                     .relative()
@@ -316,6 +323,10 @@ impl Focusable for WorkspaceView {
 impl Render for WorkspaceView {
     fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
         self.update_window_title(window, cx);
+        // A drag released outside every strip leaves no drop target behind.
+        if !cx.has_active_drag() {
+            self.drop_target = None;
+        }
         let (w, h, scale) = Self::container(window);
         let rects = self.layout.rects(w, h, scale);
         let focused = self.layout.focused();
```

- [ ] **Step 4: Run tests and lints.** Run: `cargo test -p cmux-app --locked` and the clippy line. Expected: `ok. 58 passed`; clippy clean.

- [ ] **Step 5: Run it.** `cargo run -p cmux-app`. Check: dragging a tab shows a ghost under the pointer and a 2 × 20 blue bar where it will land; dropping reorders; dropping on itself or just right of itself changes nothing; dragging onto another pane's strip moves it there (that pane gets focus); dragging a pane's only tab to another pane closes the empty pane; releasing elsewhere cancels; a plain click still selects (the drag starts only after 2 px).

- [ ] **Step 6: Format, lint, commit.**

```bash
cargo fmt -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-app
git commit -m "feat(cmux-app): drag tabs to reorder and between panes" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: Paste protection

D7 (STATUS carry: "unsafe multi-line paste prompt"). vendor/ghostty `src/input/paste.zig` `isSafe`, `src/Surface.zig` ~8160-8185, `include/ghostty/vt/paste.h`.

**Files:**
- Modify: `crates/cmux-term/src/input.rs`, `crates/cmux-app/src/terminal_view.rs`, `crates/cmux-app/src/mouse.rs`

**Interfaces:**
- Consumes: `Config::clipboard_paste_protection` (Task 5).
- Produces: `cmux_term::input::paste_needs_confirmation(text: &str, bracketed: bool) -> bool`; `TerminalView::paste_text(&mut self, text: String, &mut Window, &mut Context<Self>)` (was `paste_text(&mut self, &str)`; `paste(cx)` removed).

- [ ] **Step 1: Write the failing test.** In `crates/cmux-term/src/input.rs`, add at the top of the tests module (after `use super::*;`):

```rust
    #[test]
    fn multi_line_pastes_need_confirmation_unless_bracketed() {
        assert!(!paste_needs_confirmation("ls -la", false));
        assert!(paste_needs_confirmation("rm -rf ~\n", false));
        assert!(!paste_needs_confirmation("line1\nline2", true));
        assert!(paste_needs_confirmation("a\x1b[201~b", true));
        assert!(paste_needs_confirmation("a\x1b[201~b", false));
    }
```

- [ ] **Step 2: Run test to verify it fails.** Run: `cargo test -p cmux-term --lib input`. Expected: compile error (`cannot find function paste_needs_confirmation`).

- [ ] **Step 3: Implement the check.** Insert above `#[cfg(test)]` in `input.rs`:

```rust
/// Whether pasting `text` needs the user's OK under Ghostty's `clipboard-paste-protection`
/// (Surface.zig `completeClipboardPaste`): a newline or a bracketed-paste end
/// (`ghostty_paste_is_safe`), except that a bracketed paste without an embedded end marker is
/// trusted (`clipboard-paste-bracketed-safe`, default true).
pub fn paste_needs_confirmation(text: &str, bracketed: bool) -> bool {
    if bracketed && !text.contains("\x1b[201~") {
        return false;
    }
    // SAFETY: `text` is a live &str; pointer and length describe exactly its bytes.
    !unsafe { sys::ghostty_paste_is_safe(text.as_ptr().cast(), text.len()) }
}
```

Run: `cargo test -p cmux-term --lib`. Expected: `test result: ok. 36 passed`.

- [ ] **Step 4: Prompt before an unsafe paste.** Apply:

```diff
diff --git a/crates/cmux-app/src/mouse.rs b/crates/cmux-app/src/mouse.rs
index e80c109..3a90a0a 100644
--- a/crates/cmux-app/src/mouse.rs
+++ b/crates/cmux-app/src/mouse.rs
@@ -155,7 +155,7 @@ impl TerminalView {
             #[cfg(any(target_os = "linux", target_os = "freebsd"))]
             MouseButton::Middle => {
                 if let Some(text) = cx.read_from_primary().and_then(|item| item.text()) {
-                    self.paste_text(&text);
+                    self.paste_text(text, window, cx);
                 }
             }
             _ => {}
diff --git a/crates/cmux-app/src/terminal_view.rs b/crates/cmux-app/src/terminal_view.rs
index fde47b8..120e71b 100644
--- a/crates/cmux-app/src/terminal_view.rs
+++ b/crates/cmux-app/src/terminal_view.rs
@@ -9,15 +9,15 @@ use std::time::{Duration, Instant};
 
 use cmux_term::TermEvent;
 use cmux_term::ghostty_vt::{KeyAction, KeyEncoder, MouseEncoders};
-use cmux_term::input::{encode_focus, encode_paste};
+use cmux_term::input::{encode_focus, encode_paste, paste_needs_confirmation};
 use cmux_term::pty::{PtyEvent, Session};
 use cmux_term::selection::Selection;
 use futures::StreamExt;
 use futures::channel::mpsc;
 use gpui::{
     App, ClipboardItem, Context, EventEmitter, FocusHandle, Focusable, IntoElement, KeyDownEvent,
-    KeyUpEvent, Keystroke, MouseButton, ParentElement, Pixels, Point, Render, Styled, Subscription,
-    Task, Window, div, prelude::*, px,
+    KeyUpEvent, Keystroke, MouseButton, ParentElement, Pixels, Point, PromptLevel, Render, Styled,
+    Subscription, Task, Window, div, prelude::*, px,
 };
 
 use crate::actions::{Copy, Paste};
@@ -362,8 +362,10 @@ impl TerminalView {
         self.copy(cx);
     }
 
-    fn paste_action(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
-        self.paste(cx);
+    fn paste_action(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
+        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
+            self.paste_text(text, window, cx);
+        }
     }
 
     pub(crate) fn copy(&mut self, cx: &mut Context<Self>) {
@@ -377,15 +379,31 @@ impl TerminalView {
         }
     }
 
-    fn paste(&mut self, cx: &mut Context<Self>) {
-        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
-            self.paste_text(&text);
-        }
-    }
-
-    pub(crate) fn paste_text(&mut self, text: &str) {
+    /// Pastes `text`, first asking when Ghostty's paste protection says it is unsafe.
+    pub(crate) fn paste_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
         let bracketed = self.session.term.lock().unwrap().vt().mode(2004, false);
-        self.send_input(encode_paste(text, bracketed));
+        if !self.style.cfg.clipboard_paste_protection || !paste_needs_confirmation(&text, bracketed)
+        {
+            return self.send_input(encode_paste(&text, bracketed));
+        }
+        // gpui draws its own prompt on Linux (platform prompt is None, window/prompts.rs).
+        let answer = window.prompt(
+            PromptLevel::Warning,
+            "Paste text that may run commands?",
+            Some("The text contains a newline, so pasting it may run commands in the shell."),
+            &["Paste", "Cancel"],
+            cx,
+        );
+        cx.spawn_in(window, async move |this, cx| {
+            if answer.await == Ok(0) {
+                let _ = this.update(cx, |view, _| {
+                    // Bracketed paste may have changed while the prompt was up.
+                    let bracketed = view.session.term.lock().unwrap().vt().mode(2004, false);
+                    view.send_input(encode_paste(&text, bracketed));
+                });
+            }
+        })
+        .detach();
     }
 }
 
```

- [ ] **Step 5: Run tests and lints.** Run: `cargo test --workspace --locked --exclude ghostty-vt --exclude ghostty-vt-sys` and the clippy line. Expected: every suite passes (cmux-core 32, cmux-config 41, cmux-term lib 36 + pty 9 + spike 1, cmux-app 58); clippy clean.

- [ ] **Step 6: Run it.** `cargo run -p cmux-app`. Copy `echo one` + newline + `echo two` from another app. In a plain shell prompt with bracketed paste off (`printf '\e[?2004l'`), Super+V shows a prompt; Cancel pastes nothing, Paste pastes both lines. With bracketed paste on (a default zsh/fish/bash ≥ 5.1 prompt, or `printf '\e[?2004h'`), the same paste goes in without a prompt. A one-line paste never prompts. Middle-click PRIMARY follows the same rule. With `clipboard-paste-protection = false` in the Ghostty config, no prompt ever.

- [ ] **Step 7: Format, lint, commit.**

```bash
cargo fmt -p cmux-term -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-term crates/cmux-app
git commit -m "feat: confirm unsafe multi-line pastes (Ghostty clipboard-paste-protection)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: Exit-criteria verification (user, at the desktop)

Manual. Build release: `cargo build --release -p cmux-app`, run `target/release/cmux-app`. Screenshots: `mkdir -p ~/Pictures/cmux-m2` once, then `spectacle -b -n -a -d 1500 -o ~/Pictures/cmux-m2/<name>.png`. Record every result in the journal (Task 12 Step 1). There is no Mac at hand: the visual reference is cmux's published screenshots (`docs/assets/vertical-horizontal-tabs-and-splits.png` and `docs/assets/main-first-image.png` in github.com/manaflow-ai/cmux at 21dcc2b, also on the README) and the numbers in rc §4; say so in the journal.

**Files:** none tracked. Code changes only if a check fails (fix, re-run that check, commit with a `fix(m2):` message).

- [ ] **Step 1: Shortcut table (exit criterion 1).** With two or three panes and a few tabs, press each default and tick it in the journal. Expected behaviour in parentheses.

| Action | Chord | Expected |
|---|---|---|
| splitRight / splitDown | Super+D / Super+Shift+D | new pane right / below, focused, 50 % |
| toggleSplitZoom | Super+Shift+Return | zoom toggles; no-op with one pane |
| equalizeSplits | Super+Ctrl+Shift+= | all dividers by leaf count |
| resizePane{Left,Right,Up,Down} | Ctrl+Shift+H/L/K/J | divider moves 20 px per press by side rule |
| focus{Left,Right,Up,Down} | Super+Alt+arrows | geometric, no wrap, MRU wins |
| newSurface | Super+T | tab after the selected one, same cwd |
| closeTab | Super+W | closes tab; empty pane closes; last quits |
| closeOtherTabsInPane | Super+Alt+T | only the selected tab stays |
| nextSurface / prevSurface | Super+Shift+] / [, Ctrl+Tab / Ctrl+Shift+Tab | cycle with wrap |
| selectSurfaceByNumber | Ctrl+1..9 | Nth tab, 9 = last |
| moveSurfaceLeft / Right | Super+Shift+Alt+[ / ] | reorder, no wrap |
| copy / paste | Super+C/V, Ctrl+Shift+C/V | as M1; Ctrl+Shift+C with no selection reaches the shell |

For any chord that does nothing, check whether KWin took it: System Settings → Keyboard → Shortcuts, search for the chord (Plasma spells Super "Meta"), or `grep -i 'meta+' ~/.config/kglobalshortcutsrc`. Record each Plasma 6 *default* shortcut that intercepts a chord from the table as "Meta+X: <Plasma action>" in the journal (no machine-specific details, no custom user shortcuts). Do not change the defaults to dodge them; remapping is the answer (Step 2).

- [ ] **Step 2: Remapped binding (exit criterion 2).** Run against a scratch config, never the real one:

```bash
SCRATCH="$(mktemp -d)"
cp -r "${XDG_CONFIG_HOME:-$HOME/.config}/ghostty" "$SCRATCH/" 2>/dev/null || true
mkdir -p "$SCRATCH/cmux-linux"
printf '[keybindings]\nsplitRight = "ctrl+alt+d"\nfocusLeft = ["super+opt+left", "ctrl+alt+h"]\ncloseTab = "none"\nbogus = "super+x"\n' > "$SCRATCH/cmux-linux/config.toml"
XDG_CONFIG_HOME="$SCRATCH" target/release/cmux-app
rm -rf "$SCRATCH"
```

Expected: stderr has one `cmux-linux: config: …config.toml: keybindings: unknown action \`bogus\`` line; Ctrl+Alt+D splits right and Super+D does not; Ctrl+Alt+H and Super+Alt+Left both focus left; Super+W does nothing. No code changed.

- [ ] **Step 3: Spacing and divider look (exit criterion 3).** Make a 2 × 2 layout with two tabs in one pane and a long title in another (`printf '\e]2;%s\a' "a very long title that should truncate at two hundred and twenty pixels"`). Screenshot `cmux-linux-splits.png`. Open cmux's `vertical-horizontal-tabs-and-splits.png` next to it (save it as `cmux-mac-splits.png` in the same folder). Compare and record: strip height (28 × scale device px; measure in an image viewer), tab padding and icon/title/× spacing, selected-tab fill and accent line, separators (right edge per tab, bottom except under the selected tab), 1 px divider colour against the background, dimming of unfocused panes, button lane on the right. Note every visible difference; fix any that contradict rc §4 numbers.

- [ ] **Step 4: Behaviour spot checks.** Each one line in the journal:
  - `setsid -f sleep 300; exit` closes that pane at once (Task 1).
  - A tab whose shell exits closes; closing the last one quits.
  - New split/tab after `cd /tmp` starts in `/tmp`; with `split-inherit-working-directory = false` in a scratch Ghostty config it starts in `$HOME`.
  - `unfocused-split-opacity = 1` turns dimming off; `split-divider-color = #ff0000` paints dividers red.
  - Paste protection prompt (Task 10 Step 6) appears and both buttons work with mouse and keyboard.
  - Tab drag (Task 9 Step 5).
  - With 6+ panes, typing and divider drags stay smooth (subjective; note it).

- [ ] **Step 5 (optional, M1 carry): M1 exit criteria 2–3.** If time allows, do M1 plan Task 12 Steps 2 and 5 (Ghostty screenshot pair, `CMUX_TRACE_LATENCY=1`) and record them under M1 in the M2 journal entry.

---

### Task 12: Close-out

**Files:**
- Modify: `docs/milestones/m2-splits-tabs/journal.md`, `docs/status-archive.md`, `STATUS.md`, `docs/design.md`

- [ ] **Step 1: Journal entry.** Append to `docs/milestones/m2-splits-tabs/journal.md`, filling every `<...>` from Tasks 1–11:

```markdown
## <YYYY-MM-DD> — M2 complete

- PTY: reader polls master + wake socket + pidfd; `setsid -f` grandchild no longer keeps a pane or leaks the reader; feed panics close the tab.
- Model: cmux-core binary split tree (Bonsplit semantics); admission 160 × 100; resize 20 px by side; equalize by span count; MRU directional focus.
- Keybindings: cmux defaults with Super; `[keybindings]` in config.toml; shifted symbols mapped to US shifted keys; selectSurfaceByNumber binds 1–9.
- Exit 1: shortcut table <all ok | exceptions>. Plasma 6 defaults that intercept table chords: <Meta+…: action, …>.
- Exit 2: remapped splitRight to ctrl+alt+d without code changes: <ok>.
- Exit 3: cmux-linux-splits.png vs cmux-mac-splits.png (cmux's published screenshot; no Mac available): <match | differences>.
- Deviations from cmux: focus moves unzoom (Ghostty rule); last surface quits (no workspaces yet); no beep on refused split/resize; Ctrl+Tab remappable; accent fixed to systemBlue; chrome font = gpui UI font; divider grab offset not kept. <others found>
- Deferred: edge-drop-to-split, focus-follows-mouse, tab context menu/rename/pin, tooltips, live config reload, in-window warnings.
```

- [ ] **Step 2: design.md.** In the Architecture table, change the `cmux-config` row's "Depends on" cell from `—` to `` `toml` ``.

- [ ] **Step 3: Archive and rewrite STATUS.md.** First append the current file to the archive:

```bash
{ printf '\n## %s — M2 complete\n\n' "$(date +%F)"; cat STATUS.md; } >> docs/status-archive.md
```

Then edit `STATUS.md`: Updated date; Current milestone `M3 workspaces-sidebar — not started`; Next action `write docs/milestones/m3-workspaces-sidebar/plan.md`; M2 row `done (PR #<n>)`; Decisions: add one line each for cmux-core as the pure model, contextless gpui actions bound from `config.toml`, pidfd-based shell exit. Code map: add `cmux-core` (tree, geometry) and the new cmux-app modules (`actions`, `chrome`, `workspace`, `workspace/tab_strip`, `assets`). Replace "Carry to M2+" with "Carry to M3+": the container size must come from the workspace area once the sidebar exists (`WorkspaceView::container`); last-surface quit → close workspace (rc §1); Deferred items from this plan that M3 owns; the macOS pidfd replacement (M6); any open item from Task 11. Remove the M1 carry items this plan closed (PTY reader, prepaint split, keybinding remap, Ctrl+Shift+C passthrough, paste prompt, feed panic). Run `scripts/check-status-lines.sh`. Expected: ≤100 lines.

- [ ] **Step 4: Commit, PR, merge.**

```bash
git add docs STATUS.md
git commit -m "docs: close M2" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push -u origin m2-splits-tabs
gh pr create --base master --head m2-splits-tabs --title "M2 splits-tabs" --body "$(printf 'Split panes and per-pane tabs like cmux: binary split tree in cmux-core, draggable dividers, tab strips, geometric focus, zoom, cmux shortcut table with Super, remappable in ~/.config/cmux-linux/config.toml.\n\nExit criteria and screenshots: docs/milestones/m2-splits-tabs/journal.md.\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)')"
gh pr checks --watch
gh pr merge --merge
```

Expected: CI green, PR merged to `master`. Then put the PR number into STATUS.md's M2 row on master if it was not known before (`git switch master && git pull`, edit, commit `docs: record M2 PR number`, push).

---

## Exit criteria and carry-list check

| Requirement (spec.md / STATUS carry list) | Task |
|---|---|
| Split tree in cmux-core (H/V, ratios, close/collapse), unit-tested | 3, 4 |
| Pane tab strip; new/close/reorder tabs | 7 (keys), 8 (strip, mouse), 9 (drag) |
| Focus follows click and shortcuts | 7 (click policy, directional, cycle), 8 (tab/strip click) |
| Draggable dividers; focus movement between panes | 7 |
| Keybinding system in cmux-config: cmux defaults with Super, remappable in `~/.config/cmux-linux/config.toml` | 5, 6 |
| Exit 1: split, tab, focus, resize shortcuts match cmux's table | 5 (table), 6 (binding), 7 (handlers), 11 Step 1 |
| Exit 2: a remapped binding takes effect without code changes | 5, 6, 11 Step 2 |
| Exit 3: spacing and divider look match the Mac app, screenshot pair in journal | 8, 11 Step 3, 12 |
| Carry: PTY reader blocks until EOF (setsid grandchild) — poll + wake | 1 (plus pidfd) |
| Carry: callbacks hold weak handles | 6–9 (`cx.listener`, `subscribe_in`, `on_focus_in` are weak; drag payloads hold ids) |
| Carry: split prepaint before adding panes | 2 |
| Carry: bindings config-remappable (keys.rs BINDINGS hard-coded) | 5, 6 |
| Carry: Ctrl+Shift+C with no selection passes through | 6 |
| Carry: unsafe multi-line paste prompt | 10 |
| Carry (M6, pulled in): panic in `feed` without `Exited` | 1 |

## research UNVERIFIED items

| Item | Handled in |
|---|---|
| Wayland pointer events while a button is held outside the window (route A) | Not needed: route B (gpui drag) is used; Task 7 Step 9 checks a drag that leaves the window. |
| taffy rounding of fractional flex sizes | Not needed: panes are absolutely positioned from `Layout::rects`, which snaps divider positions to device pixels. |
| `min_w_0` / `min_h_0` names | Not used; `min_w(px(0.))` / `min_h(px(0.))`. |
| Contextless binding outranks contextual ones | All bindings contextless (Decisions). |
| Bonsplit zoom vs directional focus (rc §2) | Focus moves unzoom (Ghostty rule), recorded as a deviation. |
| `app.focusPaneOnFirstClick` (rc §2) | Default (false) behaviour implemented: the focusing click is not terminal input. |
| Accent from cmux `app.accentColor` (rc §4) | systemBlue constant (Decisions). |
| `resolvedSplitDividerColor` fallback (rc §4) | Not used; separator colour, as cmux's Bonsplit divider does. |
