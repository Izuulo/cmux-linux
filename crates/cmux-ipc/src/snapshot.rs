//! A copy of the window model for socket requests: the read methods and target resolution
//! (rc §6, §8), without gpui so they are unit-tested.

use serde_json::{Map, Value, json};

use crate::handles::Handle;
use crate::wire::{Reply, RpcError};

#[derive(Clone)]
pub struct Snapshot {
    pub window: Handle,
    /// Index into `workspaces`.
    pub selected: usize,
    pub workspaces: Vec<WsSnap>,
    /// Workspace groups in sidebar order (M8).
    pub groups: Vec<GroupSnap>,
}

#[derive(Debug, Clone)]
pub struct GroupSnap {
    pub key: u64,
    pub handle: Handle,
    pub name: String,
    pub collapsed: bool,
    pub pinned: bool,
    pub generated_anchor: bool,
    /// Workspace keys, the anchor first.
    pub members: Vec<u64>,
}

#[derive(Clone)]
pub struct WsSnap {
    pub key: u64,
    pub handle: Handle,
    pub title: String,
    pub custom_title: Option<String>,
    pub cwd: Option<String>,
    pub ports: Vec<u16>,
    /// Index into `panes`.
    pub focused_pane: usize,
    pub panes: Vec<PaneSnap>,
}

#[derive(Clone)]
pub struct PaneSnap {
    pub key: u64,
    pub handle: Handle,
    /// Index into `surfaces`.
    pub selected: usize,
    pub surfaces: Vec<SurfSnap>,
}

#[derive(Clone)]
pub struct SurfSnap {
    pub key: u64,
    pub handle: Handle,
    pub title: String,
}

#[derive(Clone, Copy)]
pub struct Target<'a> {
    pub ws: &'a WsSnap,
    pub pane: &'a PaneSnap,
    pub surface: &'a SurfSnap,
}

impl WsSnap {
    pub fn focused(&self) -> Target<'_> {
        let pane = &self.panes[self.focused_pane];
        Target {
            ws: self,
            pane,
            surface: &pane.surfaces[pane.selected],
        }
    }

    fn targets(&self) -> impl Iterator<Item = Target<'_>> {
        self.panes.iter().flat_map(move |pane| {
            pane.surfaces.iter().map(move |surface| Target {
                ws: self,
                pane,
                surface,
            })
        })
    }
}

/// UUID in any case, or the exact ref.
fn matches(h: &Handle, s: &str) -> bool {
    h.id.eq_ignore_ascii_case(s) || h.r#ref == s
}

/// The string value of `key`: `Ok(None)` when absent or null.
fn param<'a>(params: &'a Map<String, Value>, key: &str) -> Result<Option<&'a str>, RpcError> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => Err(RpcError::new(
            "invalid_params",
            format!("Missing or invalid {key}"),
        )),
    }
}

fn not_found(what: &str) -> RpcError {
    RpcError::new("not_found", format!("{what} not found"))
}

impl Snapshot {
    /// The read-only methods; `None` for any other method.
    pub fn read(&self, method: &str, params: &Map<String, Value>) -> Option<Reply> {
        let reply = match method {
            "window.list" => Ok(json!({"windows": [{
                "id": self.window.id, "ref": self.window.r#ref, "index": 0,
                "selected": true, "workspace_count": self.workspaces.len(),
            }]})),
            "workspace.list" => Ok(self.with_window(json!({
                "workspaces": self.workspaces.iter().enumerate()
                    .map(|(i, ws)| self.workspace_row(i, ws)).collect::<Vec<_>>(),
            }))),
            "workspace.current" => self.workspace(params).map(|ws| {
                let i = self.index_of(ws);
                self.with_window(json!({
                    "workspace_id": ws.handle.id, "workspace_ref": ws.handle.r#ref,
                    "workspace": self.workspace_row(i, ws),
                }))
            }),
            "workspace.group.list" => Ok(self.with_window(json!({
                "groups": self.groups.iter().map(|g| self.group_payload(g)).collect::<Vec<_>>(),
            }))),
            "pane.list" => self.workspace(params).map(|ws| {
                let panes: Vec<Value> = ws
                    .panes
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let sel = &p.surfaces[p.selected].handle;
                        json!({
                            "id": p.handle.id, "ref": p.handle.r#ref, "index": i,
                            "focused": i == ws.focused_pane,
                            "surface_ids": p.surfaces.iter().map(|s| &s.handle.id).collect::<Vec<_>>(),
                            "surface_refs": p.surfaces.iter().map(|s| &s.handle.r#ref).collect::<Vec<_>>(),
                            "selected_surface_id": sel.id, "selected_surface_ref": sel.r#ref,
                            "surface_count": p.surfaces.len(),
                        })
                    })
                    .collect();
                self.with_window(json!({
                    "workspace_id": ws.handle.id, "workspace_ref": ws.handle.r#ref,
                    "panes": panes,
                }))
            }),
            "pane.surfaces" => self
                .workspace(params)
                .and_then(|ws| Ok((ws, self.pane(ws, params)?)))
                .map_err(|e| match e.code.as_str() {
                    "not_found" => not_found("Pane or workspace"),
                    _ => e,
                })
                .map(|(ws, pane)| {
                    let rows: Vec<Value> = pane
                        .surfaces
                        .iter()
                        .enumerate()
                        .map(|(i, s)| surface_row(i, s, i == pane.selected))
                        .collect();
                    self.with_window(json!({
                        "workspace_id": ws.handle.id, "workspace_ref": ws.handle.r#ref,
                        "pane_id": pane.handle.id, "pane_ref": pane.handle.r#ref,
                        "surfaces": rows,
                    }))
                }),
            "surface.list" => self.workspace(params).map(|ws| {
                let active = ws.focused().surface.key;
                let rows: Vec<Value> = ws
                    .targets()
                    .enumerate()
                    .map(|(i, t)| {
                        let mut row = surface_row(i, t.surface, t.surface.key == active);
                        row["pane_id"] = json!(t.pane.handle.id);
                        row["pane_ref"] = json!(t.pane.handle.r#ref);
                        row
                    })
                    .collect();
                self.with_window(json!({
                    "workspace_id": ws.handle.id, "workspace_ref": ws.handle.r#ref,
                    "surfaces": rows,
                }))
            }),
            _ => return None,
        };
        Some(reply)
    }

    /// `window_id` absent or ours.
    pub fn check_window(&self, params: &Map<String, Value>) -> Result<(), RpcError> {
        match param(params, "window_id")? {
            Some(w) if !matches(&self.window, w) => Err(not_found("Window")),
            _ => Ok(()),
        }
    }

    /// `workspace_id`, else the selected workspace.
    pub fn workspace(&self, params: &Map<String, Value>) -> Result<&WsSnap, RpcError> {
        match param(params, "workspace_id")? {
            Some(h) => self.find_workspace(h).ok_or_else(|| not_found("Workspace")),
            None => Ok(&self.workspaces[self.selected]),
        }
    }

    /// `pane_id` in `ws`, else its focused pane.
    pub fn pane<'a>(
        &self,
        ws: &'a WsSnap,
        params: &Map<String, Value>,
    ) -> Result<&'a PaneSnap, RpcError> {
        match param(params, "pane_id")? {
            Some(h) => ws
                .panes
                .iter()
                .find(|p| matches(&p.handle, h))
                .ok_or_else(|| not_found("Pane")),
            None => Ok(&ws.panes[ws.focused_pane]),
        }
    }

    /// `surface_id` in any workspace, else the target workspace's active surface.
    pub fn surface(&self, params: &Map<String, Value>) -> Result<Target<'_>, RpcError> {
        match param(params, "surface_id")? {
            Some(h) => self
                .find_surface(h)
                .ok_or_else(|| not_found("Surface").with_data(json!({ "surface_id": h }))),
            None => Ok(self.workspace(params)?.focused()),
        }
    }

    /// cmux `create_for_caller` resolution without TTY matching (rc §8, plan D7).
    pub fn caller(
        &self,
        params: &Map<String, Value>,
    ) -> Result<(&WsSnap, Option<Target<'_>>), RpcError> {
        let pw = param(params, "preferred_workspace_id")?;
        let ps = param(params, "preferred_surface_id")?;
        // A caller whose pane has closed gets nothing rather than a ring on another pane
        // (plan Review Focus 5); a pane that only moved to another workspace still resolves.
        if ps.is_some_and(|h| self.find_surface(h).is_none())
            && pw.is_some_and(|h| self.find_workspace(h).is_some())
        {
            return Err(not_found("Surface"));
        }
        if let Some(ws) = pw.and_then(|h| self.find_workspace(h)) {
            let t = ps.and_then(|h| ws.targets().find(|t| matches(&t.surface.handle, h)));
            return Ok((ws, t));
        }
        if let Some(t) = ps.and_then(|h| self.find_surface(h)) {
            return Ok((t.ws, Some(t)));
        }
        if pw.is_some() || ps.is_some() {
            return Err(not_found("Workspace"));
        }
        Ok((&self.workspaces[self.selected], None))
    }

    /// Window, workspace, pane and surface ids and refs of `t`.
    pub fn target_payload(&self, t: &Target) -> Value {
        self.with_window(json!({
            "workspace_id": t.ws.handle.id, "workspace_ref": t.ws.handle.r#ref,
            "pane_id": t.pane.handle.id, "pane_ref": t.pane.handle.r#ref,
            "surface_id": t.surface.handle.id, "surface_ref": t.surface.handle.r#ref,
        }))
    }

    /// `v` plus `window_id`/`window_ref`.
    pub fn with_window(&self, mut v: Value) -> Value {
        v["window_id"] = json!(self.window.id);
        v["window_ref"] = json!(self.window.r#ref);
        v
    }

    /// `group_id` (required): a UUID or a `workspace_group:N` ref (research-groups.md §12).
    pub fn group(&self, params: &Map<String, Value>) -> Result<&GroupSnap, RpcError> {
        let invalid = || RpcError::new("invalid_params", "Missing or invalid group_id");
        let h = param(params, "group_id")?.ok_or_else(invalid)?;
        match self.groups.iter().find(|g| matches(&g.handle, h)) {
            Some(g) => Ok(g),
            // cmux: an unresolvable ref is a bad parameter, an unknown id a missing group.
            None if h.contains(':') || h.is_empty() => Err(invalid()),
            None => Err(not_found("Group").with_data(json!({ "group_id": h }))),
        }
    }

    /// cmux's group payload (rg §12.2), without colour, icon and external ids.
    pub fn group_payload(&self, g: &GroupSnap) -> Value {
        let members: Vec<&Handle> = g
            .members
            .iter()
            .filter_map(|k| self.workspaces.iter().find(|ws| ws.key == *k))
            .map(|ws| &ws.handle)
            .collect();
        let anchor = members.first();
        json!({
            "id": g.handle.id, "ref": g.handle.r#ref, "name": g.name,
            "is_collapsed": g.collapsed, "is_pinned": g.pinned, "is_empty": false,
            "anchor_workspace_id": anchor.map(|h| &h.id),
            "anchor_workspace_ref": anchor.map(|h| &h.r#ref),
            "member_workspace_ids": members.iter().map(|h| &h.id).collect::<Vec<_>>(),
            "member_workspace_refs": members.iter().map(|h| &h.r#ref).collect::<Vec<_>>(),
            "member_count": members.len(),
            "anchor_workspace_provenance": if g.generated_anchor { "generated" } else { "user" },
            "anchor_workspace_is_generated": g.generated_anchor,
        })
    }

    pub fn find_workspace(&self, h: &str) -> Option<&WsSnap> {
        self.workspaces.iter().find(|ws| matches(&ws.handle, h))
    }

    fn find_surface(&self, h: &str) -> Option<Target<'_>> {
        // `tab:N` is cmux's alias for `surface:N`.
        let alias = h.strip_prefix("tab:").map(|n| format!("surface:{n}"));
        let h = alias.as_deref().unwrap_or(h);
        self.workspaces
            .iter()
            .flat_map(|ws| ws.targets())
            .find(|t| matches(&t.surface.handle, h))
    }

    fn index_of(&self, ws: &WsSnap) -> usize {
        self.workspaces
            .iter()
            .position(|w| w.key == ws.key)
            .unwrap_or(0)
    }

    fn workspace_row(&self, i: usize, ws: &WsSnap) -> Value {
        json!({
            "id": ws.handle.id, "ref": ws.handle.r#ref, "index": i,
            "title": ws.title, "custom_title": ws.custom_title,
            "has_custom_title": ws.custom_title.is_some(),
            "selected": i == self.selected, "pinned": false,
            "listening_ports": ws.ports, "current_directory": ws.cwd,
        })
    }
}

fn surface_row(i: usize, s: &SurfSnap, selected: bool) -> Value {
    json!({
        "id": s.handle.id, "ref": s.handle.r#ref, "index": i,
        "title": s.title, "type": "terminal", "selected": selected,
    })
}
