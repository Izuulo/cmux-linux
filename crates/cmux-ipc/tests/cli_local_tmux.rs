//! `cmux local-tmux` against a fake tmux that logs its arguments (M10 Part 3).

mod support;

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;

use serde_json::json;
use support::{T, temp_dir};

/// A fake tmux: logs one line per call; `has-session` succeeds only for `=live`.
fn fake_tmux() -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = temp_dir();
    let log = dir.join("log");
    let bin = dir.join("tmux");
    std::fs::write(
        &bin,
        format!(
            "#!/bin/sh\necho \"$*\" >> '{}'\ncase \"$*\" in *has-session*=live*) exit 0;; *has-session*) exit 1;; esac\n",
            log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    (bin, log)
}

#[test]
fn start_creates_then_attaches_the_caller() {
    let t = T::new(HashMap::from([("surface.send_text", Ok(json!({})))]));
    let (bin, log) = fake_tmux();
    let state = temp_dir();
    let env = [
        ("CMUX_LOCAL_TMUX_BIN", bin.to_str().unwrap()),
        ("XDG_STATE_HOME", state.to_str().unwrap()),
    ];
    let (code, out, err) = t.run_env(
        &["local-tmux", "start", "work", "--command", "sleep 9"],
        &env,
    );
    assert_eq!((code, out.as_str()), (0, "OK work\n"), "{err}");
    let sock = state.join("cmux-linux/local-tmux/tmux.sock");
    let calls = std::fs::read_to_string(&log).unwrap();
    let s = sock.display();
    assert_eq!(
        calls,
        format!(
            "-S {s} has-session -t =work\n-S {s} start-server ; set-option -g history-limit 10000 ; new-session -d -s work sleep 9\n"
        )
    );
    let mode = std::fs::metadata(sock.parent().unwrap())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700);
    let (method, params) = t.last();
    assert_eq!(method, "surface.send_text");
    assert_eq!(params["surface_id"], t.s1());
    let b = bin.display();
    assert_eq!(
        params["text"],
        format!(
            "'{b}' -S '{s}' has-session -t =work && TMUX= CMUX_LOCAL_TMUX=1 '{b}' -S '{s}' attach-session -t =work\n"
        )
    );
}

#[test]
fn errors_name_attach_and_missing_tmux() {
    let t = T::new(HashMap::from([("surface.send_text", Ok(json!({})))]));
    let (bin, _) = fake_tmux();
    let state = temp_dir();
    let env = [
        ("CMUX_LOCAL_TMUX_BIN", bin.to_str().unwrap()),
        ("XDG_STATE_HOME", state.to_str().unwrap()),
    ];
    let (code, _, err) = t.run_env(&["local-tmux", "start", "a.b"], &env);
    assert_eq!(code, 1);
    assert!(err.contains("invalid name"), "{err}");
    let (code, _, err) = t.run_env(&["local-tmux", "start", "live", "--detached"], &env);
    assert_eq!(code, 1);
    assert!(err.contains("already exists"), "{err}");
    let (code, _, err) = t.run_env(&["local-tmux", "attach", "gone"], &env);
    assert_eq!(code, 1);
    assert!(err.contains("no session `gone`"), "{err}");
    let (code, out, _) = t.run_env(&["local-tmux", "attach", "live"], &env);
    assert_eq!((code, out.as_str()), (0, "OK live\n"));
    let missing = [
        ("CMUX_LOCAL_TMUX_BIN", "/nonexistent/tmux"),
        ("XDG_STATE_HOME", state.to_str().unwrap()),
    ];
    let (code, _, err) = t.run_env(&["local-tmux", "list"], &missing);
    assert_eq!(code, 1);
    assert!(err.contains("not found"), "{err}");
}
