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

pub(crate) fn is_uuid(s: &str) -> bool {
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
        self.resolve_as(kind, handle, scope, false)
    }

    /// `resolve`, but pane and surface refs are looked up too, so the result is a UUID.
    pub(crate) fn resolve_uuid(
        &mut self,
        kind: Kind,
        handle: &str,
        scope: &Map<String, Value>,
    ) -> Result<String, CliError> {
        self.resolve_as(kind, handle, scope, true)
    }

    fn resolve_as(
        &mut self,
        kind: Kind,
        handle: &str,
        scope: &Map<String, Value>,
        uuid: bool,
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
            Kind::WorkspaceGroup => ("workspace.group.list", "groups"),
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
        if is_ref && !uuid && matches!(kind, Kind::Pane | Kind::Surface) {
            return Ok(handle.to_string());
        }
        // `tab:N` is `surface:N` (rc §6).
        let handle = &match handle.strip_prefix("tab:") {
            Some(n) if kind == Kind::Surface => format!("surface:{n}"),
            _ => handle.to_string(),
        };
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
    pub(crate) fn window_or_caller(&mut self, f: &Flags) -> Result<Map<String, Value>, CliError> {
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

/// `1/true/yes/on` or `0/false/no/off`.
pub(crate) fn parse_bool(v: &str) -> Option<bool> {
    match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn focus(f: &Flags) -> Result<bool, CliError> {
    match f.get("--focus") {
        None => Ok(false),
        Some(v) => parse_bool(v).map_or_else(|| fail("--focus must be true|false"), Ok),
    }
}

/// `--command <text>` as typed input: the text plus Enter; blank text is dropped.
fn initial_input(f: &Flags, params: &mut Map<String, Value>) {
    if let Some(c) = f.get("--command").filter(|c| !c.trim().is_empty()) {
        params.insert("initial_input".into(), format!("{c}\r").into());
    }
}

impl Cmd<'_> {
    /// Window, workspace and surface for the surface commands (`send`, `send-key`,
    /// `new-split`): `--surface`/`--panel`, else `CMUX_SURFACE_ID` when neither a workspace nor
    /// a window was given; `--workspace`, else `CMUX_WORKSPACE_ID` when no window was given and
    /// the surface isn't a handle that names its own workspace.
    fn surface_scope(&mut self, f: &Flags) -> Result<Map<String, Value>, CliError> {
        let mut scope = self.window_scope(f)?;
        let windowed = scope.contains_key("window_id");
        let surface = f.get("--surface").or(f.get("--panel")).map(str::to_string);
        match f.get("--workspace") {
            Some(w) => {
                let ws = self.resolve(Kind::Workspace, w, &scope)?;
                scope.insert("workspace_id".into(), ws.into());
            }
            None if !windowed && surface.as_deref().is_none_or(|s| s.parse::<u64>().is_ok()) => {
                if let Some(ws) = self.env("CMUX_WORKSPACE_ID") {
                    scope.insert("workspace_id".into(), ws.into());
                }
            }
            None => {}
        }
        let surface = match surface {
            Some(s) => Some(self.resolve(Kind::Surface, &s, &scope)?),
            None if f.get("--workspace").is_none() && !windowed => {
                self.env("CMUX_SURFACE_ID").map(str::to_string)
            }
            None => None,
        };
        if let Some(s) = surface {
            scope.insert("surface_id".into(), s.into());
        }
        Ok(scope)
    }

    /// `OK <surface> <workspace>`, with cmux's suffix when the terminal hasn't started yet.
    fn sent(&self, v: Value) -> String {
        let queued = v["queued"].as_bool() == Some(true);
        let out = self.ok(v, &["surface", "workspace"]);
        if queued && !self.g.json {
            return out.trim_end().to_string()
                + " queued (terminal starting; input will be sent when its PTY is ready)\n";
        }
        out
    }

    /// `~` expanded, relative paths made absolute against the cwd, `.` parts dropped.
    pub(crate) fn absolute(&self, path: &str) -> String {
        let home = self.env("HOME").unwrap_or("");
        let expanded = match path.strip_prefix('~') {
            Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{home}{rest}"),
            _ => path.to_string(),
        };
        let joined = self.ctx.cwd.join(expanded);
        joined
            .components()
            .collect::<std::path::PathBuf>()
            .display()
            .to_string()
    }
}

const NEW_WORKSPACE_FLAGS: &[&str] = &["--name", "--cwd", "--command", "--window", "--focus"];

pub(crate) fn new_workspace(cmd: &mut Cmd) -> Result<String, CliError> {
    let args = cmd.args.clone();
    for a in args.iter().take_while(|a| *a != "--") {
        let flag = a.split('=').next().unwrap_or(a);
        if flag.starts_with("--") && !NEW_WORKSPACE_FLAGS.contains(&flag) {
            return fail(format!(
                "new-workspace: unknown flag '{flag}'. Known flags: --name <title>, --cwd <path>, --command <text>, --window <id|ref|index>, --focus <true|false>"
            ));
        }
    }
    let f = flags("new-workspace", &args, NEW_WORKSPACE_FLAGS, &[]).map_err(|e| {
        if e.msg == "new-workspace: --command requires a value" {
            CliError::from("new-workspace: --command requires <text>".to_string())
        } else {
            e
        }
    })?;
    no_positional("new-workspace", &f)?;
    let focus = focus(&f)?;
    let mut params = cmd.window_or_caller(&f)?;
    if let Some(name) = f.get("--name") {
        params.insert("title".into(), name.into());
    }
    if let Some(cwd) = f.get("--cwd") {
        params.insert("cwd".into(), cmd.absolute(cwd).into());
    }
    initial_input(&f, &mut params);
    params.insert("focus".into(), focus.into());
    let v = cmd.call("workspace.create", params)?;
    // Always the ref, whatever --json/--id-format say (rc §8).
    let ws = v["workspace_ref"].as_str().or(v["workspace_id"].as_str());
    Ok(format!("OK {}\n", ws.unwrap_or("")))
}

pub(crate) fn new_split(cmd: &mut Cmd) -> Result<String, CliError> {
    let args = cmd.args.clone();
    let known = [
        "--workspace",
        "--surface",
        "--panel",
        "--window",
        "--command",
        "--focus",
    ];
    for a in args.iter().take_while(|a| *a != "--") {
        let flag = a.split('=').next().unwrap_or(a);
        if flag.starts_with("--") && !known.contains(&flag) {
            return fail(format!("new-split: unknown flag '{flag}'"));
        }
    }
    let f = flags("new-split", &args, &known, &[])?;
    let Some(dir) = f.positional.first() else {
        return fail("new-split requires a direction");
    };
    let direction = match dir.to_ascii_lowercase().as_str() {
        "left" | "l" => "left",
        "right" | "r" => "right",
        "up" | "u" => "up",
        "down" | "d" => "down",
        _ => return fail("new-split: direction must be left|right|up|down"),
    };
    let focus = focus(&f)?;
    let mut params = cmd.surface_scope(&f)?;
    params.insert("direction".into(), direction.into());
    initial_input(&f, &mut params);
    params.insert("focus".into(), focus.into());
    let v = cmd.call("surface.split", params)?;
    Ok(cmd.ok(v, &["surface", "workspace"]))
}

/// The leading options of `send`/`send-key`, then everything else (after an optional `--`).
fn send_args(name: &str, args: &[String], bools: &[&str]) -> Result<Flags, CliError> {
    let mut f = Flags {
        values: HashMap::new(),
        bools: Vec::new(),
        positional: Vec::new(),
    };
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        let (flag, inline) = match arg.split_once('=') {
            Some((fl, v)) if fl.starts_with("--") => (fl, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        if arg == "--" {
            i += 1;
            break;
        } else if bools.contains(&flag) && inline.is_none() {
            f.bools.push(flag.to_string());
        } else if ["--workspace", "--surface", "--window"].contains(&flag) {
            let value = match inline {
                Some(v) => v,
                None => {
                    i += 1;
                    match args.get(i) {
                        Some(v) => v.clone(),
                        None => return fail(format!("{name}: {flag} requires a value")),
                    }
                }
            };
            f.values.insert(flag.to_string(), value);
        } else {
            break;
        }
        i += 1;
    }
    f.positional = args[i.min(args.len())..].to_vec();
    Ok(f)
}

/// cmux's escapes for typed text: `\n` and `\r` press Enter, `\t` is Tab.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('\\', Some('n' | 'r')) => {
                chars.next();
                out.push('\r');
            }
            ('\\', Some('t')) => {
                chars.next();
                out.push('\t');
            }
            _ => out.push(c),
        }
    }
    out
}

pub(crate) fn send(cmd: &mut Cmd) -> Result<String, CliError> {
    let f = send_args("send", &cmd.args.clone(), &["--paste", "--force"])?;
    let text = f.positional.join(" ");
    if text.is_empty() {
        return fail("send requires text");
    }
    let paste = f.bools.iter().any(|b| b == "--paste");
    let mut params = cmd.surface_scope(&f)?;
    let (method, text) = if paste {
        ("terminal.paste", text)
    } else {
        ("surface.send_text", unescape(&text))
    };
    params.insert("text".into(), text.into());
    let v = cmd.call(method, params)?;
    Ok(cmd.sent(v))
}

pub(crate) fn send_key(cmd: &mut Cmd) -> Result<String, CliError> {
    let f = send_args("send-key", &cmd.args.clone(), &["--force"])?;
    let Some(key) = f.positional.first().cloned() else {
        return fail("send-key requires a key");
    };
    if f.positional.len() > 1 {
        return fail(format!(
            "send-key: unexpected arguments: {}",
            f.positional[1..].join(" ")
        ));
    }
    let mut params = cmd.surface_scope(&f)?;
    params.insert("key".into(), key.into());
    let v = cmd.call("surface.send_key", params)?;
    Ok(cmd.sent(v))
}
