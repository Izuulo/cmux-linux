# m1-terminal — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-03 — M1 built, reviewed and checked on the desktop
**Done.** Thirteen planned tasks, each with its own review:
- Toolchain pin.
- Ghostty config parser and loader (includes, themes, `~/`).
- `cmux-term`: Term wrapper with DA/DSR replies, bell and title events, resize and scroll.
- PTY session: non-blocking drop, process-group SIGHUP then SIGKILL.
- Tracked-point selection and paste encoding.
- gpui key mapping and colour resolution.
- Terminal view and element, mouse, IME.

**Desktop check.** The user ran the release build on KDE Plasma Wayland and confirmed it works as a terminal (Claude Code, everyday shell use). The malformed-config check passed: both warnings were printed with `file:line`, the window still opened, and no stray process was left. Numeric cell-size and latency measurements and the screenshot pair were not recorded this session. Redo them if a visual or latency regression is suspected.

**Ghostty fidelity decisions** (deliberate, matched to Ghostty source rather than cmux Swift):
- Ctrl+[ / Ctrl+I / Ctrl+M send CSI-u (`ESC[91;5u` …), Ghostty's fixterms behaviour (key_encode.zig ~847).
- Unset selection colours invert the defaults. An unset cursor uses the default fg as fill and the default bg as glyph.
- Invisible text skips its glyph and all decorations. `background-opacity-cells` is honoured.
- Click count caps at 3. Padding snaps to device pixels.
- Mode 1007 (alternate scroll) is handled in the app, as libghostty-vt leaves it to the surface.

**Deviations and known gaps:**
- `line_gap` is 0, because gpui 0.2.2 has no public FontMetrics. Fonts with a large line gap get slightly shorter cells than Ghostty.
- Underline styles are simplified: double, dotted and dashed all draw as a single 1px line.
- No blur (deferred to M3, see M0 journal).
- Touchpad scroll may run faster than Ghostty, because gpui's Wayland backend multiplies pixel deltas.
- No drag threshold, shift-click extend or autoscroll.
- The IME was tested only through gpui's text-input path, not with fcitx/ibus.
- The full list of minor findings carries in STATUS.md under "Carry to M2+".

## 2026-11-03 — Final branch review
- **Verdict.** Opus whole-branch review returned Ready after fixes, with no critical issues.
- **Fixed before merge:**
  - `libc` added to NOTICE.
  - `copy` now releases the Term lock before writing the clipboard.
- **Exit criteria.** Criterion 1 is met by the user's desktop check. Criteria 2 (Ghostty screenshot pair) and 3 (latency trace) are not yet measured. They are carried in STATUS and must not be reported as verified until they are recorded here.
- **Promoted to M2:**
  - PTY reader wake pipe (`setsid` grandchild → stuck pane).
  - Weak-handle callbacks.
  - Split prepaint.
  - Remappable bindings.
  - Ctrl+Shift+C passthrough when nothing is selected.
  - Paste protection.
- **Carried to M6:** `feed` panic guard.
