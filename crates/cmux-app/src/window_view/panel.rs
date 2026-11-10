//! The notification panel (plan M4 D7, rn §7): cmux's titlebar popover as an overlay under the
//! sidebar's bell.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cmux_core::NotificationId;
use gpui::{
    AnyElement, BoxShadow, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Rgba, SharedString, StatefulInteractiveElement, Styled, Window, anchored,
    deferred, div, point, prelude::*, px, svg,
};

use super::WindowView;
use crate::chrome::{self, SidebarPalette};

// rn §7 metrics.
const WIDTH: f32 = 560.0;
const HEIGHT: f32 = 760.0;
const MARGIN: f32 = 8.0;
const HEADER_PAD_X: f32 = 14.0;
const HEADER_PAD_Y: f32 = 10.0;
const ROW_MIN_H: f32 = 56.0;
const UNREAD_BAR: f32 = 2.5;
const CLOSE: f32 = 20.0;
const HOVER_ALPHA: f32 = 0.11;
/// A click outside closes the panel; a click on the bell right after must not reopen it.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

fn alpha(c: Rgba, a: f32) -> Rgba {
    Rgba { a, ..c }
}

impl WindowView {
    /// The bell's click: toggles the panel, unless the same click just closed it.
    pub(super) fn toggle_panel(&mut self, cx: &mut Context<Self>) {
        if self
            .panel_dismissed
            .is_some_and(|at| at.elapsed() < REOPEN_GUARD)
        {
            return;
        }
        self.panel_open = !self.panel_open;
        cx.notify();
    }

    pub(super) fn close_panel(&mut self, cx: &mut Context<Self>) {
        if self.panel_open {
            self.panel_open = false;
            self.panel_dismissed = Some(Instant::now());
            cx.notify();
        }
    }

    pub(super) fn render_panel(
        &self,
        window: &Window,
        cx: &Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        if !self.panel_open {
            return None;
        }
        let cfg = &self.style.cfg;
        let p = chrome::sidebar_palette(chrome::rgba(cfg.background, 1.0));
        let size = window.viewport_size();
        let w = WIDTH.min(f32::from(size.width) - 2. * MARGIN).max(0.);
        let at = self.bell_anchor(window);
        let h = HEIGHT
            .min(f32::from(size.height) - f32::from(at.y) - MARGIN)
            .max(0.);
        let list = self.notifications.list();
        let unread = self.notifications.total_unread();
        let header = self.render_panel_header(p, unread, !list.is_empty(), cx);
        let body: AnyElement = if list.is_empty() {
            empty_state(p).into_any_element()
        } else {
            div()
                .id("notif-list")
                .flex()
                .flex_col()
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .children(list.iter().map(|n| {
                    self.render_panel_row(
                        n.id,
                        self.title_of(n.workspace),
                        n.title.clone(),
                        n.body.clone(),
                        clock(n.created),
                        !n.read,
                        p,
                        cx,
                    )
                }))
                .into_any_element()
        };
        Some(deferred(
            anchored()
                .position(at)
                .snap_to_window_with_margin(px(MARGIN))
                .child(
                    div()
                        .id("notif-panel")
                        .occlude()
                        .flex()
                        .flex_col()
                        .w(px(w))
                        .h(px(h))
                        .overflow_hidden()
                        .rounded(px(8.))
                        .border_1()
                        .border_color(p.border)
                        .bg(p.bg)
                        .shadow(vec![BoxShadow {
                            color: alpha(gpui::black().into(), 0.3).into(),
                            offset: point(px(0.), px(4.)),
                            blur_radius: px(16.),
                            spread_radius: px(0.),
                        }])
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_panel(cx)))
                        .child(header)
                        .child(body),
                ),
        ))
    }

    fn render_panel_header(
        &self,
        p: SidebarPalette,
        unread: usize,
        any: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let button = |id: &'static str, label: &'static str, enabled: bool| {
            div()
                .id(id)
                .px(px(8.))
                .py(px(3.))
                .rounded(px(6.))
                .text_size(px(12.))
                .text_color(p.title)
                .bg(alpha(p.title, 0.08))
                .when(!enabled, |d| d.opacity(0.4))
                .when(enabled, |d| d.hover(|s| s.bg(alpha(p.title, 0.14))))
                .child(label)
        };
        div()
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .px(px(HEADER_PAD_X))
            .py(px(HEADER_PAD_Y))
            .border_b_1()
            .border_color(p.border)
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.title)
                    .child("Notifications"),
            )
            .when(unread > 0, |d| {
                d.child(
                    div()
                        .px(px(6.))
                        .rounded_full()
                        .bg(p.accent)
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(gpui::white())
                        .child(unread.to_string()),
                )
            })
            .child(div().flex_1())
            .child(
                button("notif-jump", "Jump to Latest", unread > 0).when(unread > 0, |d| {
                    d.on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.close_panel(cx);
                        this.jump_to_latest_unread(window, cx);
                    }))
                }),
            )
            .child(button("notif-clear", "Clear All", any).when(any, |d| {
                d.on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.clear_all(cx)))
            }))
    }

    #[allow(clippy::too_many_arguments)]
    fn render_panel_row(
        &self,
        id: NotificationId,
        workspace: String,
        title: String,
        body: String,
        time: String,
        unread: bool,
        p: SidebarPalette,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let group = SharedString::from(format!("notif-{}", id.0));
        let line = |size: f32, c: Rgba| {
            div()
                .text_size(px(size))
                .text_color(c)
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
        };
        div()
            .id(("notif-row", id.0 as usize))
            .group(group.clone())
            .relative()
            .flex()
            .flex_row()
            .flex_none()
            .min_h(px(ROW_MIN_H))
            .border_b_1()
            .border_color(p.border)
            .hover(|s| s.bg(alpha(p.title, HOVER_ALPHA)))
            .child(
                div()
                    .flex_none()
                    .w(px(UNREAD_BAR))
                    .when(unread, |d| d.bg(p.accent)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.))
                    .gap(px(2.))
                    .px(px(12.))
                    .py(px(8.))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                line(12.5, p.title)
                                    .flex_1()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(workspace),
                            )
                            .child(line(10.5, p.detail).flex_none().child(time)),
                    )
                    .when(!title.trim().is_empty(), |d| {
                        d.child(
                            line(10.5, p.detail)
                                .font_weight(FontWeight::MEDIUM)
                                .child(title),
                        )
                    })
                    .when(!body.trim().is_empty(), |d| {
                        d.child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.detail)
                                .line_clamp(2)
                                .text_ellipsis()
                                .child(super::sidebar::clip_message(&body).to_string()),
                        )
                    }),
            )
            .child(
                div()
                    .id(("notif-x", id.0 as usize))
                    .absolute()
                    .top(px(8.))
                    .right(px(8.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(CLOSE))
                    .rounded_full()
                    .opacity(0.)
                    .group_hover(group, |s| s.opacity(1.))
                    .hover(|s| s.bg(alpha(p.title, HOVER_ALPHA)))
                    .child(svg().path("icons/x.svg").size(px(9.)).text_color(p.detail))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        let gone = this.notifications.remove(id);
                        this.after_change(gone, cx);
                    })),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.close_panel(cx);
                this.jump_to(id, window, cx);
            }))
    }
}

fn empty_state(p: SidebarPalette) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .child(
            svg()
                .path("icons/bell-off.svg")
                .size(px(28.))
                .text_color(p.detail),
        )
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.title)
                .child("No notifications yet"),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(p.detail)
                .child("Desktop notifications will appear here."),
        )
}

/// `HH:MM` in local time (cmux: `.shortened` time; a 24 h clock here, std has no locale).
pub(crate) fn clock(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()) as libc::time_t;
    // SAFETY: localtime_r only writes the tm we own; a zeroed tm is a valid out-parameter.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&secs, &mut tm) }.is_null() {
        return "--:--".into();
    }
    format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_is_two_digit_24h() {
        for t in [
            std::time::UNIX_EPOCH,
            std::time::SystemTime::now(),
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(86_399),
        ] {
            let c = clock(t);
            let b = c.as_bytes();
            assert_eq!(b.len(), 5, "{c}");
            assert!(
                b[2] == b':' && [0, 1, 3, 4].iter().all(|&i| b[i].is_ascii_digit()),
                "{c}"
            );
            assert!(c[..2].parse::<u8>().unwrap() < 24 && c[3..].parse::<u8>().unwrap() < 60);
        }
    }
}
