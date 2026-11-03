//! Real PTYs running `sh`. Unix only, like the rest of M1.
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use cmux_term::pty::{CommandBuilder, PtyEvent, Session};

/// Runs `script` under `sh -c`, optionally typing `input`, and returns the screen text after exit.
fn run(script: &str, input: Option<&[u8]>) -> String {
    run_with(script, |session| {
        if let Some(input) = input {
            session.write(input.to_vec());
        }
    })
}

/// Like [`run`], but `before` gets the live session first (to write, resize, ...).
fn run_with(script: &str, before: impl FnOnce(&Session)) -> String {
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
    before(&session);
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

#[test]
fn resize_reaches_the_child() {
    let out = run_with("read x; stty size", |s| {
        s.resize(100, 40, 8, 16).unwrap();
        s.write(b"\n".to_vec());
    });
    assert!(out.contains("40 100"), "{out}");
}

fn alive(pid: u32) -> bool {
    // SAFETY: signal 0 only probes for existence.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[test]
fn drop_returns_promptly_even_when_the_child_ignores_sighup() {
    let mut cmd = CommandBuilder::new("sh");
    cmd.args(["-c", "trap '' HUP; sleep 30"]);
    let session = Session::spawn(cmd, 80, 24, |_| {}, |_| {}).unwrap();
    let pid = session.process_id().unwrap();
    std::thread::sleep(Duration::from_millis(100)); // let sh install the trap
    let t = Instant::now();
    drop(session);
    let took = t.elapsed();
    eprintln!("drop took {took:?}");
    assert!(took < Duration::from_millis(100), "drop took {took:?}");
    let deadline = Instant::now() + Duration::from_secs(2);
    while alive(pid) {
        assert!(Instant::now() < deadline, "child {pid} still alive");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn no_events_after_drop() {
    use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    let mut cmd = CommandBuilder::new("sh");
    // The subshell keeps printing (and ignores SIGHUP) for a while after the drop.
    cmd.args([
        "-c",
        "trap '' HUP; (while true; do echo hi; sleep 0.01; done) & wait",
    ]);
    let session = Session::spawn(
        cmd,
        80,
        24,
        |_| {},
        move |_| {
            c.fetch_add(1, SeqCst);
        },
    )
    .unwrap();
    let pid = session.process_id().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while count.load(SeqCst) == 0 {
        assert!(Instant::now() < deadline, "no output");
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(session);
    std::thread::sleep(Duration::from_millis(50)); // one in-flight event may land
    let settled = count.load(SeqCst);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(count.load(SeqCst), settled, "events after drop");
    let deadline = Instant::now() + Duration::from_secs(2);
    while alive(pid) {
        assert!(Instant::now() < deadline, "child {pid} still alive");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn shell_exit_ends_the_session_even_if_a_setsid_grandchild_holds_the_pty() {
    // Before M2 the reader waited for EOF, which never came while `sleep` kept the slave open.
    let t = Instant::now();
    let out = run("setsid -f sleep 5; echo started", None);
    assert!(out.contains("started"), "{out}");
    assert!(
        t.elapsed() < Duration::from_secs(2),
        "took {:?}",
        t.elapsed()
    );
}

#[test]
fn drop_releases_the_reader_even_if_a_setsid_grandchild_holds_the_pty() {
    let token = Arc::new(());
    let held = token.clone();
    let mut cmd = CommandBuilder::new("sh");
    cmd.args(["-c", "setsid -f sleep 5; sleep 30"]);
    let session = Session::spawn(
        cmd,
        80,
        24,
        |_| {},
        move |_| {
            let _keep = &held;
        },
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    drop(session);
    // The reader drops its event closure (and the `held` clone) when it returns.
    let deadline = Instant::now() + Duration::from_secs(2);
    while Arc::strong_count(&token) > 1 {
        assert!(Instant::now() < deadline, "reader thread still running");
        std::thread::sleep(Duration::from_millis(10));
    }
}
