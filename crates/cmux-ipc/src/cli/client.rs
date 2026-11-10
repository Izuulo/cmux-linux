//! The CLI side of the socket: cmux's connect checks, `auth`, v1 lines and v2 calls (rc §1, §4).

use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use crate::wire::{self, RpcError};

const RETRY_FOR: Duration = Duration::from_millis(350);
const RETRY_EVERY: Duration = Duration::from_millis(25);

pub(crate) struct Client {
    reader: BufReader<UnixStream>,
}

impl Client {
    /// `timeout`: how long to wait for each reply.
    pub(crate) fn connect(path: &Path, timeout: Duration) -> Result<Client, String> {
        let p = path.display();
        // SAFETY: geteuid has no preconditions.
        let me = unsafe { libc::geteuid() };
        let not_found = || format!("Socket not found at {p}");
        let link = std::fs::symlink_metadata(path).map_err(|_| not_found())?;
        if link.file_type().is_symlink() && link.uid() != me {
            return Err(format!(
                "Socket link at {p} is not owned by the current user — refusing to connect"
            ));
        }
        let meta = std::fs::metadata(path).map_err(|_| not_found())?;
        if !meta.file_type().is_socket() {
            return Err(format!("Path exists at {p} but is not a Unix socket"));
        }
        if meta.uid() != me {
            return Err(format!(
                "Socket at {p} is not owned by the current user — refusing to connect"
            ));
        }
        let start = Instant::now();
        let stream = loop {
            match UnixStream::connect(path) {
                Ok(s) => break s,
                Err(e)
                    if matches!(
                        e.kind(),
                        ErrorKind::ConnectionRefused | ErrorKind::NotFound | ErrorKind::WouldBlock
                    ) && start.elapsed() < RETRY_FOR =>
                {
                    std::thread::sleep(RETRY_EVERY)
                }
                Err(e) => {
                    let errno = e.raw_os_error().unwrap_or(0);
                    let text = e.to_string();
                    let text = text.split(" (os error").next().unwrap_or(&text);
                    return Err(format!(
                        "Failed to connect to socket at {p} ({text}, errno {errno})"
                    ));
                }
            }
        };
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| e.to_string())?;
        Ok(Client {
            reader: BufReader::new(stream),
        })
    }

    /// Sends `auth <password>`. A server without the command (older cmux) is fine.
    pub(crate) fn auth(&mut self, password: &str) -> Result<(), String> {
        let reply = self.v1(&format!("auth {password}"))?;
        if reply.starts_with("ERROR") && !reply.contains("Unknown command 'auth'") {
            return Err(reply);
        }
        Ok(())
    }

    /// A v1 line and its one-line reply. An `ERROR:` reply is an error, shown verbatim.
    pub(crate) fn v1(&mut self, line: &str) -> Result<String, String> {
        self.send(line)?;
        let reply = self.read()?;
        if reply.starts_with("ERROR") {
            return Err(reply);
        }
        Ok(reply)
    }

    #[allow(dead_code)] // the v2 commands arrive in plan Task 5
    /// A v2 call. A server error comes back as `Ok(Err(..))`.
    pub(crate) fn call(
        &mut self,
        method: &str,
        params: &Map<String, Value>,
    ) -> Result<Result<Value, RpcError>, String> {
        let id = uuid::Uuid::new_v4().to_string().to_uppercase();
        self.send(&wire::request_line(&id, method, params))?;
        let reply = self.read()?;
        // A v1-style refusal (access denied, auth required) can answer a v2 line.
        if reply.starts_with("ERROR") {
            return Err(reply);
        }
        wire::parse_response(&reply, &id)
    }

    fn send(&mut self, line: &str) -> Result<(), String> {
        let stream = self.reader.get_mut();
        stream
            .write_all(format!("{line}\n").as_bytes())
            .map_err(|_| "Socket closed before reply".to_string())
    }

    fn read(&mut self) -> Result<String, String> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => Err("Socket closed before reply".into()),
            Ok(_) if !line.ends_with('\n') => Err("Socket closed before complete reply".into()),
            Ok(_) => Ok(line.trim_end_matches(['\n', '\r']).to_string()),
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                Err("Command timed out".into())
            }
            Err(_) => Err("Socket read error".into()),
        }
    }
}
