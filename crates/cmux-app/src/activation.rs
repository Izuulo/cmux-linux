//! Raising the window from a desktop-notification click (plan M4 D9). gpui 0.2.2's `activate`
//! requests a fresh xdg-activation token, which KWin refuses for a background app; the
//! notification server hands us one tied to the click, so we activate with that.

use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;

/// Our queue is never dispatched; see blur.rs.
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
delegate_noop!(State: ignore XdgActivationV1);

fn log<E: std::fmt::Display>(what: &'static str) -> impl FnOnce(&E) {
    crate::wayland::log("activation", what)
}

/// Asks the compositor to focus the window with `token`. False on X11, without
/// xdg_activation_v1, or on any error; the caller then falls back to gpui's own request.
pub fn activate(window: &gpui::Window, cx: &gpui::App, token: &str) -> bool {
    let Some((conn, surface)) = crate::wayland::guest_surface(window, cx, "activation") else {
        return false;
    };
    let Some((globals, queue)) = registry_queue_init::<State>(&conn)
        .inspect_err(log("registry"))
        .ok()
    else {
        return false;
    };
    let Some(activation) = globals
        .bind::<XdgActivationV1, _, _>(&queue.handle(), 1..=1, ())
        .inspect_err(log("xdg_activation_v1"))
        .ok()
    else {
        return false;
    };
    activation.activate(token.to_string(), &surface);
    activation.destroy();
    conn.flush().inspect_err(log("flush")).is_ok()
}
