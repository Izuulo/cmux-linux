//! The control socket in the app (M5 plan D1-D8): the server, the `CMUX_*` environment of
//! every shell, and the answers to socket requests.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use cmux_core::{PaneId, SurfaceId, WorkspaceId};
use cmux_ipc::handles::{Handles, Kind};
use cmux_ipc::paths;
use cmux_ipc::server::{Access, DEADLINE, Request, Server};
use cmux_ipc::snapshot::{PaneSnap, Snapshot, SurfSnap, WsSnap};
use cmux_ipc::wire::{Reply, RpcError};
use futures::channel::mpsc::UnboundedReceiver;
use gpui::{App, Context, Window};
use serde_json::{Map, Value, json};

use super::WindowView;

/// What a new shell needs to find the app: shared by the window and its workspaces.
pub(crate) struct ShellEnv {
    pub handles: RefCell<Handles>,
    pub socket: Option<PathBuf>,
    cli_dir: Option<PathBuf>,
    path: Option<String>,
}

impl ShellEnv {
    pub fn new(socket: Option<PathBuf>) -> Self {
        // Our `cmux` sits next to the app binary (cargo's target dir, or an install's bin dir).
        let cli_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .filter(|dir| dir.join("cmux").is_file());
        Self {
            handles: RefCell::new(Handles::new()),
            socket,
            cli_dir,
            path: std::env::var("PATH").ok(),
        }
    }

    /// The env vars for a shell in `ws`/`surface` (minting their handles).
    pub fn for_surface(&self, ws: WorkspaceId, surface: SurfaceId) -> Vec<(String, String)> {
        let mut h = self.handles.borrow_mut();
        let w = h.handle(Kind::Workspace, (ws.0, 0));
        let s = h.handle(Kind::Surface, (ws.0, surface.0));
        cmux_ipc::env::terminal_env(
            &w.id,
            &s.id,
            self.socket.as_deref(),
            self.cli_dir.as_deref(),
            self.path.as_deref(),
        )
    }
}

/// Starts the socket server (plan D1, D3); `None` when it is off or can't start, after one
/// stderr line.
pub(super) fn start_server() -> Option<(Server, UnboundedReceiver<Request>)> {
    let env = |k: &str| std::env::var(k).ok();
    let mode = env("CMUX_SOCKET_MODE");
    let password_file = paths::password_file(&env);
    let access = Access::from_env(mode.as_deref(), password_file.as_deref())?;
    let Some(path) = paths::app_socket(&env) else {
        eprintln!(
            "cmux-linux: socket: neither HOME nor XDG_STATE_HOME is set; the cmux CLI is off"
        );
        return None;
    };
    let started = match Server::start(&path, access.clone(), DEADLINE) {
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            // Another instance owns the default socket: this one gets its own.
            let own = path.with_file_name(format!("cmux-{}.sock", std::process::id()));
            eprintln!(
                "cmux-linux: socket: {} is in use; this window uses {}",
                path.display(),
                own.display()
            );
            Server::start(&own, access, DEADLINE)
        }
        other => other,
    };
    match started {
        Ok(started) => Some(started),
        Err(e) => {
            eprintln!("cmux-linux: socket: {e}; the cmux CLI is off");
            None
        }
    }
}

fn require(params: &Map<String, Value>, key: &str) -> Result<(), RpcError> {
    match params.get(key) {
        Some(Value::String(s)) if !s.is_empty() => Ok(()),
        _ => Err(RpcError::new(
            "invalid_params",
            format!("Missing or invalid {key}"),
        )),
    }
}

impl WindowView {
    /// The window model as the socket sees it.
    pub(super) fn snapshot(&self, cx: &App) -> Snapshot {
        let mut h = self.shell.handles.borrow_mut();
        let window = h.handle(Kind::Window, (0, 0));
        let workspaces: Vec<WsSnap> = self
            .workspaces
            .list()
            .iter()
            .filter_map(|w| {
                let view = self.views.get(&w.id)?.read(cx);
                let layout = view.layout();
                let ws = w.id.0;
                let panes: Vec<PaneSnap> = layout
                    .panes()
                    .into_iter()
                    .map(|p| PaneSnap {
                        key: p.id.0,
                        handle: h.handle(Kind::Pane, (ws, p.id.0)),
                        selected: p.selected,
                        surfaces: p
                            .tabs
                            .iter()
                            .map(|s| SurfSnap {
                                key: s.0,
                                handle: h.handle(Kind::Surface, (ws, s.0)),
                                title: view
                                    .terminal(*s)
                                    .and_then(|t| t.read(cx).title().map(str::to_string))
                                    .unwrap_or_else(|| "Terminal".into()),
                            })
                            .collect(),
                    })
                    .collect();
                let focused = layout.focused().0;
                Some(WsSnap {
                    key: ws,
                    handle: h.handle(Kind::Workspace, (ws, 0)),
                    title: self.title_of(w.id),
                    custom_title: w.custom_title.clone(),
                    cwd: view.active_cwd(cx),
                    ports: self
                        .meta
                        .get(&w.id)
                        .map(|m| m.ports.clone())
                        .unwrap_or_default(),
                    focused_pane: panes.iter().position(|p| p.key == focused).unwrap_or(0),
                    panes,
                })
            })
            .collect();
        let selected = workspaces
            .iter()
            .position(|w| w.key == self.workspaces.selected().0)
            .unwrap_or(0);
        Snapshot {
            window,
            selected,
            workspaces,
        }
    }

    /// Answers one socket request (everything but ping/auth, which the server answers).
    pub(super) fn control(
        &mut self,
        method: &str,
        params: &Map<String, Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Reply {
        let snap = self.snapshot(cx);
        snap.check_window(params)?;
        if let Some(reply) = snap.read(method, params) {
            return reply;
        }
        match method {
            "workspace.select" => {
                require(params, "workspace_id")?;
                let ws = snap.workspace(params)?;
                self.select(WorkspaceId(ws.key), window, cx);
                Ok(snap.with_window(json!({
                    "workspace_id": ws.handle.id, "workspace_ref": ws.handle.r#ref,
                })))
            }
            "pane.focus" => {
                require(params, "pane_id")?;
                let ws = snap.workspace(params)?;
                let pane = snap.pane(ws, params)?;
                let id = WorkspaceId(ws.key);
                if let Some(view) = self.views.get(&id).cloned() {
                    view.update(cx, |v, cx| v.focus_pane(PaneId(pane.key), window, cx));
                }
                if id != self.workspaces.selected() {
                    self.select(id, window, cx);
                }
                Ok(snap.with_window(json!({
                    "workspace_id": ws.handle.id, "workspace_ref": ws.handle.r#ref,
                    "pane_id": pane.handle.id, "pane_ref": pane.handle.r#ref,
                })))
            }
            "surface.focus" => {
                require(params, "surface_id")?;
                let t = snap.surface(params)?;
                let id = WorkspaceId(t.ws.key);
                // Target tab first, as in `jump_to`: selecting reads the active surface.
                if let Some(view) = self.views.get(&id).cloned() {
                    view.update(cx, |v, cx| {
                        v.focus_surface(SurfaceId(t.surface.key), window, cx)
                    });
                }
                if id != self.workspaces.selected() {
                    self.select(id, window, cx);
                }
                Ok(snap.target_payload(&t))
            }
            _ => Err(RpcError::new("method_not_found", "Unknown method")),
        }
    }
}
