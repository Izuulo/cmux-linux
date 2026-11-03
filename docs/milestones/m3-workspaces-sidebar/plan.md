# M3 workspaces-sidebar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Multiple workspaces in a cmux-style vertical sidebar. Each row shows the workspace title, per-directory git branch and cwd, the PR, and listening ports. The rows update within ~2 s, and nothing runs on the UI thread. Super+1–9, next/prev, new, close, rename and reorder behave like cmux. On KWin 6.8 the window is blurred through `ext_background_effect_v1`.

**Architecture:**
- `cmux-core` gains `Workspaces`, a pure ordered list of workspace ids, custom names and the selection, with cmux's insert/close/cycle/number/move rules.
- `cmux-meta` gets its first code: pure collectors for git branch (`.git/HEAD`), display paths, listening ports (`/proc` by controlling TTY) and PRs (`gh pr list --json`, with cmux's selection rules and cadence). Each is unit-tested against fixtures.
- `cmux-app` gets a new root `WindowView`. It owns `Workspaces`, one `WorkspaceView` per workspace (the M2 view, now one per workspace), the sidebar, and two background poll tasks: metadata every 1.5 s, PRs on cmux's 10 s/60 s cadence. On Linux, a ~60-line Wayland shim sets a blur region on gpui's own surface.

**Tech Stack:**
- Rust 2024 (toolchain 1.99.0), gpui `=0.2.2`, libghostty-vt (vendored), portable-pty 0.9, libc 0.2.
- serde_json 1 (already in `Cargo.lock`).
- Linux only: wayland-client 0.31, wayland-backend 0.3 (`client_system`), wayland-protocols 0.32.9+ (`client`, `staging`), raw-window-handle 0.6. All are already in `Cargo.lock`.
- Lucide icons (ISC). The `gh` CLI at runtime, optional.

**Spec:** `docs/milestones/m3-workspaces-sidebar/spec.md`. Research: `research-cmux.md` (cited `rc §n`) and `research-gpui.md` (`rg §n`), both in the same folder.

## Global Constraints

Carried from M0–M2. Every task's requirements include these:
- Licence: GPL-3.0-or-later for our crates. Every added dependency, vendored file or asset goes in `NOTICE` with its licence. Never copy from cmux's BUSL-1.1 directories.
- Zig must be exactly 0.16.0 (`scripts/install-zig.sh`). Run `export PATH="$PWD/.tools/zig:$PATH"` from the repo root before any cargo command.
- Pass `--locked` to build, test and clippy.
- Keep libc pinned at 0.2.189 in `Cargo.lock`. Never run a bare `cargo update`, and adding a dependency must not move libc. Check with `grep -A1 'name = "libc"' Cargo.lock`.
- Keep gpui pinned at `=0.2.2` and the toolchain at `rust-toolchain.toml` (1.99.0).
- No personal info in tracked files: names, emails, home paths with usernames, machine details, or the user's config contents.
- `STATUS.md` is max 100 lines. Append it to `docs/status-archive.md` before replacing it.
- Before each commit, run `cargo fmt -p <crates you touched>` and `cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings`.
- Dependency rules:
  - `cmux-core` depends on nothing.
  - `cmux-meta` depends only on std and `serde_json`. It needs no `cmux-core` (design.md lists one; Task 12 corrects the table).
  - `cmux-term` and `cmux-config` never depend on gpui.
- Keep Linux-only code (`/proc`, pidfd, Wayland) behind `cfg(target_os = "linux")`, or make it degrade to "no data" elsewhere.
- UI sizes are cmux points used as logical pixels (1 pt = 1 px).

New for M3:
- Work happens on branch `m3-workspaces-sidebar`, which is based on `m2-splits-tabs`. If M2 has merged by execution time, rebase onto `master` first. It merges at Task 12.
- **One commit per task step that changes files.** The owner wants a long history. Every commit message ends with a blank line and the attribution lines the session is given (at least `Co-Authored-By: <model> <noreply@anthropic.com>`).
- **Unlike M2, this plan's code was not compiled before it was written.** It was written on a host without the Linux toolchain. Signatures are exact, but the executor compiles each task and may adjust gpui call details. Any name a later task consumes must still match the task's **Interfaces** block; record any change in the journal.
- Behaviour and numbers come from rc §n and gpui patterns from rg §n. If something is unclear, read the cmux Swift file named there and match it. Record deviations in the journal.
- Screenshots go to `~/Pictures/cmux-m3/`, outside the repo. The journal names them by file name only.

## Review Focus

These are the inputs most likely to break for a real user that no feature test would exercise by default. Each has its test in the named task.
1. **Hidden workspaces stealing focus.** Every `WorkspaceView` registers `on_focus_lost` (M2), so a hidden workspace could pull focus into its invisible terminal. Only the active workspace may refocus. Task 7 adds an `active` flag; Task 8 Step 6 checks it at the desktop.
2. **`/proc` races and permissions.** A pid can vanish mid-scan, other users' `fd` dirs are unreadable, and `comm` can contain spaces and `)`. The scan skips each of these silently and still reports the other ports. Test: Task 4 `scan_skips_vanished_and_unreadable_processes`, `tty_nr_survives_parens_in_comm`.
3. **`gh` missing, logged out, offline or hanging.** The sidebar shows no PR. The UI never waits on `gh`: it runs on the background executor with a 15 s kill deadline. Failures back off up to 15 min, and a missing binary disables PR polling for the session. Test: Task 5 `fetch_without_gh_binary_is_not_found`, `failures_back_off`.
4. **Paths that look like home.** `/home/al` is not `~` when home is `/home/alice`. Trailing slashes and `/` must be handled. Test: Task 3 `display_dir_*`.
5. **Repos that aren't plain.** Detached HEAD, a worktree's `.git` *file*, `/` with no repo, and a `HEAD` with CRLF all give a correct branch or none, never a panic. Test: Task 3 `branch_*`.

## Decisions made in this plan

| Decision | Why |
|---|---|
| **D1 Model.** `cmux_core::Workspaces` holds only order, ids, custom names and the selection. Each workspace's `Layout` stays in its `WorkspaceView`. | rc §1 rules need unit tests; moving `Layout` out of `WorkspaceView` would churn all of M2. The spec's "workspaces in cmux-core" is the list rules. |
| New workspace goes **after the selected one** and is selected. Double-click on empty sidebar space puts it **at the end**. | rc §1 `afterCurrent` default; pinning is out of scope. |
| New workspace cwd is the selected workspace's active terminal cwd, else `$HOME`. | rc §1, with `app.workspaceInheritWorkingDirectory` hard-wired true. Ghostty `working-directory` and `window-inherit-working-directory` are not parsed yet (Deferred). |
| **Close** asks "Close workspace?" / "This will close the workspace and all of its panels." with Close/Cancel when any of its shells has a foreground job other than itself. It never asks otherwise. | rc §1 `commandRunning`. On Linux, "running" = the PTY's foreground process group ≠ the shell's pid (`tcgetpgrp`). There is no shell integration yet. |
| Closing the **last** workspace (shortcut, or its last surface exiting) quits the app. | rc §1 closes the window, and we have one window. |
| Selection after close: the workspace that slides into the slot, else the new last one. Closing an unselected workspace keeps the selection. | rc §1 `CloseSelection`. |
| `selectWorkspaceByNumber`: 1–8 pick that index if it exists, 9 picks the last. Out of range does nothing, but the key is still consumed. | rc §2. Bindings are consumed by gpui anyway. |
| next/prev **wrap**; moveWorkspaceUp/Down move ±1 **without** wrap. | rc §2. |
| Rename is **inline**: double-click a row, or Super+Shift+R on the selected row (shows the sidebar if hidden). Enter or focus loss commits, Esc cancels. Typing first replaces the whole name ("preselected"). Empty or whitespace clears the custom name. | rc §1. cmux's Super+Shift+R opens a command-palette prompt; there is no palette before M11, so this deviation is recorded. |
| The rename field is a minimal key-driven input: chars, Backspace, Enter, Escape, a caret at the end. No IME, selection or mouse. | rg §2 option (a). `shortcut:` comment; upgrade to the trimmed gpui `input.rs` if CJK names matter. |
| Title = custom name, else the focused surface's OSC title, else "Terminal". | rc §1. Spinner stripping is deferred: we have no agent spinners yet. |
| **D2 Metadata.** A row shows one line per unique directory across all its panels (tree order), as `branch · dir`, plus one PR line per unique PR URL, plus one ports line. | rc §3 row contents 5–7 and rc §4 "all panels". |
| Git branch comes from reading `.git/HEAD`. A `.git` *file* means `gitdir:` is followed. Only `ref: refs/heads/<b>` gives a branch; detached HEAD gives none. It is re-read every 1.5 s: no file watching, no `git` spawn. | rc §4 (a) is exactly this; rg §3 says polling a few bytes meets "~2 s". The dirty `*` marker needs `git status` and is deferred. |
| Directory = OSC 7 cwd, else `/proc/<pid>/cwd` (M2's `TerminalView::cwd`). Shown as `~` + rest when under `$HOME`, else in full, tail-truncated by the layout. A directory equal to the title is shown once (in the title only). | rc §3 directory shortening (`pathLastSegmentOnly` false), `droppingTitleEcho`. |
| Ports: every process whose `tty_nr` equals a panel shell's `tty_nr`, with its TCP/TCP6 sockets in state `0A`, sorted ascending and deduped. Rescanned every 1.5 s in the same background pass. | rc §4 matches by controlling TTY. We poll because there are no shell `ports_kick`s yet; scanning `/proc` costs ~ms. |
| PR: `gh pr list --head <branch> --state all --json number,state,url,updatedAt,mergedAt,headRefName --limit 20`, run in the panel's directory. | The spec says "via gh". cmux calls GitHub's REST API with `gh auth token` (rc §4); `gh` resolves the repo and auth itself and needs no HTTP client. Recorded deviation. |
| PR selection and cadence follow cmux: keep `headRefName == branch`; open > merged > closed, then latest `updatedAt`, then highest number; merged > 14 days ago → none. Cadence 10 s for the selected workspace's branches, 60 s otherwise, 15 min once merged or closed. Failures back off 60 s × 2ⁿ (max 15 min) and keep the last result, drawn at 0.5 opacity. ≤ 3 fetches per tick, one at a time. `main`/`master` are never looked up. | rc §4. Jitter, ETags and `report_pr_action` hints are deferred (no shell integration). |
| **D3 Sidebar.** Width default 240, min 240, max 600, and the terminal area keeps ≥ 300. Resize band reaches 6 into the sidebar and 4 into the content. Visible by default; Super+B toggles it. Width and visibility are not persisted. | rc §3 widths; persistence is M10. |
| Sidebar background = the terminal background at `background-opacity` (`TermStyle::window_background`), with a 1 px trailing border in the M2 separator colour. | rc §3 `matchTerminalBackground` default true. |
| Top band 28 px with two 20 px buttons (icon 12, spacing 6, radius 6) at its right edge, inset 6: toggle sidebar (Lucide `panel-left`) and new workspace (Lucide `plus`). First row starts at y = 30. | rc §3 titlebar controls. KDE draws its own title bar, so the band lives in the sidebar. There is no mouse way back once hidden (Super+B only); `shortcut:` comment. |
| Row: outer padding h 6, inner h 10 / v 8, radius 6, 2 px between rows, 4 px between sections. Title 12.5 semibold (UI font), 1 line, tail-truncated. Detail lines 10 px in the terminal font family. | rc §3 row metrics. |
| Colours: selected row = solid cmux accent `#0088FF` on light bg / `#0091FF` on dark. Selected title white 1.0, details white 0.75. Unselected: no fill; title = black/white 0.85, details 0.5 (light) / 0.55 (dark). Hover: no fill, reveals the × (9 px, only when > 1 workspace). | rc §3 selection. White passes the 2.75 contrast rule on both accents, so the dark-text fallback is not implemented. Unselected alphas approximate the macOS label colours (UNVERIFIED in rc). |
| PR line: icon (Lucide `git-pull-request` open, `git-merge` merged, `circle-x` closed), then `PR #N` underlined (click → `cx.open_url`), then the state word `open`/`merged`/`closed`. 10 px semibold, detail colour, not state-coloured. | rc §3 row content 6. |
| Ports line: `:3000` items, underlined, gap 4. Click opens `http://localhost:3000`. | rc §3 row content 7. Tooltips are deferred (gpui tooltips need a view). |
| Drag to reorder: gpui `on_drag` per row with a ghost, gap = first row whose vertical midpoint is below the pointer, a 2 px accent line inset 8 centred in the gap, dragged row at 0.6 opacity. Drop → `Workspaces::move_to`. | rc §1 reorder, rg §2. |
| **D4 Blur.** `cmux-app/src/blur.rs` (Linux) binds `ext_background_effect_manager_v1` on a foreign-display `Connection` and sets a blur region covering the **whole surface** (compositor clips it). Only when Ghostty `background-blur` is on and `background-opacity < 1`. The window is `Transparent` when opacity < 1, else `Opaque`. | rg §1.3 (don't upgrade gpui), rg §1.4. The sidebar shares the terminal backdrop (rc §3), so blur is window-wide like Ghostty/cmux. Without `background-blur` nothing is blurred, as in Ghostty. Plasma ≤ 6.6 (kde-blur only) loses blur: `shortcut:` comment. |
| `background-blur` parses Ghostty's `true`/`false`/number; on = `true` or a number > 0. | vendor/ghostty `Config.zig` "background-blur". |
| **D5 Background work.** `WindowView` runs one `cx.spawn` loop (inputs read on the UI thread as plain data → `background_executor().spawn(collect)` → apply if changed) and a second PR loop ticking every 2 s. Both stop when the view is gone. | rg §3 poll pattern. Exit criterion 3. |
| **D6 Keys.** New cmux rawValues with defaults: `newTab` super+n, `closeWorkspace` super+shift+w, `renameWorkspace` super+shift+r, `selectWorkspaceByNumber` super+1 (binds 1–9), `nextSidebarTab` super+ctrl+], `prevSidebarTab` super+ctrl+[, `moveWorkspaceUp` super+ctrl+opt+[, `moveWorkspaceDown` super+ctrl+opt+], `toggleSidebar` super+b. | rc §2. Plasma default conflicts (e.g. Meta+1–9 task-manager entries) are recorded in Task 11; remapping is the answer, as in M2. |

## Deferred (with reason)

- Persisting sidebar width, visibility and workspace order: M10.
- Ghostty `working-directory`, `window-inherit-working-directory`, cmux `app.newWorkspacePlacement` variants, pinned workspaces, groups, colours, descriptions: not in the M3 spec, or M8.
- Git dirty `*` (needs a bounded `git status`), branch icon, compact layout, `pathLastSegmentOnly`, the fitting-candidate path shortening, spinner stripping.
- PR ETags, jitter, `report_pr_action` command hints, GitHub REST: no shell integration before M5. `gh` is enough for the spec.
- Shell-integration kicks for ports and git (`ports_kick`, `report_git_branch`): M5 socket.
- Super-held number hints (`⌘1` badges), tooltips, sidebar context menu, focusSidebar, reopenClosedWorkspace, the Cmd+P switcher (M11).
- Notification text and badges: M4.
- Blur on Plasma ≤ 6.6 (kde-blur) and on macOS: M6.

## File structure

| Path | Responsibility |
|---|---|
| `crates/cmux-core/src/workspaces.rs` | `WorkspaceId`, `Workspace`, `Workspaces`, `WorkspaceClosed` |
| `crates/cmux-config/src/keybindings.rs` | + workspace defaults; numbered handling for two actions |
| `crates/cmux-config/src/ghostty/mod.rs` | + `background_blur` |
| `crates/cmux-meta/src/lib.rs` | `PanelInput`, `WorkspaceInput`, `DirLine`, `WorkspaceMeta`, `collect` |
| `crates/cmux-meta/src/git.rs` | `find_git_dir`, `parse_head`, `branch` |
| `crates/cmux-meta/src/path.rs` | `display_dir` |
| `crates/cmux-meta/src/ports.rs` | `/proc` parsing and `listening_ports` |
| `crates/cmux-meta/src/pr.rs` | `gh` JSON parsing, `select_pr`, `fetch`, `PrPoller` |
| `crates/cmux-term/src/pty.rs` | + `Session::foreground_busy` |
| `crates/cmux-app/src/actions.rs` | + workspace actions |
| `crates/cmux-app/src/terminal_view.rs` | + `pid`, `busy` |
| `crates/cmux-app/src/workspace.rs` | `WorkspaceEvent`, inset, active flag, panel inputs, title, busy; no `cx.quit` |
| `crates/cmux-app/src/window_view.rs` | Root view: workspaces, actions, close prompt, poll loops, layout |
| `crates/cmux-app/src/window_view/sidebar.rs` | Sidebar render: band, rows, resize, reorder drag |
| `crates/cmux-app/src/window_view/rename.rs` | `RenameInput` |
| `crates/cmux-app/src/chrome.rs` | + `SidebarPalette`, `sidebar_palette`, `cmux_accent` |
| `crates/cmux-app/src/blur.rs` | Linux `WindowBlur` shim |
| `crates/cmux-app/assets/icons/` | + `plus`, `panel-left`, `git-pull-request`, `git-merge`, `circle-x` |
| `crates/cmux-app/src/main.rs` | Open `WindowView`; window background; blur |

---

### Task 1: cmux-core `Workspaces`

D1. rc §1, §2.

**Files:**
- Create: `crates/cmux-core/src/workspaces.rs`
- Modify: `crates/cmux-core/src/lib.rs` (add `mod workspaces; pub use workspaces::*;`, doc line → "M3: … and the workspace list")

**Interfaces — Produces:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspaceId(pub u64);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace { pub id: WorkspaceId, pub custom_title: Option<String> }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceClosed { Closed, Last, NotFound }
pub struct Workspaces { /* list: Vec<Workspace>, selected: usize, next_id: u64 */ }
impl Workspaces {
    pub fn new() -> Self;                                   // one workspace, selected (also Default)
    pub fn list(&self) -> &[Workspace];
    pub fn len(&self) -> usize;
    pub fn selected(&self) -> WorkspaceId;
    pub fn selected_index(&self) -> usize;
    pub fn index_of(&self, id: WorkspaceId) -> Option<usize>;
    pub fn add_after_selected(&mut self) -> WorkspaceId;    // inserted at selected+1, selected
    pub fn add_at_end(&mut self) -> WorkspaceId;            // appended, selected
    pub fn close(&mut self, id: WorkspaceId) -> WorkspaceClosed; // Last leaves the list unchanged
    pub fn select(&mut self, id: WorkspaceId) -> bool;      // false if unknown or already selected
    pub fn select_number(&mut self, n: usize) -> bool;      // 1..=8 → index n-1, 9 → last
    pub fn cycle(&mut self, forward: bool) -> bool;         // wraps; false with one workspace
    pub fn move_selected(&mut self, delta: isize) -> bool;  // no wrap
    pub fn move_to(&mut self, id: WorkspaceId, gap: usize) -> bool; // gap 0..=len in current order
    pub fn rename(&mut self, id: WorkspaceId, name: &str) -> bool;  // trimmed; empty → None
    pub fn custom_title(&self, id: WorkspaceId) -> Option<&str>;
}
```
Ids are never reused. `move_to` returns false and changes nothing when `gap` is `index` or `index + 1`.

- [x] **Step 1: Write the failing tests** in `workspaces.rs` `#[cfg(test)] mod tests`:

```rust
fn ids(w: &Workspaces) -> Vec<u64> { w.list().iter().map(|x| x.id.0).collect() }

#[test]
fn new_has_one_selected_workspace() {
    let w = Workspaces::new();
    assert_eq!(w.len(), 1);
    assert_eq!(w.selected(), w.list()[0].id);
}

#[test]
fn add_after_selected_inserts_next_and_selects() {
    let mut w = Workspaces::new();              // [a]
    let b = w.add_after_selected();             // [a b]
    w.select(w.list()[0].id);
    let c = w.add_after_selected();             // [a c b]
    assert_eq!(ids(&w), vec![w.list()[0].id.0, c.0, b.0]);
    assert_eq!(w.selected(), c);
    let d = w.add_at_end();
    assert_eq!(*ids(&w).last().unwrap(), d.0);
    assert_eq!(w.selected(), d);
}

#[test]
fn close_selects_the_slot_then_the_last() {
    let mut w = Workspaces::new();
    let a = w.selected();
    let b = w.add_at_end();
    let c = w.add_at_end();
    w.select(b);
    assert_eq!(w.close(b), WorkspaceClosed::Closed);
    assert_eq!(w.selected(), c, "c slid into b's slot");
    assert_eq!(w.close(c), WorkspaceClosed::Closed);
    assert_eq!(w.selected(), a, "no slot left: new last");
    assert_eq!(w.close(a), WorkspaceClosed::Last);
    assert_eq!(w.len(), 1);
    assert_eq!(w.close(WorkspaceId(999)), WorkspaceClosed::NotFound);
}

#[test]
fn closing_an_unselected_workspace_keeps_the_selection() {
    let mut w = Workspaces::new();
    let a = w.selected();
    let b = w.add_at_end();
    w.select(a);
    w.close(b);
    assert_eq!(w.selected(), a);
    let c = w.add_at_end();          // [a c], c selected
    w.close(a);
    assert_eq!(w.selected(), c);
    assert_eq!(w.selected_index(), 0);
}

#[test]
fn select_number_nine_is_last_and_out_of_range_is_noop() {
    let mut w = Workspaces::new();
    let a = w.selected();
    w.add_at_end();
    let c = w.add_at_end();
    assert!(w.select_number(1));
    assert_eq!(w.selected(), a);
    assert!(w.select_number(9));
    assert_eq!(w.selected(), c);
    assert!(!w.select_number(5));
    assert_eq!(w.selected(), c);
    assert!(!w.select_number(0));
}

#[test]
fn cycle_wraps_and_move_does_not() {
    let mut w = Workspaces::new();
    let a = w.selected();
    let b = w.add_at_end();
    assert!(w.cycle(true));
    assert_eq!(w.selected(), a, "b → wraps to a");
    assert!(w.cycle(false));
    assert_eq!(w.selected(), b);
    assert!(!w.move_selected(1), "b is last");
    assert!(w.move_selected(-1));
    assert_eq!(ids(&w), vec![b.0, a.0]);
    let mut one = Workspaces::new();
    assert!(!one.cycle(true));
}

#[test]
fn move_to_uses_gaps_in_current_order() {
    let mut w = Workspaces::new();
    let a = w.selected();
    let b = w.add_at_end();
    let c = w.add_at_end();
    assert!(!w.move_to(a, 0));
    assert!(!w.move_to(a, 1));
    assert!(w.move_to(a, 2));
    assert_eq!(ids(&w), vec![b.0, a.0, c.0]);
    assert!(w.move_to(c, 0));
    assert_eq!(ids(&w), vec![c.0, b.0, a.0]);
    assert!(w.move_to(c, 3));
    assert_eq!(ids(&w), vec![b.0, a.0, c.0]);
    assert_eq!(w.selected(), c, "moving keeps the selected id");
}

#[test]
fn rename_trims_and_empty_clears() {
    let mut w = Workspaces::new();
    let a = w.selected();
    assert!(w.rename(a, "  api  "));
    assert_eq!(w.custom_title(a), Some("api"));
    assert!(w.rename(a, "   "));
    assert_eq!(w.custom_title(a), None);
    assert!(!w.rename(WorkspaceId(42), "x"));
}

#[test]
fn ids_are_never_reused() {
    let mut w = Workspaces::new();
    let b = w.add_at_end();
    w.close(b);
    assert_ne!(w.add_at_end(), b);
}
```

- [x] **Step 2: Run them.** `cargo test -p cmux-core workspaces`. Expected: compile errors (`Workspaces` not found).
- [x] **Step 3: Implement** the Interfaces block in `workspaces.rs`. The id counter starts at 1.
- [x] **Step 4: Run them.** `cargo test -p cmux-core`. Expected: all M2 tests plus 9 new ones pass.
- [x] **Step 5: Commit.** `cargo fmt -p cmux-core`, clippy line, then `git add crates/cmux-core && git commit -m "feat(cmux-core): workspace list with cmux insert, close, cycle, number, move and rename rules"`.

---

### Task 2: cmux-config: workspace keybindings and `background-blur`

D4, D6. rc §2.

**Files:**
- Modify: `crates/cmux-config/src/keybindings.rs`
- Modify: `crates/cmux-config/src/ghostty/mod.rs`

**Interfaces — Produces:**
- `DEFAULTS` gains, in this order after `moveSurfaceRight`: `("newTab", &["super+n"])`, `("closeWorkspace", &["super+shift+w"])`, `("renameWorkspace", &["super+shift+r"])`, `("selectWorkspaceByNumber", &["super+1"])`, `("nextSidebarTab", &["super+ctrl+]"])`, `("prevSidebarTab", &["super+ctrl+["])`, `("moveWorkspaceUp", &["super+ctrl+opt+["])`, `("moveWorkspaceDown", &["super+ctrl+opt+]"])`, `("toggleSidebar", &["super+b"])`.
- `const NUMBERED: &str` becomes `const NUMBERED: &[&str] = &["selectSurfaceByNumber", "selectWorkspaceByNumber"];`. Every `action == NUMBERED` becomes `NUMBERED.contains(&action)`.
- `Config.background_blur: bool`, default `false`.

- [x] **Step 1: Failing tests.** In `keybindings.rs` tests:

```rust
#[test]
fn workspace_defaults_bind_like_cmux() {
    let k = keybindings(None, "config.toml");
    let chord = |a: &str| k.bindings.iter().filter(|b| b.action == a).map(|b| b.keystroke.clone()).collect::<Vec<_>>();
    assert_eq!(chord("newTab"), vec!["super-n"]);
    assert_eq!(chord("closeWorkspace"), vec!["super-shift-w"]);
    assert_eq!(chord("nextSidebarTab"), vec!["super-ctrl-]"]);
    assert_eq!(chord("moveWorkspaceDown"), vec!["super-ctrl-alt-]"]);
    assert_eq!(chord("toggleSidebar"), vec!["super-b"]);
    let numbered: Vec<_> = k.bindings.iter().filter(|b| b.action == "selectWorkspaceByNumber")
        .map(|b| (b.keystroke.as_str(), b.number)).collect();
    assert_eq!(numbered.len(), 9);
    assert_eq!(numbered[0], ("super-1", Some(1)));
    assert_eq!(numbered[8], ("super-9", Some(9)));
}

#[test]
fn workspace_number_remap_keeps_digit_family() {
    let k = keybindings(Some("[keybindings]\nselectWorkspaceByNumber = \"alt+1\"\n"), "config.toml");
    assert!(k.warnings.is_empty(), "{:?}", k.warnings);
    assert!(k.bindings.iter().any(|b| b.action == "selectWorkspaceByNumber" && b.keystroke == "alt-9" && b.number == Some(9)));
}
```

Match the keystroke spelling to what the existing `super+opt+left` test produces for `opt` (M2 tests say `alt`). In `ghostty/mod.rs` tests:

```rust
#[test]
fn background_blur_accepts_bool_and_radius() {
    for (v, on) in [("true", true), ("false", false), ("20", true), ("0", false)] {
        let mut cfg = Config::default();
        cfg.apply("background-blur", v).unwrap();
        assert_eq!(cfg.background_blur, on, "{v}");
    }
    assert!(Config::default().apply("background-blur", "maybe").is_err());
}
```

- [x] **Step 2: Run.** `cargo test -p cmux-config`. Expected: the new tests fail.
- [x] **Step 3: Implement.** Add the defaults and `NUMBERED`. Add `background-blur`: `true`/`false` → bool, else `number(value)? > 0.0`, else the parse error the other keys give. Extend the module's doc comment on the defaults table with "workspace actions: rc §2 (M3)".
- [x] **Step 4: Run.** `cargo test -p cmux-config --locked`. Expected: all pass.
- [x] **Step 5: Commit.** `git commit -m "feat(cmux-config): cmux workspace shortcuts and Ghostty background-blur"`.

---

### Task 3: cmux-meta: git branch and display paths

D2. rc §4 (a), rc §3 directory shortening.

**Files:**
- Create: `crates/cmux-meta/src/git.rs`, `crates/cmux-meta/src/path.rs`
- Modify: `crates/cmux-meta/src/lib.rs` (`pub mod git; pub mod path;`, doc "sidebar metadata collectors: pure functions over the filesystem, unit-tested on fixtures")

**Interfaces — Produces:**
```rust
// git.rs
pub fn find_git_dir(start: &Path) -> Option<PathBuf>; // walk up; `.git` dir, or `.git` file "gitdir: X" (X relative to that file's dir)
pub fn parse_head(contents: &str) -> Option<String>;  // "ref: refs/heads/<b>" (trimmed, CRLF ok) → b; else None
pub fn branch(dir: &Path) -> Option<(PathBuf, String)>; // (git_dir, branch)
// path.rs
pub fn display_dir(dir: &str, home: Option<&str>) -> String;
```

Test fixtures: a helper `fn scratch(name: &str) -> PathBuf` that creates `std::env::temp_dir()/cmux-meta-<pid>-<name>` fresh (removing any old one). Tests clean up after themselves. No new dev-dependency.

- [x] **Step 1: Failing tests.**

```rust
// git.rs tests
#[test]
fn parse_head_branch_and_detached() {
    assert_eq!(parse_head("ref: refs/heads/main\n").as_deref(), Some("main"));
    assert_eq!(parse_head("ref: refs/heads/feat/x\r\n").as_deref(), Some("feat/x"));
    assert_eq!(parse_head("3b6e338c0ffee\n"), None);
    assert_eq!(parse_head("ref: refs/remotes/origin/main\n"), None);
    assert_eq!(parse_head(""), None);
}

#[test]
fn branch_walks_up_from_a_subdirectory() {
    let root = scratch("walk");
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/dev\n").unwrap();
    std::fs::create_dir_all(root.join("a/b")).unwrap();
    assert_eq!(branch(&root.join("a/b")), Some((root.join(".git"), "dev".into())));
}

#[test]
fn branch_follows_a_worktree_gitdir_file() {
    let root = scratch("worktree");
    let real = root.join("main/.git/worktrees/wt");
    std::fs::create_dir_all(&real).unwrap();
    std::fs::write(real.join("HEAD"), "ref: refs/heads/topic\n").unwrap();
    std::fs::create_dir_all(root.join("wt")).unwrap();
    std::fs::write(root.join("wt/.git"), "gitdir: ../main/.git/worktrees/wt\n").unwrap();
    let (dir, b) = branch(&root.join("wt")).unwrap();
    assert_eq!(b, "topic");
    assert!(dir.ends_with("main/.git/worktrees/wt"));
}

#[test]
fn branch_none_for_detached_or_no_repo() {
    let root = scratch("detached");
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::write(root.join(".git/HEAD"), "0123456789abcdef\n").unwrap();
    assert_eq!(branch(&root), None);
    assert_eq!(branch(Path::new("/")), None); // CI's / has no .git
}

// path.rs tests
#[test]
fn display_dir_home_prefix_only_on_component_boundary() {
    assert_eq!(display_dir("/home/alice", Some("/home/alice")), "~");
    assert_eq!(display_dir("/home/alice/src/x", Some("/home/alice")), "~/src/x");
    assert_eq!(display_dir("/home/alice/src/x/", Some("/home/alice/")), "~/src/x");
    assert_eq!(display_dir("/home/al", Some("/home/alice")), "/home/al");
    assert_eq!(display_dir("/home/alicex", Some("/home/alice")), "/home/alicex");
    assert_eq!(display_dir("/", Some("/home/alice")), "/");
    assert_eq!(display_dir("/tmp", None), "/tmp");
}
```

- [x] **Step 2: Run.** `cargo test -p cmux-meta`. Expected: compile errors.
- [x] **Step 3: Implement.** `find_git_dir` checks `dir.join(".git")`: `is_dir()` → return it; `is_file()` → read it, strip the `gitdir:` prefix, trim, and resolve relative to the file's parent. Then go to the parent, stopping at the root. `branch` = `find_git_dir` → read `HEAD` (an error gives None) → `parse_head`. `display_dir` trims trailing `/` from both, except a bare `/`.
- [x] **Step 4: Run.** Expected: 5 tests pass.
- [x] **Step 5: Commit.** `git commit -m "feat(cmux-meta): git branch from .git/HEAD (worktrees, detached) and ~ display paths"`.

---

### Task 4: cmux-meta: listening ports from `/proc`

D2. rc §4 ports.

**Files:**
- Create: `crates/cmux-meta/src/ports.rs`
- Modify: `crates/cmux-meta/src/lib.rs` (`pub mod ports;`)

**Interfaces — Produces:**
```rust
pub fn parse_listen_inodes(tcp: &str) -> HashMap<u64, u16>; // /proc/net/tcp{,6}: state 0A → inode → local port
pub fn parse_tty_nr(stat: &str) -> Option<i32>;             // field 7, parsed after the LAST ')'
pub fn socket_inode(link: &str) -> Option<u64>;             // "socket:[123]" → 123
/// Ports per tty for every process under `proc_root` whose tty_nr is in `ttys` (0 never matches).
pub fn listening_ports(proc_root: &Path, ttys: &HashSet<i32>) -> HashMap<i32, BTreeSet<u16>>;
pub fn tty_of(proc_root: &Path, pid: u32) -> Option<i32>;   // reads <root>/<pid>/stat
```

- [x] **Step 1: Failing tests.** Fixture `/proc` trees use the same `scratch` helper as Task 3. Move it to `lib.rs` as `#[cfg(test)] pub(crate) fn scratch`. Symlinks via `std::os::unix::fs::symlink`.

```rust
const TCP: &str = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:0BB8 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 4242 1 0000000000000000 100 0 0 10 0
   1: 0100007F:1F90 0100007F:D431 01 00000000:00000000 00:00000000 00000000  1000        0 4343 1 0000000000000000 20 4 30 10 -1
";

#[test]
fn listen_inodes_only_state_0a() {
    let m = parse_listen_inodes(TCP);
    assert_eq!(m.get(&4242), Some(&3000));
    assert!(!m.contains_key(&4343), "ESTABLISHED is not listening");
    assert!(parse_listen_inodes("").is_empty());
}

#[test]
fn tty_nr_survives_parens_in_comm() {
    assert_eq!(parse_tty_nr("123 (node) S 1 123 123 34816 123 4194560"), Some(34816));
    assert_eq!(parse_tty_nr("9 (a) b (c)) R 1 9 9 34817 9 0"), Some(34817));
    assert_eq!(parse_tty_nr("garbage"), None);
}

#[test]
fn socket_inode_parses_only_sockets() {
    assert_eq!(socket_inode("socket:[4242]"), Some(4242));
    assert_eq!(socket_inode("pipe:[1]"), None);
    assert_eq!(socket_inode("/dev/pts/3"), None);
}

#[test]
fn scan_skips_vanished_and_unreadable_processes() {
    let root = crate::scratch("proc");
    std::fs::create_dir_all(root.join("net")).unwrap();
    std::fs::write(root.join("net/tcp"), TCP).unwrap();
    std::fs::write(root.join("net/tcp6"), "  sl  local_address\n").unwrap();
    // pid 10 on tty 34816 listening on 3000
    std::fs::create_dir_all(root.join("10/fd")).unwrap();
    std::fs::write(root.join("10/stat"), "10 (node) S 1 10 10 34816 10 0").unwrap();
    std::os::unix::fs::symlink("socket:[4242]", root.join("10/fd/3")).unwrap();
    // pid 11 on the same tty: stat but no fd dir (unreadable or vanished)
    std::fs::create_dir_all(root.join("11")).unwrap();
    std::fs::write(root.join("11/stat"), "11 (sh) S 1 11 11 34816 11 0").unwrap();
    // pid 12: directory without stat (vanished mid-scan)
    std::fs::create_dir_all(root.join("12")).unwrap();
    // pid 13 on another tty, same socket: not ours
    std::fs::create_dir_all(root.join("13/fd")).unwrap();
    std::fs::write(root.join("13/stat"), "13 (x) S 1 13 13 34999 13 0").unwrap();
    std::os::unix::fs::symlink("socket:[4242]", root.join("13/fd/3")).unwrap();
    let ttys = HashSet::from([34816, 0]);
    let got = listening_ports(&root, &ttys);
    assert_eq!(got.get(&34816).map(|s| s.iter().copied().collect::<Vec<_>>()), Some(vec![3000]));
    assert!(!got.contains_key(&34999));
    assert_eq!(tty_of(&root, 10), Some(34816));
    assert_eq!(tty_of(&root, 12), None);
}
```

- [x] **Step 2: Run.** Expected: compile errors.
- [x] **Step 3: Implement.**
  - `parse_listen_inodes` skips the header. Per line, split on whitespace: field 1 is `local_address` (port = hex after the last `:`), field 3 is `st`, field 9 is the inode. Skip malformed lines.
  - `listening_ports` reads `net/tcp` and `net/tcp6` (a missing file counts as empty), then iterates numeric entries of `proc_root`, reads `stat`, and filters on `ttys` (excluding 0). For each match it runs `read_dir(fd)` → `read_link` → `socket_inode` → map lookup.
  - Every IO error is skipped, never propagated.
- [x] **Step 4: Run.** Expected: 4 tests pass.
- [x] **Step 5: Commit.** `git commit -m "feat(cmux-meta): listening ports by controlling tty from /proc"`.

---

### Task 5: cmux-meta: pull requests via `gh`

D2. rc §4 pull requests.

**Files:**
- Create: `crates/cmux-meta/src/pr.rs`
- Modify: `crates/cmux-meta/src/lib.rs` (`pub mod pr;`), `crates/cmux-meta/Cargo.toml` (`serde_json = "1"` — root `[workspace.dependencies]` gets `serde_json = "1"`, resolves to the locked 1.0.154), `NOTICE` (serde_json, MIT OR Apache-2.0)

**Interfaces — Produces:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrState { Closed, Merged, Open }   // Ord = display priority
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pr { pub number: u64, pub state: PrState, pub url: String }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrItem { pub pr: Pr, pub head: String, pub updated_at: String, pub merged_at: Option<String> }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError { NotFound, Failed(String) }

pub const GH_ARGS: &[&str] = &["pr", "list", "--state", "all", "--limit", "20",
    "--json", "number,state,url,updatedAt,mergedAt,headRefName", "--head"]; // branch appended
pub fn parse_gh_pr_list(json: &str) -> Result<Vec<PrItem>, String>;
pub fn days_from_iso(iso: &str) -> Option<i64>;   // "2026-11-03T…" → days since 1970-01-01
pub fn select_pr(items: &[PrItem], branch: &str, today: i64) -> Option<Pr>;
pub fn fetch(program: &str, dir: &Path, branch: &str, deadline: Duration) -> Result<Vec<PrItem>, FetchError>;
pub fn skip_branch(branch: &str) -> bool;          // "main" | "master"

pub type PrKey = (PathBuf, String);               // (git_dir, branch)
pub struct PrPoller { /* entries, disabled */ }
impl PrPoller {
    pub fn new() -> Self;
    pub fn due(&self, key: &PrKey, focused: bool, now: Instant) -> bool;
    pub fn record(&mut self, key: PrKey, result: Result<Option<Pr>, FetchError>, now: Instant);
    pub fn get(&self, key: &PrKey) -> Option<(&Pr, bool)>;   // (pr, stale)
    pub fn disabled(&self) -> bool;                          // after a NotFound
}
```

The cadence rules: never fetched → due. Otherwise the interval is:
- after a failure: `min(60 s × 2^(failures−1), 15 min)`;
- if the last PR is merged or closed: 15 min;
- otherwise: 10 s when focused, 60 s when not.

`record(Err(Failed))` keeps the previous PR and marks it stale. `record(Err(NotFound))` sets `disabled`. `record(Ok(None))` clears the PR.

- [x] **Step 1: Failing tests.**

```rust
const JSON: &str = r#"[
 {"number":7,"state":"CLOSED","url":"https://x/7","updatedAt":"2026-10-30T10:00:00Z","mergedAt":null,"headRefName":"feat"},
 {"number":9,"state":"OPEN","url":"https://x/9","updatedAt":"2026-10-01T10:00:00Z","mergedAt":null,"headRefName":"feat"},
 {"number":8,"state":"MERGED","url":"https://x/8","updatedAt":"2026-11-01T10:00:00Z","mergedAt":"2026-11-01T09:00:00Z","headRefName":"feat"},
 {"number":5,"state":"OPEN","url":"https://x/5","updatedAt":"2026-11-02T10:00:00Z","mergedAt":null,"headRefName":"other"}
]"#;

#[test]
fn parses_gh_json() {
    let items = parse_gh_pr_list(JSON).unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[2].pr.state, PrState::Merged);
    assert_eq!(items[2].merged_at.as_deref(), Some("2026-11-01T09:00:00Z"));
    assert!(parse_gh_pr_list("not json").is_err());
    assert!(parse_gh_pr_list("[]").unwrap().is_empty());
}

#[test]
fn days_from_iso_counts_from_epoch() {
    assert_eq!(days_from_iso("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(days_from_iso("2000-03-01T00:00:00Z"), Some(11017));
    assert_eq!(days_from_iso("nope"), None);
}

#[test]
fn select_prefers_open_then_recent_and_hides_old_merges() {
    let items = parse_gh_pr_list(JSON).unwrap();
    let today = days_from_iso("2026-11-03T00:00:00Z").unwrap();
    assert_eq!(select_pr(&items, "feat", today).unwrap().number, 9, "open beats newer merged");
    let no_open: Vec<_> = items.iter().filter(|i| i.pr.number != 9).cloned().collect();
    assert_eq!(select_pr(&no_open, "feat", today).unwrap().number, 8);
    let later = days_from_iso("2026-11-20T00:00:00Z").unwrap();
    assert_eq!(select_pr(&no_open, "feat", later).unwrap().number, 7, "merge > 14 days old hidden");
    assert_eq!(select_pr(&items, "none", today), None);
}

#[test]
fn fetch_without_gh_binary_is_not_found() {
    let r = fetch("cmux-meta-no-such-gh", Path::new("/"), "feat", Duration::from_secs(1));
    assert_eq!(r, Err(FetchError::NotFound));
}

#[test]
fn fetch_kills_a_hanging_command() {
    let mut cmd = Command::new("sleep");
    cmd.arg("5");
    let t = Instant::now();
    let r = run_with_deadline(cmd, Duration::from_millis(300));
    assert_eq!(r.err(), Some(FetchError::Failed("timed out".into())));
    assert!(t.elapsed() < Duration::from_secs(2));
}

#[test]
fn cadence_focused_background_and_final() {
    let mut p = PrPoller::new();
    let k: PrKey = ("/r/.git".into(), "feat".into());
    let t0 = Instant::now();
    assert!(p.due(&k, false, t0));
    p.record(k.clone(), Ok(Some(Pr { number: 1, state: PrState::Open, url: "u".into() })), t0);
    assert!(!p.due(&k, true, t0 + Duration::from_secs(9)));
    assert!(p.due(&k, true, t0 + Duration::from_secs(10)));
    assert!(!p.due(&k, false, t0 + Duration::from_secs(59)));
    assert!(p.due(&k, false, t0 + Duration::from_secs(60)));
    p.record(k.clone(), Ok(Some(Pr { number: 1, state: PrState::Merged, url: "u".into() })), t0);
    assert!(!p.due(&k, true, t0 + Duration::from_secs(899)));
    assert!(p.due(&k, true, t0 + Duration::from_secs(900)));
}

#[test]
fn failures_back_off_and_keep_the_last_pr_stale() {
    let mut p = PrPoller::new();
    let k: PrKey = ("/r/.git".into(), "feat".into());
    let t0 = Instant::now();
    p.record(k.clone(), Ok(Some(Pr { number: 3, state: PrState::Open, url: "u".into() })), t0);
    p.record(k.clone(), Err(FetchError::Failed("offline".into())), t0);
    assert_eq!(p.get(&k).map(|(pr, stale)| (pr.number, stale)), Some((3, true)));
    assert!(!p.due(&k, true, t0 + Duration::from_secs(59)));
    assert!(p.due(&k, true, t0 + Duration::from_secs(60)));
    p.record(k.clone(), Err(FetchError::Failed("offline".into())), t0);
    assert!(!p.due(&k, true, t0 + Duration::from_secs(119)));
    assert!(!p.disabled());
    p.record(k.clone(), Err(FetchError::NotFound), t0);
    assert!(p.disabled());
}
```

`fetch` builds `Command::new(program).args(GH_ARGS).arg(branch).current_dir(dir)` with stdin null and stdout/stderr piped, then calls the private `fn run_with_deadline(cmd: Command, deadline: Duration) -> Result<std::process::Output, FetchError>`. That helper is what the hanging-command test exercises.

- [x] **Step 2: Run.** Expected: compile errors.
- [x] **Step 3: Implement.**
  - `parse_gh_pr_list` uses `serde_json::Value`. State strings are `OPEN`/`MERGED`/`CLOSED`; anything else skips the item. An empty-string or null `mergedAt` becomes None.
  - `days_from_iso` parses `YYYY-MM-DD` and uses the civil-from-days algorithm (Howard Hinnant's `days_from_civil`, public domain; cite it in a comment).
  - `select_pr`: filter on head, drop merged items whose `days_from_iso(merged_at) < today − 14`, then take the max by `(state, updated_at, number)`.
  - `run_with_deadline`: `spawn` (an `ErrorKind::NotFound` → `FetchError::NotFound`), then `try_wait` every 50 ms. Past the deadline it calls `kill` and `wait` and returns `Failed("timed out")`. A non-zero exit gives `Failed(<first stderr line>)`.
- [x] **Step 4: Run.** Expected: 7 tests pass. Check libc: `grep -A1 'name = "libc"' Cargo.lock` still shows 0.2.189.
- [x] **Step 5: Commit.** `git add Cargo.toml Cargo.lock NOTICE crates/cmux-meta && git commit -m "feat(cmux-meta): PR badge via gh pr list with cmux selection, cadence and backoff"`.

---

### Task 6: cmux-meta `collect`, and PTY foreground job detection

D1 close prompt, D2. rc §1 close, rc §4 "which panels".

**Files:**
- Modify: `crates/cmux-meta/src/lib.rs`
- Modify: `crates/cmux-term/src/pty.rs`, `crates/cmux-term/tests/pty.rs`

**Interfaces — Produces:**
```rust
// cmux-meta
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PanelInput { pub cwd: Option<String>, pub pid: Option<u32> }
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceInput { pub panels: Vec<PanelInput> }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirLine { pub path: String, pub display: String, pub branch: Option<String>, pub git_dir: Option<PathBuf> }
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceMeta { pub dirs: Vec<DirLine>, pub ports: Vec<u16> }
/// One entry per input, same order. Pure apart from reading the filesystem.
pub fn collect(inputs: &[WorkspaceInput], proc_root: &Path, home: Option<&str>) -> Vec<WorkspaceMeta>;

// cmux-term
impl Session { pub fn foreground_busy(&self) -> bool; } // tcgetpgrp(master) is Some and != shell pid
```

- [x] **Step 1: Failing tests.**

```rust
// cmux-meta lib.rs
#[test]
fn collect_dedups_directories_in_panel_order_and_unions_ports() {
    let root = scratch("collect");
    let repo = root.join("repo");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    std::fs::write(repo.join(".git/HEAD"), "ref: refs/heads/feat\n").unwrap();
    let proc_root = root.join("proc");
    std::fs::create_dir_all(proc_root.join("net")).unwrap();
    std::fs::write(proc_root.join("net/tcp"), ports::tests::TCP).unwrap();
    std::fs::create_dir_all(proc_root.join("10/fd")).unwrap();
    std::fs::write(proc_root.join("10/stat"), "10 (sh) S 1 10 10 34816 10 0").unwrap();
    std::fs::create_dir_all(proc_root.join("20/fd")).unwrap();
    std::fs::write(proc_root.join("20/stat"), "20 (node) S 10 20 10 34816 20 0").unwrap();
    std::os::unix::fs::symlink("socket:[4242]", proc_root.join("20/fd/5")).unwrap();
    let r = repo.to_str().unwrap().to_string();
    let input = WorkspaceInput { panels: vec![
        PanelInput { cwd: Some(r.clone()), pid: Some(10) },
        PanelInput { cwd: Some("/".into()), pid: None },
        PanelInput { cwd: Some(format!("{r}/")), pid: Some(10) },
    ] };
    let meta = collect(&[input, WorkspaceInput::default()], &proc_root, None);
    assert_eq!(meta.len(), 2);
    let dirs: Vec<_> = meta[0].dirs.iter().map(|d| (d.path.as_str(), d.branch.as_deref())).collect();
    assert_eq!(dirs, vec![(r.as_str(), Some("feat")), ("/", None)]);
    assert_eq!(meta[0].ports, vec![3000]);
    assert_eq!(meta[1], WorkspaceMeta::default());
}
```

Make `ports.rs`'s test module `#[cfg(test)] pub(crate) mod tests` and its `TCP` constant `pub(crate)`. Directories are deduped on the path with any trailing `/` trimmed (`/` itself stays `/`), not on `canonicalize`, which is slow on network mounts and resolves symlinks the user typed.

```rust
// cmux-term tests/pty.rs
#[test]
fn foreground_busy_tracks_a_running_job() {
    let (tx, _rx) = mpsc::channel::<()>();
    let cmd = CommandBuilder::new("sh"); // interactive: job control on a tty
    let session = Session::spawn(cmd, 80, 24, |_| {}, move |_| { let _ = tx.send(()); }).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(!session.foreground_busy(), "idle shell");
    session.write(b"sleep 3\n".to_vec());
    let t = Instant::now();
    while !session.foreground_busy() && t.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(session.foreground_busy(), "sleep is the foreground job");
}
```

Match the `Session::spawn` closure signatures used by the existing tests in this file.

- [x] **Step 2: Run.** `cargo test -p cmux-meta && cargo test -p cmux-term --test pty foreground`. Expected: compile errors.
- [x] **Step 3: Implement.**
  - `collect` takes the ttys of all panel pids via `ports::tty_of`, then runs one `listening_ports` scan for all inputs and unions per workspace. `display` = `path::display_dir`; `branch`/`git_dir` = `git::branch`.
  - `foreground_busy`: use portable-pty's `MasterPty::process_group_leader()` if 0.9 has it (check `~/.cargo/registry/src/*/portable-pty-0.9*/src/lib.rs`). Otherwise use `libc::tcgetpgrp` on `master.as_raw_fd()`. Compare with `process_id()`; None on either side → false.
- [x] **Step 4: Run.** Expected: pass.
- [x] **Step 5: Commit.** Use two commits: `feat(cmux-meta): collect per-workspace directories, branches and ports` (cmux-meta), then `feat(cmux-term): foreground_busy from the PTY's foreground process group` (cmux-term).

---

### Task 7: WorkspaceView becomes one workspace among many

D1, D2. Carry list: "container size must come from the workspace area", "last-surface quit → close workspace".

**Files:**
- Modify: `crates/cmux-app/src/workspace.rs`, `crates/cmux-app/src/terminal_view.rs`, `crates/cmux-app/src/actions.rs`

**Interfaces:**
- Consumes: `cmux_meta::PanelInput` (add `cmux-meta = { path = "crates/cmux-meta" }` to root `[workspace.dependencies]` and to `cmux-app`'s deps); `Session::foreground_busy`.
- Produces:
```rust
// terminal_view.rs
impl TerminalView { pub fn pid(&self) -> Option<u32>; pub fn busy(&self) -> bool; }
// workspace.rs
pub enum WorkspaceEvent { Empty, Changed }          // impl EventEmitter<WorkspaceEvent> for WorkspaceView
impl WorkspaceView {
    pub fn new(style: Rc<TermStyle>, cwd: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> Self;
    pub fn has_terminals(&self) -> bool;             // false if the first shell failed to start
    pub fn set_left_inset(&mut self, inset: f32);    // container = viewport minus this width
    pub fn set_active(&mut self, active: bool);      // only the active workspace refocuses on focus loss
    pub fn focus_active(&mut self, window: &mut Window, cx: &mut Context<Self>); // now pub
    pub fn title(&self, cx: &App) -> Option<String>; // focused surface's OSC title
    pub fn active_cwd(&self, cx: &App) -> Option<String>;
    pub fn panels(&self, cx: &App) -> Vec<cmux_meta::PanelInput>; // layout.surfaces() order
    pub fn busy(&self, cx: &App) -> bool;            // any surface busy
}
// actions.rs: NewWorkspace, CloseWorkspace, RenameWorkspace, NextWorkspace, PrevWorkspace,
// MoveWorkspaceUp, MoveWorkspaceDown, ToggleSidebar, and
// #[action(namespace = cmux, no_json)] pub struct SelectWorkspaceByNumber(pub u8);
// action_for: "newTab" → NewWorkspace, "closeWorkspace", "renameWorkspace",
// "nextSidebarTab" → NextWorkspace, "prevSidebarTab" → PrevWorkspace, "moveWorkspaceUp",
// "moveWorkspaceDown", "toggleSidebar", "selectWorkspaceByNumber" → SelectWorkspaceByNumber(b.number?)
```

Behaviour changes:
- `Closed::LastSurface` now drops the surface and emits `WorkspaceEvent::Empty`; no more `cx.quit`.
- `TerminalEvent::TitleChanged` → `cx.notify()` + `cx.emit(WorkspaceEvent::Changed)`.
- A failed first shell sets `has_terminals = false` instead of quitting.
- `on_focus_lost` only refocuses when `active`. `active` starts `true`, so the M2 single-workspace path is unchanged until Task 8.
- `container(window)` becomes a method `self.container(window)` returning `(viewport_w − left_inset, viewport_h, scale)`.

- [ ] **Step 1: Failing test.** M2's `every_config_action_has_a_gpui_action` in `actions.rs` already keeps `action_for` in sync with `DEFAULTS`. Run it: `cargo test -p cmux-app --lib actions`. Expected: it fails on `newTab` (Task 2 added the defaults). `the_default_table_parses_in_gpui` covers the new chords once they bind.
- [ ] **Step 2: Implement** the actions, then the TerminalView and WorkspaceView items above. `main.rs` passes `None` for `cwd` for now.
- [ ] **Step 3: Run.** `cargo test -p cmux-app --locked`, then the clippy line. Expected: pass.
- [ ] **Step 4: Run it.** `cargo run -p cmux-app`. Expected: same as M2. Closing the last tab now leaves an empty window (Task 8 wires `Empty`); note it, it isn't a bug yet.
- [ ] **Step 5: Commit.** `git commit -m "feat(cmux-app): WorkspaceView reports empty/changed, takes an inset and cwd; workspace actions"`.

---

### Task 8: WindowView: workspaces, switching, close prompt (no sidebar yet)

D1, D6. rc §1, §2.

**Files:**
- Create: `crates/cmux-app/src/window_view.rs`
- Modify: `crates/cmux-app/src/lib.rs` (`pub mod window_view;`), `crates/cmux-app/src/main.rs` (open `WindowView::new(style, window, cx)`)

**Interfaces:**
- Consumes: Task 1 `Workspaces`; Task 7 `WorkspaceView` API and actions.
- Produces:
```rust
pub struct WindowView { /* workspaces: Workspaces, views: HashMap<WorkspaceId, Entity<WorkspaceView>>,
    style: Rc<TermStyle>, sidebar_visible: bool, sidebar_width: f32, focus: FocusHandle, _subs */ }
impl WindowView {
    pub fn new(style: Rc<TermStyle>, window: &mut Window, cx: &mut Context<Self>) -> Self; // quits if the first shell fails
    pub fn workspaces(&self) -> &Workspaces;
    fn new_workspace(&mut self, at_end: bool, window: &mut Window, cx: &mut Context<Self>);
    fn select(&mut self, id: WorkspaceId, window: &mut Window, cx: &mut Context<Self>);
    fn request_close(&mut self, id: WorkspaceId, window: &mut Window, cx: &mut Context<Self>); // prompts if busy
    fn close_now(&mut self, id: WorkspaceId, window: &mut Window, cx: &mut Context<Self>);     // Last → cx.quit()
    pub(crate) fn title_of(&self, id: WorkspaceId, cx: &App) -> String; // custom, else OSC title, else "Terminal"
}
pub const SIDEBAR_DEFAULT: f32 = 240.0; pub const SIDEBAR_MIN: f32 = 240.0;
pub const SIDEBAR_MAX: f32 = 600.0;     pub const CONTENT_MIN: f32 = 300.0;
```

Rules:
- `new_workspace`: cwd = selected view's `active_cwd`. Add to the model (`add_after_selected` or `add_at_end`), create the view, and subscribe (`Empty` → `close_now(id)`, `Changed` → `cx.notify()`). If `!has_terminals()`, `close_now` it again (the selection returns to the previous workspace, which it slid next to), then `select`.
- `select`: `set_active(false)` on the old view, `set_active(true)` on the new one, `focus_active`, `cx.notify()`.
- `request_close`: if `busy`, `window.prompt(PromptLevel::Warning, "Close workspace?", Some("This will close the workspace and all of its panels."), &["Close", "Cancel"], cx)`, then `cx.spawn_in` awaits it and answer 0 → `close_now`. This is the M2 paste-prompt pattern in `terminal_view.rs`.
- `close_now`: model `close`. `Last` → `cx.quit()`. `Closed` → drop the view entity (its terminals drop with it) and `select(new selected)`.
- Render: `div().flex().flex_row().size_full()` with the selected workspace's view in a `flex_1().min_w(px(0.))` child. Call `set_left_inset(0.)` (Task 9 passes the sidebar width). Actions go on the root div: NewWorkspace, CloseWorkspace (selected), NextWorkspace/PrevWorkspace (`cycle`), SelectWorkspaceByNumber, MoveWorkspaceUp/Down (`move_selected` + notify). ToggleSidebar and RenameWorkspace are no-ops until Task 9/10.

- [ ] **Step 1: Implement** as above. All rules here are covered by Task 1's model tests; this task is wiring.
- [ ] **Step 2: Run tests and lints.** `cargo test -p cmux-app --locked` and the clippy line. Expected: pass.
- [ ] **Step 3: Commit.** `git commit -m "feat(cmux-app): WindowView with workspaces, switching and close confirmation"`.
- [ ] **Step 4: Run it** at the desktop and check each item:
  - Super+N opens a new workspace in the same cwd.
  - Super+1 / Super+9 / Super+Ctrl+] / [ switch, and the hidden workspace's shells keep running (`sleep 5; echo done` in one, switch away and back).
  - Super+Shift+W on an idle workspace closes it without a prompt.
  - With `sleep 100` running, the prompt appears; Cancel keeps the workspace and Close closes it.
  - `exit` in the last tab of a non-last workspace closes that workspace; in the last workspace it quits.
- [ ] **Step 5: Run it (Review Focus 1).** With two workspaces, click another app's window and back, and close a tab. Keys must reach the *visible* terminal (type `echo hi`). If not, fix before going on, then commit `fix(m3): …`.

---

### Task 9: Sidebar: band, rows, metadata loop, resize, toggle

D2, D3, D5. rc §3, §4; rg §2, §3.

**Files:**
- Create: `crates/cmux-app/src/window_view/sidebar.rs`
- Modify: `crates/cmux-app/src/window_view.rs`, `crates/cmux-app/src/chrome.rs`, `crates/cmux-app/src/assets.rs`, `crates/cmux-app/assets/icons/` (+5 Lucide SVGs from lucide-static 0.460.0, same as M2), `NOTICE` (Lucide line lists the new five)

**Interfaces:**
- Consumes: Task 6 `collect`, `WorkspaceMeta`, `DirLine`; Task 5 `PrPoller`, `Pr`, `PrState`, `fetch`, `select_pr`, `skip_branch`, `days_from_iso`.
- Produces:
```rust
// chrome.rs
pub fn cmux_accent(light_bg: bool) -> Rgba;  // #0088FF / #0091FF
pub struct SidebarPalette { pub bg: Rgba, pub border: Rgba, pub selected_fill: Rgba,
    pub title: Rgba, pub detail: Rgba, pub selected_title: Rgba, pub selected_detail: Rgba, pub accent: Rgba }
pub fn sidebar_palette(bg: Rgba) -> SidebarPalette;
// sidebar.rs (pure part, unit-tested)
pub(crate) struct RowModel { pub title: String, pub dirs: Vec<(Option<String>, Option<String>)>, // (branch, display dir)
    pub prs: Vec<(Pr, bool)>, pub ports: Vec<u16> }
pub(crate) fn row_model(title: &str, meta: &WorkspaceMeta, prs: &[(Pr, bool)]) -> RowModel;
pub(crate) fn clamp_width(w: f32, window_w: f32) -> f32; // [MIN, MAX] and ≤ window_w − CONTENT_MIN, never < MIN
```

- [ ] **Step 1: Failing tests.**

```rust
// chrome.rs
#[test]
fn sidebar_palette_uses_cmux_accent_and_label_alphas() {
    let dark = Rgba { r: 0.1, g: 0.1, b: 0.1, a: 1.0 };
    let p = sidebar_palette(dark);
    assert_eq!(p.selected_fill, cmux_accent(false));
    assert_eq!((p.title.r, p.title.a), (1.0, 0.85));
    assert_eq!(p.detail.a, 0.55);
    assert_eq!((p.selected_title.r, p.selected_title.a), (1.0, 1.0));
    assert_eq!(p.selected_detail.a, 0.75);
    let light = sidebar_palette(Rgba { r: 0.97, g: 0.97, b: 0.97, a: 1.0 });
    assert_eq!(light.selected_fill, cmux_accent(true));
    assert_eq!((light.title.r, light.detail.a), (0.0, 0.5));
    assert_eq!(light.border, separator(Rgba { r: 0.97, g: 0.97, b: 0.97, a: 1.0 }));
}

// sidebar.rs
#[test]
fn row_model_drops_the_title_echo_and_keeps_branch() {
    let meta = WorkspaceMeta { dirs: vec![
        DirLine { path: "/home/u/api".into(), display: "~/api".into(), branch: Some("feat".into()), git_dir: None },
        DirLine { path: "/tmp".into(), display: "/tmp".into(), branch: None, git_dir: None },
    ], ports: vec![3000, 5173] };
    let r = row_model("~/api", &meta, &[]);
    assert_eq!(r.dirs, vec![(Some("feat".into()), None), (None, Some("/tmp".into()))]);
    assert_eq!(r.ports, vec![3000, 5173]);
}

#[test]
fn clamp_width_keeps_content_room() {
    assert_eq!(clamp_width(100.0, 2000.0), 240.0);
    assert_eq!(clamp_width(900.0, 2000.0), 600.0);
    assert_eq!(clamp_width(500.0, 700.0), 400.0);
    assert_eq!(clamp_width(500.0, 400.0), 240.0, "never below the minimum");
}
```

- [ ] **Step 2: Run.** Expected: compile errors.
- [ ] **Step 3: Implement the pure parts** (`cmux_accent`, `sidebar_palette`, `row_model`, `clamp_width`). Run: pass. Commit `feat(cmux-app): sidebar palette and row model`.
- [ ] **Step 4: Implement the metadata loops** in `window_view.rs`, using the rg §3 poll pattern:
  - **Fields:** `meta: HashMap<WorkspaceId, WorkspaceMeta>`, `prs: PrPoller`, `pr_in_flight: bool`, and `_tasks: Vec<Task<()>>`.
  - **Metadata loop (every 1500 ms):** read `[(id, WorkspaceInput { panels: view.panels(cx) })]`, then run `collect(&inputs, Path::new("/proc"), home.as_deref())` on `background_executor`. `home` is `std::env::var("HOME").ok()`, read once. Store the result and call `cx.notify()` only if it changed.
  - **PR loop (every 2000 ms):** if `!prs.disabled()` and nothing is in flight, gather every `(git_dir, branch)` from `meta` where `!skip_branch`, and mark one `focused` if its workspace is selected. Fetch up to 3 due keys one after another in a single background task, using `fetch("gh", dir, branch, 15 s)` with `dir` = the `DirLine.path`. Map each result to `select_pr(&items, branch, today)`, where `today` = `SystemTime::now()` / 86400. `record` it, then notify.
  - Commit `feat(cmux-app): background metadata and PR polling`.
- [ ] **Step 5: Implement the sidebar render** in `sidebar.rs` as `impl WindowView { pub(crate) fn render_sidebar(&self, window, cx) -> impl IntoElement }`. Use the D3 numbers: band 28 with buttons, the list from y 30 (`overflow_y_scroll` + `ScrollHandle`, `scroll_to_item` on selection change), and rows as in D3. Wire these interactions:
  - click → `select`;
  - × (group hover, only when `len > 1`) → `request_close`;
  - PR → `cx.open_url(&pr.url)`; port → `cx.open_url(&format!("http://localhost:{p}"))`;
  - stale PR at `.opacity(0.5)`;
  - double-click empty list space → `new_workspace(true)`;
  - resize band per D3 (M2 divider drag pattern, `clamp_width`), with `set_left_inset(width)` on every view when visible, else `0.`;
  - `ToggleSidebar` flips visibility.
  - Commit `feat(cmux-app): sidebar with workspace rows, branch, cwd, PR and ports`.
- [ ] **Step 6: Run it.**
  - Three workspaces: one in a git repo on a feature branch with an open PR, one running `python3 -m http.server 8123`, one in `/tmp`.
  - Expected: branch · `~/…` lines, `PR #N open`, and `:8123`.
  - `git switch` elsewhere updates the branch within 2 s; stopping the server drops the port within ~3 s.
  - Typing stays smooth throughout.
  - Super+B hides and shows the sidebar and the panes reflow; dragging the edge resizes within 240–600.
  - Fix anything off, then commit `fix(m3): …` per fix.

---

### Task 10: Rename and drag-to-reorder

D1 rename, D3 drag. rc §1 rename/reorder, rg §2.

**Files:**
- Create: `crates/cmux-app/src/window_view/rename.rs`
- Modify: `crates/cmux-app/src/window_view.rs`, `crates/cmux-app/src/window_view/sidebar.rs`

**Interfaces — Produces:**
```rust
// rename.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenameKey<'a> { Char(&'a str), Backspace, Enter, Escape, Other }
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenameOutcome { Editing, Commit(String), Cancel }
pub(crate) struct RenameState { pub text: String, pub replace_on_type: bool }
impl RenameState {
    pub fn new(current: &str) -> Self;                          // replace_on_type = true
    pub fn apply(&mut self, key: RenameKey) -> RenameOutcome;
}
pub(crate) struct RenameInput { /* id: WorkspaceId, state: RenameState, focus: FocusHandle */ }
pub(crate) enum RenameEvent { Done(WorkspaceId, Option<String>) } // None = cancelled
// window_view.rs: rename: Option<Entity<RenameInput>>; fn start_rename(&mut self, id, window, cx)
// sidebar.rs: struct DraggedWorkspace { id: WorkspaceId, title: SharedString }; drop_gap: Option<usize> on WindowView
pub(crate) fn drop_gap(row_mids: &[f32], y: f32) -> usize;     // first row whose mid is below y, else len
```

- [ ] **Step 1: Failing tests.**

```rust
// rename.rs
#[test]
fn first_char_replaces_then_appends() {
    let mut s = RenameState::new("Terminal");
    assert_eq!(s.apply(RenameKey::Char("a")), RenameOutcome::Editing);
    assert_eq!(s.text, "a");
    s.apply(RenameKey::Char("pi"));
    assert_eq!(s.text, "api");
    s.apply(RenameKey::Backspace);
    assert_eq!(s.text, "ap");
    assert_eq!(s.apply(RenameKey::Enter), RenameOutcome::Commit("ap".into()));
}

#[test]
fn backspace_first_clears_and_escape_cancels() {
    let mut s = RenameState::new("é名");
    s.apply(RenameKey::Backspace);
    assert_eq!(s.text, "", "preselected text is deleted whole");
    let mut s = RenameState::new("x");
    s.replace_on_type = false;
    s.text = "é名".into();
    s.apply(RenameKey::Backspace);
    assert_eq!(s.text, "é", "backspace removes one char, not one byte");
    assert_eq!(s.apply(RenameKey::Escape), RenameOutcome::Cancel);
}

// sidebar.rs
#[test]
fn drop_gap_uses_row_midpoints() {
    let mids = [20.0, 60.0, 100.0];
    assert_eq!(drop_gap(&mids, 5.0), 0);
    assert_eq!(drop_gap(&mids, 59.0), 1);
    assert_eq!(drop_gap(&mids, 61.0), 2);
    assert_eq!(drop_gap(&mids, 500.0), 3);
}
```

- [ ] **Step 2: Run.** Expected: compile errors.
- [ ] **Step 3: Implement** `RenameState`/`drop_gap`. Run: pass. Commit `feat(cmux-app): rename state and drop-gap maths`.
- [ ] **Step 4: Implement the UI.**
  - **`RenameInput`:** a focused div with `on_key_down`. Map the event as follows: `keystroke.key_char` → `Char`; `backspace`/`enter`/`escape` → the matching key; anything else → `Other`, with `cx.stop_propagation()` for all of these so they don't trigger bindings. Losing focus (`cx.on_blur`) commits. Render it in the row's title slot with the same font, text plus a 1 px caret in the title colour. The placeholder "Workspace name" shows when empty.
  - **`start_rename`:** opened by double-clicking a row (`ClickEvent::click_count() == 2`) or by `RenameWorkspace` on the selected row, which first sets `sidebar_visible = true`. The prefill is `title_of(id)`.
  - **`Done(id, Some(t))`:** call `workspaces.rename(id, &t)` and focus the active terminal. On `None`, just refocus. Closing a workspace that is being renamed drops the input.
  - Commit `feat(cmux-app): inline workspace rename`.
- [ ] **Step 5: Implement the drag.**
  - Each row calls `.on_drag(DraggedWorkspace, |d, _, _, cx| cx.new(|_| ghost(d)))`. The ghost is the title in a rounded accent-tinted box.
  - The list takes `on_drag_move::<DraggedWorkspace>`. It turns the row bounds (from `on_children_prepainted`) into midpoints and computes `drop_gap`. It draws the 2 px accent line inset 8 at that gap, and the dragged row at 0.6 opacity.
  - `on_drop` calls `workspaces.move_to(id, gap)`.
  - A drag released outside the list clears `drop_gap`, as M2's tab strip does.
  - Commit `feat(cmux-app): drag workspaces to reorder`.
- [ ] **Step 6: Run it.** Check each item; fix anything that fails and commit each fix.
  - Double-click → type → Enter renames. Esc reverts. Clicking a terminal commits. An empty name reverts to the OSC title.
  - Super+Shift+R renames the selected row, and while renaming Super+N does not create a workspace.
  - Drag row 3 above row 1, then Super+1 selects the moved one.
  - Super+Ctrl+Opt+[ / ] move the selected row.

---

### Task 11: Blur shim

D4. rg §1.4. Last because it is the only unverified platform call; nothing depends on it.

**Files:**
- Create: `crates/cmux-app/src/blur.rs`
- Modify: `crates/cmux-app/src/lib.rs` (`#[cfg(target_os = "linux")] pub mod blur;`), `crates/cmux-app/Cargo.toml` (`[target.'cfg(target_os = "linux")'.dependencies]` with the four crates from rg §1.4), root `Cargo.toml` (workspace deps), `crates/cmux-app/src/main.rs`, `NOTICE` (wayland-rs crates: MIT; raw-window-handle: MIT OR Apache-2.0 OR Zlib)

**Interfaces — Produces:**
```rust
pub struct WindowBlur { /* conn, queue, effect */ }
impl WindowBlur {
    /// Binds ext_background_effect_manager_v1 on gpui's own display and blurs the whole surface.
    /// None on X11, without the protocol, or if any step fails (logged once to stderr).
    /// `cx` guards with `cx.compositor_name()`: gpui 0.2.2's X11 window panics on raw handles.
    pub fn enable(window: &gpui::Window, cx: &gpui::App) -> Option<Self>;
}
```

`main.rs`:
- `window_background` = `Transparent` if `background_opacity < 1`, else `Opaque`.
- Inside `open_window`'s closure on Linux, if `cfg.background_blur && background_opacity < 1`, call `WindowBlur::enable(window)` and keep the result alive in `WindowView` (`_blur: Option<WindowBlur>` field, `cfg`-gated).
- Remove the M1 comment about `Blurred`.
- The blur region is `create_region` + `add(0, 0, i32::MAX / 2, i32::MAX / 2)`, set once, then `destroy` + `flush`. The compositor clips it (rg §1.4).

- [ ] **Step 1: Implement** from rg §1.4's sketch, dropping `set_width` (whole surface, set once). Add the dependency lines, then run `cargo build -p cmux-app --locked`. Expected: builds, and `Cargo.lock` gains only dependency edges (no new packages; libc unchanged).
- [ ] **Step 2: Lints.** Run the clippy line. Each `unsafe` gets a `// SAFETY:` line: the pointers come from gpui's live window, and our `WindowBlur` is dropped with `WindowView`, before the window.
- [ ] **Step 3: Commit.** `git commit -m "feat(cmux-app): KWin blur via ext_background_effect_v1 on gpui's surface"`.
- [ ] **Step 4: Run it.** Use a scratch Ghostty config with `background-opacity = 0.8` and `background-blur = true` (the Task 12 Step 2 pattern from M2, never the real config). Expected: window content behind is blurred on Plasma 6.8. Without `background-blur` it is translucent but not blurred. At opacity 1 it is opaque. If `enable` returns None or crashes, record which rg §1.4 risk hit in the journal. Keep the `Transparent` change and leave blur off (`shortcut:` comment) rather than block the milestone.

---

### Task 12: Exit criteria, close-out

**Files:** `docs/milestones/m3-workspaces-sidebar/journal.md`, `docs/status-archive.md`, `STATUS.md`, `docs/design.md`, `README.md` (shortcut table gains the workspace rows)

- [ ] **Step 1: Exit 1.**
  - Open 5+ workspaces covering: two repos on different branches, one with an open PR, one with a merged PR, a worktree, `/tmp`, and a dev server.
  - Record that every row is correct and that a branch switch or port change appears in ≤ ~2 s.
  - Record any Plasma 6 *default* shortcut that intercepts Super+1–9, Super+N, Super+B and so on, as "Meta+X: <Plasma action>". Don't change the defaults.
- [ ] **Step 2: Exit 2.**
  - Screenshot `cmux-linux-sidebar.png` (spectacle, as in M2).
  - Reference: cmux's published screenshots (`docs/assets/main-first-image.png` at 21dcc2b). If a Mac with cmux is available, take a real `cmux-mac-sidebar.png` with the same workspaces instead and say so.
  - Compare and record: width, band, row padding and radius, title/detail fonts, selected fill, branch · dir line, PR line, ports line, border.
- [ ] **Step 3: Exit 3.**
  - With 8 workspaces in repos, hold a key down in a terminal for 10 s while `gh` is slow (block it: `PATH=$(mktemp -d):$PATH` with a `gh` script that `sleep 30`s).
  - Typing must not stall. Record it, plus a `CMUX_TRACE_LATENCY=1` sample if M1's trace exists by then.
- [ ] **Step 4: Journal entry.** Append "M3 complete": the decisions, every deviation (gh instead of REST, inline rename for Super+Shift+R, no dirty marker, polling instead of watchers, blur whole-window and only with `background-blur`, sidebar band instead of titlebar controls, no persistence), and every Task 11/12 result.
- [ ] **Step 5: design.md.** In the `cmux-meta` row, change "Depends on" to `serde_json` and "git branch (watch `.git/HEAD`)" to "git branch (`.git/HEAD`)". The `cmux-app` row is unchanged.
- [ ] **Step 6: STATUS.** Archive, then rewrite: M3 done (PR #n), next M4 notifications, a code map for `cmux-meta`/`window_view`/`blur`, the carry list (M4 owns sidebar notification text; M5 owns shell-integration kicks; M6 owns macOS ports/cwd/blur), and remove the closed M2 carry items. Run `scripts/check-status-lines.sh`.
- [ ] **Step 7: PR.** Push, open the PR against `master` (body ends with the session's PR attribution line), `gh pr checks --watch`, then merge. Commit `docs: record M3 PR number` on master if needed.

---

## Exit criteria and scope check

| Requirement | Task |
|---|---|
| Workspaces in `cmux-core`; new/close/rename/reorder | 1 (model), 8 (new/close), 10 (rename, drag), 8 (keyboard move) |
| Super+1–8, Super+9 (last), next/prev | 1, 2, 7, 8 |
| Sidebar: title, cwd (OSC 7), git branch, ports, PR (gh, cached, rate-limited) | 3, 4, 5, 6, 9 |
| `cmux-meta` collectors unit-tested against fixtures | 3, 4, 5, 6 |
| Translucent sidebar using KWin blur (`ext_background_effect_v1`) | 9 (sidebar on terminal backdrop), 11 |
| Exit 1: correct for 5+ workspaces, ~2 s updates | 9 Step 6, 12 Step 1 |
| Exit 2: visual match, screenshot pair | 9, 12 Step 2 |
| Exit 3: no collector blocks the UI thread | 9 Step 4 (background executor, gh deadline), 12 Step 3 |
| Carry: container from the workspace area | 7 (`set_left_inset`), 9 |
| Carry: last surface → close workspace | 7, 8 |
