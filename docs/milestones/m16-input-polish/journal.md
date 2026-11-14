# M16 input polish — Journal

Append-only session notes.

## 2026-11-14 — Built (plan Tasks 1–9)
- **Findings while planning** (Ghostty source, `vendor/ghostty`):
  - Ghostty's "drag threshold" is per cell, not a pixel distance (`SelectionGesture.zig`
    `dragSelection`): a cell joins a drag selection once the pointer crosses 60 % of its width,
    measured against the press position inside its cell. Ported as `cmux_term::gesture::cell_span`
    (non-rectangular; Alt rectangle selection is not ported).
  - Shift-click extend (`Surface.zig` ~5801): Shift + left press, a selection exists, not a quick
    repeat → a drag to the press from the original anchor, in the original mode. "Not a quick
    repeat" = gpui `click_count == 1` (gpui's 400 ms / 5 px rule, Ghostty uses its own interval).
  - Autoscroll: 1 px edge, one row per 15 ms tick (`termio/Thread.zig`), stops on release.
  - gpui delivers `on_mouse_move` only while hovered, so drags that leave the pane now come in
    through a window-level listener registered in `TerminalElement::paint`.
  - Wheel: gpui's Wayland backend triples touchpad pixels and makes a notch 3 lines on both
    axes, and turns Shift+wheel into horizontal. Ghostty takes touchpad pixels as they come and
    sends one report per horizontal notch, so Linux divides by 3 (`GPUI_AXIS_FACTOR`).
    Horizontal goes only to mouse-tracking apps (buttons 6/7); Ghostty has no horizontal
    scrollback. The touchpad *feel* is still an owner check.
  - Resize: `Session::resize` already holds the `Term` lock only for `vt.resize` (the reflow,
    which is the state swap the next frame needs) and does the ioctl afterwards. No change.
  - Blink: we draw no SGR 5 text blink, so only a focused blinking cursor needs the timer.
- **Deviations:** underline position and 1 px thickness are gpui's (baseline + 0.618 descent),
  not the font's underline metrics (gpui 0.2.2 exposes none); dots are rounded quads. The
  preedit background is opaque default bg (Ghostty skips the cells under it; with a translucent
  window the text below would show through).
- **Scripted exits** (`scripts/click-through.py`, headless KWin, new socket read
  `debug.terminal.state` in debug builds: selection text, scroll offset, viewport lines, blink
  timer, cell geometry; `debug.mouse.simulate` gained `shift` and `dx`). 2026-11-14, 17/17 PASS:
  - Exit 1 `drag_threshold`: a drag within the first cell up to 50 % selects nothing; on to
    60 %+ of the fifth cell selects `hello`; double-click `hello`, Shift-click on `world` →
    `hello world`.
  - Exit 2 `autoscroll`: held below the pane, offset 161 → 198 in 0.6 s; unchanged after release.
  - Exit 3 `horizontal_wheel`: `cat -v` under `?1000h ?1006h` shows `<67;` (right) and `<66;`.
  - Exit 4: unit tests `single_and_double_bars`, `dashes_follow_ghostty_widths`,
    `dots_are_round_and_at_least_one`, `preedit_*` (3), `candidate_offset_counts_cells_not_code_units`.
  - Exit 5 `idle_blink`: DECSCUSR steady → no blink task; blinking → task; steady → gone. With
    no task, nothing wakes the view (a structural check, not a wake-up count).
- **Found by script:** the sidebar's resize grip overlaps the terminal's column 0 by a few
  pixels, so a press there starts a sidebar drag, not a selection. Not fixed (out of scope);
  the check selects from column 6. Recorded in STATUS.
- **Lesson:** a synthetic wheel line is 1/3 of a real gpui notch; scripts send `dx=3` for one.
