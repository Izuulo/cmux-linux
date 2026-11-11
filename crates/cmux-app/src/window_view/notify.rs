//! Notifications in the window (plan M4 D3, D8): arrivals, cmux's read rules, reorder and the
//! jump to the latest unread. The rules themselves live in `cmux_core::Notifications`.

use std::collections::HashSet;
use std::time::SystemTime;

use cmux_core::{NotificationId, SurfaceId, WorkspaceId};
use gpui::{Context, Window};

use super::WindowView;
use crate::attention::is_focused_arrival;

impl WindowView {
    /// A new notification on `surface` (OSC or the socket). `desktop`: post a desktop bubble
    /// unless it is a focused arrival (`cmux notify --desktop false` passes false, M5 plan D7).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn on_notify(
        &mut self,
        ws: WorkspaceId,
        surface: SurfaceId,
        title: String,
        subtitle: String,
        body: String,
        desktop: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> NotificationId {
        let title = if title.trim().is_empty() {
            self.title_of(ws)
        } else {
            title
        };
        let mut payload = serde_json::Map::new();
        payload.insert("title".into(), title.as_str().into());
        payload.insert("subtitle".into(), subtitle.as_str().into());
        payload.insert("body".into(), body.as_str().into());
        self.emit("notification.created", Some(ws), Some(surface), payload, cx);
        let selected = ws == self.workspaces.selected();
        let focused = is_focused_arrival(
            window.is_window_active(),
            selected,
            self.active_surface(ws, cx) == Some(surface),
        );
        let added = self.notifications.add(
            ws,
            surface,
            title,
            subtitle,
            body,
            focused,
            SystemTime::now(),
        );
        #[cfg(not(target_os = "linux"))]
        let _ = desktop;
        #[cfg(target_os = "linux")]
        if !focused && desktop && self.desktop.is_some() {
            // The new bubble reuses the replaced one's (replaces_id): don't close it.
            for old in &added.replaced {
                self.desktop_ids.retire(*old);
            }
            if let Some(replaces) = self.desktop_ids.want(added.id, ws, surface) {
                self.post_desktop(added.id, ws, surface, replaces, cx);
            }
        }
        if !focused {
            // rn §6 app.reorderOnNotification: the workspace moves to the top.
            self.workspaces.bump_to_top(ws);
            if selected {
                self.flash(ws, surface, cx);
            }
        }
        self.after_change(added.replaced, cx);
        added.id
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
        // Target tab first: selecting the workspace reads its active surface, which must be
        // this one, not whichever pane was active there before.
        view.update(cx, |v, cx| v.focus_surface(surface, window, cx));
        if ws != self.workspaces.selected() {
            self.select(ws, window, cx);
        }
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
    pub(super) fn after_change(&mut self, gone: Vec<NotificationId>, cx: &mut Context<Self>) {
        #[cfg(target_os = "linux")]
        self.withdraw_desktop(gone, cx);
        #[cfg(not(target_os = "linux"))]
        let _ = gone;
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

#[cfg(target_os = "linux")]
use crate::desktop_notify::Next;

/// Linux desktop bubbles (plan M4 D9).
#[cfg(target_os = "linux")]
impl WindowView {
    /// Connects to the notification server and follows its clicks for the window's lifetime.
    pub(super) fn desktop_loop(window: &mut Window, cx: &mut Context<Self>) -> gpui::Task<()> {
        use futures::StreamExt;

        use crate::desktop_notify::{DesktopNotifier, Signal};

        cx.spawn_in(window, async move |this, cx| {
            let Some(notifier) = DesktopNotifier::connect(None).await else {
                return;
            };
            let notifier = std::rc::Rc::new(notifier);
            let Some(mut signals) = notifier.signals().await else {
                return;
            };
            if this
                .update(cx, |this, _| this.desktop = Some(notifier.clone()))
                .is_err()
            {
                return;
            }
            while let Some(signal) = signals.next().await {
                let alive = this.update_in(cx, |this, window, cx| match signal {
                    Signal::Activated { server, token } => {
                        let Some(id) = this.desktop_ids.forget_server(server) else {
                            return;
                        };
                        let raised =
                            token.is_some_and(|t| crate::activation::activate(window, cx, &t));
                        if !raised {
                            window.activate_window();
                        }
                        this.jump_to(id, window, cx);
                    }
                    Signal::Closed(server) => {
                        this.desktop_ids.forget_server(server);
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        })
    }

    /// Posts `id`'s bubble; when the reply is in, posts whatever queued up behind it.
    fn post_desktop(
        &mut self,
        id: NotificationId,
        ws: WorkspaceId,
        surface: SurfaceId,
        replaces: u32,
        cx: &mut Context<Self>,
    ) {
        let (Some(notifier), Some(n)) = (self.desktop.clone(), self.notifications.get(id)) else {
            return;
        };
        let (summary, body) = (n.title.clone(), n.desktop_body());
        cx.spawn(async move |this, cx| {
            let server = notifier.post(&summary, &body, replaces).await;
            let _ = this.update(cx, |this, cx| {
                let store = &this.notifications;
                let alive = |q| store.get(q).is_some_and(|n| !n.read);
                match this.desktop_ids.posted(id, ws, surface, server, alive) {
                    Next::Nothing => {}
                    Next::Post { id, replaces } => this.post_desktop(id, ws, surface, replaces, cx),
                    Next::Close(server) => {
                        cx.spawn(async move |_, _| notifier.close(server).await)
                            .detach();
                    }
                }
            });
        })
        .detach();
    }

    /// Read or removed notifications lose their bubbles (rn §9).
    fn withdraw_desktop(&mut self, gone: Vec<NotificationId>, cx: &mut Context<Self>) {
        let Some(notifier) = self.desktop.clone() else {
            return;
        };
        let servers: Vec<u32> = gone
            .into_iter()
            .filter_map(|id| self.desktop_ids.forget(id))
            .collect();
        if servers.is_empty() {
            return;
        }
        cx.spawn(async move |_, _| {
            for s in servers {
                notifier.close(s).await;
            }
        })
        .detach();
    }
}
