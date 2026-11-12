//! Workspace groups in the window (M8 plan D3-D9): what the header, its menu and the group
//! shortcuts do. The rules themselves are in `cmux_core::groups`.

use cmux_core::{GroupId, HeaderClick, Placement, WorkspaceId};
use gpui::{Context, PromptLevel, Window};

use super::WindowView;

/// The Delete Group prompt's body for `n` workspaces besides the anchor (rg §6.11).
pub(super) fn delete_prompt(name: &str, n: usize) -> Option<String> {
    match n {
        0 => None,
        1 => Some(format!(
            "Delete the group “{name}” and close its workspace?"
        )),
        _ => Some(format!(
            "Delete the group “{name}” and close its {n} workspaces?"
        )),
    }
}

impl WindowView {
    /// Starts the shell of a workspace the model just added (and selected). `focus: false`
    /// hands the selection back to `old`. False, with the workspace removed again, if the
    /// shell did not start.
    pub(super) fn adopt(
        &mut self,
        id: WorkspaceId,
        old: WorkspaceId,
        cwd: Option<String>,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.make_view(id, cwd, None, window, cx) {
            self.workspaces.close(id);
            self.select(old, window, cx);
            return false;
        }
        if focus {
            self.select(id, window, cx);
        } else {
            // The new view focused its shell as it started: give focus back.
            self.workspaces.select(old);
            self.views[&id].update(cx, |v, cx| v.set_active(false, cx));
            if let Some(v) = self.views.get(&old) {
                v.update(cx, |v, cx| v.focus_active(window, cx));
            }
            cx.notify();
        }
        true
    }

    fn cwd_of(&self, ws: WorkspaceId, cx: &Context<Self>) -> Option<String> {
        self.views.get(&ws).and_then(|v| v.read(cx).active_cwd(cx))
    }

    /// `newWorkspaceGroup`: a group with a fresh anchor in the selected workspace's directory.
    pub(super) fn new_empty_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let old = self.workspaces.selected();
        let cwd = self.cwd_of(old, cx);
        let (_, anchor) = self.workspaces.create_empty_group("");
        self.adopt(anchor, old, cwd, true, window, cx);
    }

    /// A new workspace inside `g`, in the anchor's directory (rg §7.1).
    pub(super) fn spawn_in_group(
        &mut self,
        g: GroupId,
        placement: Placement,
        reference: Option<WorkspaceId>,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<WorkspaceId> {
        let old = self.workspaces.selected();
        let cwd = self
            .cwd_of(self.workspaces.group(g)?.anchor, cx)
            .or_else(|| self.cwd_of(old, cx));
        let id = self.workspaces.add_in_group(g, placement, reference)?;
        self.adopt(id, old, cwd, focus, window, cx).then_some(id)
    }

    /// Collapses or expands; a selection the collapse hid moves to the anchor.
    pub(super) fn set_group_collapsed(
        &mut self,
        g: GroupId,
        collapsed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let before = self.workspaces.selected();
        self.workspaces.set_collapsed(g, collapsed);
        let now = self.workspaces.selected();
        if now != before {
            self.select(now, window, cx);
        }
        cx.notify();
    }

    pub(super) fn toggle_group(&mut self, g: GroupId, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(collapsed) = self.workspaces.group(g).map(|g| g.collapsed) {
            self.set_group_collapsed(g, !collapsed, window, cx);
        }
    }

    /// A click on the header's name (plan D5).
    pub(super) fn click_header(&mut self, g: GroupId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(anchor) = self.workspaces.group(g).map(|g| g.anchor) else {
            return;
        };
        let untouched = !self.typed.contains(&anchor)
            && self
                .views
                .get(&anchor)
                .is_some_and(|v| v.read(cx).layout().surfaces().len() == 1);
        match self.workspaces.header_target(g, untouched) {
            Some(HeaderClick::Select(ws)) => self.select(ws, window, cx),
            Some(HeaderClick::ToggleCollapse) => self.toggle_group(g, window, cx),
            None => {}
        }
    }

    /// The header's unread badge: every member while collapsed, else the anchor (rg §4.8).
    pub(super) fn group_unread(&self, g: GroupId) -> usize {
        let Some(group) = self.workspaces.group(g) else {
            return 0;
        };
        if group.collapsed {
            self.workspaces
                .group_members(g)
                .into_iter()
                .map(|ws| self.notifications.unread_count(ws))
                .sum()
        } else {
            self.notifications.unread_count(group.anchor)
        }
    }

    /// Delete Group: asks first when it would close more than the anchor or the group is
    /// pinned (rg §6.10).
    pub(super) fn request_delete_group(
        &mut self,
        g: GroupId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self.workspaces.group(g) else {
            return;
        };
        let others = self.workspaces.group_members(g).len() - 1;
        if others == 0 && !group.pinned {
            self.delete_group(g, window, cx);
            return;
        }
        let body = delete_prompt(&group.name, others);
        let answer = window.prompt(
            PromptLevel::Warning,
            "Delete this group?",
            body.as_deref(),
            &["Delete", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if answer.await == Ok(0) {
                let _ = this.update_in(cx, |this, window, cx| {
                    this.delete_group(g, window, cx);
                });
            }
        })
        .detach();
    }

    /// Closes every workspace of the group, the anchor last; how many. A window that would be
    /// left empty gets a fresh workspace first.
    pub(super) fn delete_group(
        &mut self,
        g: GroupId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        let mut members = self.workspaces.group_members(g);
        if members.len() == self.workspaces.len() {
            self.new_workspace(true, window, cx);
        }
        members.rotate_left(1); // anchor last: no promotion on the way
        for ws in &members {
            self.close_now(*ws, window, cx);
        }
        members.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_prompt_counts_the_workspaces_besides_the_anchor() {
        assert_eq!(delete_prompt("api", 0), None);
        assert_eq!(
            delete_prompt("api", 1).unwrap(),
            "Delete the group “api” and close its workspace?"
        );
        assert_eq!(
            delete_prompt("api", 3).unwrap(),
            "Delete the group “api” and close its 3 workspaces?"
        );
    }
}
