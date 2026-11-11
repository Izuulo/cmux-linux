//! `agent.message.*` on the socket (M9 plan D2, cmux `docs/agent-messages.md`): the app keeps
//! the store; agents' hooks claim their mail. Nothing is typed into a terminal.

use std::time::{SystemTime, UNIX_EPOCH};

use cmux_core::{AgentMessage, MessageState, Recipient, SurfaceId, WorkspaceId, render_messages};
use cmux_ipc::handles::{Kind, new_uuid};
use cmux_ipc::snapshot::Snapshot;
use cmux_ipc::wire::{Reply, RpcError};
use gpui::Context;
use serde_json::{Map, Value, json};

use super::WindowView;

const LIST_DEFAULT: usize = 50;

fn err(code: &str, msg: &str) -> RpcError {
    RpcError::new(code, msg)
}

fn text<'a>(params: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    params
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

/// The terminal a surface handle names, if it is open.
fn surface(snap: &Snapshot, handle: Option<&str>) -> Option<Recipient> {
    let t = snap.find_surface(handle?)?;
    Some((WorkspaceId(t.ws.key), SurfaceId(t.surface.key)))
}

impl WindowView {
    fn message_payload(&self, m: &AgentMessage) -> Value {
        let mut h = self.shell.handles.borrow_mut();
        let mut ids = |r: Recipient| {
            (
                h.handle(Kind::Workspace, (r.0.0, 0)),
                h.handle(Kind::Surface, (r.0.0, r.1.0)),
            )
        };
        let (ws, s) = ids(m.to);
        let sender = m.sender.map(&mut ids);
        json!({
            "id": m.id, "thread_id": m.thread_id, "reply_to": m.reply_to,
            "from": m.from, "body": m.body, "state": m.state.name(),
            "created_at_ms": m.created_ms,
            "recipient_workspace_id": ws.id, "recipient_workspace_ref": ws.r#ref,
            "recipient_surface_id": s.id, "recipient_surface_ref": s.r#ref,
            "sender_surface_id": sender.as_ref().map(|(_, s)| &s.id),
            "sender_surface_ref": sender.as_ref().map(|(_, s)| &s.r#ref),
        })
    }

    pub(super) fn message_from_socket(
        &mut self,
        snap: &Snapshot,
        method: &str,
        params: &Map<String, Value>,
        cx: &mut Context<Self>,
    ) -> Reply {
        match method {
            "agent.message.send" => self.message_send(snap, params, cx),
            "agent.message.list" => {
                let to = match text(params, "surface") {
                    None => None,
                    Some(target) => Some(self.message_recipient(snap, target)?),
                };
                let states: Vec<MessageState> = match params.get("state") {
                    None | Some(Value::Null) => Vec::new(),
                    Some(Value::String(s)) => vec![s.as_str()],
                    Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
                    Some(_) => return Err(err("invalid_params", "Invalid state")),
                }
                .into_iter()
                .map(|s| {
                    MessageState::parse(s).ok_or_else(|| err("invalid_params", "Invalid state"))
                })
                .collect::<Result<_, _>>()?;
                let limit = params
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map_or(LIST_DEFAULT, |n| n as usize);
                let rows: Vec<Value> = self
                    .messages
                    .list(to, &states, limit)
                    .into_iter()
                    .map(|m| self.message_payload(m))
                    .collect();
                Ok(json!({ "messages": rows }))
            }
            "agent.message.claim" => {
                let Some(to) = surface(snap, text(params, "surface_id")) else {
                    return Err(err("invalid_params", "Missing or invalid surface_id"));
                };
                // The recipient's turn ended: what it was shown before is read now.
                if params.get("mark_delivered_read").and_then(Value::as_bool) == Some(true) {
                    self.messages.mark_delivered_read(to);
                }
                let got = self.messages.claim(to);
                let rows: Vec<Value> = got.iter().map(|m| self.message_payload(m)).collect();
                Ok(json!({ "messages": rows, "text": render_messages(&got) }))
            }
            "agent.message.mark_read" => {
                let read = if let Some(to) = surface(snap, text(params, "surface_id")) {
                    self.messages.mark_delivered_read(to)
                } else {
                    let ids: Vec<&str> = match params.get("ids") {
                        Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
                        _ => text(params, "id").into_iter().collect(),
                    };
                    if ids.is_empty() {
                        return Err(err("invalid_params", "Missing ids, id or surface_id"));
                    }
                    self.messages.mark_read(&ids)
                };
                Ok(json!({ "read": read }))
            }
            _ => Err(err("method_not_found", "Unknown method")),
        }
    }

    /// A target's terminal: the surface named, or the workspace's agent (the surface whose hook
    /// last claimed messages), else its focused surface (plan D4).
    fn message_recipient(&self, snap: &Snapshot, target: &str) -> Result<Recipient, RpcError> {
        let (ws, t) = snap.message_target(target)?;
        let id = WorkspaceId(ws.key);
        if let Some(t) = t {
            return Ok((id, SurfaceId(t.surface.key)));
        }
        let open = |s: &SurfaceId| {
            ws.panes
                .iter()
                .flat_map(|p| &p.surfaces)
                .any(|x| x.key == s.0)
        };
        let agent = self.messages.agent_surface(id).filter(open);
        Ok((id, agent.unwrap_or(SurfaceId(ws.focused().surface.key))))
    }

    fn message_send(
        &mut self,
        snap: &Snapshot,
        params: &Map<String, Value>,
        cx: &mut Context<Self>,
    ) -> Reply {
        let Some(body) = params.get("body").and_then(Value::as_str) else {
            return Err(err("invalid_params", "Missing body"));
        };
        let sender = surface(snap, text(params, "sender_surface_id"));
        let reply_to = text(params, "reply_to");
        let to = match (text(params, "target"), reply_to) {
            (Some(target), _) => self.message_recipient(snap, target)?,
            (None, Some(id)) => {
                let Some(original) = self.messages.get(id) else {
                    return Err(err("not_found", "Message not found"));
                };
                // Back to where it came from, if that terminal is still open.
                let back = original.sender.filter(|(ws, s)| {
                    snap.workspaces.iter().any(|w| {
                        w.key == ws.0
                            && w.panes
                                .iter()
                                .flat_map(|p| &p.surfaces)
                                .any(|x| x.key == s.0)
                    })
                });
                back.ok_or_else(|| err("invalid_state", "The message's sender is gone"))?
            }
            (None, None) => return Err(err("invalid_params", "Missing target or reply_to")),
        };
        // `--from` defaults to the sending workspace's title (am §1).
        let from = match (text(params, "from"), sender) {
            (Some(f), _) => f.to_string(),
            (None, Some((ws, _))) => self.title_of(ws).chars().take(64).collect(),
            (None, None) => "cmux".to_string(),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);
        let sent = self
            .messages
            .send(new_uuid(), &from, body, to, sender, reply_to, now)
            .map_err(|e| err("invalid_params", e.text()))?
            .clone();
        cx.notify();
        let mut reply = self.message_payload(&sent);
        reply["recipient_workspace_title"] = json!(self.title_of(to.0));
        reply["recipient_has_agent"] = json!(self.messages.agent_surface(to.0) == Some(to.1));
        Ok(reply)
    }
}
