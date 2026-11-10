//! Desktop notifications (plan M4 D9, rn §9): freedesktop `org.freedesktop.Notifications` over
//! the session bus. The id bookkeeping and text rules are plain code; the D-Bus client is Linux
//! only.

use std::collections::{HashMap, HashSet};

use cmux_core::{NotificationId, SurfaceId, WorkspaceId};

/// What to do once a post's reply is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    Nothing,
    /// Post the newest notification that queued up behind the reply.
    Post {
        id: NotificationId,
        replaces: u32,
    },
    /// The posted notification was read or removed meanwhile: take its bubble down.
    Close(u32),
}

/// Which desktop bubble (server id) shows which notification. One bubble per surface: a new
/// notification there reuses the old bubble (`replaces_id`), as cmux withdraws it (rn §9).
/// At most one post per surface is out at a time; newer ones wait, and only the newest is
/// posted when the reply comes back, so a flood of OSCs is a handful of D-Bus calls.
#[derive(Debug, Default)]
pub struct DesktopIds {
    by_local: HashMap<NotificationId, (u32, WorkspaceId, SurfaceId)>,
    by_server: HashMap<u32, NotificationId>,
    by_surface: HashMap<(WorkspaceId, SurfaceId), u32>,
    in_flight: HashSet<(WorkspaceId, SurfaceId)>,
    queued: HashMap<(WorkspaceId, SurfaceId), NotificationId>,
}

impl DesktopIds {
    /// `id` wants a bubble. Some(replaces_id) = post it now; None = it waits for the reply
    /// of the post already out for this surface.
    pub fn want(&mut self, id: NotificationId, ws: WorkspaceId, s: SurfaceId) -> Option<u32> {
        if !self.in_flight.insert((ws, s)) {
            self.queued.insert((ws, s), id);
            return None;
        }
        Some(self.replaces(ws, s))
    }

    /// The reply to `id`'s post (`server` None = it failed). `alive`: still unread in the store.
    pub fn posted(
        &mut self,
        id: NotificationId,
        ws: WorkspaceId,
        s: SurfaceId,
        server: Option<u32>,
        alive: impl Fn(NotificationId) -> bool,
    ) -> Next {
        self.in_flight.remove(&(ws, s));
        if let Some(server) = server {
            self.record(id, ws, s, server);
        }
        if let Some(q) = self.queued.remove(&(ws, s))
            && alive(q)
        {
            self.retire(id);
            self.in_flight.insert((ws, s));
            return Next::Post {
                id: q,
                replaces: self.replaces(ws, s),
            };
        }
        if !alive(id)
            && let Some(server) = self.forget(id)
        {
            return Next::Close(server);
        }
        Next::Nothing
    }

    /// `id` was replaced by a newer notification on its surface that reuses its bubble: drop
    /// the mapping but keep the surface's bubble.
    pub fn retire(&mut self, id: NotificationId) {
        if let Some((server, _, _)) = self.by_local.remove(&id)
            && self.by_server.get(&server) == Some(&id)
        {
            self.by_server.remove(&server);
        }
    }

    /// The bubble to replace for a new notification on this surface; 0 = a new bubble.
    pub fn replaces(&self, ws: WorkspaceId, s: SurfaceId) -> u32 {
        self.by_surface.get(&(ws, s)).copied().unwrap_or(0)
    }

    pub fn record(&mut self, id: NotificationId, ws: WorkspaceId, s: SurfaceId, server: u32) {
        if let Some(old) = self.by_server.insert(server, id)
            && old != id
        {
            self.by_local.remove(&old);
        }
        self.by_local.insert(id, (server, ws, s));
        self.by_surface.insert((ws, s), server);
    }

    pub fn server_of(&self, id: NotificationId) -> Option<u32> {
        self.by_local.get(&id).map(|e| e.0)
    }

    pub fn local_of(&self, server: u32) -> Option<NotificationId> {
        self.by_server.get(&server).copied()
    }

    /// Drops `id`'s bubble from the map; returns the server id to close.
    pub fn forget(&mut self, id: NotificationId) -> Option<u32> {
        self.queued.retain(|_, q| *q != id);
        let (server, ws, s) = self.by_local.remove(&id)?;
        if self.by_server.get(&server) == Some(&id) {
            self.by_server.remove(&server);
        }
        if self.by_surface.get(&(ws, s)) == Some(&server) {
            self.by_surface.remove(&(ws, s));
        }
        Some(server)
    }

    /// The server closed a bubble (expired, dismissed, clicked).
    pub fn forget_server(&mut self, server: u32) -> Option<NotificationId> {
        let id = self.local_of(server)?;
        self.forget(id);
        Some(id)
    }
}

/// The body as sent: servers that render markup get `& < >` escaped, others the plain text.
pub fn body_text(body: &str, markup: bool) -> String {
    if !markup {
        return body.to_string();
    }
    body.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(target_os = "linux")]
pub use dbus::{DesktopNotifier, Signal};

#[cfg(target_os = "linux")]
mod dbus {
    use std::collections::HashMap;

    use futures::{Stream, StreamExt, stream};
    use zbus::zvariant::Value;

    #[zbus::proxy(
        interface = "org.freedesktop.Notifications",
        default_service = "org.freedesktop.Notifications",
        default_path = "/org/freedesktop/Notifications"
    )]
    trait Notifications {
        #[allow(clippy::too_many_arguments)]
        fn notify(
            &self,
            app_name: &str,
            replaces_id: u32,
            app_icon: &str,
            summary: &str,
            body: &str,
            actions: &[&str],
            hints: HashMap<&str, Value<'_>>,
            expire_timeout: i32,
        ) -> zbus::Result<u32>;

        fn close_notification(&self, id: u32) -> zbus::Result<()>;

        fn get_capabilities(&self) -> zbus::Result<Vec<String>>;

        #[zbus(signal)]
        fn action_invoked(&self, id: u32, action_key: String) -> zbus::Result<()>;

        #[zbus(signal)]
        fn activation_token(&self, id: u32, activation_token: String) -> zbus::Result<()>;

        #[zbus(signal)]
        fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
    }

    /// What the notification server tells us about our bubbles.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Signal {
        /// The bubble was clicked; `token` is its xdg-activation token, if the server sent one.
        Activated {
            server: u32,
            token: Option<String>,
        },
        Closed(u32),
    }

    pub struct DesktopNotifier {
        proxy: NotificationsProxy<'static>,
        markup: bool,
    }

    fn off(e: impl std::fmt::Display) {
        eprintln!("cmux-linux: desktop notifications off: {e}");
    }

    impl DesktopNotifier {
        /// The session bus (`None`) or `address`; None, with one stderr line, if there is no
        /// bus or no notification server.
        pub async fn connect(address: Option<&str>) -> Option<Self> {
            let builder = match address {
                None => zbus::connection::Builder::session(),
                Some(a) => zbus::connection::Builder::address(a),
            };
            let conn = match builder {
                Ok(b) => b.build().await,
                Err(e) => Err(e),
            }
            .inspect_err(|e| off(e))
            .ok()?;
            let proxy = NotificationsProxy::new(&conn)
                .await
                .inspect_err(|e| off(e))
                .ok()?;
            let caps = proxy
                .get_capabilities()
                .await
                .inspect_err(|e| off(e))
                .ok()?;
            Some(Self {
                markup: caps.iter().any(|c| c == "body-markup"),
                proxy,
            })
        }

        /// Shows a bubble; returns its server id.
        pub async fn post(&self, summary: &str, body: &str, replaces: u32) -> Option<u32> {
            let hints = HashMap::from([
                ("desktop-entry", Value::from("cmux-linux")),
                ("sound-name", Value::from("message-new-instant")),
            ]);
            let body = super::body_text(body, self.markup);
            self.proxy
                .notify(
                    "cmux-linux",
                    replaces,
                    "",
                    summary,
                    &body,
                    &["default", "Open"],
                    hints,
                    -1,
                )
                .await
                .inspect_err(|e| eprintln!("cmux-linux: desktop notification failed: {e}"))
                .ok()
        }

        pub async fn close(&self, server: u32) {
            let _ = self.proxy.close_notification(server).await;
        }

        /// Clicks and closes. `ActivationToken` comes just before its `ActionInvoked`, so the
        /// token is paired with the click here.
        pub async fn signals(&self) -> Option<impl Stream<Item = Signal> + use<>> {
            enum Raw {
                Token(u32, String),
                Action(u32, String),
                Closed(u32),
            }
            let tokens = self.proxy.receive_activation_token().await.ok()?;
            let actions = self.proxy.receive_action_invoked().await.ok()?;
            let closed = self.proxy.receive_notification_closed().await.ok()?;
            let raw = stream::select_all([
                tokens
                    .filter_map(|s| async move {
                        let a = s.args().ok()?;
                        Some(Raw::Token(a.id, a.activation_token))
                    })
                    .boxed(),
                actions
                    .filter_map(|s| async move {
                        let a = s.args().ok()?;
                        Some(Raw::Action(a.id, a.action_key))
                    })
                    .boxed(),
                closed
                    .filter_map(|s| async move { Some(Raw::Closed(s.args().ok()?.id)) })
                    .boxed(),
            ]);
            Some(
                raw.scan(HashMap::<u32, String>::new(), |pending, r| {
                    let out = match r {
                        Raw::Token(id, t) => {
                            pending.insert(id, t);
                            None
                        }
                        Raw::Action(id, key) if key == "default" => Some(Signal::Activated {
                            server: id,
                            token: pending.remove(&id),
                        }),
                        Raw::Action(..) => None,
                        Raw::Closed(id) => {
                            pending.remove(&id);
                            Some(Signal::Closed(id))
                        }
                    };
                    futures::future::ready(Some(out))
                })
                .filter_map(futures::future::ready),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cmux_core::{NotificationId, SurfaceId, WorkspaceId};

    const W: WorkspaceId = WorkspaceId(1);
    const S: SurfaceId = SurfaceId(2);

    #[test]
    fn replaces_id_follows_the_surface() {
        let mut ids = DesktopIds::default();
        assert_eq!(ids.replaces(W, S), 0);
        ids.record(NotificationId(1), W, S, 40);
        assert_eq!(ids.replaces(W, S), 40);
        assert_eq!(ids.replaces(WorkspaceId(9), S), 0);
        ids.record(NotificationId(2), W, S, 40);
        assert_eq!(ids.server_of(NotificationId(2)), Some(40));
        assert_eq!(ids.local_of(40), Some(NotificationId(2)));
        assert_eq!(
            ids.server_of(NotificationId(1)),
            None,
            "replaced bubble is the new one"
        );
    }

    #[test]
    fn unknown_server_id_maps_to_nothing() {
        let ids = DesktopIds::default();
        assert_eq!(ids.local_of(7), None);
        assert_eq!(ids.server_of(NotificationId(7)), None);
    }

    #[test]
    fn forget_clears_both_directions() {
        let mut ids = DesktopIds::default();
        ids.record(NotificationId(1), W, S, 40);
        assert_eq!(ids.forget(NotificationId(1)), Some(40));
        assert_eq!(ids.local_of(40), None);
        assert_eq!(ids.replaces(W, S), 0);
        assert_eq!(ids.forget(NotificationId(1)), None);
        ids.record(NotificationId(3), W, S, 41);
        assert_eq!(ids.forget_server(41), Some(NotificationId(3)));
        assert_eq!(ids.server_of(NotificationId(3)), None);
    }

    const N1: NotificationId = NotificationId(1);
    const N2: NotificationId = NotificationId(2);
    const N3: NotificationId = NotificationId(3);

    #[test]
    fn replacement_keeps_the_bubble() {
        let mut ids = DesktopIds::default();
        assert_eq!(ids.want(N1, W, S), Some(0));
        assert_eq!(ids.posted(N1, W, S, Some(40), |_| true), Next::Nothing);
        // N2 replaces N1 in the store: N1's bubble is reused, not closed.
        ids.retire(N1);
        assert_eq!(ids.forget(N1), None);
        assert_eq!(ids.want(N2, W, S), Some(40));
        assert_eq!(ids.posted(N2, W, S, Some(40), |_| true), Next::Nothing);
        assert_eq!(ids.local_of(40), Some(N2));
        assert_eq!(ids.forget(N2), Some(40));
    }

    #[test]
    fn one_post_in_flight_per_surface() {
        let mut ids = DesktopIds::default();
        assert_eq!(ids.want(N1, W, S), Some(0));
        assert_eq!(ids.want(N2, W, S), None, "queued behind N1");
        assert_eq!(ids.want(N3, W, S), None, "replaces N2 in the queue");
        assert_eq!(
            ids.posted(N1, W, S, Some(40), |_| true),
            Next::Post {
                id: N3,
                replaces: 40
            }
        );
        assert_eq!(ids.posted(N3, W, S, Some(40), |_| true), Next::Nothing);
        assert_eq!(ids.local_of(40), Some(N3));
        assert_eq!(ids.server_of(N1), None);
    }

    #[test]
    fn posted_after_read_closes() {
        let mut ids = DesktopIds::default();
        ids.want(N1, W, S);
        assert_eq!(ids.posted(N1, W, S, Some(40), |_| false), Next::Close(40));
        assert_eq!(ids.replaces(W, S), 0);
        assert_eq!(ids.local_of(40), None);
    }

    #[test]
    fn queued_but_gone_is_dropped() {
        let mut ids = DesktopIds::default();
        ids.want(N1, W, S);
        ids.want(N2, W, S);
        assert_eq!(ids.posted(N1, W, S, Some(40), |_| false), Next::Close(40));
        assert_eq!(ids.want(N3, W, S), Some(0), "nothing in flight any more");
    }

    #[test]
    fn failed_post_frees_the_surface() {
        let mut ids = DesktopIds::default();
        ids.want(N1, W, S);
        assert_eq!(ids.posted(N1, W, S, None, |_| true), Next::Nothing);
        assert_eq!(ids.want(N2, W, S), Some(0));
    }

    #[test]
    fn body_is_escaped_only_with_markup() {
        assert_eq!(body_text("a<b>&c", true), "a&lt;b&gt;&amp;c");
        assert_eq!(body_text("a<b>&c", false), "a<b>&c");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn click_carries_the_token_sent_before_it() {
        use std::collections::HashMap;
        let sig = |member: &str, body: &(u32, &str)| {
            zbus::Message::signal(
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
                member,
            )
            .unwrap()
            .build(body)
            .unwrap()
        };
        let mut pending = HashMap::new();
        assert_eq!(pair(&mut pending, &sig("ActivationToken", &(7, "tok"))), None);
        assert_eq!(
            pair(&mut pending, &sig("ActionInvoked", &(7, "default"))),
            Some(Signal::Activated { server: 7, token: Some("tok".into()) })
        );
        assert_eq!(
            pair(&mut pending, &sig("ActionInvoked", &(8, "default"))),
            Some(Signal::Activated { server: 8, token: None })
        );
        assert_eq!(pair(&mut pending, &sig("ActionInvoked", &(8, "other"))), None);
        let closed = zbus::Message::signal(
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "NotificationClosed",
        )
        .unwrap()
        .build(&(9u32, 2u32))
        .unwrap();
        assert_eq!(pair(&mut pending, &closed), Some(Signal::Closed(9)));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn connect_without_bus_is_none() {
        let n = futures::executor::block_on(DesktopNotifier::connect(Some(
            "unix:path=/nonexistent/cmux-linux-test-bus",
        )));
        assert!(n.is_none());
    }
}
