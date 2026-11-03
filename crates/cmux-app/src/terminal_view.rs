//! The terminal view: owns the PTY session, turns gpui input into PTY bytes and hosts the
//! element that paints the grid. The PTY-to-repaint loop and input routing follow Zed's
//! terminal (zed-industries/zed@69e2130, crates/terminal/src/terminal.rs and
//! crates/terminal_view/src/terminal_view.rs, GPL-3.0-or-later, Copyright Zed Industries).

use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

use cmux_term::TermEvent;
use cmux_term::ghostty_vt::{KeyAction, KeyEncoder, MouseEncoders};
use cmux_term::input::{encode_focus, encode_paste};
use cmux_term::pty::{PtyEvent, Session};
use cmux_term::selection::Selection;
use futures::StreamExt;
use futures::channel::mpsc;
use gpui::{
    App, ClipboardItem, Context, FocusHandle, Focusable, IntoElement, KeyDownEvent, KeyUpEvent,
    Keystroke, MouseButton, ParentElement, Pixels, Point, Render, Styled, Subscription, Task,
    Window, div, prelude::*, px,
};

use crate::keys::{self, AppAction};
use crate::style::TermStyle;
use crate::terminal_element::TerminalElement;

/// Ghostty's default window-padding-x/y is 2 points.
const PADDING_PX: f32 = 2.0 * 96.0 / 72.0;

/// Ghostty floors window padding to whole device pixels (Surface.zig `@floor(padding * dpi / 72)`).
pub fn snap_padding(logical: f32, scale: f32) -> f32 {
    (logical * scale).floor() / scale
}

/// A logical coordinate moved to the nearest whole device pixel.
pub fn snap_to_device(logical: f32, scale: f32) -> f32 {
    (logical * scale).round() / scale
}

/// Grid geometry the element measured in its last prepaint, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub origin: Point<Pixels>,
    pub cell_w: f32,
    pub cell_h: f32,
    pub cols: u16,
    pub rows: u16,
    pub scale: f32,
}

impl Geometry {
    /// The viewport cell under a window position, clamped to the grid.
    pub fn cell_at(&self, pos: Point<Pixels>) -> (u16, u16) {
        let x = f32::from(pos.x - self.origin.x) / self.cell_w;
        let y = f32::from(pos.y - self.origin.y) / self.cell_h;
        let clamp = |v: f32, n: u16| (v.max(0.0) as u16).min(n.saturating_sub(1));
        (clamp(x, self.cols), clamp(y, self.rows))
    }

    /// Whether the PTY needs a new size: the grid, the cell size or the scale (so the pixel
    /// size follows a move to another-DPI monitor) differs. The origin does not matter.
    pub fn needs_resize(&self, old: &Geometry) -> bool {
        (old.cols, old.rows, old.cell_w, old.cell_h, old.scale)
            != (self.cols, self.rows, self.cell_w, self.cell_h, self.scale)
    }

    /// Logical to device pixels. Cells are already snapped, so cell sizes come out whole.
    pub fn device(&self, logical: f32) -> u32 {
        (logical * self.scale).round() as u32
    }
}

pub struct TerminalView {
    pub(crate) session: Session,
    pub(crate) style: Rc<TermStyle>,
    pub(crate) focus: FocusHandle,
    pub(crate) geometry: Option<Geometry>,
    pub(crate) selection: Option<Selection>,
    /// IME preedit text, painted at the cursor.
    pub(crate) marked_text: String,
    /// Viewport cursor cell from the last prepaint, for IME placement.
    pub(crate) cursor_cell: Option<(u16, u16)>,
    pub(crate) cursor_blinks: bool,
    pub(crate) blink_on: bool,
    key_encoder: KeyEncoder,
    pub(crate) mouse: MouseEncoders,
    /// A local selection drag is in progress.
    pub(crate) dragging: bool,
    /// Wheel pixels not yet turned into whole lines.
    pub(crate) scroll_px: f32,
    /// `Keystroke.key` of presses consumed by an app binding, whose release must not reach the PTY.
    bound_keys: HashSet<String>,
    title: Option<String>,
    title_dirty: bool,
    /// `CMUX_TRACE_LATENCY=1`: key press time, waiting for its echo.
    trace_latency: bool,
    key_at: Option<Instant>,
    pub(crate) echo_at: Option<Instant>,
    _tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl TerminalView {
    pub fn new(style: Rc<TermStyle>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (tx, mut rx) = mpsc::unbounded::<PtyEvent>();
        let session = Session::spawn(
            Session::shell_command(),
            80,
            24,
            |term| style.apply_to(term),
            move |ev| {
                let _ = tx.unbounded_send(ev);
            },
        )
        .expect("cmux-linux: could not start the shell on a PTY");

        let pty_task = cx.spawn(async move |this, cx| {
            while let Some(first) = rx.next().await {
                let mut batch = vec![first];
                // First chunk repaints at once (typing echo); a burst then coalesces into one
                // repaint per ~4 ms, as Zed's terminal subscribe loop does.
                while !batch.is_empty() {
                    let events = std::mem::take(&mut batch);
                    if this
                        .update(cx, |view, cx| view.on_pty_events(events, cx))
                        .is_err()
                    {
                        return;
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(4))
                        .await;
                    while let Ok(ev) = rx.try_recv() {
                        batch.push(ev);
                    }
                }
            }
        });
        let blink_task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(600))
                    .await;
                let alive = this.update(cx, |view, cx| {
                    view.blink_on = !view.blink_on;
                    if view.cursor_blinks {
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        });

        let focus = cx.focus_handle();
        let subscriptions = vec![
            cx.on_focus_in(&focus, window, |view, _, cx| view.focus_changed(true, cx)),
            cx.on_blur(&focus, window, |view, _, cx| view.focus_changed(false, cx)),
        ];

        Self {
            session,
            style,
            focus,
            geometry: None,
            selection: None,
            marked_text: String::new(),
            cursor_cell: None,
            cursor_blinks: false,
            blink_on: true,
            key_encoder: KeyEncoder::new().expect("libghostty-vt key encoder"),
            mouse: MouseEncoders::new().expect("libghostty-vt mouse encoders"),
            dragging: false,
            scroll_px: 0.0,
            bound_keys: HashSet::new(),
            title: None,
            title_dirty: false,
            trace_latency: std::env::var_os("CMUX_TRACE_LATENCY").is_some(),
            key_at: None,
            echo_at: None,
            _tasks: vec![pty_task, blink_task],
            _subscriptions: subscriptions,
        }
    }

    fn on_pty_events(&mut self, events: Vec<PtyEvent>, cx: &mut Context<Self>) {
        for event in events {
            match event {
                // Ghostty closes the surface when its shell exits; M1 has one surface.
                PtyEvent::Exited => return cx.quit(),
                PtyEvent::Output(events) => {
                    if let Some(at) = self.key_at.take() {
                        self.echo_at = Some(at);
                    }
                    for e in events {
                        match e {
                            TermEvent::Title(t) => {
                                self.title = Some(t);
                                self.title_dirty = true;
                            }
                            // Bell, cwd and notifications feed the sidebar and attention UI (M3/M4).
                            TermEvent::Bell | TermEvent::Cwd(_) | TermEvent::Notify { .. } => {}
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    /// Called by the element after painting: logs key-to-echo-to-paint latency when tracing.
    pub(crate) fn painted(&mut self) {
        if let Some(at) = self.echo_at.take()
            && self.trace_latency
        {
            eprintln!(
                "latency key->paint {:.1} ms",
                at.elapsed().as_secs_f64() * 1000.0
            );
        }
    }

    /// Writes user input and snaps the viewport back to the live screen, like Ghostty.
    pub(crate) fn send_input(&mut self, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }
        self.session.term.lock().unwrap().scroll_to_bottom();
        self.session.write(bytes);
        self.blink_on = true;
        if self.trace_latency {
            self.key_at = Some(Instant::now());
        }
    }

    fn focus_changed(&mut self, focused: bool, cx: &mut Context<Self>) {
        // A binding key released while unfocused never sends key_up here; forget it.
        self.bound_keys.clear();
        // A release lost while unfocused would otherwise keep clicks local on a tracking TUI.
        self.dragging = false;
        // The IME is disabled on leave and never ends the preedit itself.
        if !focused {
            self.marked_text.clear();
        }
        if self.session.term.lock().unwrap().vt().mode(1004, false) {
            self.session.write(encode_focus(focused));
        }
        cx.notify();
    }

    pub(crate) fn set_geometry(&mut self, g: Geometry) {
        let changed = self.geometry.is_none_or(|old| g.needs_resize(&old));
        self.geometry = Some(g);
        if changed {
            let (w, h) = (g.device(g.cell_w), g.device(g.cell_h));
            if let Err(e) = self.session.resize(g.cols, g.rows, w, h) {
                eprintln!("cmux-linux: resize failed: {e}");
            }
        }
    }

    pub(crate) fn encode_key(&mut self, ks: &Keystroke, action: KeyAction) -> Vec<u8> {
        let input = keys::key_input(ks, action);
        let term = self.session.term.lock().unwrap();
        self.key_encoder.sync_from_terminal(term.vt());
        let mut out = Vec::new();
        let _ = self.key_encoder.encode(&input, &mut out);
        out
    }

    fn key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(action) = keys::binding(&ev.keystroke) {
            self.bound_keys.insert(ev.keystroke.key.clone());
            match action {
                AppAction::Copy => self.copy(cx),
                AppAction::Paste => self.paste(cx),
            }
            return cx.stop_propagation();
        }
        // Super combos are desktop and app shortcuts, never terminal input. gpui's Linux text
        // fallback already skips them (wayland/window.rs:725-727, x11/window.rs:1004 only insert
        // `key_char` for shift-or-no modifiers), but replay_pending_input (window.rs:3969-3978)
        // inserts `key_char` whenever the key propagates, so stop it here too.
        if ev.keystroke.modifiers.platform {
            return cx.stop_propagation();
        }
        let action = if ev.is_held {
            KeyAction::Repeat
        } else {
            KeyAction::Press
        };
        let bytes = self.encode_key(&ev.keystroke, action);
        if !bytes.is_empty() {
            // Stopping here also keeps gpui from re-sending the key as IME text. A composed key
            // ends the dead-key preedit; gpui sends no unmark for it (wayland/client.rs:1320).
            self.selection = None;
            self.marked_text.clear();
            self.send_input(bytes);
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn key_up(&mut self, ev: &KeyUpEvent, _: &mut Window, _: &mut Context<Self>) {
        if self.bound_keys.remove(&ev.keystroke.key) || ev.keystroke.modifiers.platform {
            return;
        }
        // Empty unless the app asked for kitty's report-event-types flag.
        let bytes = self.encode_key(&ev.keystroke, KeyAction::Release);
        if !bytes.is_empty() {
            self.session.write(bytes);
        }
    }

    pub(crate) fn copy(&mut self, cx: &mut Context<Self>) {
        let Some(selection) = &self.selection else {
            return;
        };
        if let Some(text) = selection.text(&mut self.session.term.lock().unwrap()) {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.paste_text(&text);
        }
    }

    pub(crate) fn paste_text(&mut self, text: &str) {
        let bracketed = self.session.term.lock().unwrap().vt().mode(2004, false);
        self.send_input(encode_paste(text, bracketed));
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.title_dirty {
            self.title_dirty = false;
            if let Some(title) = &self.title {
                window.set_window_title(title);
            }
        }
        div()
            .size_full()
            .p(px(snap_padding(PADDING_PX, window.scale_factor())))
            .bg(self.style.window_background())
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_scroll_wheel(cx.listener(Self::scroll_wheel))
            .child(TerminalElement::new(cx.entity()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::point;

    #[test]
    fn cell_at_maps_and_clamps_positions() {
        let g = Geometry {
            origin: point(px(2.), px(2.)),
            cell_w: 10.0,
            cell_h: 20.0,
            cols: 80,
            rows: 24,
            scale: 1.0,
        };
        assert_eq!(g.cell_at(point(px(2.), px(2.))), (0, 0));
        assert_eq!(g.cell_at(point(px(31.), px(41.))), (2, 1));
        assert_eq!(g.cell_at(point(px(-5.), px(-5.))), (0, 0));
        assert_eq!(g.cell_at(point(px(5000.), px(5000.))), (79, 23));
    }

    #[test]
    fn device_pixels_follow_the_scale() {
        let g = Geometry {
            origin: point(px(0.), px(0.)),
            cell_w: 9.5,
            cell_h: 16.0,
            cols: 1,
            rows: 1,
            scale: 2.0,
        };
        assert_eq!((g.device(g.cell_w), g.device(g.cell_h)), (19, 32));
    }

    #[test]
    fn padding_snaps_down_to_whole_device_pixels() {
        // 2pt = 2.667 logical px; Ghostty floors the device pixels (Surface.zig).
        assert_eq!(snap_padding(PADDING_PX, 1.0), 2.0);
        assert_eq!(snap_padding(PADDING_PX, 1.25), 2.4);
        assert_eq!(snap_padding(PADDING_PX, 2.0), 2.5);
    }

    #[test]
    fn device_snap_rounds_to_whole_pixels() {
        assert_eq!(snap_to_device(10.3, 1.0), 10.0);
        assert_eq!(snap_to_device(2.4, 1.25), 2.4);
        assert_eq!(snap_to_device(2.6, 2.0), 2.5);
    }

    #[test]
    fn a_scale_only_change_resizes() {
        let g = Geometry {
            origin: point(px(0.), px(0.)),
            cell_w: 10.0,
            cell_h: 20.0,
            cols: 80,
            rows: 24,
            scale: 1.0,
        };
        assert!(!g.needs_resize(&g));
        assert!(Geometry { scale: 2.0, ..g }.needs_resize(&g));
        let moved = Geometry {
            origin: point(px(5.), px(5.)),
            ..g
        };
        assert!(!moved.needs_resize(&g));
    }
}
