//! The workspace sidebar (plan D2, D3): top band, one row per workspace, resize band.

use crate::ui::{px, s};
use cmux_core::{GroupId, Row, WorkspaceId};
use cmux_meta::WorkspaceMeta;
use cmux_meta::pr::{Pr, PrState};
use gpui::{
    AppContext, ClickEvent, Context, CursorStyle, DragMoveEvent, EmptyView, FontWeight,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Render,
    Rgba, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, svg,
};

use super::menu::{self, MenuAction};
use super::rename::{RenameEvent, RenameInput, RenameStyle};
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
/// The close button's slot, reserved whenever the row can close (`SidebarWorkspaceRowCellView`).
const CLOSE_SLOT: f32 = 16.0;
/// Gap between the title and its leading badge / trailing close slot.
const TITLE_ROW_GAP: f32 = 8.0;
const ITEM_GAP: f32 = 4.0;
/// Lines inside the branch/dir and PR blocks.
const LINE_GAP: f32 = 1.0;
/// Unread badge on a row (rn §6) and on the bell (classic titlebar style).
const BADGE: f32 = 16.0;
const BADGE_PX: f32 = 9.0;
const BELL_BADGE: f32 = 12.0;
const BELL_BADGE_PX: f32 = 7.0;
const BELL_BADGE_OFFSET: f32 = 3.0;
/// Band icon tint: secondary at these opacities (`HeaderChromeIconStyle`).
const ICON_ALPHA: f32 = 0.86;
const ICON_HOVER_ALPHA: f32 = 0.96;
/// Notification message under the title (rn §6); the selected row's white is at this alpha.
const MESSAGE_LINES: usize = 12;
const MESSAGE_ALPHA: f32 = 0.8;
/// The close glyph on the selected row: white at this alpha.
const CLOSE_ALPHA: f32 = 0.7;
/// The resize band reaches this far into the sidebar and into the content.
const RESIZE_IN: f32 = 6.0;
const RESIZE_OUT: f32 = 4.0;

// Group header (research-groups.md §5.8) and the indent of its member rows.
const HEADER_PAD_Y: f32 = 5.0;
const HEADER_RADIUS: f32 = 4.0;
const HEADER_TRAIL: f32 = 10.0;
const HEADER_PX: f32 = 11.0;
const HEADER_BOX: f32 = 14.0;
const CHEVRON_ICON: f32 = 9.0;
const HEADER_ICON: f32 = 11.0;
const HEADER_PLUS_BOX: f32 = 16.0;
const HEADER_GAP: f32 = 4.0;
const HEADER_NAME_GAP: f32 = 6.0;
/// The name: primary at this alpha, full when the anchor is selected.
const HEADER_NAME_ALPHA: f32 = 0.9;
const HEADER_SELECTED_FILL: f32 = 0.08;
const MEMBER_INDENT: f32 = 12.0;

/// The sidebar edge being dragged.
#[derive(Clone, Copy)]
pub(super) struct SidebarDrag;

/// The row being dragged; also the drag ghost.
#[derive(Clone)]
pub(super) struct DraggedWorkspace {
    pub id: WorkspaceId,
    pub title: SharedString,
    pub accent: gpui::Rgba,
}

impl Render for DraggedWorkspace {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(ROW_PAD_X))
            .py(px(ROW_PAD_Y))
            .rounded(px(RADIUS))
            .bg(self.accent)
            .text_size(px(TITLE_PX))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(gpui::white())
            .whitespace_nowrap()
            .child(self.title.clone())
    }
}

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
                Some(d.display.clone()).filter(|s| s != title && !s.is_empty()),
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
/// The sidebar row's unread badge shows the full count (rn §6).
pub(crate) fn badge_label(count: usize) -> String {
    count.to_string()
}

/// The bell's badge: nothing at 0, else the count capped at 99 without a "+" (rn §9).
pub(crate) fn bell_label(count: usize) -> Option<String> {
    (count > 0).then(|| count.min(99).to_string())
}

/// cmux bounds the sidebar message to 4096 characters before layout (rn §6).
pub(crate) fn clip_message(s: &str) -> &str {
    s.char_indices().nth(4096).map_or(s, |(i, _)| &s[..i])
}

pub(crate) fn clamp_width(w: f32, window_w: f32) -> f32 {
    w.clamp(s(SIDEBAR_MIN), s(SIDEBAR_MAX))
        .min(window_w - s(CONTENT_MIN))
        .max(s(SIDEBAR_MIN))
}

/// The gap (0..=len) a dragged row drops into: before the first row whose midpoint is below `y`.
pub(crate) fn drop_gap(row_mids: &[f32], y: f32) -> usize {
    row_mids
        .iter()
        .position(|m| *m > y)
        .unwrap_or(row_mids.len())
}

impl WindowView {
    /// The drawn sidebar width, or `None` when hidden.
    pub(super) fn sidebar_px(&self, window: &Window) -> Option<f32> {
        let window_w = f32::from(window.viewport_size().width) - self.right_sidebar_px(window);
        self.sidebar_visible
            .then(|| clamp_width(self.sidebar_width, window_w))
    }

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

    pub(super) fn on_row_drag(
        &mut self,
        ev: &DragMoveEvent<DraggedWorkspace>,
        cx: &mut Context<Self>,
    ) {
        let gap = ev.bounds.contains(&ev.event.position).then(|| {
            let mids: Vec<f32> = self
                .row_bounds
                .borrow()
                .iter()
                .map(|b| f32::from(b.center().y))
                .collect();
            drop_gap(&mids, f32::from(ev.event.position.y))
        });
        let dragging = Some(ev.drag(cx).id);
        if self.drop_gap != gap || self.dragging != dragging {
            self.drop_gap = gap;
            self.dragging = dragging;
            cx.notify();
        }
    }

    fn drop_row(&mut self, d: &DraggedWorkspace, cx: &mut Context<Self>) {
        if let Some(gap) = self.drop_gap.take() {
            self.workspaces.move_to(d.id, gap);
        }
        self.dragging = None;
        cx.notify();
    }

    /// Stores a finished rename: a group's name or a workspace's title.
    fn apply_rename(&mut self, id: WorkspaceId, text: Option<&str>) {
        match (self.rename_group.take(), text) {
            (Some(g), Some(t)) => {
                self.workspaces.rename_group(g, t);
            }
            (None, Some(t)) => {
                self.workspaces.rename(id, t);
            }
            (_, None) => {}
        }
    }

    /// Commits the open rename, if any, without refocusing.
    pub(super) fn commit_rename(&mut self, cx: &mut Context<Self>) {
        if let Some(r) = self.rename.take() {
            let (id, text) = r.read(cx).snapshot();
            self.apply_rename(id, text.as_deref());
        }
    }

    /// Renames the workspace inline; for a group's anchor, which has no row, the group.
    pub(super) fn start_rename(
        &mut self,
        id: WorkspaceId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(g) = self
            .workspaces
            .group_of(id)
            .filter(|_| self.workspaces.is_anchor(id))
        {
            return self.start_group_rename(g, window, cx);
        }
        let p = self.palette();
        let text = if id == self.workspaces.selected() {
            p.selected_title
        } else {
            p.title
        };
        let style = RenameStyle {
            size: TITLE_PX,
            text,
            hint: p.detail,
        };
        let current = self.title_of(id);
        let custom = self.workspaces.custom_title(id).is_some();
        self.open_rename(id, None, &current, custom, style, window, cx);
    }

    /// Renames the group inline in its header (plan D9).
    pub(super) fn start_group_rename(
        &mut self,
        g: GroupId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((anchor, name)) = self.workspaces.group(g).map(|g| (g.anchor, g.name.clone()))
        else {
            return;
        };
        let p = self.palette();
        let style = RenameStyle {
            size: HEADER_PX,
            text: p.title,
            hint: p.detail,
        };
        self.open_rename(anchor, Some(g), &name, true, style, window, cx);
    }

    #[allow(clippy::too_many_arguments)]
    fn open_rename(
        &mut self,
        id: WorkspaceId,
        group: Option<GroupId>,
        current: &str,
        custom: bool,
        style: RenameStyle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_rename(cx);
        self.sidebar_visible = true;
        let input = cx.new(|cx| RenameInput::new(id, current, custom, style, window, cx));
        cx.subscribe_in(&input, window, |this, input, ev, window, cx| {
            let RenameEvent::Done(id, text) = ev;
            if this.rename.as_ref() != Some(input) {
                return;
            }
            this.rename = None;
            this.apply_rename(*id, text.as_deref());
            let sel = this.workspaces.selected();
            this.select(sel, window, cx);
        })
        .detach();
        self.rename = Some(input);
        self.rename_group = group;
        cx.notify();
    }

    pub(super) fn palette(&self) -> SidebarPalette {
        let cfg = &self.style.cfg;
        chrome::sidebar_palette(chrome::rgba(cfg.background, cfg.background_opacity))
    }

    /// Straddles the sidebar's trailing edge; drawn above the content so it wins the hover test.
    pub(super) fn render_resize_band(&self, width: f32) -> impl IntoElement + use<> {
        div()
            .id("sidebar-resize")
            .absolute()
            .top_0()
            .left(gpui::px(width - s(RESIZE_IN)))
            .w(px(RESIZE_IN + RESIZE_OUT))
            .h_full()
            .cursor(CursorStyle::ResizeLeftRight)
            .occlude()
            .on_drag(SidebarDrag, |_, _, _, cx| cx.new(|_| EmptyView))
    }

    /// The way back with the mouse while the sidebar is hidden: the same toggle, in the
    /// window's top-left corner (the tab strip there indents by `csd::REOPEN_LEAD`).
    pub(super) fn render_reopen(&self, cx: &Context<Self>) -> impl IntoElement {
        band_button("sidebar-reopen", "icons/panel-left.svg", self.palette())
            .absolute()
            .top(px((BAND_H - BUTTON) / 2.0))
            .left(px(BAND_INSET))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_sidebar(cx)))
    }

    pub(crate) fn render_sidebar(
        &self,
        width: f32,
        radius: f32,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let p = self.palette();
        let button = |id, icon| band_button(id, icon, p);
        let band = crate::csd::window_drag(div(), true)
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
            .child(self.render_bell(p, cx))
            .child(
                button("sidebar-new", "icons/plus.svg").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| this.new_workspace(false, window, cx),
                )),
            );
        let selected = self.workspaces.selected();
        let closable = self.workspaces.len() > 1;
        let rows: Vec<_> = self
            .workspaces
            .rows()
            .into_iter()
            .map(|row| match row {
                Row::Header { group, anchor } => self
                    .render_header(group, anchor == selected, p, cx)
                    .into_any_element(),
                Row::Workspace { id, member } => self
                    .render_row(id, id == selected, closable, member, p, cx)
                    .into_any_element(),
            })
            .collect();
        let n = rows.len();
        let bounds = self.row_bounds.clone();
        let line_y = self.drop_gap.and_then(|g| {
            let b = self.row_bounds.borrow();
            let y = match g.checked_sub(1).and_then(|k| b.get(k)) {
                Some(prev) if g >= n => prev.bottom(),
                _ => b.get(g)?.top(),
            };
            Some(f32::from(y))
        });
        div()
            .relative()
            .flex()
            .flex_col()
            .flex_none()
            .w(gpui::px(width))
            .h_full()
            .bg(p.bg)
            .rounded_tl(gpui::px(radius))
            .rounded_bl(gpui::px(radius))
            .border_r_1()
            .border_color(p.border)
            .child(band)
            .child(
                div()
                    // Row bounds for the drop-gap maths (the filler below is dropped).
                    .on_children_prepainted(move |mut b, _, _| {
                        b.truncate(n);
                        *bounds.borrow_mut() = b;
                    })
                    .on_drag_move(cx.listener(
                        |this, ev: &DragMoveEvent<DraggedWorkspace>, _, cx| {
                            this.on_row_drag(ev, cx)
                        },
                    ))
                    .on_drop(cx.listener(|this, d: &DraggedWorkspace, _, cx| this.drop_row(d, cx)))
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
                        div()
                            .id("ws-list-empty")
                            .flex_1()
                            .on_click(cx.listener(|this, ev: &ClickEvent, window, cx| {
                                if ev.click_count() == 2 {
                                    this.new_workspace(true, window, cx);
                                }
                            }))
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                                    let items = vec![Some((
                                        "New Empty Workspace Group".to_string(),
                                        MenuAction::NewEmptyGroup,
                                    ))];
                                    this.open_menu(ev.position, items, cx);
                                }),
                            ),
                    ),
            )
            .when_some(line_y, |d, y| {
                // Window and sidebar share an origin (the sidebar is the root's first child).
                d.child(
                    div()
                        .absolute()
                        .top(gpui::px(y - s(1.)))
                        .left(px(8.))
                        .right(px(8.))
                        .h(px(2.))
                        .rounded(px(1.))
                        .bg(p.accent),
                )
            })
    }

    /// The bell (cmux's titlebar bell, here in the band): toggles the notification panel; its
    /// badge counts unread notifications.
    fn render_bell(&self, p: SidebarPalette, cx: &Context<Self>) -> impl IntoElement + use<> {
        let badge = bell_label(self.notifications.total_unread()).map(|label| {
            div()
                .absolute()
                .top(px(-BELL_BADGE_OFFSET))
                .right(px(-BELL_BADGE_OFFSET))
                .flex()
                .items_center()
                .justify_center()
                .size(px(BELL_BADGE))
                .rounded_full()
                .bg(p.accent)
                .text_size(px(BELL_BADGE_PX))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(gpui::white())
                .child(label)
        });
        div()
            .id("sidebar-bell")
            .group("sidebar-bell")
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .size(px(BUTTON))
            .rounded(px(RADIUS))
            .child(band_icon("sidebar-bell", "icons/bell.svg", p))
            .children(badge)
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_panel(cx)))
    }

    /// Where the notification panel hangs: under the bell, or the top-left corner when the
    /// sidebar is hidden.
    pub(super) fn bell_anchor(&self, window: &Window) -> gpui::Point<Pixels> {
        let x = self.sidebar_px(window).map_or(s(BAND_INSET), |w| {
            // Band buttons from the right: new, then the bell.
            w - s(BAND_INSET + 2. * BUTTON + BUTTON_GAP)
        });
        gpui::point(gpui::px(x), px(BAND_H))
    }

    fn render_row(
        &self,
        id: WorkspaceId,
        selected: bool,
        closable: bool,
        member: bool,
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
        let m = row_model(&self.title_of(id), meta, &prs);
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
        let group = SharedString::from(format!("ws-row-{}", id.0));
        let close = closable.then(|| {
            let glyph = if selected {
                Rgba {
                    a: CLOSE_ALPHA,
                    ..title_c
                }
            } else {
                detail_c
            };
            div()
                .id(("ws-close", id.0 as usize))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .ml(px(TITLE_ROW_GAP))
                .size(px(CLOSE_SLOT))
                .opacity(0.)
                .group_hover(group.clone(), |s| s.opacity(1.))
                .child(
                    svg()
                        .path("icons/x.svg")
                        .size(px(CLOSE_GLYPH))
                        .text_color(glyph),
                )
                .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    // A fast double-click must not also close the row sliding under the cursor.
                    if ev.click_count() == 1 {
                        this.request_close(id, window, cx);
                    }
                }))
        });
        let renaming = self
            .rename
            .as_ref()
            .filter(|r| self.rename_group.is_none() && r.read(cx).id() == id);
        let dragging = self.dragging == Some(id) && cx.has_active_drag();
        let ghost = DraggedWorkspace {
            id,
            title: m.title.clone().into(),
            accent: p.accent,
        };
        let unread = self.notifications.unread_count(id);
        let badge = (unread > 0).then(|| {
            let (fill, text) = if selected {
                (Rgba { a: 0.25, ..title_c }, title_c)
            } else {
                (p.accent, gpui::white().into())
            };
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .mr(px(TITLE_ROW_GAP))
                .size(px(BADGE))
                .rounded_full()
                .bg(fill)
                .text_size(px(BADGE_PX))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(text)
                .child(badge_label(unread))
        });
        let message = self
            .notifications
            .latest(id)
            .and_then(|n| n.summary())
            .map(|text| {
                div()
                    .text_size(px(DETAIL_PX))
                    .text_color(if selected {
                        Rgba {
                            a: MESSAGE_ALPHA,
                            ..title_c
                        }
                    } else {
                        detail_c
                    })
                    .line_clamp(MESSAGE_LINES)
                    .text_ellipsis()
                    .child(clip_message(text).to_string())
            });
        let title = div()
            .flex()
            .flex_row()
            .items_center()
            .children(badge)
            .child(match renaming {
                Some(r) => r.clone().into_any_element(),
                None => div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_size(px(TITLE_PX))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(title_c)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(m.title)
                    .into_any_element(),
            })
            .children(close);
        let dirs: Vec<_> = m
            .dirs
            .into_iter()
            .map(|(branch, dir)| {
                let text = match (branch, dir) {
                    (Some(b), Some(d)) => format!("{b} · {d}"),
                    (Some(s), None) | (None, Some(s)) => s,
                    (None, None) => String::new(),
                };
                detail().child(text)
            })
            .collect();
        let prs: Vec<_> = m
            .prs
            .into_iter()
            .enumerate()
            .map(|(k, (pr, stale))| {
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
                            .id(SharedString::from(format!("ws-pr-{}-{k}", id.0)))
                            .underline()
                            .child(format!("PR #{}", pr.number))
                            .on_click(move |_, _, cx| {
                                cx.stop_propagation();
                                cx.open_url(&url);
                            }),
                    )
                    .child(word)
            })
            .collect();
        // Lines inside one block sit 1 apart; blocks sit SECTION_GAP apart.
        let block = |lines: Vec<_>| {
            (!lines.is_empty()).then(|| div().flex().flex_col().gap(px(LINE_GAP)).children(lines))
        };
        // A remote workspace's ports are on its host (not forwarded).
        let port_host = self
            .views
            .get(&id)
            .and_then(|v| Some(cmux_meta::remote::host(v.read(cx).remote()?.last()?).to_string()))
            .map(|h| if h.contains(':') { format!("[{h}]") } else { h })
            .unwrap_or_else(|| "localhost".into());
        let ports = (!m.ports.is_empty()).then(|| {
            detail()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap(px(ITEM_GAP))
                .children(m.ports.into_iter().map(|port| {
                    let url = format!("http://{port_host}:{port}");
                    div()
                        .id(SharedString::from(format!("ws-port-{}-{port}", id.0)))
                        .underline()
                        .child(format!(":{port}"))
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            cx.open_url(&url);
                        })
                }))
        });
        div()
            .id(("ws", id.0 as usize))
            .group(group)
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(SECTION_GAP))
            .px(px(ROW_PAD_X))
            .py(px(ROW_PAD_Y))
            .rounded(px(RADIUS))
            .when(member, |d| d.ml(px(MEMBER_INDENT)))
            .when(selected, |d| d.bg(p.selected_fill))
            .when(dragging, |d| d.opacity(0.6))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    let own = this.workspaces.group_of(id);
                    let groups: Vec<_> = this
                        .workspaces
                        .groups()
                        .iter()
                        .map(|g| (g.id, g.name.clone()))
                        .collect();
                    this.open_menu(ev.position, menu::workspace_items(id, own, &groups), cx);
                }),
            )
            .when(renaming.is_none(), |d| {
                d.on_drag(ghost, |g, _, _, cx| cx.new(|_| g.clone()))
            })
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                this.select(id, window, cx);
                if ev.click_count() == 2 {
                    this.start_rename(id, window, cx);
                }
            }))
            .child(title)
            .children(message)
            .children(block(dirs))
            .children(block(prs))
            .children(ports)
    }
}

impl WindowView {
    /// A group's header (rg §5): chevron, folder, name, unread badge, and `+` on hover. It is
    /// the anchor workspace's row.
    fn render_header(
        &self,
        g: GroupId,
        selected: bool,
        p: SidebarPalette,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let group = self.workspaces.group(g).expect("rows come from the model");
        let (anchor, collapsed, pinned) = (group.anchor, group.collapsed, group.pinned);
        let hover = SharedString::from(format!("ws-group-{}", g.0));
        let glyph = |icon: &'static str, size: f32, frame: f32| {
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(frame))
                .child(svg().path(icon).size(px(size)).text_color(p.detail))
        };
        let chevron = if collapsed {
            "icons/chevron-right.svg"
        } else {
            "icons/chevron-down.svg"
        };
        let renaming = self
            .rename
            .as_ref()
            .filter(|_| self.rename_group == Some(g));
        let name = match renaming {
            Some(r) => r.clone().into_any_element(),
            None => div()
                .flex_1()
                .min_w(px(0.))
                .text_size(px(HEADER_PX))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(Rgba {
                    a: if selected { 1.0 } else { HEADER_NAME_ALPHA },
                    ..p.title
                })
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(group.name.clone())
                .into_any_element(),
        };
        let unread = self.group_unread(g);
        let badge = (unread > 0).then(|| {
            div()
                .flex_none()
                .ml(px(HEADER_NAME_GAP))
                .px(px(5.))
                .py(px(1.))
                .rounded_full()
                .bg(p.accent)
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(gpui::white())
                .child(badge_label(unread))
        });
        let ghost = DraggedWorkspace {
            id: anchor,
            title: group.name.clone().into(),
            accent: p.accent,
        };
        let dragging = self.dragging == Some(anchor) && cx.has_active_drag();
        div()
            .id(("ws-group", g.0 as usize))
            .group(hover.clone())
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .py(px(HEADER_PAD_Y))
            .pr(px(HEADER_TRAIL))
            .rounded(px(HEADER_RADIUS))
            .when(selected, |d| {
                d.bg(Rgba {
                    a: HEADER_SELECTED_FILL,
                    ..p.title
                })
            })
            .when(dragging, |d| d.opacity(0.6))
            .when(renaming.is_none(), |d| {
                d.on_drag(ghost, |g, _, _, cx| cx.new(|_| g.clone()))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.click_header(g, window, cx);
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    this.open_menu(ev.position, menu::header_items(g, pinned), cx);
                }),
            )
            .when(pinned, |d| {
                d.child(glyph("icons/pin.svg", CHEVRON_ICON, HEADER_BOX).mr(px(HEADER_GAP)))
            })
            .child(
                glyph(chevron, CHEVRON_ICON, HEADER_BOX)
                    .id(("ws-group-chevron", g.0 as usize))
                    .mr(px(HEADER_GAP))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        this.toggle_group(g, window, cx);
                    })),
            )
            .child(glyph("icons/folder.svg", HEADER_ICON, HEADER_BOX).mr(px(HEADER_NAME_GAP)))
            .child(name)
            .children(badge)
            .child(
                glyph("icons/plus.svg", HEADER_ICON, HEADER_PLUS_BOX)
                    .id(("ws-group-plus", g.0 as usize))
                    .ml(px(HEADER_GAP))
                    .opacity(0.)
                    .group_hover(hover, |s| s.opacity(1.))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        this.spawn_in_group(g, Default::default(), None, true, window, cx);
                    })),
            )
    }
}

/// A band button's glyph: secondary at 0.86, brighter while hovered (`HeaderChromeIconStyle`).
fn band_button(
    id: &'static str,
    icon: &'static str,
    p: SidebarPalette,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .group(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(BUTTON))
        .rounded(px(RADIUS))
        .child(band_icon(id, icon, p))
}

fn band_icon(group: &'static str, icon: &'static str, p: SidebarPalette) -> impl IntoElement {
    let tint = |a: f32| Rgba {
        a: p.detail.a * a,
        ..p.detail
    };
    svg()
        .path(icon)
        .size(px(BUTTON_ICON))
        .text_color(tint(ICON_ALPHA))
        .group_hover(group, move |s| s.text_color(tint(ICON_HOVER_ALPHA)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn bell_label_caps_at_99() {
        assert_eq!(bell_label(0), None);
        assert_eq!(bell_label(7).as_deref(), Some("7"));
        assert_eq!(bell_label(150).as_deref(), Some("99"));
    }

    #[test]
    fn badge_label_has_no_cap() {
        assert_eq!(badge_label(150), "150");
    }

    #[test]
    fn clip_message_respects_char_boundaries() {
        let s = "é".repeat(5000);
        assert_eq!(clip_message(&s).chars().count(), 4096);
        assert_eq!(clip_message("short"), "short");
    }

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
    fn drop_gap_uses_row_midpoints() {
        let mids = [20.0, 60.0, 100.0];
        assert_eq!(drop_gap(&mids, 5.0), 0);
        assert_eq!(drop_gap(&mids, 59.0), 1);
        assert_eq!(drop_gap(&mids, 61.0), 2);
        assert_eq!(drop_gap(&mids, 500.0), 3);
    }

    #[test]
    fn clamp_width_keeps_content_room() {
        assert_eq!(clamp_width(100.0, 2000.0), 240.0);
        assert_eq!(clamp_width(900.0, 2000.0), 600.0);
        assert_eq!(clamp_width(500.0, 700.0), 400.0);
        assert_eq!(clamp_width(500.0, 400.0), 240.0, "never below the minimum");
    }
}
