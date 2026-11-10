//! A guest view of gpui's own Wayland connection and window surface, for the small protocol
//! shims gpui 0.2.2 lacks (blur.rs, activation.rs).

use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Proxy};

pub(crate) fn log<E: std::fmt::Display>(tag: &'static str, what: &'static str) -> impl FnOnce(&E) {
    move |e| eprintln!("cmux-linux: {tag}: {what}: {e}")
}

/// gpui's `wl_display` as a guest `Connection`, and the window's `wl_surface` on it. None on
/// X11 or if a handle is unavailable (logged under `tag`).
pub(crate) fn guest_surface(
    window: &gpui::Window,
    cx: &gpui::App,
    tag: &'static str,
) -> Option<(Connection, WlSurface)> {
    // gpui 0.2.2's X11 window panics (`unimplemented!`) on raw handles, so ask first.
    if cx.compositor_name() != "Wayland" {
        return None;
    }
    let display = window
        .display_handle()
        .inspect_err(log(tag, "display handle"))
        .ok()?;
    // Trait call: gpui's inherent `Window::window_handle` returns its own handle type.
    let handle = HasWindowHandle::window_handle(window)
        .inspect_err(log(tag, "window handle"))
        .ok()?;
    let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(handle)) =
        (display.as_raw(), handle.as_raw())
    else {
        return None;
    };
    // SAFETY: the pointer is gpui's live wl_display. Our guest Backend never disconnects it,
    // and it outlives every entity (WindowView is released during App::shutdown while the
    // platform is alive); a surface gone first just leaves our objects inert.
    let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
    let conn = Connection::from_backend(backend);
    // SAFETY: the pointer is the wl_surface proxy of gpui's live window (borrowed from
    // `window` for this call), and from_ptr checks its interface; callers use the surface
    // only while setting their own objects up.
    let id = unsafe { ObjectId::from_ptr(WlSurface::interface(), handle.surface.as_ptr().cast()) }
        .inspect_err(log(tag, "surface id"))
        .ok()?;
    let surface = WlSurface::from_id(&conn, id)
        .inspect_err(log(tag, "surface"))
        .ok()?;
    Some((conn, surface))
}
