//! NSMenu look: list, accent row under the pointer, shortcut right-aligned.

use gpui::{
    AnyElement, Div, ElementId, InteractiveElement, IntoElement, ParentElement, SharedString,
    Stateful, Styled, div,
};

use super::{Theme, glass_panel, metric};
use crate::ui::px;

/// Separator row: 1 px line with 5 px above and below.
pub const SEPARATOR_H: f32 = 11.0;

pub fn menu_height(items: usize, separators: usize) -> f32 {
    items as f32 * metric::MENU_ITEM_H + separators as f32 * SEPARATOR_H + 2.0 * metric::MENU_INSET
}

pub fn menu(t: &Theme, rows: Vec<AnyElement>) -> Div {
    glass_panel(t, metric::MENU_RADIUS, t.menu)
        .w(px(metric::MENU_W))
        .py(px(metric::MENU_INSET))
        .flex()
        .flex_col()
        .children(rows)
}

pub fn menu_item(
    id: impl Into<ElementId>,
    t: &Theme,
    label: &str,
    shortcut: Option<&str>,
) -> Stateful<Div> {
    let (hover, hover_text) = (t.menu_hover, t.menu_hover_text);
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .h(px(metric::MENU_ITEM_H))
        .mx(px(metric::MENU_INSET))
        .px(px(metric::MENU_ITEM_PAD_X - metric::MENU_INSET))
        .rounded(px(metric::MENU_ITEM_RADIUS))
        .text_size(px(metric::BODY_TEXT))
        .text_color(t.text)
        .hover(move |s| s.bg(hover).text_color(hover_text))
        .child(
            div()
                .flex_1()
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(SharedString::from(label.to_string())),
        )
        .children(shortcut.map(|k| {
            div()
                .flex_none()
                .pl(px(12.))
                .opacity(0.55)
                .child(SharedString::from(k.to_string()))
        }))
}

pub fn menu_separator(t: &Theme) -> AnyElement {
    div()
        .flex_none()
        .h(px(1.))
        .my(px((SEPARATOR_H - 1.0) / 2.0))
        .mx(px(metric::MENU_ITEM_PAD_X))
        .bg(t.separator)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_counts_items_separators_and_insets() {
        let h = menu_height(3, 1);
        assert_eq!(
            h,
            3.0 * metric::MENU_ITEM_H + SEPARATOR_H + 2.0 * metric::MENU_INSET
        );
    }
}
