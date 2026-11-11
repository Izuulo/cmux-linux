//! `workspace.auto_name` on the socket (M9 plan N2): an agent's `Stop` hook reports a finished
//! turn; the app runs the naming pass on the background pool and applies the title.

use std::path::PathBuf;
use std::time::Instant;

use cmux_core::{SurfaceId, WorkspaceId};
use cmux_ipc::snapshot::Snapshot;
use cmux_ipc::wire::{Reply, RpcError};
use cmux_meta::autoname::{self, NamingState, Pass};
use gpui::Context;
use serde_json::{Map, Value, json};

use super::WindowView;

/// One workspace's throttle state (plan N4). Not persisted.
#[derive(Default)]
pub(super) struct Naming {
    named_lines: Option<usize>,
    last_attempt: Option<Instant>,
    in_flight: bool,
}

impl WindowView {
    pub(super) fn auto_name_from_socket(
        &mut self,
        snap: &Snapshot,
        params: &Map<String, Value>,
        cx: &mut Context<Self>,
    ) -> Reply {
        let text = |key| params.get(key).and_then(Value::as_str);
        let Some(t) = text("surface_id").and_then(|h| snap.find_surface(h)) else {
            return Err(RpcError::new(
                "invalid_params",
                "Missing or invalid surface_id",
            ));
        };
        let Some(path) = text("transcript_path").filter(|p| !p.is_empty()) else {
            return Err(RpcError::new("invalid_params", "Missing transcript_path"));
        };
        let ws = WorkspaceId(t.ws.key);
        // The turn's end is an event whether or not naming is on (plan A2).
        let mut payload = Map::new();
        payload.insert("source".into(), "claude".into());
        payload.insert("transcript_path".into(), path.into());
        self.emit(
            "agent.hook.Stop",
            Some(ws),
            Some(SurfaceId(t.surface.key)),
            payload,
            cx,
        );
        let queued = self.start_naming(ws, PathBuf::from(path), cx);
        Ok(json!({ "queued": queued }))
    }

    /// False when the user named the workspace or a pass is already running. Whether naming is
    /// switched on is read with the pass, off the UI thread.
    fn start_naming(
        &mut self,
        ws: WorkspaceId,
        transcript: PathBuf,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.workspaces.accepts_auto_title(ws) {
            return false;
        }
        let Some((dock, _)) = super::dock::dock_config_path() else {
            return false;
        };
        let config = dock.with_file_name("config.toml");
        let n = self.naming.entry(ws).or_default();
        if n.in_flight {
            return false;
        }
        n.in_flight = true;
        let state = NamingState {
            named_lines: n.named_lines,
            since_attempt: n.last_attempt.map(|t| t.elapsed()),
        };
        let current = self.workspaces.custom_title(ws).map(str::to_string);
        cx.spawn(async move |this, cx| {
            let pass = cx
                .background_executor()
                .spawn(async move {
                    let setting = cmux_config::auto_naming::load(&config);
                    if !setting.enabled {
                        return Pass::Skipped;
                    }
                    autoname::pass(&setting.command, &transcript, state, current.as_deref())
                })
                .await;
            let _ = this.update(cx, |s, cx| s.naming_done(ws, pass, cx));
        })
        .detach();
        true
    }

    fn naming_done(&mut self, ws: WorkspaceId, pass: Pass, cx: &mut Context<Self>) {
        // Closed meanwhile: nothing to name, nothing to remember.
        let Some(n) = self.naming.get_mut(&ws) else {
            return;
        };
        n.in_flight = false;
        match pass {
            Pass::Skipped => {}
            Pass::Reseed { lines } => n.named_lines = Some(lines),
            Pass::Ran { lines, title } => {
                n.last_attempt = Some(Instant::now());
                // The user may have named it while the command ran: theirs stays.
                if let Some(title) = title
                    && self.workspaces.set_auto_title(ws, &title)
                {
                    n.named_lines = Some(lines);
                    cx.notify();
                }
            }
        }
    }
}
