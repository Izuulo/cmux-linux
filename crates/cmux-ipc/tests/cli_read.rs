mod support;

use std::collections::HashMap;

use cmux_ipc::wire::RpcError;
use serde_json::{Value, json};
use support::{Fake, T, err, fixture, ok};

#[test]
fn list_workspaces_text() {
    let t = T::new(HashMap::new());
    assert_eq!(
        ok(t.run(&["list-workspaces"])),
        "* workspace:1  Terminal  [selected]\n  workspace:2  Build\n"
    );
    assert_eq!(
        ok(t.run(&["list-workspaces", "--id-format", "both"])),
        format!(
            "* workspace:1 {}  Terminal  [selected]\n  workspace:2 {}  Build\n",
            t.a(),
            t.b()
        )
    );
    assert_eq!(
        ok(t.run(&["--id-format", "uuids", "list-workspaces"])),
        format!("* {}  Terminal  [selected]\n  {}  Build\n", t.a(), t.b())
    );
}

#[test]
fn list_workspaces_json_keeps_workspace_ids() {
    let t = T::new(HashMap::new());
    let v: Value = serde_json::from_str(&ok(t.run(&["list-workspaces", "--json"]))).unwrap();
    assert_eq!(v["workspaces"][0]["id"], json!(t.a()));
    assert_eq!(v["workspaces"][0]["ref"], json!("workspace:1"));
    assert_eq!(v["window_ref"], json!("window:1"));
    assert!(v.get("window_id").is_none());
    let v: Value = serde_json::from_str(&ok(t.run(&[
        "--json",
        "list-workspaces",
        "--id-format",
        "refs",
    ])))
    .unwrap();
    assert!(v["workspaces"][0].get("id").is_none());
    let text = ok(t.run(&["--json", "list-workspaces"]));
    assert!(
        text.starts_with("{\n  \"window_ref\" : \"window:1\""),
        "{text}"
    );
}

#[test]
fn list_panes_text() {
    let t = T::new(HashMap::new());
    assert_eq!(
        ok(t.run(&["list-panes"])),
        "  pane:1  [2 surfaces]\n* pane:2  [1 surface]  [focused]\n"
    );
    assert_eq!(
        t.last(),
        ("pane.list".into(), json!({"workspace_id": t.a()}))
    );
    let v: Value = serde_json::from_str(&ok(t.run(&["list-panes", "--json"]))).unwrap();
    assert!(v["panes"][0].get("id").is_some());
    assert!(v["panes"][0].get("surface_ids").is_some());
    assert_eq!(
        ok(t.run(&["list-panes", "--workspace", "workspace:2"])),
        "* pane:3  [1 surface]  [focused]\n"
    );
}

#[test]
fn list_pane_surfaces_text() {
    let t = T::new(HashMap::new());
    assert_eq!(
        ok(t.run(&["list-pane-surfaces", "--pane", "pane:1"])),
        "* surface:1  zsh  [selected]\n  surface:2  vim\n"
    );
    assert_eq!(
        ok(t.run(&["list-pane-surfaces"])),
        "* surface:3  htop  [selected]\n"
    );
    // An index resolves through pane.list.
    assert_eq!(
        ok(t.run(&["list-pane-surfaces", "--pane", "0"])),
        "* surface:1  zsh  [selected]\n  surface:2  vim\n"
    );
    assert_eq!(
        err(t.run(&["list-pane-surfaces", "--pane", "7"])),
        "Error: Pane index not found\n"
    );
}

#[test]
fn current_workspace_text() {
    let t = T::new(HashMap::new());
    assert_eq!(ok(t.run(&["current-workspace"])), "workspace:1\n");
    assert_eq!(
        ok(t.run(&["current-workspace", "--id-format", "uuids"])),
        format!("{}\n", t.a())
    );
    assert_eq!(
        ok(t.run(&["current-workspace", "--id-format", "both"])),
        format!("workspace:1 ({})\n", t.a())
    );
}

#[test]
fn select_workspace_by_index() {
    let t = T::new(HashMap::new());
    let reply =
        t.s.with_window(json!({"workspace_id": t.b(), "workspace_ref": "workspace:2"}));
    let t = T {
        fake: Fake::start(
            t.s.clone(),
            HashMap::from([("workspace.select", Ok(reply))]),
        ),
        s: t.s,
    };
    assert_eq!(
        ok(t.run(&["select-workspace", "--workspace", "1"])),
        "OK workspace:2\n"
    );
    assert_eq!(
        t.last(),
        ("workspace.select".into(), json!({"workspace_id": t.b()}))
    );
    assert_eq!(
        err(t.run(&["select-workspace"])),
        "Error: select-workspace requires --workspace\n"
    );
    assert_eq!(
        err(t.run(&["select-workspace", "--workspace", "workspace:9"])),
        "Error: Workspace ref not found: workspace:9\n"
    );
    assert_eq!(
        err(t.run(&["select-workspace", "--workspace", "nope"])),
        "Error: Invalid workspace handle: nope (expected UUID, ref like workspace:1, or index)\n"
    );
    assert_eq!(
        err(t.run(&["select-workspace", "--workspace", "5"])),
        "Error: Workspace index not found\n"
    );
    assert_eq!(
        err(t.run(&["select-workspace", "--workspace", "workspace:2", "x"])),
        "Error: select-workspace: unexpected argument x\n"
    );
    assert_eq!(
        err(t.run(&["select-workspace", "--bogus"])),
        "Error: select-workspace: unknown option --bogus\n"
    );
}

#[test]
fn focus_pane_positional_and_flag() {
    let s = fixture();
    let (a, p1) = (&s.workspaces[0], &s.workspaces[0].panes[0]);
    let reply = s.with_window(json!({
        "workspace_id": a.handle.id, "workspace_ref": "workspace:1",
        "pane_id": p1.handle.id, "pane_ref": "pane:1",
    }));
    let t = T {
        fake: Fake::start(s.clone(), HashMap::from([("pane.focus", Ok(reply))])),
        s,
    };
    for args in [
        &["focus-pane", "pane:1"][..],
        &["focus-pane", "--pane", "pane:1"],
    ] {
        assert_eq!(ok(t.run(args)), "OK pane:1 workspace:1\n");
        assert_eq!(
            t.last(),
            (
                "pane.focus".into(),
                json!({"pane_id": "pane:1", "workspace_id": t.a()})
            )
        );
    }
    assert_eq!(
        err(t.run(&["focus-pane"])),
        "Error: focus-pane requires --pane <id|ref>\n"
    );
}

#[test]
fn focus_panel() {
    let s = fixture();
    let a = &s.workspaces[0];
    let target = cmux_ipc::snapshot::Target {
        ws: a,
        pane: &a.panes[0],
        surface: &a.panes[0].surfaces[1],
    };
    let reply = s.target_payload(&target);
    let t = T {
        fake: Fake::start(s.clone(), HashMap::from([("surface.focus", Ok(reply))])),
        s,
    };
    assert_eq!(
        ok(t.run(&["focus-panel", "--panel", "surface:2"])),
        "OK surface:2 workspace:1\n"
    );
    assert_eq!(t.last().1["surface_id"], json!("surface:2"));
    assert_eq!(
        err(t.run(&["focus-panel"])),
        "Error: focus-panel requires --panel\n"
    );
}

#[test]
fn ping_prints_pong() {
    let t = T::new(HashMap::new());
    assert_eq!(ok(t.run(&["ping"])), "PONG\n");
}

#[test]
fn server_error_is_code_colon_message() {
    let t = T::new(HashMap::from([(
        "workspace.list",
        Err(RpcError::new("not_found", "Workspace not found")),
    )]));
    assert_eq!(
        err(t.run(&["list-workspaces"])),
        "Error: not_found: Workspace not found\n"
    );
}

#[test]
fn window_flag_resolves_through_window_list() {
    let t = T::new(HashMap::new());
    assert_eq!(
        ok(t.run(&["list-workspaces", "--window", "window:1"])),
        "* workspace:1  Terminal  [selected]\n  workspace:2  Build\n"
    );
    assert_eq!(
        t.last(),
        ("workspace.list".into(), json!({"window_id": t.s.window.id}))
    );
    assert_eq!(
        err(t.run(&["--window", "window:4", "list-workspaces"])),
        "Error: Window not found: window:4\n"
    );
    assert_eq!(
        ok(t.run(&["list-workspaces", "--window", "0"]))
            .lines()
            .count(),
        2
    );
}
