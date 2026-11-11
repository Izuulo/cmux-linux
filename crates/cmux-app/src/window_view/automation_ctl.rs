//! Automations in the app (M9 plan Part 3): `emit` is the event bus, called where things
//! happen; matching rules' actions run in order, off the UI thread where they can.

use std::path::PathBuf;
use std::time::Instant;

use cmux_core::{SurfaceId, WorkspaceId};
use cmux_ipc::automation::{self, Action, Rules};
use cmux_ipc::handles::Kind;
use cmux_ipc::wire::Reply;
use gpui::{Context, Window};
use serde_json::{Map, Value, json};

use super::WindowView;

/// `~/.config/cmux-linux/automations.json` (`$XDG_CONFIG_HOME` first).
fn path() -> Option<PathBuf> {
    let (dock, _) = super::dock::dock_config_path()?;
    Some(dock.with_file_name("automations.json"))
}

/// No file: no rules, no warning.
pub(super) fn load() -> Rules {
    let Some(text) = path().and_then(|p| std::fs::read_to_string(p).ok()) else {
        return Rules::default();
    };
    let rules = automation::parse(&text);
    for w in &rules.warnings {
        eprintln!("cmux-linux: automations.json: {w}");
    }
    rules
}

impl WindowView {
    /// Something happened in `ws` (on `surface`): rules that match run their actions.
    pub(super) fn emit(
        &mut self,
        name: &str,
        ws: Option<WorkspaceId>,
        surface: Option<SurfaceId>,
        mut payload: Map<String, Value>,
        cx: &mut Context<Self>,
    ) {
        // A rule's own notification is not an event (plan A6).
        if self.automating || self.automations.rules.is_empty() {
            return;
        }
        if let Some(ws) = ws {
            let mut h = self.shell.handles.borrow_mut();
            payload.insert(
                "workspace_id".into(),
                h.handle(Kind::Workspace, (ws.0, 0)).id.into(),
            );
            if let Some(s) = surface {
                payload.insert(
                    "surface_id".into(),
                    h.handle(Kind::Surface, (ws.0, s.0)).id.into(),
                );
            }
            drop(h);
            payload.insert("workspace_title".into(), self.title_of(ws).into());
        }
        let event = automation::event(name, payload);
        let firings = self.automations.fire(&event, Instant::now());
        if firings.is_empty() {
            return;
        }
        let (window, line) = (self.window_handle, event.to_string());
        let (name, category) = (
            name.to_string(),
            event["category"].as_str().unwrap_or("").to_string(),
        );
        cx.spawn(async move |this, cx| {
            for f in firings {
                for action in f.actions {
                    match action {
                        Action::Notify {
                            title,
                            subtitle,
                            body,
                        } => {
                            let title = if title.is_empty() {
                                format!("Automation: {}", f.rule)
                            } else {
                                title
                            };
                            let _ = window.update(cx, |_, window, cx| {
                                this.update(cx, |s, cx| {
                                    s.automation_notify(
                                        ws, surface, title, subtitle, body, window, cx,
                                    )
                                })
                            });
                        }
                        Action::Run { command, timeout } => {
                            let (line, rule) = (line.clone(), f.rule.clone());
                            let (name, category) = (name.clone(), category.clone());
                            cx.background_executor()
                                .spawn(async move {
                                    // cmux's variable names.
                                    let env = [
                                        ("CMUX_AUTOMATION_EVENT", line.as_str()),
                                        ("CMUX_AUTOMATION_EVENT_JSON", line.as_str()),
                                        ("CMUX_AUTOMATION_RULE_ID", rule.as_str()),
                                        ("CMUX_AUTOMATION_EVENT_NAME", name.as_str()),
                                        ("CMUX_AUTOMATION_EVENT_CATEGORY", category.as_str()),
                                    ];
                                    cmux_meta::sh(&command, &env, "", timeout)
                                })
                                .await;
                        }
                    }
                }
            }
        })
        .detach();
    }

    /// The `notify` action: on the event's terminal if it is still open, else the selected
    /// workspace's active one.
    #[allow(clippy::too_many_arguments)]
    fn automation_notify(
        &mut self,
        ws: Option<WorkspaceId>,
        surface: Option<SurfaceId>,
        title: String,
        subtitle: String,
        body: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ws = ws
            .filter(|w| self.views.contains_key(w))
            .unwrap_or(self.workspaces.selected());
        let Some(view) = self.views.get(&ws) else {
            return;
        };
        let view = view.read(cx);
        let surface = surface
            .filter(|s| view.terminal(*s).is_some())
            .unwrap_or(view.layout().active_surface());
        self.automating = true;
        self.on_notify(ws, surface, title, subtitle, body, true, window, cx);
        self.automating = false;
    }

    pub(super) fn automation_from_socket(&mut self, method: &str) -> Reply {
        if method == "automation.reload" {
            self.automations = load();
        }
        let rules: Vec<Value> = self.automations.rules.iter().map(|r| r.summary()).collect();
        Ok(json!({
            "path": path(), "rules": rules, "warnings": self.automations.warnings,
        }))
    }
}
