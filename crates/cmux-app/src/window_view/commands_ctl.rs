//! Running `cmux.json` commands from the palette (M11 plan C4, C5): the trust prompt, a plain
//! command typed into the focused terminal, a workspace command's layout.

use std::path::{Path, PathBuf};

use cmux_config::commands::{self, Command, Loaded, Node, Restart, Run, Surface, Trust};
use cmux_core::{Layout, Orientation, Shape, WorkspaceId};
use gpui::{AppContext, Context, PromptLevel, Window};

use super::WindowView;
use crate::workspace::WorkspaceView;

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

/// `trusted-commands.json` in the state directory (next to `session.json`).
fn trust_path() -> Option<PathBuf> {
    Some(cmux_ipc::paths::state_dir(&env)?.join("trusted-commands.json"))
}

/// A layout as a shape, the focused pane (tree order: the first with a `focus` surface) and
/// every surface in `Layout::surfaces` order. No layout: one plain terminal.
fn plan(layout: Option<&Node>) -> (Shape, usize, Vec<Surface>) {
    fn walk(
        n: &Node,
        out: &mut Vec<Surface>,
        panes: &mut usize,
        focus: &mut Option<usize>,
    ) -> Shape {
        match n {
            Node::Pane(s) => {
                let selected = s.iter().position(|x| x.focus);
                if selected.is_some() && focus.is_none() {
                    *focus = Some(*panes);
                }
                *panes += 1;
                out.extend(s.iter().cloned());
                Shape::Pane {
                    tabs: s.len(),
                    selected: selected.unwrap_or(0),
                }
            }
            Node::Split {
                horizontal,
                ratio,
                first,
                second,
            } => Shape::Split {
                orientation: if *horizontal {
                    Orientation::Horizontal
                } else {
                    Orientation::Vertical
                },
                ratio: *ratio,
                first: Box::new(walk(first, out, panes, focus)),
                second: Box::new(walk(second, out, panes, focus)),
            },
        }
    }
    let Some(n) = layout else {
        return (
            Shape::Pane {
                tabs: 1,
                selected: 0,
            },
            0,
            vec![Surface::default()],
        );
    };
    let (mut out, mut panes, mut focus) = (Vec::new(), 0, None);
    let shape = walk(n, &mut out, &mut panes, &mut focus);
    (shape, focus.unwrap_or(0), out)
}

impl WindowView {
    /// Both `cmux.json` files, the project one found from the selected workspace's cwd.
    pub(super) fn load_commands(&self, cx: &Context<Self>) -> Loaded {
        let sel = self.workspaces.selected();
        let cwd = self.views.get(&sel).and_then(|v| v.read(cx).active_cwd(cx));
        let global = commands::global_path(&env);
        let l = commands::load(global.as_deref(), cwd.as_deref().map(Path::new));
        for w in &l.warnings {
            eprintln!("cmux-linux: {w}");
        }
        l
    }

    /// Runs `c`, asking first when the trust rules say so (plan C5).
    pub(super) fn run_custom(&mut self, c: Command, window: &mut Window, cx: &mut Context<Self>) {
        let store = trust_path();
        let mut trust = store.as_deref().map(Trust::load).unwrap_or_default();
        if !trust.needs_prompt(&c) {
            return self.exec_custom(c, window, cx);
        }
        let title = commands::sanitize(&c.name);
        let detail = format!(
            "This project action comes from:\n\n{}\n\nIt will run:\n\n{}",
            commands::sanitize(&c.source.display().to_string()),
            commands::disclosure(&c),
        );
        let answer = window.prompt(
            PromptLevel::Warning,
            if title.is_empty() {
                "Run Project Action?"
            } else {
                &title
            },
            Some(&detail),
            &["Run", "Trust and Run", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            match answer.await {
                Ok(0) => {}
                Ok(1) => {
                    if let Some(p) = &store
                        && let Err(e) = trust.trust(&c, p)
                    {
                        eprintln!("cmux-linux: could not save {}: {e}", p.display());
                    }
                }
                _ => return,
            }
            let _ = this.update_in(cx, |this, window, cx| this.exec_custom(c, window, cx));
        })
        .detach();
    }

    fn exec_custom(&mut self, c: Command, window: &mut Window, cx: &mut Context<Self>) {
        let w = match c.run {
            Run::Workspace(w) => w,
            Run::Shell(line) => {
                let sel = self.workspaces.selected();
                let term = self.views.get(&sel).and_then(|v| {
                    let v = v.read(cx);
                    v.terminal(v.layout().active_surface()).cloned()
                });
                if let Some(t) = term {
                    t.update(cx, |t, cx| {
                        t.send_scripted(format!("{line}\r").into_bytes(), cx)
                    });
                }
                return;
            }
        };
        let title = w.name.clone().unwrap_or(c.name);
        let existing = self
            .workspaces
            .list()
            .iter()
            .find(|x| x.custom_title.as_deref() == Some(title.as_str()))
            .map(|x| x.id);
        let replace = match (existing, c.restart) {
            (None, _) | (Some(_), Restart::New) => None,
            (Some(id), Restart::Ignore) => return self.select(id, window, cx),
            (Some(id), Restart::Recreate) => Some(id),
            (Some(id), Restart::Confirm) => {
                let answer = window.prompt(
                    PromptLevel::Warning,
                    "Workspace Already Exists",
                    Some(
                        "A workspace with this name already exists. Close it and create a new one?",
                    ),
                    &["Recreate", "Cancel"],
                    cx,
                );
                cx.spawn_in(window, async move |this, cx| {
                    let recreate = answer.await == Ok(0);
                    let _ = this.update_in(cx, |this, window, cx| {
                        if recreate {
                            this.open_custom(title, w, Some(id), window, cx);
                        } else {
                            this.select(id, window, cx);
                        }
                    });
                })
                .detach();
                return;
            }
        };
        self.open_custom(title, w, replace, window, cx);
    }

    /// A new workspace titled `title` after the selected one, built from `w`; `replace` is
    /// closed once it stands.
    fn open_custom(
        &mut self,
        title: String,
        w: commands::WorkspaceDef,
        replace: Option<WorkspaceId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let old = self.workspaces.selected();
        let home = env("HOME").unwrap_or_default();
        let base = self
            .views
            .get(&old)
            .and_then(|v| v.read(cx).active_cwd(cx))
            .unwrap_or_else(|| home.clone());
        let root = commands::resolve_cwd(w.cwd.as_deref(), &base, &home);
        let (shape, focused, surfaces) = plan(w.layout.as_ref());
        let starts = surfaces
            .iter()
            .map(|s| {
                let cwd = commands::resolve_cwd(s.cwd.as_deref(), &root, &home);
                // The surface's env after the workspace's, so it wins.
                let env = w.env.iter().chain(&s.env).cloned().collect();
                (Some(cwd), env)
            })
            .collect();
        // shortcut: surface `name` is not shown (tabs show the OSC title); add with tab titles.
        let inputs = surfaces.into_iter().map(|s| s.command).collect();
        let id = self.workspaces.add_after_selected();
        let (style, shell) = (self.style.clone(), self.shell.clone());
        let layout = Layout::from_shape(&shape, focused);
        let view = cx
            .new(|cx| WorkspaceView::custom(id, shell, style, layout, starts, inputs, window, cx));
        if !self.register_view(id, view, window, cx) {
            self.workspaces.close(id);
            self.select(old, window, cx);
            return;
        }
        self.workspaces.rename(id, &title);
        self.refresh_title(id, cx);
        if let Some(r) = replace.filter(|r| *r != id) {
            self.close_now(r, window, cx);
        }
        self.select(id, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_become_shapes_in_surface_order() {
        let t = |cmd: &str, focus| Surface {
            command: Some(cmd.into()),
            focus,
            ..Default::default()
        };
        let n = Node::Split {
            horizontal: true,
            ratio: 0.3,
            first: Box::new(Node::Pane(vec![t("a", false)])),
            second: Box::new(Node::Split {
                horizontal: false,
                ratio: 0.5,
                first: Box::new(Node::Pane(vec![t("b", false), t("c", true)])),
                second: Box::new(Node::Pane(vec![t("d", true)])),
            }),
        };
        let (shape, focused, surfaces) = plan(Some(&n));
        assert_eq!(focused, 1);
        let cmds: Vec<_> = surfaces
            .iter()
            .map(|s| s.command.as_deref().unwrap())
            .collect();
        assert_eq!(cmds, ["a", "b", "c", "d"]);
        let l = Layout::from_shape(&shape, focused);
        assert_eq!(l.surfaces().len(), 4);
        // The focused pane shows its focus surface, "c".
        assert_eq!(l.active_surface(), l.surfaces()[2]);
        let Shape::Split { orientation, .. } = shape else {
            panic!()
        };
        assert_eq!(orientation, Orientation::Horizontal);

        let (shape, focused, surfaces) = plan(None);
        assert_eq!(
            (shape, focused, surfaces.len()),
            (
                Shape::Pane {
                    tabs: 1,
                    selected: 0
                },
                0,
                1
            )
        );
    }
}
