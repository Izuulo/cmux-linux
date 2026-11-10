mod support;

use std::collections::HashMap;
use std::process::Command;

use cmux_ipc::wire::RpcError;
use serde_json::{Value, json};
use support::{T, err, ok};

const N: &str = "0F8B8D5C-1A2B-4C3D-8E9F-001122334455";

fn t() -> T {
    let base = T::new(HashMap::new());
    let reply = json!({"workspace_id": base.a(), "surface_id": base.s1(), "id": N});
    base.with(HashMap::from([
        ("notification.create", Ok(reply.clone())),
        ("notification.create_for_target", Ok(reply.clone())),
        ("notification.create_for_caller", Ok(reply)),
        ("notification.clear", Ok(json!({"cleared": true}))),
        ("notification.dismiss", Ok(json!({"dismissed": 1}))),
    ]))
}

#[test]
fn notify_for_caller() {
    let t = t();
    assert_eq!(
        ok(t.run(&["notify", "--title", "Done", "--body", "Task complete"])),
        format!("OK notification:{N}\n")
    );
    assert_eq!(
        t.last(),
        (
            "notification.create_for_caller".into(),
            json!({"title": "Done", "subtitle": "", "body": "Task complete", "caller": true,
                   "prefer_tty": false, "preferred_workspace_id": t.a(),
                   "preferred_surface_id": t.s1()})
        )
    );
    assert_eq!(
        ok(t.run(&["notify", "--title", "Done", "--id-format", "uuids"])),
        format!("OK {N}\n")
    );
    assert_eq!(
        ok(t.run(&["--id-format", "both", "notify"])),
        format!("OK notification:{N} ({N})\n")
    );
    assert_eq!(t.last().1["title"], json!("Notification"));
    let v: Value = serde_json::from_str(&ok(t.run(&["notify", "--json"]))).unwrap();
    assert_eq!(v["id"], json!(N));

    let nameless = t.with(HashMap::from([(
        "notification.create_for_caller",
        Ok(json!({"workspace_id": t.a(), "surface_id": null, "id": null})),
    )]));
    assert_eq!(ok(nameless.run(&["notify"])), "OK\n");
}

#[test]
fn notify_outside_a_terminal_has_no_preferences() {
    let t = t();
    let out = t.fake.run(&["notify", "--title", "x"], &[]);
    assert_eq!(out.code, 0);
    let p = t.last().1;
    assert!(p.get("preferred_workspace_id").is_none());
    assert!(p.get("preferred_surface_id").is_none());
}

#[test]
fn notify_desktop_flag() {
    let t = t();
    ok(t.run(&["notify", "--desktop", "false"]));
    assert_eq!(t.last().1["effects"], json!({"desktop": false}));
    ok(t.run(&["notify", "--desktop=0"]));
    assert_eq!(t.last().1["effects"], json!({"desktop": false}));
    ok(t.run(&["notify", "--desktop", "yes"]));
    assert_eq!(t.last().1["effects"], json!({"desktop": true}));
    ok(t.run(&["notify"]));
    assert!(t.last().1.get("effects").is_none());
    assert_eq!(
        err(t.run(&["notify", "--desktop", "maybe"])),
        "Error: --desktop must be true|false\n"
    );
    ok(t.run(&["notify", "--reply"]));
    assert_eq!(t.last().1["reply_shape"], json!("text"));
}

#[test]
fn notify_routing() {
    let t = t();
    let s2 = t.s.workspaces[0].panes[0].surfaces[1].handle.id.clone();
    ok(t.run(&["notify", "--surface", &s2, "--title", "x"]));
    let (method, p) = t.last();
    assert_eq!(method, "notification.create");
    assert_eq!(p["surface_id"], json!(s2));
    assert!(p.get("workspace_id").is_none() && p.get("window_id").is_none());

    ok(t.run(&[
        "notify",
        "--workspace",
        "workspace:1",
        "--surface",
        "surface:2",
    ]));
    assert_eq!(
        t.last(),
        (
            "notification.create_for_target".into(),
            json!({"workspace_id": t.a(), "surface_id": s2, "title": "Notification",
                   "subtitle": "", "body": ""})
        )
    );
    // A surface ref with no --workspace: the caller's workspace.
    ok(t.run(&["notify", "--surface", "surface:2"]));
    assert_eq!(t.last().1["workspace_id"], json!(t.a()));

    ok(t.run(&["notify", "--window", "window:1"]));
    let (method, p) = t.last();
    assert_eq!(method, "notification.create");
    assert_eq!(
        p["workspace_id"],
        json!(t.a()),
        "the window's current workspace"
    );
    assert_eq!(p["window_id"], json!(t.s.window.id));

    ok(t.run(&["notify", "--workspace", "workspace:2"]));
    let (method, p) = t.last();
    assert_eq!(method, "notification.create_for_caller");
    assert_eq!(p["preferred_workspace_id"], json!(t.b()));
}

#[test]
fn notify_bad_args() {
    let t = t();
    let before = t.fake.requests().len();
    for (args, bad) in [
        (&["notify", "--body"][..], "--body"),
        (&["notify", "--title", "--body", "x"], "--title"),
        (&["notify", "--tab", "0"], "--tab"),
        (&["notify", "--title", "a", "stray"], "stray"),
    ] {
        assert_eq!(
            err(t.run(args)),
            format!("Error: notify: unexpected arguments: {bad}\n"),
            "{args:?}"
        );
    }
    assert_eq!(t.fake.requests().len(), before, "nothing sent");
    ok(t.run(&["notify", "--title=--body"]));
    assert_eq!(t.last().1["title"], json!("--body"));
}

#[test]
fn notify_clear() {
    let t = t();
    assert_eq!(
        ok(t.run(&["notify", "--clear", "--desktop", "maybe"])),
        "OK\n"
    );
    assert_eq!(
        t.last(),
        (
            "notification.clear".into(),
            json!({"caller": true, "prefer_tty": false, "preferred_workspace_id": t.a(),
                   "preferred_surface_id": t.s1()})
        )
    );
    ok(t.run(&["notify", "--clear", "--workspace", "workspace:2"]));
    assert_eq!(t.last().1, json!({"workspace_id": t.b()}));
    ok(t.run(&[
        "notify",
        "--clear",
        "--workspace",
        "workspace:1",
        "--surface",
        "surface:2",
    ]));
    let s2 = t.s.workspaces[0].panes[0].surfaces[1].handle.id.clone();
    assert_eq!(t.last().1, json!({"workspace_id": t.a(), "surface_id": s2}));
    let before = t.fake.requests().len();
    let out = t
        .fake
        .run(&["notify", "--clear", "--surface", "surface:1"], &[]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr,
        "Error: notify --clear --surface requires workspace or window context; specify --workspace or --window\n"
    );
    assert_eq!(t.fake.requests().len(), before);
}

#[test]
fn notify_not_found_exits_1() {
    let t = T::new(HashMap::new()).with(HashMap::from([(
        "notification.create_for_caller",
        Err(RpcError::new("not_found", "Workspace not found")),
    )]));
    assert_eq!(
        err(t.run(&["notify", "--title", "x"])),
        "Error: not_found: Workspace not found\n"
    );
}

#[test]
fn dismiss_rules() {
    let t = t();
    assert_eq!(
        ok(t.run(&["dismiss-notification", "--id", &format!("Notification:{N}")])),
        "OK\n"
    );
    assert_eq!(t.last(), ("notification.dismiss".into(), json!({"id": N})));
    ok(t.run(&["dismiss-notification", "--all-read"]));
    assert_eq!(t.last().1, json!({"all_read": true}));
    let msg = "Error: dismiss-notification requires exactly one of --id or --all-read\n";
    assert_eq!(err(t.run(&["dismiss-notification"])), msg);
    assert_eq!(
        err(t.run(&["dismiss-notification", "--id", N, "--all-read"])),
        msg
    );
    assert_eq!(
        err(t.run(&["dismiss-notification", "x"])),
        "Error: dismiss-notification: unexpected arguments: x\n"
    );
}

/// Spec exit criterion 1: the hook command lines from cmux's docs, run by bash, unchanged.
#[test]
fn hook_commands_work_unchanged() {
    let t = t();
    let bin = std::path::Path::new(env!("CARGO_BIN_EXE_cmux"));
    let path = format!(
        "{}:{}",
        bin.parent().unwrap().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let script = r#"
command -v cmux &>/dev/null && cmux notify --title "Claude Code" --subtitle "Permission" --body "Approval needed"
notification_id="$(cmux notify --title "Done" --body "Task complete" --id-format uuids | awk '$1 == "OK" {print $2}')"
cmux dismiss-notification --id "$notification_id"
"#;
    let out = Command::new("bash")
        .args(["-c", script])
        .env("PATH", path)
        .env("CMUX_SOCKET_PATH", &t.fake.path)
        .env("CMUX_WORKSPACE_ID", t.a())
        .env("CMUX_SURFACE_ID", t.s1())
        .env_remove("CMUX_SOCKET")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let reqs = t.fake.requests();
    let methods: Vec<&str> = reqs.iter().map(|(m, _)| m.as_str()).collect();
    assert_eq!(
        methods,
        [
            "notification.create_for_caller",
            "notification.create_for_caller",
            "notification.dismiss"
        ]
    );
    assert_eq!(reqs[0].1["subtitle"], json!("Permission"));
    assert_eq!(reqs[0].1["preferred_surface_id"], json!(t.s1()));
    assert_eq!(reqs[2].1, json!({"id": N}));
}
