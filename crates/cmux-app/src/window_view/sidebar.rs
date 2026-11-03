//! The workspace sidebar (plan D2, D3): top band, one row per workspace, resize band.

use cmux_core::WorkspaceId;
use cmux_meta::WorkspaceMeta;
use cmux_meta::pr::{Pr, PrState};
use gpui::{
    AppContext, ClickEvent, Context, CursorStyle, DragMoveEvent, EmptyView, FontWeight,
    InteractiveElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement,
    Styled, Window, div, prelude::*, px, svg,
};

use super::{CONTENT_MIN, SIDEBAR_MAX, SIDEBAR_MIN, WindowView};
use crate::chrome::{self, SidebarPalette};

// Plan D3 metrics (cmux points as logical px).
const BAND_H: f32 = 28.0;
const LIST_TOP: f32 = 30.0;
const BUTTON: f32 = 20.0;
const BUTTON_ICON: f32 = 12.0;
const BUTTON_GAP: f32 = 6.0;
const BAND_INSET: f32 = 6.0;
const RADIUS: f32 = 6.0;
const OUTER_PAD_X: f32 = 6.0;
const ROW_PAD_X: f32 = 10.0;
const ROW_PAD_Y: f32 = 8.0;
const ROW_GAP: f32 = 2.0;
const SECTION_GAP: f32 = 4.0;
const TITLE_PX: f32 = 12.5;
const DETAIL_PX: f32 = 10.0;
const CLOSE_GLYPH: f32 = 9.0;
const ITEM_GAP: f32 = 4.0;
/// The resize band reaches this far into the sidebar and into the content.
const RESIZE_IN: f32 = 6.0;
const RESIZE_OUT: f32 = 4.0;

/// The sidebar edge being dragged.
#[derive(Clone, Copy)]
pub(super) struct SidebarDrag;

/// What one row shows below its title.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RowModel {
    pub title: String,
    /// (branch, display dir); the dir is `None` when it repeats the title.
    pub dirs: Vec<(Option<String>, Option<String>)>,
    /// (pr, stale), one per unique URL.
    pub prs: Vec<(Pr, bool)>,
    pub ports: Vec<u16>,
}

pub(crate) fn row_model(title: &str, meta: &WorkspaceMeta, prs: &[(Pr, bool)]) -> RowModel {
    let dirs = meta
        .dirs
        .iter()
        .map(|d| {
            (
                d.branch.clone(),
                Some(d.display.clone()).filter(|s| s != title),
            )
        })
        .filter(|(b, d)| b.is_some() || d.is_some())
        .collect();
    let mut unique: Vec<(Pr, bool)> = Vec::new();
    for p in prs {
        if !unique.iter().any(|(u, _)| u.url == p.0.url) {
            unique.push(p.clone());
        }
    }
    RowModel {
        title: title.to_string(),
        dirs,
        prs: unique,
        ports: meta.ports.clone(),
    }
}

/// [MIN, MAX], leaving the terminal area at least CONTENT_MIN, but never below MIN.
pub(crate) fn clamp_width(w: f32, window_w: f32) -> f32 {
    w.clamp(SIDEBAR_MIN, SIDEBAR_MAX)
        .min(window_w - CONTENT_MIN)
        .max(SIDEBAR_MIN)
}

impl WindowView {
    /// The drawn sidebar width, or `None` when hidden.
    pub(super) fn sidebar_px(&self, window: &Window) -> Option<f32> {
        let window_w = f32::from(window.viewport_size().width);
        self.sidebar_visible
            .then(|| clamp_width(self.sidebar_width, window_w))
    }

    // shortcut: once hidden there is no mouse way back (Super+B only); add an edge button if
    // users get stuck.
    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_visible = !self.sidebar_visible;
        cx.notify();
    }

    pub(super) fn on_sidebar_drag(
        &mut self,
        ev: &DragMoveEvent<SidebarDrag>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let x = f32::from(ev.event.position.x - ev.bounds.left());
        let w = clamp_width(x, f32::from(window.viewport_size().width));
        if w != self.sidebar_width {
            self.sidebar_width = w;
            cx.notify();
        }
    }

    /// Straddles the sidebar's trailing edge; drawn above the content so it wins the hover test.
    pub(super) fn render_resize_band(&self, width: f32) -> impl IntoElement + use<> {
        div()
            .id("sidebar-resize")
            .absolute()
            .top_0()
            .left(px(width - RESIZE_IN))
            .w(px(RESIZE_IN + RESIZE_OUT))
            .h_full()
            .cursor(CursorStyle::ResizeLeftRight)
            .occlude()
            .on_drag(SidebarDrag, |_, _, _, cx| cx.new(|_| EmptyView))
    }

    pub(crate) fn render_sidebar(
        &self,
        width: f32,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let cfg = &self.style.cfg;
        let p = chrome::sidebar_palette(chrome::rgba(cfg.background, cfg.background_opacity));
        let button = |id: &'static str, icon: &'static str| {
            div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .size(px(BUTTON))
                .rounded(px(RADIUS))
                .child(svg().path(icon).size(px(BUTTON_ICON)).text_color(p.detail))
                .active(|s| s.opacity(0.72))
        };
        let band = div()
            .flex()
            .flex_row()
            .flex_none()
            .justify_end()
            .items_center()
            .gap(px(BUTTON_GAP))
            .h(px(BAND_H))
            .px(px(BAND_INSET))
            .child(
                button("sidebar-toggle", "icons/panel-left.svg")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_sidebar(cx))),
            )
            .child(
                button("sidebar-new", "icons/plus.svg").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| this.new_workspace(false, window, cx),
                )),
            );
        let list = self.workspaces.list();
        let selected = self.workspaces.selected();
        let closable = list.len() > 1;
        let rows: Vec<_> = list
            .iter()
            .enumerate()
            .map(|(ix, w)| self.render_row(ix, w.id, w.id == selected, closable, p, cx))
            .collect();
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(width))
            .h_full()
            .bg(p.bg)
            .border_r_1()
            .border_color(p.border)
            .child(band)
            .child(
                div()
                    .id("ws-list")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.))
                    .mt(px(LIST_TOP - BAND_H))
                    .px(px(OUTER_PAD_X))
                    .gap(px(ROW_GAP))
                    .overflow_y_scroll()
                    .track_scroll(&self.sidebar_scroll)
                    .children(rows)
                    .child(
                        // Empty space below the rows: double-click adds a workspace at the end.
                        div().id("ws-list-empty").flex_1().on_click(cx.listener(
                            |this, ev: &ClickEvent, window, cx| {
                                if ev.click_count() == 2 {
                                    this.new_workspace(true, window, cx);
                                }
                            },
                        )),
                    ),
            )
    }

    fn render_row(
        &self,
        ix: usize,
        id: WorkspaceId,
        selected: bool,
        closable: bool,
        p: SidebarPalette,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let empty = WorkspaceMeta::default();
        let meta = self.meta.get(&id).unwrap_or(&empty);
        let prs: Vec<(Pr, bool)> = meta
            .dirs
            .iter()
            .filter_map(|d| {
                let key = (d.git_dir.clone()?, d.branch.clone()?);
                self.prs.get(&key).map(|(pr, stale)| (pr.clone(), stale))
            })
            .collect();
        let m = row_model(&self.title_of(id, cx), meta, &prs);
        let (title_c, detail_c) = if selected {
            (p.selected_title, p.selected_detail)
        } else {
            (p.title, p.detail)
        };
        let family = self.style.font.family.clone();
        let detail = move || {
            div()
                .text_size(px(DETAIL_PX))
                .font_family(family.clone())
                .text_color(detail_c)
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
        };
        let group = SharedString::from(format!("ws-row-{ix}"));
        let close = closable.then(|| {
            div()
                .id(("ws-close", ix))
                .flex_none()
                .ml(px(ITEM_GAP))
                .opacity(0.)
                .group_hover(group.clone(), |s| s.opacity(1.))
                .child(
                    svg()
                        .path("icons/x.svg")
                        .size(px(CLOSE_GLYPH))
                        .text_color(detail_c),
                )
                .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    // A fast double-click must not also close the row sliding under the cursor.
                    if ev.click_count() == 1 {
                        this.request_close(id, window, cx);
                    }
                }))
        });
        let title = div()
            .flex()
            .flex_row()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_size(px(TITLE_PX))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(title_c)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(m.title),
            )
            .children(close);
        let dirs = m.dirs.into_iter().map(|(branch, dir)| {
            let text = match (branch, dir) {
                (Some(b), Some(d)) => format!("{b} · {d}"),
                (Some(s), None) | (None, Some(s)) => s,
                (None, None) => String::new(),
            };
            detail().child(text)
        });
        let prs = m.prs.into_iter().enumerate().map(|(k, (pr, stale))| {
            let (icon, word) = match pr.state {
                PrState::Open => ("icons/git-pull-request.svg", "open"),
                PrState::Merged => ("icons/git-merge.svg", "merged"),
                PrState::Closed => ("icons/circle-x.svg", "closed"),
            };
            let url = pr.url;
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(ITEM_GAP))
                .text_size(px(DETAIL_PX))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(detail_c)
                .whitespace_nowrap()
                .when(stale, |d| d.opacity(0.5))
                .child(
                    svg()
                        .path(icon)
                        .flex_none()
                        .size(px(DETAIL_PX))
                        .text_color(detail_c),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("ws-pr-{ix}-{k}")))
                        .underline()
                        .child(format!("PR #{}", pr.number))
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            cx.open_url(&url);
                        }),
                )
                .child(word)
        });
        let ports = (!m.ports.is_empty()).then(|| {
            detail()
                .flex()
                .flex_row()
                .gap(px(ITEM_GAP))
                .children(m.ports.into_iter().map(|port| {
                    div()
                        .id(SharedString::from(format!("ws-port-{ix}-{port}")))
                        .underline()
                        .child(format!(":{port}"))
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            cx.open_url(&format!("http://localhost:{port}"));
                        })
                }))
        });
        div()
            .id(("ws", ix))
            .group(group)
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(SECTION_GAP))
            .px(px(ROW_PAD_X))
            .py(px(ROW_PAD_Y))
            .rounded(px(RADIUS))
            .when(selected, |d| d.bg(p.selected_fill))
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| this.select(id, window, cx)),
            )
            .child(title)
            .children(dirs)
            .children(prs)
            .children(ports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cmux_meta::{DirLine, WorkspaceMeta};

    #[test]
    fn row_model_drops_the_title_echo_and_keeps_branch() {
        let meta = WorkspaceMeta {
            dirs: vec![
                DirLine {
                    path: "/home/u/api".into(),
                    display: "~/api".into(),
                    branch: Some("feat".into()),
                    git_dir: None,
                },
                DirLine {
                    path: "/tmp".into(),
                    display: "/tmp".into(),
                    branch: None,
                    git_dir: None,
                },
            ],
            ports: vec![3000, 5173],
        };
        let r = row_model("~/api", &meta, &[]);
        assert_eq!(
            r.dirs,
            vec![(Some("feat".into()), None), (None, Some("/tmp".into()))]
        );
        assert_eq!(r.ports, vec![3000, 5173]);
    }

    #[test]
    fn clamp_width_keeps_content_room() {
        assert_eq!(clamp_width(100.0, 2000.0), 240.0);
        assert_eq!(clamp_width(900.0, 2000.0), 600.0);
        assert_eq!(clamp_width(500.0, 700.0), 400.0);
        assert_eq!(clamp_width(500.0, 400.0), 240.0, "never below the minimum");
    }
}
