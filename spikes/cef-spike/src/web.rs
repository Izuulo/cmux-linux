//! CEF side of the spike: process setup, one off-screen browser, input forwarding.
use cef::*;
use futures::channel::mpsc::UnboundedSender;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// State shared between CEF's UI thread and gpui's main thread.
pub struct Shared {
    /// Logical (DIP) view size and device scale, set by gpui, read by CEF.
    pub view: Mutex<(i32, i32, f32)>,
    browser: Mutex<Option<Browser>>,
    pub frames: UnboundedSender<Arc<gpui::RenderImage>>,
    pub title: Mutex<String>,
    pub url: Mutex<String>,
    pub loading: AtomicBool,
    pub closed: AtomicBool,
    pub paints: AtomicU64,
    pub last_paint_size: Mutex<(i32, i32)>,
}
// CEF objects are thread-safe ref-counted handles; host methods post to the UI thread themselves.
unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

impl Shared {
    pub fn new(frames: UnboundedSender<Arc<gpui::RenderImage>>) -> Arc<Self> {
        Arc::new(Self {
            view: Mutex::new((800, 600, 1.0)),
            browser: Mutex::new(None),
            frames,
            title: Mutex::default(),
            url: Mutex::default(),
            loading: AtomicBool::new(true),
            closed: AtomicBool::new(false),
            paints: AtomicU64::new(0),
            last_paint_size: Mutex::default(),
        })
    }

    fn host(&self) -> Option<BrowserHost> {
        self.browser.lock().unwrap().as_ref()?.host()
    }

    fn mouse(&self, x: f32, y: f32, mods: u32) -> MouseEvent {
        let k = if zoom_dsf() {
            self.view.lock().unwrap().2
        } else {
            1.0
        };
        mouse_ev(x * k, y * k, mods)
    }

    pub fn has_browser(&self) -> bool {
        self.browser.lock().unwrap().is_some()
    }

    pub fn resized(&self, scale_changed: bool) {
        if let Some(h) = self.host() {
            if scale_changed {
                h.notify_screen_info_changed();
            }
            if zoom_dsf() {
                h.set_zoom_level((self.view.lock().unwrap().2 as f64).ln() / 1.2f64.ln());
            }
            h.was_resized();
        }
    }

    pub fn focus(&self, on: bool) {
        if let Some(h) = self.host() {
            h.set_focus(on as i32);
        }
    }

    pub fn mouse_move(&self, x: f32, y: f32, mods: u32) {
        if let Some(h) = self.host() {
            h.send_mouse_move_event(Some(&self.mouse(x, y, mods)), 0);
        }
    }

    pub fn mouse_click(
        &self,
        x: f32,
        y: f32,
        mods: u32,
        button: MouseButtonType,
        up: bool,
        count: i32,
    ) {
        if let Some(h) = self.host() {
            h.send_mouse_click_event(Some(&self.mouse(x, y, mods)), button, up as i32, count);
        }
    }

    pub fn wheel(&self, x: f32, y: f32, mods: u32, dx: i32, dy: i32) {
        if let Some(h) = self.host() {
            h.send_mouse_wheel_event(Some(&self.mouse(x, y, mods)), dx, dy);
        }
    }

    pub fn key(&self, ty: KeyEventType, vk: i32, ch: u16, mods: u32) {
        if let Some(h) = self.host() {
            let ev = KeyEvent {
                type_: ty,
                modifiers: mods,
                windows_key_code: vk,
                character: ch,
                unmodified_character: ch,
                ..Default::default()
            };
            h.send_key_event(Some(&ev));
        }
    }

    pub fn exec_js(&self, code: &str) {
        let frame = self
            .browser
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|b| b.main_frame());
        if let Some(f) = frame {
            f.execute_java_script(Some(&CefString::from(code)), None, 0);
        }
    }

    pub fn load(&self, url: &str) {
        let frame = self
            .browser
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|b| b.main_frame());
        if let Some(f) = frame {
            f.load_url(Some(&CefString::from(url)));
        }
    }

    pub fn close(&self) {
        if let Some(h) = self.host() {
            h.close_browser(1);
        }
    }
}

/// `SPIKE_DSF=zoom`: CEF is told scale 1 and a device-pixel view, and page zoom = scale.
pub fn zoom_dsf() -> bool {
    static Z: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *Z.get_or_init(|| std::env::var("SPIKE_DSF").is_ok_and(|v| v == "zoom"))
}

fn mouse_ev(x: f32, y: f32, modifiers: u32) -> MouseEvent {
    MouseEvent {
        x: x as i32,
        y: y as i32,
        modifiers,
    }
}

wrap_app! {
    pub struct SpikeApp {
        switches: Vec<String>,
    }

    impl App {
        fn on_before_command_line_processing(
            &self,
            _process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            let Some(cl) = command_line else { return };
            for s in &self.switches {
                match s.split_once('=') {
                    Some((k, v)) => cl.append_switch_with_value(
                        Some(&CefString::from(k)),
                        Some(&CefString::from(v)),
                    ),
                    None => cl.append_switch(Some(&CefString::from(s.as_str()))),
                }
            }
        }
    }
}

/// Extra Chromium switches from `SPIKE_SWITCHES="a,b=c"`.
pub fn app() -> cef::App {
    let switches = std::env::var("SPIKE_SWITCHES")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    SpikeApp::new(switches)
}

wrap_render_handler! {
    struct Rh {
        s: Arc<Shared>,
    }

    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            let (w, h, scale) = *self.s.view.lock().unwrap();
            let k = if zoom_dsf() { scale } else { 1.0 };
            let (w, h) = ((w as f32 * k) as i32, (h as f32 * k) as i32);
            if let Some(r) = rect {
                *r = Rect { x: 0, y: 0, width: w.max(1), height: h.max(1) };
            }
        }

        fn screen_info(&self, _browser: Option<&mut Browser>, info: Option<&mut ScreenInfo>) -> ::std::os::raw::c_int {
            let (w, h, scale) = *self.s.view.lock().unwrap();
            let Some(i) = info else { return 0 };
            if std::env::var_os("SPIKE_DEBUG").is_some() {
                eprintln!("[spike] screen_info in: dsf={} size={} depth={} rect={:?}", i.device_scale_factor, i.size, i.depth, (i.rect.width, i.rect.height));
            }
            i.depth = 24;
            i.depth_per_component = 8;
            i.device_scale_factor = if zoom_dsf() { 1.0 } else { scale };
            i.rect = Rect { x: 0, y: 0, width: w, height: h };
            i.available_rect = i.rect.clone();
            1
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
            // shortcut: popups (<select> dropdowns) are ignored, M13 must composite PET_POPUP.
            if type_.get_raw() != PaintElementType::VIEW.get_raw() || buffer.is_null() {
                return;
            }
            let len = width as usize * height as usize * 4;
            let bgra = unsafe { std::slice::from_raw_parts(buffer, len) }.to_vec();
            // gpui's RenderImage holds BGRA, the same order CEF paints in: no swizzle.
            let buf = image::RgbaImage::from_raw(width as u32, height as u32, bgra).unwrap();
            let img = gpui::RenderImage::new(smallvec::smallvec![image::Frame::new(buf)]);
            self.s.paints.fetch_add(1, Ordering::Relaxed);
            *self.s.last_paint_size.lock().unwrap() = (width, height);
            let _ = self.s.frames.unbounded_send(Arc::new(img));
        }
    }
}

wrap_display_handler! {
    struct Dh {
        s: Arc<Shared>,
    }

    impl DisplayHandler {
        fn on_title_change(&self, _browser: Option<&mut Browser>, title: Option<&CefString>) {
            let t = title.map(|t| t.to_string()).unwrap_or_default();
            *self.s.title.lock().unwrap() = t;
        }

        fn on_address_change(&self, _browser: Option<&mut Browser>, frame: Option<&mut Frame>, url: Option<&CefString>) {
            if frame.is_some_and(|f| f.is_main() == 1) {
                let u = url.map(|u| u.to_string()).unwrap_or_default();
                eprintln!("[spike] address: {u}");
                *self.s.url.lock().unwrap() = u;
                // Chromium keeps zoom per host: re-apply it after each navigation.
                self.s.resized(false);
            }
        }
    }
}

wrap_load_handler! {
    struct Lh {
        s: Arc<Shared>,
    }

    impl LoadHandler {
        fn on_loading_state_change(&self, _browser: Option<&mut Browser>, is_loading: ::std::os::raw::c_int, _back: ::std::os::raw::c_int, _fwd: ::std::os::raw::c_int) {
            self.s.loading.store(is_loading != 0, Ordering::SeqCst);
        }
    }
}

wrap_life_span_handler! {
    struct Ls {
        s: Arc<Shared>,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            eprintln!("[spike] browser created");
            *self.s.browser.lock().unwrap() = browser.map(|b| b.clone());
            self.s.resized(false);
        }

        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            self.s.browser.lock().unwrap().take();
            self.s.closed.store(true, Ordering::SeqCst);
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

/// `initialize` for windowless rendering. `external_pump`: we call `do_message_loop_work`
/// from a gpui timer; otherwise CEF runs its own UI thread.
pub fn init(args: &args::Args, app: &mut cef::App, external_pump: bool) -> bool {
    let cache = std::env::var("SPIKE_CACHE").unwrap_or_else(|_| {
        format!(
            "{}/cef-spike",
            std::env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".into())
        )
    });
    let settings = Settings {
        windowless_rendering_enabled: 1,
        external_message_pump: external_pump as i32,
        multi_threaded_message_loop: (!external_pump) as i32,
        no_sandbox: std::env::var_os("SPIKE_NO_SANDBOX").is_some() as i32,
        root_cache_path: CefString::from(cache.as_str()),
        cache_path: CefString::from(format!("{cache}/default").as_str()),
        ..Default::default()
    };
    initialize(
        Some(args.as_main_args()),
        Some(&settings),
        Some(app),
        std::ptr::null_mut(),
    ) == 1
}

pub fn create_browser(s: &Arc<Shared>, url: &str) -> bool {
    let wi = WindowInfo::default().set_as_windowless(0);
    let mut client = Cl::new(
        Rh::new(s.clone()),
        Dh::new(s.clone()),
        Lh::new(s.clone()),
        Ls::new(s.clone()),
    );
    let bs = BrowserSettings {
        windowless_frame_rate: 60,
        ..Default::default()
    };
    browser_host_create_browser(
        Some(&wi),
        Some(&mut client),
        Some(&CefString::from(url)),
        Some(&bs),
        None,
        None,
    ) == 1
}
