# M16 input polish — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** mouse, IME and terminal drawing behave like Ghostty (spec.md).

**Architecture:** Ghostty's selection rules (60 % cell threshold, autoscroll direction) become
pure functions in `cmux-term::gesture`; `cmux-app/src/mouse.rs` stays the glue and keeps the
existing `Selection` (word/line drags already work). Underline geometry, the preedit range and
the blink decision are pure functions next to their callers, each unit-tested. Scripted exits
use a new debug-only socket read, `debug.terminal.state`.

**Tech Stack:** Rust, gpui 0.2.2 (vendored), libghostty-vt (`cmux_term::ghostty_vt`),
Python click-through in headless KWin.

## Global Constraints
- GPL-3.0-or-later; Ghostty is MIT (already in `NOTICE`). Any new direct dependency goes in `NOTICE`.
- Match Ghostty source (`vendor/ghostty`) where the spec says "like Ghostty"; record deviations in `journal.md`.
- `export PATH="$PWD/.tools/zig:$PATH"` before any cargo command.
- No personal info in tracked files. Every commit ends with the session's attribution lines.

## Findings that shape the plan (read before Task 1)
- **Ghostty's "drag threshold" is per cell, not a pixel distance** (`SelectionGesture.zig`
  `dragSelection`): a cell joins the selection when the drag crosses 60 % of its width
  (`round(cell_w * 0.6)`), measured from the press position inside the cell. A press and a drag
  that stay on the same side of that point select nothing.
- **Shift-click extend** (`Surface.zig` ~5801): left press with Shift, a selection exists, the
  gesture has a click, and the press is not a quick repeat → handled as a drag to the press
  point from the original anchor. Our test for "not a quick repeat": gpui `click_count == 1`.
- **Autoscroll** (`SelectionGesture.drag`, `termio/Thread.zig`): drag `y <= 1` px from the top
  of the surface → up; `y > height - 1` → down; a 15 ms timer scrolls the viewport 1 row per
  tick and re-runs the drag at the last pointer position; release stops it.
- **gpui delivers `on_mouse_move` only while hovered**, so a drag that leaves the pane needs a
  window-level `window.on_mouse_event::<MouseMoveEvent>` registered in `TerminalElement::paint`.
- **Wheel** (`Surface.zig scrollCallback`): precision (touchpad) y pixels × 1, discrete y
  ticks × 3 lines; x: discrete ticks → one report each, precision x pixels / `cell_w`. x only
  ever goes to the app (buttons 6/7 = `WheelLeft`/`WheelRight`), never scrolls the viewport.
  gpui Wayland multiplies continuous axis values by 3 (`client.rs` `let modifier = 3.0`) and
  turns a discrete notch into 3 lines on either axis (`SCROLL_LINES`). Shift+wheel arrives as
  x (gpui swaps the axis). So on Linux: y `Pixels` ÷ 3; x (both kinds) ÷ 3 after
  `pixel_delta(cell_w)`.
- **Preedit** (`renderer/State.zig Preedit.range`): placed at the cursor; if wider than the cells
  to the right edge it shifts left; if wider than the row, leading codepoints are dropped. The
  cells under it show no terminal content (Ghostty skips them), glyphs in the screen fg, single
  underline.
- **Underlines** (`font/sprite/draw/special.zig`): single = one bar; double = bars at
  `y - t` and `y + t`; dashed = dashes of `w/3 + 1` px, every other one; dotted = dots of
  radius `t/√2`, count `max(1, min(ceil(w/(4r)), floor(w/(3r)), floor(w/(2r+1))))`, evenly
  spaced; curly stays gpui's `wavy`. We keep today's line position (gpui: baseline +
  `descent * 0.618`) and thickness 1 px so single underlines do not move.
- **Resize**: `Session::resize` already holds the `Term` lock only for `vt.resize` (the reflow
  that *is* the state swap) and does the ioctl after; the next frame needs the reflowed state, so
  no code change. Journal it.
- **Blink**: we do not draw SGR 5 text blink (neither cell.blink path exists), so only the cursor
  blinks. The blink timer task should exist only while `focused && cursor_blinks`.

## File map
- Create `crates/cmux-term/src/gesture.rs` — `cell_span`, `Autoscroll`, `autoscroll`.
- Modify `crates/cmux-term/src/lib.rs` — `pub mod gesture;`
- Modify `crates/cmux-term/src/selection.rs` — `Selection::cells`; drop `is_empty_click`.
- Modify `crates/cmux-app/src/mouse.rs` — threshold, shift-extend, autoscroll, wheel.
- Modify `crates/cmux-app/src/terminal_view.rs` — new fields, blink task on demand.
- Modify `crates/cmux-app/src/terminal_element.rs` — window drag listener, underlines, faces, preedit.
- Modify `crates/cmux-app/src/ime.rs` — `bounds_for_range` over the placed preedit.
- Modify `crates/cmux-app/src/window_view/control.rs` — `debug.terminal.state`, `shift`/`dx` in `debug.mouse.simulate`.
- Modify `scripts/click-through.py` — three input checks.
- Modify `crates/cmux-app/Cargo.toml`, `NOTICE` — `unicode-width` (already in `Cargo.lock`).

---

### Task 1: `cmux-term::gesture` — Ghostty's cell threshold and autoscroll direction

**Files:** Create `crates/cmux-term/src/gesture.rs`; modify `crates/cmux-term/src/lib.rs`.

**Interfaces — Produces:**
- `pub fn cell_span(click: SelectionPoint, click_frac: f32, drag: SelectionPoint, drag_frac: f32, cols: u16) -> Option<(SelectionPoint, SelectionPoint)>` — ordered `(start, end)` inclusive, or `None` (no cell selected). `*_frac` in `[0, 1)`: position inside the cell.
- `pub enum Autoscroll { None, Up, Down }` (`Debug, Clone, Copy, PartialEq, Eq`)
- `pub fn autoscroll(y: f32, height: f32) -> Autoscroll` — `y` relative to the grid top.

- [ ] **Step 1: tests first** (in `gesture.rs` `mod tests`):

```rust
fn p(column: u16, row: u32) -> SelectionPoint { SelectionPoint { column, row } }

#[test]
fn same_cell_selects_only_after_crossing_sixty_percent() {
    // Press left of the threshold, drag still left of it: nothing.
    assert_eq!(cell_span(p(3, 0), 0.1, p(3, 0), 0.5, 80), None);
    // Drag crosses it: that one cell.
    assert_eq!(cell_span(p(3, 0), 0.1, p(3, 0), 0.7, 80), Some((p(3, 0), p(3, 0))));
}

#[test]
fn rightward_drag_includes_cells_by_threshold() {
    // Click right of the threshold excludes the click cell; drag left of it excludes the drag cell.
    assert_eq!(cell_span(p(2, 0), 0.8, p(6, 0), 0.2, 80), Some((p(3, 0), p(5, 0))));
    assert_eq!(cell_span(p(2, 0), 0.1, p(6, 0), 0.9, 80), Some((p(2, 0), p(6, 0))));
    // Adjacent cells, both excluded: nothing.
    assert_eq!(cell_span(p(2, 0), 0.8, p(3, 0), 0.2, 80), None);
}

#[test]
fn leftward_drag_mirrors_the_rule() {
    assert_eq!(cell_span(p(6, 0), 0.2, p(2, 0), 0.8, 80), Some((p(3, 0), p(5, 0))));
    assert_eq!(cell_span(p(6, 0), 0.9, p(2, 0), 0.1, 80), Some((p(2, 0), p(6, 0))));
}

#[test]
fn excluded_ends_wrap_across_rows() {
    // Click at the last column, right half: start wraps to the next row's first cell.
    assert_eq!(cell_span(p(79, 0), 0.9, p(5, 1), 0.9, 80), Some((p(0, 1), p(5, 1))));
    // Drag ends at column 0, left half: end wraps back to the previous row's last cell.
    assert_eq!(cell_span(p(2, 0), 0.1, p(0, 1), 0.1, 80), Some((p(2, 0), p(79, 0))));
}

#[test]
fn autoscroll_uses_a_one_pixel_edge() {
    assert_eq!(autoscroll(-20.0, 400.0), Autoscroll::Up);
    assert_eq!(autoscroll(1.0, 400.0), Autoscroll::Up);
    assert_eq!(autoscroll(200.0, 400.0), Autoscroll::None);
    assert_eq!(autoscroll(399.5, 400.0), Autoscroll::Down);
}
```

- [ ] **Step 2:** `cargo test -p cmux-term gesture` → fails to compile (no module).

- [ ] **Step 3: implement** (`gesture.rs`; port of `SelectionGesture.zig` `dragSelection`, non-rectangular, and the `drag` autoscroll rule; MIT header comment naming the source):

```rust
//! Ghostty's selection gesture rules (vendor/ghostty src/terminal/SelectionGesture.zig, MIT):
//! the within-cell threshold for cell drags and the autoscroll edge. Pure; the app owns timers.

use ghostty_vt::SelectionPoint;

/// Share of a cell's width the pointer must cross for that cell to join a drag selection.
const THRESHOLD: f32 = 0.6;
/// Ghostty's 1 px edge so fullscreen-edge drags still autoscroll.
const EDGE: f32 = 1.0;

fn key(p: SelectionPoint) -> (u32, u16) { (p.row, p.column) }

fn right(p: SelectionPoint, cols: u16) -> SelectionPoint {
    if p.column + 1 < cols { SelectionPoint { column: p.column + 1, ..p } }
    else { SelectionPoint { column: 0, row: p.row + 1 } }
}

fn left(p: SelectionPoint, cols: u16) -> SelectionPoint {
    if p.column > 0 { SelectionPoint { column: p.column - 1, ..p } }
    else if p.row > 0 { SelectionPoint { column: cols - 1, row: p.row - 1 } }
    else { p }
}

pub fn cell_span(
    click: SelectionPoint, click_frac: f32, drag: SelectionPoint, drag_frac: f32, cols: u16,
) -> Option<(SelectionPoint, SelectionPoint)> {
    if cols == 0 { return None; }
    let same = click == drag;
    let backwards = if same { drag_frac < click_frac } else { key(drag) < key(click) };
    let include_click = if backwards { click_frac >= THRESHOLD } else { click_frac < THRESHOLD };
    let include_drag = if backwards { drag_frac < THRESHOLD } else { drag_frac >= THRESHOLD };
    let start = match (include_click, backwards) {
        (true, _) => click,
        (false, true) => left(click, cols),
        (false, false) => right(click, cols),
    };
    let end = match (include_drag, backwards) {
        (true, _) => drag,
        (false, true) => right(drag, cols),
        (false, false) => left(drag, cols),
    };
    if (!include_click && (same || end == click)) || (!include_drag && start == drag) {
        return None;
    }
    Some(if key(start) <= key(end) { (start, end) } else { (end, start) })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Autoscroll { None, Up, Down }

pub fn autoscroll(y: f32, height: f32) -> Autoscroll {
    if y <= EDGE { Autoscroll::Up } else if y > height - EDGE { Autoscroll::Down } else { Autoscroll::None }
}
```

  and `pub mod gesture;` in `lib.rs`.

- [ ] **Step 4:** `cargo test -p cmux-term gesture` → 5 passed.
- [ ] **Step 5:** commit `M16 Task 1: Ghostty cell threshold and autoscroll edge`.

### Task 2: cell drags use the threshold; Shift-click extends

**Files:** `crates/cmux-term/src/selection.rs`, `crates/cmux-app/src/mouse.rs`, `crates/cmux-app/src/terminal_view.rs`.

**Interfaces:**
- Consumes `gesture::cell_span`.
- Produces `Selection::cells(term: &mut Term, start: SelectionPoint, end: SelectionPoint) -> Option<Selection>` (mode `Cell`, anchor = range = `[start, end]`); `Selection::mode(&self) -> SelectMode`.
- Produces on `TerminalView`: `pub(crate) press: Option<(SelectionPoint, f32)>` — anchor cell and in-cell x fraction of the last left press, kept after release (Ghostty keeps the gesture for Shift-click).
- Produces in `mouse.rs`: `fn cell_frac(g: &Geometry, pos: Point<Pixels>) -> f32` and `fn drag_to(&mut self, pos: Point<Pixels>, cx)` (used by Task 3).

- [ ] **Step 1: test** in `selection.rs`:

```rust
#[test]
fn cells_selects_the_given_range() {
    let mut t = term(b"hello world");
    let s = Selection::cells(&mut t, p(6, 0), p(10, 0)).unwrap();
    assert_eq!(s.text(&mut t).as_deref(), Some("world"));
}
```
  Delete the `is_empty_click` test block (lines ~262–274) and the method: a cell selection now exists only once the threshold is crossed. Test in `mouse.rs`:

```rust
#[test]
fn cell_frac_is_the_position_inside_the_cell() {
    let g = Geometry { origin: point(px(2.), px(0.)), cell_w: 10.0, cell_h: 20.0, cols: 80, rows: 24, scale: 1.0 };
    assert_eq!(cell_frac(&g, point(px(2.), px(5.))), 0.0);
    assert_eq!(cell_frac(&g, point(px(29.), px(5.))), 0.7);
    // Left of the grid clamps to 0, past the last column to just under 1.
    assert_eq!(cell_frac(&g, point(px(-9.), px(5.))), 0.0);
    assert!(cell_frac(&g, point(px(5000.), px(5.))) > 0.99);
}
```

- [ ] **Step 2:** run `cargo test -p cmux-term selection` and `cargo test -p cmux-app mouse` → compile errors.
- [ ] **Step 3: implement**
  - `selection.rs`:
    ```rust
    pub fn cells(term: &mut Term, start: SelectionPoint, end: SelectionPoint) -> Option<Self> {
        let r = SelectionRange { start, end };
        Some(Self { mode: SelectMode::Cell, epoch: term.epoch(), anchor: track(term, r)?, range: track(term, r)? })
    }
    pub fn mode(&self) -> SelectMode { self.mode }
    ```
  - `terminal_view.rs`: add field `pub(crate) press: Option<(SelectionPoint, f32)>` (init `None`).
  - `mouse.rs`:
    ```rust
    /// Where inside its cell the pointer is, 0..1; clamped to the grid like `cell_at`.
    fn cell_frac(g: &Geometry, pos: Point<Pixels>) -> f32 {
        let x = (f32::from(pos.x - g.origin.x) / g.cell_w).clamp(0.0, f32::from(g.cols) - 1e-3);
        x.fract()
    }
    ```
    Left press (non-reporting), replacing the current `MouseButton::Left` arm:
    ```rust
    MouseButton::Left => {
        let Some(point) = self.point_at(ev.position) else { return };
        let g = self.geometry.expect("point_at implies geometry");
        // Ghostty: Shift-click continues the last gesture as a drag (Surface.zig ~5801).
        if ev.modifiers.shift && ev.click_count == 1 && self.selection.is_some() && self.press.is_some() {
            self.dragging = true;
            self.drag_to(ev.position, cx);
            return;
        }
        let mode = select_mode(ev.click_count);
        self.press = Some((point, cell_frac(&g, ev.position)));
        self.selection = match mode {
            // A cell selection starts only once a drag crosses the threshold.
            SelectMode::Cell => None,
            _ => Selection::new(&mut self.session.term.lock().unwrap(), point, mode),
        };
        self.dragging = true;
        cx.notify();
    }
    ```
    New shared drag step (called from `mouse_move` and Task 3's ticks):
    ```rust
    pub(crate) fn drag_to(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
        let (Some(g), Some(point), Some((anchor, frac))) = (self.geometry, self.point_at(pos), self.press) else { return };
        let mut term = self.session.term.lock().unwrap();
        match self.selection.as_mut() {
            Some(s) if s.mode() != SelectMode::Cell => s.extend(&mut term, point),
            _ => {
                self.selection = gesture::cell_span(anchor, frac, point, cell_frac(&g, pos), g.cols)
                    .and_then(|(s, e)| Selection::cells(&mut term, s, e));
            }
        }
        drop(term);
        cx.notify();
    }
    ```
    `mouse_move`: replace the `if self.dragging && ... selection.extend` block with
    `if self.dragging && ev.pressed_button == Some(MouseButton::Left) { self.drag_to(ev.position, cx); }`.
    `mouse_up`: delete the `is_empty_click` block (a `None` selection already means "just a click").
- [ ] **Step 4:** `cargo test -p cmux-term && cargo test -p cmux-app mouse` → pass; `cargo clippy -p cmux-app -p cmux-term` clean.
- [ ] **Step 5:** commit `M16 Task 2: cell drags use Ghostty's threshold; Shift-click extends`.

### Task 3: autoscroll while a drag is outside the grid

**Files:** `crates/cmux-app/src/mouse.rs`, `crates/cmux-app/src/terminal_view.rs`, `crates/cmux-app/src/terminal_element.rs`.

**Interfaces:**
- Consumes `gesture::{autoscroll, Autoscroll}`, `drag_to`.
- Produces on `TerminalView`: `pub(crate) drag_pos: Point<Pixels>`, `pub(crate) autoscroll_task: Option<Task<()>>`.

- [ ] **Step 1: implement** (glue only; the rules were tested in Task 1, the whole path is scripted in Task 8):
  - `terminal_view.rs`: fields `drag_pos: Point<Pixels>` (default `point(px(0.), px(0.))`), `autoscroll_task: Option<Task<()>>` (init `None`). In `focus_changed` where `dragging = false`, also `self.autoscroll_task = None;`.
  - `mouse.rs`, at the end of `drag_to` before `cx.notify()`:
    ```rust
    self.drag_pos = pos;
    let dir = gesture::autoscroll(f32::from(pos.y - g.origin.y), f32::from(g.rows) * g.cell_h);
    if dir == Autoscroll::None {
        self.autoscroll_task = None;
    } else if self.autoscroll_task.is_none() {
        // Ghostty: one row per 15 ms tick, then the drag re-runs at the last pointer position.
        self.autoscroll_task = Some(cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_millis(15)).await;
            let go = this.update(cx, |view, cx| {
                let Some(g) = view.geometry else { return false };
                let y = f32::from(view.drag_pos.y - g.origin.y);
                let lines = match gesture::autoscroll(y, f32::from(g.rows) * g.cell_h) {
                    Autoscroll::Up => -1,
                    Autoscroll::Down => 1,
                    Autoscroll::None => return false,
                };
                if !view.dragging { return false; }
                view.session.term.lock().unwrap().scroll(lines);
                view.drag_to(view.drag_pos, cx);
                true
            });
            if go != Ok(true) {
                let _ = this.update(cx, |view, _| view.autoscroll_task = None);
                break;
            }
        }));
    }
    ```
    (`Term::scroll(isize)`: negative = toward history, as `scroll_wheel` uses it.) Careful: `drag_to` sets `autoscroll_task = None` when the pointer comes back in, which drops (cancels) the running task — correct, since the update closure has returned before the drop takes effect at the next await.
    `mouse_up` (left, local): `self.autoscroll_task = None;`.
  - `terminal_element.rs` `paint`, after `window.handle_input(...)`: moves outside the pane during a drag.
    ```rust
    // gpui sends on_mouse_move only while hovered; a selection drag that leaves the pane
    // (autoscroll, other panes) still has to reach us.
    let view = self.view.clone();
    window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
        if phase == DispatchPhase::Bubble && !bounds.contains(&ev.position)
            && ev.pressed_button == Some(MouseButton::Left)
        {
            view.update(cx, |v, cx| if v.dragging && !v.tracking() { v.drag_to(ev.position, cx) });
        }
    });
    ```
    (make `tracking` `pub(crate)`.)
- [ ] **Step 2:** `cargo build -p cmux-app && cargo clippy -p cmux-app` clean; `cargo test -p cmux-app` passes.
- [ ] **Step 3:** commit `M16 Task 3: selection autoscroll (15 ms, 1 row) outside the grid`.

### Task 4: wheel — horizontal reports and Ghostty's touchpad rate

**Files:** `crates/cmux-app/src/mouse.rs`, `crates/cmux-app/src/terminal_view.rs`.

**Interfaces — Produces:** `fn wheel_px(delta: ScrollDelta, cell_w: f32, cell_h: f32) -> (f32, f32)` — `(x, y)` pixels in Ghostty's units; field `scroll_px_x: f32`.

- [ ] **Step 1: tests** (`mouse.rs`):

```rust
#[test]
fn wheel_px_undoes_gpuis_wayland_factor() {
    // One notch: gpui sends 3 lines → Ghostty's 3 lines vertically, 1 report horizontally.
    assert_eq!(wheel_px(ScrollDelta::Lines(point(0., 3.)), 8., 16.), (0., 48.));
    assert_eq!(wheel_px(ScrollDelta::Lines(point(3., 0.)), 8., 16.), (8., 0.));
    // Touchpad: gpui triples pixels on Linux; Ghostty uses them as-is.
    #[cfg(target_os = "linux")]
    assert_eq!(wheel_px(ScrollDelta::Pixels(point(px(24.), px(30.))), 8., 16.), (8., 10.));
}
```
- [ ] **Step 2:** `cargo test -p cmux-app mouse` → fails.
- [ ] **Step 3: implement**
    ```rust
    /// gpui's Wayland backend triples touchpad pixels and makes a notch 3 lines on both axes
    /// (vendor/gpui wayland/client.rs); Ghostty uses touchpad pixels as-is and a horizontal
    /// notch as one report.
    const GPUI_AXIS_FACTOR: f32 = if cfg!(target_os = "linux") { 3.0 } else { 1.0 };

    fn wheel_px(delta: ScrollDelta, cell_w: f32, cell_h: f32) -> (f32, f32) {
        let x = f32::from(delta.pixel_delta(px(cell_w)).x) / GPUI_AXIS_FACTOR;
        let y = f32::from(delta.pixel_delta(px(cell_h)).y);
        let y = if matches!(delta, ScrollDelta::Pixels(_)) { y / GPUI_AXIS_FACTOR } else { y };
        (x, y)
    }
    ```
    In `scroll_wheel`: `let (dx, dy) = wheel_px(ev.delta, g.cell_w, g.cell_h);` y keeps today's
    `wheel_lines(self.scroll_px, dy, g.cell_h)`; add `let (cols, rest_x) = wheel_lines(self.scroll_px_x, dx, g.cell_w); self.scroll_px_x = rest_x;`.
    Early return becomes `if lines == 0 && cols == 0 { return; }`. In the `tracking` branch after the
    y loop:
    ```rust
    // Ghostty: positive x is right (button 7), as gpui's positive x is wheel-left (button 6).
    let button = if cols > 0 { VtButton::WheelLeft } else { VtButton::WheelRight };
    for _ in 0..cols.unsigned_abs() {
        self.report(MouseAction::Press, Some(button), ev.position, &ev.modifiers);
    }
    ```
    The alternate-scroll and viewport branches guard with `if lines != 0` (x never scrolls).
    Field `scroll_px_x: f32` (init `0.0`) on `TerminalView`. Delete the stale comment about SCROLL_LINES.
    Check the sign once against gpui: `ScrollDelta` positive x = content moves right = wheel
    left. If the Task 8 script shows button 66 for a right scroll, swap.
- [ ] **Step 4:** `cargo test -p cmux-app mouse` → pass.
- [ ] **Step 5:** commit `M16 Task 4: horizontal wheel reports; touchpad pixels at Ghostty's rate`.

### Task 5: Ghostty underline styles

**Files:** `crates/cmux-app/src/terminal_element.rs`.

**Interfaces — Produces:** `fn underline_marks(style: VtUnderline, x: f32, w: f32, y: f32, t: f32) -> Vec<(Bounds<Pixels>, f32)>` — rects and corner radius (dots round); `Run.underline: Option<VtUnderline>`; `Frame.underline_y: f32` (offset from cell top).

- [ ] **Step 1: tests**

```rust
fn ys(m: &[(Bounds<Pixels>, f32)]) -> Vec<f32> { m.iter().map(|(b, _)| f32::from(b.origin.y)).collect() }

#[test]
fn single_and_double_bars() {
    assert_eq!(ys(&underline_marks(VtUnderline::Single, 0., 10., 14., 1.)), vec![14.]);
    assert_eq!(ys(&underline_marks(VtUnderline::Double, 0., 10., 14., 1.)), vec![13., 15.]);
}

#[test]
fn dashes_follow_ghostty_widths() {
    // w = 10: dash 4 px, count 3 → dashes at 0 and 8 (every other one), the last clipped to w.
    let m = underline_marks(VtUnderline::Dashed, 0., 10., 14., 1.);
    let xs: Vec<_> = m.iter().map(|(b, _)| (f32::from(b.origin.x), f32::from(b.size.width))).collect();
    assert_eq!(xs, vec![(0., 4.), (8., 2.)]);
}

#[test]
fn dots_are_round_and_at_least_one() {
    let m = underline_marks(VtUnderline::Dotted, 0., 10., 14., 1.);
    assert!(!m.is_empty() && m.iter().all(|(b, r)| *r * 2. == f32::from(b.size.width)));
    assert_eq!(underline_marks(VtUnderline::Dotted, 0., 1., 14., 1.).len(), 1);
}
```
- [ ] **Step 2:** `cargo test -p cmux-app terminal_element` → fails.
- [ ] **Step 3: implement**
    ```rust
    /// Ghostty's underline sprites (font/sprite/draw/special.zig) as quads: `y` is the top of the
    /// single bar, `t` its thickness. Curly is drawn by gpui (`wavy`).
    fn underline_marks(style: VtUnderline, x: f32, w: f32, y: f32, t: f32) -> Vec<(Bounds<Pixels>, f32)> {
        let bar = |bx: f32, by: f32, bw: f32| (Bounds::new(point(px(bx), px(by)), size(px(bw), px(t))), 0.0);
        match style {
            VtUnderline::Double => vec![bar(x, y - t, w), bar(x, y + t, w)],
            VtUnderline::Dashed => {
                let dash = (w / 3.0).floor() + 1.0;
                let count = (w / dash).floor() as u32 + 1;
                (0..count).step_by(2).map(|i| i as f32 * dash).filter(|dx| *dx < w)
                    .map(|dx| bar(x + dx, y, dash.min(w - dx))).collect()
            }
            VtUnderline::Dotted => {
                let r = std::f32::consts::FRAC_1_SQRT_2 * t;
                let n = ((w / (4.0 * r)).ceil().min((w / (3.0 * r)).floor()).min((w / (2.0 * r + 1.0)).floor())).max(1.0);
                let gap = w / n;
                (0..n as u32).map(|i| {
                    let cx = x + gap * (i as f32 + 0.5);
                    (Bounds::new(point(px(cx - r), px(y + t / 2.0 - r)), size(px(2.0 * r), px(2.0 * r))), r)
                }).collect()
            }
            _ => vec![bar(x, y, w)],
        }
    }
    ```
    `Run` gains `underline: Option<VtUnderline>`; in `collect_cells` set
    `underline: cell.underline.then(|| cell.underline_style.unwrap_or(VtUnderline::Single))` and the
    `TextRun.underline` only for `Curly` (keep the `wavy: true` style). `push_run` also requires
    `last.underline == run.underline`. Other `Run` constructions (cursor glyph, preedit, tests) use `None`.
    `Frame.underline_y`: in `prepaint` from `measure`'s font metrics — return `ascent`/`descent` from
    `measure` alongside `Geometry` and set
    `underline_y = (cell_h - ascent - descent) / 2 + ascent + descent * 0.618` (gpui's own position).
    In `paint_run`, after `line.paint`, for `Some(u)` not `Curly`:
    ```rust
    let cell = self.cell_bounds(run.col, run.row, run.cells);
    for (b, r) in underline_marks(u, f32::from(cell.origin.x), f32::from(cell.size.width),
                                  f32::from(cell.origin.y) + self.underline_y, 1.0) {
        window.paint_quad(fill(b, run.style.color).corner_radii(px(r)));
    }
    ```
- [ ] **Step 4:** `cargo test -p cmux-app terminal_element` → pass.
- [ ] **Step 5:** commit `M16 Task 5: double, dotted and dashed underlines like Ghostty`.

### Task 6: one Font per style run

**Files:** `crates/cmux-app/src/terminal_element.rs`.

- [ ] **Step 1: implement** — in `collect_cells` build the four faces once:
    `let faces = [face(&style.font, false, false), face(&style.font, true, false), face(&style.font, false, true), face(&style.font, true, true)];`
    index `faces[usize::from(cell.bold) | usize::from(cell.italic) << 1]`. Before building a
    `TextRun`, try to extend the last run without cloning:
    ```rust
    let face_i = usize::from(cell.bold) | usize::from(cell.italic) << 1;
    if batchable && let Some(last) = out.runs.last_mut()
        && last.batchable && last.row == row && last.col + last.cells == col
        && last.face == face_i && last.style.color == fg && last.underline == underline
        && last.style.strikethrough.is_some() == cell.strikethrough
    {
        last.text.push_str(&cell.text); last.style.len += cell.text.len(); last.cells += 1;
        continue;
    }
    ```
    `Run` gains `face: usize` (cursor glyph / preedit / tests: `0`). `push_run` and `same_style` stay
    for the non-fast path. `UnderCursor.font` takes `faces[face_i].clone()` (once per frame).
- [ ] **Step 2:** `cargo test -p cmux-app terminal_element` (existing batching tests) pass; clippy clean.
- [ ] **Step 3:** commit `M16 Task 6: no Font clone per cell in prepaint`.

### Task 7: IME preedit placement and caret rect

**Files:** `crates/cmux-app/src/terminal_element.rs`, `crates/cmux-app/src/ime.rs`, `crates/cmux-app/src/terminal_view.rs`, `crates/cmux-app/Cargo.toml`, `NOTICE`.

**Interfaces — Produces:** `pub(crate) fn preedit_range(text: &str, cursor_col: u16, cols: u16) -> (u16, usize, u16)` in `ime.rs` — `(start column, byte offset of the first drawn char, width in cells)`; `TerminalView.preedit_col: Option<(u16, u16)>` (start column, row) written by prepaint.

- [ ] **Step 1: tests** (`ime.rs`):

```rust
#[test]
fn preedit_sits_at_the_cursor_when_it_fits() {
    assert_eq!(preedit_range("ab", 2, 10), (2, 0, 2));
}

#[test]
fn preedit_shifts_left_at_the_right_edge() {
    // "かん" is 4 cells; from column 8 of 10 it would end at 11, so it starts at 6.
    assert_eq!(preedit_range("かん", 8, 10), (6, 0, 4));
}

#[test]
fn preedit_wider_than_the_row_drops_leading_chars() {
    // 6 cells into a 4-column row: keep the last 4 cells ("んじ"), from byte 3.
    assert_eq!(preedit_range("かんじ", 1, 4), (0, 3, 4));
}
```
- [ ] **Step 2:** `cargo test -p cmux-app ime` → fails.
- [ ] **Step 3: implement** — add `unicode-width = "0.2"` (match the version in `Cargo.lock`) to
  `crates/cmux-app/Cargo.toml` and a `NOTICE` entry (MIT/Apache-2.0).
    ```rust
    /// Ghostty's `Preedit.range` (renderer/State.zig): at the cursor, shifted left to end at the
    /// last column; if wider than the row, leading characters are dropped.
    pub(crate) fn preedit_range(text: &str, cursor_col: u16, cols: u16) -> (u16, usize, u16) {
        use unicode_width::UnicodeWidthChar;
        let mut w = 0u16;
        let mut offset = 0;
        for (i, ch) in text.char_indices().rev() {
            let cw = ch.width().unwrap_or(0).max(1) as u16;
            if w + cw > cols { offset = i + ch.len_utf8(); break; }
            w += cw;
        }
        let start = cursor_col.min(cols.saturating_sub(w));
        (start, offset, w)
    }
    ```
  In `plan_cursor`, compute `let (start, off, w) = ime::preedit_range(&marked_text, c.x, cols)` (pass
  `cols` from geometry) and change `Marked` to `(start, row, text[off..], color, w)`. `paint_marked`:
  first `window.paint_quad(fill(self.cell_bounds(start, row, w), self.default_bg))` (add
  `default_bg: Hsla` to `Frame`, from `info.default_bg` with the window's background opacity like
  `collect_cells`), then the run with `cells: w`. Prepaint writes `view.preedit_col = Some((start, row))`
  (or `None` without preedit).
  `bounds_for_range` (ime.rs): use `self.preedit_col.or(self.cursor_cell)` as the origin and the
  width of `marked_text[..byte(range.start)]` in cells (`UnicodeWidthStr::width`) as the x offset
  instead of `range.start` cells. Range indices are UTF-16; convert with
  `marked_text.encode_utf16()` prefix → `String::from_utf16_lossy`.
- [ ] **Step 4:** `cargo test -p cmux-app ime` → pass; `cargo deny check licenses` passes.
- [ ] **Step 5:** commit `M16 Task 7: IME preedit placed like Ghostty; candidate rect at the preedit`.

### Task 8: blink task only when blinking

**Files:** `crates/cmux-app/src/terminal_view.rs`, `crates/cmux-app/src/terminal_element.rs`.

**Interfaces — Produces:** `pub(crate) fn blink_wanted(focused: bool, cursor_blinks: bool) -> bool`; `TerminalView.blink_task: Option<Task<()>>`; `fn sync_blink(&mut self, focused: bool, cx: &mut Context<Self>)`.

- [ ] **Step 1: test**

```rust
#[test]
fn blink_timer_runs_only_for_a_focused_blinking_cursor() {
    assert!(blink_wanted(true, true));
    assert!(!blink_wanted(false, true));
    assert!(!blink_wanted(true, false));
}
```
- [ ] **Step 2:** `cargo test -p cmux-app blink` → fails.
- [ ] **Step 3: implement** — remove `blink_task` from `_tasks`; field `blink_task: Option<Task<()>>`.
    ```rust
    /// Ghostty blinks only a focused cursor; no timer otherwise, so an idle app sleeps.
    pub(crate) fn blink_wanted(focused: bool, cursor_blinks: bool) -> bool { focused && cursor_blinks }

    fn sync_blink(&mut self, focused: bool, cx: &mut Context<Self>) {
        if !blink_wanted(focused, self.cursor_blinks) {
            self.blink_task = None;
            self.blink_on = true;
        } else if self.blink_task.is_none() {
            self.blink_task = Some(cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(Duration::from_millis(600)).await;
                if this.update(cx, |v, cx| { v.blink_on = !v.blink_on; cx.notify(); }).is_err() { break; }
            }));
        }
    }
    ```
    Call `self.sync_blink(focused, cx)` at the end of `focus_changed`; in `prepaint`'s
    `self.view.update(cx, |view, cx| …)` after setting `cursor_blinks`, call
    `view.sync_blink(focused, cx)` (change the closure to take `cx`).
- [ ] **Step 4:** `cargo test -p cmux-app` passes.
- [ ] **Step 5:** commit `M16 Task 8: blink timer only for a focused blinking cursor`.

### Task 9: scripted exits, debug read, docs

**Files:** `crates/cmux-app/src/window_view/control.rs`, `scripts/click-through.py`, `docs/milestones/m16-input-polish/journal.md`, `STATUS.md`, `docs/status-archive.md`.

**Interfaces — Produces:** socket `debug.terminal.state {surface_id?}` (debug builds) →
`{"selection": str|null, "scroll_offset": int, "lines": [str], "blink_timer": bool}`;
`debug.mouse.simulate` accepts `shift: bool` and, for `scroll`, `dx` (lines, + = right).

- [ ] **Step 1: implement `debug.terminal.state`** next to `debug.ui.bounds` (`#[cfg(debug_assertions)]`):
    resolve with `snap.surface(params)?` and `view_of(..).terminal(..)` exactly like
    `send_from_socket`; read `selection.text(&mut term)`, `term.vt().scrollbar().map_or(0, |s| s.offset)`,
    `term.text_lines()`, and `view.blink_task.is_some()`. Add a `mouse_events` unit test for
    `shift` and `dx` (`modifiers.shift == true`; `ScrollDelta::Lines(point(-dx, -dy))`, matching
    the existing `dy` sign convention).
- [ ] **Step 2: click-through checks** (append to `CHECKS`):
  - `drag_threshold`: in a fresh workspace `rpc("surface.send_text", text="printf 'hello world'\n")`;
    find the terminal element (`find("terminal")` — check the id with `elements("term")` first),
    take the row of `hello` from `lines`; `down` at cell 0 x-frac 0.1, `drag` to cell 0 x-frac 0.5
    → `selection` is null; `drag` to cell 4 x-frac 0.9, `up` → `selection == "hello"`. Then a
    double-click (`click_count=2`) on `hello`, then Shift-`click` on `world` (wait 0.5 s first) →
    `selection == "hello world"`.
  - `autoscroll`: `seq 1 300` in the pane; scroll up 20 lines with `scroll dy=-20`; `down` mid-pane,
    `drag` to y = window bottom − 2 (below the pane), wait 0.6 s → `scroll_offset` grew by ≥ 10;
    `up`, read offset, wait 0.5 s → unchanged.
  - `horizontal_wheel`: `printf '\e[?1000h\e[?1006h'; cat -v` in the pane; `scroll dx=1` and `dx=-1`
    over it → `lines` contain `[[<67;` and `[[<66;`; then `ctrl+c` and `printf '\e[?1000l\e[?1006l'`.
  - `idle_blink`: with Ghostty config `cursor-style-blink = false` written to `CFG/ghostty/config`
    before launch, `blink_timer` is false for the focused terminal (exit 5). Also check that it is true
    after writing `cursor-style-blink = true` and a fresh terminal opens (config reload) — if
    reload needs a restart, keep only the false case and say so in the journal.
- [ ] **Step 3:** `cargo build -p cmux-app -p cmux-ipc && scripts/click-through.py` → all PASS
  (13 old + 4 new). Fix what fails.
- [ ] **Step 4: docs** — `journal.md`: dated entry with the "Findings" above (threshold is per
  cell, shift-extend rule, resize needs no change, no text blink), results per exit, and anything
  changed while scripting. Archive `STATUS.md` to `docs/status-archive.md` under
  `## 2026-11-14 — before M16`, then update: current milestone M16, the M16 row, and remove the
  "Mouse:"/"Render:"/"IME:" carry items that are now done (keep: touchpad feel and real IME as
  owner checks, ESC[3J epoch, title/bell coalescing). ≤ 100 lines.
- [ ] **Step 5:** `cargo fmt --all && cargo clippy --workspace && cargo test --workspace`; commit
  `M16 Task 9: scripted input checks, journal, STATUS`; push and open the PR.
