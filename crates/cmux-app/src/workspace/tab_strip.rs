//! A pane's tab strip, sized and coloured like Bonsplit's tab bar as cmux configures it
//! (research-cmux.md §4 "Pane tab strip"; numbers from `TabBarMetrics.swift`, logical px).

use cmux_core::{Orientation, Pane, PaneId, SurfaceId};
use gpui::{
    ClickEvent, Context, DragMoveEvent, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled,
    Window, div, prelude::*, px, svg,
};

use super::WorkspaceView;
use crate::chrome::{self, TabPalette};

/// cmux `sharedChromeBarHeight`.
pub const TAB_BAR_HEIGHT: f32 = 28.0;
const TAB_MIN_W: f32 = 48.0;
const TAB_MAX_W: f32 = 220.0;
const TAB_PAD: f32 = 6.0;
const ICON_SLOT: f32 = 14.0;
/// The terminal glyph is drawn at max(10, 14 - 2.5) (`TabItemView.swift:975`).
const ICON_GLYPH: f32 = 11.5;
const CONTENT_GAP: f32 = 6.0;
const TITLE_PX: f32 = 11.0;
const CLOSE_SLOT: f32 = 16.0;
const CLOSE_GLYPH: f32 = 9.0;
const INDICATOR_H: f32 = 1.5;
const UNREAD_DOT: f32 = 6.0;
const LANE_LEAD: f32 = 6.0;
const LANE_TRAIL: f32 = 8.0;
const BUTTON_W: f32 = 22.0;
const BUTTON_GAP: f32 = 4.0;
const BUTTON_GLYPH: f32 = 12.0;
/// Tab drop insertion bar (`TabBarMetrics.dropIndicatorWidth/Height`).
const DROP_BAR_W: f32 = 2.0;
const DROP_BAR_H: f32 = 20.0;

/// A tab being dragged; also the ghost drawn under the pointer. Ids only, so a drag never
/// keeps a closed pane or terminal alive.
#[derive(Clone)]
pub(super) struct DraggedTab {
    surface: SurfaceId,
    title: SharedString,
    palette: TabPalette,
}

impl Render for DraggedTab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .h(px(TAB_BAR_HEIGHT))
            .max_w(px(TAB_MAX_W))
            .px(px(TAB_PAD))
            .bg(self.palette.bar)
            .text_size(px(TITLE_PX))
            .text_color(self.palette.active_text)
            .whitespace_nowrap()
            .overflow_hidden()
            .text_ellipsis()
            .child(self.title.clone())
    }
}

/// Bonsplit `TabDropIndexResolver`: before the first tab whose midpoint is right of `x`,
/// else after the last.
pub fn insertion_index(mid_xs: &[f32], x: f32) -> usize {
    mid_xs.iter().position(|m| *m > x).unwrap_or(mid_xs.len())
}

/// The right-hand split-action buttons (cmux's default set minus the browser).
#[derive(Debug, Clone, Copy)]
enum StripButton {
    NewTerminal,
    SplitRight,
    SplitDown,
}

impl WorkspaceView {
    pub(super) fn palette(&self, focused: bool) -> TabPalette {
        let cfg = &self.style.cfg;
        chrome::tab_palette(
            chrome::rgba(cfg.background, cfg.background_opacity),
            focused,
        )
    }

    fn tab_title(&self, surface: SurfaceId, cx: &Context<Self>) -> SharedString {
        self.surfaces
            .get(&surface)
            .and_then(|s| {
                s.view
                    .read(cx)
                    .title()
                    .map(|t| SharedString::from(t.to_string()))
            })
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "Terminal".into())
    }

    pub(super) fn render_tab_strip(
        &self,
        pane: &Pane,
        focused: bool,
        strip_x: f32,
        chrome: super::PaneChrome,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let p = self.palette(focused);
        let pane_id = pane.id;
        let bounds = self.tab_bounds.clone();
        let drop_bar = self.drop_bar_x(pane_id, strip_x).map(|x| {
            div()
                .absolute()
                .left(px(x - DROP_BAR_W / 2.0))
                .top(px((TAB_BAR_HEIGHT - DROP_BAR_H) / 2.0))
                .w(px(DROP_BAR_W))
                .h(px(DROP_BAR_H))
                .bg(p.accent)
        });
        let tabs: Vec<_> = pane
            .tabs
            .iter()
            .enumerate()
            .map(|(ix, s)| self.render_tab(*s, ix == pane.selected, p, cx))
            .collect();
        let buttons = [
            (StripButton::NewTerminal, "icons/square-terminal.svg"),
            (StripButton::SplitRight, "icons/columns-2.svg"),
            (StripButton::SplitDown, "icons/rows-2.svg"),
        ]
        .map(|(b, icon)| {
            div()
                .id(SharedString::from(format!("strip-{}-{:?}", pane_id.0, b)))
                .flex()
                .items_center()
                .justify_center()
                .w(px(BUTTON_W))
                .h_full()
                .child(
                    svg()
                        .path(icon)
                        .size(px(BUTTON_GLYPH))
                        .text_color(p.inactive_text),
                )
                .active(|s| s.opacity(0.72))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.strip_button(pane_id, b, window, cx)
                }))
        });
        div()
            .relative()
            .flex()
            .flex_row()
            .flex_none()
            .w_full()
            .h(px(TAB_BAR_HEIGHT))
            .pl(px(chrome.lead))
            .pr(px(chrome.trail))
            .rounded_tl(px(chrome.tl))
            .rounded_tr(px(chrome.tr))
            .bg(p.bar)
            .on_drag_move(
                cx.listener(move |this, ev: &DragMoveEvent<DraggedTab>, _, cx| {
                    this.tab_drag_moved(pane_id, ev, cx)
                }),
            )
            .on_drop(cx.listener(move |this, d: &DraggedTab, window, cx| {
                this.drop_tab(pane_id, d.surface, window, cx)
            }))
            .child(
                div()
                    // Tab bounds for drop-index maths (they are content-sized).
                    .on_children_prepainted(move |b, _, _| {
                        bounds.borrow_mut().insert(pane_id, b);
                    })
                    .id(SharedString::from(format!("tabs-{}", pane_id.0)))
                    .flex()
                    .flex_row()
                    .flex_shrink()
                    .min_w(px(0.))
                    .h_full()
                    .overflow_x_scroll()
                    .children(tabs),
            )
            .child(
                // Empty strip area: the bottom separator continues; double-click opens a tab,
                // and a drag moves a self-decorated window.
                crate::csd::window_drag(div(), false)
                    .id(SharedString::from(format!("strip-empty-{}", pane_id.0)))
                    .flex_1()
                    .h_full()
                    .border_b_1()
                    .border_color(p.separator)
                    .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                        if ev.click_count() == 2 {
                            this.layout.focus_pane(pane_id);
                            this.new_tab(window, cx);
                        } else if this.layout.focus_pane(pane_id) {
                            this.focus_active(window, cx);
                        }
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_none()
                    .items_center()
                    .gap(px(BUTTON_GAP))
                    .pl(px(LANE_LEAD))
                    .pr(px(LANE_TRAIL))
                    .h_full()
                    .border_b_1()
                    .border_color(p.separator)
                    .children(buttons),
            )
            .children(drop_bar)
    }

    /// Where the insertion bar goes while a tab is dragged over this strip (strip-relative x).
    fn drop_bar_x(&self, pane: PaneId, strip_x: f32) -> Option<f32> {
        let (target, index) = self.drop_target?;
        if target != pane {
            return None;
        }
        let bounds = self.tab_bounds.borrow();
        let tabs = bounds.get(&pane)?;
        let x = match tabs.get(index) {
            Some(b) => b.origin.x,
            None => tabs.last()?.right(),
        };
        Some(f32::from(x) - strip_x)
    }

    fn tab_drag_moved(
        &mut self,
        pane: PaneId,
        ev: &DragMoveEvent<DraggedTab>,
        cx: &mut Context<Self>,
    ) {
        // Every strip hears every move; only the one under the pointer claims the drop.
        let inside = ev.bounds.contains(&ev.event.position);
        let target = if inside {
            let mids: Vec<f32> = self
                .tab_bounds
                .borrow()
                .get(&pane)
                .map(|b| b.iter().map(|b| f32::from(b.center().x)).collect())
                .unwrap_or_default();
            Some((pane, insertion_index(&mids, f32::from(ev.event.position.x))))
        } else if self.drop_target.is_some_and(|(p, _)| p == pane) {
            None
        } else {
            return;
        };
        if self.drop_target != target {
            self.drop_target = target;
            cx.notify();
        }
    }

    fn drop_tab(
        &mut self,
        pane: PaneId,
        surface: SurfaceId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let index = match self.drop_target.take() {
            Some((p, ix)) if p == pane => ix,
            _ => self.layout.pane(pane).map_or(0, |p| p.tabs.len()),
        };
        self.layout.move_surface(surface, pane, index);
        self.focus_active(window, cx);
    }

    fn render_tab(
        &self,
        surface: SurfaceId,
        selected: bool,
        p: TabPalette,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let group = SharedString::from(format!("tab-{}", surface.0));
        let text = if selected {
            p.active_text
        } else {
            p.inactive_text
        };
        let ghost = DraggedTab {
            surface,
            title: self.tab_title(surface, cx),
            palette: p,
        };
        let close_group = SharedString::from(format!("tab-close-{}", surface.0));
        // Bonsplit paints the unread dot in systemBlue, never desaturated (`TabBarColors.swift:363`).
        let dot = chrome::accent(chrome::wcag_is_light(chrome::rgba(
            self.style.cfg.background,
            1.0,
        )));
        // gpui's svg paints only its own text colour (it does not inherit), so the button is
        // revealed with opacity (which applies to children) and the glyph brightens through its
        // own group hover.
        let close = div()
            .id(close_group.clone())
            .group(close_group.clone())
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .ml(px(CONTENT_GAP))
            .size(px(CLOSE_SLOT))
            .rounded_full()
            // Shown on the selected or hovered tab, with a hover circle over the button.
            .opacity(if selected { 1.0 } else { 0.0 })
            .group_hover(group.clone(), |s| s.opacity(1.0))
            .hover(|s| s.bg(p.hover_fill))
            .child(
                svg()
                    .path("icons/x.svg")
                    .size(px(CLOSE_GLYPH))
                    .text_color(p.inactive_text)
                    .group_hover(close_group, |s| s.text_color(p.active_text)),
            )
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                cx.stop_propagation();
                // A fast double-click must not close this tab and then hit the next one
                // sliding under the cursor.
                if ev.click_count() > 1 {
                    return;
                }
                this.close_surface(surface, window, cx);
            }));
        div()
            .id(SharedString::from(format!("tab-{}", surface.0)))
            .group(group.clone())
            .relative()
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .h_full()
            .min_w(px(TAB_MIN_W))
            .max_w(px(TAB_MAX_W))
            .px(px(TAB_PAD))
            .border_r_1()
            .border_color(p.separator)
            .when(selected, |d| d.bg(p.active_fill))
            // The selected tab merges with the content below: no bottom separator under it.
            .when(!selected, |d| d.border_b_1().hover(|s| s.bg(p.hover_fill)))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(ICON_SLOT))
                    .child(
                        svg()
                            .path("icons/terminal.svg")
                            .size(px(ICON_GLYPH))
                            .text_color(text),
                    ),
            )
            .child(
                div()
                    .ml(px(CONTENT_GAP))
                    .flex_shrink()
                    .min_w(px(0.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_size(px(TITLE_PX))
                    .text_color(text)
                    .child(self.tab_title(surface, cx)),
            )
            .child(close)
            // Unread dot in the close slot of a background tab; hover shows the × instead
            // (Bonsplit TabItemView.swift:1159-1172, plan M4 D5).
            .when(!selected && self.attention.contains(&surface), |d| {
                d.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .right(px(TAB_PAD))
                        .w(px(CLOSE_SLOT))
                        .flex()
                        .items_center()
                        .justify_center()
                        .group_hover(group.clone(), |s| s.opacity(0.))
                        .child(div().size(px(UNREAD_DOT)).rounded_full().bg(dot)),
                )
            })
            .when(selected, |d| {
                d.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right(px(1.))
                        .h(px(INDICATOR_H))
                        .bg(p.accent),
                )
            })
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                this.layout.select_surface(surface);
                // Double-click on a tab toggles pane zoom (`TabItemView.swift:424`).
                if ev.click_count() == 2 {
                    this.layout.toggle_zoom();
                }
                this.focus_active(window, cx);
            }))
            .on_drag(ghost, |tab, _, _, cx| cx.new(|_| tab.clone()))
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    this.close_surface(surface, window, cx)
                }),
            )
    }

    fn strip_button(
        &mut self,
        pane: PaneId,
        b: StripButton,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.layout.focus_pane(pane);
        match b {
            StripButton::NewTerminal => self.new_tab(window, cx),
            StripButton::SplitRight => self.split(Orientation::Horizontal, window, cx),
            StripButton::SplitDown => self.split(Orientation::Vertical, window, cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_index_is_before_the_first_tab_whose_middle_is_right_of_the_pointer() {
        let mids = [30.0, 90.0, 150.0];
        assert_eq!(insertion_index(&mids, 0.0), 0);
        assert_eq!(insertion_index(&mids, 31.0), 1);
        assert_eq!(insertion_index(&mids, 120.0), 2);
        assert_eq!(insertion_index(&mids, 400.0), 3);
        assert_eq!(insertion_index(&[], 10.0), 0);
    }
}
