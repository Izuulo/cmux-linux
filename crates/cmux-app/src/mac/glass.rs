//! The surfaces under popovers, menus and the palette (plan K4).

use gpui::{Div, ParentElement, Rgba, Styled, div};

use super::{Theme, metric, popover_shadow};
use crate::ui::px;

/// A popover or menu body: `fill`, a 1 px outer dark line and a 1 px inner light line.
pub fn glass_panel(t: &Theme, radius: f32, fill: Rgba) -> Div {
    div()
        .relative()
        .overflow_hidden()
        .rounded(px(radius))
        .bg(fill)
        .border_1()
        .border_color(t.edge_outer)
        .shadow(popover_shadow())
        // First child, so the content draws above the line.
        .child(
            div()
                .absolute()
                .inset_0()
                .rounded(px(radius - 1.0))
                .border_1()
                .border_color(t.edge_inner),
        )
}

/// The command palette's box: the terminal background with one light border, no shadow.
pub fn palette_panel(t: &Theme) -> Div {
    div()
        .overflow_hidden()
        .rounded(px(metric::PALETTE_RADIUS))
        .bg(t.palette)
        .border_1()
        .border_color(t.palette_border)
}
