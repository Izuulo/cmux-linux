mod support;

use std::collections::HashMap;

use cmux_ipc::handles::{Handle, Kind};
use cmux_ipc::snapshot::{GroupSnap, Snapshot};
use cmux_ipc::wire::{Reply, RpcError};
use serde_json::{Map, Value, json};
use support::{Fake, fixture};

const G: &str = "0000AAAA-0000-4000-8000-000000000001";

/// The fixture with one pinned group "api" holding A (anchor) and B.
fn grouped() -> Snapshot {
    let mut s = fixture();
    s.groups.push(GroupSnap {
        key: 1,
        handle: Handle {
            id: G.into(),
            r#ref: format!("{}:1", Kind::WorkspaceGroup.name()),
        },
        name: "api".into(),
        collapsed: false,
        pinned: true,
        generated_anchor: false,
        members: vec![1, 2],
    });
    s
}

fn fake_on(s: Snapshot, canned: &[(&'static str, Reply)]) -> Fake {
    Fake::start(s, canned.iter().cloned().collect::<HashMap<_, _>>())
}

fn fake(canned: &[(&'static str, Reply)]) -> Fake {
    fake_on(grouped(), canned)
}

fn run(f: &Fake, args: &[&str]) -> (i32, String, String) {
    let mut all = vec!["workspace-group"];
    all.extend_from_slice(args);
    let out = f.run(&all, &[]);
    (out.code, out.stdout, out.stderr)
}

fn last(f: &Fake) -> (String, Value) {
    f.requests().last().cloned().unwrap()
}

#[test]
fn list_prints_cmux_rows() {
    let f = fake(&[]);
    assert_eq!(
        run(&f, &["list"]).1,
        "workspace_group:1  api  (2 members) [pinned]\n"
    );
    let none = Fake::start(fixture(), HashMap::new());
    assert_eq!(run(&none, &["list"]).1, "No groups\n");
}

#[test]
fn group_payload_names_anchor_and_members() {
    let s = grouped();
    let (a, b) = (&s.workspaces[0].handle, &s.workspaces[1].handle);
    let v = s
        .read("workspace.group.list", &Map::new())
        .unwrap()
        .unwrap();
    assert_eq!(
        v["groups"],
        json!([{
            "id": G, "ref": "workspace_group:1", "name": "api",
            "is_collapsed": false, "is_pinned": true, "is_empty": false,
            "anchor_workspace_id": a.id, "anchor_workspace_ref": "workspace:1",
            "member_workspace_ids": [a.id, b.id],
            "member_workspace_refs": ["workspace:1", "workspace:2"],
            "member_count": 2,
            "anchor_workspace_provenance": "user", "anchor_workspace_is_generated": false,
        }])
    );
}

#[test]
fn group_id_takes_a_uuid_or_ref() {
    let s = grouped();
    let p = |v: &str| json!({ "group_id": v }).as_object().unwrap().clone();
    assert_eq!(s.group(&p(G)).unwrap().key, 1);
    assert_eq!(s.group(&p(&G.to_lowercase())).unwrap().key, 1);
    assert_eq!(s.group(&p("workspace_group:1")).unwrap().key, 1);
    let bad_ref = s.group(&p("workspace_group:7")).unwrap_err();
    assert_eq!(
        (bad_ref.code.as_str(), bad_ref.message.as_str()),
        ("invalid_params", "Missing or invalid group_id")
    );
    let unknown = s
        .group(&p("0000AAAA-0000-4000-8000-00000000000F"))
        .unwrap_err();
    assert_eq!(
        (unknown.code.as_str(), unknown.message.as_str()),
        ("not_found", "Group not found")
    );
    assert_eq!(s.group(&Map::new()).unwrap_err().code, "invalid_params");
}

#[test]
fn create_resolves_from_and_prints_the_group() {
    let reply = json!({ "group": { "id": G, "ref": "workspace_group:1" }, "created": true });
    let s = grouped();
    let f = fake_on(s.clone(), &[("workspace.group.create", Ok(reply))]);
    let out = run(&f, &["create", "--name", "api", "--from", "workspace:1,1"]);
    assert_eq!(out.1, "OK workspace_group:1\n", "{}", out.2);
    let ids: Vec<&str> = s.workspaces.iter().map(|w| w.handle.id.as_str()).collect();
    assert_eq!(
        last(&f),
        (
            "workspace.group.create".into(),
            json!({ "name": "api", "child_workspace_ids": [ids[0], ids[1]] }),
        ),
        "a ref and a 0-based index both become ids"
    );
}

#[test]
fn create_without_from_makes_an_empty_group_in_cwd() {
    let reply = json!({ "group": { "id": G, "ref": "workspace_group:1" }, "created": true });
    let f = fake(&[("workspace.group.create", Ok(reply))]);
    assert_eq!(
        run(&f, &["create", "--cwd", "/srv/x"]).1,
        "OK workspace_group:1\n"
    );
    assert_eq!(last(&f).1, json!({ "name": "", "cwd": "/srv/x" }));
}

#[test]
fn dissolve_and_delete_lines() {
    let kept =
        |n| Ok(json!({ "group_id": G, "operation": "dissolved", "kept_workspace_count": n }));
    let closed = Ok(json!({
        "group_id": G, "operation": "closed_workspaces", "closed_workspace_count": 3,
    }));
    let f = fake(&[
        ("workspace.group.ungroup", kept(1)),
        ("workspace.group.delete", kept(2)),
    ]);
    assert_eq!(
        run(&f, &["ungroup", "workspace_group:1"]).1,
        "OK group dissolved (kept 1 workspace)\n"
    );
    assert_eq!(
        run(&f, &["delete", "--group", G]).1,
        "OK group dissolved (kept 2 workspaces)\n"
    );
    assert_eq!(last(&f).1, json!({ "group_id": G }));
    let f = fake(&[("workspace.group.delete", closed)]);
    assert_eq!(
        run(&f, &["delete", "workspace_group:1", "--close-workspaces"]).1,
        "OK group deleted (closed 3 workspaces)\n"
    );
    assert_eq!(
        last(&f).1,
        json!({ "group_id": "workspace_group:1", "close_workspaces": true })
    );
}

#[test]
fn simple_subcommands_send_their_method() {
    let ok = Ok(json!({}));
    let methods = ["collapse", "expand", "pin", "unpin", "focus"];
    let canned: Vec<(&'static str, Reply)> = [
        "workspace.group.collapse",
        "workspace.group.expand",
        "workspace.group.pin",
        "workspace.group.unpin",
        "workspace.group.focus",
        "workspace.group.rename",
        "workspace.group.add",
        "workspace.group.set_anchor",
        "workspace.group.remove",
    ]
    .into_iter()
    .map(|m| (m, ok.clone()))
    .collect();
    let s = grouped();
    let f = fake_on(s.clone(), &canned);
    for m in methods {
        assert_eq!(run(&f, &[m, "workspace_group:1"]).1, "OK\n");
        assert_eq!(
            last(&f),
            (
                format!("workspace.group.{m}"),
                json!({ "group_id": "workspace_group:1" })
            )
        );
    }
    assert_eq!(run(&f, &["rename", G, "--name", "web"]).1, "OK\n");
    assert_eq!(last(&f).1, json!({ "group_id": G, "name": "web" }));
    let a = s.workspaces[0].handle.id.clone();
    for (sub, method) in [
        ("add", "workspace.group.add"),
        ("set-anchor", "workspace.group.set_anchor"),
    ] {
        assert_eq!(
            run(&f, &[sub, "--group", G, "--workspace", "workspace:1"]).1,
            "OK\n"
        );
        assert_eq!(
            last(&f),
            (method.into(), json!({ "group_id": G, "workspace_id": a }))
        );
    }
    assert_eq!(run(&f, &["remove", "--workspace", "workspace:1"]).1, "OK\n");
    assert_eq!(last(&f).1, json!({ "workspace_id": a }));
}

#[test]
fn new_workspace_prints_the_workspace_ref() {
    let reply = json!({ "group_id": G, "workspace_id": "W9", "workspace_ref": "workspace:9" });
    let f = fake(&[("workspace.group.new_workspace", Ok(reply))]);
    assert_eq!(
        run(
            &f,
            &["new-workspace", "workspace_group:1", "--placement", "top"]
        )
        .1,
        "OK workspace:9\n"
    );
    assert_eq!(
        last(&f).1,
        json!({ "group_id": "workspace_group:1", "placement": "top" })
    );
}

#[test]
fn usage_and_server_errors() {
    let denied = Err(RpcError::new("not_found", "Group not found"));
    let f = fake(&[("workspace.group.pin", denied)]);
    let e = |args: &[&str]| {
        let out = run(&f, args);
        assert_eq!(out.0, 1, "{}", out.1);
        out.2
    };
    assert_eq!(
        e(&["pin"]),
        "Error: workspace-group pin requires a group id or --group <id>\n"
    );
    assert_eq!(e(&["pin", G]), "Error: not_found: Group not found\n");
    assert_eq!(
        e(&["rename", G]),
        "Error: workspace-group rename requires --name <name>\n"
    );
    assert_eq!(
        e(&["add", "--group", G]),
        "Error: workspace-group add requires --group <id> --workspace <id>\n"
    );
    assert_eq!(
        e(&["remove"]),
        "Error: workspace-group remove requires --workspace <id>\n"
    );
    assert_eq!(
        e(&["shuffle"]),
        "Error: Unknown workspace-group subcommand: shuffle\n"
    );
    assert_eq!(
        e(&["pin", G, "--loud"]),
        "Error: workspace-group pin: unknown option --loud\n"
    );
}
