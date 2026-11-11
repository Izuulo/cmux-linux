//! `cmux __tmux-compat` against the fake app (M9 plan T3). The fixture's workspace A has
//! surface:1 "zsh" (the caller), surface:2 "vim" and surface:3 "htop" (focused).

mod support;

use std::collections::HashMap;

use cmux_ipc::wire::Reply;
use serde_json::json;
use support::{T, err, ok};

fn t() -> T {
    let split: Reply = Ok(json!({"surface_id": "NEW", "surface_ref": "surface:9"}));
    T::new(HashMap::new()).with(HashMap::from([
        ("surface.split", split),
        ("surface.send_text", Ok(json!({}))),
        ("surface.close", Ok(json!({}))),
        ("surface.focus", Ok(json!({}))),
    ]))
}

fn tmux(t: &T, args: &[&str]) -> (i32, String, String) {
    let mut all = vec!["__tmux-compat"];
    all.extend(args);
    t.run(&all)
}

#[test]
fn split_window_splits_the_target_and_prints_the_new_pane() {
    let t = t();
    let out = tmux(
        &t,
        &[
            "split-window",
            "-t",
            "%2",
            "-h",
            "-dP",
            "-F",
            "#{pane_id}",
            "-c",
            "/w dir",
            "claude",
            "--agent-id",
            "a",
        ],
    );
    assert_eq!(ok(out), "%9\n");
    let (method, params) = t.last();
    assert_eq!(method, "surface.split");
    assert_eq!(params["direction"], "right");
    assert_eq!(params["focus"], false);
    assert_eq!(
        params["initial_input"],
        "cd '/w dir' && claude --agent-id a\n"
    );
    let vim = t.fake.requests()[0].clone();
    assert_eq!(vim.0, "surface.list");
    // No target: the caller's own terminal, split downwards, focused, nothing typed.
    assert_eq!(ok(tmux(&t, &["split-window"])), "");
    let (_, params) = t.last();
    assert_eq!(params["surface_id"], t.s1());
    assert_eq!(params["direction"], "down");
    assert_eq!(params["focus"], true);
    assert_eq!(params.get("initial_input"), None);
}

#[test]
fn send_keys_types_into_the_pane() {
    let t = t();
    assert_eq!(
        ok(tmux(
            &t,
            &["send-keys", "-t", "%1", "cd /x && claude", "Enter"]
        )),
        ""
    );
    let (method, params) = t.last();
    assert_eq!(method, "surface.send_text");
    assert_eq!(
        params,
        json!({"surface_id": t.s1(), "text": "cd /x && claude\r"})
    );
    assert_eq!(
        err(tmux(&t, &["send-keys", "-t", "%42", "x"])),
        "Error: can't find pane: %42\n"
    );
}

#[test]
fn panes_are_listed_and_described() {
    let t = t();
    assert_eq!(
        ok(tmux(
            &t,
            &[
                "list-panes",
                "-F",
                "#{pane_id} #{pane_title} #{pane_active}"
            ]
        )),
        "%1 zsh 0\n%2 vim 0\n%3 htop 1\n"
    );
    assert_eq!(
        ok(tmux(&t, &["display-message", "-p", "#{pane_id}"])),
        "%1\n"
    );
    // TMUX_PANE names the caller for a lead whose focus has moved on.
    let out = t.run_env(
        &[
            "__tmux-compat",
            "display-message",
            "-p",
            "-F",
            "#{session_name}:#{window_index} #{pane_id}",
        ],
        &[("TMUX_PANE", "%2")],
    );
    assert_eq!(ok(out), "cmux:0 %2\n");
    assert_eq!(
        ok(tmux(&t, &["-L", "sock", "display-message", "hidden"])),
        ""
    );
    assert_eq!(ok(tmux(&t, &["-V"])), "tmux 3.4\n");
}

#[test]
fn kill_select_noops_and_the_unsupported() {
    let t = t();
    assert_eq!(ok(tmux(&t, &["kill-pane", "-t", "%3"])), "");
    assert_eq!(t.last().0, "surface.close");
    assert_eq!(ok(tmux(&t, &["select-pane", "-t", "%2"])), "");
    assert_eq!(t.last().0, "surface.focus");
    let asked = t.fake.requests().len();
    for args in [
        &["select-pane", "-t", "%2", "-T", "researcher"][..],
        &["select-layout", "-t", "%1", "main-vertical"],
        &["resize-pane", "-t", "%1", "-x", "30%"],
        &[
            "set-option",
            "-p",
            "-t",
            "%2",
            "pane-border-style",
            "fg=blue",
        ],
        &["has-session", "-t", "cmux"],
    ] {
        assert_eq!(ok(tmux(&t, args)), "", "{args:?}");
    }
    assert_eq!(t.fake.requests().len(), asked, "no-ops ask the app nothing");
    assert_eq!(
        err(tmux(&t, &["capture-pane", "-p"])),
        "Error: Unsupported tmux compatibility command: capture-pane\n"
    );
}

#[test]
fn claude_teams_needs_one_of_our_terminals() {
    let t = t();
    assert_eq!(
        err(t.run(&["claude-teams", "--continue"])),
        "Error: cmux claude-teams must be run in a cmux-linux terminal\n"
    );
    assert!(ok(t.run(&["claude-teams", "--help"])).starts_with("cmux claude-teams\n\nUsage:"));
}

#[test]
fn respawn_pane_runs_the_command_from_a_script() {
    let home = std::env::temp_dir().join(format!("cmux-respawn-test-{}", std::process::id()));
    let env = [
        ("HOME", home.to_str().unwrap()),
        ("PATH", "/shim:/usr/bin"),
        ("TMUX", "/tmp/cmux-claude-teams/W,0,1"),
    ];
    let t = t();
    let command = "cd /w && env A='b c' claude --agent-id a";
    let args = [
        "__tmux-compat",
        "respawn-pane",
        "-k",
        "-t",
        "%2",
        "-c",
        "/w dir",
        command,
    ];
    assert_eq!(ok(t.run_env(&args, &env)), "");
    let sent = t.fake.requests();
    assert_eq!(
        sent[sent.len() - 2].1["text"],
        "\x03",
        "ends the placeholder first"
    );
    let (method, params) = t.last();
    assert_eq!(method, "surface.send_text");
    let line = params["text"].as_str().unwrap();
    let file = line
        .strip_prefix("exec sh '")
        .and_then(|l| l.strip_suffix("'\n"))
        .expect(line);
    assert_eq!(
        std::fs::read_to_string(file).unwrap(),
        format!(
            "rm -f \"$0\"\nexport PATH='/shim:/usr/bin'\nexport TMUX='/tmp/cmux-claude-teams/W,0,1'\n\
             export TMUX_PANE='%2'\ncd '/w dir' || exit\n{command}\n"
        )
    );
    std::fs::remove_dir_all(home).unwrap();
    assert!(err(tmux(&t, &["respawn-pane", "-k", "-t", "%2"])).contains("needs a command"));
}
