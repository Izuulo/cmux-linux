mod support;

use std::collections::HashMap;

use cmux_ipc::wire::{Reply, RpcError};
use serde_json::{Value, json};
use support::{T, err, ok};

fn t(canned: &[(&'static str, Reply)]) -> T {
    T::new(HashMap::new()).with(canned.iter().cloned().collect())
}

fn sent() -> Reply {
    Ok(json!({"id": "M1", "state": "queued", "recipient_workspace_title": "Build"}))
}

#[test]
fn message_sends_target_body_and_where_it_came_from() {
    let t = t(&[("agent.message.send", sent())]);
    assert_eq!(
        ok(t.run(&[
            "agent",
            "message",
            "Build",
            "--from",
            "reviewer",
            "Review posted."
        ])),
        "OK M1 queued for Build\n"
    );
    assert_eq!(
        t.last(),
        (
            "agent.message.send".into(),
            json!({
                "target": "Build", "body": "Review posted.", "from": "reviewer",
                "sender_workspace_id": t.a(), "sender_surface_id": t.s1(),
            })
        )
    );
}

#[test]
fn message_reads_stdin_and_replies() {
    let t = t(&[("agent.message.send", sent())]);
    let args = ["agent", "message", "surface:4", "-"];
    let out = t.run_with_stdin(&args, &[], "long\ntext\n");
    assert_eq!(out.0, 0, "{}", out.2);
    assert_eq!(t.last().1["body"], json!("long\ntext\n"));
    ok(t.run(&["agent", "message", "--reply-to", "M0", "Done."]));
    let p = t.last().1;
    assert_eq!(
        (&p["reply_to"], &p["body"]),
        (&json!("M0"), &json!("Done."))
    );
    assert_eq!(p.get("target"), None);
}

#[test]
fn message_usage_and_server_errors() {
    let refused = Err(RpcError::new(
        "not_found",
        "No workspace or surface matches 'x'",
    ));
    let t = t(&[("agent.message.send", refused)]);
    assert!(
        err(t.run(&["agent", "message", "only-target"])).starts_with("Error: Usage: cmux agent")
    );
    assert!(err(t.run(&["agent"])).starts_with("Error: Usage: cmux agent"));
    assert!(err(t.run(&["agent", "message", "--reply-to", "M0"])).starts_with("Error: Usage"));
    assert_eq!(
        err(t.run(&["agent", "message", "x", "hi"])),
        "Error: not_found: No workspace or surface matches 'x'\n"
    );
}

fn row(id: &str, state: &str, body: &str) -> Value {
    json!({"id": id, "state": state, "from": "a", "body": body, "recipient_surface_ref": "surface:4"})
}

#[test]
fn inbox_lists_newest_first_as_the_server_sends_them() {
    let list =
        Ok(json!({"messages": [row("M2", "queued", "two\nmore"), row("M1", "read", "one")]}));
    let t = t(&[
        ("agent.message.list", list),
        ("agent.message.mark_read", Ok(json!({"read": ["M2"]}))),
    ]);
    assert_eq!(
        ok(t.run(&[
            "agent",
            "inbox",
            "--surface",
            "Build",
            "--state",
            "queued",
            "--limit",
            "5"
        ])),
        "queued     M2  a -> surface:4: two\nread       M1  a -> surface:4: one\n"
    );
    assert_eq!(
        t.last(),
        (
            "agent.message.list".into(),
            json!({"surface": "Build", "state": "queued", "limit": 5})
        )
    );
    ok(t.run(&["agent", "inbox", "--mark-read"]));
    assert_eq!(
        t.last(),
        (
            "agent.message.mark_read".into(),
            json!({"ids": ["M2", "M1"]})
        )
    );
    assert_eq!(
        err(t.run(&["agent", "inbox", "--limit", "many"])),
        "Error: agent inbox: --limit must be a number\n"
    );
}

#[test]
fn inbox_empty() {
    let t = t(&[("agent.message.list", Ok(json!({"messages": []})))]);
    assert_eq!(ok(t.run(&["agent", "inbox"])), "No messages\n");
}

#[test]
fn automation_lists_rules_and_warnings() {
    let reply = Ok(
        json!({"path": "/c/automations.json", "warnings": ["rule `x`: bad"], "rules": [
            {"id": "a", "enabled": true, "events": ["workspace.*"], "categories": [], "actions": 2, "firings": 3},
            {"id": "b", "enabled": false, "events": [], "categories": ["feed"], "actions": 1, "firings": 0},
        ]}),
    );
    let t = t(&[
        ("automation.list", reply),
        ("automation.reload", Ok(json!({"rules": []}))),
    ]);
    assert_eq!(
        ok(t.run(&["automation", "list"])),
        "a  workspace.*  actions=2 firings=3\nb  feed  actions=1 firings=0  [disabled]\nwarning: rule `x`: bad\n"
    );
    assert_eq!(ok(t.run(&["automation", "reload"])), "No automations\n");
    assert_eq!(t.last().0, "automation.reload");
    assert_eq!(
        err(t.run(&["automation"])),
        "Error: Usage: cmux automation <list|reload>\n"
    );
}
