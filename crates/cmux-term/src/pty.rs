//! Runs a command on a PTY and pumps bytes between it and a [`Term`].
//!
//! Threads: a reader feeds PTY output into the `Term` (under its mutex) and reports events;
//! a writer owns the PTY writer and drains one channel that carries both user input and the
//! terminal's own replies, so nothing ever writes from inside `Term::feed`.

use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

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
    child: Box<dyn Child + Send + Sync>,
}

impl Session {
    /// The user's login shell (`$SHELL`, else the passwd entry) in `$HOME`.
    pub fn shell_command() -> CommandBuilder {
        let mut cmd = CommandBuilder::new_default_prog();
        // shortcut: xterm-256color, not xterm-ghostty, since that terminfo may be missing; kitty
        // keyboard is negotiated by escape sequence either way. Ship terminfo in M6.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Some(home) = std::env::var_os("HOME") {
            cmd.cwd(home);
        }
        cmd
    }

    /// Spawns `cmd`. `init` runs on the new `Term` before any output is read (set default
    /// colours there so the first OSC 10/11 query already answers). `on_event` runs on the
    /// reader thread.
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
        let child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;
        // Our copy of the slave must close, or the reader never sees EOF when the child exits.
        drop(pair.slave);
        let master = pair.master;
        let mut reader = master.try_clone_reader().map_err(io::Error::other)?;
        let mut pty_writer = master.take_writer().map_err(io::Error::other)?;

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
        thread::Builder::new()
            .name("pty-reader".into())
            .spawn(move || {
                let mut buf = vec![0u8; 64 * 1024];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let events = shared.lock().unwrap().feed(&buf[..n]);
                            on_event(PtyEvent::Output(events));
                        }
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
                on_event(PtyEvent::Exited);
            })?;

        Ok(Session {
            term,
            writer: tx,
            master,
            child,
        })
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
        let _ = self.child.kill();
        // Reap it so no zombie outlives the session; SIGKILL makes this return promptly.
        let _ = self.child.wait();
    }
}
