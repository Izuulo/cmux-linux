//! Automations (M9 plan Part 3, cmux `docs/automations.md`): rules from a versioned JSON file,
//! matched against the app's events. Pure: the app emits events and carries out the actions.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

const RUN_TIMEOUT: u64 = 60;
const RUN_TIMEOUT_MAX: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Notify {
        title: String,
        subtitle: String,
        body: String,
    },
    Run {
        command: String,
        timeout: Duration,
    },
}

#[derive(Debug)]
pub struct Rule {
    pub id: String,
    pub enabled: bool,
    /// `when.event` and `when.category` patterns; an empty list matches everything.
    events: Vec<String>,
    categories: Vec<String>,
    conditions: Map<String, Value>,
    interval: Duration,
    maximum: usize,
    actions: Vec<Action>,
    fired: VecDeque<Instant>,
    pub firings: u64,
}

/// One rule's actions for one event, templates filled in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Firing {
    pub rule: String,
    pub actions: Vec<Action>,
}

#[derive(Debug, Default)]
pub struct Rules {
    pub rules: Vec<Rule>,
    /// Why a rule (or the file) was left out.
    pub warnings: Vec<String>,
}

/// An event as rules and `run` commands see it (cmux `docs/events.md`, without the stream's
/// sequence fields). The category is the name's first segment.
pub fn event(name: &str, payload: Map<String, Value>) -> Value {
    json!({
        "type": "event", "name": name,
        "category": name.split('.').next().unwrap_or(name),
        "source": "cmux-linux",
        "workspace_id": payload.get("workspace_id"),
        "surface_id": payload.get("surface_id"),
        "payload": payload,
    })
}

fn strings(v: Option<&Value>) -> Result<Vec<String>, String> {
    match v {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(s)) => Ok(vec![s.clone()]),
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| {
                x.as_str()
                    .map(str::to_string)
                    .ok_or("expected strings".into())
            })
            .collect(),
        Some(_) => Err("expected a string or a list of strings".into()),
    }
}

fn action(v: &Value) -> Result<Action, String> {
    let text = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| v.get(k)?.as_str())
            .unwrap_or("")
            .to_string()
    };
    match v.get("action").and_then(Value::as_str) {
        Some("notify") => Ok(Action::Notify {
            title: text(&["title"]),
            subtitle: text(&["subtitle"]),
            body: text(&["body", "message"]),
        }),
        Some("run") => {
            let command = text(&["command", "cmd"]);
            if command.trim().is_empty() {
                return Err("run needs a command".into());
            }
            let secs = v.get("timeout_seconds").and_then(Value::as_u64);
            let secs = secs.unwrap_or(RUN_TIMEOUT).clamp(1, RUN_TIMEOUT_MAX);
            Ok(Action::Run {
                command,
                timeout: Duration::from_secs(secs),
            })
        }
        Some(other) => Err(format!("action `{other}` is not supported")),
        None => Err("an action has no `action`".into()),
    }
}

fn rule(v: &Value) -> Result<Rule, String> {
    let id = v.get("id").and_then(Value::as_str).unwrap_or("").trim();
    if id.is_empty() {
        return Err("a rule has no id".into());
    }
    let fail = |e: String| format!("rule `{id}`: {e}");
    let when = v.get("when").unwrap_or(&Value::Null);
    let events = strings(when.get("event")).map_err(fail)?;
    let categories = strings(when.get("category")).map_err(fail)?;
    if events.is_empty() && categories.is_empty() {
        return Err(fail("`when` names no event or category".into()));
    }
    let conditions = match v.get("where") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(m)) => m.clone(),
        Some(_) => return Err(fail("`where` is not an object".into())),
    };
    let limit = v.get("rate_limit").unwrap_or(&Value::Null);
    let interval = limit.get("interval_seconds").and_then(Value::as_f64);
    let maximum = limit.get("maximum").and_then(Value::as_u64).unwrap_or(1);
    let actions = match v.get("then").and_then(Value::as_array) {
        Some(a) if !a.is_empty() => a.iter().map(action).collect::<Result<Vec<_>, _>>(),
        _ => Err("`then` has no actions".into()),
    }
    .map_err(fail)?;
    Ok(Rule {
        id: id.to_string(),
        enabled: v.get("enabled").and_then(Value::as_bool) != Some(false),
        events,
        categories,
        conditions,
        // cmux's default: one firing a second.
        interval: Duration::from_secs_f64(interval.filter(|s| *s > 0.0).unwrap_or(1.0)),
        maximum: maximum.max(1) as usize,
        actions,
        fired: VecDeque::new(),
        firings: 0,
    })
}

/// The rules of an `automations.json`. A bad rule is left out with a warning; a file that is
/// not version 1 loads nothing.
pub fn parse(text: &str) -> Rules {
    let mut out = Rules::default();
    let v: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            out.warnings.push(format!("not valid JSON: {e}"));
            return out;
        }
    };
    if v.get("version").and_then(Value::as_u64) != Some(1) {
        out.warnings.push("`version` must be 1".into());
        return out;
    }
    for r in v
        .get("rules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match rule(r) {
            Ok(r) if out.rules.iter().any(|x| x.id == r.id) => {
                out.warnings.push(format!("rule `{}`: id used twice", r.id));
            }
            Ok(r) => out.rules.push(r),
            Err(e) => out.warnings.push(e),
        }
    }
    out
}

/// Exact, or a `*` at either end (or both) as a wildcard.
fn pattern_matches(pattern: &str, name: &str) -> bool {
    match (pattern.strip_prefix('*'), pattern.strip_suffix('*')) {
        _ if pattern == "*" => true,
        (Some(p), Some(_)) => name.contains(&p[..p.len() - 1]),
        (Some(suffix), None) => name.ends_with(suffix),
        (None, Some(prefix)) => name.starts_with(prefix),
        (None, None) => pattern == name,
    }
}

fn plain(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(_) | Value::Bool(_) => Some(v.to_string()),
        _ => None,
    }
}

/// The payload value a `where` key means (cmux's aliases, then the key itself).
fn lookup(payload: &Value, key: &str) -> Option<String> {
    let keys: &[&str] = match key {
        "workspace.title" | "title" => &["title", "custom_title", "workspace_title"],
        "agent" => &["agent", "_source", "source"],
        "surface.kind" | "surface_kind" => &["kind", "type", "surface_kind"],
        _ => return plain(payload.get(key)?),
    };
    keys.iter().find_map(|k| plain(payload.get(k)?))
}

/// A missing value matches nothing (so only `not` holds for it). Comparisons ignore case.
fn holds(predicate: &Value, value: Option<&str>) -> bool {
    let value = value.map(str::to_lowercase);
    let same = |want: &Value| plain(want).map(|w| w.to_lowercase()) == value;
    let Value::Object(ops) = predicate else {
        return same(predicate);
    };
    ops.iter().all(|(op, want)| {
        let text = |f: fn(&str, &str) -> bool| match (plain(want), &value) {
            (Some(w), Some(v)) => f(v, &w.to_lowercase()),
            _ => false,
        };
        match op.as_str() {
            "equals" => same(want),
            "contains" => text(|v, w| v.contains(w)),
            "prefix" => text(|v, w| v.starts_with(w)),
            "suffix" => text(|v, w| v.ends_with(w)),
            "in" => want.as_array().is_some_and(|a| a.iter().any(same)),
            "not" => !holds(want, value.as_deref()),
            _ => false,
        }
    })
}

/// `{{event.name}}`, `{{event.category}}`, `{{event.source}}` and `{{payload.<key>}}`.
fn render(template: &str, event: &Value) -> String {
    let mut out = template.to_string();
    for key in ["name", "category", "source"] {
        let v = event.get(key).and_then(Value::as_str).unwrap_or("");
        out = out.replace(&format!("{{{{event.{key}}}}}"), v);
    }
    for (key, v) in event
        .get("payload")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if let Some(v) = plain(v) {
            out = out.replace(&format!("{{{{payload.{key}}}}}"), &v);
        }
    }
    out
}

impl Rule {
    fn matches(&self, event: &Value) -> bool {
        let field = |k| event.get(k).and_then(Value::as_str).unwrap_or("");
        let any = |patterns: &[String], name: &str| {
            patterns.is_empty() || patterns.iter().any(|p| pattern_matches(p, name))
        };
        let payload = event.get("payload").unwrap_or(&Value::Null);
        self.enabled
            && any(&self.events, field("name"))
            && any(&self.categories, field("category"))
            && self
                .conditions
                .iter()
                .all(|(key, p)| holds(p, lookup(payload, key).as_deref()))
    }

    /// At most `maximum` firings in any `interval`.
    fn admit(&mut self, now: Instant) -> bool {
        while self
            .fired
            .front()
            .is_some_and(|t| now.duration_since(*t) >= self.interval)
        {
            self.fired.pop_front();
        }
        if self.fired.len() >= self.maximum {
            return false;
        }
        self.fired.push_back(now);
        self.firings += 1;
        true
    }

    pub fn summary(&self) -> Value {
        json!({
            "id": self.id, "enabled": self.enabled,
            "events": self.events, "categories": self.categories,
            "actions": self.actions.len(), "firings": self.firings,
        })
    }
}

impl Rules {
    /// The rules that fire on `event`, in file order, each with its actions filled in.
    pub fn fire(&mut self, event: &Value, now: Instant) -> Vec<Firing> {
        let mut out = Vec::new();
        for r in &mut self.rules {
            if !r.matches(event) || !r.admit(now) {
                continue;
            }
            let actions = r.actions.iter().map(|a| match a {
                Action::Notify {
                    title,
                    subtitle,
                    body,
                } => Action::Notify {
                    title: render(title, event),
                    subtitle: render(subtitle, event),
                    body: render(body, event),
                },
                run => run.clone(),
            });
            out.push(Firing {
                rule: r.id.clone(),
                actions: actions.collect(),
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(name: &str, payload: Value) -> Value {
        event(name, payload.as_object().cloned().unwrap())
    }

    const FILE: &str = r#"{"version": 1, "rules": [
        {"id": "input", "when": {"event": "notification.created"},
         "where": {"workspace.title": "Dispatch", "body": {"contains": "INPUT", "not": {"suffix": "!"}}},
         "rate_limit": {"interval_seconds": 30, "maximum": 2},
         "then": [
            {"action": "notify", "title": "{{event.name}} in {{payload.workspace_title}}", "body": "{{payload.nope}} {{event.category}}"},
            {"action": "run", "cmd": "true", "timeout_seconds": 900}]},
        {"id": "off", "enabled": false, "when": {"category": "*"}, "then": [{"action": "run", "command": "x"}]},
        {"id": "ws", "when": {"event": ["workspace.*", "*.Stop"]}, "then": [{"action": "run", "command": "x"}]},
        {"id": "web", "when": {"event": "a"}, "then": [{"action": "run", "command": "x"}, {"action": "webhook", "url": "u"}]},
        {"id": "ws", "when": {"event": "a"}, "then": [{"action": "run", "command": "x"}]},
        {"id": "none", "when": {}, "then": [{"action": "run", "command": "x"}]},
        {"when": {"event": "a"}, "then": []}
    ]}"#;

    #[test]
    fn parse_keeps_good_rules_and_explains_the_rest() {
        let r = parse(FILE);
        let ids: Vec<_> = r.rules.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["input", "off", "ws"]);
        assert_eq!(
            r.warnings,
            vec![
                "rule `web`: action `webhook` is not supported",
                "rule `ws`: id used twice",
                "rule `none`: `when` names no event or category",
                "a rule has no id",
            ]
        );
        assert_eq!(parse("{").rules.len(), 0);
        assert_eq!(
            parse(r#"{"version": 2, "rules": []}"#).warnings,
            vec!["`version` must be 1"]
        );
        assert!(parse(r#"{"version": 1}"#).warnings.is_empty());
    }

    #[test]
    fn fires_on_match_with_templates_filled() {
        let mut r = parse(FILE);
        let now = Instant::now();
        let hit = ev(
            "notification.created",
            json!({"workspace_title": "dispatch", "body": "Needs input now"}),
        );
        let fired = r.fire(&hit, now);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].rule, "input");
        assert_eq!(
            fired[0].actions,
            vec![
                Action::Notify {
                    title: "notification.created in dispatch".into(),
                    subtitle: String::new(),
                    body: "{{payload.nope}} notification".into(),
                },
                Action::Run {
                    command: "true".into(),
                    timeout: Duration::from_secs(300)
                },
            ]
        );
        for (name, payload) in [
            (
                "notification.read",
                json!({"workspace_title": "dispatch", "body": "input"}),
            ),
            (
                "notification.created",
                json!({"workspace_title": "other", "body": "input"}),
            ),
            (
                "notification.created",
                json!({"workspace_title": "dispatch", "body": "input!"}),
            ),
            (
                "notification.created",
                json!({"workspace_title": "dispatch"}),
            ),
        ] {
            assert_eq!(r.fire(&ev(name, payload), now), vec![], "{name}");
        }
    }

    #[test]
    fn rate_limit_is_a_sliding_window() {
        let mut r = parse(FILE);
        let hit = ev(
            "notification.created",
            json!({"title": "Dispatch", "body": "input"}),
        );
        let t = Instant::now();
        assert_eq!(r.fire(&hit, t).len(), 1);
        assert_eq!(r.fire(&hit, t + Duration::from_secs(1)).len(), 1);
        assert_eq!(
            r.fire(&hit, t + Duration::from_secs(29)).len(),
            0,
            "two in 30 s"
        );
        assert_eq!(r.fire(&hit, t + Duration::from_secs(30)).len(), 1);
        assert_eq!(r.rules[0].firings, 3);
        // Default: one a second.
        let stop = ev("agent.hook.Stop", json!({}));
        assert_eq!(r.fire(&stop, t).len(), 1);
        assert_eq!(r.fire(&ev("workspace.closed", json!({})), t).len(), 0);
        assert_eq!(r.fire(&stop, t + Duration::from_secs(1)).len(), 1);
    }

    #[test]
    fn patterns_and_predicates() {
        assert!(pattern_matches("*", "a.b") && pattern_matches("a.*", "a.b"));
        assert!(pattern_matches("*.b", "a.b") && pattern_matches("*.b*", "a.bc"));
        assert!(!pattern_matches("a", "a.b") && !pattern_matches("b.*", "a.b"));
        assert!(holds(&json!("ABC"), Some("abc")) && holds(&json!(3), Some("3")));
        assert!(holds(&json!({"in": ["x", "Abc"]}), Some("abc")));
        assert!(holds(&json!({"prefix": "a", "suffix": "C"}), Some("abc")));
        assert!(!holds(&json!({"equals": "abc", "bogus": 1}), Some("abc")));
        assert!(!holds(&json!("abc"), None) && holds(&json!({"not": "abc"}), None));
        let p = json!({"type": "terminal", "source": "claude", "n": 2});
        assert_eq!(lookup(&p, "surface.kind").as_deref(), Some("terminal"));
        assert_eq!(lookup(&p, "agent").as_deref(), Some("claude"));
        assert_eq!(lookup(&p, "n").as_deref(), Some("2"));
        assert_eq!(lookup(&p, "workspace.tag"), None);
    }
}
