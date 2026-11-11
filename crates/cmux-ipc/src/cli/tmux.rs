//! Claude Teams (M9 plan Part 4): `cmux claude-teams` starts Claude Code's teammate mode with a
//! private `tmux` on `PATH`; that script runs `cmux __tmux-compat`, which turns the tmux
//! commands a team issues into our splits. A tmux pane `%N` is our `surface:N` (plan T2).

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;

use serde_json::{Map, Value};

use super::commands::parse_bool;
use super::{CliError, Cmd, fail};
use crate::paths;

/// tmux-style options: clustered (`-dP`), a value attached or next, ending at the first
/// argument that is not an option.
struct Parsed {
    values: HashMap<char, String>,
    bools: Vec<char>,
    positional: Vec<String>,
}

impl Parsed {
    fn has(&self, flag: char) -> bool {
        self.bools.contains(&flag)
    }

    fn value(&self, flag: char) -> Option<&str> {
        self.values.get(&flag).map(String::as_str)
    }
}

fn parse(args: &[String], value_flags: &str, bool_flags: &str) -> Result<Parsed, CliError> {
    let mut p = Parsed {
        values: HashMap::new(),
        bools: Vec::new(),
        positional: Vec::new(),
    };
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if !a.starts_with('-') || a.len() < 2 {
            break;
        }
        for (at, c) in a.char_indices().skip(1) {
            if value_flags.contains(c) {
                let rest = &a[at + c.len_utf8()..];
                let v = if rest.is_empty() {
                    i += 1;
                    args.get(i).cloned()
                } else {
                    Some(rest.to_string())
                };
                let Some(v) = v else {
                    return fail(format!("tmux: option requires an argument -- {c}"));
                };
                p.values.insert(c, v);
                break;
            } else if bool_flags.contains(c) {
                p.bools.push(c);
            } else {
                return fail(format!("Unsupported tmux compatibility flag: -{c}"));
            }
        }
        i += 1;
    }
    p.positional = args[i.min(args.len())..].to_vec();
    Ok(p)
}

/// One terminal of the caller's workspace, as tmux sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pane {
    /// The N of `%N` and of `surface:N`.
    n: String,
    id: String,
    index: usize,
    title: String,
    active: bool,
}

fn panes(cmd: &mut Cmd) -> Result<Vec<Pane>, CliError> {
    let mut params = Map::new();
    if let Some(ws) = cmd.env("CMUX_WORKSPACE_ID") {
        params.insert("workspace_id".into(), ws.into());
    }
    let v = cmd.call("surface.list", params)?;
    let text = |row: &Value, key: &str| row[key].as_str().unwrap_or("").to_string();
    Ok(v["surfaces"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, row)| Pane {
            n: text(row, "ref").trim_start_matches("surface:").to_string(),
            id: text(row, "id"),
            index,
            title: text(row, "title"),
            active: row["selected"] == true,
        })
        .collect())
}

/// The terminal this command runs in, else the focused one.
fn caller<'a>(cmd: &Cmd, panes: &'a [Pane]) -> Result<&'a Pane, CliError> {
    let own = cmd.env("CMUX_SURFACE_ID");
    let found = panes
        .iter()
        .find(|p| own.is_some_and(|id| p.id.eq_ignore_ascii_case(id)))
        .or(panes.iter().find(|p| p.active))
        .or(panes.first());
    found.ok_or_else(|| CliError::from("tmux: no current pane".to_string()))
}

/// `-t %N` (also as the end of `session:window.%N`), else `TMUX_PANE`, else the caller. A
/// session or window target means the caller's workspace, so its pane.
fn target<'a>(cmd: &Cmd, panes: &'a [Pane], t: Option<&str>) -> Result<&'a Pane, CliError> {
    let want = t.or(cmd.env("TMUX_PANE")).unwrap_or("");
    match want.rsplit_once('%') {
        Some((_, n)) => panes
            .iter()
            .find(|p| p.n == n)
            .ok_or_else(|| CliError::from(format!("can't find pane: %{n}"))),
        None => caller(cmd, panes),
    }
}

/// tmux `#{name}` formats, for the names a team asks for; unknown ones are empty.
fn render(format: &str, p: &Pane) -> String {
    let mut out = String::new();
    let mut rest = format;
    while let Some(at) = rest.find("#{") {
        out.push_str(&rest[..at]);
        let Some(end) = rest[at..].find('}') else {
            rest = &rest[at..];
            break;
        };
        out.push_str(&match &rest[at + 2..at + end] {
            "pane_id" => format!("%{}", p.n),
            "pane_index" => p.index.to_string(),
            "pane_title" => p.title.clone(),
            "pane_active" => u8::from(p.active).to_string(),
            "session_name" | "window_name" => "cmux".into(),
            "session_id" => "$0".into(),
            "window_id" => "@0".into(),
            "window_index" => "0".into(),
            "window_active" => "1".into(),
            _ => String::new(),
        });
        rest = &rest[at + end + 1..];
    }
    out + rest
}

/// What `send-keys` arguments type: tmux key names become their bytes, the rest is text.
fn keys_text(keys: &[String], literal: bool) -> String {
    keys.iter()
        .map(|k| match k.as_str() {
            _ if literal => k.clone(),
            "Enter" | "KPEnter" => "\r".into(),
            "Tab" => "\t".into(),
            "Space" => " ".into(),
            "Escape" => "\x1b".into(),
            "BSpace" => "\x7f".into(),
            _ => match k.strip_prefix("C-").map(str::as_bytes) {
                // Ctrl with a letter or one of `@[\]^_`.
                Some([c]) if c.is_ascii_alphabetic() || (b'@'..=b'_').contains(c) => {
                    char::from(c.to_ascii_uppercase() & 0x1f).to_string()
                }
                _ => k.clone(),
            },
        })
        .collect()
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn surface_params(p: &Pane) -> Map<String, Value> {
    let mut params = Map::new();
    params.insert("surface_id".into(), p.id.as_str().into());
    params
}

/// `cmux __tmux-compat <tmux arguments>`: what the private `tmux` script runs (plan T3).
pub(crate) fn compat(cmd: &mut Cmd) -> Result<String, CliError> {
    let all = cmd.args.clone();
    let mut args = all.as_slice();
    // tmux's own options before the command.
    loop {
        match args.first().map(String::as_str) {
            Some("-V") => return Ok("tmux 3.4\n".into()),
            Some("-L" | "-S" | "-f") => args = args.get(2..).unwrap_or_default(),
            Some("-2" | "-u") => args = &args[1..],
            _ => break,
        }
    }
    let Some((name, rest)) = args.split_first() else {
        return fail("usage: tmux <command> [options]");
    };
    match name.as_str() {
        "split-window" | "splitw" => {
            let a = parse(rest, "cFlte", "PbdfhvZ")?;
            let panes = panes(cmd)?;
            let from = target(cmd, &panes, a.value('t'))?;
            let direction = match (a.has('h'), a.has('b')) {
                (true, false) => "right",
                (true, true) => "left",
                (false, false) => "down",
                (false, true) => "up",
            };
            let mut params = surface_params(from);
            params.insert("direction".into(), direction.into());
            // tmux focuses the new pane unless -d.
            params.insert("focus".into(), (!a.has('d')).into());
            let command = a.positional.join(" ");
            let input = match (a.value('c'), command.is_empty()) {
                (Some(dir), true) => format!("cd {}\n", quote(dir)),
                (Some(dir), false) => format!("cd {} && {command}\n", quote(dir)),
                (None, false) => format!("{command}\n"),
                (None, true) => String::new(),
            };
            if !input.is_empty() {
                params.insert("initial_input".into(), input.into());
            }
            let v = cmd.call("surface.split", params)?;
            if !a.has('P') {
                return Ok(String::new());
            }
            let new = Pane {
                n: v["surface_ref"]
                    .as_str()
                    .unwrap_or("")
                    .trim_start_matches("surface:")
                    .to_string(),
                id: v["surface_id"].as_str().unwrap_or("").to_string(),
                index: panes.len(),
                title: String::new(),
                active: !a.has('d'),
            };
            let format = a
                .value('F')
                .unwrap_or("#{session_name}:#{window_index}.#{pane_index}");
            Ok(render(format, &new) + "\n")
        }
        "send-keys" | "send" => {
            let a = parse(rest, "tN", "lRMXH")?;
            let panes = panes(cmd)?;
            let mut params = surface_params(target(cmd, &panes, a.value('t'))?);
            let text = keys_text(&a.positional, a.has('l'));
            if !text.is_empty() {
                params.insert("text".into(), text.into());
                cmd.call("surface.send_text", params)?;
            }
            Ok(String::new())
        }
        "kill-pane" | "killp" => {
            let a = parse(rest, "t", "")?;
            let panes = panes(cmd)?;
            let params = surface_params(target(cmd, &panes, a.value('t'))?);
            cmd.call("surface.close", params)?;
            Ok(String::new())
        }
        "list-panes" | "lsp" => {
            let a = parse(rest, "tFf", "as")?;
            let format = a.value('F').unwrap_or("#{pane_index}: #{pane_id}");
            let panes = panes(cmd)?;
            Ok(panes.iter().map(|p| render(format, p) + "\n").collect())
        }
        "display-message" | "display" | "displayp" => {
            let a = parse(rest, "tFc", "pv")?;
            let panes = panes(cmd)?;
            let pane = target(cmd, &panes, a.value('t'))?;
            let format = match a.positional.is_empty() {
                true => a.value('F').unwrap_or("").to_string(),
                false => a.positional.join(" "),
            };
            // Without -p tmux shows it in its status line; we have none.
            Ok(match a.has('p') {
                true => render(&format, pane) + "\n",
                false => String::new(),
            })
        }
        "select-pane" | "selectp" => {
            let a = parse(rest, "tTP", "DLRUdelMmZ")?;
            // Titles and border styles: tmux decoration, nothing to do.
            if a.value('T').is_some() || a.value('P').is_some() {
                return Ok(String::new());
            }
            let panes = panes(cmd)?;
            let params = surface_params(target(cmd, &panes, a.value('t'))?);
            cmd.call("surface.focus", params)?;
            Ok(String::new())
        }
        // Layout, options, hooks and names: accepted, nothing to do (plan T3).
        "has-session" | "has" | "select-layout" | "selectl" | "resize-pane" | "resizep"
        | "set-option" | "set" | "set-window-option" | "setw" | "rename-window" | "renamew"
        | "refresh-client" | "set-hook" | "bind-key" | "unbind-key" | "source-file" => {
            Ok(String::new())
        }
        other => fail(format!("Unsupported tmux compatibility command: {other}")),
    }
}

/// cmux's nudge (`claudeTeamsTeamSpawnGuidance`): only a named teammate gets a pane.
const GUIDANCE: &str = "You are Claude Code running inside cmux, started with `cmux claude-teams`. Agent teams are enabled and every NAMED teammate opens in its own split pane. When the user asks you to start a team, demo teams, or run several subagents/teammates in parallel, spawn them as named teammates: make one Agent tool call per teammate (that tool is named Task on pre-2.x CLIs), each with a distinct `name` (a short role), all in a single message so they run concurrently in their own split panes. Prefer named teammates over in-process subagents for any team or parallel-agent request. If the user asks for an open-ended demo such as \"make a demo team with 5 subagents\" without naming a topic, do not ask which feature — pick that many sensible roles and spawn them right away.";

/// Claude's arguments: teammate mode `auto` and the nudge, unless the user chose their own.
fn teams_args(args: &[String]) -> Vec<String> {
    let has = |names: &[&str]| {
        args.iter().any(|a| {
            names
                .iter()
                .any(|n| a == n || a.strip_prefix(n).is_some_and(|r| r.starts_with('=')))
        })
    };
    let mut out = Vec::new();
    if !has(&["--teammate-mode"]) {
        out.extend(["--teammate-mode".to_string(), "auto".to_string()]);
    }
    let prompts = [
        "--system-prompt",
        "--system-prompt-file",
        "--append-system-prompt",
        "--append-system-prompt-file",
    ];
    if !has(&prompts) {
        out.extend(["--append-system-prompt".to_string(), GUIDANCE.to_string()]);
    }
    out.extend(args.iter().cloned());
    out
}

/// The environment that makes Claude's auto mode take us for tmux (cmux
/// `configureTmuxCompatEnvironment`). `pane` is the caller's N.
fn teams_env(
    shim: &Path,
    path: Option<&str>,
    workspace: &str,
    pane: &str,
) -> Vec<(String, String)> {
    let path = match path {
        Some(p) => format!("{}:{p}", shim.display()),
        None => shim.display().to_string(),
    };
    [
        ("PATH", path),
        (
            "TMUX",
            format!("/tmp/cmux-claude-teams/{workspace},0,{pane}"),
        ),
        ("TMUX_PANE", format!("%{pane}")),
        ("TERM", "screen-256color".to_string()),
        ("CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS", "1".to_string()),
    ]
    .map(|(k, v)| (k.to_string(), v))
    .to_vec()
}

/// The private `tmux`: a script, so it works whatever `tmux` the user has or lacks.
fn write_shim(dir: &Path, exe: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let file = dir.join("tmux");
    let script = format!(
        "#!/bin/sh\nexec {} __tmux-compat \"$@\"\n",
        quote(&exe.to_string_lossy())
    );
    std::fs::write(&file, script)?;
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))
}

/// `cmux claude-teams [claude-args...]` (plan T1). Replaces this process with `claude`.
pub(crate) fn claude_teams(cmd: &mut Cmd) -> Result<String, CliError> {
    let linux = cmd.env("CMUX_LINUX").and_then(parse_bool) == Some(true);
    if !linux || cmd.env("CMUX_SURFACE_ID").is_none() {
        return fail("cmux claude-teams must be run in a cmux-linux terminal");
    }
    let Some(exe) = cmd.ctx.exe.clone() else {
        return fail("cannot find the cmux binary");
    };
    let panes = panes(cmd)?;
    let me = caller(cmd, &panes)?.clone();
    let env = |k: &str| cmd.ctx.env.get(k).cloned();
    let Some(shim) = paths::state_dir(&env).map(|d| d.join("claude-teams-bin")) else {
        return fail("cannot find a place for the tmux shim: set HOME");
    };
    write_shim(&shim, &exe).map_err(|e| format!("cannot write {}: {e}", shim.display()))?;
    let workspace = cmd.env("CMUX_WORKSPACE_ID").unwrap_or("default");
    let vars = teams_env(&shim, cmd.env("PATH"), workspace, &me.n);
    let e = std::process::Command::new("claude")
        .args(teams_args(&cmd.args))
        .envs(vars)
        .env_remove("TERM_PROGRAM")
        .exec();
    fail(format!("cannot run claude: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn parse_reads_clusters_values_and_stops_at_the_command() {
        let p = parse(
            &s(&["-dP", "-F#{pane_id}", "-t", "%3", "claude", "-p"]),
            "Ft",
            "dPh",
        )
        .unwrap();
        assert!(p.has('d') && p.has('P') && !p.has('h'));
        assert_eq!(
            (p.value('F'), p.value('t')),
            (Some("#{pane_id}"), Some("%3"))
        );
        assert_eq!(p.positional, s(&["claude", "-p"]));
        let dashes = parse(&s(&["-h", "--", "-x"]), "", "h").unwrap();
        assert_eq!(dashes.positional, s(&["-x"]));
        assert!(parse(&s(&["-q"]), "", "h").is_err());
        assert!(parse(&s(&["-t"]), "t", "").is_err());
    }

    #[test]
    fn render_fills_known_names_and_drops_the_rest() {
        let p = Pane {
            n: "7".into(),
            id: "X".into(),
            index: 2,
            title: "zsh".into(),
            active: true,
        };
        assert_eq!(
            render(
                "#{session_name}:#{window_index}.#{pane_index} #{pane_id} #{pane_active}#{nope}!",
                &p
            ),
            "cmux:0.2 %7 1!"
        );
        assert_eq!(render("plain #{open", &p), "plain #{open");
    }

    #[test]
    fn keys_become_bytes_unless_literal() {
        let keys = s(&["echo hi", "Enter", "C-c", "C-[", "Tab", "Space", "C-xyz"]);
        assert_eq!(keys_text(&keys, false), "echo hi\r\x03\x1b\t C-xyz");
        assert_eq!(keys_text(&s(&["Enter", "C-c"]), true), "EnterC-c");
    }

    #[test]
    fn teams_args_default_the_mode_and_nudge() {
        let a = teams_args(&s(&["--continue"]));
        assert_eq!(
            a[..3],
            s(&["--teammate-mode", "auto", "--append-system-prompt"])
        );
        assert_eq!(a.last().unwrap(), "--continue");
        assert_eq!(a.len(), 5);
        let own = s(&["--teammate-mode=tmux", "--system-prompt-file", "p.md"]);
        assert_eq!(teams_args(&own), own);
    }

    #[test]
    fn teams_env_puts_the_shim_first() {
        let env: HashMap<_, _> = teams_env(Path::new("/s/bin"), Some("/usr/bin"), "W", "4")
            .into_iter()
            .collect();
        assert_eq!(env["PATH"], "/s/bin:/usr/bin");
        assert_eq!(env["TMUX"], "/tmp/cmux-claude-teams/W,0,4");
        assert_eq!(env["TMUX_PANE"], "%4");
        assert_eq!(env["CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS"], "1");
    }

    #[test]
    fn shim_is_an_executable_script_that_calls_us() {
        let dir = std::env::temp_dir().join(format!("cmux-shim-{}", std::process::id()));
        write_shim(&dir, Path::new("/opt/it's/cmux")).unwrap();
        let file = dir.join("tmux");
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "#!/bin/sh\nexec '/opt/it'\\''s/cmux' __tmux-compat \"$@\"\n"
        );
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o755
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
