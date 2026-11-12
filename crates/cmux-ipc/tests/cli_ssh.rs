//! `cmux ssh` (M11 Part 3): the argv it sends on `workspace.create`.

mod support;

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;

use serde_json::json;
use support::{T, err, ok, temp_dir};

#[test]
fn ssh_creates_a_remote_workspace() {
    let t = T::new(HashMap::from([(
        "workspace.create",
        Ok(json!({"workspace_id": "W3", "workspace_ref": "workspace:3"})),
    )]));
    let state = temp_dir();
    let env = [("XDG_STATE_HOME", state.to_str().unwrap()), ("HOME", "/h")];
    let out = t.run_env(
        &[
            "ssh",
            "dev@box",
            "--port=2222",
            "--identity",
            "~/.ssh/k",
            "--ssh-option",
            "User=x",
            "--ssh-option",
            "Compression=yes",
            "--command",
            "htop",
        ],
        &env,
    );
    assert_eq!(ok(out), "OK workspace:3\n");
    let dir = state.join("cmux-linux/ssh");
    let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700);
    let (method, p) = t.last();
    assert_eq!(method, "workspace.create");
    let cp = format!("ControlPath={}/%C", dir.display());
    let want: Vec<&str> = vec![
        "ssh",
        "-tt",
        "-o",
        "ControlMaster=auto",
        "-o",
        &cp,
        "-o",
        "ControlPersist=600",
        "-o",
        "StrictHostKeyChecking=accept-new",
        "-o",
        "ServerAliveInterval=20",
        "-o",
        "ServerAliveCountMax=2",
        "-o",
        "SetEnv=COLORTERM=truecolor",
        "-p",
        "2222",
        "-i",
        "/h/.ssh/k",
        "-o",
        "User=x",
        "-o",
        "Compression=yes",
        "dev@box",
    ];
    assert_eq!(
        p,
        json!({"remote": want, "title": "dev@box", "initial_input": "htop\r", "focus": true,
               "workspace_id": t.a(), "surface_id": t.s1()})
    );
    ok(t.run_env(&["ssh", "h", "--name", "Box", "--focus", "false"], &env));
    let p = t.last().1;
    assert_eq!((&p["title"], &p["focus"]), (&json!("Box"), &json!(false)));
    assert!(p.get("initial_input").is_none());
}

#[test]
fn ssh_rejects_bad_input() {
    let t = T::new(HashMap::new());
    let state = temp_dir();
    let env = [("XDG_STATE_HOME", state.to_str().unwrap())];
    for (args, want) in [
        (vec!["ssh"], "Usage: cmux ssh"),
        (vec!["ssh", "-oProxyCommand=x"], "invalid destination"),
        (vec!["ssh", "h", "--port", "0"], "--port must be"),
        (vec!["ssh", "h", "--port", "x"], "--port must be"),
        (
            vec!["ssh", "h", "--transport", "mosh"],
            "unknown option --transport",
        ),
        (vec!["ssh", "h", "i"], "unexpected argument i"),
        (
            vec!["ssh", "h", "--ssh-option"],
            "--ssh-option requires a value",
        ),
    ] {
        let e = err(t.run_env(&args, &env));
        assert!(e.contains(want), "{args:?}: {e}");
    }
    assert!(t.fake.requests().is_empty());
}
