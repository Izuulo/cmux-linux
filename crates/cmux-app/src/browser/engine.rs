//! CEF side of the browser (M13 plan E1–E3, from the spike `spikes/cef-spike/src/web.rs`):
//! process setup, one off-screen browser per `Engine`, frames and page state to gpui.
//!
//! Threads: CEF runs its own UI thread (`multi_threaded_message_loop`); its callbacks fill
//! `Engine` and send `Msg`s that a gpui task applies. Host methods post to CEF's UI thread.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use cef::*;
use futures::channel::mpsc::UnboundedSender;
use gpui::{CursorStyle, RenderImage};

/// What the CEF thread tells the view.
pub enum Msg {
    /// New pixels wait in `Engine::frames` (sent only when the slot was empty, so at most one
    /// page frame and one popup frame are ever queued).
    Frame,
    /// Title, URL, loading, history, cursor or popup changed.
    Changed,
    /// The page asked for a new window (`target=_blank`, `window.open`).
    OpenTab(String),
}

#[derive(Default, Clone)]
pub struct Page {
    pub title: String,
    pub url: String,
    pub loading: bool,
    pub can_back: bool,
    pub can_forward: bool,
    pub cursor: Option<CursorStyle>,
    /// Device-pixel rect of a shown `<select>` popup.
    pub popup: Option<(i32, i32, i32, i32)>,
    /// Device-pixel rect of the first IME composition character.
    pub ime: Option<(i32, i32, i32, i32)>,
}

/// The newest (page, `<select>` popup) frames.
pub type Frames = (Option<Arc<RenderImage>>, Option<Arc<RenderImage>>);

/// `Browser` is a thread-safe ref-counted CEF handle, but cef-rs doesn't mark it `Send`.
struct Handle(Browser);
// SAFETY: CEF browser objects are ref-counted thread-safely; host methods post to CEF's UI thread.
unsafe impl Send for Handle {}

pub struct Engine {
    /// Page size in device pixels and the window scale.
    view: Mutex<(i32, i32, f32)>,
    browser: Mutex<Option<Handle>>,
    tx: UnboundedSender<Msg>,
    pub page: Mutex<Page>,
    /// The newest page frame and `<select>` popup frame not yet taken by the view.
    frames: Mutex<Frames>,
    hidden: AtomicBool,
    close_requested: AtomicBool,
    closed: AtomicBool,
}

impl Engine {
    fn host(&self) -> Option<BrowserHost> {
        self.browser.lock().unwrap().as_ref()?.0.host()
    }

    fn with_browser(&self, f: impl FnOnce(&Browser)) {
        if let Some(b) = self.browser.lock().unwrap().as_ref() {
            f(&b.0);
        }
    }

    fn changed(&self) {
        let _ = self.tx.unbounded_send(Msg::Changed);
    }

    pub fn scale(&self) -> f32 {
        self.view.lock().unwrap().2
    }

    /// The page area's size in device pixels. CEF ignores `device_scale_factor` on Linux
    /// (spike), so the page is laid out in device pixels and zoomed by `scale` instead.
    pub fn set_view(&self, w: i32, h: i32, scale: f32) {
        let v = (w, h, scale);
        if std::mem::replace(&mut *self.view.lock().unwrap(), v) != v {
            self.resized();
        }
    }

    fn resized(&self) {
        if let Some(h) = self.host() {
            h.set_zoom_level((self.scale() as f64).ln() / 1.2f64.ln());
            h.was_resized();
        }
    }

    /// Taken by the view on `Msg::Frame`: (page, popup).
    pub fn take_frames(&self) -> Frames {
        std::mem::take(&mut *self.frames.lock().unwrap())
    }

    /// A tab not on screen stops painting (and animating) until shown again.
    pub fn set_hidden(&self, hidden: bool) {
        if self.hidden.swap(hidden, Ordering::SeqCst) != hidden
            && let Some(h) = self.host()
        {
            h.was_hidden(hidden as i32);
        }
    }

    pub fn focus(&self, on: bool) {
        if let Some(h) = self.host() {
            h.set_focus(on as i32);
        }
    }

    pub fn mouse_move(&self, ev: &MouseEvent, leave: bool) {
        if let Some(h) = self.host() {
            h.send_mouse_move_event(Some(ev), leave as i32);
        }
    }

    pub fn mouse_click(&self, ev: &MouseEvent, button: MouseButtonType, up: bool, count: i32) {
        if let Some(h) = self.host() {
            h.send_mouse_click_event(Some(ev), button, up as i32, count);
        }
    }

    pub fn wheel(&self, ev: &MouseEvent, dx: i32, dy: i32) {
        if let Some(h) = self.host() {
            h.send_mouse_wheel_event(Some(ev), dx, dy);
        }
    }

    pub fn key(&self, ev: &KeyEvent) {
        if let Some(h) = self.host() {
            h.send_key_event(Some(ev));
        }
    }

    pub fn ime_compose(&self, text: &str) {
        if let Some(h) = self.host() {
            let none = invalid_range();
            let caret = text.encode_utf16().count() as u32;
            h.ime_set_composition(
                Some(&CefString::from(text)),
                None,
                Some(&none),
                Some(&Range {
                    from: caret,
                    to: caret,
                }),
            );
        }
    }

    pub fn ime_commit(&self, text: &str) {
        if let Some(h) = self.host() {
            h.ime_commit_text(Some(&CefString::from(text)), Some(&invalid_range()), 0);
        }
    }

    pub fn ime_cancel(&self) {
        if let Some(h) = self.host() {
            h.ime_cancel_composition();
        }
    }

    pub fn load(&self, url: &str) {
        self.with_browser(|b| {
            if let Some(f) = b.main_frame() {
                f.load_url(Some(&CefString::from(url)));
            }
        });
    }

    pub fn back(&self) {
        self.with_browser(|b| b.go_back());
    }

    pub fn forward(&self) {
        self.with_browser(|b| b.go_forward());
    }

    /// Reload, or stop while loading (cmux's reload/stop button).
    pub fn reload_or_stop(&self) {
        let loading = self.page.lock().unwrap().loading;
        self.with_browser(|b| if loading { b.stop_load() } else { b.reload() });
    }

    pub fn copy(&self) {
        self.with_browser(|b| {
            if let Some(f) = b.focused_frame() {
                f.copy();
            }
        });
    }

    pub fn paste(&self) {
        self.with_browser(|b| {
            if let Some(f) = b.focused_frame() {
                f.paste();
            }
        });
    }

    /// Closes the browser (the tab closed or the app quits). Also before it exists.
    pub fn close(&self) {
        self.close_requested.store(true, Ordering::SeqCst);
        if let Some(h) = self.host() {
            h.close_browser(1);
        }
    }
}

fn invalid_range() -> Range {
    Range {
        from: u32::MAX,
        to: u32::MAX,
    }
}

fn frame(buffer: *const u8, width: i32, height: i32) -> Option<Arc<RenderImage>> {
    if buffer.is_null() || width <= 0 || height <= 0 {
        return None;
    }
    let len = width as usize * height as usize * 4;
    // SAFETY: CEF hands a `width * height * 4` BGRA buffer, valid for this call.
    let bgra = unsafe { std::slice::from_raw_parts(buffer, len) }.to_vec();
    // gpui's RenderImage holds BGRA, the order CEF paints in: no swizzle.
    let buf = image::RgbaImage::from_raw(width as u32, height as u32, bgra)?;
    Some(Arc::new(RenderImage::new(smallvec::smallvec![
        image::Frame::new(buf)
    ])))
}

wrap_render_handler! {
    struct Rh {
        e: Arc<Engine>,
    }

    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            let (w, h, _) = *self.e.view.lock().unwrap();
            if let Some(r) = rect {
                *r = Rect { x: 0, y: 0, width: w.max(1), height: h.max(1) };
            }
        }

        fn screen_info(&self, _browser: Option<&mut Browser>, info: Option<&mut ScreenInfo>) -> ::std::os::raw::c_int {
            let (w, h, _) = *self.e.view.lock().unwrap();
            let Some(i) = info else { return 0 };
            i.depth = 24;
            i.depth_per_component = 8;
            i.device_scale_factor = 1.0;
            i.rect = Rect { x: 0, y: 0, width: w.max(1), height: h.max(1) };
            i.available_rect = i.rect.clone();
            1
        }

        fn on_popup_show(&self, _browser: Option<&mut Browser>, show: ::std::os::raw::c_int) {
            if show == 0 {
                self.e.page.lock().unwrap().popup = None;
                self.e.changed();
            }
        }

        fn on_popup_size(&self, _browser: Option<&mut Browser>, rect: Option<&Rect>) {
            if let Some(r) = rect {
                self.e.page.lock().unwrap().popup = Some((r.x, r.y, r.width, r.height));
                self.e.changed();
            }
        }

        fn on_paint(
            &self,
            _browser: Option<&mut Browser>,
            type_: PaintElementType,
            _dirty_rects: Option<&[Rect]>,
            buffer: *const u8,
            width: ::std::os::raw::c_int,
            height: ::std::os::raw::c_int
        ) {
            let Some(img) = frame(buffer, width, height) else { return };
            let popup = type_.get_raw() == PaintElementType::POPUP.get_raw();
            let mut slots = self.e.frames.lock().unwrap();
            let wake = slots.0.is_none() && slots.1.is_none();
            match popup {
                true => slots.1 = Some(img),
                false => slots.0 = Some(img),
            }
            drop(slots);
            if wake {
                let _ = self.e.tx.unbounded_send(Msg::Frame);
            }
        }

        fn on_ime_composition_range_changed(
            &self,
            _browser: Option<&mut Browser>,
            _selected_range: Option<&cef::Range>,
            character_bounds: Option<&[Rect]>,
        ) {
            let r = character_bounds.and_then(|b| b.first()).map(|r| (r.x, r.y, r.width, r.height));
            self.e.page.lock().unwrap().ime = r;
        }
    }
}

wrap_display_handler! {
    struct Dh {
        e: Arc<Engine>,
    }

    impl DisplayHandler {
        fn on_title_change(&self, _browser: Option<&mut Browser>, title: Option<&CefString>) {
            self.e.page.lock().unwrap().title = title.map(|t| t.to_string()).unwrap_or_default();
            self.e.changed();
        }

        fn on_address_change(&self, _browser: Option<&mut Browser>, frame: Option<&mut Frame>, url: Option<&CefString>) {
            if frame.is_some_and(|f| f.is_main() == 1) {
                self.e.page.lock().unwrap().url = url.map(|u| u.to_string()).unwrap_or_default();
                // Chromium keeps zoom per host: re-apply the scale after each navigation.
                self.e.resized();
                self.e.changed();
            }
        }

        fn on_cursor_change(
            &self,
            _browser: Option<&mut Browser>,
            _cursor: ::std::os::raw::c_ulong,
            type_: CursorType,
            _custom_cursor_info: Option<&CursorInfo>,
        ) -> ::std::os::raw::c_int {
            let c = super::input::cursor(type_.get_raw() as i32);
            let mut p = self.e.page.lock().unwrap();
            if p.cursor != c {
                p.cursor = c;
                drop(p);
                self.e.changed();
            }
            1
        }
    }
}

wrap_load_handler! {
    struct Lh {
        e: Arc<Engine>,
    }

    impl LoadHandler {
        fn on_loading_state_change(&self, _browser: Option<&mut Browser>, is_loading: ::std::os::raw::c_int, can_back: ::std::os::raw::c_int, can_forward: ::std::os::raw::c_int) {
            {
                let mut p = self.e.page.lock().unwrap();
                p.loading = is_loading != 0;
                p.can_back = can_back != 0;
                p.can_forward = can_forward != 0;
            }
            self.e.changed();
        }
    }
}

wrap_life_span_handler! {
    struct Ls {
        e: Arc<Engine>,
    }

    impl LifeSpanHandler {
        #[allow(clippy::too_many_arguments)]
        fn on_before_popup(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _popup_id: ::std::os::raw::c_int,
            target_url: Option<&CefString>,
            _target_frame_name: Option<&CefString>,
            _target_disposition: WindowOpenDisposition,
            user_gesture: ::std::os::raw::c_int,
            _popup_features: Option<&PopupFeatures>,
            _window_info: Option<&mut WindowInfo>,
            _client: Option<&mut Option<Client>>,
            _settings: Option<&mut BrowserSettings>,
            _extra_info: Option<&mut Option<DictionaryValue>>,
            _no_javascript_access: Option<&mut ::std::os::raw::c_int>,
        ) -> ::std::os::raw::c_int {
            // Plan B3: a new browser tab in the same pane instead of a window; only for a click
            // or key, so a page can't open tabs on its own.
            let url = target_url.map(|u| u.to_string()).filter(|u| !u.is_empty());
            if let Some(u) = url.filter(|_| user_gesture != 0) {
                let _ = self.e.tx.unbounded_send(Msg::OpenTab(u));
            }
            1
        }

        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let Some(b) = browser else { return };
            *self.e.browser.lock().unwrap() = Some(Handle(b.clone()));
            self.e.resized();
            // Hidden before CEF made it (a background tab).
            if self.e.hidden.load(Ordering::SeqCst)
                && let Some(h) = b.host()
            {
                h.was_hidden(1);
            }
            if self.e.close_requested.load(Ordering::SeqCst) {
                self.e.close();
            }
        }

        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            self.e.browser.lock().unwrap().take();
            self.e.closed.store(true, Ordering::SeqCst);
        }
    }
}

wrap_client! {
    struct Cl {
        rh: RenderHandler,
        dh: DisplayHandler,
        lh: LoadHandler,
        ls: LifeSpanHandler,
    }

    impl Client {
        fn render_handler(&self) -> Option<RenderHandler> {
            Some(self.rh.clone())
        }
        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(self.dh.clone())
        }
        fn load_handler(&self) -> Option<LoadHandler> {
            Some(self.lh.clone())
        }
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(self.ls.clone())
        }
    }
}

// ---------- process ----------

thread_local! {
    /// Main-thread CEF state: the arguments and app `execute_process` saw, until `initialize`.
    static BOOT: RefCell<Boot> = const { RefCell::new(Boot::Unset) };
}

enum Boot {
    Unset,
    Ready(cef::args::Args),
    Started,
    Failed,
}

/// Every engine made, for `shutdown`.
static LIVE: Mutex<Vec<Weak<Engine>>> = Mutex::new(Vec::new());

/// Must run first in `main`: our own binary is CEF's renderer, GPU and utility process, and
/// those return here and exit (plan E2). The browser process continues.
pub fn run_helper() {
    let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    let args = cef::args::Args::new();
    let code = cef::execute_process(Some(args.as_main_args()), None, std::ptr::null_mut());
    if code >= 0 {
        std::process::exit(code);
    }
    BOOT.with_borrow_mut(|b| *b = Boot::Ready(args));
}

/// `initialize` on first use (main thread), so a session without browser tabs starts no
/// Chromium processes. False if CEF could not start.
fn ensure_started() -> bool {
    BOOT.with_borrow_mut(|b| match std::mem::replace(b, Boot::Failed) {
        Boot::Started => {
            *b = Boot::Started;
            true
        }
        Boot::Ready(args) => {
            let env = |k: &str| std::env::var(k).ok();
            let Some(dir) = cmux_ipc::paths::state_dir(&env).map(|d| d.join("browser")) else {
                eprintln!("cmux-linux: browser: neither HOME nor XDG_STATE_HOME is set");
                return false;
            };
            let dir = dir.to_string_lossy();
            let settings = Settings {
                windowless_rendering_enabled: 1,
                multi_threaded_message_loop: 1,
                root_cache_path: CefString::from(dir.as_ref()),
                cache_path: CefString::from(format!("{dir}/default").as_str()),
                ..Default::default()
            };
            let ok = initialize(
                Some(args.as_main_args()),
                Some(&settings),
                None,
                std::ptr::null_mut(),
            ) == 1;
            if ok {
                *b = Boot::Started;
            } else {
                eprintln!("cmux-linux: browser: CEF failed to initialize");
            }
            ok
        }
        // `run_helper` never ran (a test) or an earlier start failed.
        Boot::Unset | Boot::Failed => false,
    })
}

/// A new off-screen browser at `url`. `None` if CEF isn't available.
pub fn create(url: &str, view: (i32, i32, f32), tx: UnboundedSender<Msg>) -> Option<Arc<Engine>> {
    if !ensure_started() {
        return None;
    }
    let e = Arc::new(Engine {
        view: Mutex::new(view),
        browser: Mutex::new(None),
        tx,
        page: Mutex::new(Page {
            url: url.to_string(),
            loading: true,
            ..Default::default()
        }),
        frames: Mutex::default(),
        hidden: AtomicBool::new(false),
        close_requested: AtomicBool::new(false),
        closed: AtomicBool::new(false),
    });
    let wi = WindowInfo::default().set_as_windowless(0);
    let mut client = Cl::new(
        Rh::new(e.clone()),
        Dh::new(e.clone()),
        Lh::new(e.clone()),
        Ls::new(e.clone()),
    );
    let bs = BrowserSettings {
        windowless_frame_rate: 60,
        ..Default::default()
    };
    // Async: `create_browser_sync` must run on CEF's UI thread; `on_after_created` hands it over.
    let ok = browser_host_create_browser(
        Some(&wi),
        Some(&mut client),
        Some(&CefString::from(url)),
        Some(&bs),
        None,
        None,
    ) == 1;
    if !ok {
        return None;
    }
    let mut live = LIVE.lock().unwrap();
    live.retain(|w| w.strong_count() > 0);
    live.push(Arc::downgrade(&e));
    Some(e)
}

/// After gpui's loop ends: close every browser, wait (bounded) for them, then shut CEF down.
pub fn shutdown() {
    if !BOOT.with_borrow(|b| matches!(b, Boot::Started)) {
        return;
    }
    let live: Vec<_> = LIVE
        .lock()
        .unwrap()
        .iter()
        .filter_map(Weak::upgrade)
        .collect();
    live.iter().for_each(|e| e.close());
    let end = Instant::now() + Duration::from_secs(2);
    while Instant::now() < end && !live.iter().all(|e| e.closed.load(Ordering::SeqCst)) {
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(live);
    cef::shutdown();
}
