//! `cmux local-tmux start|list|attach|close` (M10 plan T1, T2; cmux `docs/local-tmux.md`).
//! The tmux server owns the processes; attaching types the attach line into the caller's
//! terminal, so the app needs nothing of its own for it.

use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value, json};

use super::{CliError, Cmd, fail};
use crate::local_tmux::{self, HISTORY_LIMIT};

const USAGE: &str =
    "Usage: cmux local-tmux start <name> [--cwd <dir>] [--command <cmd>] [--detached]
       cmux local-tmux list
       cmux local-tmux attach <name>
       cmux local-tmux close <name>";

struct Tmux {
    bin: String,
    socket: PathBuf,
}

impl Tmux {
    fn new(cmd: &Cmd) -> Result<Tmux, CliError> {
        let env = |k: &str| cmd.ctx.env.get(k).cloned();
        let Some(socket) = local_tmux::socket_path(&env) else {
            return fail("local-tmux: no state directory (set HOME or XDG_STATE_HOME)");
        };
        Ok(Tmux {
            bin: local_tmux::bin(&env),
            socket,
        })
    }

    /// The socket's directory, private to this user (lt: mode 0700).
    fn make_dir(&self) -> Result<(), CliError> {
        let dir = self.socket.parent().expect("socket has a parent");
        let made = std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .and_then(|()| std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)));
        made.or_else(|e| fail(format!("local-tmux: {}: {e}", dir.display())))
    }

    /// Runs tmux on our socket: `Ok(stdout)`, or `Err(stderr)` when it exits non-zero.
    fn run(&self, args: &[&str]) -> Result<Result<String, String>, CliError> {
        let out = Command::new(&self.bin)
            .arg("-S")
            .arg(&self.socket)
            .args(args)
            .output();
        let out = match out {
            Ok(o) => o,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return fail(format!(
                    "local-tmux: `{}` not found; install tmux or set CMUX_LOCAL_TMUX_BIN",
                    self.bin
                ));
            }
            Err(e) => return fail(format!("local-tmux: {}: {e}", self.bin)),
        };
        let text = |b: &[u8]| String::from_utf8_lossy(b).trim_end().to_string();
        Ok(if out.status.success() {
            Ok(text(&out.stdout))
        } else {
            Err(text(&out.stderr))
        })
    }

    fn has(&self, name: &str) -> Result<bool, CliError> {
        Ok(self
            .run(&["has-session", "-t", &format!("={name}")])?
            .is_ok())
    }

    fn attach_line(&self, name: &str) -> String {
        local_tmux::attach_line(&self.bin, &self.socket.to_string_lossy(), name)
    }
}

fn name_arg(name: Option<&String>) -> Result<String, CliError> {
    match name {
        Some(n) if local_tmux::valid_name(n) => Ok(n.clone()),
        Some(n) => fail(format!(
            "local-tmux: invalid name `{n}` (letters, digits, _ and -, at most 128)"
        )),
        None => fail(USAGE),
    }
}

/// Types the attach line into the caller's terminal.
fn attach(cmd: &mut Cmd, tmux: &Tmux, name: &str) -> Result<String, CliError> {
    let Some(surface) = cmd.env("CMUX_SURFACE_ID").map(str::to_string) else {
        return fail(format!(
            "local-tmux: not in a cmux terminal; attach from one, or run: {}",
            tmux.attach_line(name)
        ));
    };
    let mut p = Map::new();
    p.insert("surface_id".into(), surface.into());
    p.insert(
        "text".into(),
        format!("{}\n", tmux.attach_line(name)).into(),
    );
    cmd.call("surface.send_text", p)?;
    Ok(format!("OK {name}\n"))
}

pub(crate) fn local_tmux(cmd: &mut Cmd) -> Result<String, CliError> {
    let args = cmd.args.clone();
    let tmux = Tmux::new(cmd)?;
    match args.first().map(String::as_str) {
        Some("start") => {
            let name = name_arg(args.get(1))?;
            let (mut cwd, mut command, mut detached) = (None, None, false);
            let mut it = args.iter().skip(2);
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--cwd" | "--command" => {
                        let Some(v) = it.next() else {
                            return fail(format!("local-tmux start: {a} requires a value"));
                        };
                        if a == "--cwd" {
                            cwd = Some(v.clone());
                        } else {
                            command = Some(v.clone());
                        }
                    }
                    "--detached" => detached = true,
                    _ => return fail(format!("local-tmux start: unknown option {a}")),
                }
            }
            if let Some(d) = &cwd
                && !Path::new(d).is_dir()
            {
                return fail(format!("local-tmux start: no such directory: {d}"));
            }
            tmux.make_dir()?;
            if tmux.has(&name)? {
                return fail(format!("local-tmux: session `{name}` already exists"));
            }
            let mut tail = vec!["new-session", "-d", "-s", &name];
            if let Some(d) = &cwd {
                tail.extend(["-c", d]);
            }
            if let Some(c) = &command {
                tail.push(c);
            }
            // One request: the limit is set before the session's first window exists.
            let mut all = vec!["start-server", ";", "set-option", "-g", "history-limit"];
            all.extend([HISTORY_LIMIT, ";"]);
            all.extend(tail);
            if let Err(e) = tmux.run(&all)? {
                return fail(format!("local-tmux start: {e}"));
            }
            if detached {
                return Ok(format!("OK {name}\n"));
            }
            attach(cmd, &tmux, &name)
        }
        Some("attach") => {
            let name = name_arg(args.get(1))?;
            if !tmux.has(&name)? {
                return fail(format!("local-tmux: no session `{name}`"));
            }
            attach(cmd, &tmux, &name)
        }
        Some("close") => {
            let name = name_arg(args.get(1))?;
            match tmux.run(&["kill-session", "-t", &format!("={name}")])? {
                Ok(_) => Ok(format!("OK {name}\n")),
                Err(_) => fail(format!("local-tmux: no session `{name}`")),
            }
        }
        Some("list") if args.len() == 1 => {
            let fmt = "#{session_name}\t#{session_attached}\t#{session_path}";
            // No server yet means no sessions.
            let text = tmux.run(&["list-sessions", "-F", fmt])?.unwrap_or_default();
            let rows: Vec<Value> = text
                .lines()
                .filter_map(|l| {
                    let mut f = l.splitn(3, '\t');
                    let (n, a, p) = (f.next()?, f.next()?, f.next().unwrap_or(""));
                    Some(json!({"name": n, "attached": a.parse::<u64>().unwrap_or(0), "path": p}))
                })
                .collect();
            if cmd.g.json {
                let v = json!({"socket_path": tmux.socket, "sessions": rows});
                return Ok(cmd.json(v, &[]));
            }
            if rows.is_empty() {
                return Ok("No sessions\n".into());
            }
            Ok(rows
                .iter()
                .map(|r| {
                    format!(
                        "{}  attached={}  {}\n",
                        r["name"].as_str().unwrap_or(""),
                        r["attached"],
                        r["path"].as_str().unwrap_or("")
                    )
                })
                .collect())
        }
        _ => fail(USAGE),
    }
}
