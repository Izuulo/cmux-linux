//! `cmux workspace-group <subcommand>` (M8 plan D11, research-groups.md §12.5): cmux's
//! subcommands and output lines, minus colour, icon, move and idempotency keys.

use serde_json::{Map, Value};

use super::commands::flags;
use super::{CliError, Cmd, fail};
use crate::handles::Kind;
use crate::json::row_handle;

const VALUES: &[&str] = &[
    "--name",
    "--cwd",
    "--from",
    "--group",
    "--workspace",
    "--placement",
    "--window",
];

fn plural(n: u64) -> &'static str {
    if n == 1 { "" } else { "s" }
}

pub(crate) fn workspace_group(cmd: &mut Cmd) -> Result<String, CliError> {
    let args = cmd.args.clone();
    let Some((sub, rest)) = args.split_first() else {
        return fail("workspace-group requires a subcommand (see cmux workspace-group --help)");
    };
    let sub = sub.as_str();
    let f = flags(
        &format!("workspace-group {sub}"),
        rest,
        VALUES,
        &["--close-workspaces"],
    )?;
    let mut params = cmd.window_scope(&f)?;
    let scope = params.clone();
    // The group: `--group`, else the first positional.
    let group = |params: &mut Map<String, Value>| match f
        .get("--group")
        .or(f.positional.first().map(String::as_str))
    {
        Some(g) => {
            params.insert("group_id".into(), g.trim().into());
            Ok(())
        }
        None => fail(format!(
            "workspace-group {sub} requires a group id or --group <id>"
        )),
    };
    let method = match sub {
        "list" => {
            let v = cmd.call("workspace.group.list", params)?;
            if cmd.g.json {
                return Ok(cmd.json(v, &[]));
            }
            let rows = v["groups"].as_array().cloned().unwrap_or_default();
            if rows.is_empty() {
                return Ok("No groups\n".into());
            }
            return Ok(rows
                .iter()
                .map(|r| {
                    let mut line = format!(
                        "{}  {}  ({} members)",
                        row_handle(r["id"].as_str(), r["ref"].as_str(), cmd.id_format()),
                        r["name"].as_str().unwrap_or(""),
                        r["member_count"].as_u64().unwrap_or(0),
                    );
                    for (key, tag) in [("is_pinned", " [pinned]"), ("is_collapsed", " [collapsed]")]
                    {
                        if r[key].as_bool() == Some(true) {
                            line.push_str(tag);
                        }
                    }
                    line + "\n"
                })
                .collect());
        }
        "create" => {
            params.insert("name".into(), f.get("--name").unwrap_or("").into());
            if let Some(cwd) = f.get("--cwd") {
                params.insert("cwd".into(), cmd.absolute(cwd).into());
            }
            if let Some(from) = f.get("--from") {
                let ids = from
                    .split(',')
                    .filter(|h| !h.trim().is_empty())
                    .map(|h| cmd.resolve(Kind::Workspace, h, &scope).map(Value::from))
                    .collect::<Result<Vec<_>, _>>()?;
                params.insert("child_workspace_ids".into(), ids.into());
            }
            let v = cmd.call("workspace.group.create", params)?;
            if cmd.g.json {
                return Ok(cmd.json(v, &[]));
            }
            let g = &v["group"];
            return Ok(format!(
                "OK {}\n",
                row_handle(g["id"].as_str(), g["ref"].as_str(), cmd.id_format())
            ));
        }
        "ungroup" | "delete" => {
            group(&mut params)?;
            if sub == "delete" && f.bools.iter().any(|b| b == "--close-workspaces") {
                params.insert("close_workspaces".into(), true.into());
            }
            let v = cmd.call(&format!("workspace.group.{sub}"), params)?;
            if cmd.g.json {
                return Ok(cmd.json(v, &[]));
            }
            return Ok(match v["closed_workspace_count"].as_u64() {
                Some(n) => format!("OK group deleted (closed {n} workspace{})\n", plural(n)),
                None => {
                    let n = v["kept_workspace_count"].as_u64().unwrap_or(0);
                    format!("OK group dissolved (kept {n} workspace{})\n", plural(n))
                }
            });
        }
        "rename" => {
            group(&mut params)?;
            let Some(name) = f.get("--name") else {
                return fail("workspace-group rename requires --name <name>");
            };
            params.insert("name".into(), name.into());
            "workspace.group.rename"
        }
        "collapse" | "expand" | "pin" | "unpin" | "focus" => {
            group(&mut params)?;
            &format!("workspace.group.{sub}")
        }
        "add" | "set-anchor" => {
            let (Some(g), Some(ws)) = (f.get("--group"), f.get("--workspace")) else {
                return fail(format!(
                    "workspace-group {sub} requires --group <id> --workspace <id>"
                ));
            };
            params.insert("group_id".into(), g.trim().into());
            let ws = cmd.resolve(Kind::Workspace, ws, &scope)?;
            params.insert("workspace_id".into(), ws.into());
            if sub == "add" {
                "workspace.group.add"
            } else {
                "workspace.group.set_anchor"
            }
        }
        "remove" => {
            let Some(ws) = f.get("--workspace") else {
                return fail("workspace-group remove requires --workspace <id>");
            };
            let ws = cmd.resolve(Kind::Workspace, ws, &scope)?;
            params.insert("workspace_id".into(), ws.into());
            "workspace.group.remove"
        }
        "new-workspace" => {
            group(&mut params)?;
            if let Some(p) = f.get("--placement") {
                params.insert("placement".into(), p.into());
            }
            let v = cmd.call("workspace.group.new_workspace", params)?;
            if cmd.g.json {
                return Ok(cmd.json(v, &[]));
            }
            let ws = v["workspace_ref"].as_str().or(v["workspace_id"].as_str());
            return Ok(format!("OK {}\n", ws.unwrap_or("")));
        }
        other => return fail(format!("Unknown workspace-group subcommand: {other}")),
    };
    let v = cmd.call(method, params)?;
    Ok(if cmd.g.json {
        cmd.json(v, &[])
    } else {
        "OK\n".into()
    })
}
