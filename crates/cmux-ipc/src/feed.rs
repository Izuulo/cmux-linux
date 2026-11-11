//! Feed wire format: cmux's `feed.*` socket methods and Claude Code's hook stdin/stdout, mapped
//! to and from the `cmux_core::feed` model.

use std::time::Duration;

use cmux_core::{
    Decision, FeedKind, PermissionMode, PlanMode, Preview, QuestionOption, QuestionPrompt,
};
use serde_json::{Map, Value, json};

use crate::json::pretty;
use crate::wire::RpcError;

pub const WAIT_SECS: u64 = 115;
pub const MAX_WAIT_SECS: f64 = 120.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedPush {
    pub request_id: String,
    pub wait: Duration,
    pub source: String,
    pub workspace_id: String,
    pub surface_id: String,
    pub cwd: String,
    pub kind: FeedKind,
}

fn invalid(msg: String) -> RpcError {
    RpcError::new("invalid_params", msg)
}

pub fn wait_of(params: &Map<String, Value>) -> Result<Duration, RpcError> {
    let Some(v) = params.get("wait_timeout_seconds") else {
        return Ok(Duration::ZERO);
    };
    match v.as_f64() {
        Some(s) if (0.0..=MAX_WAIT_SECS).contains(&s) => Ok(Duration::from_secs_f64(s)),
        _ => Err(invalid(
            "feed.push wait_timeout_seconds must be between 0 and 120".into(),
        )),
    }
}

fn str_of<'a>(m: &'a Map<String, Value>, k: &str) -> Option<&'a str> {
    m.get(k).and_then(Value::as_str)
}

pub fn parse_push(params: &Map<String, Value>) -> Result<FeedPush, RpcError> {
    let wait = wait_of(params)?;
    let missing = |k: &str| invalid(format!("feed.push requires event.{k}"));
    let ev = params
        .get("event")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("feed.push requires event".into()))?;
    let need = |k: &str| {
        str_of(ev, k)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .ok_or_else(|| missing(k))
    };
    let request_id = need("_opencode_request_id")?;
    let workspace_id = need("workspace_id")?;
    let surface_id = need("surface_id")?;
    let tool = str_of(ev, "tool_name").unwrap_or("");
    let empty = Value::Object(Map::new());
    let input = ev.get("tool_input").unwrap_or(&empty);
    Ok(FeedPush {
        request_id,
        wait,
        source: str_of(ev, "_source").unwrap_or("").into(),
        workspace_id,
        surface_id,
        cwd: str_of(ev, "cwd").unwrap_or("").into(),
        kind: parse_kind(tool, input),
    })
}

fn parse_kind(tool: &str, input: &Value) -> FeedKind {
    let s = |k: &str| input.get(k).and_then(Value::as_str);
    match tool {
        "ExitPlanMode" => FeedKind::ExitPlan {
            plan: s("plan").unwrap_or("").into(),
            allowed_prompts: input
                .get("allowedPrompts")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|p| {
                    p.as_str().or_else(|| {
                        ["prompt", "description", "text"]
                            .iter()
                            .find_map(|k| p.get(k).and_then(Value::as_str))
                    })
                })
                .map(String::from)
                .collect(),
            plan_file: s("planFilePath").or(s("plan_file_path")).map(String::from),
        },
        "AskUserQuestion" => FeedKind::Question {
            prompts: input
                .get("questions")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|q| {
                    let t = |k: &str| q.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                    QuestionPrompt {
                        header: t("header"),
                        question: t("question"),
                        multi_select: q.get("multiSelect").and_then(Value::as_bool) == Some(true),
                        options: q
                            .get("options")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                            .map(|o| {
                                let t = |k: &str| {
                                    o.get(k).and_then(Value::as_str).unwrap_or("").to_string()
                                };
                                QuestionOption {
                                    label: t("label"),
                                    description: t("description"),
                                }
                            })
                            .collect(),
                    }
                })
                .collect(),
        },
        _ => FeedKind::Permission {
            tool: tool.into(),
            preview: match tool {
                "Bash" => Preview::Command {
                    command: s("command").unwrap_or("").into(),
                    description: s("description").map(String::from),
                },
                "Write" => Preview::File {
                    path: s("file_path").unwrap_or("").into(),
                    first_line: s("content").map(|c| c.lines().next().unwrap_or("").into()),
                },
                "Edit" | "MultiEdit" | "Read" => Preview::File {
                    path: s("file_path").unwrap_or("").into(),
                    first_line: None,
                },
                _ => Preview::Raw(pretty(input)),
            },
        },
    }
}

pub fn parse_reply(
    method: &str,
    params: &Map<String, Value>,
) -> Result<(String, Decision), RpcError> {
    let request_id = str_of(params, "request_id")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| invalid(format!("{method} requires request_id")))?
        .to_string();
    let mode = str_of(params, "mode");
    let feedback = str_of(params, "feedback")
        .filter(|f| !f.trim().is_empty())
        .map(String::from);
    let d = match method {
        "feed.question.reply" => {
            let sel = params
                .get("selections")
                .and_then(Value::as_array)
                .and_then(|a| a.iter().map(|v| v.as_str().map(String::from)).collect())
                .ok_or_else(|| invalid(format!("{method} requires selections: [string]")))?;
            Decision::Question { selections: sel }
        }
        "feed.exit_plan.reply" => {
            let mode = match mode {
                Some("ultraplan") => PlanMode::Ultraplan,
                Some("bypassPermissions") => PlanMode::BypassPermissions,
                Some("autoAccept") => PlanMode::AutoAccept,
                Some("manual") => PlanMode::Manual,
                Some("deny") => PlanMode::Deny,
                _ => {
                    return Err(invalid(format!(
                        "{method} requires mode ∈ ultraplan|bypassPermissions|autoAccept|manual|deny"
                    )));
                }
            };
            Decision::ExitPlan { mode, feedback }
        }
        _ => Decision::Permission(match mode {
            Some("once") => PermissionMode::Once,
            Some("always") => PermissionMode::Always,
            Some("all") => PermissionMode::All,
            Some("bypass") => PermissionMode::Bypass,
            Some("deny") => PermissionMode::Deny,
            _ => {
                return Err(invalid(format!(
                    "{method} requires mode ∈ once|always|all|bypass|deny"
                )));
            }
        }),
    };
    Ok((request_id, d))
}

pub fn decision_json(d: &Decision) -> Value {
    match d {
        Decision::Permission(m) => json!({"kind": "permission", "mode": match m {
            PermissionMode::Once => "once",
            PermissionMode::Always => "always",
            PermissionMode::All => "all",
            PermissionMode::Bypass => "bypass",
            PermissionMode::Deny => "deny",
        }}),
        Decision::ExitPlan { mode, feedback } => {
            let mut v = json!({"kind": "exit_plan", "mode": match mode {
                PlanMode::Ultraplan => "ultraplan",
                PlanMode::BypassPermissions => "bypassPermissions",
                PlanMode::AutoAccept => "autoAccept",
                PlanMode::Manual => "manual",
                PlanMode::Deny => "deny",
            }});
            if let Some(f) = feedback {
                v["feedback"] = json!(f);
            }
            v
        }
        Decision::Question { selections } => json!({"kind": "question", "selections": selections}),
    }
}

pub fn decision_from_json(v: &Value) -> Option<Decision> {
    let m = v.as_object()?;
    let method = match str_of(m, "kind")? {
        "permission" => "feed.permission.reply",
        "exit_plan" => "feed.exit_plan.reply",
        "question" => "feed.question.reply",
        _ => return None,
    };
    let mut p = m.clone();
    p.insert("request_id".into(), json!("-"));
    parse_reply(method, &p).ok().map(|(_, d)| d)
}

pub fn push_params(
    stdin: &Value,
    workspace_id: &str,
    surface_id: &str,
    request_id: &str,
) -> Map<String, Value> {
    let tool = stdin.get("tool_name").and_then(Value::as_str);
    let event = match tool {
        Some("ExitPlanMode") => "ExitPlanMode",
        Some("AskUserQuestion") => "AskUserQuestion",
        _ => "PermissionRequest",
    };
    let mut ev = Map::new();
    ev.insert("hook_event_name".into(), json!(event));
    for k in ["session_id", "cwd", "tool_name", "tool_input"] {
        if let Some(v) = stdin.get(k) {
            ev.insert(k.into(), v.clone());
        }
    }
    ev.insert("_source".into(), json!("claude"));
    ev.insert("workspace_id".into(), json!(workspace_id));
    ev.insert("surface_id".into(), json!(surface_id));
    ev.insert("_opencode_request_id".into(), json!(request_id));
    let mut p = Map::new();
    p.insert("wait_timeout_seconds".into(), json!(WAIT_SECS));
    p.insert("event".into(), Value::Object(ev));
    p
}

pub fn hook_output(stdin: &Value, d: &Decision) -> Value {
    let input = stdin.get("tool_input").cloned().unwrap_or(json!({}));
    let deny = |m: &str| json!({"behavior": "deny", "message": m});
    let allow_input = || json!({"behavior": "allow", "updatedInput": input});
    let decision = match d {
        Decision::Permission(PermissionMode::Deny) => deny("User denied permission via cmux Feed."),
        Decision::Permission(m) => {
            let mut v = json!({"behavior": "allow"});
            let sugg = stdin.get("permission_suggestions");
            if matches!(m, PermissionMode::Always | PermissionMode::All)
                && let Some(s) = sugg.filter(|s| s.as_array().is_none_or(|a| !a.is_empty()))
                && !s.is_null()
            {
                v["updatedPermissions"] = s.clone();
            }
            v
        }
        Decision::ExitPlan {
            feedback: Some(f), ..
        } => deny(&format!(
            "User rejected the plan via cmux Feed and wants this change: {f}"
        )),
        Decision::ExitPlan { mode, .. } => match mode {
            PlanMode::Deny => deny("User rejected the plan via cmux Feed."),
            PlanMode::Ultraplan => deny(
                "User chose Ultraplan via cmux Feed. Refine this plan with Ultraplan on Claude Code on the web.",
            ),
            PlanMode::AutoAccept => {
                let mut v = allow_input();
                v["updatedPermissions"] =
                    json!([{"type": "setMode", "mode": "auto", "destination": "session"}]);
                v
            }
            PlanMode::Manual | PlanMode::BypassPermissions => allow_input(),
        },
        Decision::Question { selections }
            if selections.len() == 1 && selections[0] == "Skip interview and plan immediately" =>
        {
            deny(
                "User chose Skip interview and plan immediately via cmux Feed. Do not ask more interview questions. Write the plan now.",
            )
        }
        Decision::Question { selections } => {
            let qs = input.get("questions").and_then(Value::as_array);
            let answers: Map<String, Value> = selections
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let key = qs
                        .and_then(|q| q.get(i))
                        .and_then(|q| q.get("question"))
                        .and_then(Value::as_str)
                        .map_or_else(|| format!("Answer {}", i + 1), String::from);
                    (key, json!(a))
                })
                .collect();
            let mut v = allow_input();
            v["updatedInput"]["answers"] = Value::Object(answers);
            v
        }
    };
    json!({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": decision}})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }
    fn push(tool: &str, input: Value) -> Map<String, Value> {
        obj(json!({"wait_timeout_seconds": 115, "event": {
            "_opencode_request_id": "r1", "_source": "claude", "workspace_id": "w",
            "surface_id": "s", "cwd": "/a/b", "tool_name": tool, "tool_input": input}}))
    }
    fn kind(tool: &str, input: Value) -> FeedKind {
        parse_push(&push(tool, input)).unwrap().kind
    }
    fn out(stdin: Value, d: Decision) -> Value {
        hook_output(&stdin, &d)["hookSpecificOutput"]["decision"].clone()
    }

    #[test]
    fn wait_bounds() {
        let w = |v: Value| wait_of(&obj(json!({"wait_timeout_seconds": v})));
        assert_eq!(wait_of(&Map::new()).unwrap(), Duration::ZERO);
        assert_eq!(w(json!(115)).unwrap(), Duration::from_secs(115));
        assert_eq!(w(json!(120.0)).unwrap(), Duration::from_secs(120));
        let e = RpcError::new(
            "invalid_params",
            "feed.push wait_timeout_seconds must be between 0 and 120",
        );
        assert_eq!(w(json!(-1)), Err(e.clone()));
        assert_eq!(w(json!(121)), Err(e.clone()));
        assert_eq!(w(json!("x")), Err(e));
    }

    #[test]
    fn parse_bash_permission() {
        let p = parse_push(&push(
            "Bash",
            json!({"command": "ls", "description": "list"}),
        ))
        .unwrap();
        assert_eq!(p.request_id, "r1");
        assert_eq!(p.wait, Duration::from_secs(115));
        assert_eq!((p.source.as_str(), p.cwd.as_str()), ("claude", "/a/b"));
        assert_eq!(
            p.kind,
            FeedKind::Permission {
                tool: "Bash".into(),
                preview: Preview::Command {
                    command: "ls".into(),
                    description: Some("list".into())
                }
            }
        );
    }

    #[test]
    fn parse_write_preview() {
        let f = |tool, input| match kind(tool, input) {
            FeedKind::Permission { preview, .. } => preview,
            _ => panic!(),
        };
        assert_eq!(
            f("Write", json!({"file_path": "/x", "content": "one\ntwo"})),
            Preview::File {
                path: "/x".into(),
                first_line: Some("one".into())
            }
        );
        assert_eq!(
            f("Edit", json!({"file_path": "/x"})),
            Preview::File {
                path: "/x".into(),
                first_line: None
            }
        );
        assert_eq!(
            f("Glob", json!({"a": 1})),
            Preview::Raw(pretty(&json!({"a": 1})))
        );
    }

    #[test]
    fn parse_plan() {
        assert_eq!(
            kind(
                "ExitPlanMode",
                json!({"plan": "do it", "allowedPrompts": [{"tool": "Bash", "prompt": "run tests"}, "x"],
                       "plan_file_path": "/p.md"})
            ),
            FeedKind::ExitPlan {
                plan: "do it".into(),
                allowed_prompts: vec!["run tests".into(), "x".into()],
                plan_file: Some("/p.md".into())
            }
        );
    }

    #[test]
    fn parse_questions() {
        let k = kind(
            "AskUserQuestion",
            json!({"questions": [{"question": "Which?", "header": "Auth", "multiSelect": true,
                "options": [{"label": "A", "description": "a"}, {"label": "B"}]}]}),
        );
        assert_eq!(
            k,
            FeedKind::Question {
                prompts: vec![QuestionPrompt {
                    header: "Auth".into(),
                    question: "Which?".into(),
                    multi_select: true,
                    options: vec![
                        QuestionOption {
                            label: "A".into(),
                            description: "a".into()
                        },
                        QuestionOption {
                            label: "B".into(),
                            description: "".into()
                        }
                    ]
                }]
            }
        );
    }

    #[test]
    fn parse_push_errors() {
        let err = |p: Map<String, Value>| parse_push(&p).unwrap_err();
        assert_eq!(err(Map::new()).code, "invalid_params");
        for k in ["_opencode_request_id", "workspace_id", "surface_id"] {
            let mut p = push("Bash", json!({}));
            p["event"].as_object_mut().unwrap().remove(k);
            assert_eq!(err(p).message, format!("feed.push requires event.{k}"));
        }
    }

    #[test]
    fn reply_errors_are_cmux_texts() {
        let e = |m: &str, v: Value| parse_reply(m, &obj(v)).unwrap_err().message;
        assert_eq!(
            e("feed.permission.reply", json!({})),
            "feed.permission.reply requires request_id"
        );
        assert_eq!(
            e(
                "feed.permission.reply",
                json!({"request_id": "a", "mode": "x"})
            ),
            "feed.permission.reply requires mode ∈ once|always|all|bypass|deny"
        );
        assert_eq!(
            e(
                "feed.question.reply",
                json!({"request_id": "a", "selections": [1]})
            ),
            "feed.question.reply requires selections: [string]"
        );
        assert_eq!(
            e("feed.exit_plan.reply", json!({"request_id": "a"})),
            "feed.exit_plan.reply requires mode ∈ ultraplan|bypassPermissions|autoAccept|manual|deny"
        );
        let (_, d) = parse_reply(
            "feed.exit_plan.reply",
            &obj(json!({"request_id": "a", "mode": "manual", "feedback": "  "})),
        )
        .unwrap();
        assert_eq!(
            d,
            Decision::ExitPlan {
                mode: PlanMode::Manual,
                feedback: None
            }
        );
    }

    #[test]
    fn decision_json_round_trips() {
        let mut all = vec![
            Decision::Question {
                selections: vec!["a".into(), "b".into()],
            },
            Decision::ExitPlan {
                mode: PlanMode::Manual,
                feedback: Some("f".into()),
            },
        ];
        for m in [
            PermissionMode::Once,
            PermissionMode::Always,
            PermissionMode::All,
            PermissionMode::Bypass,
            PermissionMode::Deny,
        ] {
            all.push(Decision::Permission(m));
        }
        for mode in [
            PlanMode::Ultraplan,
            PlanMode::BypassPermissions,
            PlanMode::AutoAccept,
            PlanMode::Manual,
            PlanMode::Deny,
        ] {
            all.push(Decision::ExitPlan {
                mode,
                feedback: None,
            });
        }
        for d in all {
            assert_eq!(decision_from_json(&decision_json(&d)), Some(d));
        }
        assert_eq!(
            decision_json(&Decision::Permission(PermissionMode::Once)),
            json!({"kind": "permission", "mode": "once"})
        );
        assert_eq!(decision_from_json(&json!({"kind": "x"})), None);
    }

    #[test]
    fn push_params_shape() {
        let stdin = json!({"session_id": "S", "cwd": "/c", "tool_name": "ExitPlanMode",
            "tool_input": {"plan": "p"}, "other": 1});
        assert_eq!(
            Value::Object(push_params(&stdin, "w", "s", "r")),
            json!({"wait_timeout_seconds": 115, "event": {
                "session_id": "S", "hook_event_name": "ExitPlanMode", "_source": "claude",
                "workspace_id": "w", "surface_id": "s", "cwd": "/c", "tool_name": "ExitPlanMode",
                "tool_input": {"plan": "p"}, "_opencode_request_id": "r"}})
        );
        let p = push_params(&json!({"tool_name": "Bash"}), "w", "s", "r");
        let ev = p["event"].as_object().unwrap();
        assert_eq!(ev["hook_event_name"], "PermissionRequest");
        assert!(!ev.contains_key("session_id") && !ev.contains_key("cwd"));
    }

    #[test]
    fn hook_output_wrapper() {
        assert_eq!(
            hook_output(&json!({}), &Decision::Permission(PermissionMode::Once)),
            json!({"hookSpecificOutput": {"hookEventName": "PermissionRequest",
                "decision": {"behavior": "allow"}}})
        );
    }

    #[test]
    fn hook_output_once_and_bypass() {
        for m in [PermissionMode::Once, PermissionMode::Bypass] {
            assert_eq!(
                out(
                    json!({"permission_suggestions": [1]}),
                    Decision::Permission(m)
                ),
                json!({"behavior": "allow"})
            );
        }
    }

    #[test]
    fn hook_output_always() {
        let s = json!([{"type": "addRules", "destination": "session"}]);
        for m in [PermissionMode::Always, PermissionMode::All] {
            assert_eq!(
                out(
                    json!({"permission_suggestions": s}),
                    Decision::Permission(m)
                ),
                json!({"behavior": "allow", "updatedPermissions": s})
            );
        }
        let none = Decision::Permission(PermissionMode::Always);
        assert_eq!(out(json!({}), none.clone()), json!({"behavior": "allow"}));
        assert_eq!(
            out(json!({"permission_suggestions": []}), none),
            json!({"behavior": "allow"})
        );
    }

    #[test]
    fn hook_output_deny() {
        assert_eq!(
            out(json!({}), Decision::Permission(PermissionMode::Deny)),
            json!({"behavior": "deny", "message": "User denied permission via cmux Feed."})
        );
    }

    #[test]
    fn hook_output_plan_manual() {
        let i = json!({"plan": "p"});
        for mode in [PlanMode::Manual, PlanMode::BypassPermissions] {
            assert_eq!(
                out(
                    json!({"tool_input": i}),
                    Decision::ExitPlan {
                        mode,
                        feedback: None
                    }
                ),
                json!({"behavior": "allow", "updatedInput": i})
            );
        }
    }

    #[test]
    fn hook_output_plan_auto() {
        assert_eq!(
            out(
                json!({"tool_input": {"plan": "p"}}),
                Decision::ExitPlan {
                    mode: PlanMode::AutoAccept,
                    feedback: None
                }
            ),
            json!({"behavior": "allow", "updatedInput": {"plan": "p"},
                "updatedPermissions": [{"type": "setMode", "mode": "auto", "destination": "session"}]})
        );
    }

    #[test]
    fn hook_output_plan_ultraplan_feedback_deny() {
        let e = |mode, feedback| out(json!({}), Decision::ExitPlan { mode, feedback });
        assert_eq!(
            e(PlanMode::Ultraplan, None),
            json!({"behavior": "deny", "message": "User chose Ultraplan via cmux Feed. Refine this plan with Ultraplan on Claude Code on the web."})
        );
        assert_eq!(
            e(PlanMode::Manual, Some("use sqlite".into())),
            json!({"behavior": "deny", "message": "User rejected the plan via cmux Feed and wants this change: use sqlite"})
        );
        assert_eq!(
            e(PlanMode::Deny, None),
            json!({"behavior": "deny", "message": "User rejected the plan via cmux Feed."})
        );
    }

    #[test]
    fn hook_output_question() {
        let stdin =
            json!({"tool_input": {"questions": [{"question": "Which?"}, {"question": "Where?"}]}});
        assert_eq!(
            out(
                stdin.clone(),
                Decision::Question {
                    selections: vec!["A, B".into(), "C".into(), "D".into()]
                }
            ),
            json!({"behavior": "allow", "updatedInput": {
                "questions": [{"question": "Which?"}, {"question": "Where?"}],
                "answers": {"Which?": "A, B", "Where?": "C", "Answer 3": "D"}}})
        );
        assert_eq!(
            out(
                stdin,
                Decision::Question {
                    selections: vec!["Skip interview and plan immediately".into()]
                }
            )["behavior"],
            "deny"
        );
    }
}
