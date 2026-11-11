//! `cmux agent message` and `cmux agent inbox` (M9 plan D3, cmux `docs/agent-messages.md`).

use std::io::Read;

use serde_json::{Map, Value};

use super::commands::flags;
use super::{CliError, Cmd, fail};

const USAGE: &str = "Usage: cmux agent message <target> [--from <name>] <text|-> | cmux agent message --reply-to <id> <text|-> | cmux agent inbox [--surface <target>] [--state <state>] [--limit <n>] [--mark-read]";

pub(crate) fn agent(cmd: &mut Cmd) -> Result<String, CliError> {
    let args = cmd.args.clone();
    match args.split_first() {
        Some((sub, rest)) if sub == "message" => message(cmd, rest),
        Some((sub, rest)) if sub == "inbox" => inbox(cmd, rest),
        _ => fail(USAGE),
    }
}

fn message(cmd: &mut Cmd, args: &[String]) -> Result<String, CliError> {
    let f = flags("agent message", args, &["--from", "--reply-to"], &[])?;
    let mut params = Map::new();
    let text = match (f.get("--reply-to"), f.positional.as_slice()) {
        (Some(id), [text]) => {
            params.insert("reply_to".into(), id.into());
            text
        }
        (None, [target, text]) => {
            params.insert("target".into(), target.as_str().into());
            text
        }
        _ => return fail(USAGE),
    };
    let body = if text == "-" {
        match &cmd.ctx.stdin {
            Some(s) => s.clone(),
            None => {
                let mut s = String::new();
                std::io::stdin()
                    .read_to_string(&mut s)
                    .map_err(|e| format!("agent message: cannot read stdin: {e}"))?;
                s
            }
        }
    } else {
        text.clone()
    };
    params.insert("body".into(), body.into());
    if let Some(from) = f.get("--from") {
        params.insert("from".into(), from.into());
    }
    // Where it comes from: a reply to it finds its way back.
    for (env, key) in [
        ("CMUX_WORKSPACE_ID", "sender_workspace_id"),
        ("CMUX_SURFACE_ID", "sender_surface_id"),
    ] {
        if let Some(v) = cmd.env(env) {
            params.insert(key.into(), v.into());
        }
    }
    let v = cmd.call("agent.message.send", params)?;
    if cmd.g.json {
        return Ok(cmd.json(v, &[]));
    }
    Ok(format!(
        "OK {} queued for {}\n",
        v["id"].as_str().unwrap_or(""),
        v["recipient_workspace_title"].as_str().unwrap_or("")
    ))
}

fn inbox(cmd: &mut Cmd, args: &[String]) -> Result<String, CliError> {
    let f = flags(
        "agent inbox",
        args,
        &["--surface", "--state", "--limit"],
        &["--mark-read"],
    )?;
    if let Some(p) = f.positional.first() {
        return fail(format!("agent inbox: unexpected argument {p}"));
    }
    let mut params = Map::new();
    if let Some(s) = f.get("--surface") {
        params.insert("surface".into(), s.into());
    }
    if let Some(s) = f.get("--state") {
        params.insert("state".into(), s.into());
    }
    if let Some(n) = f.get("--limit") {
        let n: u64 = n
            .parse()
            .map_err(|_| "agent inbox: --limit must be a number".to_string())?;
        params.insert("limit".into(), n.into());
    }
    let v = cmd.call("agent.message.list", params)?;
    let rows = v["messages"].as_array().cloned().unwrap_or_default();
    if f.bools.iter().any(|b| b == "--mark-read") && !rows.is_empty() {
        let ids: Vec<Value> = rows.iter().map(|r| r["id"].clone()).collect();
        let mut params = Map::new();
        params.insert("ids".into(), ids.into());
        cmd.call("agent.message.mark_read", params)?;
    }
    if cmd.g.json {
        return Ok(cmd.json(v, &[]));
    }
    if rows.is_empty() {
        return Ok("No messages\n".into());
    }
    Ok(rows
        .iter()
        .map(|r| {
            let s = |k: &str| r[k].as_str().unwrap_or("");
            format!(
                "{:<9}  {}  {} -> {}: {}\n",
                s("state"),
                s("id"),
                s("from"),
                s("recipient_surface_ref"),
                s("body").lines().next().unwrap_or("")
            )
        })
        .collect())
}
