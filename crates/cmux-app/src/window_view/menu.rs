//! The sidebar's context menu (M8 plan D8): a small popup at the pointer for workspace rows,
//! group headers and the blank area below the list.

use crate::ui::px;
use cmux_core::{GroupId, WorkspaceId};
use gpui::{
    BoxShadow, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Pixels, Point,
    Rgba, SharedString, StatefulInteractiveElement, Styled, Window, anchored, deferred, div, point,
};

use super::WindowView;
use crate::chrome;

const WIDTH: f32 = 220.0;
const MARGIN: f32 = 4.0;
const PAD_Y: f32 = 4.0;
const ITEM_PAD_X: f32 = 10.0;
const ITEM_PAD_Y: f32 = 4.0;
const ITEM_PX: f32 = 12.0;
const ITEM_RADIUS: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuAction {
    NewEmptyGroup,
    GroupFromWorkspace(WorkspaceId),
    MoveToGroup(WorkspaceId, GroupId),
    RemoveFromGroup(WorkspaceId),
    NewInGroup(GroupId),
    RenameGroup(GroupId),
    PinGroup(GroupId, bool),
    Ungroup(GroupId),
    DeleteGroup(GroupId),
    OpenSettings,
}

/// `None` is a separator.
pub(super) type MenuItems = Vec<Option<(String, MenuAction)>>;

pub(super) struct Menu {
    at: Point<Pixels>,
    items: MenuItems,
}

/// A workspace row's menu (rg §11a); "Move to Group" is one item per other group.
pub(super) fn workspace_items(
    ws: WorkspaceId,
    own: Option<GroupId>,
    groups: &[(GroupId, String)],
) -> MenuItems {
    let mut items = vec![
        Some((
            "New Empty Workspace Group".into(),
            MenuAction::NewEmptyGroup,
        )),
        Some((
            "New Group from Workspace".into(),
            MenuAction::GroupFromWorkspace(ws),
        )),
    ];
    let others: Vec<_> = groups.iter().filter(|(g, _)| Some(*g) != own).collect();
    if !others.is_empty() || own.is_some() {
        items.push(None);
    }
    for (g, name) in others {
        items.push(Some((
            format!("Move to Group “{name}”"),
            MenuAction::MoveToGroup(ws, *g),
        )));
    }
    if own.is_some() {
        items.push(Some((
            "Remove from Group".into(),
            MenuAction::RemoveFromGroup(ws),
        )));
    }
    items
}

/// A group header's menu (rg §11b, without the notification, config and docs entries).
pub(super) fn header_items(g: GroupId, pinned: bool) -> MenuItems {
    let pin = if pinned { "Unpin Group" } else { "Pin Group" };
    vec![
        Some(("New Workspace in Group".into(), MenuAction::NewInGroup(g))),
        None,
        Some(("Rename Group…".into(), MenuAction::RenameGroup(g))),
        Some((pin.into(), MenuAction::PinGroup(g, !pinned))),
        None,
        Some(("Ungroup Workspaces".into(), MenuAction::Ungroup(g))),
        Some(("Delete Group".into(), MenuAction::DeleteGroup(g))),
    ]
}

impl WindowView {
    pub(super) fn open_menu(
        &mut self,
        at: Point<Pixels>,
        items: MenuItems,
        cx: &mut Context<Self>,
    ) {
        self.menu = Some(Menu { at, items });
        cx.notify();
    }

    /// False if no menu was open.
    pub(super) fn close_menu(&mut self, cx: &mut Context<Self>) -> bool {
        let open = self.menu.take().is_some();
        if open {
            cx.notify();
        }
        open
    }

    pub(super) fn render_menu(&self, cx: &Context<Self>) -> Option<impl IntoElement + use<>> {
        let menu = self.menu.as_ref()?;
        let cfg = &self.style.cfg;
        let p = chrome::sidebar_palette(chrome::rgba(cfg.background, 1.0));
        let items = menu.items.iter().enumerate().map(|(k, item)| match item {
            None => div()
                .flex_none()
                .h(px(1.))
                .my(px(PAD_Y))
                .bg(p.border)
                .into_any_element(),
            Some((label, action)) => {
                let action = *action;
                div()
                    .id(("menu-item", k))
                    .mx(px(PAD_Y))
                    .px(px(ITEM_PAD_X - PAD_Y))
                    .py(px(ITEM_PAD_Y))
                    .rounded(px(ITEM_RADIUS))
                    .text_size(px(ITEM_PX))
                    .text_color(p.title)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .hover(|s| s.bg(p.accent).text_color(gpui::white()))
                    .child(SharedString::from(label.clone()))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.close_menu(cx);
                        this.run_menu(action, window, cx);
                    }))
                    .into_any_element()
            }
        });
        Some(deferred(
            anchored()
                .position(menu.at)
                .snap_to_window_with_margin(px(MARGIN))
                .child(
                    div()
                        .id("context-menu")
                        .occlude()
                        .flex()
                        .flex_col()
                        .w(px(WIDTH))
                        .py(px(PAD_Y))
                        .rounded(px(6.))
                        .border_1()
                        .border_color(p.border)
                        .bg(p.bg)
                        .shadow(vec![BoxShadow {
                            color: Rgba {
                                a: 0.3,
                                ..gpui::black().into()
                            }
                            .into(),
                            offset: point(px(0.), px(4.)),
                            blur_radius: px(16.),
                            spread_radius: px(0.),
                        }])
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.close_menu(cx);
                        }))
                        .children(items),
                ),
        ))
    }

    fn run_menu(&mut self, action: MenuAction, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            MenuAction::NewEmptyGroup => self.new_empty_group(window, cx),
            MenuAction::GroupFromWorkspace(ws) => {
                if self.workspaces.create_group("", &[ws]).is_ok() {
                    self.select(ws, window, cx);
                }
            }
            MenuAction::MoveToGroup(ws, g) => {
                if self.workspaces.add_to_group(g, ws).is_ok() {
                    cx.notify();
                }
            }
            MenuAction::RemoveFromGroup(ws) => {
                if self.workspaces.remove_from_group(ws).is_ok() {
                    cx.notify();
                }
            }
            MenuAction::NewInGroup(g) => {
                self.spawn_in_group(g, Default::default(), None, true, window, cx);
            }
            MenuAction::RenameGroup(g) => self.start_group_rename(g, window, cx),
            MenuAction::PinGroup(g, pinned) => {
                if self.workspaces.set_pinned(g, pinned) {
                    cx.notify();
                }
            }
            MenuAction::Ungroup(g) => {
                if self.workspaces.ungroup(g).is_some() {
                    cx.notify();
                }
            }
            MenuAction::DeleteGroup(g) => self.request_delete_group(g, window, cx),
            MenuAction::OpenSettings => self.open_settings(cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(items: &MenuItems) -> Vec<&str> {
        items
            .iter()
            .map(|i| i.as_ref().map_or("-", |(l, _)| l.as_str()))
            .collect()
    }

    #[test]
    fn workspace_menu_lists_the_other_groups() {
        let groups = [(GroupId(1), "api".to_string()), (GroupId(2), "web".into())];
        let plain = workspace_items(WorkspaceId(5), None, &[]);
        assert_eq!(
            labels(&plain),
            ["New Empty Workspace Group", "New Group from Workspace"]
        );
        let member = workspace_items(WorkspaceId(5), Some(GroupId(1)), &groups);
        assert_eq!(
            labels(&member),
            [
                "New Empty Workspace Group",
                "New Group from Workspace",
                "-",
                "Move to Group “web”",
                "Remove from Group"
            ]
        );
    }

    #[test]
    fn header_menu_offers_the_opposite_pin_state() {
        let l = header_items(GroupId(1), false);
        assert_eq!(
            labels(&l),
            [
                "New Workspace in Group",
                "-",
                "Rename Group…",
                "Pin Group",
                "-",
                "Ungroup Workspaces",
                "Delete Group"
            ]
        );
        assert!(labels(&header_items(GroupId(1), true)).contains(&"Unpin Group"));
    }
}
