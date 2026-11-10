mod support;

use cmux_ipc::wire::RpcError;
use serde_json::{Map, Value, json};
use support::fixture;

fn p(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn workspace_list_payload() {
    let s = fixture();
    let (w, a, b) = (&s.window, &s.workspaces[0].handle, &s.workspaces[1].handle);
    assert_eq!(
        s.read("workspace.list", &Map::new()).unwrap().unwrap(),
        json!({
            "window_id": w.id, "window_ref": w.r#ref,
            "workspaces": [
                {"id": a.id, "ref": "workspace:1", "index": 0, "title": "Terminal",
                 "custom_title": null, "has_custom_title": false, "selected": true,
                 "pinned": false, "listening_ports": [3000], "current_directory": "/home/u/src"},
                {"id": b.id, "ref": "workspace:2", "index": 1, "title": "Build",
                 "custom_title": "Build", "has_custom_title": true, "selected": false,
                 "pinned": false, "listening_ports": [], "current_directory": null},
            ]
        })
    );
}

#[test]
fn workspace_current_payload() {
    let s = fixture();
    let v = s.read("workspace.current", &Map::new()).unwrap().unwrap();
    assert_eq!(v["workspace_id"], json!(s.workspaces[0].handle.id));
    assert_eq!(v["workspace_ref"], json!("workspace:1"));
    assert_eq!(v["window_ref"], json!("window:1"));
    assert_eq!(v["workspace"]["title"], json!("Terminal"));
}

#[test]
fn window_list_payload() {
    let s = fixture();
    assert_eq!(
        s.read("window.list", &Map::new()).unwrap().unwrap(),
        json!({"windows": [{"id": s.window.id, "ref": "window:1", "index": 0,
                            "selected": true, "workspace_count": 2}]})
    );
}

#[test]
fn pane_list_payload() {
    let s = fixture();
    let a = &s.workspaces[0];
    let (p1, p2) = (&a.panes[0], &a.panes[1]);
    let (s1, s2, s3) = (
        &p1.surfaces[0].handle,
        &p1.surfaces[1].handle,
        &p2.surfaces[0].handle,
    );
    assert_eq!(
        s.read("pane.list", &Map::new()).unwrap().unwrap(),
        json!({
            "workspace_id": a.handle.id, "workspace_ref": "workspace:1",
            "window_id": s.window.id, "window_ref": "window:1",
            "panes": [
                {"id": p1.handle.id, "ref": "pane:1", "index": 0, "focused": false,
                 "surface_ids": [s1.id, s2.id], "surface_refs": ["surface:1", "surface:2"],
                 "selected_surface_id": s1.id, "selected_surface_ref": "surface:1",
                 "surface_count": 2},
                {"id": p2.handle.id, "ref": "pane:2", "index": 1, "focused": true,
                 "surface_ids": [s3.id], "surface_refs": ["surface:3"],
                 "selected_surface_id": s3.id, "selected_surface_ref": "surface:3",
                 "surface_count": 1},
            ]
        })
    );
}

#[test]
fn pane_surfaces_payload() {
    let s = fixture();
    let a = &s.workspaces[0];
    let p1 = &a.panes[0];
    let v = s
        .read("pane.surfaces", &p(json!({"pane_id": "pane:1"})))
        .unwrap()
        .unwrap();
    assert_eq!(
        v,
        json!({
            "workspace_id": a.handle.id, "workspace_ref": "workspace:1",
            "pane_id": p1.handle.id, "pane_ref": "pane:1",
            "window_id": s.window.id, "window_ref": "window:1",
            "surfaces": [
                {"id": p1.surfaces[0].handle.id, "ref": "surface:1", "index": 0,
                 "title": "zsh", "type": "terminal", "selected": true},
                {"id": p1.surfaces[1].handle.id, "ref": "surface:2", "index": 1,
                 "title": "vim", "type": "terminal", "selected": false},
            ]
        })
    );
    // No pane_id: the focused pane.
    let v = s.read("pane.surfaces", &Map::new()).unwrap().unwrap();
    assert_eq!(v["pane_ref"], json!("pane:2"));
}

#[test]
fn surface_list_indexes_span_panes() {
    let s = fixture();
    let v = s.read("surface.list", &Map::new()).unwrap().unwrap();
    let rows = v["surfaces"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2]["ref"], json!("surface:3"));
    assert_eq!(rows[2]["index"], json!(2));
    assert_eq!(rows[2]["pane_ref"], json!("pane:2"));
    // The workspace's active surface is the focused pane's selected one.
    assert_eq!(rows[2]["selected"], json!(true));
    assert_eq!(rows[0]["selected"], json!(false));
}

#[test]
fn other_methods_are_not_reads() {
    assert!(fixture().read("surface.split", &Map::new()).is_none());
}

#[test]
fn handles_resolve() {
    let s = fixture();
    let b = &s.workspaces[1];
    let lower = b.handle.id.to_lowercase();
    assert_eq!(
        s.workspace(&p(json!({"workspace_id": lower}))).unwrap().key,
        2
    );
    assert_eq!(
        s.workspace(&p(json!({"workspace_id": "workspace:2"})))
            .unwrap()
            .key,
        2
    );
    let t = s.surface(&p(json!({"surface_id": "tab:3"}))).unwrap();
    assert_eq!(t.surface.handle.r#ref, "surface:3");
    assert_eq!(
        s.workspace(&p(json!({"workspace_id": "workspace:9"})))
            .err(),
        Some(RpcError::new("not_found", "Workspace not found"))
    );
    assert_eq!(
        s.surface(&p(json!({"surface_id": "surface:9"}))).err(),
        Some(
            RpcError::new("not_found", "Surface not found")
                .with_data(json!({"surface_id": "surface:9"}))
        )
    );
    let a = &s.workspaces[0];
    assert_eq!(
        s.pane(a, &p(json!({"pane_id": "pane:9"}))).err(),
        Some(RpcError::new("not_found", "Pane not found"))
    );
    assert_eq!(
        s.read("pane.surfaces", &p(json!({"pane_id": "pane:9"}))),
        Some(Err(RpcError::new(
            "not_found",
            "Pane or workspace not found"
        )))
    );
    assert_eq!(
        s.workspace(&p(json!({"workspace_id": 5}))).err(),
        Some(RpcError::new(
            "invalid_params",
            "Missing or invalid workspace_id"
        ))
    );
}

#[test]
fn surface_defaults_to_focused() {
    let s = fixture();
    let t = s.surface(&Map::new()).unwrap();
    assert_eq!((t.ws.key, t.pane.key, t.surface.key), (1, 2, 3));
    let b = s.workspaces[1].handle.id.clone();
    let t = s.surface(&p(json!({"workspace_id": b}))).unwrap();
    assert_eq!((t.ws.key, t.surface.key), (2, 1));
    // A surface id wins over the workspace and is found in any workspace.
    let t = s
        .surface(&p(
            json!({"workspace_id": "workspace:1", "surface_id": "surface:4"}),
        ))
        .unwrap();
    assert_eq!((t.ws.key, t.surface.key), (2, 1));
}

#[test]
fn target_payload_has_all_handles() {
    let s = fixture();
    let t = s.surface(&Map::new()).unwrap();
    let v = s.target_payload(&t);
    assert_eq!(v["window_ref"], json!("window:1"));
    assert_eq!(v["workspace_ref"], json!("workspace:1"));
    assert_eq!(v["pane_ref"], json!("pane:2"));
    assert_eq!(v["surface_ref"], json!("surface:3"));
    assert_eq!(v["surface_id"], json!(t.surface.handle.id));
}

#[test]
fn caller_rules() {
    let s = fixture();
    let (a, b) = (
        s.workspaces[0].handle.id.clone(),
        s.workspaces[1].handle.id.clone(),
    );
    let s1 = s.workspaces[0].panes[0].surfaces[0].handle.id.clone();
    let (ws, t) = s
        .caller(&p(
            json!({"preferred_workspace_id": a, "preferred_surface_id": s1}),
        ))
        .unwrap();
    assert_eq!((ws.key, t.map(|t| t.surface.key)), (1, Some(1)));
    let (ws, t) = s
        .caller(&p(
            json!({"preferred_workspace_id": b, "preferred_surface_id": s1}),
        ))
        .unwrap();
    assert_eq!((ws.key, t.is_none()), (2, true));
    let (ws, t) = s.caller(&p(json!({"preferred_surface_id": s1}))).unwrap();
    assert_eq!((ws.key, t.map(|t| t.surface.key)), (1, Some(1)));
    let (ws, t) = s.caller(&Map::new()).unwrap();
    assert_eq!((ws.key, t.is_none()), (1, true));
}

#[test]
fn caller_with_gone_ids_is_not_found() {
    let s = fixture();
    let gone = "00000000-0000-4000-8000-000000000000";
    assert_eq!(
        s.caller(&p(
            json!({"preferred_workspace_id": gone, "preferred_surface_id": gone})
        ))
        .err(),
        Some(RpcError::new("not_found", "Workspace not found"))
    );
    // The pane closed but its workspace is open: still nothing lands on another pane.
    let a = s.workspaces[0].handle.id.clone();
    assert_eq!(
        s.caller(&p(
            json!({"preferred_workspace_id": a, "preferred_surface_id": gone})
        ))
        .err(),
        Some(RpcError::new("not_found", "Surface not found"))
    );
}

#[test]
fn unknown_window_is_not_found() {
    let s = fixture();
    assert!(s.check_window(&Map::new()).is_ok());
    assert!(s.check_window(&p(json!({"window_id": "window:1"}))).is_ok());
    assert!(
        s.check_window(&p(json!({"window_id": s.window.id.to_lowercase()})))
            .is_ok()
    );
    assert_eq!(
        s.check_window(&p(json!({"window_id": "window:2"}))).err(),
        Some(RpcError::new("not_found", "Window not found"))
    );
}
