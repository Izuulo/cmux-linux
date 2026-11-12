//! Agent session resume (M10 Part 2): `[[vault.agents]]` in `config.toml` (plan V2), how a
//! running agent is recognised, and the line that resumes it (V3). cmux `docs/vault.md`, with
//! only the `argvOption` session id source.

use std::path::Path;

/// One custom agent: a foreground process named `process_name` whose argv carries the session
/// id after `argv_option` is resumed with `resume_command`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    pub id: String,
    pub name: String,
    pub process_name: String,
    pub argv_option: String,
    /// Placeholders `{{sessionId}}` (required), `{{executable}}`, `{{cwd}}`.
    pub resume_command: String,
}

/// Claude Code, reported by its hooks rather than found by process.
pub const CLAUDE: &str = "claude";

/// Session ids are typed into a shell: letters, digits, `.`, `_`, `-` only (plan V3).
pub fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// The registrations and a warning for each entry left out.
pub fn registrations(text: &str) -> (Vec<Registration>, Vec<String>) {
    let (mut regs, mut warnings) = (Vec::new(), Vec::new());
    let Ok(table) = text.parse::<toml::Table>() else {
        return (regs, warnings);
    };
    let Some(agents) = table
        .get("vault")
        .and_then(|v| v.get("agents"))
        .and_then(toml::Value::as_array)
    else {
        return (regs, warnings);
    };
    for (i, a) in agents.iter().enumerate() {
        let s = |k: &str| {
            a.get(k)
                .and_then(toml::Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        };
        let (Some(id), Some(process_name), Some(argv_option), Some(resume_command)) = (
            s("id"),
            s("process_name"),
            s("argv_option"),
            s("resume_command"),
        ) else {
            warnings.push(format!(
                "vault.agents[{i}]: needs id, process_name, argv_option and resume_command"
            ));
            continue;
        };
        if !resume_command.contains("{{sessionId}}") {
            warnings.push(format!(
                "vault.agents[{i}] ({id}): resume_command must include {{{{sessionId}}}}"
            ));
            continue;
        }
        if id == CLAUDE || regs.iter().any(|r: &Registration| r.id == id) {
            warnings.push(format!("vault.agents[{i}]: id `{id}` is taken"));
            continue;
        }
        regs.push(Registration {
            name: s("name").unwrap_or_else(|| id.clone()),
            id,
            process_name,
            argv_option,
            resume_command,
        });
    }
    (regs, warnings)
}

/// Every `[[vault.agents]]` entry as written, unfinished ones included (for the Settings
/// window, which must not drop what it can't use yet). A missing name is the id. Entries
/// without an id are left out, as `write::set_agents` leaves them alone.
pub fn entries_as_written(text: &str) -> Vec<Registration> {
    let Ok(table) = text.parse::<toml::Table>() else {
        return Vec::new();
    };
    let Some(agents) = table
        .get("vault")
        .and_then(|v| v.get("agents"))
        .and_then(toml::Value::as_array)
    else {
        return Vec::new();
    };
    agents
        .iter()
        .filter(|a| {
            a.get("id")
                .and_then(toml::Value::as_str)
                .is_some_and(|id| !id.trim().is_empty())
        })
        .map(|a| {
            let s = |k: &str| {
                a.get(k)
                    .and_then(toml::Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            };
            let id = s("id").trim().to_string();
            let name = a.get("name").map_or_else(|| id.clone(), |_| s("name"));
            Registration {
                id,
                name,
                process_name: s("process_name"),
                argv_option: s("argv_option"),
                resume_command: s("resume_command"),
            }
        })
        .collect()
}

pub fn load(path: &Path) -> (Vec<Registration>, Vec<String>) {
    registrations(&std::fs::read_to_string(path).unwrap_or_default())
}

/// The registration a process matches and its session id: one of `names` (the process's own
/// name, its executable's) or argv[0]'s file name is `process_name`, ignoring case, as cmux's
/// `executableBasenames`; and argv has `<argv_option> <id>` or `<argv_option>=<id>`.
pub fn detect<'a>(
    regs: &'a [Registration],
    names: &[&str],
    argv: &[String],
) -> Option<(&'a Registration, String)> {
    let base = |p: &str| Path::new(p).file_name()?.to_str().map(str::to_string);
    let names: Vec<String> = names
        .iter()
        .filter_map(|n| base(n))
        .chain(argv.first().and_then(|a| base(a)))
        .collect();
    let named = |r: &&Registration| {
        names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&r.process_name))
    };
    regs.iter().filter(named).find_map(|r| {
        let eq = format!("{}=", r.argv_option);
        let id = argv.iter().enumerate().skip(1).find_map(|(i, a)| {
            if *a == r.argv_option {
                argv.get(i + 1).cloned()
            } else {
                a.strip_prefix(&eq).map(str::to_string)
            }
        })?;
        valid_session_id(&id).then_some((r, id))
    })
}

/// Single-quoted for `sh`, bash, zsh and fish.
fn quote(s: &str) -> String {
    if !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-+:@".contains(&b))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The line typed into the restored shell, without the newline. `None` for an unknown agent
/// or an invalid id.
pub fn resume_line(
    regs: &[Registration],
    kind: &str,
    session_id: &str,
    executable: Option<&str>,
    cwd: Option<&str>,
) -> Option<String> {
    if !valid_session_id(session_id) {
        return None;
    }
    if kind == CLAUDE {
        return Some(format!("claude --resume {session_id}"));
    }
    let r = regs.iter().find(|r| r.id == kind)?;
    Some(
        r.resume_command
            .replace("{{sessionId}}", session_id)
            .replace(
                "{{executable}}",
                &quote(executable.unwrap_or(&r.process_name)),
            )
            .replace("{{cwd}}", &quote(cwd.unwrap_or("."))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: &str = r#"
[[vault.agents]]
id = "pi"
name = "Pi"
process_name = "pi"
argv_option = "--session"
resume_command = "{{executable}} --session {{sessionId}}"

[[vault.agents]]
id = "broken"
process_name = "x"
argv_option = "-s"
resume_command = "x --continue"

[[vault.agents]]
id = "claude"
process_name = "c"
argv_option = "-s"
resume_command = "c {{sessionId}}"
"#;

    fn argv(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn entries_as_written_keeps_unfinished_ones() {
        let all = entries_as_written(&format!("{CFG}\n[[vault.agents]]\nname = \"no id\"\n"));
        let ids: Vec<&str> = all.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["pi", "broken", "claude"]);
        assert_eq!(all[0].name, "Pi");
        assert_eq!(all[1].name, "broken");
        assert_eq!(all[1].resume_command, "x --continue");
        assert!(entries_as_written("not toml [").is_empty());
    }

    #[test]
    fn parses_and_warns() {
        let (regs, warnings) = registrations(CFG);
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].name, "Pi");
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("{{sessionId}}"), "{warnings:?}");
        assert_eq!(registrations("not toml [").0, vec![]);
    }

    #[test]
    fn detects_by_name_and_option() {
        let (regs, _) = registrations(CFG);
        let got = detect(&regs, &[], &argv(&["/usr/bin/pi", "--session", "abc-1"]));
        assert_eq!(
            got.map(|(r, id)| (r.id.as_str(), id)),
            Some(("pi", "abc-1".into()))
        );
        let eq = detect(&regs, &[], &argv(&["PI", "--session=x2"]));
        assert_eq!(eq.map(|(_, id)| id), Some("x2".into()));
        // A node script that set its process title: argv[0] is node, the name is pi.
        let node = argv(&["node", "/lib/pi/cli.js", "--session", "n1"]);
        assert!(detect(&regs, &[], &node).is_none());
        assert_eq!(
            detect(&regs, &["pi"], &node).map(|(_, id)| id),
            Some("n1".into())
        );
        assert!(detect(&regs, &[], &argv(&["pip", "--session", "a"])).is_none());
        assert!(detect(&regs, &[], &argv(&["pi", "--session"])).is_none());
        assert!(detect(&regs, &[], &argv(&["pi", "--session", "a;rm"])).is_none());
    }

    #[test]
    fn resume_lines() {
        let (regs, _) = registrations(CFG);
        assert_eq!(
            resume_line(&regs, CLAUDE, "5f1c-aa", None, None).as_deref(),
            Some("claude --resume 5f1c-aa")
        );
        assert_eq!(
            resume_line(&regs, "pi", "s1", Some("/opt/my pi/pi"), None).as_deref(),
            Some("'/opt/my pi/pi' --session s1")
        );
        assert_eq!(resume_line(&regs, CLAUDE, "$(x)", None, None), None);
        assert_eq!(resume_line(&regs, "gone", "s1", None, None), None);
    }
}
