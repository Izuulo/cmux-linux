//! Client-side window decorations: our own close/minimize/zoom buttons, window move and
//! edge resize, so the compositor draws no title bar or border around the window.
//! The buttons sit top-right (owner's choice; cmux has them top-left) and the top-right strip
//! indents for them. The window's corners are rounded by whatever is drawn in each corner.

use std::cell::Cell;

use crate::ui::px;
use gpui::{
    AnyElement, CursorStyle, Decorations, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, Point, ResizeEdge, StatefulInteractiveElement, Styled, Window, div, rgb,
};

const DOT: f32 = 12.0;
const DOT_GAP: f32 = 8.0;
const DOT_INSET: f32 = 10.0;
/// Top of the buttons: centred in the 28 px band.
const DOT_TOP: f32 = 8.0;
/// A press must travel this far before it moves the window, so clicks stay clicks.
const MOVE_SLOP: f32 = 4.0;
const EDGE: f32 = 5.0;
const CORNER: f32 = 12.0;
const RADIUS: f32 = 10.0;

thread_local! {
    /// Where the left button went down on a draggable strip, until released or moved away.
    static PRESSED: Cell<Option<Point<Pixels>>> = const { Cell::new(None) };
}

fn client_decorated(window: &Window) -> bool {
    matches!(window.window_decorations(), Decorations::Client { .. })
}

/// Width the window buttons take from the top-right corner (0 when the compositor decorates).
pub fn controls_width(window: &Window) -> f32 {
    match client_decorated(window) && !window.is_fullscreen() {
        true => crate::ui::s(2.0 * DOT_INSET + 3.0 * DOT + 2.0 * DOT_GAP),
        false => 0.0,
    }
}

/// How far a top strip must indent on one side to clear `reserved` px in that window corner,
/// when the strip's end is `edge_gap` px from that window edge.
pub fn inset(reserved: f32, edge_gap: f32, strip_y: f32) -> f32 {
    match strip_y == 0.0 {
        true => (reserved - edge_gap).max(0.0),
        false => 0.0,
    }
}

/// Radius of the window's own corners: none when the compositor decorates, or when the window
/// is maximized, fullscreen or tiled.
pub fn corner_radius(window: &Window) -> f32 {
    let Decorations::Client { tiling } = window.window_decorations() else {
        return 0.0;
    };
    let tiled = tiling.top || tiling.bottom || tiling.left || tiling.right;
    match tiled || window.is_maximized() || window.is_fullscreen() {
        true => 0.0,
        false => RADIUS,
    }
}

/// What the top-left strip clears while the sidebar is hidden: the reopen button that
/// `sidebar.rs` draws there (its 20 px button with the band's 6 px inset either side).
pub const REOPEN_LEAD: f32 = 32.0;

/// Makes empty top-bar space move the window when dragged; `zoom` adds double-click to
/// maximize. Does nothing under server-side decorations.
pub fn window_drag<E: InteractiveElement>(el: E, zoom: bool) -> E {
    el.on_mouse_down(MouseButton::Left, move |ev, window, _| {
        if !client_decorated(window) {
            return;
        }
        if zoom && ev.click_count == 2 {
            window.zoom_window();
        } else {
            PRESSED.set(Some(ev.position));
        }
    })
    .on_mouse_move(|ev, window, _| {
        let Some(from) = PRESSED.get().filter(|_| ev.dragging()) else {
            PRESSED.set(None);
            return;
        };
        let d = ev.position - from;
        if f32::from(d.x).abs().max(f32::from(d.y).abs()) > MOVE_SLOP {
            PRESSED.set(None);
            window.start_window_move();
        }
    })
    .on_mouse_up(MouseButton::Left, |_, _, _| PRESSED.set(None))
    .on_mouse_up_out(MouseButton::Left, |_, _, _| PRESSED.set(None))
}

/// The close, minimize and zoom buttons, when we decorate the window ourselves.
pub fn controls(window: &Window) -> Option<AnyElement> {
    if controls_width(window) == 0.0 {
        return None;
    }
    let dot = |id: &'static str, color: u32, act: fn(&mut Window)| {
        div()
            .id(id)
            .size(px(DOT))
            .rounded_full()
            .bg(rgb(color))
            .hover(|s| s.opacity(0.75))
            // Keep the press from arming a window move on the band underneath.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |_, window, _| act(window))
    };
    Some(
        div()
            .absolute()
            .top(px(DOT_TOP))
            .right(px(DOT_INSET))
            .flex()
            .flex_row()
            .gap(px(DOT_GAP))
            // Close sits in the corner, as on other Linux windows.
            .child(dot("window-minimize", 0xfebc2e, |w| w.minimize_window()))
            .child(dot("window-zoom", 0x28c840, |w| w.zoom_window()))
            .child(dot("window-close", 0xff5f57, |w| w.remove_window()))
            .into_any_element(),
    )
}

/// Invisible grips on the window's edges and corners (the compositor gives a self-decorated
/// window none). Left out when maximized, fullscreen, or on a tiled side.
pub fn resize_grips(window: &Window) -> Vec<AnyElement> {
    let Decorations::Client { tiling } = window.window_decorations() else {
        return Vec::new();
    };
    if window.is_maximized() || window.is_fullscreen() {
        return Vec::new();
    }
    use CursorStyle::{
        ResizeLeftRight as H, ResizeUpDown as V, ResizeUpLeftDownRight as Nwse,
        ResizeUpRightDownLeft as Nesw,
    };
    use ResizeEdge::*;
    let (t, b, l, r) = (!tiling.top, !tiling.bottom, !tiling.left, !tiling.right);
    // (edge, shown, cursor, anchored to top, to left, width, height); None = the full side.
    let grips = [
        (Top, t, V, true, true, None, Some(EDGE)),
        (Bottom, b, V, false, true, None, Some(EDGE)),
        (Left, l, H, true, true, Some(EDGE), None),
        (Right, r, H, true, false, Some(EDGE), None),
        // Corners last, so they sit on top of the sides.
        (
            TopLeft,
            t && l,
            Nwse,
            true,
            true,
            Some(CORNER),
            Some(CORNER),
        ),
        (
            TopRight,
            t && r,
            Nesw,
            true,
            false,
            Some(CORNER),
            Some(CORNER),
        ),
        (
            BottomLeft,
            b && l,
            Nesw,
            false,
            true,
            Some(CORNER),
            Some(CORNER),
        ),
        (
            BottomRight,
            b && r,
            Nwse,
            false,
            false,
            Some(CORNER),
            Some(CORNER),
        ),
    ];
    grips
        .into_iter()
        .filter(|g| g.1)
        .map(|(edge, _, cursor, top, left, w, h)| {
            let d = div().absolute().occlude().cursor(cursor);
            let d = if top { d.top_0() } else { d.bottom_0() };
            let d = if left { d.left_0() } else { d.right_0() };
            let d = match w {
                Some(w) => d.w(px(w)),
                None => d.w_full(),
            };
            let d = match h {
                Some(h) => d.h(px(h)),
                None => d.h_full(),
            };
            d.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                cx.stop_propagation();
                window.start_window_resize(edge);
            })
            .into_any_element()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::inset as lead;

    #[test]
    fn only_a_top_corner_strip_indents() {
        assert_eq!(lead(72.0, 0.0, 0.0), 72.0);
        assert_eq!(lead(72.0, 50.0, 0.0), 22.0);
        assert_eq!(lead(72.0, 240.0, 0.0), 0.0, "past the sidebar");
        assert_eq!(lead(72.0, 0.0, 300.0), 0.0, "a lower pane");
        assert_eq!(lead(0.0, 0.0, 0.0), 0.0, "compositor decorates");
    }
}
