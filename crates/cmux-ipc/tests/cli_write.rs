mod support;

use std::collections::HashMap;

use serde_json::{Value, json};
use support::{T, err, ok};

/// A fake whose mutating methods answer with the A/S1 target payload, plus `extra` fields.
fn t(extra: Value) -> T {
    let base = T::new(HashMap::new());
    let s = &base.s;
    let a = &s.workspaces[0];
    let target = cmux_ipc::snapshot::Target {
        ws: a,
        pane: &a.panes[0],
        surface: &a.panes[0].surfaces[0],
    };
    let mut payload = s.target_payload(&target);
    for (k, v) in extra.as_object().unwrap() {
        payload[k] = v.clone();
    }
    let canned = [
        "surface.split",
        "surface.send_text",
        "surface.send_key",
        "terminal.paste",
    ]
    .into_iter()
    .map(|m| (m, Ok(payload.clone())))
    .chain([(
        "workspace.create",
        Ok(s.with_window(json!({
            "workspace_id": "W3", "workspace_ref": "workspace:3",
            "surface_id": "S9", "surface_ref": "surface:9",
        }))),
    )])
    .collect();
    base.with(canned)
}

#[test]
fn new_workspace_params() {
    let t = t(json!({}));
    for json_flag in [false, true] {
        let mut args = vec![
            "new-workspace",
            "--name",
            "Build",
            "--cwd",
            "~/p",
            "--command",
            "npm test",
        ];
        if json_flag {
            args.push("--json");
        }
        assert_eq!(ok(t.run_env(&args, &[("HOME", "/h")])), "OK workspace:3\n");
        assert_eq!(
            t.last(),
            (
                "workspace.create".into(),
                json!({"title": "Build", "cwd": "/h/p", "initial_input": "npm test\r",
                       "focus": false, "workspace_id": t.a(), "surface_id": t.s1()})
            )
        );
    }
    // --id-format doesn't change it either: always the ref.
    assert_eq!(
        ok(t.run(&["new-workspace", "--id-format", "uuids"])),
        "OK workspace:3\n"
    );
    ok(t.run(&["new-workspace", "--cwd", "."]));
    assert_eq!(t.last().1["cwd"], json!("/work"));
    ok(t.run(&["new-workspace", "--cwd", "sub/../x"]));
    assert_eq!(t.last().1["cwd"], json!("/work/sub/../x"));
    ok(t.run(&["new-workspace", "--focus", "true", "--command", "  "]));
    assert_eq!(t.last().1["focus"], json!(true));
    assert!(
        t.last().1.get("initial_input").is_none(),
        "blank command dropped"
    );
    assert_eq!(
        err(t.run(&["new-workspace", "--focus", "maybe"])),
        "Error: --focus must be true|false\n"
    );
    assert_eq!(
        err(t.run(&["new-workspace", "--name"])),
        "Error: new-workspace: --name requires a value\n"
    );
    assert_eq!(
        err(t.run(&["new-workspace", "--command"])),
        "Error: new-workspace: --command requires <text>\n"
    );
    assert_eq!(
        err(t.run(&["new-workspace", "x"])),
        "Error: new-workspace: unexpected argument x\n"
    );
}

#[test]
fn new_workspace_unknown_flag() {
    let t = t(json!({}));
    assert_eq!(
        err(t.run(&["new-workspace", "--layout", "{}"])),
        "Error: new-workspace: unknown flag '--layout'. Known flags: --name <title>, --cwd <path>, --command <text>, --window <id|ref|index>, --focus <true|false>\n"
    );
    assert!(t.fake.requests().is_empty());
}

#[test]
fn new_split_directions() {
    let t = t(json!({"type": "terminal"}));
    assert_eq!(
        ok(t.run(&["new-split", "right"])),
        "OK surface:1 workspace:1\n"
    );
    assert_eq!(
        t.last(),
        (
            "surface.split".into(),
            json!({"direction": "right", "surface_id": t.s1(), "workspace_id": t.a(),
                   "focus": false})
        )
    );
    for (given, want) in [("U", "up"), ("l", "left"), ("Down", "down"), ("R", "right")] {
        ok(t.run(&["new-split", given]));
        assert_eq!(t.last().1["direction"], json!(want));
    }
    assert_eq!(
        err(t.run(&["new-split"])),
        "Error: new-split requires a direction\n"
    );
    assert_eq!(
        err(t.run(&["new-split", "sideways"])),
        "Error: new-split: direction must be left|right|up|down\n"
    );
    ok(t.run(&[
        "new-split",
        "down",
        "--panel",
        "surface:2",
        "--command",
        "htop",
        "--focus",
        "true",
    ]));
    let p = t.last().1;
    assert_eq!(
        (
            p["surface_id"].clone(),
            p["initial_input"].clone(),
            p["focus"].clone()
        ),
        (json!("surface:2"), json!("htop\r"), json!(true))
    );
    ok(t.run(&["new-split", "left", "--workspace", "workspace:2"]));
    let p = t.last().1;
    assert_eq!(p["workspace_id"], json!(t.b()));
    assert!(p.get("surface_id").is_none());
    assert_eq!(
        err(t.run(&["new-split", "left", "--bogus"])),
        "Error: new-split: unknown flag '--bogus'\n"
    );
    let v: Value = serde_json::from_str(&ok(t.run(&["new-split", "up", "--json"]))).unwrap();
    assert_eq!(v["type"], json!("terminal"));
}

#[test]
fn send_text_rules() {
    let t = t(json!({"queued": false}));
    assert_eq!(
        ok(t.run(&["send", "echo hi\\n"])),
        "OK surface:1 workspace:1\n"
    );
    assert_eq!(
        t.last(),
        (
            "surface.send_text".into(),
            json!({"text": "echo hi\r", "surface_id": t.s1(), "workspace_id": t.a()})
        )
    );
    ok(t.run(&["send", "a\\tb\\rc"]));
    assert_eq!(t.last().1["text"], json!("a\tb\rc"));
    ok(t.run(&["send", "echo", "--force"]));
    assert_eq!(t.last().1["text"], json!("echo --force"));
    ok(t.run(&["send", "--force", "--", "-x"]));
    assert_eq!(t.last().1["text"], json!("-x"));
    ok(t.run(&["send", "--surface", "surface:2", "ls"]));
    assert_eq!(t.last().1["surface_id"], json!("surface:2"));
    assert_eq!(err(t.run(&["send"])), "Error: send requires text\n");
    assert_eq!(
        err(t.run(&["send", "--force"])),
        "Error: send requires text\n"
    );
    ok(t.run(&["send", "--paste", "a\\nb"]));
    assert_eq!(t.last().0, "terminal.paste");
    assert_eq!(t.last().1["text"], json!("a\\nb"));
}

#[test]
fn send_key_rules() {
    let t = t(json!({"queued": false}));
    assert_eq!(
        ok(t.run(&["send-key", "ctrl+c"])),
        "OK surface:1 workspace:1\n"
    );
    assert_eq!(
        t.last(),
        (
            "surface.send_key".into(),
            json!({"key": "ctrl+c", "surface_id": t.s1(), "workspace_id": t.a()})
        )
    );
    ok(t.run(&["send-key", "--force", "--", "enter"]));
    assert_eq!(t.last().1["key"], json!("enter"));
    let before = t.fake.requests().len();
    assert_eq!(
        err(t.run(&["send-key", "a", "b"])),
        "Error: send-key: unexpected arguments: b\n"
    );
    assert_eq!(t.fake.requests().len(), before, "nothing sent");
    assert_eq!(
        err(t.run(&["send-key"])),
        "Error: send-key requires a key\n"
    );
}

#[test]
fn queued_suffix() {
    let t = t(json!({"queued": true}));
    assert_eq!(
        ok(t.run(&["send", "x"])),
        "OK surface:1 workspace:1 queued (terminal starting; input will be sent when its PTY is ready)\n"
    );
}
