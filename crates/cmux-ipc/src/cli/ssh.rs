//! `cmux ssh <destination>` (M11 plan S1): a workspace whose terminals all run one ssh argv.

use std::collections::HashMap;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

use super::commands::{Flags, parse_bool};
use super::{CliError, Cmd, fail};
use crate::ssh::{self, Target};

const USAGE: &str = "Usage: cmux ssh <destination> [--port <n>] [--identity <path>] [--name <title>] [--command <text>] [--ssh-option <opt>]... [--focus <true|false>]";

pub(crate) fn ssh(cmd: &mut Cmd) -> Result<String, CliError> {
    let args = cmd.args.clone();
    let mut t = Target::default();
    let mut values: HashMap<String, String> = HashMap::new();
    let mut dest = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if !arg.starts_with("--") {
            if dest.replace(arg.clone()).is_some() {
                return fail(format!("ssh: unexpected argument {arg}\n{USAGE}"));
            }
            continue;
        }
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) => (f, Some(v.to_string())),
            None => (arg.as_str(), None),
        };
        if !matches!(
            flag,
            "--port"
                | "--identity"
                | "--name"
                | "--command"
                | "--ssh-option"
                | "--focus"
                | "--window"
        ) {
            return fail(format!("ssh: unknown option {flag}\n{USAGE}"));
        }
        let Some(v) = inline.or_else(|| it.next().cloned()) else {
            return fail(format!("ssh: {flag} requires a value"));
        };
        if flag == "--ssh-option" {
            t.options.push(v);
        } else {
            values.insert(flag.to_string(), v);
        }
    }
    let Some(dest) = dest.filter(|d| !d.trim().is_empty()) else {
        return fail(USAGE);
    };
    // ssh would read it as an option.
    if dest.starts_with('-') {
        return fail(format!("ssh: invalid destination `{dest}`"));
    }
    t.destination = dest;
    if let Some(p) = values.get("--port") {
        match p.parse::<u16>() {
            Ok(n) if n > 0 => t.port = Some(n),
            _ => return fail("ssh: --port must be 1-65535"),
        }
    }
    t.identity = values.get("--identity").map(|i| cmd.absolute(i));
    let focus = match values.get("--focus") {
        None => true,
        Some(v) => parse_bool(v).map_or_else(|| fail("--focus must be true|false"), Ok)?,
    };
    let env = |k: &str| cmd.ctx.env.get(k).cloned();
    let Some(dir) = ssh::control_dir(&env) else {
        return fail("ssh: no state directory (set HOME or XDG_STATE_HOME)");
    };
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)
        .and_then(|()| std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)))
        .or_else(|e| fail(format!("ssh: {}: {e}", dir.display())))?;
    let f = Flags {
        values: values
            .get("--window")
            .map(|w| HashMap::from([("--window".to_string(), w.clone())]))
            .unwrap_or_default(),
        bools: Vec::new(),
        positional: Vec::new(),
    };
    let mut params = cmd.window_or_caller(&f)?;
    let title = values.get("--name").unwrap_or(&t.destination);
    params.insert("title".into(), title.as_str().into());
    params.insert("remote".into(), ssh::argv(&t, &dir).into());
    if let Some(c) = values.get("--command").filter(|c| !c.trim().is_empty()) {
        params.insert("initial_input".into(), format!("{c}\r").into());
    }
    params.insert("focus".into(), focus.into());
    let v = cmd.call("workspace.create", params)?;
    let ws = v["workspace_ref"].as_str().or(v["workspace_id"].as_str());
    Ok(format!("OK {}\n", ws.unwrap_or("")))
}
