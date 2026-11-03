//! KWin blur behind the window (plan D4, rg §1.4). gpui 0.2.2 only binds `org_kde_kwin_blur`,
//! but KWin 6.8 offers `ext_background_effect_v1`, so we bind it ourselves on gpui's own
//! `wl_display`, with our own event queue, and blur the whole surface.
// shortcut: Plasma <= 6.6 offers only org_kde_kwin_blur and gets no blur; add a kde-blur path if needed.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, delegate_noop};
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1;
use wayland_protocols::ext::background_effect::v1::client::ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1;

/// Keeps the blur on while alive; dropping it removes the blur on the next commit.
pub struct WindowBlur {
    conn: Connection,
    _queue: EventQueue<State>,
    effect: ExtBackgroundEffectSurfaceV1,
}

/// Our queue is never dispatched after setup: nothing here needs events.
struct State;

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
delegate_noop!(State: WlCompositor);
delegate_noop!(State: WlRegion);
delegate_noop!(State: ExtBackgroundEffectSurfaceV1);
delegate_noop!(State: ignore ExtBackgroundEffectManagerV1);

fn log<E: std::fmt::Display>(what: &'static str) -> impl FnOnce(&E) {
    move |e| eprintln!("cmux-linux: blur: {what}: {e}")
}

impl WindowBlur {
    /// Binds ext_background_effect_manager_v1 on gpui's own display and blurs the whole surface.
    /// None on X11, without the protocol, or if any step fails (logged once to stderr).
    pub fn enable(window: &gpui::Window, cx: &gpui::App) -> Option<Self> {
        // gpui 0.2.2's X11 window panics (`unimplemented!`) on raw handles, so ask first.
        if cx.compositor_name() != "Wayland" {
            return None;
        }
        let display = window
            .display_handle()
            .inspect_err(log("display handle"))
            .ok()?;
        // Trait call: gpui's inherent `Window::window_handle` returns its own handle type.
        let handle = HasWindowHandle::window_handle(window)
            .inspect_err(log("window handle"))
            .ok()?;
        let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(handle)) =
            (display.as_raw(), handle.as_raw())
        else {
            return None;
        };
        // SAFETY: the pointer is gpui's live wl_display, which gpui keeps open for the whole
        // app; our Backend is a guest that never closes it, and WindowBlur is dropped with
        // WindowView, before the window.
        let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
        let conn = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<State>(&conn)
            .inspect_err(log("registry"))
            .ok()?;
        let qh = queue.handle();
        let manager: ExtBackgroundEffectManagerV1 = globals
            .bind(&qh, 1..=1, ())
            .inspect_err(log("ext_background_effect_manager_v1"))
            .ok()?;
        let compositor: WlCompositor = globals
            .bind(&qh, 1..=1, ())
            .inspect_err(log("wl_compositor"))
            .ok()?;
        // SAFETY: the pointer is the wl_surface proxy of gpui's live window (borrowed from
        // `window` for this call), and from_ptr checks its interface; the id is only used
        // inside this function.
        let id =
            unsafe { ObjectId::from_ptr(WlSurface::interface(), handle.surface.as_ptr().cast()) }
                .inspect_err(log("surface id"))
                .ok()?;
        let surface = WlSurface::from_id(&conn, id)
            .inspect_err(log("surface"))
            .ok()?;
        let effect = manager.get_background_effect(&surface, &qh, ());
        manager.destroy(); // The effect outlives its manager (protocol).
        // Whole surface, set once: the compositor clips the region to the surface, and it
        // applies on gpui's next wl_surface.commit.
        let region = compositor.create_region(&qh, ());
        region.add(0, 0, i32::MAX / 2, i32::MAX / 2);
        effect.set_blur_region(Some(&region));
        region.destroy();
        conn.flush().inspect_err(log("flush")).ok()?;
        Some(Self {
            conn,
            _queue: queue,
            effect,
        })
    }
}

impl Drop for WindowBlur {
    /// Cleans up before the guest Backend goes; inert (but valid) if the surface is already gone.
    fn drop(&mut self) {
        self.effect.destroy();
        let _ = self.conn.flush();
    }
}
