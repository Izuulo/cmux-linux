# M6 parity-release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** cmux-linux looks like cmux on the Mac, runs properly on macOS, installs from a PKGBUILD on CachyOS, and stays smooth with 20 busy panes.

**Architecture:**
- macOS gaps close where the Linux code already has the `/proc` or pidfd version: `cmux-meta` gets a `darwin` module (libproc) for cwd, tty and listening ports; `cmux-term` gets a macOS cwd and a kqueue exit watch in place of the pidfd. A macOS CI job keeps them building.
- Packaging is files only: `packaging/arch/PKGBUILD`, `packaging/cmux-linux.desktop`, a `deny.toml` licence gate. An Arch container job in CI runs `makepkg`.
- Parity and performance are measured on this Mac against the installed `cmux.app` (both apps draw on the same screen), then repeated on the KDE desktop. Fixes are constant changes in the files that already hold the values.

**Tech Stack:** Rust 2024 (toolchain per `rust-toolchain.toml`), gpui `=0.2.2`, libc 0.2.189 (already a dependency; `cmux-meta` gains it on macOS only), cargo-deny (CI tool, not shipped), makepkg.

**Spec:** `docs/milestones/m6-parity-release/spec.md`.

## Global Constraints

Carried from M0–M5:
- Licence GPL-3.0-or-later. Every added dependency, vendored file or asset goes in `NOTICE`. Never copy from cmux's BUSL-1.1 directories; check a directory's `LICENSE` before reading it.
- Zig exactly 0.16.0; `export PATH="$PWD/.tools/zig:$PATH"` from the repo root before any cargo command.
- `--locked` on build, test and clippy. Task 2 may change `Cargo.lock` only in `cmux-meta`'s dependency list (adds `libc`). libc stays 0.2.189. Never a bare `cargo update`.
- Before each commit: `cargo fmt -p <crates touched>` and `cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings`.
- `cmux-core` depends on nothing; `cmux-ipc` never depends on gpui. OS-specific code sits behind `cfg(target_os = …)`.
- macOS host builds: `cargo build --release --features gpui/runtime_shaders`. A **debug** app build hangs in gpui's `Application::new`; always run the app from a release build.
- No personal info in tracked files. `STATUS.md` max 100 lines, archived before replacement.

New for M6:
- Branch `m6-parity-release` from `master`. One commit per step that changes files; messages end with a blank line and the session's attribution lines.
- This plan's code was not compiled. Interface changes go in the journal.
- Screenshots go to `~/Pictures/cmux-m6/`, outside the repo.
- cmux reference: a sparse clone of `https://github.com/manaflow-ai/cmux` (`Sources/`) in a scratch directory. Record its commit in the journal; cite `Sources/<file>:<line>` for every parity value.

## Review Focus

1. **The installed app puts its own dir first on every terminal's `PATH`** (M5 D8). Installed in `/usr/bin`, that would push `/usr/bin` ahead of `~/.local/bin` and friends. The package installs both binaries to `/usr/lib/cmux-linux/` with symlinks in `/usr/bin`, so only that dir is prefixed. Check: Task 5 Step 5 (`namcap` and the file list), Task 8 Step 1 (`echo $PATH` in the installed app).
2. **The app started from a terminal of cmux or of another cmux-linux** inherits that terminal's `CMUX_*` and passes them to its shells, so hooks talk to the wrong app. Inherited `CMUX_*` names are removed before ours are set. Test: Task 4 `stale_cmux_keys_drops_inherited_only`, `spawned_shell_has_no_inherited_cmux_vars`.
3. **A process the user can't inspect on macOS** (root's `sudo python3 -m http.server` in a pane, or a process that exits mid-scan). Its ports are skipped; other ports and the rest of the sidebar still show; nothing panics. Test: Task 2 `cwd_of_unreadable_pid_is_none`, `listening_ports_finds_own_listener`.
4. **On macOS, a shell exits while a background job still holds the PTY** (`some-server &` then `exit`). The pane closes within the same bound as Linux (2 s). Test: Task 3, the existing test ported.
5. **Nineteen panes flood output while the user types in the twentieth.** Typing stays within two frames and memory stays bounded by scrollback. Check: Task 7 targets.

## Decisions made in this plan

| Decision | Why |
|---|---|
| **D1 macOS metadata via libproc**, as cmux does (`Sources/ListeningPortLookup.swift`, `Sources/TerminalWorkingDirectoryResolver.swift`): cwd = `proc_pidinfo(PROC_PIDVNODEPATHINFO)`; tty = `proc_bsdinfo.e_tdev` (`PROC_PIDTBSDINFO`); pids on a tty = `proc_listpids(PROC_TTY_ONLY = 3, tdev)`; ports = `PROC_PIDLISTFDS`, then `proc_pidfdinfo(PROC_PIDFDSOCKETINFO = 3)` on socket fds, kind TCP (`SOCKINFO_TCP = 2`), state `TSI_S_LISTEN = 1`, port = `insi_lport` big-endian. libc 0.2.189 lacks `socket_fdinfo`, `PROC_PIDFDSOCKETINFO`, `PROC_TTY_ONLY`, `SOCKINFO_TCP`, `TSI_S_LISTEN`: mirror them as `#[repr(C)]` from the SDK's `<sys/proc_info.h>` in `darwin.rs` (struct layouts only). | Not `lsof`: cmux dropped it, it sweeps every fd number and forks, every 1.5 s. A short read means "skip", as cmux. |
| **D2 Exit watch on macOS** = a kqueue with one `EVFILT_PROC`/`NOTE_EXIT` filter (`EV_ADD | EV_ONESHOT`) on the shell pid, polled like the pidfd. | Same reader loop, one more `cfg` arm. |
| **D3 Inherited `CMUX_*`** (every variable whose name starts with `CMUX_`) is removed from the shell's environment unless we set it. `CMUXTERM_*` (CLI tuning) passes through. | M5 journal: launching from a cmux terminal leaked its hook shims and token. |
| **D4 macOS blur:** Ghostty `background-blur` on macOS → `WindowBackgroundAppearance::Blurred` (gpui's `NSVisualEffectView`). Linux keeps `Transparent` + our KWin shim. | One line; gpui already does it. |
| **D5 Install layout:** `/usr/lib/cmux-linux/cmux-linux` (the `cmux-app` binary, renamed at install) and `/usr/lib/cmux-linux/cmux`; symlinks `/usr/bin/cmux-linux`, `/usr/bin/cmux`; `/usr/share/applications/cmux-linux.desktop`; `NOTICE`, `vendor/LICENSE-MIT`, the Lucide licence and the font licence in `/usr/share/licenses/cmux-linux/`. `current_exe()` resolves the symlink, so `PATH` gets `/usr/lib/cmux-linux`. | Review Focus 1. The `.desktop` name equals the Wayland `app_id` and the notification `desktop-entry` (`cmux-linux`), so KDE ties window, taskbar icon and bubbles to it. |
| **D6 PKGBUILD shape:** `pkgname=cmux-linux-git`, `pkgver()` from `git describe --long --tags` (fallback `r<count>.<hash>`), sources: `git+${_repo:-https://github.com/Izuulo/cmux-linux.git}`, `git+https://github.com/manaflow-ai/ghostty.git` (submodule wired in `prepare()` with `git config submodule.vendor/ghostty.url "$srcdir/ghostty"`), and Zig's `zig-x86_64-linux-0.16.0.tar.xz` with its sha256 (below). `arch=('x86_64')`, `makedepends=('git' 'rustup' 'clang')`, `depends=('fontconfig' 'freetype2' 'libxkbcommon' 'libxkbcommon-x11' 'wayland' 'vulkan-icd-loader' 'libx11' 'libxcb' 'gcc-libs' 'glibc')` (namcap corrects this list). `prepare()` runs `cargo fetch --locked`; `build()` puts the unpacked Zig on `PATH` and runs `cargo build --release --locked -p cmux-app -p cmux-ipc`. Icon: `Icon=utilities-terminal` (theme icon, no asset). Publishing to the AUR and tagging a release are the owner's call, not tasks. | `-git` builds without a release tag; `rustup` honours `rust-toolchain.toml` (1.99.0) where Arch's `rust` might lag; `_repo` lets CI build the checked-out commit. Network in `build()` (Ghostty's Zig deps) works under plain `makepkg`. |
| **D7 Licence gate:** `deny.toml` with `[licenses] allow = [...]` (the GPL-3.0-compatible set that `cargo deny check licenses` reports for today's tree), `confidence-threshold = 0.9`; anything else fails CI. Result recorded in `NOTICE`'s header line ("all Rust dependencies: see deny.toml"). | Closes the STATUS carry item. |
| **D8 Performance targets** (spec gives none): with 19 panes running `while :; do seq 100000; done` and the 20th taking keystrokes, 60 s in: key→paint p50 ≤ 16.7 ms and p95 ≤ 33 ms (`CMUX_TRACE_LATENCY=1`, ≥ 50 keys); app RSS ≤ 1 GiB (scrollback is 10 000 lines × 20); after Ctrl+C to all 19, app CPU < 5 % within 5 s. Idle (one pane): p95 ≤ 16.7 ms, which is also M1 exit 3 when run on KDE. | One frame at 60 Hz idle, two under load. |
| **D9 Parity method:** both apps on this Mac, same Ghostty config, same window size (1100 × 700 pt), same layout (2 workspaces, 2 × 2 splits, one tab strip with 3 tabs, one unread notification). Screenshot pairs per area; values read from the Swift source win over pixel guesses. | gpui draws the same on both OSes, so a Mac pair is a fair reference; KDE gets a spot check in Task 8. |

## Deferred (with reason)
- M5's deferred CLI commands (`identify`, `rpc`, `list-notifications`, `clear-notifications`), capability tokens, socket settings UI: not in this spec. Add them only with the owner's OK to the spec.
- Blur on Plasma ≤ 6.6 (`org_kde_kwin_blur`): only if Task 8 finds the desktop's KWin lacks `ext_background_effect_v1`.
- The gpui debug-build hang on macOS, a `.app` bundle, an aarch64 Linux package.

## File structure

| File | Responsibility |
|---|---|
| `scripts/install-zig.sh` | macOS rows, `shasum` (T1) |
| `.github/workflows/ci.yml` | `macos` job (T1), licence gate and `arch` job (T5) |
| `crates/cmux-meta/src/darwin.rs` (new), `lib.rs`, `Cargo.toml` | libproc cwd, tty, ports; `collect` dispatch (T2) |
| `crates/cmux-term/src/pty.rs`, `tests/pty.rs` | macOS cwd, kqueue exit watch, `CMUX_*` scrub (T2–T4) |
| `crates/cmux-app/src/main.rs` | macOS blur (T4) |
| `packaging/arch/PKGBUILD`, `packaging/cmux-linux.desktop`, `deny.toml` (new) | package and licence gate (T5) |
| `scripts/perf-panes.sh` (new) | 20-pane load, RSS and CPU readout (T7) |
| `crates/cmux-app/src/{chrome,attention}.rs`, `window_view/{sidebar,panel}.rs`, `workspace/tab_strip.rs` | parity constants (T6) |
| `README.md`, `NOTICE`, milestone journal, `STATUS.md` | docs (T5, T8) |

---

### Task 1: macOS dev setup and CI

**Files:** Modify `scripts/install-zig.sh`, `.github/workflows/ci.yml`.

- [ ] **Step 1: Add the macOS rows and drop `sha256sum`.** Rows `Darwin-arm64` → `aarch64-macos`, sha `b23d70deaa879b5c2d486ed3316f7eaa53e84acf6fc9cc747de152450d401489`; `Darwin-x86_64` → `x86_64-macos`, sha `0387557ed1877bc6a2e1802c8391953baddba76081876301c522f52977b52ba7`. Check with `shasum -a 256 -c -` (macOS and Ubuntu both ship it). Remove the `shortcut:` line.
- [ ] **Step 2: Run it on the Mac.** `scripts/install-zig.sh && .tools/zig/zig version` → `0.16.0`. Then `export PATH="$PWD/.tools/zig:$PATH"; cargo test --workspace --locked --release --features gpui/runtime_shaders` → all pass (record any macOS-only failure in the journal and fix it here).
- [ ] **Step 3: Add a `macos` job** on `macos-15`: checkout with submodules, toolchain 1.99.0, rust-cache, `scripts/install-zig.sh` + `$GITHUB_PATH`, then `cargo build --workspace --locked --release --features gpui/runtime_shaders` and the same with `test`. Release, because of the debug hang.
- [ ] **Step 4: Commit, push, `gh run watch`.** Expected: `linux` and `macos` green.

### Task 2: macOS cwd and listening ports

**Files:** Create `crates/cmux-meta/src/darwin.rs`; modify `crates/cmux-meta/src/lib.rs`, `crates/cmux-meta/Cargo.toml` (`[target.'cfg(target_os = "macos")'.dependencies] libc.workspace = true`), `crates/cmux-term/src/pty.rs`.

**Interfaces:**
- Produces (`cmux_meta::darwin`, macOS only): `pub fn cwd(pid: u32) -> Option<String>`, `pub fn tty_of(pid: u32) -> Option<i32>` (`None` for no tty), `pub fn listening_ports(ttys: &HashSet<i32>) -> HashMap<i32, BTreeSet<u16>>` — the same shapes as `ports::tty_of`/`ports::listening_ports` minus `proc_root`. Private: `fn ports_of_pid(pid: u32) -> BTreeSet<u16>` (empty on any error).
- `collect(inputs, proc_root, home)` keeps its signature; on macOS it ignores `proc_root` and calls `darwin`. Its Linux fixture tests become `#[cfg(target_os = "linux")]`.
- `cmux_term::process_cwd` on macOS uses the same `PROC_PIDVNODEPATHINFO` call (12 lines duplicated rather than a `cmux-term → cmux-meta` edge).

- [ ] **Step 1: Failing tests in `darwin.rs`** (`#[cfg(test)]`):
  - `cwd_of_self_is_current_dir`: `cwd(std::process::id())` == `std::env::current_dir()`.
  - `cwd_of_unreadable_pid_is_none`: `cwd(1)` is `None` (launchd, root) and `cwd(u32::MAX)` is `None`.
  - `listening_ports_finds_own_listener`: bind a `TcpListener` on `127.0.0.1:0`; the private `ports_of_pid(std::process::id())` contains its port, and `ports_of_pid(1)` is empty (EPERM skipped, no panic). The tty → pids hop is covered by Step 5.
  - `tty_of_gone_pid_is_none`: `tty_of(u32::MAX)` is `None`.
- [ ] **Step 2: Run** `cargo test -p cmux-meta --locked` on the Mac → fail (no `darwin` module).
- [ ] **Step 3: Implement** per D1. `listening_ports`: for each tty, `proc_listpids(PROC_TTY_ONLY, tty)`, then `ports_of_pid` for each pid; a pid that errors (EPERM, gone, short read) contributes nothing. Wire `collect` and `cmux_term::process_cwd`.
- [ ] **Step 4: Run** `cargo test -p cmux-meta -p cmux-term --locked` on the Mac → pass.
- [ ] **Step 5: Run it.** Release build; in a pane `cd /tmp && python3 -m http.server 8765`; the sidebar shows `/tmp` (no OSC 7 needed) and port 8765 within 2 s. Screenshot `sidebar-ports-mac.png`.
- [ ] **Step 6: Commit; push; CI green on both jobs.**

### Task 3: Shell exit on macOS while a job holds the PTY

**Files:** Modify `crates/cmux-term/src/pty.rs` (`pidfd` → `exit_watch`), `crates/cmux-term/tests/pty.rs`.

- [ ] **Step 1: Port the test.** In `shell_exit_ends_the_session_even_if_a_setsid_grandchild_holds_the_pty`, replace `setsid -f sleep 5` with `(trap '' HUP; exec sleep 5) &` (works without `setsid`, which macOS lacks) and drop its `cfg(linux)`. The `keeps_writing` test stays Linux-only (`timeout` is GNU).
- [ ] **Step 2: Run** `cargo test -p cmux-term --locked --test pty shell_exit_ends` on the Mac → FAIL (takes ~5 s). On Linux CI it must still pass.
- [ ] **Step 3: Implement D2** as the `cfg(target_os = "macos")` arm of `exit_watch(pid: u32) -> Option<OwnedFd>` (rename of `pidfd`; the reader already polls the fd). Update the module doc and remove the `shortcut:` comment.
- [ ] **Step 4: Run** the test → PASS (< 2 s) on the Mac; push; CI green.
- [ ] **Step 5: Commit.**

### Task 4: No inherited `CMUX_*`; blur on macOS

**Files:** Modify `crates/cmux-term/src/pty.rs` (where the `CommandBuilder` gets `env`), `crates/cmux-term/tests/pty.rs`, `crates/cmux-app/src/main.rs`.

**Interfaces:** Produces `fn stale_cmux_keys(inherited: impl IntoIterator<Item = OsString>, ours: &[(String, String)]) -> Vec<OsString>` (private to `pty.rs`); the spawn calls `cmd.env_remove(k)` for each before applying `env`.

- [ ] **Step 1: Failing unit test** `stale_cmux_keys_drops_inherited_only`: inherited `["PATH", "CMUX_SURFACE_ID", "CMUX_CLAUDE_HOOK_SHIM", "CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC"]`, ours `[("CMUX_SURFACE_ID", "x")]` → `["CMUX_CLAUDE_HOOK_SHIM"]`.
- [ ] **Step 2: Integration test** `spawned_shell_has_no_inherited_cmux_vars` in `tests/pty.rs`: run `env` in a session spawned with `env = [("CMUX_SURFACE_ID","ours")]`; the output has `CMUX_SURFACE_ID=ours` and no other `CMUX_` line. (No `set_var` in tests: it is `unsafe` with threads. Run from a cmux or cmux-linux terminal, the harness inherits real `CMUX_*`, so it fails before Step 3; record whether it did.)
- [ ] **Step 3: Run both** → FAIL. **Implement D3. Run** → PASS.
- [ ] **Step 4: D4** in `main.rs`: `background_blur` on macOS → `Blurred` (when `background_opacity < 1.0` too, as on Linux; opaque stays `Opaque`). Release build with `background-opacity = 0.8` and `background-blur = true` in a temp Ghostty config (`XDG_CONFIG_HOME`); the window blurs. Screenshot `blur-mac.png`.
- [ ] **Step 5: Commit; push; CI green.**

### Task 5: Linux package and licence gate

**Files:** Create `packaging/arch/PKGBUILD`, `packaging/cmux-linux.desktop`, `deny.toml`; modify `.github/workflows/ci.yml`, `NOTICE`, `README.md`.

- [ ] **Step 1: Licence audit.** `cargo install cargo-deny --locked` (dev tool, not a dependency), write `deny.toml` per D7 with the licences `cargo deny check licenses` lists; re-run → `licenses ok`. Any licence that isn't GPL-3.0-compatible stops the task: record it in the journal and ask the owner.
- [ ] **Step 2: `.desktop`.** `Type=Application`, `Name=cmux-linux`, `GenericName=Terminal`, `Comment=Terminal workspace for coding agents`, `Exec=cmux-linux`, `Icon=utilities-terminal`, `Terminal=false`, `Categories=System;TerminalEmulator;`, `StartupWMClass=cmux-linux`. Check: `desktop-file-validate packaging/cmux-linux.desktop` (in the Arch job) → no output.
- [ ] **Step 3: PKGBUILD** per D5/D6 (`license=('GPL-3.0-or-later')`; `package()` installs the files of D5 with `install -Dm755`/`-Dm644` and `ln -s`).
- [ ] **Step 4: CI job `arch`** in `archlinux:base-devel`: `pacman -Syu --noconfirm git rustup clang namcap desktop-file-utils` plus `depends`; `rustup default 1.99.0`; a non-root `builder` user runs `_repo="git+file://$GITHUB_WORKSPACE" makepkg -s --noconfirm` in a copy of `packaging/arch`; then `namcap *.pkg.tar.zst`, `pacman -Qlp *.pkg.tar.zst` contains `/usr/lib/cmux-linux/cmux` and `/usr/bin/cmux-linux`, and `desktop-file-validate`. Add `cargo deny check licenses` (EmbarkStudios/cargo-deny-action, pinned to a release tag) to the `linux` job.
- [ ] **Step 5: Push; `gh run watch`** → `arch` green; read namcap's output and fix `depends` until it reports no missing or unneeded dependency (warnings about `-git` naming are fine).
- [ ] **Step 6: Docs.** README "Install" section: Arch/CachyOS (`cd packaging/arch && makepkg -si`), macOS (release build command, run `target/release/cmux-app`). NOTICE: the licence-gate line. Commit.

### Task 6: Visual parity pass (Mac, against `cmux.app`)

**Files:** constants in `crates/cmux-app/src/chrome.rs`, `attention.rs`, `window_view/sidebar.rs`, `window_view/panel.rs`, `workspace/tab_strip.rs`, colours in `colors.rs`; journal.

- [ ] **Step 1: Set up per D9.** Note the `cmux.app` version (About) and the cmux source commit. Same layout in both apps (our side via the `cmux` CLI). Pairs to `~/Pictures/cmux-m6/`: `window`, `sidebar`, `tab-strip`, `splits`, `ring`, `panel`, `flash.mov` (screen recording, both rings flashing).
- [ ] **Step 2: Write the checklist in the journal**, one row per item: area, our value (file:const), cmux value (`Sources/<file>:<line>`), match ✓ / fixed / gap + reason. Areas and items, at least:
  - sidebar: width (default, min, max), row height and padding, title/detail font sizes and weights, selected-row fill and radius, badge size and colour, branch/dir/port lines, top band and buttons;
  - tab strip: height, tab min/max width, padding, title size, active indicator, unread dot, close glyph and hover;
  - splits: divider thickness and colour, hit area, unfocused-pane dimming;
  - ring: colour, inset, radius, stroke, flash duration and curve, tab highlight;
  - panel: width, row layout, empty state;
  - window: background and chrome colours with the default theme and with one dark Ghostty theme.
- [ ] **Step 3: Fix each mismatch** in the const that holds it, one commit per area (`parity(sidebar): …`). A gap that needs new behaviour (not a value) is a journal entry, not a fix, unless it is under ~30 lines.
- [ ] **Step 4: Retake the pairs** and tick the checklist. Push; CI green.

### Task 7: Performance check

**Files:** Create `scripts/perf-panes.sh`; journal.

- [ ] **Step 1: Write the script.** Run from a pane of the app under test (cmuxOnly lets descendants in). It builds 4 rows × 5 columns in the current workspace with `cmux new-split down` ×3 then `cmux new-split right --surface <ref>` ×4 per row (refs from `cmux list-pane-surfaces --json`), starts D8's loop in every surface but the caller with `cmux send`, sleeps 60 s, prints the app's RSS and %CPU (`ps -o rss=,%cpu= -p "$APP_PID"`, `APP_PID` defaulting to `pgrep -nx cmux-app || pgrep -nx cmux-linux`), sends `ctrl+c` to the 19 with `cmux send-key`, sleeps 5 s, prints %CPU again. Exits 1 with a message if a split fails (`invalid_state: Not enough space to split` → enlarge the window).
- [ ] **Step 2: Run on the Mac.** Start the app with `CMUX_TRACE_LATENCY=1 target/release/cmux-app 2> ~/Pictures/cmux-m6/latency.log`, full-screen. Run the script; while the loops run, type ≥ 50 keys in the caller pane. Compute p50/p95 from the `latency key->paint` lines. Also the idle run (one pane).
- [ ] **Step 3: Record** in the journal a table: host, panes, p50, p95, RSS, CPU after stop, verdict per D8 target.
- [ ] **Step 4: Fix any missed target** with superpowers:systematic-debugging (profile first: `samply record` on macOS). Re-run and record. Commit script and fixes.

### Task 8: KDE desktop run, docs, merge

Owner at the CachyOS desktop.

- [ ] **Step 1: Install** with `cd packaging/arch && makepkg -si`. Start from the KDE launcher. Checks: taskbar icon and name, `echo $PATH | cut -d: -f1` in a pane → `/usr/lib/cmux-linux`, `command -v cmux` → `/usr/lib/cmux-linux/cmux`, `cmux notify --title hi` → ring and a KDE bubble attributed to cmux-linux.
- [ ] **Step 2: Repeat Task 7 Step 2** on KDE (this also measures M1 exit 3); record.
- [ ] **Step 3: Parity spot check on KDE:** retake `window` and `ring`; compare with the Mac pair; note any Linux-only difference (fonts, scale, blur). Check that KWin offers `ext_background_effect_v1` (`wayland-info | grep background_effect`); if not, add the Plasma ≤ 6.6 blur item to STATUS.
- [ ] **Step 4: The pending desktop checks** from STATUS (M5 Task 12 Step 1, M4 Task 9 Step 1, M2 Task 11, M3 Task 12 Steps 1–3), each recorded in its journal.
- [ ] **Step 5: Docs.** Journal entry with the exit criteria and their evidence; README milestone table; archive and replace `STATUS.md` (M6 done or what remains; carry items closed by this plan removed: macOS ports/cwd, pidfd, macOS blur, licence audit, CMUX_* leak, install-zig macOS row). Commit, push, PR, CI green, merge on the owner's OK.
