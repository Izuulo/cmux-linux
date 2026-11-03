//! The terminal view: owns the PTY session, turns gpui input into PTY bytes and hosts the
//! element that paints the grid. The PTY-to-repaint loop and input routing follow Zed's
//! terminal (zed-industries/zed@69e2130, crates/terminal/src/terminal.rs and
//! crates/terminal_view/src/terminal_view.rs, GPL-3.0-or-later, Copyright Zed Industries).

use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

use cmux_term::TermEvent;
use cmux_term::ghostty_vt::{KeyAction, KeyEncoder, MouseEncoders};
use cmux_term::input::{encode_focus, encode_paste, paste_needs_confirmation};
use cmux_term::pty::{PtyEvent, Session};
use cmux_term::selection::Selection;
use futures::StreamExt;
use futures::channel::mpsc;
use gpui::{
    App, ClipboardItem, Context, EventEmitter, FocusHandle, Focusable, IntoElement, KeyDownEvent,
    KeyUpEvent, Keystroke, MouseButton, ParentElement, Pixels, Point, PromptLevel, Render, Styled,
    Subscription, Task, Window, div, prelude::*, px,
};

use crate::actions::{Copy, Paste};
use crate::keys;
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

/// What the workspace hears from a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalEvent {
    /// The shell exited; the workspace closes this tab.
    Exited,
    /// The OSC 0/2 title changed.
    TitleChanged,
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
    /// `Keystroke.key` of presses that went to the PTY, so their release goes there too even
    /// when a binding (a Copy with nothing selected) let the press through.
    forwarded_keys: HashSet<String>,
    title: Option<String>,
    /// Last OSC 7 directory, for new splits and tabs.
    cwd: Option<String>,
    /// Where the shell was started; the last resort when neither OSC 7 nor the OS knows.
    start_cwd: Option<String>,
    /// `CMUX_TRACE_LATENCY=1`: key press time, waiting for its echo.
    trace_latency: bool,
    key_at: Option<Instant>,
    pub(crate) echo_at: Option<Instant>,
    _tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl TerminalView {
    /// Starts the user's shell, in `cwd` when it is an existing directory (else `$HOME`).
    /// Fallible so the caller can survive a failed spawn; pass the result to [`Self::new`].
    pub fn start(
        style: &TermStyle,
        cwd: Option<&str>,
    ) -> std::io::Result<(Session, mpsc::UnboundedReceiver<PtyEvent>)> {
        let (tx, rx) = mpsc::unbounded::<PtyEvent>();
        let mut cmd = Session::shell_command();
        if let Some(dir) = cwd.filter(|d| std::path::Path::new(d).is_dir()) {
            cmd.cwd(dir);
        }
        let session = Session::spawn(
            cmd,
            80,
            24,
            |term| style.apply_to(term),
            move |ev| {
                let _ = tx.unbounded_send(ev);
            },
        )?;
        Ok((session, rx))
    }

    pub fn new(
        style: Rc<TermStyle>,
        cwd: Option<String>,
        (session, mut rx): (Session, mpsc::UnboundedReceiver<PtyEvent>),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
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
            forwarded_keys: HashSet::new(),
            title: None,
            cwd: None,
            start_cwd: cwd,
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
                // Ghostty closes the surface when its shell exits; the workspace does that.
                PtyEvent::Exited => return cx.emit(TerminalEvent::Exited),
                PtyEvent::Output(events) => {
                    if let Some(at) = self.key_at.take() {
                        self.echo_at = Some(at);
                    }
                    for e in events {
                        match e {
                            TermEvent::Title(t) => {
                                self.title = Some(t);
                                cx.emit(TerminalEvent::TitleChanged);
                            }
                            TermEvent::Cwd(dir) => self.cwd = Some(dir),
                            // Bell and notifications feed the sidebar and attention UI (M3/M4).
                            TermEvent::Bell | TermEvent::Notify { .. } => {}
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    /// The OSC 0/2 title, if the shell or program set one.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The working directory for new splits and tabs: the last OSC 7 one (what the shell
    /// reports), else the OS's view of the shell's cwd, else where it was started.
    pub fn cwd(&self) -> Option<String> {
        self.cwd
            .clone()
            .or_else(|| self.session.cwd())
            .or_else(|| self.start_cwd.clone())
    }

    /// Only the OSC 7 cwd: no syscalls, safe to call on the UI thread for the sidebar.
    pub fn osc_cwd(&self) -> Option<String> {
        self.cwd.clone()
    }

    /// The shell's process id.
    pub fn pid(&self) -> Option<u32> {
        self.session.process_id()
    }

    /// Whether a foreground job (not just the idle shell) is running.
    pub fn busy(&self) -> bool {
        self.session.foreground_busy()
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
        // A key released while unfocused never sends key_up here; forget it.
        self.forwarded_keys.clear();
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

    /// Runs only for keys no action consumed: gpui dispatches bound chords to their action
    /// handlers first (research-gpui.md §1.6).
    fn key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
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
            self.forwarded_keys.insert(ev.keystroke.key.clone());
            self.send_input(bytes);
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn key_up(&mut self, ev: &KeyUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        let forwarded = self.forwarded_keys.remove(&ev.keystroke.key);
        if ev.keystroke.modifiers.platform {
            return;
        }
        // A release whose press ran an app action must not reach the PTY either.
        let bound = || {
            !cx.key_bindings()
                .borrow()
                .bindings_for_input(std::slice::from_ref(&ev.keystroke), &window.context_stack())
                .0
                .is_empty()
        };
        if !forwarded && bound() {
            return;
        }
        // Empty unless the app asked for kitty's report-event-types flag.
        let bytes = self.encode_key(&ev.keystroke, KeyAction::Release);
        if !bytes.is_empty() {
            self.session.write(bytes);
        }
    }

    /// Copy action. With nothing selected the key falls through to `key_down` (and so to the
    /// PTY for Ctrl+Shift+C), like Ghostty's `performable:` bindings.
    fn copy_action(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if self.selection.is_none() {
            return cx.propagate();
        }
        self.copy(cx);
    }

    fn paste_action(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.paste_text(text, window, cx);
        }
    }

    pub(crate) fn copy(&mut self, cx: &mut Context<Self>) {
        let Some(selection) = &self.selection else {
            return;
        };
        // Bind first so the Term lock is released before the clipboard call.
        let text = selection.text(&mut self.session.term.lock().unwrap());
        if let Some(text) = text {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    /// Pastes `text`, first asking when Ghostty's paste protection says it is unsafe.
    pub(crate) fn paste_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let bracketed = self.session.term.lock().unwrap().vt().mode(2004, false);
        if !self.style.cfg.clipboard_paste_protection || !paste_needs_confirmation(&text, bracketed)
        {
            return self.send_input(encode_paste(&text, bracketed));
        }
        // gpui draws its own prompt on Linux (platform prompt is None, window/prompts.rs).
        let answer = window.prompt(
            PromptLevel::Warning,
            "Paste text that may run commands?",
            Some("The text contains a newline, so pasting it may run commands in the shell."),
            &["Paste", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if answer.await == Ok(0) {
                let _ = this.update(cx, |view, _| {
                    // Bracketed paste may have changed while the prompt was up.
                    let bracketed = view.session.term.lock().unwrap().vt().mode(2004, false);
                    view.send_input(encode_paste(&text, bracketed));
                });
            }
        })
        .detach();
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl EventEmitter<TerminalEvent> for TerminalView {}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p(px(snap_padding(PADDING_PX, window.scale_factor())))
            .bg(self.style.window_background())
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_action(cx.listener(Self::copy_action))
            .on_action(cx.listener(Self::paste_action))
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            // A selection drag released over another pane or a divider still has to end here.
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, ev, w, cx| {
                    if this.dragging {
                        this.mouse_up(ev, w, cx);
                    }
                }),
            )
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
