//! The opt-in local-tmux profile (M10 Part 3, cmux `docs/local-tmux.md`): one user-owned tmux
//! server whose sessions outlive the app. Shared by the CLI and the app's save and restore.

use std::path::PathBuf;

use crate::paths::{self, Env};

/// The `Agent` kind the app saves an attached terminal under (M10 plan T3).
pub const KIND: &str = "local-tmux";
/// Set on the attaching client; only such a client is reattached on restore (lt).
pub const MARKER: &str = "CMUX_LOCAL_TMUX=1";
/// lt: "New sessions receive a bounded 10,000-line tmux history limit."
pub const HISTORY_LIMIT: &str = "10000";

/// `$XDG_STATE_HOME/cmux-linux/local-tmux/tmux.sock`.
pub fn socket_path(env: Env) -> Option<PathBuf> {
    Some(paths::state_dir(env)?.join("local-tmux/tmux.sock"))
}

/// `CMUX_LOCAL_TMUX_BIN`, else `tmux` from `PATH`.
pub fn bin(env: Env) -> String {
    env("CMUX_LOCAL_TMUX_BIN")
        .filter(|b| !b.trim().is_empty())
        .unwrap_or_else(|| "tmux".into())
}

/// lt: names match `[A-Za-z0-9_-]+`, at most 128 characters.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The line typed into a shell to attach `name` (sh, bash, zsh and fish 3+). Checks the session
/// first, so a dead server prints tmux's error and leaves the shell.
pub fn attach_line(bin: &str, socket: &str, name: &str) -> String {
    let (b, s) = (quote(bin), quote(socket));
    format!(
        "{b} -S {s} has-session -t ={name} && TMUX= {MARKER} {b} -S {s} attach-session -t ={name}"
    )
}

/// The session a tmux client process is attached to on `socket`: argv is `<tmux> ... -S
/// <socket> ... attach|attach-session|a ... -t [=]<name>`, and its environment carries
/// `MARKER`.
pub fn detect(argv: &[String], environ: &[String], socket: &str) -> Option<String> {
    if !environ.iter().any(|e| e == MARKER) {
        return None;
    }
    let mut it = argv.iter().skip(1);
    let (mut sock_ok, mut attach, mut target) = (false, false, None);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-S" => sock_ok = it.next().is_some_and(|s| s == socket),
            "-t" => target = it.next().cloned(),
            "attach" | "attach-session" | "a" => attach = true,
            _ => {}
        }
    }
    let name = target?.trim_start_matches('=').to_string();
    (sock_ok && attach && valid_name(&name)).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn paths_and_names() {
        let env = |k: &str| (k == "HOME").then(|| "/h".to_string());
        assert_eq!(
            socket_path(&env),
            Some(PathBuf::from(
                "/h/.local/state/cmux-linux/local-tmux/tmux.sock"
            ))
        );
        assert_eq!(bin(&env), "tmux");
        assert!(valid_name("work_1-a"));
        assert!(!valid_name("a.b") && !valid_name("") && !valid_name(&"x".repeat(129)));
    }

    #[test]
    fn attach_line_round_trips_through_detect() {
        let line = attach_line("tmux", "/s/tmux.sock", "work");
        assert_eq!(
            line,
            "'tmux' -S '/s/tmux.sock' has-session -t =work && TMUX= CMUX_LOCAL_TMUX=1 'tmux' -S '/s/tmux.sock' attach-session -t =work"
        );
        let argv = v(&[
            "tmux",
            "-S",
            "/s/tmux.sock",
            "attach-session",
            "-t",
            "=work",
        ]);
        let env = v(&["TMUX=", MARKER]);
        assert_eq!(detect(&argv, &env, "/s/tmux.sock").as_deref(), Some("work"));
        assert_eq!(detect(&argv, &v(&["TMUX="]), "/s/tmux.sock"), None);
        assert_eq!(detect(&argv, &env, "/other.sock"), None);
        let no_attach = v(&["tmux", "-S", "/s/tmux.sock", "new-session", "-t", "work"]);
        assert_eq!(detect(&no_attach, &env, "/s/tmux.sock"), None);
    }
}
