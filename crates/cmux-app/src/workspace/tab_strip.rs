//! A pane's tab strip, sized and coloured like Bonsplit's tab bar as cmux configures it
//! (research-cmux.md §4 "Pane tab strip"; numbers from `TabBarMetrics.swift`, logical px).

use cmux_core::{Orientation, Pane, PaneId};
use gpui::{
    ClickEvent, Context, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px,
    svg,
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
const LANE_LEAD: f32 = 6.0;
const LANE_TRAIL: f32 = 8.0;
const BUTTON_W: f32 = 22.0;
const BUTTON_GAP: f32 = 4.0;
const BUTTON_GLYPH: f32 = 12.0;

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

    fn tab_title(&self, surface: cmux_core::SurfaceId, cx: &Context<Self>) -> SharedString {
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
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let p = self.palette(focused);
        let pane_id = pane.id;
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
            .flex()
            .flex_row()
            .flex_none()
            .w_full()
            .h(px(TAB_BAR_HEIGHT))
            .bg(p.bar)
            .child(
                div()
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
                // Empty strip area: the bottom separator continues; double-click opens a tab.
                div()
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
    }

    fn render_tab(
        &self,
        surface: cmux_core::SurfaceId,
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
        let close_group = SharedString::from(format!("tab-close-{}", surface.0));
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
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.close_surface(surface, window, cx);
            }));
        div()
            .id(SharedString::from(format!("tab-{}", surface.0)))
            .group(group)
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
