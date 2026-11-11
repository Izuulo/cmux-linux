mod support;

use std::collections::HashMap;

use cmux_ipc::feed::push_params;
use serde_json::{Value, json};
use support::T;

const LINUX: (&str, &str) = ("CMUX_LINUX", "1");

fn t(result: Value) -> T {
    T::new(HashMap::new()).with_feed(result)
}

fn hook(t: &T, stdin: &str, env: &[(&str, &str)]) -> (i32, Value) {
    let (code, out, err) = t.run_with_stdin(&["hooks", "feed", "--source", "claude"], env, stdin);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(out.matches('\n').count(), 1, "one line: {out:?}");
    (code, serde_json::from_str(&out).unwrap())
}

const BASH: &str =
    r#"{"session_id":"s","cwd":"/w","tool_name":"Bash","tool_input":{"command":"ls"}}"#;

#[test]
fn feed_hook_allow_once() {
    let t = t(json!({"status":"resolved","decision":{"kind":"permission","mode":"once"}}));
    let (_, v) = hook(&t, BASH, &[LINUX]);
    assert_eq!(
        v,
        json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}})
    );
    let (method, params) = t.last();
    assert_eq!(method, "feed.push");
    let id = params["event"]["_opencode_request_id"].as_str().unwrap();
    assert_eq!(id, id.to_uppercase());
    assert_eq!(id.len(), 36);
    let stdin: Value = serde_json::from_str(BASH).unwrap();
    let want = push_params(&stdin, &t.a(), &t.s1(), id);
    assert_eq!(params, Value::Object(want));
    assert_eq!(params["wait_timeout_seconds"], json!(115));
}

#[test]
fn feed_hook_plan_auto() {
    let t = t(json!({"status":"resolved","decision":{"kind":"exit_plan","mode":"autoAccept"}}));
    let stdin = r#"{"tool_name":"ExitPlanMode","tool_input":{"plan":"p"}}"#;
    let (_, v) = hook(&t, stdin, &[LINUX]);
    let d = &v["hookSpecificOutput"]["decision"];
    assert_eq!(d["behavior"], json!("allow"));
    assert_eq!(d["updatedInput"], json!({"plan":"p"}));
    assert_eq!(
        d["updatedPermissions"],
        json!([{"type":"setMode","mode":"auto","destination":"session"}])
    );
}

#[test]
fn feed_hook_question_with_non_object_tool_input() {
    let t = t(json!({"status":"resolved","decision":{"kind":"question","selections":["a"]}}));
    let (_, v) = hook(
        &t,
        r#"{"tool_name":"AskUserQuestion","tool_input":"x"}"#,
        &[LINUX],
    );
    assert_eq!(
        v["hookSpecificOutput"]["decision"]["behavior"],
        json!("allow")
    );
}

#[test]
fn feed_hook_timed_out_prints_empty() {
    let t = t(json!({"status":"timed_out"}));
    assert_eq!(hook(&t, BASH, &[LINUX]).1, json!({}));
    assert_eq!(t.last().0, "feed.push");
}

#[test]
fn feed_hook_outside_cmux_linux_prints_empty() {
    let t = t(json!({"status":"timed_out"}));
    assert_eq!(hook(&t, BASH, &[]).1, json!({}));
    assert_eq!(hook(&t, BASH, &[("CMUX_LINUX", "0")]).1, json!({}));
    assert!(t.fake.requests().is_empty());
    let (code, out, _) = t.run_with_stdin(
        &["hooks", "feed", "--source", "claude"],
        &[LINUX, ("CMUX_SURFACE_ID", "nope")],
        BASH,
    );
    assert_eq!((code, out.trim()), (0, "{}"));
    assert!(t.fake.requests().is_empty());
}

#[test]
fn feed_hook_without_socket_prints_empty() {
    let t = t(json!({}));
    let (_, v) = hook(
        &t,
        BASH,
        &[LINUX, ("CMUX_SOCKET_PATH", "/nonexistent/cmux.sock")],
    );
    assert_eq!(v, json!({}));
}

#[test]
fn feed_hook_bad_stdin_prints_empty() {
    let t = t(json!({"status":"timed_out"}));
    for s in ["", "not json", "[1]", "\"x\""] {
        assert_eq!(hook(&t, s, &[LINUX]).1, json!({}));
    }
    assert!(t.fake.requests().is_empty());
}

#[test]
fn feed_hook_other_source_prints_empty() {
    let t = t(json!({"status":"timed_out"}));
    let (code, out, _) = t.run_with_stdin(&["hooks", "feed", "--source", "codex"], &[LINUX], BASH);
    assert_eq!((code, out.trim()), (0, "{}"));
    assert!(t.fake.requests().is_empty());
}

#[test]
fn feed_hook_requires_source() {
    let t = t(json!({}));
    let (code, _, err) = t.run_with_stdin(&["hooks", "feed"], &[LINUX], BASH);
    assert_eq!(code, 1);
    assert_eq!(
        err,
        "Error: cmux hooks feed requires --source <agent-name>\n"
    );
}
