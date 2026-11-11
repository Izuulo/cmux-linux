//! The control socket server (rc §1-3, plan D1-D3): one thread per connection; requests the
//! server can't answer itself go to the app over a channel and wait up to a deadline.

use std::fs::Permissions;
use std::io::{self, BufRead, BufReader, ErrorKind, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, SyncSender, sync_channel};
use std::time::Duration;

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use serde_json::{Map, Value, json};

use crate::wire::{self, Line, Reply, RpcError};

/// How long a request waits for the app (cmux's main-actor hop deadline).
pub const DEADLINE: Duration = Duration::from_secs(10);
/// Slack on top of `feed.push`'s own wait; the app answers `timed_out` at the wait itself.
pub const FEED_GRACE: Duration = Duration::from_secs(5);
const MAX_LINE: u64 = 4 << 20;
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const DENIED: &str = "ERROR: Access denied - only processes started inside cmux can connect";

#[derive(Debug, Clone, PartialEq)]
pub enum Access {
    /// Same user, and the peer process descends from `root` (the app).
    CmuxOnly {
        root: u32,
    },
    /// Same user, then `auth <password>`. `None`: password mode with no password configured.
    Password(Option<String>),
    AllowAll,
}

impl Access {
    /// `CMUX_SOCKET_MODE` (rc §3); `None` = off, no server.
    pub fn from_env(mode: Option<&str>, password_file: Option<&Path>) -> Option<Access> {
        let mode = mode.map(|m| m.trim().to_ascii_lowercase().replace(['-', '_'], ""));
        match mode.as_deref() {
            Some("off") => None,
            Some("password") => Some(Access::Password(
                password_file
                    .and_then(|p| std::fs::read_to_string(p).ok())
                    .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
                    .filter(|s| !s.is_empty()),
            )),
            Some("allowall" | "openaccess" | "fullopenaccess" | "full") => Some(Access::AllowAll),
            // shortcut: unknown modes, cmux's `automation` among them, get the default; add
            // automation (same user, no ancestry) if someone asks for it.
            _ => Some(Access::CmuxOnly {
                root: std::process::id(),
            }),
        }
    }
}

/// A request for the app. Dropping it unanswered fails the client with `internal_error`.
pub struct Request {
    pub method: String,
    pub params: Map<String, Value>,
    reply: SyncSender<Reply>,
}

impl Request {
    pub fn respond(self, reply: Reply) {
        let _ = self.reply.send(reply);
    }
}

pub struct Server {
    path: PathBuf,
    ino: u64,
    stop: Arc<AtomicBool>,
}

impl Server {
    /// Binds `path` (replacing a dead socket, never a live one: `AddrInUse`; never a non-socket
    /// file: `AlreadyExists`) and serves it until dropped.
    pub fn start(
        path: &Path,
        access: Access,
        deadline: Duration,
    ) -> io::Result<(Server, UnboundedReceiver<Request>)> {
        if let Some(dir) = path
            .parent()
            .filter(|d| !d.as_os_str().is_empty() && !d.exists())
        {
            std::fs::create_dir_all(dir)?;
            std::fs::set_permissions(dir, Permissions::from_mode(0o700))?;
        }
        clear_stale(path)?;
        let listener = UnixListener::bind(path)?;
        let mode = if access == Access::AllowAll {
            0o666
        } else {
            0o600
        };
        std::fs::set_permissions(path, Permissions::from_mode(mode))?;
        let ino = std::fs::metadata(path)?.ino();
        let (tx, rx) = unbounded();
        let stop = Arc::new(AtomicBool::new(false));
        let access = Arc::new(access);
        let stopped = stop.clone();
        std::thread::Builder::new()
            .name("cmux-socket".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stopped.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    let (access, tx) = (access.clone(), tx.clone());
                    // shortcut: one thread per connection, unbounded; cmux caps its pool and
                    // answers `overloaded`. Add a cap if many clients ever show up.
                    let _ = std::thread::Builder::new()
                        .name("cmux-socket-conn".into())
                        .spawn(move || serve(stream, &access, &tx, deadline));
                }
            })?;
        Ok((
            Server {
                path: path.to_path_buf(),
                ino,
                stop,
            },
            rx,
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wakes the accept loop so it sees `stop`.
        let _ = UnixStream::connect(&self.path);
        if std::fs::metadata(&self.path).is_ok_and(|m| m.ino() == self.ino) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn clear_stale(path: &Path) -> io::Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if !meta.file_type().is_socket() {
        return Err(io::Error::new(
            ErrorKind::AlreadyExists,
            format!("{} exists and is not a socket", path.display()),
        ));
    }
    // SAFETY: geteuid has no preconditions.
    if meta.uid() != unsafe { libc::geteuid() } {
        return Err(io::Error::new(
            ErrorKind::PermissionDenied,
            format!("{} belongs to another user", path.display()),
        ));
    }
    match UnixStream::connect(path) {
        Err(e) if matches!(e.kind(), ErrorKind::ConnectionRefused | ErrorKind::NotFound) => {
            std::fs::remove_file(path)
        }
        _ => Err(io::Error::new(
            ErrorKind::AddrInUse,
            format!("a server is already listening on {}", path.display()),
        )),
    }
}

fn serve(stream: UnixStream, access: &Access, tx: &UnboundedSender<Request>, deadline: Duration) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    if let Err(msg) = admit(&stream, access) {
        let _ = writeln!(writer, "{msg}");
        return;
    }
    let mut authed = !matches!(access, Access::Password(_));
    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    loop {
        line.clear();
        match (&mut reader)
            .take(MAX_LINE + 1)
            .read_until(b'\n', &mut line)
        {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if line.last() != Some(&b'\n') && line.len() as u64 > MAX_LINE {
            return;
        }
        let reply = handle(&line, access, &mut authed, tx, deadline);
        if writeln!(writer, "{reply}").is_err() {
            return;
        }
    }
}

fn admit(stream: &UnixStream, access: &Access) -> Result<(), &'static str> {
    // SAFETY: geteuid has no preconditions.
    let me = unsafe { libc::geteuid() };
    match access {
        Access::AllowAll => Ok(()),
        Access::Password(_) => match peer::cred(stream) {
            Some((_, uid)) if uid == me => Ok(()),
            _ => Err(DENIED),
        },
        Access::CmuxOnly { root } => {
            let Some((pid, uid)) = peer::cred(stream) else {
                return Err("ERROR: Unable to verify client process");
            };
            if uid == me && is_descendant(pid, *root, peer::parent_of) {
                Ok(())
            } else {
                Err(DENIED)
            }
        }
    }
}

fn handle(
    line: &[u8],
    access: &Access,
    authed: &mut bool,
    tx: &UnboundedSender<Request>,
    deadline: Duration,
) -> String {
    match wire::parse_line(line) {
        Line::Bad(response) => response,
        Line::V1 { cmd, args } => match cmd.as_str() {
            "" => "ERROR: Empty command".into(),
            "auth" => v1_auth(access, &args, authed).into(),
            _ if !*authed => "ERROR: Authentication required — send auth <password> first".into(),
            "ping" => "PONG".into(),
            _ => format!("ERROR: Unknown command '{cmd}'. Use 'help' for available commands."),
        },
        Line::V2 { id, method, params } => {
            let reply = match method.as_str() {
                "auth.login" => v2_auth(access, &params, authed),
                _ if !*authed => Err(RpcError::new(
                    "auth_required",
                    "Authentication required. Send auth <password> first.",
                )),
                "system.ping" => Ok(json!({ "pong": true })),
                "feed.push" => match crate::feed::wait_of(&params) {
                    Ok(wait) => ask_app(tx, method, params, wait + FEED_GRACE),
                    Err(e) => Err(e),
                },
                _ => ask_app(tx, method, params, deadline),
            };
            wire::response(&id, &reply)
        }
    }
}

const NO_PASSWORD: &str = "Password mode is enabled but no socket password is configured.";

fn v1_auth(access: &Access, given: &str, authed: &mut bool) -> &'static str {
    let Access::Password(password) = access else {
        return "OK: Authentication not required";
    };
    let Some(password) = password else {
        return "ERROR: Password mode is enabled but no socket password is configured.";
    };
    if given.is_empty() {
        "ERROR: Missing password. Usage: auth <password>"
    } else if same(given, password) {
        *authed = true;
        "OK: Authenticated"
    } else {
        "ERROR: Invalid password"
    }
}

fn v2_auth(access: &Access, params: &Map<String, Value>, authed: &mut bool) -> Reply {
    let ok = Ok(json!({ "authenticated": true }));
    let Access::Password(password) = access else {
        return ok;
    };
    let Some(given) = params.get("password").and_then(Value::as_str) else {
        return Err(RpcError::new(
            "invalid_params",
            "auth.login requires params.password",
        ));
    };
    let Some(password) = password else {
        return Err(RpcError::new("auth_unconfigured", NO_PASSWORD));
    };
    if same(given, password) {
        *authed = true;
        ok
    } else {
        Err(RpcError::new("auth_failed", "Invalid password"))
    }
}

/// Constant-time for equal lengths.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0, |acc, (x, y)| acc | (x ^ y))
            == 0
}

fn ask_app(
    tx: &UnboundedSender<Request>,
    method: String,
    params: Map<String, Value>,
    deadline: Duration,
) -> Reply {
    let dead = || RpcError::new("internal_error", "The app did not answer");
    let (reply, answer) = sync_channel(1);
    if tx
        .unbounded_send(Request {
            method,
            params,
            reply,
        })
        .is_err()
    {
        return Err(dead());
    }
    match answer.recv_timeout(deadline) {
        Ok(reply) => reply,
        Err(RecvTimeoutError::Timeout) => {
            Err(
                RpcError::new("timeout", "Timed out waiting for the app").with_data(json!({
                    "retryable": true,
                    "deadline_ms": deadline.as_millis() as u64,
                    "stage": "main_actor",
                })),
            )
        }
        Err(RecvTimeoutError::Disconnected) => Err(dead()),
    }
}

/// Whether `pid` is `root` or below it, walking at most 128 parents (rc §3).
pub fn is_descendant(pid: u32, root: u32, parent_of: impl Fn(u32) -> Option<u32>) -> bool {
    let mut cur = pid;
    for _ in 0..=128 {
        if cur == root {
            return true;
        }
        match parent_of(cur) {
            Some(p) if p != cur && p != 0 => cur = p,
            _ => return false,
        }
    }
    false
}

/// The peer's pid and uid, and a process's parent.
mod peer {
    use std::os::unix::io::AsRawFd;
    use std::os::unix::net::UnixStream;

    #[cfg(target_os = "linux")]
    pub fn cred(s: &UnixStream) -> Option<(u32, u32)> {
        // SAFETY: zeroed ucred is valid; getsockopt writes at most `len` bytes into it.
        let mut c: libc::ucred = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let r = unsafe {
            libc::getsockopt(
                s.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut c as *mut libc::ucred).cast(),
                &mut len,
            )
        };
        (r == 0 && c.pid > 0).then_some((c.pid as u32, c.uid))
    }

    #[cfg(target_os = "linux")]
    pub fn parent_of(pid: u32) -> Option<u32> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // `comm` may hold spaces and `)`: the fields resume after the last `)`; state, then ppid.
        stat.rsplit_once(')')?
            .1
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    }

    #[cfg(target_os = "macos")]
    pub fn cred(s: &UnixStream) -> Option<(u32, u32)> {
        let fd = s.as_raw_fd();
        let mut pid: libc::pid_t = 0;
        let mut len = std::mem::size_of::<libc::pid_t>() as libc::socklen_t;
        // SAFETY: both calls write into the locals they are given, within the stated sizes.
        let r = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_LOCAL,
                libc::LOCAL_PEERPID,
                (&mut pid as *mut libc::pid_t).cast(),
                &mut len,
            )
        };
        let (mut uid, mut gid) = (0, 0);
        let r2 = unsafe { libc::getpeereid(fd, &mut uid, &mut gid) };
        (r == 0 && r2 == 0 && pid > 0).then_some((pid as u32, uid))
    }

    #[cfg(target_os = "macos")]
    pub fn parent_of(pid: u32) -> Option<u32> {
        // SAFETY: zeroed proc_bsdinfo is valid; proc_pidinfo writes at most `size` bytes.
        let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
        let n = unsafe {
            libc::proc_pidinfo(
                pid as libc::c_int,
                libc::PROC_PIDTBSDINFO,
                0,
                (&mut info as *mut libc::proc_bsdinfo).cast(),
                size,
            )
        };
        (n == size).then_some(info.pbi_ppid)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub fn cred(_: &UnixStream) -> Option<(u32, u32)> {
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub fn parent_of(_: u32) -> Option<u32> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn descendant_walk() {
        let parents: HashMap<u32, u32> = [(50, 40), (40, 30), (30, 1), (7, 8), (8, 7)].into();
        let parent_of = |p: u32| parents.get(&p).copied();
        assert!(is_descendant(50, 30, parent_of));
        assert!(is_descendant(50, 50, parent_of));
        assert!(!is_descendant(50, 99, parent_of));
        assert!(!is_descendant(7, 99, parent_of), "a loop terminates");
        // 200 levels deep: past the 128-level cap.
        let deep = |p: u32| (p < 1000).then_some(p + 1);
        assert!(is_descendant(900, 1000, deep));
        assert!(!is_descendant(700, 1000, deep));
    }

    #[test]
    fn access_modes() {
        let dir = std::env::temp_dir().join(format!("cmux-ipc-access-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("pw");
        std::fs::write(&file, "  secret \nignored\n").unwrap();
        let me = std::process::id();
        let cases = [
            (None, Some(Access::CmuxOnly { root: me })),
            (Some("cmuxOnly"), Some(Access::CmuxOnly { root: me })),
            (Some("cmux-only"), Some(Access::CmuxOnly { root: me })),
            (Some("cmux_only"), Some(Access::CmuxOnly { root: me })),
            (Some("allowAll"), Some(Access::AllowAll)),
            (Some("openaccess"), Some(Access::AllowAll)),
            (Some("fullopenaccess"), Some(Access::AllowAll)),
            (Some("full"), Some(Access::AllowAll)),
            (Some("off"), None),
            (
                Some("password"),
                Some(Access::Password(Some("secret".into()))),
            ),
            (Some("bogus"), Some(Access::CmuxOnly { root: me })),
        ];
        for (mode, want) in cases {
            assert_eq!(Access::from_env(mode, Some(&file)), want, "{mode:?}");
        }
        assert_eq!(
            Access::from_env(Some("password"), Some(&dir.join("missing"))),
            Some(Access::Password(None))
        );
        std::fs::write(&file, "\n").unwrap();
        assert_eq!(
            Access::from_env(Some("password"), Some(&file)),
            Some(Access::Password(None))
        );
    }
}
