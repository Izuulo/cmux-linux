//! Notifications in the window (plan M4 D3, D8): arrivals, cmux's read rules, reorder and the
//! jump to the latest unread. The rules themselves live in `cmux_core::Notifications`.

use std::collections::HashSet;
use std::time::SystemTime;

use cmux_core::{NotificationId, SurfaceId, WorkspaceId};
use gpui::{Context, Window};

use super::WindowView;
use crate::attention::is_focused_arrival;

impl WindowView {
    pub(super) fn on_notify(
        &mut self,
        ws: WorkspaceId,
        surface: SurfaceId,
        title: String,
        body: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = if title.trim().is_empty() {
            self.title_of(ws)
        } else {
            title
        };
        let selected = ws == self.workspaces.selected();
        let focused = is_focused_arrival(
            window.is_window_active(),
            selected,
            self.active_surface(ws, cx) == Some(surface),
        );
        let added = self
            .notifications
            .add(ws, surface, title, body, focused, SystemTime::now());
        if !focused {
            // rn §6 app.reorderOnNotification: the workspace moves to the top.
            self.workspaces.move_to(ws, 0);
            if selected {
                self.flash(ws, surface, cx);
            }
        }
        self.after_change(added.replaced, cx);
    }

    /// Focus reached a terminal: reading counts only in the selected workspace of an active
    /// window (rn §3 activeFocus).
    pub(super) fn on_focused(
        &mut self,
        ws: WorkspaceId,
        surface: SurfaceId,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if window.is_window_active() && ws == self.workspaces.selected() {
            self.read_surface(ws, surface, cx);
        }
    }

    /// Typing reads and clears the ring even while the window is inactive (rn §3).
    pub(super) fn on_typed(&mut self, ws: WorkspaceId, surface: SurfaceId, cx: &mut Context<Self>) {
        let before = self.notifications.generation();
        let read = self.notifications.typed(ws, surface);
        if !read.is_empty() {
            self.flash(ws, surface, cx);
        }
        if self.notifications.generation() != before {
            self.after_change(read, cx);
        }
    }

    pub(super) fn on_closed(
        &mut self,
        ws: WorkspaceId,
        surface: SurfaceId,
        cx: &mut Context<Self>,
    ) {
        let gone = self.notifications.remove_surface(ws, surface);
        if !gone.is_empty() {
            self.after_change(gone, cx);
        }
    }

    /// The selected workspace's active surface is now in front of the user: on selecting a
    /// workspace or the window becoming active (rn §3).
    pub(super) fn read_selected(&mut self, window: &Window, cx: &mut Context<Self>) {
        let ws = self.workspaces.selected();
        if window.is_window_active()
            && let Some(surface) = self.active_surface(ws, cx)
        {
            self.read_surface(ws, surface, cx);
        }
        self.push_attention(cx);
    }

    fn read_surface(&mut self, ws: WorkspaceId, surface: SurfaceId, cx: &mut Context<Self>) {
        let read = self.notifications.mark_surface_read(ws, surface);
        if !read.is_empty() {
            self.flash(ws, surface, cx);
            self.after_change(read, cx);
        }
    }

    /// Opens a notification's surface (panel click, jump, desktop click) and marks it read.
    /// False when its workspace or surface has gone.
    pub(crate) fn jump_to(
        &mut self,
        id: NotificationId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(n) = self.notifications.get(id) else {
            return false;
        };
        let (ws, surface) = (n.workspace, n.surface);
        let Some(view) = self.views.get(&ws).cloned() else {
            return false;
        };
        if view.read(cx).terminal(surface).is_none() {
            return false;
        }
        if ws != self.workspaces.selected() {
            self.select(ws, window, cx);
        }
        view.update(cx, |v, cx| v.focus_surface(surface, window, cx));
        let read = self.notifications.mark_read(id);
        self.flash(ws, surface, cx);
        self.after_change(read, cx);
        true
    }

    /// Super+Shift+U: the newest unread notification anywhere; nothing when none (rn §8).
    pub(super) fn jump_to_latest_unread(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.notifications.latest_unread().map(|n| n.id) {
            self.jump_to(id, window, cx);
        }
    }

    pub(super) fn mark_all_read(&mut self, cx: &mut Context<Self>) {
        let read = self.notifications.mark_all_read();
        self.after_change(read, cx);
    }

    pub(super) fn clear_all(&mut self, cx: &mut Context<Self>) {
        let gone = self.notifications.clear_all();
        self.after_change(gone, cx);
    }

    /// After any store change. `gone`: ids read or removed, whose desktop bubbles go (M4 D9).
    pub(super) fn after_change(&mut self, _gone: Vec<NotificationId>, cx: &mut Context<Self>) {
        self.push_attention(cx);
        cx.notify();
    }

    /// Ring state for the selected (the only drawn) workspace.
    pub(super) fn push_attention(&mut self, cx: &mut Context<Self>) {
        let ws = self.workspaces.selected();
        let Some(view) = self.views.get(&ws).cloned() else {
            return;
        };
        let ring: HashSet<SurfaceId> = view
            .read(cx)
            .layout()
            .surfaces()
            .into_iter()
            .filter(|s| self.notifications.shows_ring(ws, *s))
            .collect();
        view.update(cx, |v, cx| v.set_attention(ring, cx));
    }

    fn flash(&mut self, ws: WorkspaceId, surface: SurfaceId, cx: &mut Context<Self>) {
        if ws == self.workspaces.selected()
            && let Some(view) = self.views.get(&ws).cloned()
        {
            view.update(cx, |v, cx| v.flash(surface, cx));
        }
    }

    fn active_surface(&self, ws: WorkspaceId, cx: &Context<Self>) -> Option<SurfaceId> {
        Some(self.views.get(&ws)?.read(cx).layout().active_surface())
    }
}
