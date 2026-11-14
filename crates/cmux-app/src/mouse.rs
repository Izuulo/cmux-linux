//! Mouse input for the terminal view: local selection and wheel scrolling, or reporting to
//! the application when it enabled mouse tracking (Shift bypasses reporting for clicks and
//! drags, as in Ghostty). Routing follows Zed's `register_mouse_listeners` and
//! `determine_scroll_lines` (zed-industries/zed@69e2130, GPL-3.0-or-later) and Ghostty's
//! `Surface.zig` `scrollCallback`.

use cmux_term::ghostty_vt::{
    KeyAction, MouseAction, MouseButton as VtButton, MouseInput, Screen, SelectionPoint,
};
use std::time::Duration;

use cmux_term::gesture::{self, Autoscroll};
use cmux_term::selection::{SelectMode, Selection};
use gpui::{
    Context, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Point, ScrollWheelEvent, Window, px,
};

use crate::keys;
use crate::terminal_view::{Geometry, TerminalEvent, TerminalView};

fn vt_button(b: MouseButton) -> Option<VtButton> {
    match b {
        MouseButton::Left => Some(VtButton::Left),
        MouseButton::Right => Some(VtButton::Right),
        MouseButton::Middle => Some(VtButton::Middle),
        MouseButton::Navigate(_) => None,
    }
}

/// Ghostty caps the click counter at 3 (SelectionGesture.zig ~664: `@min(count + 1, 3)`),
/// so a fourth rapid click stays on Line. gpui's count grows without bound.
fn select_mode(click_count: usize) -> SelectMode {
    match click_count {
        0 | 1 => SelectMode::Cell,
        2 => SelectMode::Word,
        _ => SelectMode::Line,
    }
}

/// Where inside its cell the pointer is, `0..1`; clamped to the grid like `Geometry::cell_at`.
fn cell_frac(g: &Geometry, pos: Point<Pixels>) -> f32 {
    let x = (f32::from(pos.x - g.origin.x) / g.cell_w).clamp(0.0, f32::from(g.cols) - 1e-3);
    x.fract()
}

/// Absolute screen row of a viewport row, given the scrollbar offset of the viewport top.
fn absolute_row(offset: u64, viewport_row: u16) -> u32 {
    (offset + u64::from(viewport_row)) as u32
}

/// Adds `delta_px` (positive = wheel up) to the pending pixels and splits off whole lines.
/// Returns the lines to scroll and the pixels left over.
fn wheel_lines(pending_px: f32, delta_px: f32, cell_h: f32) -> (i32, f32) {
    let total = pending_px + delta_px;
    let lines = (total / cell_h) as i32;
    (lines, total - lines as f32 * cell_h)
}

impl TerminalView {
    pub(crate) fn tracking(&self) -> bool {
        self.session.term.lock().unwrap().vt().mouse_tracking()
    }

    /// A click or drag goes to the app unless Shift is held or a local selection drag is
    /// already running (Shift may be released mid-drag).
    fn reporting(&self, mods: &Modifiers) -> bool {
        !mods.shift && !self.dragging && self.tracking()
    }

    /// Absolute selection point (scrollback offset + viewport row) under a window position.
    fn point_at(&self, pos: Point<Pixels>) -> Option<SelectionPoint> {
        let (column, row) = self.geometry?.cell_at(pos);
        let offset = self
            .session
            .term
            .lock()
            .unwrap()
            .vt()
            .scrollbar()
            .map_or(0, |s| s.offset);
        Some(SelectionPoint {
            column,
            row: absolute_row(offset, row),
        })
    }

    fn link_at(&self, pos: Point<Pixels>) -> Option<String> {
        let (column, row) = self.geometry?.cell_at(pos);
        self.session.term.lock().unwrap().link_at(column, row)
    }

    fn report(
        &mut self,
        action: MouseAction,
        button: Option<VtButton>,
        pos: Point<Pixels>,
        mods: &Modifiers,
    ) {
        let Some(g) = self.geometry else { return };
        let (cw, ch) = (g.device(g.cell_w), g.device(g.cell_h));
        let input = MouseInput {
            action,
            button,
            mods: keys::mods(mods),
            // Device pixels, like the cell size the terminal was resized with.
            position: (
                f32::from(pos.x - g.origin.x) * g.scale,
                f32::from(pos.y - g.origin.y) * g.scale,
            ),
            screen_size: (cw * u32::from(g.cols), ch * u32::from(g.rows)),
            cell_size: (cw, ch),
            any_button_pressed: action != MouseAction::Release && button.is_some(),
        };
        let mut out = Vec::new();
        if action == MouseAction::Release {
            // The release encoder keeps the protocol that was active at press time.
            let _ = self.mouse.encode_release(input, &mut out);
        } else {
            self.mouse
                .sync_from_terminal(self.session.term.lock().unwrap().vt());
            let _ = self.mouse.encode(input, &mut out);
        }
        if !out.is_empty() {
            self.session.write(out);
        }
    }

    pub(crate) fn mouse_down(
        &mut self,
        ev: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A divider or tab drag owns the pointer (research-gpui.md §1.2 route B).
        if cx.has_active_drag() {
            return;
        }
        // A link press whose release went to a browser tab that replaced us must not eat this one.
        self.link_press = false;
        let was_focused = self.focus.is_focused(window);
        window.focus(&self.focus);
        // Ghostty's link chord: Ctrl alone opens the link under the pointer, even when the app
        // tracks the mouse (`mouseLinkRefreshAllowedState`).
        let m = &ev.modifiers;
        if ev.button == MouseButton::Left
            && m.control
            && !(m.shift || m.alt || m.platform || m.function)
            && let Some(url) = self.link_at(ev.position)
        {
            self.link_press = true;
            cx.emit(TerminalEvent::OpenLink(url));
            return;
        }
        // The click that focuses another pane is not terminal input, unless the app tracks the
        // mouse (cmux TerminalPointerFocusActivationPolicy).
        if !was_focused && !self.tracking() {
            return;
        }
        if self.reporting(&ev.modifiers) {
            if let Some(b) = vt_button(ev.button) {
                self.report(MouseAction::Press, Some(b), ev.position, &ev.modifiers);
            }
            // Ghostty drops the selection when a click goes to the app (Surface.zig ~5940).
            if self.selection.take().is_some() {
                cx.notify();
            }
            return;
        }
        match ev.button {
            MouseButton::Left => {
                let (Some(g), Some(point)) = (self.geometry, self.point_at(ev.position)) else {
                    return;
                };
                // Ghostty: Shift-click continues the last gesture as a drag to the press, unless
                // it is a quick repeat (Surface.zig ~5801).
                if ev.modifiers.shift
                    && ev.click_count == 1
                    && self.selection.is_some()
                    && self.press.is_some()
                {
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
            // Middle click pastes the primary selection (X11/Wayland convention).
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            MouseButton::Middle => {
                if let Some(text) = cx.read_from_primary().and_then(|item| item.text()) {
                    self.paste_text(text, window, cx);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn mouse_move(
        &mut self,
        ev: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if cx.has_active_drag() {
            return;
        }
        if self.reporting(&ev.modifiers) {
            let button = ev.pressed_button.and_then(vt_button);
            self.report(MouseAction::Motion, button, ev.position, &ev.modifiers);
            return;
        }
        if self.dragging && ev.pressed_button == Some(MouseButton::Left) {
            self.drag_to(ev.position, cx);
        }
    }

    /// One drag step to `pos`: word and line selections grow by units from their anchor, cell
    /// selections follow Ghostty's within-cell threshold from the press.
    pub(crate) fn drag_to(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
        let (Some(g), Some(point), Some((anchor, frac))) =
            (self.geometry, self.point_at(pos), self.press)
        else {
            return;
        };
        let mut term = self.session.term.lock().unwrap();
        match self.selection.as_mut() {
            Some(s) if s.mode() != SelectMode::Cell => s.extend(&mut term, point),
            _ => {
                self.selection =
                    gesture::cell_span(anchor, frac, point, cell_frac(&g, pos), g.cols)
                        .and_then(|(s, e)| Selection::cells(&mut term, s, e));
            }
        }
        drop(term);
        self.drag_pos = pos;
        let dir = gesture::autoscroll(f32::from(pos.y - g.origin.y), f32::from(g.rows) * g.cell_h);
        if dir == Autoscroll::None {
            self.autoscroll_task = None;
        } else if self.autoscroll_task.is_none() {
            self.autoscroll_task = Some(cx.spawn(async move |this, cx| {
                // Ghostty: one row per 15 ms tick (termio/Thread.zig), then the drag re-runs at
                // the last pointer position.
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(15))
                        .await;
                    let go = this.update(cx, |view, cx| view.autoscroll_tick(cx));
                    if !matches!(go, Ok(true)) {
                        let _ = this.update(cx, |view, _| view.autoscroll_task = None);
                        break;
                    }
                }
            }));
        }
        cx.notify();
    }

    /// One autoscroll step; false once the drag ended or came back inside the grid.
    fn autoscroll_tick(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(g) = self.geometry else { return false };
        let y = f32::from(self.drag_pos.y - g.origin.y);
        let lines = match gesture::autoscroll(y, f32::from(g.rows) * g.cell_h) {
            Autoscroll::Up => -1,
            Autoscroll::Down => 1,
            Autoscroll::None => return false,
        };
        if !self.dragging {
            return false;
        }
        self.session.term.lock().unwrap().scroll(lines);
        self.drag_to(self.drag_pos, cx);
        true
    }

    pub(crate) fn mouse_up(&mut self, ev: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if cx.has_active_drag() {
            return;
        }
        if ev.button == MouseButton::Left && std::mem::take(&mut self.link_press) {
            return;
        }
        if self.reporting(&ev.modifiers) {
            if let Some(b) = vt_button(ev.button) {
                self.report(MouseAction::Release, Some(b), ev.position, &ev.modifiers);
            }
            return;
        }
        if ev.button != MouseButton::Left || !self.dragging {
            return;
        }
        self.dragging = false;
        self.autoscroll_task = None;
        let mut term = self.session.term.lock().unwrap();
        let text = self.selection.as_ref().and_then(|s| s.text(&mut term));
        drop(term);
        // Ghostty's copy-on-select default on Linux: the selection goes to PRIMARY. Never with
        // the Term lock held, and never an empty string over the previous PRIMARY.
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if let Some(text) = text.filter(|t| !t.is_empty()) {
            cx.write_to_primary(gpui::ClipboardItem::new_string(text));
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        let _ = text;
        cx.notify();
    }

    pub(crate) fn scroll_wheel(
        &mut self,
        ev: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(g) = self.geometry else { return };
        // gpui's Wayland backend already turns one wheel notch into 3 lines (SCROLL_LINES in
        // platform/linux/platform.rs), Ghostty's discrete default, so no multiplier here.
        let (lines, rest) = wheel_lines(
            self.scroll_px,
            f32::from(ev.delta.pixel_delta(px(g.cell_h)).y),
            g.cell_h,
        );
        self.scroll_px = rest;
        if lines == 0 {
            return;
        }
        // gpui: positive y is wheel-up, toward history.
        let (tracking, alternate_scroll) = {
            let term = self.session.term.lock().unwrap();
            let vt = term.vt();
            (
                vt.mouse_tracking(),
                vt.active_screen() == Screen::Alternate && vt.mode(1007, false),
            )
        };
        // Unlike clicks, Ghostty reports the wheel even with Shift held.
        if tracking {
            self.selection = None;
            let button = if lines > 0 {
                VtButton::WheelUp
            } else {
                VtButton::WheelDown
            };
            for _ in 0..lines.unsigned_abs() {
                self.report(MouseAction::Press, Some(button), ev.position, &ev.modifiers);
            }
        } else if alternate_scroll {
            // Mode 1007 is not handled by libghostty-vt (Ghostty does it in Surface.zig
            // scrollCallback), so full-screen apps such as less get arrow keys here.
            self.selection = None;
            let key = if lines > 0 { "up" } else { "down" };
            let arrow = Keystroke {
                modifiers: Modifiers::default(),
                key: key.into(),
                key_char: None,
            };
            // The key encoder picks ESC O A or ESC [ A from the cursor-key mode (DECCKM).
            let one = self.encode_key(&arrow, KeyAction::Press);
            self.session
                .write(one.repeat(lines.unsigned_abs() as usize));
        } else {
            self.session.term.lock().unwrap().scroll(-(lines as isize));
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_cap_at_line_like_ghostty() {
        let modes: Vec<_> = (0..=5).map(select_mode).collect();
        use SelectMode::*;
        assert_eq!(modes, [Cell, Cell, Word, Line, Line, Line]);
    }

    #[test]
    fn cell_frac_is_the_position_inside_the_cell() {
        let g = Geometry {
            origin: gpui::point(px(2.), px(0.)),
            cell_w: 10.0,
            cell_h: 20.0,
            cols: 80,
            rows: 24,
            scale: 1.0,
        };
        assert_eq!(cell_frac(&g, gpui::point(px(2.), px(5.))), 0.0);
        assert!((cell_frac(&g, gpui::point(px(29.), px(5.))) - 0.7).abs() < 1e-5);
        assert_eq!(cell_frac(&g, gpui::point(px(-9.), px(5.))), 0.0);
        assert!(cell_frac(&g, gpui::point(px(5000.), px(5.))) > 0.99);
    }

    #[test]
    fn viewport_rows_become_absolute_through_the_scroll_offset() {
        assert_eq!(absolute_row(0, 5), 5);
        assert_eq!(absolute_row(120, 5), 125);
    }

    #[test]
    fn wheel_accumulates_partial_lines() {
        // Three small touchpad moves of 6 px with 16 px cells make one line.
        let (mut pending, mut total) = (0.0, 0);
        for _ in 0..3 {
            let (l, rest) = wheel_lines(pending, 6.0, 16.0);
            (pending, total) = (rest, total + l);
        }
        assert_eq!((total, pending), (1, 2.0));
        // One notch (3 lines) and the opposite direction, truncating toward zero.
        assert_eq!(wheel_lines(0.0, 48.0, 16.0), (3, 0.0));
        assert_eq!(wheel_lines(0.0, -48.0, 16.0), (-3, 0.0));
        assert_eq!(wheel_lines(0.0, -10.0, 16.0), (0, -10.0));
    }
}
