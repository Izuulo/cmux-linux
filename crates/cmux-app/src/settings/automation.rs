//! The Automation section (M12 plan S7): `[auto_naming]` and one card per `[[vault.agents]]`
//! entry, all in config.toml.

use cmux_config::auto_naming::DEFAULT_COMMAND;
use cmux_config::vault::{CLAUDE, Registration};
use cmux_config::write;
use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Styled,
    Window, div, prelude::*,
};

use super::controls::{self, CONTROL_W, Theme};
use super::{FieldId, SettingsView};
use crate::ui::px;

/// Each agent field: config key, title.
const AGENT_FIELDS: &[(&str, &str)] = &[
    ("id", "ID"),
    ("name", "Name"),
    ("process_name", "Process Name"),
    ("argv_option", "Session ID Option"),
    ("resume_command", "Resume Command"),
];

fn get<'a>(a: &'a Registration, key: &str) -> &'a str {
    match key {
        "id" => &a.id,
        "name" => &a.name,
        "process_name" => &a.process_name,
        "argv_option" => &a.argv_option,
        _ => &a.resume_command,
    }
}

/// `agents` with agent `n`'s `key` set to `text`, or why not. An empty name is the id.
fn with_field(
    agents: &[Registration],
    n: usize,
    key: &str,
    text: &str,
) -> Result<Vec<Registration>, String> {
    let mut agents = agents.to_vec();
    let others_have = |id: &str| agents.iter().enumerate().any(|(k, a)| k != n && a.id == id);
    if key == "id" && (text.is_empty() || text == CLAUDE || others_have(text)) {
        return Err(format!("Agent ID `{text}` is empty or taken."));
    }
    let a = agents.get_mut(n).ok_or("That agent is gone.")?;
    match key {
        "id" => {
            if a.name == a.id {
                a.name = text.to_string();
            }
            a.id = text.to_string();
        }
        "name" if text.is_empty() => a.name = a.id.clone(),
        "name" => a.name = text.to_string(),
        "process_name" => a.process_name = text.to_string(),
        "argv_option" => a.argv_option = text.to_string(),
        _ => a.resume_command = text.to_string(),
    }
    Ok(agents)
}

/// The first of `agent`, `agent-2`, … that no agent uses.
fn new_id(agents: &[Registration]) -> String {
    (1..)
        .map(|k| {
            if k == 1 {
                "agent".to_string()
            } else {
                format!("agent-{k}")
            }
        })
        .find(|id| agents.iter().all(|a| &a.id != id))
        .expect("an unused id")
}

impl SettingsView {
    /// The custom command, or `None` for the default.
    fn naming_command(&self) -> Option<String> {
        (self.auto.command != DEFAULT_COMMAND).then(|| self.auto.command.clone())
    }

    pub(super) fn commit_naming_command(&mut self, text: &str, cx: &mut Context<Self>) {
        let enabled = self.auto.enabled;
        self.write_toml(|p| write::set_auto_naming(p, enabled, Some(text)), cx);
    }

    pub(super) fn commit_agent(&mut self, n: usize, key: &str, text: &str, cx: &mut Context<Self>) {
        match with_field(&self.agents, n, key, text) {
            // An id change renames in place; set_agents would replace the table.
            Ok(_) if key == "id" => {
                let old = self.agents[n].id.clone();
                self.write_toml(|p| write::rename_agent(p, &old, text), cx)
            }
            Ok(agents) => self.write_toml(|p| write::set_agents(p, &agents), cx),
            Err(e) => self.error = Some(e),
        }
    }

    pub(super) fn button(
        &self,
        id: impl Into<gpui::ElementId>,
        label: &'static str,
        c: Theme,
        cx: &Context<Self>,
        f: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        div()
            .id(id)
            .flex()
            .items_center()
            .h(px(22.))
            .px(px(10.))
            .rounded(px(5.))
            .bg(c.control)
            .text_size(px(12.))
            .hover(move |s| s.bg(c.hover))
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| f(this, cx)))
            .into_any_element()
    }

    pub(super) fn render_automation(
        &self,
        c: Theme,
        window: &Window,
        cx: &Context<Self>,
    ) -> Vec<AnyElement> {
        let on = self.auto.enabled;
        let toggle = controls::toggle(
            "auto-naming",
            on,
            c,
            self.handler(cx, move |this, _, cx| {
                let cmd = this.naming_command();
                this.write_toml(|p| write::set_auto_naming(p, !on, cmd.as_deref()), cx);
            }),
        );
        let command = self.field(
            FieldId::AutoNamingCommand,
            "auto-naming-command",
            &self.naming_command().unwrap_or_default(),
            "Default (claude -p, Haiku)",
            None,
            CONTROL_W,
            window,
            cx,
        );
        let naming = controls::card(
            vec![
                controls::row(
                    "Auto-name Workspaces",
                    Some("auto_naming.enabled · titles a workspace from its agent's turns"),
                    toggle,
                    None,
                    c,
                ),
                controls::row(
                    "Naming Command",
                    Some("auto_naming.command · reads the prompt on stdin, prints the title"),
                    command,
                    None,
                    c,
                ),
            ],
            c,
        );
        let mut out = vec![
            naming.into_any_element(),
            controls::header("Agents", c).into_any_element(),
            div()
                .pl(px(2.))
                .text_size(px(11.))
                .text_color(c.secondary)
                .child(
                    "Agents whose sessions are resumed after a restart ([[vault.agents]] in \
                     config.toml). The resume command needs {{sessionId}}.",
                )
                .into_any_element(),
        ];
        for (n, a) in self.agents.iter().enumerate() {
            let mut rows: Vec<AnyElement> = AGENT_FIELDS
                .iter()
                .map(|&(key, title)| {
                    let field = self.field(
                        FieldId::Agent(n, key),
                        (key, n),
                        get(a, key),
                        "",
                        None,
                        CONTROL_W,
                        window,
                        cx,
                    );
                    controls::row(title, Some(key), field, None, c)
                })
                .collect();
            let id = a.id.clone();
            let remove = self.button(("remove-agent", n), "Remove", c, cx, move |this, cx| {
                // By id: the file may have changed since this frame.
                let agents: Vec<Registration> =
                    this.agents.iter().filter(|a| a.id != id).cloned().collect();
                this.write_toml(|p| write::set_agents(p, &agents), cx);
            });
            rows.push(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .px(px(14.))
                    .py(px(9.))
                    .child(remove)
                    .into_any_element(),
            );
            out.push(controls::card(rows, c).into_any_element());
        }
        out.extend(self.agent_warnings.iter().map(|w| {
            div()
                .pl(px(2.))
                .child(controls::note(w.clone(), c.warning))
                .into_any_element()
        }));
        out.push(
            div()
                .flex()
                .flex_row()
                .child(self.button("add-agent", "Add Agent", c, cx, |this, cx| {
                    let mut agents = this.agents.clone();
                    let id = new_id(&agents);
                    agents.push(Registration {
                        name: id.clone(),
                        id,
                        process_name: String::new(),
                        argv_option: String::new(),
                        resume_command: String::new(),
                    });
                    this.write_toml(|p| write::set_agents(p, &agents), cx);
                }))
                .into_any_element(),
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(id: &str) -> Registration {
        Registration {
            id: id.into(),
            name: id.into(),
            process_name: String::new(),
            argv_option: String::new(),
            resume_command: String::new(),
        }
    }

    #[test]
    fn ids_stay_unique_and_names_follow() {
        let list = vec![agent("pi"), agent("agent")];
        assert!(with_field(&list, 0, "id", "agent").is_err());
        assert!(with_field(&list, 0, "id", "").is_err());
        assert!(with_field(&list, 0, "id", "claude").is_err());
        let renamed = with_field(&list, 0, "id", "pi2").unwrap();
        assert_eq!(
            (renamed[0].id.as_str(), renamed[0].name.as_str()),
            ("pi2", "pi2")
        );
        let named = with_field(&list, 0, "name", "").unwrap();
        assert_eq!(named[0].name, "pi");
        assert_eq!(new_id(&list), "agent-2");
        assert_eq!(new_id(&[]), "agent");
    }
}
