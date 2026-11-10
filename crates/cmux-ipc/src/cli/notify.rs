//! `notify` and `dismiss-notification` (rc §8).

use serde_json::{Map, Value};

use super::commands::{is_uuid, parse_bool};
use super::{CliError, Cmd, fail};
use crate::handles::Kind;
use crate::json::IdFormat;

const VALUES: &[&str] = &[
    "--title",
    "--subtitle",
    "--body",
    "--desktop",
    "--workspace",
    "--surface",
    "--window",
];

#[derive(Default)]
struct Args {
    title: Option<String>,
    subtitle: Option<String>,
    body: Option<String>,
    desktop: Option<String>,
    workspace: Option<String>,
    surface: Option<String>,
    window: Option<String>,
    reply: bool,
    clear: bool,
}

/// cmux's notify parsing: a value flag needs a value that isn't itself a `--` flag
/// (`--title=--body` is fine); anything else is `notify: unexpected arguments: <it>`.
fn parse(args: &[String]) -> Result<Args, CliError> {
    let unexpected = |tok: &str| fail(format!("notify: unexpected arguments: {tok}"));
    let mut a = Args::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        match flag {
            "--reply" if inline.is_none() => a.reply = true,
            "--clear" if inline.is_none() => a.clear = true,
            f if VALUES.contains(&f) => {
                let value = match inline {
                    Some(v) => v,
                    None => match it.next() {
                        Some(v) if !v.starts_with("--") => v.clone(),
                        _ => return unexpected(flag),
                    },
                };
                let slot = match f {
                    "--title" => &mut a.title,
                    "--subtitle" => &mut a.subtitle,
                    "--body" => &mut a.body,
                    "--desktop" => &mut a.desktop,
                    "--workspace" => &mut a.workspace,
                    "--surface" => &mut a.surface,
                    _ => &mut a.window,
                };
                *slot = Some(value);
            }
            _ => return unexpected(arg),
        }
    }
    Ok(a)
}

impl Cmd<'_> {
    /// `{caller, prefer_tty, preferred_*}` from `workspace`, else the env, UUIDs only.
    fn caller_params(&self, workspace: Option<String>) -> Map<String, Value> {
        let mut p = Map::new();
        p.insert("caller".into(), true.into());
        // shortcut: no TTY matching (plan D7), so never prefer the TTY, even inside tmux.
        p.insert("prefer_tty".into(), false.into());
        let env_ws = self.env("CMUX_WORKSPACE_ID").filter(|v| is_uuid(v));
        if let Some(ws) = workspace.or(env_ws.map(str::to_string)) {
            p.insert("preferred_workspace_id".into(), ws.into());
        }
        if let Some(s) = self.env("CMUX_SURFACE_ID").filter(|v| is_uuid(v)) {
            p.insert("preferred_surface_id".into(), s.into());
        }
        p
    }

    /// The current workspace of the window in `scope`.
    fn window_workspace(&mut self, scope: &Map<String, Value>) -> Result<String, CliError> {
        let v = self.call("workspace.current", scope.clone())?;
        match v["workspace_id"].as_str() {
            Some(ws) => Ok(ws.to_string()),
            None => fail(
                "notify: targeted window has no current workspace. Select a workspace in that window or pass --workspace <id|ref|index>.",
            ),
        }
    }
}

pub(crate) fn notify(cmd: &mut Cmd) -> Result<String, CliError> {
    let a = parse(&cmd.args.clone())?;
    let mut scope = Map::new();
    if let Some(w) = a.window.clone().or(cmd.g.window.clone()) {
        let id = cmd.resolve(Kind::Window, &w, &Map::new())?;
        scope.insert("window_id".into(), id.into());
    }
    let windowed = scope.contains_key("window_id");
    if a.clear {
        return clear(cmd, &a, scope);
    }
    let desktop = match &a.desktop {
        Some(v) => Some(parse_bool(v).map_or_else(|| fail("--desktop must be true|false"), Ok)?),
        None => None,
    };
    let mut params = Map::new();
    params.insert(
        "title".into(),
        a.title.clone().unwrap_or("Notification".into()).into(),
    );
    params.insert(
        "subtitle".into(),
        a.subtitle.clone().unwrap_or_default().into(),
    );
    params.insert("body".into(), a.body.clone().unwrap_or_default().into());
    if a.reply {
        params.insert("reply_shape".into(), "text".into());
    }
    if let Some(d) = desktop {
        params.insert("effects".into(), serde_json::json!({ "desktop": d }));
    }
    let workspace = match &a.workspace {
        Some(w) => Some(cmd.resolve(Kind::Workspace, w, &scope)?),
        None => None,
    };
    let method = match &a.surface {
        Some(s) if !is_uuid(s) || a.workspace.as_deref().is_some_and(|w| !is_uuid(w)) => {
            let ws = match workspace {
                Some(ws) => ws,
                None if windowed => cmd.window_workspace(&scope)?,
                None => match cmd.env("CMUX_WORKSPACE_ID") {
                    Some(ws) => ws.to_string(),
                    None => cmd.window_workspace(&scope)?,
                },
            };
            let mut target = Map::new();
            target.insert("workspace_id".into(), ws.clone().into());
            let surface = cmd.resolve_uuid(Kind::Surface, s, &target)?;
            params.insert("workspace_id".into(), ws.into());
            params.insert("surface_id".into(), surface.into());
            "notification.create_for_target"
        }
        Some(s) => {
            params.insert("surface_id".into(), s.clone().into());
            if let Some(ws) = workspace {
                params.insert("workspace_id".into(), ws.into());
            }
            params.extend(scope);
            "notification.create"
        }
        None if windowed => {
            let ws = match workspace {
                Some(ws) => ws,
                None => cmd.window_workspace(&scope)?,
            };
            params.insert("workspace_id".into(), ws.into());
            params.extend(scope);
            "notification.create"
        }
        None => {
            params.extend(cmd.caller_params(workspace));
            "notification.create_for_caller"
        }
    };
    let v = cmd.call(method, params)?;
    if cmd.g.json {
        return Ok(cmd.json(v, &[]));
    }
    Ok(match v["id"].as_str() {
        None => "OK\n".into(),
        Some(id) => match cmd.id_format() {
            IdFormat::Refs => format!("OK notification:{id}\n"),
            IdFormat::Uuids => format!("OK {id}\n"),
            IdFormat::Both => format!("OK notification:{id} ({id})\n"),
        },
    })
}

fn clear(cmd: &mut Cmd, a: &Args, scope: Map<String, Value>) -> Result<String, CliError> {
    let windowed = scope.contains_key("window_id");
    if a.surface.is_some() && a.workspace.is_none() && !windowed {
        return fail(
            "notify --clear --surface requires workspace or window context; specify --workspace or --window",
        );
    }
    let workspace = match &a.workspace {
        Some(w) => Some(cmd.resolve(Kind::Workspace, w, &scope)?),
        None if windowed => Some(cmd.window_workspace(&scope)?),
        None => None,
    };
    let params = match (workspace, &a.surface) {
        (Some(ws), Some(s)) => {
            let mut target = Map::new();
            target.insert("workspace_id".into(), ws.clone().into());
            let surface = cmd.resolve_uuid(Kind::Surface, s, &target)?;
            target.insert("surface_id".into(), surface.into());
            target
        }
        (Some(ws), None) => {
            let mut target = Map::new();
            target.insert("workspace_id".into(), ws.into());
            target
        }
        (None, _) => cmd.caller_params(None),
    };
    let v = cmd.call("notification.clear", params)?;
    if cmd.g.json {
        return Ok(cmd.json(v, &[]));
    }
    Ok("OK\n".into())
}

pub(crate) fn dismiss(cmd: &mut Cmd) -> Result<String, CliError> {
    let args = cmd.args.clone();
    let mut id = None;
    let mut all_read = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--all-read" => all_read = true,
            "--id" => match it.next() {
                Some(v) => id = Some(v.clone()),
                None => return fail("dismiss-notification: unexpected arguments: --id"),
            },
            a if a.starts_with("--id=") => id = Some(a[5..].to_string()),
            other => {
                return fail(format!(
                    "dismiss-notification: unexpected arguments: {other}"
                ));
            }
        }
    }
    let mut params = Map::new();
    match (id, all_read) {
        (Some(id), false) => {
            let id = id.trim();
            let bare = match id.get(..13) {
                Some(p) if p.eq_ignore_ascii_case("notification:") => &id[13..],
                _ => id,
            };
            params.insert("id".into(), bare.into());
        }
        (None, true) => {
            params.insert("all_read".into(), true.into());
        }
        _ => return fail("dismiss-notification requires exactly one of --id or --all-read"),
    }
    let v = cmd.call("notification.dismiss", params)?;
    if cmd.g.json {
        return Ok(cmd.json(v, &[]));
    }
    Ok("OK\n".into())
}
