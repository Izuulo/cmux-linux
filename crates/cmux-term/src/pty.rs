//! Runs a command on a PTY and pumps bytes between it and a [`Term`].
//!
//! Threads: a reader feeds PTY output into the `Term` (under its mutex) and reports events;
//! a writer owns the PTY writer and drains one channel that carries both user input and the
//! terminal's own replies, so nothing ever writes from inside `Term::feed`.
//!
//! The reader `poll`s the master, a wake socket and (Linux) a pidfd of the shell, so it never
//! blocks forever: dropping the `Session` wakes it, and the shell exiting ends the session even
//! when a `setsid` grandchild still holds the PTY slave open (macOS needs no pidfd: it revokes
//! the slave when the shell exits, so the master sees EOF).

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

pub use portable_pty::CommandBuilder;
use portable_pty::{Child, MasterPty, PtySize, native_pty_system};

use crate::{Term, TermEvent};

#[derive(Debug)]
pub enum PtyEvent {
    /// A chunk of output was parsed; repaint.
    Output(Vec<TermEvent>),
    /// The child closed the PTY (EOF, or EIO on Linux once it exits).
    Exited,
}

pub struct Session {
    pub term: Arc<Mutex<Term>>,
    writer: mpsc::Sender<Vec<u8>>,
    master: Box<dyn MasterPty + Send>,
    child: ChildGuard,
    /// Set on drop so the reader stops reporting events.
    closed: Arc<AtomicBool>,
    /// Dropped with the session; the reader sees its peer hang up and returns.
    _wake: UnixStream,
}

/// Owns the child; dropping it tears the child down without blocking the caller.
struct ChildGuard(Option<Box<dyn Child + Send + Sync>>);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let Some(mut child) = self.0.take() else {
            return;
        };
        #[cfg(unix)]
        let pgid = child.process_id().and_then(|p| i32::try_from(p).ok());
        // Like a terminal closing: hang up the shell's whole process group (the shell is the
        // session leader, so pgid == pid). portable-pty's own `kill` would sleep ~200 ms
        // before escalating, which would stall the GUI thread.
        #[cfg(unix)]
        if let Some(pgid) = pgid {
            // SAFETY: plain signal syscall; a stale pgid at worst gives ESRCH.
            unsafe { libc::killpg(pgid, libc::SIGHUP) };
        }
        // Escalate and reap off-thread so a child that ignores SIGHUP can't block us.
        let _ = thread::Builder::new()
            .name("pty-reaper".into())
            .spawn(move || {
                let deadline = Instant::now() + Duration::from_millis(500);
                while Instant::now() < deadline {
                    if !matches!(child.try_wait(), Ok(None)) {
                        return; // exited and reaped (or unwaitable)
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                #[cfg(unix)]
                if let Some(pgid) = pgid {
                    // SAFETY: as above; the child is unreaped, so the pgid cannot have been reused.
                    unsafe { libc::killpg(pgid, libc::SIGKILL) };
                }
                #[cfg(not(unix))]
                let _ = child.kill();
                let _ = child.wait();
            });
    }
}

/// Names starting `CMUX_` (not `CMUXTERM_*`, the CLI tuning knobs).
fn inherited_cmux_keys(names: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    names
        .into_iter()
        .filter(|k| k.as_encoded_bytes().starts_with(b"CMUX_"))
        .collect()
}

impl Session {
    /// The user's login shell (`$SHELL`, else the passwd entry) in `$HOME`.
    pub fn shell_command() -> CommandBuilder {
        let mut cmd = CommandBuilder::new_default_prog();
        // shortcut: xterm-256color, not xterm-ghostty, since that terminfo may be missing; kitty
        // keyboard is negotiated by escape sequence either way. Ship terminfo in M6.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        // A parent cmux's hook shims and token must not leak; the caller re-adds ours.
        for k in inherited_cmux_keys(std::env::vars_os().map(|(k, _)| k)) {
            cmd.env_remove(k);
        }
        if let Some(home) = std::env::var_os("HOME") {
            cmd.cwd(home);
        }
        cmd
    }

    /// Spawns `cmd`. `init` runs on the new `Term` before any output is read (set default
    /// colours there so the first OSC 10/11 query already answers). `on_event` runs on the
    /// reader thread. Once the returned `Session` is dropped `on_event` is not called again,
    /// except for at most one event already in flight.
    pub fn spawn(
        cmd: CommandBuilder,
        cols: u16,
        rows: u16,
        init: impl FnOnce(&mut Term),
        mut on_event: impl FnMut(PtyEvent) + Send + 'static,
    ) -> io::Result<Session> {
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = native_pty_system()
            .openpty(size)
            .map_err(io::Error::other)?;
        // Declared first so any `?` below kills and reaps the child.
        let child = ChildGuard(Some(
            pair.slave.spawn_command(cmd).map_err(io::Error::other)?,
        ));
        // Our copy of the slave must close, or the reader never sees EOF when the child exits.
        drop(pair.slave);
        let master = pair.master;
        let mut reader = master.try_clone_reader().map_err(io::Error::other)?;
        let mut pty_writer = master.take_writer().map_err(io::Error::other)?;
        let raw = master
            .as_raw_fd()
            .ok_or_else(|| io::Error::other("PTY master has no fd"))?;
        // SAFETY: `raw` is the master's fd and `master` is alive for this borrow; the clone
        // is an independent fd the reader polls.
        let master_fd = unsafe { BorrowedFd::borrow_raw(raw) }.try_clone_to_owned()?;
        let exit_fd = child
            .0
            .as_ref()
            .and_then(|c| c.process_id())
            .and_then(pidfd);
        let (wake, wake_peer) = UnixStream::pair()?;

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let mut term = Term::new(cols, rows, tx.clone()).map_err(io::Error::other)?;
        init(&mut term);
        let term = Arc::new(Mutex::new(term));

        thread::Builder::new()
            .name("pty-writer".into())
            .spawn(move || {
                for bytes in rx {
                    if pty_writer
                        .write_all(&bytes)
                        .and_then(|()| pty_writer.flush())
                        .is_err()
                    {
                        break;
                    }
                }
            })?;

        let shared = term.clone();
        let closed = Arc::new(AtomicBool::new(false));
        let reader_closed = closed.clone();
        thread::Builder::new()
            .name("pty-reader".into())
            .spawn(move || {
                let mut buf = vec![0u8; 64 * 1024];
                // Set when the shell exits: drain until output stops or the deadline passes.
                let mut drain_until: Option<Instant> = None;
                loop {
                    let child_gone = drain_until.is_some();
                    if drain_until.is_some_and(|d| Instant::now() >= d) {
                        break; // a grandchild that keeps writing must not keep the pane open
                    }
                    let ready = match wait(&master_fd, &wake_peer, exit_fd.as_ref(), child_gone) {
                        Ok(ready) => ready,
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    };
                    if ready.wake {
                        return; // the Session was dropped
                    }
                    if ready.exited {
                        // Drain what is buffered, then end even if a grandchild keeps the slave.
                        drain_until.get_or_insert(Instant::now() + DRAIN_LIMIT);
                    }
                    if !ready.master {
                        if drain_until.is_some() {
                            break;
                        }
                        continue;
                    }
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let Some(events) = guarded(&shared, |t| t.feed(&buf[..n])) else {
                                eprintln!("cmux-linux: terminal parser panicked; closing it");
                                break;
                            };
                            if reader_closed.load(Ordering::Acquire) {
                                return;
                            }
                            on_event(PtyEvent::Output(events));
                        }
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                if !reader_closed.load(Ordering::Acquire) {
                    on_event(PtyEvent::Exited);
                }
            })?;

        Ok(Session {
            term,
            writer: tx,
            master,
            child,
            closed,
            _wake: wake,
        })
    }

    /// The child's pid, if the platform reports one.
    pub fn process_id(&self) -> Option<u32> {
        self.child.0.as_ref().and_then(|c| c.process_id())
    }

    /// True when the PTY's foreground process group is not the shell's own (a job is running).
    pub fn foreground_busy(&self) -> bool {
        match (self.master.process_group_leader(), self.process_id()) {
            (Some(fg), Some(shell)) => i64::from(fg) != i64::from(shell),
            _ => false,
        }
    }

    /// The PTY's foreground process group leader, when it is not the shell (a job runs).
    pub fn foreground_pid(&self) -> Option<u32> {
        let fg = u32::try_from(self.master.process_group_leader()?).ok()?;
        (Some(fg) != self.process_id()).then_some(fg)
    }

    /// The child's current working directory as the OS reports it (see [`process_cwd`]).
    pub fn cwd(&self) -> Option<String> {
        process_cwd(self.process_id()?)
    }

    /// Queues bytes for the child. Never blocks.
    pub fn write(&self, bytes: Vec<u8>) {
        let _ = self.writer.send(bytes);
    }

    /// Resizes the terminal, then tells the kernel (which sends SIGWINCH).
    pub fn resize(&self, cols: u16, rows: u16, cell_w_px: u32, cell_h_px: u32) -> io::Result<()> {
        self.term
            .lock()
            .unwrap()
            .resize(cols, rows, cell_w_px, cell_h_px)
            .map_err(io::Error::other)?;
        let px = |cells: u16, cell: u32| (u32::from(cells) * cell).min(u32::from(u16::MAX)) as u16;
        let size = PtySize {
            rows,
            cols,
            pixel_width: px(cols, cell_w_px),
            pixel_height: px(rows, cell_h_px),
        };
        self.master.resize(size).map_err(io::Error::other)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Release);
        // Fields drop after this: `_wake` closing wakes the reader, which returns and drops its
        // `Term` clone; `writer` closing then ends the writer thread, so every master fd closes
        // and the kernel hangs up the PTY. `child` SIGHUPs the process group and reaps it.
    }
}

/// Runs `f` on the locked value. A panic inside `f` is caught and reported as `None`, and the
/// poison is cleared so the GUI thread's later `lock().unwrap()` does not panic too.
fn guarded<T, R>(m: &Mutex<T>, f: impl FnOnce(&mut T) -> R) -> Option<R> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        f(&mut m.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
    }));
    m.clear_poison();
    result.ok()
}

/// After the shell exits: stop once the master is quiet this long (ms) ...
const DRAIN_IDLE_MS: libc::c_int = 30;
/// ... or this long after the exit, whichever comes first.
const DRAIN_LIMIT: Duration = Duration::from_millis(200);

struct Ready {
    master: bool,
    wake: bool,
    exited: bool,
}

/// Blocks until the master is readable (or hung up), the wake socket's peer is gone, or the
/// child exited. Once `child_gone`, only checks whether output is still buffered.
fn wait(
    master: &OwnedFd,
    wake: &UnixStream,
    exit: Option<&OwnedFd>,
    child_gone: bool,
) -> io::Result<Ready> {
    let fd = |fd: RawFd| libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // A negative fd is ignored by poll.
    let exit_fd = exit.filter(|_| !child_gone).map_or(-1, AsRawFd::as_raw_fd);
    let mut fds = [fd(master.as_raw_fd()), fd(wake.as_raw_fd()), fd(exit_fd)];
    // The shell's last bytes can reach the master a moment after its exit, so wait briefly.
    let timeout = if child_gone { DRAIN_IDLE_MS } else { -1 };
    // SAFETY: `fds` is a valid array of 3 pollfds for the duration of the call.
    if unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) } < 0 {
        return Err(io::Error::last_os_error());
    }
    let hit = |p: &libc::pollfd| {
        p.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0
    };
    Ok(Ready {
        master: hit(&fds[0]),
        wake: hit(&fds[1]),
        exited: hit(&fds[2]),
    })
}

/// A pidfd that becomes readable when `pid` exits (Linux only).
fn pidfd(pid: u32) -> Option<OwnedFd> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::FromRawFd;
        // SAFETY: plain syscall; on success the kernel returns a new fd that nothing else owns.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0) };
        // SAFETY: `fd` is that new, open fd.
        (fd >= 0).then(|| unsafe { OwnedFd::from_raw_fd(fd as RawFd) })
    }
    #[cfg(not(target_os = "linux"))]
    {
        // macOS revokes the slave when the session leader exits, so the master sees EOF even
        // while a job holds it.
        let _ = pid;
        None
    }
}

/// A process's working directory, read from `/proc/<pid>/cwd`.
#[cfg(target_os = "linux")]
pub fn process_cwd(pid: u32) -> Option<String> {
    let dir = std::fs::read_link(format!("/proc/{pid}/cwd")).ok()?;
    dir.into_os_string().into_string().ok()
}

/// A process's working directory via `proc_pidinfo`.
#[cfg(target_os = "macos")]
pub fn process_cwd(pid: u32) -> Option<String> {
    // SAFETY: zeroed proc_vnodepathinfo is valid; proc_pidinfo writes at most `size` bytes.
    let mut info: libc::proc_vnodepathinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_vnodepathinfo>() as libc::c_int;
    let n = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            (&mut info as *mut libc::proc_vnodepathinfo).cast(),
            size,
        )
    };
    if n != size {
        return None;
    }
    let raw = &info.pvi_cdir.vip_path;
    // SAFETY: c_char is u8-sized; libc declares vip_path as nested arrays, so use the byte size.
    let bytes = unsafe {
        std::slice::from_raw_parts(raw.as_ptr().cast::<u8>(), std::mem::size_of_val(raw))
    };
    let path = std::ffi::CStr::from_bytes_until_nul(bytes).ok()?;
    path.to_str()
        .ok()
        .filter(|p| !p.is_empty())
        .map(String::from)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn process_cwd(_pid: u32) -> Option<String> {
    None
}

/// A process's argv, executable path and name (`/proc/<pid>/cmdline`, `exe`, `comm`); M10
/// custom-agent detection.
#[cfg(target_os = "linux")]
pub fn process_argv(pid: u32) -> Option<(Vec<String>, Option<String>, Option<String>)> {
    let raw = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let argv: Vec<String> = raw
        .split(|b| *b == 0)
        .filter(|a| !a.is_empty())
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect();
    let exe = std::fs::read_link(format!("/proc/{pid}/exe"))
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok());
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|c| c.trim_end().to_string());
    (!argv.is_empty()).then_some((argv, exe, comm))
}

// shortcut: no custom-agent detection off Linux; macOS needs `KERN_PROCARGS2`.
#[cfg(not(target_os = "linux"))]
pub fn process_argv(_pid: u32) -> Option<(Vec<String>, Option<String>, Option<String>)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherited_cmux_keys_matches_cmux_prefix_only() {
        let names = [
            "PATH",
            "CMUX_SURFACE_ID",
            "CMUX_CLAUDE_HOOK_SHIM",
            "CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC",
        ];
        let got = inherited_cmux_keys(names.map(OsString::from));
        assert_eq!(
            got,
            ["CMUX_SURFACE_ID", "CMUX_CLAUDE_HOOK_SHIM"].map(OsString::from)
        );
    }

    #[test]
    fn a_panic_under_the_lock_is_caught_and_unpoisoned() {
        let m = Mutex::new(1);
        assert_eq!(guarded(&m, |v| *v + 1), Some(2));
        let r: Option<()> = guarded(&m, |_| panic!("boom"));
        assert_eq!(r, None);
        assert!(!m.is_poisoned());
        assert_eq!(*m.lock().unwrap(), 1);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn process_cwd_reads_proc() {
        let here = std::env::current_dir().unwrap();
        assert_eq!(process_cwd(std::process::id()).as_deref(), here.to_str());
        assert_eq!(process_cwd(u32::MAX), None);
    }
}
