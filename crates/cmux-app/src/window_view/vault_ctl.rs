//! Agent sessions on the socket (M10 plan V1, V4): hooks report Claude's session per surface;
//! `vault.list` shows what would resume after a restart, `vault.resume` types it again.

use cmux_config::vault::{self, CLAUDE, Registration};
use cmux_core::{SurfaceId, WorkspaceId};
use cmux_ipc::snapshot::Snapshot;
use cmux_ipc::wire::{Reply, RpcError};
use gpui::Context;
use serde_json::{Map, Value, json};

use super::WindowView;
use super::session::{Agent, SavedTerm, detect_agent};

fn invalid(msg: &str) -> RpcError {
    RpcError::new("invalid_params", msg)
}

/// `[[vault.agents]]`; warnings go to stderr.
pub(super) fn load() -> Vec<Registration> {
    let Some((dock, _)) = super::dock::dock_config_path() else {
        return Vec::new();
    };
    let (regs, warnings) = vault::load(&dock.with_file_name("config.toml"));
    for w in warnings {
        eprintln!("cmux-linux: config: {w}");
    }
    regs
}

impl WindowView {
    pub(super) fn vault_from_socket(
        &mut self,
        snap: &Snapshot,
        method: &str,
        params: &Map<String, Value>,
        cx: &mut Context<Self>,
    ) -> Reply {
        match method {
            "vault.list" => Ok(json!({ "sessions": self.vault_rows(snap, cx) })),
            "surface.agent_session" | "vault.resume" => {
                if !matches!(params.get("surface_id"), Some(Value::String(s)) if !s.is_empty()) {
                    return Err(invalid("Missing or invalid surface_id"));
                }
                let t = snap.surface(params)?;
                if t.surface.browser {
                    return Err(invalid("Surface is not a terminal"));
                }
                let (ws, surface) = (WorkspaceId(t.ws.key), SurfaceId(t.surface.key));
                let reply = snap.target_payload(&t);
                let Some(view) = self.views.get(&ws).cloned() else {
                    return Err(RpcError::new("not_found", "Surface not found"));
                };
                if method == "vault.resume" {
                    let line = self
                        .agent_term(ws, surface, cx)
                        .and_then(|t| t.resume_line(&self.vault_agents));
                    let Some(line) = line else {
                        return Err(RpcError::new(
                            "not_found",
                            "No agent session on that surface",
                        ));
                    };
                    if let Some(term) = view.read(cx).terminal(surface).cloned() {
                        let bytes = format!("{line}\n").into_bytes();
                        term.update(cx, |v, cx| v.send_scripted(bytes, cx));
                    }
                    return Ok(reply);
                }
                let agent = self.agent_from_params(params)?;
                view.update(cx, |v, _| {
                    let keep = agent.is_none()
                        && params
                            .get("session_id")
                            .and_then(Value::as_str)
                            .is_some_and(|id| v.agent(surface).is_some_and(|a| a.session_id != id));
                    // A late end of an older session leaves the newer one alone.
                    if !keep {
                        v.set_agent(surface, agent);
                    }
                });
                Ok(reply)
            }
            _ => Err(RpcError::new("method_not_found", "Unknown method")),
        }
    }

    /// `{kind, session_id}` sets; `{clear: true[, session_id]}` clears.
    fn agent_from_params(&self, params: &Map<String, Value>) -> Result<Option<Agent>, RpcError> {
        if params.get("clear").and_then(Value::as_bool) == Some(true) {
            return Ok(None);
        }
        let s = |k: &str| params.get(k).and_then(Value::as_str);
        let kind = s("kind").unwrap_or(CLAUDE);
        if kind != CLAUDE && !self.vault_agents.iter().any(|r| r.id == kind) {
            return Err(invalid("Unknown agent kind"));
        }
        let Some(id) = s("session_id").filter(|id| vault::valid_session_id(id)) else {
            return Err(invalid("Missing or invalid session_id"));
        };
        Ok(Some(Agent {
            kind: kind.to_string(),
            session_id: id.to_string(),
            executable: None,
        }))
    }

    /// The surface's cwd and agent: the hook-reported one, else a registered process.
    fn agent_term(&self, ws: WorkspaceId, surface: SurfaceId, cx: &gpui::App) -> Option<SavedTerm> {
        let view = self.views.get(&ws)?.read(cx);
        let term = view.terminal(surface)?.read(cx);
        let agent = view
            .agent(surface)
            .cloned()
            .or_else(|| detect_agent(&self.vault_agents, term.foreground_pid()?))?;
        Some(SavedTerm {
            cwd: term.cwd(),
            agent: Some(agent),
            url: None,
        })
    }

    fn vault_rows(&self, snap: &Snapshot, cx: &gpui::App) -> Vec<Value> {
        let mut rows = Vec::new();
        for ws in &snap.workspaces {
            for s in ws.panes.iter().flat_map(|p| &p.surfaces) {
                let Some(t) = self.agent_term(WorkspaceId(ws.key), SurfaceId(s.key), cx) else {
                    continue;
                };
                let a = t.agent.as_ref().expect("agent_term sets it");
                let name = self.vault_agents.iter().find(|r| r.id == a.kind).map_or(
                    if a.kind == cmux_ipc::local_tmux::KIND {
                        "tmux session"
                    } else {
                        "Claude Code"
                    },
                    |r| r.name.as_str(),
                );
                rows.push(json!({
                    "workspace_id": ws.handle.id, "workspace_ref": ws.handle.r#ref,
                    "workspace_title": ws.title,
                    "surface_id": s.handle.id, "surface_ref": s.handle.r#ref,
                    "agent": a.kind, "agent_name": name, "session_id": a.session_id,
                    "cwd": t.cwd, "resume_command": t.resume_line(&self.vault_agents),
                }));
            }
        }
        rows
    }
}
