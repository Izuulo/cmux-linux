//! The ported commands (rc §8, plan D5), apart from `notify`/`dismiss-notification`.

use std::collections::HashMap;

use serde_json::{Map, Value};

use super::{CliError, Cmd, fail};
use crate::handles::Kind;
use crate::json::{format_ids, pretty, row_handle, text_handle};

/// Parsed command flags: `--x v`, `--x=v`, bare `--x` for `bools`, and positionals.
pub(crate) struct Flags {
    pub values: HashMap<String, String>,
    pub bools: Vec<String>,
    pub positional: Vec<String>,
}

impl Flags {
    pub fn get(&self, flag: &str) -> Option<&str> {
        self.values.get(flag).map(String::as_str)
    }
}

/// Unknown flags fail as `<cmd>: unknown option --x`; a value flag without one as
/// `<cmd>: --x requires a value`. Everything after `--` is positional.
pub(crate) fn flags(
    name: &str,
    args: &[String],
    values: &[&str],
    bools: &[&str],
) -> Result<Flags, CliError> {
    let mut out = Flags {
        values: HashMap::new(),
        bools: Vec::new(),
        positional: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "--" {
            out.positional.extend(it.by_ref().cloned());
            break;
        }
        if !arg.starts_with("--") {
            out.positional.push(arg.clone());
            continue;
        }
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) => (f, Some(v.to_string())),
            None => (arg.as_str(), None),
        };
        if values.contains(&flag) {
            match inline.or_else(|| it.next().cloned()) {
                Some(v) => out.values.insert(flag.to_string(), v),
                None => return fail(format!("{name}: {flag} requires a value")),
            };
        } else if bools.contains(&flag) && inline.is_none() {
            out.bools.push(flag.to_string());
        } else {
            return fail(format!("{name}: unknown option {flag}"));
        }
    }
    Ok(out)
}

fn no_positional(name: &str, f: &Flags) -> Result<(), CliError> {
    match f.positional.first() {
        Some(p) => fail(format!("{name}: unexpected argument {p}")),
        None => Ok(()),
    }
}

fn is_uuid(s: &str) -> bool {
    s.len() == 36 && uuid::Uuid::parse_str(s).is_ok()
}

fn capitalized(kind: Kind) -> String {
    let name = kind.name();
    name[..1].to_uppercase() + &name[1..]
}

impl Cmd<'_> {
    /// cmux's CLI handle resolution (rc §6): a UUID passes; a ref passes (workspace and window
    /// refs are checked first); an index is looked up in the matching list; else an error.
    /// `scope` carries the `window_id`/`workspace_id` the lookup runs in.
    pub(crate) fn resolve(
        &mut self,
        kind: Kind,
        handle: &str,
        scope: &Map<String, Value>,
    ) -> Result<String, CliError> {
        let handle = handle.trim();
        if handle.is_empty() {
            return fail(format!("{} handle is blank", capitalized(kind)));
        }
        if is_uuid(handle) {
            return Ok(handle.to_string());
        }
        let (method, field) = match kind {
            Kind::Window => ("window.list", "windows"),
            Kind::Workspace => ("workspace.list", "workspaces"),
            Kind::Pane => ("pane.list", "panes"),
            Kind::Surface => ("surface.list", "surfaces"),
        };
        let is_ref = handle.split_once(':').is_some_and(|(k, n)| {
            (k == kind.name() || (kind == Kind::Surface && k == "tab"))
                && !n.is_empty()
                && n.bytes().all(|b| b.is_ascii_digit())
        });
        let index = handle.parse::<u64>().ok();
        if !is_ref && index.is_none() {
            return fail(format!(
                "Invalid {k} handle: {handle} (expected UUID, ref like {k}:1, or index)",
                k = kind.name()
            ));
        }
        if is_ref && matches!(kind, Kind::Pane | Kind::Surface) {
            return Ok(handle.to_string());
        }
        let list = self.call(method, scope.clone())?;
        let rows = list[field].as_array().cloned().unwrap_or_default();
        let found = rows.iter().find(|r| match index {
            Some(i) if !is_ref => r["index"].as_u64() == Some(i),
            _ => r["ref"].as_str() == Some(handle),
        });
        match (found, is_ref, kind) {
            (Some(row), _, _) => Ok(row["id"].as_str().unwrap_or(handle).to_string()),
            (None, true, Kind::Window) => fail(format!("Window not found: {handle}")),
            (None, true, _) => fail(format!("{} ref not found: {handle}", capitalized(kind))),
            (None, false, _) => fail(format!("{} index not found", capitalized(kind))),
        }
    }

    /// `--window` from the command, else the global one, as `window_id`.
    pub(crate) fn window_scope(&mut self, f: &Flags) -> Result<Map<String, Value>, CliError> {
        let mut scope = Map::new();
        if let Some(w) = f
            .get("--window")
            .map(str::to_string)
            .or(self.g.window.clone())
        {
            let id = self.resolve(Kind::Window, &w, &Map::new())?;
            scope.insert("window_id".into(), id.into());
        }
        Ok(scope)
    }

    /// `--workspace`, else `CMUX_WORKSPACE_ID` when no window was given, into `scope`.
    pub(crate) fn workspace_scope(
        &mut self,
        f: &Flags,
        scope: &mut Map<String, Value>,
    ) -> Result<(), CliError> {
        let ws = match f.get("--workspace") {
            Some(w) => Some(self.resolve(Kind::Workspace, w, scope)?),
            None if !scope.contains_key("window_id") => {
                self.env("CMUX_WORKSPACE_ID").map(str::to_string)
            }
            None => None,
        };
        if let Some(ws) = ws {
            scope.insert("workspace_id".into(), ws.into());
        }
        Ok(())
    }

    /// The window, else the caller's workspace and surface (cmux `applyWindowOrCallerContext`).
    fn window_or_caller(&mut self, f: &Flags) -> Result<Map<String, Value>, CliError> {
        let mut scope = self.window_scope(f)?;
        if scope.is_empty() {
            for (env, key) in [
                ("CMUX_WORKSPACE_ID", "workspace_id"),
                ("CMUX_SURFACE_ID", "surface_id"),
            ] {
                if let Some(v) = self.env(env) {
                    scope.insert(key.into(), v.into());
                }
            }
        }
        Ok(scope)
    }

    /// `--json` output: the payload after `--id-format`. Without an explicit `--id-format`
    /// the `keep` kinds keep their UUIDs (rc §6).
    pub(crate) fn json(&self, mut v: Value, keep: &[&str]) -> String {
        let keep = if self.g.id_format.is_none() {
            keep
        } else {
            &[]
        };
        format_ids(&mut v, self.id_format(), keep);
        pretty(&v) + "\n"
    }

    /// The text handle of `v`'s `<kind>_id`/`<kind>_ref`.
    pub(crate) fn handle_of(&self, v: &Value, kind: &str) -> String {
        text_handle(
            v[format!("{kind}_id")].as_str(),
            v[format!("{kind}_ref")].as_str(),
            self.id_format(),
        )
    }

    /// `OK <a> <b>…` for the kinds' handles, or the JSON payload.
    pub(crate) fn ok(&self, v: Value, kinds: &[&str]) -> String {
        if self.g.json {
            return self.json(v, &[]);
        }
        let handles: Vec<String> = kinds.iter().map(|k| self.handle_of(&v, k)).collect();
        format!("OK {}\n", handles.join(" "))
    }

    fn row(&self, r: &Value) -> String {
        row_handle(r["id"].as_str(), r["ref"].as_str(), self.id_format())
    }
}

pub(crate) fn list_workspaces(cmd: &mut Cmd) -> Result<String, CliError> {
    let f = flags("list-workspaces", &cmd.args.clone(), &["--window"], &[])?;
    let scope = cmd.window_or_caller(&f)?;
    let v = cmd.call("workspace.list", scope)?;
    if cmd.g.json {
        return Ok(cmd.json(v, &["workspace"]));
    }
    let rows = v["workspaces"].as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        return Ok("No workspaces\n".into());
    }
    Ok(rows
        .iter()
        .map(|r| {
            let selected = r["selected"].as_bool() == Some(true);
            let title = r["title"].as_str().filter(|t| !t.is_empty());
            format!(
                "{}{}{}{}\n",
                if selected { "* " } else { "  " },
                cmd.row(r),
                title.map(|t| format!("  {t}")).unwrap_or_default(),
                if selected { "  [selected]" } else { "" },
            )
        })
        .collect())
}

pub(crate) fn current_workspace(cmd: &mut Cmd) -> Result<String, CliError> {
    let f = flags("current-workspace", &cmd.args.clone(), &["--window"], &[])?;
    no_positional("current-workspace", &f)?;
    let scope = cmd.window_or_caller(&f)?;
    let v = cmd.call("workspace.current", scope)?;
    if cmd.g.json {
        return Ok(cmd.json(v, &["workspace"]));
    }
    Ok(cmd.handle_of(&v, "workspace") + "\n")
}

pub(crate) fn list_panes(cmd: &mut Cmd) -> Result<String, CliError> {
    let f = flags(
        "list-panes",
        &cmd.args.clone(),
        &["--workspace", "--window"],
        &[],
    )?;
    no_positional("list-panes", &f)?;
    let mut scope = cmd.window_scope(&f)?;
    cmd.workspace_scope(&f, &mut scope)?;
    let v = cmd.call("pane.list", scope)?;
    if cmd.g.json {
        return Ok(cmd.json(v, &["pane", "surface"]));
    }
    let rows = v["panes"].as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        return Ok("No panes\n".into());
    }
    Ok(rows
        .iter()
        .map(|r| {
            let focused = r["focused"].as_bool() == Some(true);
            let n = r["surface_count"].as_u64().unwrap_or(0);
            format!(
                "{}{}  [{n} {}]{}\n",
                if focused { "* " } else { "  " },
                cmd.row(r),
                if n == 1 { "surface" } else { "surfaces" },
                if focused { "  [focused]" } else { "" },
            )
        })
        .collect())
}

pub(crate) fn list_pane_surfaces(cmd: &mut Cmd) -> Result<String, CliError> {
    let f = flags(
        "list-pane-surfaces",
        &cmd.args.clone(),
        &["--workspace", "--pane", "--window"],
        &[],
    )?;
    no_positional("list-pane-surfaces", &f)?;
    let mut scope = cmd.window_scope(&f)?;
    cmd.workspace_scope(&f, &mut scope)?;
    if let Some(p) = f.get("--pane") {
        let pane = cmd.resolve(Kind::Pane, p, &scope)?;
        scope.insert("pane_id".into(), pane.into());
    }
    let v = cmd.call("pane.surfaces", scope)?;
    if cmd.g.json {
        return Ok(cmd.json(v, &["pane", "surface"]));
    }
    let rows = v["surfaces"].as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        return Ok("No surfaces in pane\n".into());
    }
    Ok(rows
        .iter()
        .map(|r| {
            let selected = r["selected"].as_bool() == Some(true);
            format!(
                "{}{}  {}{}\n",
                if selected { "* " } else { "  " },
                cmd.row(r),
                r["title"].as_str().unwrap_or(""),
                if selected { "  [selected]" } else { "" },
            )
        })
        .collect())
}

pub(crate) fn select_workspace(cmd: &mut Cmd) -> Result<String, CliError> {
    let f = flags(
        "select-workspace",
        &cmd.args.clone(),
        &["--workspace", "--window"],
        &[],
    )?;
    no_positional("select-workspace", &f)?;
    let Some(ws) = f.get("--workspace") else {
        return fail("select-workspace requires --workspace");
    };
    let mut scope = cmd.window_scope(&f)?;
    let ws = cmd.resolve(Kind::Workspace, ws, &scope)?;
    scope.insert("workspace_id".into(), ws.into());
    let v = cmd.call("workspace.select", scope)?;
    Ok(cmd.ok(v, &["workspace"]))
}

pub(crate) fn focus_pane(cmd: &mut Cmd) -> Result<String, CliError> {
    let f = flags(
        "focus-pane",
        &cmd.args.clone(),
        &["--pane", "--workspace", "--window"],
        &[],
    )?;
    let Some(pane) = f.get("--pane").or(f.positional.first().map(String::as_str)) else {
        return fail("focus-pane requires --pane <id|ref>");
    };
    let pane = pane.to_string();
    let mut scope = cmd.window_scope(&f)?;
    cmd.workspace_scope(&f, &mut scope)?;
    let pane = cmd.resolve(Kind::Pane, &pane, &scope)?;
    scope.insert("pane_id".into(), pane.into());
    let v = cmd.call("pane.focus", scope)?;
    Ok(cmd.ok(v, &["pane", "workspace"]))
}

pub(crate) fn focus_panel(cmd: &mut Cmd) -> Result<String, CliError> {
    let f = flags(
        "focus-panel",
        &cmd.args.clone(),
        &["--panel", "--workspace", "--window"],
        &[],
    )?;
    no_positional("focus-panel", &f)?;
    let Some(panel) = f.get("--panel").map(str::to_string) else {
        return fail("focus-panel requires --panel");
    };
    let mut scope = cmd.window_scope(&f)?;
    cmd.workspace_scope(&f, &mut scope)?;
    let surface = cmd.resolve(Kind::Surface, &panel, &scope)?;
    scope.insert("surface_id".into(), surface.into());
    let v = cmd.call("surface.focus", scope)?;
    Ok(cmd.ok(v, &["surface", "workspace"]))
}
