//! The control socket in the app (M5 plan D1-D8): the server, the `CMUX_*` environment of
//! every shell, and the answers to socket requests.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use cmux_core::{Direction, Notification, PaneId, SurfaceId, WorkspaceId};
use cmux_ipc::handles::{Handles, Kind};
use cmux_ipc::keys::{KeySpec, parse_key, sanitize_paste, typed_bytes};
use cmux_ipc::paths;
use cmux_ipc::server::{Access, DEADLINE, Request, Server};
use cmux_ipc::snapshot::{PaneSnap, Snapshot, SurfSnap, Target, WsSnap};
use cmux_ipc::wire::{Reply, RpcError};
use cmux_term::ghostty_vt::KeyAction;
use futures::channel::mpsc::UnboundedReceiver;
use gpui::{App, Context, Keystroke, Modifiers, Window};
use serde_json::{Map, Value, json};

use super::WindowView;
use crate::workspace::SplitFailed;

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
            "workspace.create" => self.create_workspace(params, window, cx),
            "surface.split" => self.split_from_socket(&snap, params, window, cx),
            "surface.send_text" | "surface.send_key" | "terminal.paste" => {
                self.send_from_socket(&snap, method, params, cx)
            }
            "notification.create"
            | "notification.create_for_target"
            | "notification.create_for_caller" => {
                self.notify_from_socket(&snap, method, params, window, cx)
            }
            "notification.clear" => self.clear_from_socket(&snap, params, cx),
            "notification.dismiss" => self.dismiss_from_socket(params, cx),
            // Unknown or already-answered ids are not an error (plan M7 D5).
            "feed.permission.reply" | "feed.exit_plan.reply" | "feed.question.reply" => {
                let (request_id, d) = cmux_ipc::feed::parse_reply(method, params)?;
                let pending = self.feed.items().find(|i| {
                    i.request_id == request_id && i.status == cmux_core::FeedStatus::Pending
                });
                if pending.is_some_and(|i| !i.kind.accepts(&d)) {
                    let msg = format!("{method} decision does not match the pending item");
                    return Err(RpcError::new("invalid_params", msg));
                }
                self.feed_finish(&request_id, super::feed_ctl::FeedEnd::Answer(d), cx);
                self.feed_refocus(window);
                Ok(json!({ "delivered": true }))
            }
            _ => Err(RpcError::new("method_not_found", "Unknown method")),
        }
    }

    /// `workspace.create` (rc §8, plan D6): after the selected workspace, selected only with
    /// `focus:true`.
    fn create_workspace(
        &mut self,
        params: &Map<String, Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Reply {
        let text = |k: &str| params.get(k).and_then(Value::as_str);
        if params
            .get("cwd")
            .is_some_and(|v| !v.is_string() && !v.is_null())
        {
            return Err(RpcError::new("invalid_params", "cwd must be a string"));
        }
        let focus = flag(params, "focus");
        let old = self.workspaces.selected();
        let cwd = text("cwd")
            .map(str::to_string)
            .or_else(|| self.views.get(&old).and_then(|v| v.read(cx).active_cwd(cx)));
        let id = self.workspaces.add_after_selected();
        if !self.make_view(id, cwd, window, cx) {
            self.workspaces.close(id);
            self.workspaces.select(old);
            return Err(RpcError::new(
                "internal_error",
                "Failed to create workspace",
            ));
        }
        if let Some(title) = text("title") {
            self.workspaces.rename(id, title);
        }
        let view = self.views[&id].clone();
        let surface = view.read(cx).layout().active_surface();
        if let Some(input) = text("initial_input")
            && let Some(term) = view.read(cx).terminal(surface).cloned()
        {
            term.update(cx, |t, cx| t.send_scripted(input.as_bytes().to_vec(), cx));
        }
        if focus {
            self.select(id, window, cx);
        } else {
            // The new view focused its shell as it started: give focus back (plan D6).
            self.workspaces.select(old);
            view.update(cx, |v, _| v.set_active(false));
            if let Some(v) = self.views.get(&old) {
                v.update(cx, |v, cx| v.focus_active(window, cx));
            }
            cx.notify();
        }
        let mut h = self.shell.handles.borrow_mut();
        let (w, ws, s) = (
            h.handle(Kind::Window, (0, 0)),
            h.handle(Kind::Workspace, (id.0, 0)),
            h.handle(Kind::Surface, (id.0, surface.0)),
        );
        Ok(json!({
            "window_id": w.id, "window_ref": w.r#ref,
            "workspace_id": ws.id, "workspace_ref": ws.r#ref,
            "surface_id": s.id, "surface_ref": s.r#ref,
        }))
    }

    /// `surface.split` (rc §8, plan D6).
    fn split_from_socket(
        &mut self,
        snap: &Snapshot,
        params: &Map<String, Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Reply {
        let dir = match params.get("direction").and_then(Value::as_str) {
            Some("left") => Direction::Left,
            Some("right") => Direction::Right,
            Some("up") => Direction::Up,
            Some("down") => Direction::Down,
            _ => {
                return Err(RpcError::new(
                    "invalid_params",
                    "Missing or invalid direction (left|right|up|down)",
                ));
            }
        };
        let focus = flag(params, "focus");
        let t = snap.surface(params)?;
        let id = WorkspaceId(t.ws.key);
        let Some(view) = self.views.get(&id).cloned() else {
            return Err(RpcError::new("not_found", "Workspace not found"));
        };
        let input = params.get("initial_input").and_then(Value::as_str);
        let new = view
            .update(cx, |v, cx| {
                v.split_surface(SurfaceId(t.surface.key), dir, focus, input, window, cx)
            })
            .map_err(|e| match e {
                // shortcut: cmux's no-space code is UNVERIFIED (rc §8); invalid_state is ours.
                SplitFailed::NoSpace => RpcError::new("invalid_state", "Not enough space to split"),
                SplitFailed::Spawn => RpcError::new("internal_error", "Failed to create split"),
            })?;
        if focus && id != self.workspaces.selected() {
            self.select(id, window, cx);
        }
        let pane = view.read(cx).layout().pane_of(new).map_or(0, |p| p.0);
        let mut h = self.shell.handles.borrow_mut();
        let (w, ws) = (
            h.handle(Kind::Window, (0, 0)),
            h.handle(Kind::Workspace, (id.0, 0)),
        );
        let (p, s) = (
            h.handle(Kind::Pane, (id.0, pane)),
            h.handle(Kind::Surface, (id.0, new.0)),
        );
        Ok(json!({
            "window_id": w.id, "window_ref": w.r#ref,
            "workspace_id": ws.id, "workspace_ref": ws.r#ref,
            "pane_id": p.id, "pane_ref": p.r#ref,
            "surface_id": s.id, "surface_ref": s.r#ref,
            "type": "terminal",
        }))
    }

    /// `surface.send_text`, `surface.send_key`, `terminal.paste` (rc §8).
    fn send_from_socket(
        &mut self,
        snap: &Snapshot,
        method: &str,
        params: &Map<String, Value>,
        cx: &mut Context<Self>,
    ) -> Reply {
        let (field, missing) = match method {
            "surface.send_key" => ("key", "Missing key"),
            _ => ("text", "Missing text"),
        };
        let Some(value) = params
            .get(field)
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
        else {
            return Err(RpcError::new("invalid_params", missing));
        };
        let t = snap.surface(params)?;
        let Some(term) = self
            .views
            .get(&WorkspaceId(t.ws.key))
            .and_then(|v| v.read(cx).terminal(SurfaceId(t.surface.key)).cloned())
        else {
            return Err(RpcError::new("not_found", "Surface not found"));
        };
        match method {
            "surface.send_key" => {
                let Some(spec) = parse_key(value) else {
                    return Err(RpcError::new("invalid_params", "Unknown key")
                        .with_data(json!({ "key": value })));
                };
                let ks = keystroke(&spec);
                term.update(cx, |t, cx| {
                    let bytes = t.encode_key(&ks, KeyAction::Press);
                    t.send_scripted(bytes, cx);
                });
            }
            "terminal.paste" => {
                term.update(cx, |t, cx| t.paste_scripted(&sanitize_paste(value), cx));
            }
            _ => term.update(cx, |t, cx| t.send_scripted(typed_bytes(value), cx)),
        }
        let mut reply = snap.target_payload(&t);
        reply["queued"] = json!(false);
        Ok(reply)
    }
}

impl WindowView {
    /// `notification.create`, `create_for_target`, `create_for_caller` (rc §8). A
    /// workspace-level target lands on that workspace's active surface (plan D7).
    fn notify_from_socket(
        &mut self,
        snap: &Snapshot,
        method: &str,
        params: &Map<String, Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Reply {
        let desktop = match params.get("effects") {
            None | Some(Value::Null) => true,
            Some(Value::Object(e)) => e.get("desktop").and_then(Value::as_bool).unwrap_or(true),
            Some(_) => {
                return Err(RpcError::new(
                    "invalid_params",
                    "Missing or invalid effects",
                ));
            }
        };
        let t: Target = match method {
            "notification.create_for_target" => {
                require(params, "workspace_id")?;
                require(params, "surface_id")?;
                let ws = snap.workspace(params)?;
                let t = snap.surface(params)?;
                if t.ws.key != ws.key {
                    return Err(RpcError::new("not_found", "Surface not found"));
                }
                t
            }
            "notification.create_for_caller" => {
                let (ws, t) = snap.caller(params)?;
                t.unwrap_or_else(|| ws.focused())
            }
            _ => snap.surface(params)?,
        };
        let text = |k: &str| {
            params
                .get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let (ws, surface) = (WorkspaceId(t.ws.key), SurfaceId(t.surface.key));
        let id = self.on_notify(
            ws,
            surface,
            text("title"),
            text("subtitle"),
            text("body"),
            desktop,
            window,
            cx,
        );
        let mut reply = json!({
            "workspace_id": t.ws.handle.id,
            "surface_id": t.surface.handle.id,
            "id": self.shell.handles.borrow_mut().notification(id.0),
        });
        if method == "notification.create_for_target" {
            reply["workspace_ref"] = json!(t.ws.handle.r#ref);
            reply["surface_ref"] = json!(t.surface.handle.r#ref);
            reply = snap.with_window(reply);
        }
        Ok(reply)
    }

    /// `notification.clear`: a surface, a workspace, the caller's surface, or everything.
    fn clear_from_socket(
        &mut self,
        snap: &Snapshot,
        params: &Map<String, Value>,
        cx: &mut Context<Self>,
    ) -> Reply {
        let (ws, t): (Option<&WsSnap>, Option<Target>) = if flag(params, "caller") {
            let (ws, t) = snap.caller(params)?;
            (Some(ws), t)
        } else if params.get("workspace_id").is_some_and(|v| !v.is_null()) {
            let ws = snap.workspace(params)?;
            let t = match params.get("surface_id") {
                Some(v) if !v.is_null() => {
                    let t = snap.surface(params)?;
                    if t.ws.key != ws.key {
                        return Err(RpcError::new("not_found", "Surface not found"));
                    }
                    Some(t)
                }
                _ => None,
            };
            (Some(ws), t)
        } else {
            (None, None)
        };
        let gone = match (ws, &t) {
            (_, Some(t)) => self
                .notifications
                .remove_surface(WorkspaceId(t.ws.key), SurfaceId(t.surface.key)),
            (Some(ws), None) => self.notifications.remove_workspace(WorkspaceId(ws.key)),
            (None, None) => self.notifications.clear_all(),
        };
        self.after_change(gone, cx);
        Ok(json!({
            "cleared": true,
            "workspace_id": ws.map(|w| w.handle.id.clone()),
            "workspace_ref": ws.map(|w| w.handle.r#ref.clone()),
            "surface_id": t.map(|t| t.surface.handle.id.clone()),
            "surface_ref": t.map(|t| t.surface.handle.r#ref.clone()),
        }))
    }

    /// `notification.dismiss`: one notification by id, or every read one.
    fn dismiss_from_socket(
        &mut self,
        params: &Map<String, Value>,
        cx: &mut Context<Self>,
    ) -> Reply {
        if flag(params, "all_read") {
            let gone = self.notifications.remove_read();
            let n = gone.len();
            self.after_change(gone, cx);
            return Ok(json!({ "dismissed": n, "all_read": true }));
        }
        let Some(wanted) = params.get("id").and_then(Value::as_str) else {
            return Err(RpcError::new("invalid_params", "Missing or invalid id"));
        };
        let found = {
            let mut h = self.shell.handles.borrow_mut();
            self.notifications
                .list()
                .iter()
                .find(|n| h.notification(n.id.0).eq_ignore_ascii_case(wanted))
                .cloned()
        };
        let Some(n) = found else {
            return Err(RpcError::new("not_found", "Notification not found"));
        };
        let mut reply = self.notification_payload(&n);
        reply["dismissed"] = json!(1);
        let gone = self.notifications.remove(n.id);
        self.after_change(gone, cx);
        Ok(reply)
    }

    fn notification_payload(&self, n: &Notification) -> Value {
        let mut h = self.shell.handles.borrow_mut();
        let ws = h.handle(Kind::Workspace, (n.workspace.0, 0));
        let s = h.handle(Kind::Surface, (n.workspace.0, n.surface.0));
        json!({
            "id": h.notification(n.id.0),
            "workspace_id": ws.id, "workspace_ref": ws.r#ref,
            "surface_id": s.id, "surface_ref": s.r#ref,
            "title": n.title, "subtitle": n.subtitle, "body": n.body,
            "is_read": n.read,
        })
    }
}

/// A boolean param, false when absent.
fn flag(params: &Map<String, Value>, key: &str) -> bool {
    params.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// The keystroke the keyboard would deliver for `k`, so the terminal's own encoder (modes,
/// kitty keyboard) applies.
fn keystroke(k: &KeySpec) -> Keystroke {
    let typed = !(k.ctrl || k.platform);
    let key_char = match k.key.as_str() {
        "space" if typed => Some(" ".to_string()),
        c if typed && c.chars().count() == 1 => Some(if k.shift {
            c.to_uppercase()
        } else {
            c.to_string()
        }),
        _ => None,
    };
    Keystroke {
        modifiers: Modifiers {
            control: k.ctrl,
            alt: k.alt,
            shift: k.shift,
            platform: k.platform,
            function: false,
        },
        key: k.key.clone(),
        key_char,
    }
}
