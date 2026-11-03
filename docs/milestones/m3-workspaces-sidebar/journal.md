# m3-workspaces-sidebar — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-03 — Research and plan

- Branch `m3-workspaces-sidebar` starts from `m2-splits-tabs` (M2 exit checks still pending); rebase onto `master` once M2 merges.
- Research: `research-cmux.md` (cmux @ 21dcc2b Swift sources) and `research-gpui.md` (gpui 0.2.2 tarball, Zed main @ 8a47b35).
- Findings that shaped the plan: no published gpui has `ext_background_effect_v1` (Zed main added it 2026-10-30, with a platform-crate split and a wgpu renderer), so blur is a small Wayland shim on gpui's own surface; cmux's sidebar shares the terminal backdrop by default, so blur is window-wide and follows Ghostty `background-blur`; cmux gets PRs from the GitHub REST API (not `gh pr view`), and its PR rows are not state-coloured.
- Decisions: PRs via `gh pr list --json` with cmux's selection rules and cadence (spec says "via gh"); ports by controlling TTY from `/proc`; git branch by re-reading `.git/HEAD` every 1.5 s instead of watchers; Super+Shift+R renames inline (no command palette until M11).
- The plan's code was not compiled (written on a host without the Linux toolchain), unlike M2's; executors compile per task.

## 2026-11-03 — Tasks 1–11 built (subagent-driven); desktop checks pending

- Built on a macOS host: rustup 1.99.0, Zig 0.16.0 (aarch64-macos, in `.tools/`), and gpui with `--features gpui/runtime_shaders` (the Metal Toolchain component is not installed). Linux CI is the build/test/clippy gate for every push. The last green run is on the final fix wave.
- Each task had one implementer and one task review. There were fix rounds on Tasks 7 (a hidden workspace could take focus: `focus_active` is now gated on `active`), 8 (a failed new shell reselected a dead id), 10 (rename used the terminal font; Alt-Tab committed a half-typed name; ghost accent) and 11 (SAFETY comments misstated the drop order).
- The whole-branch review led to one fix wave:
  - a missing panel directory no longer turns off PR polling;
  - the M2 tab drop bar now accounts for the sidebar inset;
  - the window title is re-sent when switching back to a workspace;
  - the `/proc/<pid>/cwd` fallback moved off the UI thread into `collect`;
  - the sidebar reads cached titles, so hidden workspaces don't re-render the window;
  - Enter on an unchanged fallback title doesn't freeze it as a name;
  - closing a background workspace keeps focus where it is.
- Rulings:
  - The test literals for chords follow M2's modifier order (`ctrl-alt-shift-super`).
  - When a new workspace's shell fails, the previously selected workspace is reselected. The plan's "slid next to" wording was wrong about the model.
  - The PR line keeps the UI font. rc §3 makes only the directory and ports lines monospaced.
  - Sidebar row ids are WorkspaceId-based, so hover and clicks survive a reorder.
  - `WindowBlur::enable(window, cx)` checks `cx.compositor_name() == "Wayland"` first, because gpui 0.2.2's X11 window panics on raw handles. It is called from `WindowView::new`.
  - Rename blur is ignored while the window is inactive.
- Deviations from cmux:
  - PR data comes from `gh pr list --json`, not GitHub REST.
  - Super+Shift+R renames inline; there is no command palette before M11.
  - No git dirty `*`.
  - Polling instead of file watchers, and no shell-integration kicks.
  - Blur covers the whole window and only turns on with Ghostty `background-blur`.
  - The toggle and new-workspace buttons sit in a band at the top of the sidebar, not the titlebar.
  - Sidebar width, visibility and order are not persisted.
- Pending at the desktop (plan Task 12 Steps 1–3):
  - the "Run it" steps of Tasks 7–11;
  - exit 1 (5+ workspaces, about 2 s updates, Plasma Meta+ conflicts);
  - exit 2 (screenshot pair; a Mac with cmux is available for a real reference);
  - exit 3 (typing while `gh` hangs);
  - blur runtime risks rg §1.4 (1)–(3).
- Still deferred, triaged by the final review:
  - `run_with_deadline` drain-thread join (needs a grandchild holding the pipe);
  - keyboard move doesn't scroll the row into view;
  - drop line 1 px off and unclipped when scrolled;
  - long-name caret;
  - capability bit and kde-blur coexistence (M6).
