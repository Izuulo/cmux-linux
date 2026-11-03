//! Real PTYs running `sh`. Unix only, like the rest of M1.
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cmux_term::pty::{CommandBuilder, PtyEvent, Session};

/// Runs `script` under `sh -c`, optionally typing `input`, and returns the screen text after exit.
fn run(script: &str, input: Option<&[u8]>) -> String {
    let (tx, rx) = mpsc::channel();
    let mut cmd = CommandBuilder::new("sh");
    cmd.args(["-c", script]);
    let session = Session::spawn(
        cmd,
        80,
        24,
        |_| {},
        move |ev| {
            let _ = tx.send(matches!(ev, PtyEvent::Exited));
        },
    )
    .unwrap();
    if let Some(input) = input {
        session.write(input.to_vec());
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(true) => break,
            Ok(false) => continue,
            Err(_) => panic!("child did not exit in time"),
        }
    }
    let lines = session.term.lock().unwrap().text_lines().unwrap();
    lines.join("\n")
}

#[test]
fn child_sees_the_spawn_size() {
    assert!(run("stty size", None).contains("24 80"));
}

#[test]
fn written_bytes_reach_the_child() {
    assert!(run("read line; echo \"got:$line\"", Some(b"abc\r")).contains("got:abc"));
}

#[test]
fn terminal_replies_are_written_back_to_the_child() {
    // The DSR reply ESC [ 1 ; 1 R comes from Term's on_pty_write via the writer thread.
    let out = run(
        "stty -icanon -echo; printf '\\033[6n'; head -c 6 | od -An -tx1",
        None,
    );
    assert!(out.contains("1b 5b 31 3b 31 52"), "{out}");
}

#[test]
fn shell_command_sets_term_and_colorterm() {
    let cmd = Session::shell_command();
    assert_eq!(
        cmd.get_env("TERM").and_then(|v| v.to_str()),
        Some("xterm-256color")
    );
    assert_eq!(
        cmd.get_env("COLORTERM").and_then(|v| v.to_str()),
        Some("truecolor")
    );
}
