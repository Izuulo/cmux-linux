//! The glass surface under popovers, menus and the palette (plan K4).

use gpui::{Div, Styled, div};

use super::{Theme, popover_shadow};
use crate::ui::px;

pub fn glass_panel(t: &Theme, radius: f32) -> Div {
    div()
        .overflow_hidden()
        .rounded(px(radius))
        .bg(t.glass)
        .border_1()
        .border_color(t.glass_edge)
        .shadow(popover_shadow())
}
