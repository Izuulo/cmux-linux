//! Mouse input for the terminal view: local selection and wheel scrolling, or reporting to
//! the application when it enabled mouse tracking (Shift bypasses reporting for clicks and
//! drags, as in Ghostty). Routing follows Zed's `register_mouse_listeners` and
//! `determine_scroll_lines` (zed-industries/zed@69e2130, GPL-3.0-or-later) and Ghostty's
//! `Surface.zig` `scrollCallback`.

use cmux_term::ghostty_vt::{
    KeyAction, MouseAction, MouseButton as VtButton, MouseInput, Screen, SelectionPoint,
};
use cmux_term::selection::{SelectMode, Selection};
use gpui::{
    Context, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Point, ScrollWheelEvent, Window, px,
};

use crate::keys;
use crate::terminal_view::TerminalView;

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
    fn tracking(&self) -> bool {
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
        let was_focused = self.focus.is_focused(window);
        window.focus(&self.focus);
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
                let Some(point) = self.point_at(ev.position) else {
                    return;
                };
                let mode = select_mode(ev.click_count);
                let mut term = self.session.term.lock().unwrap();
                self.selection = Selection::new(&mut term, point, mode);
                drop(term);
                self.dragging = self.selection.is_some();
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
        if self.dragging
            && ev.pressed_button == Some(MouseButton::Left)
            && let Some(point) = self.point_at(ev.position)
            && let Some(selection) = self.selection.as_mut()
        {
            selection.extend(&mut self.session.term.lock().unwrap(), point);
            cx.notify();
        }
    }

    pub(crate) fn mouse_up(&mut self, ev: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if cx.has_active_drag() {
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
        let mut term = self.session.term.lock().unwrap();
        if self
            .selection
            .as_ref()
            .is_some_and(|s| s.is_empty_click(&term))
        {
            self.selection = None;
        }
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
