//! Desktop notifications (plan M4 D9, rn §9): freedesktop `org.freedesktop.Notifications` over
//! the session bus. The id bookkeeping and text rules are plain code; the D-Bus client is Linux
//! only.

use std::collections::HashMap;

use cmux_core::{NotificationId, SurfaceId, WorkspaceId};

/// Which desktop bubble (server id) shows which notification. One bubble per surface: a new
/// notification there replaces the old bubble (`replaces_id`), as cmux withdraws it (rn §9).
#[derive(Debug, Default)]
pub struct DesktopIds {
    by_local: HashMap<NotificationId, (u32, WorkspaceId, SurfaceId)>,
    by_server: HashMap<u32, NotificationId>,
    by_surface: HashMap<(WorkspaceId, SurfaceId), u32>,
}

impl DesktopIds {
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

    #[test]
    fn body_is_escaped_only_with_markup() {
        assert_eq!(body_text("a<b>&c", true), "a&lt;b&gt;&amp;c");
        assert_eq!(body_text("a<b>&c", false), "a<b>&c");
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
