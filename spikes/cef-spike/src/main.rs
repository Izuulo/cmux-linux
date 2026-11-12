//! M13 go/no-go spike: a CEF off-screen browser drawn inside a gpui window.
//! `SPIKE_SCRIPT=<out dir>` drives it with synthetic gpui events and logs the measurements.
mod web;

use futures::StreamExt;
use gpui::{
    App, AppContext, AsyncApp, Bounds, Context, FocusHandle, ImageSource, InteractiveElement,
    IntoElement, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, PlatformInput, Point, Render, RenderImage,
    ScrollDelta, ScrollWheelEvent, Styled, TouchPhase, Window, WindowBounds, WindowHandle,
    WindowOptions, div, img, point, px, size,
};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use web::Shared;

const START_URL: &str = "https://example.org";
const WIKI_URL: &str = "https://en.wikipedia.org/wiki/Terminal_emulator";

struct Spike {
    s: Arc<Shared>,
    frame: Option<Arc<RenderImage>>,
    stale: Vec<Arc<RenderImage>>,
    focus: FocusHandle,
    view: (i32, i32, f32),
    frames: u64,
}

impl Render for Spike {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Frames replaced since the last draw: release their atlas tiles.
        for old in self.stale.drain(..) {
            let _ = window.drop_image(old);
        }
        let vp = window.viewport_size();
        // SPIKE_FORCE_SCALE checks the HiDPI path on a 1x screen.
        let scale = std::env::var("SPIKE_FORCE_SCALE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(window.scale_factor());
        let view = (
            f32::from(vp.width) as i32,
            f32::from(vp.height) as i32,
            scale,
        );
        if view != self.view {
            let scale_changed = view.2 != self.view.2;
            self.view = view;
            *self.s.view.lock().unwrap() = view;
            self.s.resized(scale_changed);
        }

        let page = self.frame.clone().map(|f| {
            let fs = f.size(0);
            // The frame is in device pixels; gpui scales bounds by the window scale.
            img(ImageSource::Render(f))
                .w(px(fs.width.0 as f32 / scale))
                .h(px(fs.height.0 as f32 / scale))
        });
        div()
            .size_full()
            .bg(gpui::white())
            .track_focus(&self.focus)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e, window, _| this.mouse_down(e, window)),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, e, _, _| this.mouse_up(e)),
            )
            .on_mouse_move(cx.listener(|this, e, _, _| this.mouse_move(e)))
            .on_scroll_wheel(cx.listener(|this, e, _, _| this.scroll(e)))
            .on_key_down(
                cx.listener(|this, e: &KeyDownEvent, _, _| send_key(&this.s, &e.keystroke, true)),
            )
            .on_key_up(
                cx.listener(|this, e: &KeyUpEvent, _, _| send_key(&this.s, &e.keystroke, false)),
            )
            .children(page)
    }
}

// gpui event → CEF event. The script calls these with synthetic events: `Window::dispatch_event`
// returns a private type in gpui 0.2.2, so only gpui's hit-test step is skipped.
impl Spike {
    fn mouse_down(&mut self, e: &MouseDownEvent, window: &mut Window) {
        window.focus(&self.focus);
        self.s.focus(true);
        let p = e.position;
        self.s.mouse_click(
            f32::from(p.x),
            f32::from(p.y),
            mods(&e.modifiers),
            cef::MouseButtonType::LEFT,
            false,
            e.click_count as i32,
        );
    }

    fn mouse_up(&mut self, e: &MouseUpEvent) {
        let p = e.position;
        self.s.mouse_click(
            f32::from(p.x),
            f32::from(p.y),
            mods(&e.modifiers),
            cef::MouseButtonType::LEFT,
            true,
            e.click_count as i32,
        );
    }

    fn mouse_move(&mut self, e: &MouseMoveEvent) {
        let p = e.position;
        let m = mods(&e.modifiers)
            | if e.pressed_button == Some(MouseButton::Left) {
                1 << 4
            } else {
                0
            };
        self.s.mouse_move(f32::from(p.x), f32::from(p.y), m);
    }

    fn scroll(&mut self, e: &ScrollWheelEvent) {
        // Lines → pixels at Chromium's Linux wheel step.
        let d = match e.delta {
            ScrollDelta::Pixels(p) => point(f32::from(p.x), f32::from(p.y)),
            ScrollDelta::Lines(l) => point(l.x * 53.0, l.y * 53.0),
        };
        let p = e.position;
        self.s.wheel(
            f32::from(p.x),
            f32::from(p.y),
            mods(&e.modifiers),
            d.x as i32,
            d.y as i32,
        );
    }
}

fn mods(m: &Modifiers) -> u32 {
    (m.shift as u32) << 1 | (m.control as u32) << 2 | (m.alt as u32) << 3 | (m.platform as u32) << 7
}

/// gpui key name → Windows virtual-key code, which CEF's key events are keyed on.
fn vk(key: &str) -> i32 {
    match key {
        "enter" => 0x0D,
        "backspace" => 0x08,
        "tab" => 0x09,
        "escape" => 0x1B,
        "space" => 0x20,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "end" => 0x23,
        "home" => 0x24,
        "left" => 0x25,
        "up" => 0x26,
        "right" => 0x27,
        "down" => 0x28,
        "delete" => 0x2E,
        k if k.len() == 1 && k.as_bytes()[0].is_ascii_alphanumeric() => {
            k.as_bytes()[0].to_ascii_uppercase() as i32
        }
        _ => 0,
    }
}

fn send_key(s: &Shared, k: &Keystroke, down: bool) {
    let m = mods(&k.modifiers);
    let code = vk(&k.key);
    if !down {
        s.key(cef::KeyEventType::KEYUP, code, 0, m);
        return;
    }
    s.key(cef::KeyEventType::RAWKEYDOWN, code, 0, m);
    let ch: Option<u16> = match k.key.as_str() {
        "enter" => Some(13),
        "backspace" | "tab" | "escape" => None,
        _ => k.key_char.as_deref().and_then(|c| c.encode_utf16().next()),
    };
    if let Some(ch) = ch.filter(|_| !k.modifiers.control) {
        s.key(cef::KeyEventType::CHAR, code, ch, m);
    }
}

fn main() {
    // Subprocesses (renderer, GPU, utility, zygote) re-enter here and exit.
    let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    let args = cef::args::Args::new();
    let mut app = web::app();
    let code = cef::execute_process(
        Some(args.as_main_args()),
        Some(&mut app),
        std::ptr::null_mut(),
    );
    if code >= 0 {
        std::process::exit(code);
    }
    let external_pump = std::env::var("SPIKE_PUMP").is_ok_and(|v| v == "external");
    let t0 = Instant::now();
    assert!(
        web::init(&args, &mut app, external_pump),
        "cef::initialize failed"
    );
    eprintln!(
        "[spike] cef initialized in {:?} (external_pump={external_pump}) DISPLAY={:?}",
        t0.elapsed(),
        std::env::var("DISPLAY").ok()
    );

    gpui::Application::new().run(move |cx: &mut App| {
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<Arc<RenderImage>>();
        let s = Shared::new(tx);
        let bounds = Bounds::centered(None, size(px(1000.), px(750.)), cx);
        let handle: WindowHandle<Spike> = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus);
                    cx.new(|_| Spike {
                        s: s.clone(),
                        frame: None,
                        stale: Vec::new(),
                        focus,
                        view: (0, 0, 0.0),
                        frames: 0,
                    })
                },
            )
            .unwrap();
        let entity = handle.update(cx, |_, _, cx| cx.entity()).unwrap();
        assert!(web::create_browser(&s, START_URL), "create_browser failed");

        if external_pump {
            cx.spawn(async move |cx: &mut AsyncApp| {
                loop {
                    cef::do_message_loop_work();
                    cx.background_executor()
                        .timer(Duration::from_millis(5))
                        .await;
                }
            })
            .detach();
        }

        // CEF thread → gpui: keep only the newest frame, queue the replaced one for drop_image.
        cx.spawn(async move |cx: &mut AsyncApp| {
            while let Some(mut f) = rx.next().await {
                while let Ok(n) = rx.try_recv() {
                    f = n;
                }
                let _ = entity.update(cx, |this, cx| {
                    if let Some(old) = this.frame.replace(f) {
                        this.stale.push(old);
                    }
                    this.frames += 1;
                    cx.notify();
                });
            }
        })
        .detach();

        let s2 = s.clone();
        handle
            .update(cx, |_, window, cx| {
                window.on_window_should_close(cx, move |_, cx| {
                    s2.close();
                    cx.quit();
                    true
                })
            })
            .ok();

        if let Ok(out) = std::env::var("SPIKE_SCRIPT") {
            cx.spawn(async move |cx: &mut AsyncApp| {
                script(cx, handle, s, out).await;
            })
            .detach();
        }
    });
    eprintln!("[spike] gpui loop ended, shutting down CEF");
    cef::shutdown();
}

// ---------- scripted check ----------

async fn sleep(cx: &AsyncApp, ms: u64) {
    cx.background_executor()
        .timer(Duration::from_millis(ms))
        .await;
}

async fn wait_for(cx: &AsyncApp, secs: u64, mut f: impl FnMut() -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        if f() {
            return true;
        }
        sleep(cx, 50).await;
    }
    false
}

/// Evaluate a JS expression by round-tripping it through `document.title`.
async fn query(cx: &AsyncApp, s: &Shared, n: &mut u32, expr: &str) -> Option<String> {
    *n += 1;
    let tag = format!("Q{n}:");
    s.exec_js(&format!("document.title='{tag}'+String({expr})"));
    let mut out = None;
    wait_for(cx, 5, || {
        let t = s.title.lock().unwrap().clone();
        out = t.strip_prefix(&tag).map(String::from);
        out.is_some()
    })
    .await;
    out
}

fn dispatch(cx: &mut AsyncApp, h: WindowHandle<Spike>, ev: PlatformInput) {
    let _ = h.update(cx, |this, window, _| match &ev {
        PlatformInput::MouseDown(e) => this.mouse_down(e, window),
        PlatformInput::MouseUp(e) => this.mouse_up(e),
        PlatformInput::MouseMove(e) => this.mouse_move(e),
        PlatformInput::ScrollWheel(e) => this.scroll(e),
        PlatformInput::KeyDown(e) => send_key(&this.s, &e.keystroke, true),
        PlatformInput::KeyUp(e) => send_key(&this.s, &e.keystroke, false),
        _ => {}
    });
}

fn click(cx: &mut AsyncApp, h: WindowHandle<Spike>, p: Point<gpui::Pixels>) {
    let m = Modifiers::default();
    dispatch(
        cx,
        h,
        PlatformInput::MouseMove(MouseMoveEvent {
            position: p,
            pressed_button: None,
            modifiers: m,
        }),
    );
    dispatch(
        cx,
        h,
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position: p,
            modifiers: m,
            click_count: 1,
            first_mouse: false,
        }),
    );
    dispatch(
        cx,
        h,
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: p,
            modifiers: m,
            click_count: 1,
        }),
    );
}

fn type_key(cx: &mut AsyncApp, h: WindowHandle<Spike>, key: &str) {
    let ks = Keystroke::parse(key).unwrap();
    dispatch(
        cx,
        h,
        PlatformInput::KeyDown(KeyDownEvent {
            keystroke: ks.clone(),
            is_held: false,
        }),
    );
    dispatch(cx, h, PlatformInput::KeyUp(KeyUpEvent { keystroke: ks }));
}

fn save_frame(cx: &mut AsyncApp, h: WindowHandle<Spike>, path: &str) {
    let _ = h.update(cx, |this, _, _| {
        if let Some(f) = &this.frame {
            let sz = f.size(0);
            let mut px = f.as_bytes(0).unwrap().to_vec();
            px.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
            image::save_buffer(
                path,
                &px,
                sz.width.0 as u32,
                sz.height.0 as u32,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    });
}

fn screenshot(path: &str) {
    // Active window only; the window has focus right after it opens.
    let ok = std::process::Command::new("spectacle")
        .args(["-b", "-n", "-a", "-o", path])
        .status();
    eprintln!("[spike] spectacle {path}: {ok:?}");
}

/// Processes in our tree: (pid, cpu ticks, rss kB, role from --type=).
fn proc_tree() -> Vec<(u32, u64, u64, String)> {
    let me = std::process::id();
    let mut all = Vec::new();
    for e in std::fs::read_dir("/proc").unwrap().flatten() {
        let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let rest = &stat[stat.rfind(')').unwrap() + 2..];
        let f: Vec<&str> = rest.split(' ').collect();
        let ppid: u32 = f[1].parse().unwrap();
        let ticks: u64 = f[11].parse::<u64>().unwrap() + f[12].parse::<u64>().unwrap();
        let rss = f[21].parse::<u64>().unwrap() * 4;
        let cmd = std::fs::read_to_string(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        let role = cmd
            .split('\0')
            .find_map(|a| a.strip_prefix("--type="))
            .unwrap_or("browser")
            .to_string();
        all.push((pid, ppid, ticks, rss, role));
    }
    let mut tree = vec![me];
    let mut i = 0;
    while i < tree.len() {
        let p = tree[i];
        tree.extend(all.iter().filter(|a| a.1 == p).map(|a| a.0));
        i += 1;
    }
    all.into_iter()
        .filter(|a| tree.contains(&a.0))
        .map(|a| (a.0, a.2, a.3, a.4))
        .collect()
}

fn ticks_by_role() -> std::collections::BTreeMap<String, u64> {
    let mut m = std::collections::BTreeMap::new();
    for (_, t, _, role) in proc_tree() {
        *m.entry(role).or_insert(0) += t;
    }
    m
}

fn self_rss_kb() -> u64 {
    proc_tree()
        .iter()
        .find(|p| p.0 == std::process::id())
        .map(|p| p.2)
        .unwrap_or(0)
}

fn frames(cx: &mut AsyncApp, h: WindowHandle<Spike>) -> u64 {
    h.update(cx, |this, _, _| this.frames).unwrap_or(0)
}

async fn script(cx: &mut AsyncApp, h: WindowHandle<Spike>, s: Arc<Shared>, out: String) {
    let mut q = 0;
    let log = |m: String| eprintln!("[spike] {m}");

    // 1. Processes and first page.
    let loaded = wait_for(cx, 30, || {
        s.has_browser() && !s.loading.load(Ordering::SeqCst) && s.paints.load(Ordering::SeqCst) > 0
    })
    .await;
    sleep(cx, 1500).await;
    log(format!(
        "example.org loaded={loaded} paints={} title={:?}",
        s.paints.load(Ordering::SeqCst),
        s.title.lock().unwrap()
    ));
    for (pid, _, rss, role) in proc_tree() {
        let exe = std::fs::read_link(format!("/proc/{pid}/exe"))
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        log(format!("proc pid={pid} role={role} rss={rss}kB exe={exe}"));
    }
    let view = *s.view.lock().unwrap();
    log(format!(
        "view(logical w,h,scale)={view:?} last paint (px)={:?}",
        s.last_paint_size.lock().unwrap()
    ));
    save_frame(cx, h, &format!("{out}/1-example-frame.png"));
    screenshot(&format!("{out}/1-example-window.png"));

    // 2. Link click navigates (gpui MouseDown/Up → send_mouse_click_event).
    let pos = query(cx, &s, &mut q, "(()=>{const r=document.querySelector('a').getBoundingClientRect();return (r.x+r.width/2)+','+(r.y+r.height/2)})()").await;
    log(format!("link centre {pos:?}"));
    if let Some((x, y)) = pos.as_deref().and_then(|p| p.split_once(',')) {
        let before = s.url.lock().unwrap().clone();
        click(cx, h, point(px(x.parse().unwrap()), px(y.parse().unwrap())));
        let nav = wait_for(cx, 15, || *s.url.lock().unwrap() != before).await;
        log(format!(
            "CLICK navigated={nav} url {before} -> {}",
            s.url.lock().unwrap()
        ));
        wait_for(cx, 15, || !s.loading.load(Ordering::SeqCst)).await;
        sleep(cx, 1000).await;
        save_frame(cx, h, &format!("{out}/2-after-click-frame.png"));
    }

    // 3. Resize follows (logical size × scale = painted size).
    let _ = h.update(cx, |_, w, _| w.resize(size(px(760.), px(560.))));
    sleep(cx, 1500).await;
    log(format!(
        "RESIZE view={:?} last paint={:?}",
        s.view.lock().unwrap(),
        s.last_paint_size.lock().unwrap()
    ));
    save_frame(cx, h, &format!("{out}/3-resized-frame.png"));
    let _ = h.update(cx, |_, w, _| w.resize(size(px(1000.), px(750.))));
    sleep(cx, 1500).await;
    log(format!(
        "RESIZE back view={:?} last paint={:?}",
        s.view.lock().unwrap(),
        s.last_paint_size.lock().unwrap()
    ));
    // Scale change at runtime (as when the window moves to a 2x monitor).
    let orig = *s.view.lock().unwrap();
    s.view.lock().unwrap().2 = 2.0;
    s.resized(true);
    sleep(cx, 1500).await;
    let dpr = query(
        cx,
        &s,
        &mut q,
        "window.devicePixelRatio+' '+innerWidth+'x'+innerHeight",
    )
    .await;
    log(format!(
        "SCALE 2 view={:?} last paint={:?} page dpr/inner={dpr:?}",
        s.view.lock().unwrap(),
        s.last_paint_size.lock().unwrap()
    ));
    save_frame(cx, h, &format!("{out}/3-scale2-frame.png"));
    *s.view.lock().unwrap() = orig;
    s.resized(true);
    sleep(cx, 1000).await;

    // 4. Rich page, typing into a text field.
    s.load(WIKI_URL);
    wait_for(cx, 5, || s.loading.load(Ordering::SeqCst)).await;
    let ok = wait_for(cx, 30, || !s.loading.load(Ordering::SeqCst)).await;
    sleep(cx, 2000).await;
    log(format!(
        "wikipedia loaded={ok} title={:?}",
        s.title.lock().unwrap()
    ));
    save_frame(cx, h, &format!("{out}/4-wiki-frame.png"));
    screenshot(&format!("{out}/4-wiki-window.png"));
    // A real <input>, placed fixed at the top so its position is known.
    s.exec_js("const i=document.createElement('input');i.id='spk';i.style='position:fixed;top:10px;left:10px;width:300px;height:30px;z-index:99999;font-size:18px';document.body.appendChild(i)");
    sleep(cx, 500).await;
    click(cx, h, point(px(100.), px(25.)));
    sleep(cx, 300).await;
    // `key->char` is how gpui test keystrokes carry key_char, as the Wayland backend fills it.
    for k in [
        "h->h",
        "e->e",
        "l->l",
        "l->l",
        "o->o",
        "space-> ",
        "c->c",
        "e->e",
        "f->f",
        "shift-x->X",
        "backspace",
        "shift-1->!",
    ] {
        type_key(cx, h, k);
        sleep(cx, 30).await;
    }
    sleep(cx, 300).await;
    let typed = query(cx, &s, &mut q, "document.getElementById('spk').value").await;
    let focused = query(cx, &s, &mut q, "document.activeElement.id").await;
    log(format!(
        "TYPE value={typed:?} (expected \"hello cef!\") activeElement={focused:?}"
    ));
    save_frame(cx, h, &format!("{out}/5-typed-frame.png"));
    s.exec_js("document.getElementById('spk').remove()");

    // 5. Wheel scroll: does it scroll, frames/s, CPU, RSS over 1000+ frames.
    let centre = point(px(500.), px(400.));
    let y0 = query(cx, &s, &mut q, "window.scrollY").await;
    for _ in 0..5 {
        dispatch(
            cx,
            h,
            PlatformInput::ScrollWheel(ScrollWheelEvent {
                position: centre,
                delta: ScrollDelta::Lines(point(0., -3.)),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            }),
        );
        sleep(cx, 50).await;
    }
    sleep(cx, 500).await;
    let y1 = query(cx, &s, &mut q, "window.scrollY").await;
    log(format!(
        "WHEEL scrollY {y0:?} -> {y1:?} after 5 notches down"
    ));
    save_frame(cx, h, &format!("{out}/6-scrolled-frame.png"));

    let rss0 = self_rss_kb();
    let f0 = frames(cx, h);
    let p0 = s.paints.load(Ordering::SeqCst);
    let c0 = ticks_by_role();
    let t0 = Instant::now();
    let mut dir = -1.0;
    let mut i = 0u64;
    while frames(cx, h) - f0 < 1100 && t0.elapsed() < Duration::from_secs(60) {
        // ~60 wheel events/s, reversing every 2 s so the page keeps moving.
        if i % 120 == 0 {
            dir = -dir;
        }
        dispatch(
            cx,
            h,
            PlatformInput::ScrollWheel(ScrollWheelEvent {
                position: centre,
                delta: ScrollDelta::Pixels(point(px(0.), px(40. * dir))),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            }),
        );
        i += 1;
        if i % 120 == 0 {
            log(format!(
                "  rss={}kB frames={} paints={}",
                self_rss_kb(),
                frames(cx, h) - f0,
                s.paints.load(Ordering::SeqCst) - p0
            ));
        }
        sleep(cx, 16).await;
    }
    let dt = t0.elapsed().as_secs_f64();
    let c1 = ticks_by_role();
    let (df, dp) = (frames(cx, h) - f0, s.paints.load(Ordering::SeqCst) - p0);
    log(format!(
        "SCROLL {dt:.1}s: gpui frames={df} ({:.1}/s), cef paints={dp} ({:.1}/s), rss {rss0}kB -> {}kB",
        df as f64 / dt,
        dp as f64 / dt,
        self_rss_kb()
    ));
    let mut total = 0.0;
    for (role, t) in &c1 {
        let pct = (*t - c0.get(role).copied().unwrap_or(0)) as f64 / dt; // 100 ticks/s → %
        total += pct;
        log(format!("  cpu {role}: {pct:.0}%"));
    }
    log(format!("  cpu total: {total:.0}% (100% = one core)"));

    // Idle for comparison.
    let c0 = ticks_by_role();
    sleep(cx, 5000).await;
    let c1 = ticks_by_role();
    let idle: u64 = c1
        .iter()
        .map(|(r, t)| t - c0.get(r).copied().unwrap_or(0))
        .sum();
    log(format!(
        "IDLE cpu total: {:.0}% rss={}kB",
        idle as f64 / 5.0,
        self_rss_kb()
    ));

    log("done, closing".into());
    s.close();
    wait_for(cx, 5, || s.closed.load(Ordering::SeqCst)).await;
    let _ = cx.update(|cx| cx.quit());
}
