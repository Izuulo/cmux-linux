//! `cmux hooks feed --source claude`: Claude Code's `PermissionRequest` hook (plan M7, D1/D3).
//! Pushes the request to the app's Feed, waits for the answer and prints Claude's decision.
//! Every path that isn't a clear answer prints `{}`, so Claude shows its own prompt.

use std::io::Read;
use std::time::Duration;

use serde_json::{Value, json};

use super::client::Client;
use super::commands::{is_uuid, parse_bool};
use super::{CliError, Cmd, fail};
use crate::feed;

/// Past the app's 115 s wait, inside the hook's 125 s timeout (plan D1).
const READ_TIMEOUT: Duration = Duration::from_secs(118);

pub(crate) fn hooks(cmd: &mut Cmd) -> Result<String, CliError> {
    match cmd.args.first().map(String::as_str) {
        Some("feed") => feed_hook(cmd),
        Some(sub @ ("setup" | "uninstall")) => fail(format!("hooks {sub} is not available yet")),
        _ => fail("Usage: cmux hooks <setup|uninstall|feed> ..."),
    }
}

fn feed_hook(cmd: &mut Cmd) -> Result<String, CliError> {
    let mut source = None;
    let mut it = cmd.args.iter().skip(1);
    while let Some(a) = it.next() {
        if a == "--source" {
            source = it.next().cloned();
        } else if let Some(v) = a.strip_prefix("--source=") {
            source = Some(v.to_string());
        }
    }
    let Some(source) = source else {
        return fail("cmux hooks feed requires --source <agent-name>");
    };
    let empty = || Ok("{}\n".to_string());
    if source != "claude" {
        return empty();
    }
    let linux = cmd.env("CMUX_LINUX").and_then(parse_bool) == Some(true);
    let ids = (cmd.env("CMUX_WORKSPACE_ID"), cmd.env("CMUX_SURFACE_ID"));
    let (true, (Some(ws), Some(surf))) = (linux, ids) else {
        return empty();
    };
    if !is_uuid(ws) || !is_uuid(surf) {
        return empty();
    }
    let raw = match &cmd.ctx.stdin {
        Some(s) => s.clone(),
        None => {
            let mut s = String::new();
            let _ = std::io::stdin().read_to_string(&mut s);
            s
        }
    };
    let Ok(stdin @ Value::Object(_)) = serde_json::from_str::<Value>(&raw) else {
        return empty();
    };
    let id = uuid::Uuid::new_v4().to_string().to_uppercase();
    let params = feed::push_params(&stdin, ws, surf, &id);
    let reply = push(cmd, &params);
    let out = reply
        .filter(|r| r.get("status").and_then(Value::as_str) == Some("resolved"))
        .and_then(|r| feed::decision_from_json(r.get("decision")?))
        .map(|d| feed::hook_output(&stdin, &d))
        .unwrap_or_else(|| json!({}));
    Ok(format!("{out}\n"))
}

/// The `feed.push` result, or `None` on any socket or server error.
fn push(cmd: &Cmd, params: &serde_json::Map<String, Value>) -> Option<Value> {
    let path = cmd.socket_path().ok()?;
    let mut client = Client::connect(&path, READ_TIMEOUT).ok()?;
    if let Some(password) = cmd.password() {
        client.auth(&password).ok()?;
    }
    client.call("feed.push", params).ok()?.ok()
}
