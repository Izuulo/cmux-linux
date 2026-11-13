//! The browser surface (M13 plan B1–B3): cmux's chrome bar (back, forward, reload, URL field)
//! over a CEF page rendered off screen and drawn as an image (plan E3).

mod engine;
mod input;

use std::ops::Range;
use std::sync::Arc;

use cmux_config::browser::{DEFAULT_SEARCH_URL, load as load_search_url, omnibox};
use futures::StreamExt;
use gpui::{
    App, Bounds, Context, CursorStyle, ElementInputHandler, EntityInputHandler, EventEmitter,
    FocusHandle, Focusable, ImageSource, InteractiveElement, IntoElement, KeyDownEvent, KeyUpEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point,
    Render, RenderImage, Rgba, ScrollDelta, ScrollWheelEvent, StatefulInteractiveElement, Styled,
    Subscription, UTF16Selection, Window, canvas, div, img, point, prelude::*, size, svg,
};

use engine::{Engine, Msg, Page};
pub use engine::{run_helper, shutdown};

use crate::actions::{Copy, Paste};
use crate::mac::{Theme, alpha, controls::toolbar_button, metric};
use crate::ui::px;
use crate::window_view::rename::{RenameKey, RenameOutcome, RenameState};

/// cmux `BrowserChromeMetrics` at the default tab font: 26 px buttons, 4 px padding.
const BAR_H: f32 = 34.0;
const BAR_PAD_X: f32 = 8.0;
/// Chromium's Linux wheel step, in CSS pixels per line.
const WHEEL_LINE: f32 = 53.0;

pub enum BrowserEvent {
    /// Title or URL changed (the tab title follows); not sent for cursor or loading changes.
    Changed,
    /// The page asked for a new window: a browser tab in the same pane (plan B3).
    OpenTab(String),
}

pub struct BrowserView {
    /// `None` when CEF could not start.
    engine: Option<Arc<Engine>>,
    page: Page,
    frame: Option<Arc<RenderImage>>,
    popup: Option<Arc<RenderImage>>,
    /// Replaced frames that were drawn: their atlas tiles are released on the next render.
    stale: Vec<Arc<RenderImage>>,
    drawn: (bool, bool),
    root: FocusHandle,
    bar: FocusHandle,
    page_focus: FocusHandle,
    /// The URL field while it is being edited.
    field: Option<RenameState>,
    /// The page area's window-space origin, from the last paint.
    origin: Point<Pixels>,
    /// IME preedit.
    marked: String,
    /// The chrome theme and pane background, pushed by the workspace each render.
    pub(crate) colors: Option<(Theme, Rgba)>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<BrowserEvent> for BrowserView {}

impl BrowserView {
    /// A browser at `url`; a blank one starts with the URL field focused, like cmux's new tab.
    pub fn new(url: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let engine = engine::create(url, (1, 1, window.scale_factor()), tx);
        cx.spawn(async move |this, cx| {
            while let Some(first) = rx.next().await {
                let mut batch = vec![first];
                while let Ok(m) = rx.try_recv() {
                    batch.push(m);
                }
                if this.update(cx, |v, cx| v.apply(batch, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let page_focus = cx.focus_handle();
        let bar = cx.focus_handle();
        let subs = vec![
            cx.on_focus_in(&page_focus, window, |this, _, _| {
                this.engine.iter().for_each(|e| e.focus(true));
            }),
            cx.on_blur(&page_focus, window, |this, _, _| {
                this.engine.iter().for_each(|e| e.focus(false));
            }),
            // Alt-tab clears focus too; a half-typed URL survives a window switch.
            cx.on_blur(&bar, window, |this, window, cx| {
                if window.is_window_active() && this.field.take().is_some() {
                    cx.notify();
                }
            }),
        ];
        let blank = url.is_empty() || url == "about:blank";
        Self {
            page: engine
                .as_ref()
                .map(|e| e.page.lock().unwrap().clone())
                // No engine (CEF failed to start): keep the URL so the session still saves it.
                .unwrap_or_else(|| Page {
                    url: url.into(),
                    ..Default::default()
                }),
            engine,
            frame: None,
            popup: None,
            stale: Vec::new(),
            drawn: (false, false),
            root: cx.focus_handle(),
            bar,
            page_focus,
            field: blank.then(|| RenameState::new("", true)),
            origin: Point::default(),
            marked: String::new(),
            colors: None,
            _subscriptions: subs,
        }
    }

    /// Focus is inside this view while this handle or one of its children has it.
    pub fn root_focus(&self) -> &FocusHandle {
        &self.root
    }

    /// The tab title: the page title, else the URL's host.
    pub fn title(&self) -> String {
        if !self.page.title.is_empty() {
            return self.page.title.clone();
        }
        let url = &self.page.url;
        let host = url.split_once("://").map_or(url.as_str(), |(_, rest)| {
            rest.split('/').next().unwrap_or(rest)
        });
        if host.is_empty() {
            "Browser".into()
        } else {
            host.into()
        }
    }

    pub fn url(&self) -> &str {
        &self.page.url
    }

    pub fn back(&self) {
        self.engine.iter().for_each(|e| e.back());
    }

    pub fn forward(&self) {
        self.engine.iter().for_each(|e| e.forward());
    }

    pub fn reload(&self) {
        self.engine.iter().for_each(|e| e.reload_or_stop());
    }

    /// cmux `focusBrowserAddressBar`: the whole URL selected.
    pub fn focus_bar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.field = Some(RenameState::new(&self.page.url, true));
        window.focus(&self.bar);
        cx.notify();
    }

    fn apply(&mut self, batch: Vec<Msg>, cx: &mut Context<Self>) {
        let Some(e) = self.engine.clone() else { return };
        let mut changed = false;
        for m in batch {
            match m {
                Msg::Frame => {
                    let (page, popup) = e.take_frames();
                    if let Some(f) = page {
                        let old = self.frame.replace(f);
                        if std::mem::take(&mut self.drawn.0) {
                            self.stale.extend(old);
                        }
                    }
                    if let Some(f) = popup {
                        let old = self.popup.replace(f);
                        if std::mem::take(&mut self.drawn.1) {
                            self.stale.extend(old);
                        }
                    }
                }
                Msg::Changed => changed = true,
                Msg::OpenTab(url) => cx.emit(BrowserEvent::OpenTab(url)),
            }
        }
        if changed {
            let page = e.page.lock().unwrap().clone();
            let titled = page.title != self.page.title || page.url != self.page.url;
            self.page = page;
            if self.page.popup.is_none() {
                let old = self.popup.take();
                if std::mem::take(&mut self.drawn.1) {
                    self.stale.extend(old);
                }
            }
            if titled {
                cx.emit(BrowserEvent::Changed);
            }
        }
        cx.notify();
    }

    /// Tells CEF whether this tab is on screen (plan E3: hidden tabs don't paint).
    pub fn set_hidden(&self, hidden: bool) {
        self.engine.iter().for_each(|e| e.set_hidden(hidden));
    }

    /// Frees the frames' atlas tiles: call before the view goes (a closed tab or workspace).
    pub fn release(&mut self, window: &mut Window) {
        let images = self
            .stale
            .drain(..)
            .chain(self.frame.take())
            .chain(self.popup.take());
        for img in images {
            let _ = window.drop_image(img);
        }
    }

    fn navigate(&mut self, text: &str) -> bool {
        let search = cmux_config::ghostty::Env::from_system(true)
            .map(|e| load_search_url(&e.cmux_config_file()))
            .unwrap_or_else(|| DEFAULT_SEARCH_URL.into());
        let (Some(url), Some(e)) = (omnibox(text, &search), &self.engine) else {
            return false;
        };
        e.load(&url);
        self.page.url = url;
        true
    }

    fn bar_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(f) = &mut self.field else { return };
        match f.apply(RenameKey::from(&ev.keystroke)) {
            RenameOutcome::Editing => {}
            // Enter commits whatever is typed (an unchanged URL reloads it).
            _ if ev.keystroke.key == "enter" => {
                let text = f.text.clone();
                if self.navigate(&text) {
                    self.field = None;
                    window.focus(&self.page_focus);
                }
            }
            _ => {
                self.field = None;
                window.focus(&self.page_focus);
            }
        }
        cx.notify();
    }

    /// Page-area window position → CEF mouse event in device pixels.
    fn mouse(&self, p: Point<Pixels>, mods: u32) -> cef::MouseEvent {
        let k = self.engine.as_ref().map_or(1.0, |e| e.scale());
        cef::MouseEvent {
            x: (f32::from(p.x - self.origin.x) * k) as i32,
            y: (f32::from(p.y - self.origin.y) * k) as i32,
            modifiers: mods,
        }
    }

    fn mouse_down(&mut self, e: &MouseDownEvent, window: &mut Window) {
        window.focus(&self.page_focus);
        let Some(button) = cef_button(e.button) else {
            return;
        };
        let m = self.mouse(e.position, input::mods(&e.modifiers));
        self.engine
            .iter()
            .for_each(|en| en.mouse_click(&m, button, false, e.click_count as i32));
    }

    fn mouse_up(&mut self, e: &MouseUpEvent) {
        let Some(button) = cef_button(e.button) else {
            return;
        };
        let m = self.mouse(e.position, input::mods(&e.modifiers));
        self.engine
            .iter()
            .for_each(|en| en.mouse_click(&m, button, true, e.click_count as i32));
    }

    fn mouse_move(&mut self, e: &MouseMoveEvent) {
        let mods = input::mods(&e.modifiers) | input::button_flag(e.pressed_button);
        let m = self.mouse(e.position, mods);
        self.engine.iter().for_each(|en| en.mouse_move(&m, false));
    }

    fn scroll(&mut self, e: &ScrollWheelEvent) {
        let k = self.engine.as_ref().map_or(1.0, |e| e.scale());
        let d = match e.delta {
            ScrollDelta::Pixels(p) => point(f32::from(p.x), f32::from(p.y)),
            ScrollDelta::Lines(l) => point(l.x * WHEEL_LINE, l.y * WHEEL_LINE),
        };
        let m = self.mouse(e.position, input::mods(&e.modifiers));
        let (dx, dy) = ((d.x * k) as i32, (d.y * k) as i32);
        self.engine.iter().for_each(|en| en.wheel(&m, dx, dy));
    }

    fn key(&mut self, k: &gpui::Keystroke, down: bool) {
        if let Some(e) = &self.engine {
            input::key_events(k, down).iter().for_each(|ev| e.key(ev));
        }
    }

    /// The page area's bounds this frame: CEF's view size follows them.
    fn layout(&mut self, b: Bounds<Pixels>, scale: f32) {
        self.origin = b.origin;
        if let Some(e) = &self.engine {
            let w = (f32::from(b.size.width) * scale).round() as i32;
            let h = (f32::from(b.size.height) * scale).round() as i32;
            e.set_view(w.max(1), h.max(1), scale);
        }
    }

    fn render_bar(&self, t: Theme, cx: &Context<Self>) -> impl IntoElement + use<> {
        let nav = |id: &'static str, icon: &'static str, enabled: bool, f: fn(&Self)| {
            toolbar_button(id, &t, icon, enabled)
                .on_click(cx.listener(move |this, _, _, _| f(this)))
        };
        let reload_icon = if self.page.loading {
            "icons/x.svg"
        } else {
            "icons/rotate-cw.svg"
        };
        let editing = self.field.as_ref();
        let text = match editing {
            Some(f) => f.text.clone(),
            None => self.page.url.clone(),
        };
        let placeholder = text.is_empty();
        let selected = editing.is_some_and(|f| f.replace_on_type && !f.text.is_empty());
        let caret = |show: bool| {
            (show && editing.is_some()).then(|| {
                div()
                    .flex_none()
                    .w(px(1.))
                    .h(px(metric::OMNIBAR_TEXT + 2.))
                    .bg(t.text)
            })
        };
        let field = div()
            .id("browser-url")
            .track_focus(&self.bar)
            .flex()
            .flex_row()
            .items_center()
            .flex_1()
            .min_w(px(0.))
            .h(px(metric::OMNIBAR_H))
            .ml(px(4.))
            .px(px(metric::OMNIBAR_PAD_X))
            .rounded(px(metric::OMNIBAR_RADIUS))
            .bg(t.field)
            .border_1()
            .border_color(if editing.is_some() {
                t.accent
            } else {
                t.field_border
            })
            .overflow_hidden()
            .whitespace_nowrap()
            .text_size(px(metric::OMNIBAR_TEXT))
            .text_color(t.text)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    if this.field.is_none() {
                        this.focus_bar(window, cx);
                    }
                }),
            )
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                this.bar_key(ev, window, cx);
                cx.stop_propagation();
            }))
            .on_action(cx.listener(|this, _: &Paste, _, cx| {
                let text = cx.read_from_clipboard().and_then(|c| c.text());
                if let (Some(f), Some(t)) = (&mut this.field, text) {
                    f.apply(RenameKey::Char(t.trim()));
                    cx.notify();
                }
            }))
            .when(show_lock(&self.page.url, editing.is_some()), |d| {
                d.child(
                    svg()
                        .path("icons/lock.svg")
                        .flex_none()
                        .size(px(metric::LOCK_GLYPH))
                        .mr(px(4.))
                        .text_color(t.secondary),
                )
            })
            .children(caret(placeholder))
            .child(if placeholder {
                div()
                    .text_color(t.secondary)
                    .child("Search or enter URL")
                    .into_any_element()
            } else {
                div()
                    .when(selected, |d| d.bg(alpha(t.accent, 0.3)))
                    .child(text)
                    .into_any_element()
            })
            .children(caret(!placeholder));
        div()
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .h(px(BAR_H))
            .px(px(BAR_PAD_X))
            .child(nav(
                "browser-back",
                "icons/chevron-left.svg",
                self.page.can_back,
                Self::back,
            ))
            .child(nav(
                "browser-forward",
                "icons/chevron-right.svg",
                self.page.can_forward,
                Self::forward,
            ))
            .child(nav("browser-reload", reload_icon, true, Self::reload))
            .child(field)
    }

    fn render_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scale = window.scale_factor();
        let at = |f: &Arc<RenderImage>, x: i32, y: i32| {
            let s = f.size(0);
            img(ImageSource::Render(f.clone()))
                .absolute()
                .left(gpui::px(x as f32 / scale))
                .top(gpui::px(y as f32 / scale))
                .w(gpui::px(s.width.0 as f32 / scale))
                .h(gpui::px(s.height.0 as f32 / scale))
        };
        let frame = self.frame.as_ref().map(|f| at(f, 0, 0));
        let popup = self
            .popup
            .as_ref()
            .zip(self.page.popup)
            .map(|(f, (x, y, _, _))| at(f, x, y));
        self.drawn = (frame.is_some(), popup.is_some());
        let this = cx.entity();
        let focus = self.page_focus.clone();
        let failed = self.engine.is_none().then(|| {
            div()
                .p(px(16.))
                .text_color(gpui::black())
                .child("The browser engine (CEF) failed to start; see the log.")
        });
        div()
            .id("browser-page")
            .relative()
            .flex_1()
            .min_h(px(0.))
            .overflow_hidden()
            .bg(gpui::white())
            .track_focus(&self.page_focus)
            .cursor(self.page.cursor.unwrap_or(CursorStyle::Arrow))
            .on_any_mouse_down(cx.listener(|this, e, window, _| this.mouse_down(e, window)))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e, _, _| this.mouse_up(e)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, e, _, _| this.mouse_up(e)),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, e, _, _| this.mouse_up(e)),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, e, _, _| this.mouse_up(e)),
            )
            .on_mouse_move(cx.listener(|this, e, _, _| this.mouse_move(e)))
            .on_hover(cx.listener(|this, hovered: &bool, _, _| {
                if !hovered {
                    let m = this.mouse(this.origin, 0);
                    this.engine.iter().for_each(|e| e.mouse_move(&m, true));
                }
            }))
            .on_scroll_wheel(cx.listener(|this, e, _, _| this.scroll(e)))
            // After the app's bindings (they match first): the rest goes to the page.
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                this.key(&e.keystroke, true);
                cx.stop_propagation();
            }))
            .on_key_up(cx.listener(|this, e: &KeyUpEvent, _, cx| {
                this.key(&e.keystroke, false);
                cx.stop_propagation();
            }))
            .on_action(cx.listener(|this, _: &Copy, _, _| {
                this.engine.iter().for_each(|e| e.copy());
            }))
            .on_action(cx.listener(|this, _: &Paste, _, _| {
                this.engine.iter().for_each(|e| e.paste());
            }))
            .children(frame)
            .children(popup)
            .children(failed)
            .child(
                canvas(
                    |b, _, _| b,
                    move |b, _, window, cx| {
                        this.update(cx, |v, _| v.layout(b, scale));
                        window.handle_input(&focus, ElementInputHandler::new(b, this), cx);
                    },
                )
                .absolute()
                .size_full(),
            )
    }
}

fn cef_button(b: MouseButton) -> Option<cef::MouseButtonType> {
    match b {
        MouseButton::Left => Some(cef::MouseButtonType::LEFT),
        MouseButton::Middle => Some(cef::MouseButtonType::MIDDLE),
        MouseButton::Right => Some(cef::MouseButtonType::RIGHT),
        _ => None,
    }
}

/// The omnibar's lock: an https page whose field isn't being edited.
fn show_lock(url: &str, editing: bool) -> bool {
    !editing && url.starts_with("https://")
}

impl Drop for BrowserView {
    fn drop(&mut self) {
        self.engine.iter().for_each(|e| e.close());
    }
}

impl Focusable for BrowserView {
    /// The URL field while it is being edited, else the page.
    fn focus_handle(&self, _: &App) -> FocusHandle {
        match self.field {
            Some(_) => self.bar.clone(),
            None => self.page_focus.clone(),
        }
    }
}

impl Render for BrowserView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        for old in self.stale.drain(..) {
            let _ = window.drop_image(old);
        }
        let (t, bg) = self.colors.unwrap_or_else(|| {
            let bg = gpui::rgba(0x1e1e1eff);
            (Theme::new(bg), bg)
        });
        div()
            .track_focus(&self.root)
            .size_full()
            .flex()
            .flex_col()
            .bg(bg)
            .child(self.render_bar(t, cx))
            .child(self.render_page(window, cx))
    }
}

/// IME through gpui's input handler (plan B2): preedit → `ime_set_composition`, commit →
/// `ime_commit_text`. Plain keys stop propagation in `on_key_down`, so gpui's fallback never
/// types them a second time here (as in `ime.rs`).
impl EntityInputHandler for BrowserView {
    fn text_for_range(
        &mut self,
        _: Range<usize>,
        _: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        (!self.marked.is_empty()).then(|| 0..self.marked.encode_utf16().count())
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        if !std::mem::take(&mut self.marked).is_empty() {
            self.engine.iter().for_each(|e| e.ime_cancel());
        }
    }

    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.marked.clear();
        if let Some(e) = &self.engine {
            match text.is_empty() {
                true => e.ime_cancel(),
                false => e.ime_commit(text),
            }
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.marked = text.to_string();
        self.engine.iter().for_each(|e| e.ime_compose(text));
    }

    /// The candidate window goes under the composition, as CEF reports it.
    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let e = self.engine.as_ref()?;
        let k = e.scale();
        let (x, y, w, h) = e.page.lock().unwrap().ime?;
        Some(Bounds::new(
            self.origin + point(gpui::px(x as f32 / k), gpui::px(y as f32 / k)),
            size(gpui::px(w as f32 / k), gpui::px(h as f32 / k)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod lock_tests {
    use super::show_lock;

    #[test]
    fn lock_shows_for_https_when_not_editing() {
        assert!(show_lock("https://example.com", false));
        assert!(!show_lock("https://example.com", true));
        assert!(!show_lock("http://example.com", false));
        assert!(!show_lock("about:blank", false));
        assert!(!show_lock("", false));
    }
}
