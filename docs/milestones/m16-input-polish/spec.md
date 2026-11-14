# M16 input polish — Spec

**Goal:** mouse, IME and terminal drawing behave like Ghostty, closing the input and
render gaps carried since M1 ("Carry to M4+" in `STATUS.md`).

## Scope
- **Selection gesture:** port Ghostty's `SelectionGesture.zig` (MIT, `vendor/ghostty`)
  into a pure `cmux-term::gesture` module with no gpui: click count, drag threshold
  (a press starts a selection only after the pointer moves past it), Shift-click extend
  in the current mode (cell/word/line), and autoscroll ticks while a drag is above or below
  the viewport. `cmux-app/src/mouse.rs` becomes glue: it feeds gpui events in and applies
  the result. The autoscroll timer is a gpui task alive only while a drag is outside the view.
- **Horizontal wheel:** reported to mouse-tracking apps as buttons 6/7; otherwise ignored
  (Ghostty has no horizontal scrollback).
- **Touchpad rate:** pixel-to-line conversion matches Ghostty's; gpui's Wayland scaling
  of pixel deltas is undone in `wheel_lines`. Covered by unit tests on the math.
- **IME:** the preedit draws on the cell background and is clamped to the pane width;
  `bounds_for_range` returns the exact cell rectangle of the range.
- **Underlines:** double, dotted, dashed and curly drawn with Ghostty's geometry.
- **Render hot spots:** one `Font` per style run, not a clone per cell in prepaint; the
  blink task wakes only while a blinking cursor or blinking text is visible; `resize`
  does not hold the `Term` lock on the GUI thread beyond the state swap.

## Out of scope
- Real-hardware checks: physical touchpad feel, real IME (fcitx/ibus). They stay owner
  checks in `STATUS.md` and do not gate this milestone.
- `ESC[3J` epoch over-invalidation, title/bell coalescing, `line_gap`, gpui upgrade.
- New features beyond what Ghostty already does.

## Exit criteria
All scripted (`debug.mouse.simulate` in the headless KWin) or unit-tested:
1. A drag shorter than the threshold makes no selection; a longer drag selects;
   Shift-click after a double-click extends by words.
2. A drag held below the pane autoscrolls the scrollback; scrolling stops on release.
3. A horizontal wheel reaches a mouse-tracking app as button 6/7 reports.
4. Unit tests cover underline geometry for each style and the IME preedit clamp.
5. With no blinking cursor or text visible, the idle app takes no blink wake-ups
   (counted by a test hook or trace over 5 s).
