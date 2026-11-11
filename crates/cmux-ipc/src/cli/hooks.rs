//! `cmux hooks feed --source claude`: Claude Code's `PermissionRequest` hook (plan M7, D1/D3).
//! Pushes the request to the app's Feed, waits for the answer and prints Claude's decision.
//! Every path that isn't a clear answer prints `{}`, so Claude shows its own prompt.

use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
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
        Some("setup") => settings(cmd, true),
        Some("uninstall") => settings(cmd, false),
        _ => fail("Usage: cmux hooks <setup|uninstall|feed> ..."),
    }
}

/// Identifies our entries so a re-install replaces them and an uninstall finds them.
const MARKER: &str = "hooks feed --source claude";

/// D2's hook command; the absolute, single-quoted path keeps another `cmux` from answering, and
/// `test -x ... || :` makes a removed binary a silent no-op.
pub(crate) fn hook_command(exe: &Path) -> String {
    let q = format!("'{}'", exe.to_string_lossy().replace('\'', "'\\''"));
    format!("test -x {q} && {q} {MARKER} || :")
}

/// Drops our hooks, then any group, event or `hooks` object that only we emptied.
fn remove(settings: &mut Value) -> Result<(), String> {
    let Some(hooks) = settings.get_mut("hooks") else {
        return Ok(());
    };
    let Some(events) = hooks.as_object_mut() else {
        return Err("hooks in <path> is not an object".into());
    };
    let ours = |h: &Value| {
        h.get("command")
            .and_then(Value::as_str)
            .is_some_and(|c| c.contains(MARKER))
    };
    events.retain(|_, groups| {
        let Some(groups) = groups.as_array_mut() else {
            return true;
        };
        let before = groups.len();
        groups.retain_mut(|g| {
            let Some(list) = g.get_mut("hooks").and_then(Value::as_array_mut) else {
                return true;
            };
            let n = list.len();
            list.retain(|h| !ours(h));
            n == list.len() || !list.is_empty()
        });
        // Only an event we emptied goes; a user's own empty array stays.
        !(groups.is_empty() && before > 0)
    });
    if events.is_empty() {
        settings.as_object_mut().unwrap().remove("hooks");
    }
    Ok(())
}

pub(crate) fn install(settings: &mut Value, command: &str) -> Result<(), String> {
    remove(settings)?;
    let obj = settings
        .as_object_mut()
        .ok_or("<path> is not a JSON object")?;
    let group =
        json!({"matcher": "", "hooks": [{"type": "command", "command": command, "timeout": 125}]});
    let events = obj.entry("hooks").or_insert_with(|| json!({}));
    let list = events
        .as_object_mut()
        .ok_or("hooks in <path> is not an object")?
        .entry("PermissionRequest")
        .or_insert_with(|| json!([]));
    match list.as_array_mut() {
        Some(a) => a.push(group),
        None => return Err("PermissionRequest in <path> is not an array".into()),
    }
    Ok(())
}

pub(crate) fn uninstall(settings: &mut Value) -> Result<(), String> {
    if !settings.is_object() {
        return Err("<path> is not a JSON object".into());
    }
    remove(settings)
}

fn settings(cmd: &mut Cmd, install_it: bool) -> Result<String, CliError> {
    let name = if install_it {
        "hooks setup"
    } else {
        "hooks uninstall"
    };
    let mut file = None;
    let mut it = cmd.args.iter().skip(1);
    while let Some(a) = it.next() {
        if a == "--settings-file" {
            match it.next() {
                Some(v) => file = Some(v.clone()),
                None => return fail(format!("{name}: --settings-file requires a value")),
            }
        } else if let Some(v) = a.strip_prefix("--settings-file=") {
            file = Some(v.to_string());
        } else {
            return fail(format!("{name}: unknown option {a}"));
        }
    }
    let path = match (file, cmd.env("CLAUDE_CONFIG_DIR"), cmd.env("HOME")) {
        (Some(f), ..) => PathBuf::from(f),
        (None, Some(d), _) => Path::new(d).join("settings.json"),
        (None, None, Some(h)) => Path::new(h).join(".claude/settings.json"),
        _ => return fail("cannot find Claude Code's settings directory"),
    };
    let command = match (install_it, &cmd.ctx.exe) {
        (true, Some(e)) => hook_command(e),
        (true, None) => return fail("cannot find the cmux binary"),
        _ => String::new(),
    };
    // Follow a symlink so the link stays a link.
    let target = std::fs::canonicalize(&path).unwrap_or_else(|_| dangling_target(&path));
    let p = path.display();
    let old = match std::fs::read_to_string(&target) {
        Ok(t) if t.trim().is_empty() => Value::Object(Default::default()),
        Ok(t) => {
            serde_json::from_str::<Value>(&t).map_err(|e| format!("{p} is not valid JSON: {e}"))?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Value::Object(Default::default()),
        Err(e) => return fail(format!("cannot read {p}: {e}")),
    };
    let exists = target.exists();
    let mut new = old.clone();
    let done = if install_it {
        install(&mut new, &command)
    } else {
        uninstall(&mut new)
    };
    if let Err(e) = done {
        return fail(e.replace("<path>", &p.to_string()));
    }
    let changed = new != old || (install_it && !exists);
    if changed {
        write_atomic(&target, &new).map_err(|e| format!("cannot write {p}: {e}"))?;
    }
    Ok(match (install_it, changed) {
        (true, true) => format!(
            "cmux: installed Feed hooks in {p}\ncmux: restart running Claude Code sessions to load them\n"
        ),
        (true, false) => format!("cmux: Feed hooks are already installed in {p}\n"),
        (false, true) => format!("cmux: removed Feed hooks from {p}\n"),
        (false, false) => format!("cmux: no Feed hooks installed in {p}\n"),
    })
}

/// Where a dangling symlink points (so the link stays a link); `path` itself otherwise.
fn dangling_target(path: &Path) -> PathBuf {
    match std::fs::read_link(path) {
        Ok(t) => path.parent().map_or(t.clone(), |d| d.join(&t)),
        Err(_) => path.to_path_buf(),
    }
}

fn write_atomic(target: &Path, value: &Value) -> std::io::Result<()> {
    let dir = target.parent().unwrap_or(Path::new("."));
    if !dir.exists() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
    }
    let mode = std::fs::metadata(target).map(|m| m.permissions().mode() & 0o7777);
    let tmp = dir.join(format!(".cmux-settings-{}.tmp", std::process::id()));
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)?;
    let res = (|| {
        if let Ok(m) = mode {
            f.set_permissions(std::fs::Permissions::from_mode(m))?;
        }
        f.write_all(format!("{}\n", serde_json::to_string_pretty(value)?).as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&tmp, target)
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
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

#[cfg(test)]
mod tests {
    use super::*;

    const CMD: &str = "test -x '/a/cmux' && '/a/cmux' hooks feed --source claude || :";

    #[test]
    fn hook_command_quotes_path() {
        assert_eq!(hook_command(Path::new("/a/cmux")), CMD);
        assert_eq!(
            hook_command(Path::new("/a b/it's/cmux")),
            "test -x '/a b/it'\\''s/cmux' && '/a b/it'\\''s/cmux' hooks feed --source claude || :"
        );
    }

    #[test]
    fn install_then_uninstall_round_trips() {
        let orig = json!({"model":"x","hooks":{"Stop":[{"hooks":[{"type":"command","command":"s"}]}],
            "PermissionRequest":[{"matcher":"Bash","hooks":[{"type":"command","command":"mine"}]}]}});
        let mut v = orig.clone();
        install(&mut v, CMD).unwrap();
        let once = v.clone();
        install(
            &mut v,
            "test -x '/b' && '/b' hooks feed --source claude || :",
        )
        .unwrap();
        assert_eq!(v["hooks"]["PermissionRequest"].as_array().unwrap().len(), 2);
        install(&mut v, CMD).unwrap();
        assert_eq!(v, once);
        uninstall(&mut v).unwrap();
        assert_eq!(v, orig);
        let mut e = json!({});
        install(&mut e, CMD).unwrap();
        uninstall(&mut e).unwrap();
        assert_eq!(e, json!({}));
    }

    #[test]
    fn rejects_non_objects() {
        assert_eq!(
            install(&mut json!([]), CMD).unwrap_err(),
            "<path> is not a JSON object"
        );
        assert_eq!(
            uninstall(&mut json!({"hooks":[]})).unwrap_err(),
            "hooks in <path> is not an object"
        );
    }
}
