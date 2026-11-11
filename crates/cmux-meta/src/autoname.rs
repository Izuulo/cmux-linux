//! Workspace auto-naming (M9 plan Part 2): cmux's throttle, transcript excerpt, prompt and title
//! clean-up (`CLI/CMUXCLI+AutoNaming.swift`), and the pass that runs the user's command.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use crate::pr::run_with_deadline;

/// cmux's `AutoNamingConfig` defaults.
const MIN_TRANSCRIPT_LINES: usize = 12;
const MIN_LINE_GROWTH: usize = 6;
pub const MIN_INTERVAL: Duration = Duration::from_secs(180);
const DEADLINE: Duration = Duration::from_secs(60);
const MAX_TITLE: usize = 50;
const HEAD_USER_MESSAGES: usize = 2;
const TAIL_MESSAGES: usize = 4;
const MESSAGE_CHARS: usize = 240;

/// What the last passes left behind for one workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NamingState {
    /// Transcript lines at the last naming.
    pub named_lines: Option<usize>,
    /// Time since the last attempt, successful or not.
    pub since_attempt: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Throttle {
    Proceed,
    /// The transcript shrank (compaction): count growth from here, name nothing.
    Reseed,
    Skip,
}

pub fn throttle(state: NamingState, lines: usize) -> Throttle {
    if lines < MIN_TRANSCRIPT_LINES {
        return Throttle::Skip;
    }
    if state.named_lines.is_some_and(|n| lines < n) {
        return Throttle::Reseed;
    }
    // A failing command backs off too: the cooldown counts attempts, not names.
    if state.since_attempt.is_some_and(|d| d < MIN_INTERVAL) {
        return Throttle::Skip;
    }
    match state.named_lines {
        Some(n) if lines - n < MIN_LINE_GROWTH => Throttle::Skip,
        _ => Throttle::Proceed,
    }
}

/// `(role, text)` of each user and assistant text message in a Claude Code transcript; tool
/// results, thinking and unparseable lines are skipped.
fn messages(transcript: &str) -> Vec<(String, String)> {
    transcript
        .lines()
        .filter_map(|line| {
            let v: Value = serde_json::from_str(line).ok()?;
            let role = v.get("type")?.as_str()?;
            if role != "user" && role != "assistant" {
                return None;
            }
            let text = match v.get("message")?.get("content")? {
                Value::String(s) => s.clone(),
                Value::Array(items) => items
                    .iter()
                    .filter(|i| i.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|i| i.get("text")?.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => return None,
            };
            let text = text.trim();
            (!text.is_empty()).then(|| (role.to_string(), text.to_string()))
        })
        .collect()
}

/// The first user messages say what the session is for, the last messages what it is on now.
fn context(messages: &[(String, String)]) -> Option<String> {
    let head = messages
        .iter()
        .filter(|(role, _)| role == "user")
        .take(HEAD_USER_MESSAGES);
    let tail = &messages[messages.len().saturating_sub(TAIL_MESSAGES)..];
    let mut parts: Vec<String> = Vec::new();
    for (role, text) in head.chain(tail) {
        let excerpt: String = text.chars().take(MESSAGE_CHARS).collect();
        let part = format!("{role}: {excerpt}");
        if !parts.contains(&part) {
            parts.push(part);
        }
    }
    (!parts.is_empty()).then(|| parts.join("\n"))
}

/// cmux's prompt, word for word.
fn prompt(current: Option<&str>, context: &str) -> String {
    let mut lines = vec![
        "You name terminal workspace tabs for a developer running coding agents.".to_string(),
        "Given a conversation excerpt, output ONLY a short title: 2-5 words,".to_string(),
        "in the same language as the conversation, no quotes, no trailing punctuation.".to_string(),
        String::new(),
    ];
    if let Some(t) = current.filter(|t| !t.is_empty()) {
        lines.push(format!("The current title is: {t}"));
        lines.push("If that still accurately describes the conversation's main topic, output it EXACTLY as given.".to_string());
        lines.push(String::new());
    }
    lines.push("Conversation excerpt:".to_string());
    lines.push(context.to_string());
    lines.join("\n")
}

/// The answer's first non-empty line without wrapping quotes, whitespace collapsed, cut to 50
/// characters at a word. `None` if nothing usable is left or it is the current title.
fn title(raw: &str, current: Option<&str>) -> Option<String> {
    let mut t = raw.lines().map(str::trim).find(|l| !l.is_empty())?;
    loop {
        let inner = [('"', '"'), ('\'', '\''), ('\u{201C}', '\u{201D}')]
            .iter()
            .find_map(|&(a, b)| t.strip_prefix(a)?.strip_suffix(b));
        match inner {
            Some(i) => t = i.trim(),
            None => break,
        }
    }
    let mut t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > MAX_TITLE {
        let cut: String = t.chars().take(MAX_TITLE).collect();
        t = match cut.rfind(' ') {
            Some(i) if i > 0 => cut[..i].to_string(),
            _ => cut,
        };
    }
    (!t.is_empty() && Some(t.as_str()) != current).then_some(t)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pass {
    /// Nothing was run.
    Skipped,
    /// Only the baseline moves.
    Reseed { lines: usize },
    /// The command ran. `title` is `None` when it failed or kept the current title.
    Ran { lines: usize, title: Option<String> },
}

/// One naming pass, for the background pool: reads the transcript, asks the throttle, runs
/// `command` under `sh -c` with the prompt on stdin. Every failure is a quiet "no new title".
pub fn pass(command: &str, transcript: &Path, state: NamingState, current: Option<&str>) -> Pass {
    let Ok(text) = std::fs::read_to_string(transcript) else {
        return Pass::Skipped;
    };
    let lines = text.lines().count();
    match throttle(state, lines) {
        Throttle::Skip => return Pass::Skipped,
        Throttle::Reseed => return Pass::Reseed { lines },
        Throttle::Proceed => {}
    }
    let Some(context) = context(&messages(&text)) else {
        return Pass::Skipped;
    };
    let title =
        run(command, &prompt(current, &context), DEADLINE).and_then(|out| title(&out, current));
    Pass::Ran { lines, title }
}

fn run(command: &str, prompt: &str, deadline: Duration) -> Option<String> {
    let mut cmd = Command::new("sh");
    // Away from any project, so the summarizer loads no project settings or session.
    cmd.arg("-c").arg(command).current_dir(std::env::temp_dir());
    // Without these its own Stop hook does nothing: no naming loop (plan N6).
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("CMUX_") {
            cmd.env_remove(key);
        }
    }
    // shortcut: on the deadline only `sh` is killed, a child it started runs on until it ends;
    // kill the process group if stuck summarizers are ever seen.
    let out = run_with_deadline(cmd, prompt.as_bytes().to_vec(), deadline).ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(named: Option<usize>, secs: Option<u64>) -> NamingState {
        NamingState {
            named_lines: named,
            since_attempt: secs.map(Duration::from_secs),
        }
    }

    #[test]
    fn throttle_follows_cmux() {
        assert_eq!(throttle(state(None, None), 11), Throttle::Skip, "short");
        assert_eq!(throttle(state(None, None), 12), Throttle::Proceed, "first");
        assert_eq!(
            throttle(state(None, Some(10)), 40),
            Throttle::Skip,
            "failed just now"
        );
        assert_eq!(throttle(state(None, Some(180)), 40), Throttle::Proceed);
        assert_eq!(throttle(state(Some(40), Some(999)), 30), Throttle::Reseed);
        assert_eq!(
            throttle(state(Some(40), Some(60)), 80),
            Throttle::Skip,
            "too soon"
        );
        assert_eq!(
            throttle(state(Some(40), Some(999)), 45),
            Throttle::Skip,
            "no growth"
        );
        assert_eq!(throttle(state(Some(40), Some(999)), 46), Throttle::Proceed);
    }

    const TRANSCRIPT: &str = r#"{"type":"user","message":{"content":"fix the login bug"}}
not json
{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"hm"},{"type":"text","text":" Looking. "}]}}
{"type":"user","message":{"content":[{"type":"tool_result","content":"ok"}]}}
{"type":"summary","summary":"x"}
{"type":"user","message":{"content":"now add a test"}}
{"type":"assistant","message":{"content":[{"type":"text","text":"a"},{"type":"text","text":"b"}]}}
"#;

    #[test]
    fn transcript_keeps_only_text_messages() {
        let m = messages(TRANSCRIPT);
        let pairs: Vec<_> = m.iter().map(|(r, t)| (r.as_str(), t.as_str())).collect();
        assert_eq!(
            pairs,
            vec![
                ("user", "fix the login bug"),
                ("assistant", "Looking."),
                ("user", "now add a test"),
                ("assistant", "a\nb"),
            ]
        );
        // Head and tail overlap here: each message appears once.
        assert_eq!(
            context(&m).unwrap(),
            "user: fix the login bug\nuser: now add a test\nassistant: Looking.\nassistant: a\nb"
        );
        assert_eq!(context(&[]), None);
        let long = vec![("user".to_string(), "é".repeat(500))];
        assert_eq!(
            context(&long).unwrap().chars().count(),
            "user: ".len() + 240
        );
    }

    #[test]
    fn prompt_names_the_current_title() {
        assert!(!prompt(None, "user: hi").contains("current title"));
        let p = prompt(Some("Login fix"), "user: hi");
        assert!(p.contains("The current title is: Login fix\n"));
        assert!(p.ends_with("Conversation excerpt:\nuser: hi"));
    }

    #[test]
    fn title_is_cleaned() {
        assert_eq!(
            title("\n  \"'Fix   login bug'\" \nmore", None).unwrap(),
            "Fix login bug"
        );
        assert_eq!(
            title("\u{201C}Añadir pruebas\u{201D}", None).unwrap(),
            "Añadir pruebas"
        );
        assert_eq!(title("Same", Some("Same")), None);
        assert_eq!(title(" \n ", None), None);
        assert_eq!(title("\"\"", None), None);
        let long = "word ".repeat(20);
        let t = title(&long, None).unwrap();
        assert_eq!(t, "word word word word word word word word word word");
        assert_eq!(title(&"x".repeat(80), None).unwrap().len(), 50);
    }

    fn transcript(name: &str, turns: usize) -> std::path::PathBuf {
        let path = crate::scratch(name).join("t.jsonl");
        let line = "{\"type\":\"user\",\"message\":{\"content\":\"fix the login bug\"}}\n";
        std::fs::write(&path, line.repeat(turns)).unwrap();
        path
    }

    #[test]
    fn pass_runs_the_command_with_the_prompt_on_stdin() {
        let t = transcript("autoname-run", 12);
        // The fake summarizer proves it saw the excerpt and no CMUX_ variable.
        let cmd = "grep -q 'user: fix the login bug' && echo \"\\\"Login bug ${CMUX_SURFACE_ID:-fix}\\\"\"";
        let got = pass(cmd, &t, NamingState::default(), None);
        assert_eq!(
            got,
            Pass::Ran {
                lines: 12,
                title: Some("Login bug fix".into())
            }
        );
        let same = pass(cmd, &t, NamingState::default(), Some("Login bug fix"));
        assert_eq!(
            same,
            Pass::Ran {
                lines: 12,
                title: None
            }
        );
        assert_eq!(
            pass("exit 3", &t, NamingState::default(), None),
            Pass::Ran {
                lines: 12,
                title: None
            }
        );
    }

    #[test]
    fn pass_skips_without_running() {
        let boom = "echo ran > ran; echo Title";
        let short = transcript("autoname-short", 5);
        assert_eq!(
            pass(boom, &short, NamingState::default(), None),
            Pass::Skipped
        );
        let t = transcript("autoname-reseed", 20);
        let got = pass(boom, &t, state(Some(30), None), None);
        assert_eq!(got, Pass::Reseed { lines: 20 });
        let missing = Path::new("/nonexistent/t.jsonl");
        assert_eq!(
            pass(boom, missing, NamingState::default(), None),
            Pass::Skipped
        );
        assert!(!std::env::temp_dir().join("ran").exists());
    }

    #[test]
    fn a_hanging_command_is_cut_off() {
        assert_eq!(run("exec sleep 5", "p", Duration::from_millis(200)), None);
    }
}
