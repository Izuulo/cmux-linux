mod support;

use std::collections::HashMap;

use cmux_ipc::wire::RpcError;
use serde_json::json;
use support::{T, err, ok};

#[test]
fn browser_open_targets_the_callers_surface() {
    let base = T::new(HashMap::new());
    let a = &base.s.workspaces[0];
    let target = cmux_ipc::snapshot::Target {
        ws: a,
        pane: &a.panes[0],
        surface: &a.panes[0].surfaces[0],
    };
    let mut payload = base.s.target_payload(&target);
    payload["type"] = json!("browser");
    let t = base.with(HashMap::from([("browser.open", Ok(payload))]));

    assert_eq!(
        ok(t.run(&["browser", "open", "https://example.org"])),
        "OK surface:1 pane:1\n"
    );
    assert_eq!(
        t.last(),
        (
            "browser.open".into(),
            json!({"url": "https://example.org", "workspace_id": t.a(), "surface_id": t.s1()})
        )
    );
    ok(t.run(&[
        "browser",
        "open",
        "--surface",
        "surface:3",
        "file:///tmp/a.html",
    ]));
    assert_eq!(t.last().1["url"], json!("file:///tmp/a.html"));
    assert_eq!(t.last().1["surface_id"], json!("surface:3"));

    assert_eq!(
        err(t.run(&["browser", "open"])),
        "Error: browser open requires a URL\n"
    );
    assert_eq!(
        err(t.run(&["browser", "open", "https://a.b", "--focus", "true"])),
        "Error: browser open: unknown option --focus\n"
    );
    assert_eq!(
        err(t.run(&["browser", "navigate", "https://a.b"])),
        "Error: Usage: cmux browser open <url> [--surface <id|ref|index>]\n"
    );
}

#[test]
fn browser_open_passes_the_apps_error_on() {
    let t = T::new(HashMap::from([(
        "browser.open",
        Err(RpcError::new(
            "unavailable",
            "Built without browser support",
        )),
    )]));
    let (code, _, stderr) = t.run(&["browser", "open", "https://example.org"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("Built without browser support"), "{stderr}");
}
