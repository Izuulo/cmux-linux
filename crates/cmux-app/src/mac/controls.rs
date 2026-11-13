//! The Settings window's building blocks (M12 plan S2, S8; M14 K8): cards, rows and the small
//! controls (toggle, stepper, slider, segmented picker, dropdown button, text field) and the
//! sidebar row, in the Tahoe look. Each is a plain element; the caller owns the state and
//! passes click handlers.

use std::cell::Cell;
use std::rc::Rc;

use super::{Theme, alpha, glass_panel, menu_item, metric};
use crate::ui::px;
use gpui::{
    AnyElement, App, Bounds, ClickEvent, ElementId, FontWeight, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Rgba, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*, svg,
};

/// Width of a row's control column.
pub const CONTROL_W: f32 = 200.0;
const ROW_PAD_X: f32 = 14.0;
const ROW_PAD_Y: f32 = 9.0;
const SIDEBAR_ROW_H: f32 = 28.0;
const SIDEBAR_TILE: f32 = 20.0;
const SIDEBAR_GLYPH: f32 = 12.0;
const SLIDER_W: f32 = 140.0;
const SLIDER_LABEL_W: f32 = 44.0;
const KNOB: f32 = 14.0;

pub type Handler = Rc<dyn Fn(&mut Window, &mut App)>;
/// Gets the index of the clicked choice.
pub type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A section heading above a card.
pub fn header(title: &str, c: Theme) -> impl IntoElement {
    div()
        .pl(px(2.))
        .text_size(px(metric::BODY_TEXT))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c.secondary)
        .child(SharedString::from(title.to_string()))
}

/// Rows stacked in a rounded card with a separator line between them.
pub fn card(rows: Vec<AnyElement>, c: Theme) -> impl IntoElement {
    let n = rows.len();
    let mut out = Vec::with_capacity(n * 2);
    for (k, row) in rows.into_iter().enumerate() {
        out.push(row);
        if k + 1 < n {
            out.push(
                div()
                    .h(px(1.))
                    .mx(px(ROW_PAD_X))
                    .bg(c.separator)
                    .into_any_element(),
            );
        }
    }
    div()
        .flex()
        .flex_col()
        .rounded(px(metric::CARD_RADIUS))
        .bg(c.card)
        .border_1()
        .border_color(c.separator)
        .children(out)
}

/// A card's caption line (sui `SettingsCardNote`).
pub fn note(text: impl Into<SharedString>, color: Rgba) -> AnyElement {
    div()
        .px(px(ROW_PAD_X))
        .py(px(8.))
        .text_size(px(metric::SMALL_TEXT))
        .text_color(color)
        .child(text.into())
        .into_any_element()
}

/// Label left (title, optional subtitle), control right; `note` goes under the row.
pub fn row(
    title: &str,
    subtitle: Option<&str>,
    control: AnyElement,
    note_line: Option<(String, Rgba)>,
    c: Theme,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(12.))
                .px(px(ROW_PAD_X))
                .py(px(ROW_PAD_Y))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w(px(0.))
                        .gap(px(3.))
                        .child(
                            div()
                                .text_size(px(metric::BODY_TEXT))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(c.text)
                                .child(SharedString::from(title.to_string())),
                        )
                        .children(subtitle.map(|s| {
                            div()
                                .text_size(px(metric::SMALL_TEXT))
                                .text_color(c.secondary)
                                .child(SharedString::from(s.to_string()))
                        })),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_end()
                        .flex_none()
                        .w(px(CONTROL_W))
                        .child(control),
                ),
        )
        .children(note_line.map(|(t, color)| div().mt(px(-6.)).child(note(t, color))))
        .into_any_element()
}

fn button_base(id: impl Into<ElementId>, c: Theme) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .h(px(22.))
        .rounded(px(metric::CONTROL_RADIUS))
        .bg(c.control)
        .text_size(px(metric::BODY_TEXT))
        .text_color(c.text)
        .hover(move |s| s.bg(alpha(c.control, c.control.a + 0.08)))
}

/// An on/off switch.
pub fn toggle(id: &'static str, on: bool, c: Theme, click: Handler) -> AnyElement {
    let (w, h) = (metric::SWITCH_W, metric::SWITCH_H);
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .when(on, |d| d.justify_end())
        .w(px(w))
        .h(px(h))
        .p(px(2.))
        .rounded_full()
        .bg(if on { c.accent } else { c.control })
        .on_click(move |_: &ClickEvent, window, cx| click(window, cx))
        .child(div().size(px(h - 4.)).rounded_full().bg(c.menu_hover_text))
        .into_any_element()
}

/// `−  value  +`; each button calls its handler.
pub fn stepper(
    id: &'static str,
    value: String,
    c: Theme,
    down: Handler,
    up: Handler,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(metric::BODY_TEXT))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c.text)
                .child(value),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(1.))
                .child(
                    button_base((id, 0usize), c)
                        .w(px(22.))
                        .rounded_r(px(0.))
                        .child("−")
                        .on_click(move |_: &ClickEvent, window, cx| down(window, cx)),
                )
                .child(
                    button_base((id, 1usize), c)
                        .w(px(22.))
                        .rounded_l(px(0.))
                        .child("+")
                        .on_click(move |_: &ClickEvent, window, cx| up(window, cx)),
                ),
        )
        .into_any_element()
}

/// One choice of several, side by side; `pick` gets the clicked index.
pub fn segmented(
    id: &'static str,
    options: &[&str],
    selected: usize,
    c: Theme,
    pick: Pick,
) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .p(px(2.))
        .gap(px(2.))
        .rounded(px(metric::CONTROL_RADIUS))
        .bg(c.control)
        .children(options.iter().enumerate().map(|(k, label)| {
            let pick = pick.clone();
            let on = k == selected;
            div()
                .id((id, k))
                .px(px(7.))
                .py(px(2.))
                .rounded(px(metric::CONTROL_RADIUS - 2.))
                .text_size(px(metric::SMALL_TEXT))
                .text_color(if on { c.menu_hover_text } else { c.text })
                .when(on, |d| d.bg(c.accent))
                .when(!on, |d| d.hover(move |s| s.bg(c.hover)))
                .child(SharedString::from(label.to_string()))
                .on_click(move |_: &ClickEvent, window, cx| pick(k, window, cx))
        }))
        .into_any_element()
}

/// Where along a track `x` falls, 0..=1.
pub fn fraction(x: f32, left: f32, width: f32) -> f32 {
    if width <= 0.0 {
        return 0.0;
    }
    ((x - left) / width).clamp(0.0, 1.0)
}

/// A horizontal slider at `frac` (0..=1) with a percent label. `track` receives the track's
/// bounds at paint, for the caller's drag maths; `down` starts the drag.
pub fn slider(
    id: &'static str,
    frac: f32,
    label: String,
    track: Rc<Cell<Bounds<Pixels>>>,
    c: Theme,
    down: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let frac = frac.clamp(0.0, 1.0);
    let w = crate::ui::s(SLIDER_W - KNOB);
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .child(
            div()
                .id(id)
                .relative()
                .w(px(SLIDER_W))
                .h(px(KNOB))
                .on_mouse_down(MouseButton::Left, down)
                .child(
                    // Inset by half a knob so the knob's centre spans the track.
                    canvas(
                        move |b, _, _| {
                            let half = gpui::px(crate::ui::s(KNOB / 2.));
                            track.set(Bounds {
                                origin: gpui::point(b.origin.x + half, b.origin.y),
                                size: gpui::size(b.size.width - half * 2., b.size.height),
                            });
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(KNOB / 2.))
                        .right(px(KNOB / 2.))
                        .top(px(KNOB / 2. - 2.))
                        .h(px(4.))
                        .rounded_full()
                        .bg(c.control)
                        .child(
                            div()
                                .h_full()
                                .w(gpui::px(w * frac))
                                .rounded_full()
                                .bg(c.accent),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(0.))
                        .left(gpui::px(w * frac))
                        .size(px(KNOB))
                        .rounded_full()
                        .bg(c.menu_hover_text)
                        .border_1()
                        .border_color(c.separator),
                ),
        )
        .child(
            div()
                .w(px(SLIDER_LABEL_W))
                .flex()
                .justify_end()
                .text_size(px(metric::BODY_TEXT))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c.text)
                .child(label),
        )
        .into_any_element()
}

/// A button showing the current choice; the caller draws the list (`dropdown_list`) when open.
pub fn dropdown_button(id: &'static str, label: String, c: Theme, click: Handler) -> AnyElement {
    button_base(id, c)
        .w(px(CONTROL_W))
        .justify_between()
        .px(px(8.))
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(label),
        )
        .child(div().text_color(c.secondary).child("▾"))
        .on_click(move |_: &ClickEvent, window, cx| click(window, cx))
        .into_any_element()
}

/// The open dropdown's choices, under its button; `pick` gets the clicked index, `close` runs
/// on a click outside.
pub fn dropdown_list(
    id: &'static str,
    items: &[String],
    selected: Option<usize>,
    c: Theme,
    pick: Pick,
    close: Handler,
) -> AnyElement {
    gpui::deferred(
        gpui::anchored().snap_to_window_with_margin(px(4.)).child(
            glass_panel(&c, metric::MENU_RADIUS)
                .id(id)
                .occlude()
                .mt(px(24.))
                .w(px(CONTROL_W))
                .max_h(px(320.))
                .overflow_y_scroll()
                .py(px(metric::MENU_INSET - 1.))
                .on_mouse_down_out(move |_, window, cx| close(window, cx))
                .children(items.iter().enumerate().map(|(k, label)| {
                    let pick = pick.clone();
                    let on = Some(k) == selected;
                    menu_item((id, k), &c, label, None)
                        .when(on, |d| d.bg(c.hover))
                        .on_click(move |_: &ClickEvent, window, cx| pick(k, window, cx))
                })),
        ),
    )
    .with_priority(1)
    .into_any_element()
}

/// A one-line text box. `editing`: the text being typed (drawn with a caret), else `value`
/// (or `placeholder` when empty). `swatch` draws a colour square before the text.
#[allow(clippy::too_many_arguments)]
pub fn text_field(
    id: impl Into<gpui::ElementId>,
    value: &str,
    editing: Option<&str>,
    placeholder: &str,
    swatch: Option<Rgba>,
    width: f32,
    c: Theme,
    click: Handler,
) -> gpui::Stateful<gpui::Div> {
    // shortcut: the caret is a glyph after the text (no cursor movement, no IME), as in the
    // rename field; a real text input needs gpui's InputHandler.
    let (text, color) = match editing {
        Some(t) => (format!("{t}▏"), c.text),
        None if value.is_empty() => (placeholder.to_string(), c.secondary),
        None => (value.to_string(), c.text),
    };
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .w(px(width))
        .h(px(22.))
        // The 3 px focus ring is a thicker border; padding gives the 2 px back so text stays put.
        .px(px(if editing.is_some() { 4. } else { 6. }))
        .rounded(px(metric::CONTROL_RADIUS))
        .bg(c.control)
        .when(editing.is_some(), |d| {
            d.border_3().border_color(alpha(c.accent, 0.5))
        })
        .when(editing.is_none(), |d| {
            d.border_1().border_color(c.separator)
        })
        .text_size(px(metric::BODY_TEXT))
        .text_color(color)
        .overflow_hidden()
        .whitespace_nowrap()
        .children(swatch.map(|s| {
            div()
                .flex_none()
                .size(px(12.))
                .rounded(px(metric::CONTROL_RADIUS - 4.))
                .border_1()
                .border_color(c.separator)
                .bg(s)
        }))
        .child(text)
        .on_click(move |_: &ClickEvent, window, cx| click(window, cx))
}

/// A Settings sidebar entry: coloured icon tile, label; `selected` fills it like a menu hover.
/// The caller adds the click handler.
pub fn sidebar_row(
    id: impl Into<ElementId>,
    t: &Theme,
    icon: &'static str,
    label: &str,
    selected: bool,
) -> Stateful<gpui::Div> {
    let (hover, fg) = (t.hover, if selected { t.menu_hover_text } else { t.text });
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .h(px(SIDEBAR_ROW_H))
        .px(px(6.))
        .rounded(px(metric::MENU_ITEM_RADIUS))
        .text_size(px(metric::BODY_TEXT))
        .text_color(fg)
        .when(selected, |d| d.bg(t.menu_hover))
        .when(!selected, |d| d.hover(move |s| s.bg(hover)))
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .flex_none()
                .size(px(SIDEBAR_TILE))
                .rounded(px(metric::CONTROL_RADIUS))
                // On the accent-filled selected row an accent tile would vanish: a light tint instead.
                .bg(if selected {
                    super::alpha(t.menu_hover_text, 0.25)
                } else {
                    t.accent
                })
                .child(
                    svg()
                        .path(icon)
                        .size(px(SIDEBAR_GLYPH))
                        .text_color(t.menu_hover_text),
                ),
        )
        .child(SharedString::from(label.to_string()))
}

/// A square icon button for a toolbar (browser back/forward/reload); the caller adds the click
/// handler. Disabled is only dimmed: the caller leaves the handler off or ignores the click.
pub fn toolbar_button(
    id: impl Into<ElementId>,
    t: &Theme,
    icon: &'static str,
    enabled: bool,
) -> Stateful<gpui::Div> {
    let hover = t.hover;
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(metric::TOOLBAR_BUTTON))
        .rounded(px(metric::CONTROL_RADIUS))
        .opacity(if enabled { 1.0 } else { 0.4 })
        .when(enabled, |d| d.hover(move |s| s.bg(hover)))
        .child(
            svg()
                .path(icon)
                .size(px(metric::TOOLBAR_GLYPH))
                .text_color(t.text),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_clamps_to_the_track() {
        assert_eq!(fraction(50.0, 0.0, 100.0), 0.5);
        assert_eq!(fraction(-10.0, 0.0, 100.0), 0.0);
        assert_eq!(fraction(500.0, 0.0, 100.0), 1.0);
        assert_eq!(fraction(5.0, 0.0, 0.0), 0.0);
    }
}
