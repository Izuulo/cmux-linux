//! The sidebar's context menu (M8 plan D8): a small popup at the pointer for workspace rows,
//! group headers and the blank area below the list.

use crate::ui::px;
use cmux_core::{GroupId, WorkspaceId};
use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Pixels, Point,
    StatefulInteractiveElement, Styled, Window, anchored, deferred, div, prelude::FluentBuilder,
};
use std::time::Instant;

use super::WindowView;
use crate::chrome;
use crate::mac::{self, Fade, Theme, metric};

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
    pub(super) fade: Fade,
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
        self.menu = Some(Menu {
            at,
            items,
            fade: Fade::enter(),
        });
        cx.notify();
    }

    /// False if no menu was open. The menu fades out on `menu_leaving`, for looks only.
    pub(super) fn close_menu(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(mut m) = self.menu.take() else {
            return false;
        };
        m.fade = Fade::leave_from(&m.fade, Instant::now());
        self.menu_leaving = Some(m);
        cx.notify();
        true
    }

    /// The live menu, then the one still fading out (drawn on top of nothing it must block).
    pub(super) fn render_menu(&self, cx: &Context<Self>) -> Vec<AnyElement> {
        let now = Instant::now();
        let overlay = |m: &Menu, live| self.menu_overlay(m, live, now, cx).into_any_element();
        let leaving = self.menu_leaving.as_ref().map(|m| overlay(m, false));
        let live = self.menu.as_ref().map(|m| overlay(m, true));
        leaving.into_iter().chain(live).collect()
    }

    /// `live` menus take clicks and close on a click outside; a leaving one is only painted.
    fn menu_overlay(
        &self,
        m: &Menu,
        live: bool,
        now: Instant,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let t = Theme::new(chrome::rgba(self.style.cfg.background, 1.0));
        let rows = m
            .items
            .iter()
            .enumerate()
            .map(|(k, item)| match item {
                None => mac::menu_separator(&t),
                Some((label, action)) => {
                    let action = *action;
                    mac::menu_item(("menu-item", k), &t, label, None)
                        .when(live, |d| {
                            d.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.close_menu(cx);
                                this.run_menu(action, window, cx);
                            }))
                        })
                        .into_any_element()
                }
            })
            .collect();
        deferred(
            anchored()
                .position(m.at)
                .snap_to_window_with_margin(px(metric::OVERLAY_MARGIN))
                .child(
                    div()
                        .id(if live {
                            "context-menu"
                        } else {
                            "context-menu-leaving"
                        })
                        .when(live, |d| {
                            d.occlude().on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                this.close_menu(cx);
                            }))
                        })
                        .opacity(m.fade.amount_at(now))
                        .mt(px(m.fade.slide(now)))
                        .child(mac::menu(&t, rows)),
                ),
        )
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
