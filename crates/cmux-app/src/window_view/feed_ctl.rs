//! Held `feed.push` requests (M7 plan D5, D6): each waits in `feed_waiters` until a reply, its
//! timer, or its workspace/surface closing ends it through `feed_finish`, which answers it once.

use std::time::{SystemTime, UNIX_EPOCH};

use cmux_core::{
    Decision, FeedItemId, FeedKind, FeedStatus, NewItem, NotificationId, SurfaceId, WorkspaceId,
};
use cmux_ipc::feed::{decision_json, parse_push};
use cmux_ipc::server::Request;
use gpui::{Context, Window};
use serde_json::{Value, json};

use super::WindowView;

pub(super) struct Waiter {
    req: Request,
    /// The D6 notification; removed when the card ends.
    notification: NotificationId,
}

pub(super) enum FeedEnd {
    Answer(Decision),
    TimedOut,
    /// The card's workspace or surface closed: the hook gets `{}` now, not at the wait's end.
    Closed,
}

/// cmux's Feed notification texts (D6): title and body.
fn attention_text(kind: &FeedKind) -> (&'static str, String) {
    match kind {
        FeedKind::Permission { tool, .. } if tool.is_empty() => {
            ("Claude permission", "Decision needed".into())
        }
        FeedKind::Permission { tool, .. } => {
            ("Claude permission", format!("{tool} needs approval"))
        }
        FeedKind::ExitPlan { .. } => ("Claude plan ready", "Review and approve the plan".into()),
        FeedKind::Question { .. } => ("Claude question", "Agent is asking a question".into()),
    }
}

impl WindowView {
    /// `feed.push`: answers at once, or holds `req` as a pending card.
    pub(super) fn feed_push(&mut self, req: Request, window: &mut Window, cx: &mut Context<Self>) {
        let snap = self.snapshot(cx);
        if let Err(e) = snap.check_window(&req.params) {
            return req.respond(Err(e));
        }
        let push = match parse_push(&req.params) {
            Ok(p) => p,
            Err(e) => return req.respond(Err(e)),
        };
        // Both ids must name one surface (a hook from a closed or foreign terminal: no card).
        let mut ids = serde_json::Map::new();
        ids.insert("workspace_id".into(), json!(push.workspace_id));
        ids.insert("surface_id".into(), json!(push.surface_id));
        let target = match (snap.workspace(&ids), snap.surface(&ids)) {
            (Ok(ws), Ok(t)) if t.ws.key == ws.key => Some((ws.key, t.surface.key)),
            _ => None,
        };
        let Some((ws, surface)) = target.map(|(w, s)| (WorkspaceId(w), SurfaceId(s))) else {
            return req.respond(Ok(json!({"status": "not_found"})));
        };
        if push.wait.is_zero() {
            return req.respond(Ok(json!({"status": "acknowledged"})));
        }
        let (title, body) = attention_text(&push.kind);
        let request_id = push.request_id.clone();
        let created_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);
        let mut payload = serde_json::Map::new();
        payload.insert("source".into(), push.source.as_str().into());
        payload.insert("request_id".into(), request_id.as_str().into());
        payload.insert("title".into(), title.into());
        self.emit("feed.item.received", Some(ws), Some(surface), payload, cx);
        let pushed = self.feed.push(NewItem {
            request_id: push.request_id,
            source: push.source,
            workspace: ws,
            surface,
            cwd: push.cwd,
            created_ms,
            kind: push.kind,
        });
        if pushed.is_none() {
            return req.respond(Ok(json!({"status": "unavailable"})));
        }
        let notification = self.on_notify(
            ws,
            surface,
            title.into(),
            String::new(),
            body,
            true,
            window,
            cx,
        );
        self.feed_waiters
            .insert(request_id.clone(), Waiter { req, notification });
        let wait = push.wait;
        // Detached: a timer outliving its answer finds no waiter and does nothing.
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(wait).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.feed_finish(&request_id, FeedEnd::TimedOut, cx);
                this.feed_refocus(window);
            });
        })
        .detach();
        cx.notify();
    }

    /// The one place a held request is answered; nothing when it already was.
    pub(super) fn feed_finish(&mut self, request_id: &str, end: FeedEnd, cx: &mut Context<Self>) {
        let Some(w) = self.feed_waiters.remove(request_id) else {
            return;
        };
        if let Some(id) = self.feed.items().find(|i| i.request_id == request_id) {
            let id = id.id;
            self.feed_feedback.remove(&id);
            self.feed_picks.remove(&id);
            if self.feed_editing == Some(id) {
                self.feed_editing = None;
            }
        }
        let reply: Value = match end {
            FeedEnd::Answer(d) => {
                let v = json!({"status": "resolved", "decision": decision_json(&d)});
                self.feed.resolve(request_id, d);
                v
            }
            FeedEnd::TimedOut => {
                self.feed.expire(request_id);
                json!({"status": "timed_out"})
            }
            FeedEnd::Closed => {
                // Already expired when `expire_workspace` got there first.
                self.feed.expire(request_id);
                json!({"status": "unavailable"})
            }
        };
        w.req.respond(Ok(reply));
        let gone = self.notifications.remove(w.notification);
        self.after_change(gone, cx);
    }

    /// After a card ends: if its feedback field had focus, focus the Feed list. Left on the
    /// vanished field, focus would fall to the agent's terminal (WorkspaceView's focus-lost
    /// listener), and the rest of the user's typing, Enter included, would answer Claude there.
    pub(super) fn feed_refocus(&self, window: &mut Window) {
        if self.feed_editing.is_none() && self.feed_field_focus.is_focused(window) {
            window.focus(&self.feed_focus);
        }
    }

    /// A Feed button: answers the card's request if it still waits and `decision` fits its kind.
    pub(super) fn feed_answer(
        &mut self,
        id: FeedItemId,
        decision: Decision,
        cx: &mut Context<Self>,
    ) {
        let item = self.feed.get(id).filter(|i| i.kind.accepts(&decision));
        if let Some(rid) = item.map(|i| i.request_id.clone()) {
            self.feed_finish(&rid, FeedEnd::Answer(decision), cx);
        }
    }

    /// Workspace `ws` closed: its waiting cards end now.
    pub(super) fn feed_close_workspace(&mut self, ws: WorkspaceId, cx: &mut Context<Self>) {
        for rid in self.feed.expire_workspace(ws) {
            self.feed_finish(&rid, FeedEnd::Closed, cx);
        }
    }

    /// A surface closed: its waiting cards end now.
    pub(super) fn feed_close_surface(
        &mut self,
        ws: WorkspaceId,
        surface: SurfaceId,
        cx: &mut Context<Self>,
    ) {
        let ids: Vec<String> = self
            .feed
            .items()
            .filter(|i| i.workspace == ws && i.surface == surface)
            .filter(|i| i.status == FeedStatus::Pending)
            .map(|i| i.request_id.clone())
            .collect();
        for rid in ids {
            self.feed_finish(&rid, FeedEnd::Closed, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use cmux_core::Preview;

    use super::*;

    #[test]
    fn attention_texts_match_cmux() {
        let perm = |tool: &str| FeedKind::Permission {
            tool: tool.into(),
            preview: Preview::Raw(String::new()),
        };
        assert_eq!(
            attention_text(&perm("Bash")),
            ("Claude permission", "Bash needs approval".into())
        );
        assert_eq!(
            attention_text(&perm("")),
            ("Claude permission", "Decision needed".into())
        );
        let plan = FeedKind::ExitPlan {
            plan: String::new(),
            allowed_prompts: vec![],
            plan_file: None,
        };
        assert_eq!(
            attention_text(&plan),
            ("Claude plan ready", "Review and approve the plan".into())
        );
        assert_eq!(
            attention_text(&FeedKind::Question { prompts: vec![] }),
            ("Claude question", "Agent is asking a question".into())
        );
    }
}
