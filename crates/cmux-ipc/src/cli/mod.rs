//! The `cmux` command-line client (rc §4, §7-9). A library function so tests run it in-process.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Map, Value};

use crate::json::IdFormat;
use crate::paths;
use crate::wire::RpcError;

mod agent;
mod args;
mod automation;
mod client;
mod commands;
mod groups;
mod help;
mod hooks;
mod notify;

use args::Global;
use client::Client;

/// What the CLI sees of its process: the environment and the working directory.
pub struct Ctx {
    pub env: HashMap<String, String>,
    pub cwd: PathBuf,
    /// Standard input for commands that read it; `None` reads the process's own.
    pub stdin: Option<String>,
    /// This executable's path.
    pub exe: Option<PathBuf>,
}

impl Ctx {
    /// The process's own environment and working directory.
    pub fn from_process() -> Ctx {
        Ctx {
            env: std::env::vars().collect(),
            cwd: std::env::current_dir().unwrap_or_default(),
            stdin: None,
            exe: std::env::current_exe().ok(),
        }
    }
}

pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

/// An error printed as `Error: <msg>` with exit `code`.
#[derive(Debug)]
pub(crate) struct CliError {
    pub msg: String,
    pub code: i32,
}

pub(crate) fn fail<T>(msg: impl Into<String>) -> Result<T, CliError> {
    Err(CliError {
        msg: msg.into(),
        code: 1,
    })
}

impl From<String> for CliError {
    fn from(msg: String) -> Self {
        CliError { msg, code: 1 }
    }
}

/// Runs one invocation; `args` excludes the program name.
pub fn run(args: &[String], ctx: &Ctx) -> Outcome {
    match run_inner(args, ctx) {
        Ok(stdout) => Outcome {
            stdout,
            stderr: String::new(),
            code: 0,
        },
        Err(e) => Outcome {
            stdout: String::new(),
            stderr: format!("Error: {}\n", e.msg),
            code: e.code,
        },
    }
}

fn run_inner(args: &[String], ctx: &Ctx) -> Result<String, CliError> {
    let parsed = args::parse(args)?;
    let (global, name, rest, wants_help) = match parsed {
        args::Parsed::Help => return Ok(help::top_level()),
        args::Parsed::Version => return Ok(version()),
        args::Parsed::Command {
            global,
            name,
            args,
            wants_help,
        } => (global, name, args, wants_help),
    };
    match name.as_str() {
        "version" => return Ok(version()),
        "help" => return Ok(help::top_level()),
        _ => {}
    }
    let Some(_) = help::usage_line(&name) else {
        return Err(unknown_command(&name));
    };
    if wants_help {
        return Ok(help::command(&name).unwrap_or_else(help::top_level));
    }
    let mut cmd = Cmd {
        ctx,
        g: global,
        args: rest,
        client: None,
    };
    match name.as_str() {
        "ping" => ping(&mut cmd),
        "list-workspaces" => commands::list_workspaces(&mut cmd),
        "current-workspace" => commands::current_workspace(&mut cmd),
        "list-panes" => commands::list_panes(&mut cmd),
        "list-pane-surfaces" => commands::list_pane_surfaces(&mut cmd),
        "select-workspace" => commands::select_workspace(&mut cmd),
        "focus-pane" => commands::focus_pane(&mut cmd),
        "focus-panel" => commands::focus_panel(&mut cmd),
        "new-workspace" => commands::new_workspace(&mut cmd),
        "new-split" => commands::new_split(&mut cmd),
        "send" => commands::send(&mut cmd),
        "send-key" => commands::send_key(&mut cmd),
        "notify" => notify::notify(&mut cmd),
        "dismiss-notification" => notify::dismiss(&mut cmd),
        "hooks" => hooks::hooks(&mut cmd),
        "workspace-group" => groups::workspace_group(&mut cmd),
        "agent" => agent::agent(&mut cmd),
        "automation" => automation::automation(&mut cmd),
        other => fail(format!("{other} is not available yet")),
    }
}

fn version() -> String {
    format!("cmux {}\n", env!("CARGO_PKG_VERSION"))
}

fn unknown_command(name: &str) -> CliError {
    let suggestion = help::GROUPS
        .iter()
        .flat_map(|(_, cmds)| cmds.iter())
        .map(|c| (distance(name, c), *c))
        .filter(|(d, _)| *d <= 2)
        .min()
        .map(|(_, c)| format!("Did you mean '{c}'? "))
        .unwrap_or_default();
    CliError {
        msg: format!(
            "Unknown command '{name}'. {suggestion}Run 'cmux --help' for the full command list."
        ),
        code: 2,
    }
}

/// Levenshtein distance.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// One command's invocation: options, arguments and a lazily connected client.
pub(crate) struct Cmd<'a> {
    pub ctx: &'a Ctx,
    pub g: Global,
    pub args: Vec<String>,
    client: Option<Client>,
}

impl Cmd<'_> {
    /// A non-blank environment variable, trimmed.
    pub(crate) fn env(&self, key: &str) -> Option<&str> {
        self.ctx
            .env
            .get(key)
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
    }

    pub(crate) fn id_format(&self) -> IdFormat {
        self.g.id_format.unwrap_or(IdFormat::Refs)
    }

    pub(crate) fn client(&mut self) -> Result<&mut Client, CliError> {
        if self.client.is_none() {
            let path = self.socket_path()?;
            let timeout = self
                .env("CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC")
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|s| *s > 0.0)
                .map(Duration::from_secs_f64)
                .unwrap_or(Duration::from_secs(15));
            let mut client = Client::connect(&path, timeout)?;
            if let Some(password) = self.password() {
                client.auth(&password)?;
            }
            self.client = Some(client);
        }
        Ok(self.client.as_mut().expect("just connected"))
    }

    /// A v2 call; a server error becomes `<code>: <message>`.
    pub(crate) fn call(
        &mut self,
        method: &str,
        params: Map<String, Value>,
    ) -> Result<Value, CliError> {
        match self.client()?.call(method, &params)? {
            Ok(v) => Ok(v),
            Err(RpcError { code, message, .. }) => fail(format!("{code}: {message}")),
        }
    }

    fn socket_path(&self) -> Result<PathBuf, CliError> {
        if let Some(s) = &self.g.socket {
            return Ok(PathBuf::from(s));
        }
        let (a, b) = (self.env("CMUX_SOCKET_PATH"), self.env("CMUX_SOCKET"));
        if let (Some(a), Some(b)) = (a, b)
            && a != b
        {
            return fail(
                "Refusing to choose socket: CMUX_SOCKET_PATH and CMUX_SOCKET differ. Use CMUX_SOCKET_PATH or unset CMUX_SOCKET.",
            );
        }
        if let Some(p) = a.or(b) {
            return Ok(PathBuf::from(p));
        }
        let env = |k: &str| self.ctx.env.get(k).cloned();
        match paths::default_socket(&env) {
            Some(p) => Ok(p),
            None => fail("No socket path: set CMUX_SOCKET_PATH or pass --socket"),
        }
    }

    /// `--password`, then `CMUX_SOCKET_PASSWORD`, then the password file (plan D3).
    fn password(&self) -> Option<String> {
        if let Some(p) = &self.g.password {
            return Some(p.clone());
        }
        if let Some(p) = self.env("CMUX_SOCKET_PASSWORD") {
            return Some(p.to_string());
        }
        let env = |k: &str| self.ctx.env.get(k).cloned();
        let file = std::fs::read_to_string(paths::password_file(&env)?).ok()?;
        file.lines()
            .next()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
    }
}

fn ping(cmd: &mut Cmd) -> Result<String, CliError> {
    let reply = cmd.client()?.v1("ping")?;
    Ok(format!("{reply}\n"))
}
