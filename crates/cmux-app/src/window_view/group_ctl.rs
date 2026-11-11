//! `workspace.group.*` on the socket (M8 plan D11, research-groups.md §12): cmux's params,
//! payloads and error texts. Only `focus` changes the selection.

use cmux_core::{GroupError, GroupId, Placement, WorkspaceId};
use cmux_ipc::snapshot::Snapshot;
use cmux_ipc::wire::{Reply, RpcError};
use gpui::{Context, Window};
use serde_json::{Map, Value, json};

use super::WindowView;

fn err(code: &str, msg: &str) -> RpcError {
    RpcError::new(code, msg)
}

fn text<'a>(params: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    params.get(key).and_then(Value::as_str)
}

/// A required workspace handle; `missing` is the message when it is absent or not a string.
fn workspace(
    snap: &Snapshot,
    params: &Map<String, Value>,
    missing: &str,
    unknown: &str,
) -> Result<(WorkspaceId, String), RpcError> {
    let h = text(params, "workspace_id")
        .filter(|h| !h.is_empty())
        .ok_or_else(|| err("invalid_params", missing))?;
    let ws = snap
        .find_workspace(h)
        .ok_or_else(|| err("not_found", unknown))?;
    Ok((WorkspaceId(ws.key), ws.handle.id.clone()))
}

impl WindowView {
    pub(super) fn group_from_socket(
        &mut self,
        snap: &Snapshot,
        method: &str,
        params: &Map<String, Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Reply {
        if method == "workspace.group.create" {
            return self.group_create(snap, params, window, cx);
        }
        if method == "workspace.group.remove" {
            let (ws, id) = workspace(
                snap,
                params,
                "Missing or invalid workspace_id",
                "Workspace not in a group",
            )?;
            return match self.workspaces.remove_from_group(ws) {
                Ok(()) => {
                    cx.notify();
                    Ok(json!({ "workspace_id": id }))
                }
                Err(_) => Err(err("not_found", "Workspace not in a group")),
            };
        }
        // cmux reports a missing id together with the other required parameter.
        let pair = match method {
            "workspace.group.rename" => Some("Missing group_id or name"),
            "workspace.group.add" | "workspace.group.set_anchor" => {
                Some("Missing group_id or workspace_id")
            }
            _ => None,
        };
        if let Some(msg) = pair.filter(|_| text(params, "group_id").is_none()) {
            return Err(err("invalid_params", msg));
        }
        let g = snap.group(params)?;
        let (gid, id) = (GroupId(g.key), g.handle.id.clone());
        let reply = match method {
            "workspace.group.ungroup" => self.group_dissolve(gid, &id),
            "workspace.group.delete" => {
                if params.get("close_workspaces").and_then(Value::as_bool) == Some(true) {
                    let n = self.delete_group(gid, window, cx);
                    json!({
                        "group_id": id, "operation": "closed_workspaces",
                        "closed_workspace_count": n,
                    })
                } else {
                    self.group_dissolve(gid, &id)
                }
            }
            "workspace.group.rename" => {
                let name = text(params, "name").unwrap_or("").trim();
                if name.is_empty() {
                    return Err(err("invalid_params", "Missing group_id or name"));
                }
                self.workspaces.rename_group(gid, name);
                json!({ "group_id": id, "name": name })
            }
            "workspace.group.collapse" | "workspace.group.expand" => {
                let collapsed = method.ends_with("collapse");
                self.set_group_collapsed(gid, collapsed, window, cx);
                json!({ "group_id": id, "is_collapsed": collapsed })
            }
            "workspace.group.pin" | "workspace.group.unpin" => {
                let pinned = method.ends_with(".pin");
                self.workspaces.set_pinned(gid, pinned);
                json!({ "group_id": id, "is_pinned": pinned })
            }
            "workspace.group.add" => {
                let missing = "Missing group_id or workspace_id";
                let (ws, ws_id) = workspace(snap, params, missing, "Group or workspace not found")?;
                match self.workspaces.add_to_group(gid, ws) {
                    Ok(()) => json!({ "group_id": id, "workspace_id": ws_id }),
                    Err(GroupError::AnchorElsewhere) => {
                        return Err(err(
                            "invalid_state",
                            "Workspace is the anchor of another group; ungroup it first",
                        ));
                    }
                    Err(_) => return Err(err("not_found", "Group or workspace not found")),
                }
            }
            "workspace.group.set_anchor" => {
                let unknown = "Group not found or workspace not a member";
                let (ws, ws_id) =
                    workspace(snap, params, "Missing group_id or workspace_id", unknown)?;
                if self.workspaces.set_anchor(gid, ws).is_err() {
                    return Err(err("not_found", unknown));
                }
                json!({ "group_id": id, "anchor_workspace_id": ws_id })
            }
            "workspace.group.new_workspace" => {
                let placement = match text(params, "placement") {
                    None => Placement::default(),
                    Some(p) => Placement::parse(p).ok_or_else(|| {
                        err(
                            "invalid_params",
                            "placement must be one of: afterCurrent, top, end",
                        )
                    })?,
                };
                let Some(ws) = self.spawn_in_group(gid, placement, None, false, window, cx) else {
                    return Err(err("internal_error", "Failed to create workspace"));
                };
                let h = self.workspace_handle(ws);
                json!({ "group_id": id, "workspace_id": h.id, "workspace_ref": h.r#ref })
            }
            "workspace.group.focus" => {
                let anchor = self
                    .workspaces
                    .group(gid)
                    .expect("the snapshot is current")
                    .anchor;
                self.select(anchor, window, cx);
                let h = self.workspace_handle(anchor);
                json!({
                    "group_id": id,
                    "anchor_workspace_id": h.id, "anchor_workspace_ref": h.r#ref,
                })
            }
            _ => return Err(err("method_not_found", "Unknown method")),
        };
        cx.notify();
        Ok(reply)
    }

    fn group_dissolve(&mut self, g: GroupId, id: &str) -> Value {
        let kept = self.workspaces.ungroup(g).unwrap_or(0);
        json!({ "group_id": id, "operation": "dissolved", "kept_workspace_count": kept })
    }

    /// `workspace.group.create`: from `child_workspace_ids`, or an empty group with a new
    /// anchor in `cwd`. The selection stays where it is (rg §3.9).
    fn group_create(
        &mut self,
        snap: &Snapshot,
        params: &Map<String, Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Reply {
        let name = text(params, "name").unwrap_or("");
        let children = match params.get("child_workspace_ids") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(a)) if a.iter().all(Value::is_string) => {
                a.iter().filter_map(Value::as_str).collect()
            }
            Some(_) => {
                return Err(err(
                    "invalid_params",
                    "child_workspace_ids must be an array of workspace handles",
                ));
            }
        };
        let gid = if children.is_empty() {
            let old = self.workspaces.selected();
            let cwd = text(params, "cwd")
                .map(str::to_string)
                .or_else(|| self.views.get(&old).and_then(|v| v.read(cx).active_cwd(cx)));
            let (gid, anchor) = self.workspaces.create_empty_group(name);
            if !self.adopt(anchor, old, cwd, false, window, cx) {
                return Err(err("not_created", "Group was not created"));
            }
            gid
        } else {
            let (found, missing): (Vec<_>, Vec<_>) = children
                .iter()
                .map(|h| {
                    snap.find_workspace(h)
                        .map(|ws| WorkspaceId(ws.key))
                        .ok_or(*h)
                })
                .partition(Result::is_ok);
            if !missing.is_empty() {
                let handles: Vec<&str> = missing.into_iter().filter_map(Result::err).collect();
                return Err(err(
                    "invalid_params",
                    &format!("Unresolved child workspace handles: {}", handles.join(", ")),
                ));
            }
            let ids: Vec<WorkspaceId> = found.into_iter().filter_map(Result::ok).collect();
            match self.workspaces.create_group(name, &ids) {
                Ok(gid) => gid,
                Err(GroupError::AllAnchors) => {
                    return Err(err(
                        "invalid_state",
                        "All requested children are ineligible because they are already group anchors; ungroup them first",
                    ));
                }
                Err(_) => return Err(err("not_created", "Group was not created")),
            }
        };
        cx.notify();
        let snap = self.snapshot(cx);
        let group = snap
            .groups
            .iter()
            .find(|g| g.key == gid.0)
            .expect("just created");
        Ok(snap.with_window(json!({ "group": snap.group_payload(group), "created": true })))
    }
}
