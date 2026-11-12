//! Layout restore (M10 Part 1): `session.json` written every 8 s when it changed and on quit,
//! read once at start (plan L1–L6).

use std::path::{Path, PathBuf};
use std::time::Duration;

use cmux_core::{Group, GroupId, Layout, Orientation, Shape, Workspace, WorkspaceId, Workspaces};
use gpui::{App, Context, Entity, Task, Window, WindowBounds};
use serde_json::{Value, json};

use cmux_config::vault::{self, Registration};
use cmux_ipc::local_tmux;

use super::WindowView;
use super::dock::RightMode;
use crate::workspace::WorkspaceView;

const VERSION: u64 = 1;
/// cmux `SessionPersistencePolicy.autosaveInterval`.
const SAVE_EVERY: Duration = Duration::from_secs(8);

/// An agent session running in a terminal (M10 plan V1, V2): Claude by its hooks, a
/// `[[vault.agents]]` registration by its process.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    /// `claude` or a registration's id.
    pub kind: String,
    pub session_id: String,
    /// The detected process's executable (`{{executable}}`).
    pub executable: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SavedTerm {
    pub cwd: Option<String>,
    pub agent: Option<Agent>,
}

/// One workspace's (or the Dock's) panes: the tree, the focused pane and each surface.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedLayout {
    pub shape: Shape,
    pub focused: usize,
    pub terms: Vec<SavedTerm>,
}

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

/// What a foreground process `pid` is, if anything to resume: a client attached to a
/// local-tmux session (plan T3), else a registered agent (V2).
pub fn detect_agent(regs: &[Registration], pid: u32) -> Option<Agent> {
    let (argv, exe, comm) = cmux_term::pty::process_argv(pid)?;
    if let Some(sock) = local_tmux::socket_path(&env)
        && let Some(environ) = cmux_term::pty::process_environ(pid)
        && let Some(name) = local_tmux::detect(&argv, &environ, &sock.to_string_lossy())
    {
        return Some(Agent {
            kind: local_tmux::KIND.into(),
            session_id: name,
            executable: exe,
        });
    }
    if regs.is_empty() {
        return None;
    }
    let names: Vec<&str> = [&comm, &exe]
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect();
    let (r, session_id) = vault::detect(regs, &names, &argv)?;
    Some(Agent {
        kind: r.id.clone(),
        session_id,
        executable: exe,
    })
}

impl SavedTerm {
    /// The line that resumes this terminal's agent, if it has a valid one.
    pub fn resume_line(&self, regs: &[Registration]) -> Option<String> {
        let a = self.agent.as_ref()?;
        let (exe, cwd) = (a.executable.as_deref(), self.cwd.as_deref());
        if a.kind == local_tmux::KIND {
            let sock = local_tmux::socket_path(&env)?;
            // The configured binary, as the CLI's line: a wrapper's executable is its interpreter.
            let bin = local_tmux::bin(&env);
            return local_tmux::valid_name(&a.session_id)
                .then(|| local_tmux::attach_line(&bin, &sock.to_string_lossy(), &a.session_id));
        }
        vault::resume_line(regs, &a.kind, &a.session_id, exe, cwd)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SavedWorkspace {
    pub title: Option<String>,
    pub auto_title: bool,
    /// Index into `Saved::groups`.
    pub group: Option<usize>,
    pub layout: SavedLayout,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SavedGroup {
    pub name: String,
    pub collapsed: bool,
    pub pinned: bool,
    pub generated_anchor: bool,
    /// Index into `Saved::workspaces`.
    pub anchor: usize,
}

/// Everything `session.json` holds.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
    pub sidebar_visible: bool,
    pub sidebar_width: f32,
    pub right_open: bool,
    pub right_dock: bool,
    pub right_width: f32,
    pub selected: usize,
    pub groups: Vec<SavedGroup>,
    pub workspaces: Vec<SavedWorkspace>,
    pub dock: Option<SavedLayout>,
}

/// `session.json` in the state directory (next to `cmux.sock`).
pub fn path() -> Option<PathBuf> {
    let env = |k: &str| std::env::var(k).ok();
    Some(cmux_ipc::paths::state_dir(&env)?.join("session.json"))
}

/// The saved session, unless restore is off (plan L2): `CMUX_DISABLE_SESSION_RESTORE=1` or any
/// launch argument. A file that doesn't parse is moved to `session.json.bad`.
pub fn load() -> Option<Saved> {
    if std::env::var("CMUX_DISABLE_SESSION_RESTORE").is_ok_and(|v| v == "1")
        || std::env::args_os().len() > 1
    {
        return None;
    }
    let path = path()?;
    let text = std::fs::read_to_string(&path).ok()?;
    let saved = serde_json::from_str(&text).ok().and_then(|v| from_json(&v));
    if saved.is_none() {
        eprintln!(
            "cmux-linux: {} could not be read; starting fresh",
            path.display()
        );
        let _ = std::fs::rename(&path, path.with_extension("json.bad"));
    }
    saved
}

/// Writes `text` through a temp file and a rename, so a crash never leaves half a file.
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

fn shape_json(s: &Shape) -> Value {
    match s {
        Shape::Pane { tabs, selected } => json!({"pane": {"tabs": tabs, "selected": selected}}),
        Shape::Split {
            orientation,
            ratio,
            first,
            second,
        } => json!({"split": {
            "orientation": match orientation {
                Orientation::Horizontal => "horizontal",
                Orientation::Vertical => "vertical",
            },
            "ratio": ratio,
            "first": shape_json(first),
            "second": shape_json(second),
        }}),
    }
}

/// Depth-limited, so a hand-made file can't overflow the stack.
fn shape_from(v: &Value, depth: usize) -> Option<Shape> {
    if depth > 64 {
        return None;
    }
    if let Some(p) = v.get("pane") {
        return Some(Shape::Pane {
            tabs: p.get("tabs")?.as_u64()?.min(512) as usize,
            selected: p.get("selected")?.as_u64()? as usize,
        });
    }
    let s = v.get("split")?;
    Some(Shape::Split {
        orientation: match s.get("orientation")?.as_str()? {
            "horizontal" => Orientation::Horizontal,
            "vertical" => Orientation::Vertical,
            _ => return None,
        },
        ratio: s.get("ratio")?.as_f64()? as f32,
        first: Box::new(shape_from(s.get("first")?, depth + 1)?),
        second: Box::new(shape_from(s.get("second")?, depth + 1)?),
    })
}

fn layout_json(l: &SavedLayout) -> Value {
    json!({
        "layout": shape_json(&l.shape),
        "focused": l.focused,
        "terminals": l.terms.iter().map(|t| json!({
            "cwd": t.cwd,
            "agent": t.agent.as_ref().map(|a| json!({
                "kind": a.kind, "session_id": a.session_id, "executable": a.executable,
            })),
        })).collect::<Vec<_>>(),
    })
}

fn layout_from(v: &Value) -> Option<SavedLayout> {
    Some(SavedLayout {
        shape: shape_from(v.get("layout")?, 0)?,
        focused: v.get("focused")?.as_u64()? as usize,
        terms: v
            .get("terminals")?
            .as_array()?
            .iter()
            .map(|t| {
                let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
                SavedTerm {
                    cwd: s(t, "cwd"),
                    // An agent entry that doesn't parse is dropped, not the file.
                    agent: t.get("agent").and_then(|a| {
                        Some(Agent {
                            kind: s(a, "kind")?,
                            session_id: s(a, "session_id")
                                .filter(|id| vault::valid_session_id(id))?,
                            executable: s(a, "executable"),
                        })
                    }),
                }
            })
            .collect(),
    })
}

pub fn to_json(s: &Saved) -> Value {
    json!({
        "version": VERSION,
        "window": {"width": s.width, "height": s.height, "maximized": s.maximized},
        "sidebar": {"visible": s.sidebar_visible, "width": s.sidebar_width},
        "right_sidebar": {
            "open": s.right_open,
            "mode": if s.right_dock { "dock" } else { "feed" },
            "width": s.right_width,
        },
        "selected": s.selected,
        "groups": s.groups.iter().map(|g| json!({
            "name": g.name,
            "collapsed": g.collapsed,
            "pinned": g.pinned,
            "generated_anchor": g.generated_anchor,
            "anchor": g.anchor,
        })).collect::<Vec<_>>(),
        "workspaces": s.workspaces.iter().map(|w| {
            let mut v = layout_json(&w.layout);
            v["title"] = json!(w.title);
            v["auto_title"] = json!(w.auto_title);
            v["group"] = json!(w.group);
            v
        }).collect::<Vec<_>>(),
        "dock": s.dock.as_ref().map(layout_json),
    })
}

pub fn from_json(v: &Value) -> Option<Saved> {
    if v.get("version")?.as_u64()? != VERSION {
        return None;
    }
    let f = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).map(|x| x as f32);
    let b = |v: &Value, k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
    let (win, side, right) = (v.get("window")?, v.get("sidebar")?, v.get("right_sidebar")?);
    let workspaces = v
        .get("workspaces")?
        .as_array()?
        .iter()
        .map(|w| {
            Some(SavedWorkspace {
                title: w.get("title").and_then(Value::as_str).map(str::to_string),
                auto_title: b(w, "auto_title"),
                group: w.get("group").and_then(Value::as_u64).map(|g| g as usize),
                layout: layout_from(w)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let groups = v
        .get("groups")?
        .as_array()?
        .iter()
        .map(|g| {
            Some(SavedGroup {
                name: g.get("name")?.as_str()?.to_string(),
                collapsed: b(g, "collapsed"),
                pinned: b(g, "pinned"),
                generated_anchor: b(g, "generated_anchor"),
                anchor: g.get("anchor")?.as_u64()? as usize,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Saved {
        width: f(win, "width")?,
        height: f(win, "height")?,
        maximized: b(win, "maximized"),
        sidebar_visible: b(side, "visible"),
        sidebar_width: f(side, "width")?,
        right_open: b(right, "open"),
        right_dock: right.get("mode").and_then(Value::as_str) == Some("dock"),
        right_width: f(right, "width")?,
        selected: v.get("selected")?.as_u64()? as usize,
        groups,
        workspaces,
        dock: match v.get("dock") {
            None | Some(Value::Null) => None,
            Some(d) => Some(layout_from(d)?),
        },
    })
}

/// The list rebuilt with ids 1..n (workspaces) and 1..m (groups); a group or anchor index out
/// of range is dropped, and `normalize` repairs the rest.
pub fn workspaces_from(s: &Saved) -> Workspaces {
    let gid = |i: usize| GroupId(i as u64 + 1);
    let list = s
        .workspaces
        .iter()
        .enumerate()
        .map(|(i, w)| Workspace {
            id: WorkspaceId(i as u64 + 1),
            custom_title: w.title.clone(),
            auto_title: w.auto_title && w.title.is_some(),
            group: w.group.filter(|g| *g < s.groups.len()).map(gid),
        })
        .collect();
    let groups = s
        .groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.anchor < s.workspaces.len())
        .map(|(i, g)| Group {
            id: gid(i),
            name: g.name.clone(),
            anchor: WorkspaceId(g.anchor as u64 + 1),
            collapsed: g.collapsed,
            pinned: g.pinned,
            generated_anchor: g.generated_anchor,
        })
        .collect();
    Workspaces::from_parts(list, groups, s.selected)
}

impl SavedLayout {
    fn of(view: &Entity<WorkspaceView>, regs: &[Registration], cx: &App) -> SavedLayout {
        let (shape, focused, terms) = view.read(cx).saved(regs, cx);
        SavedLayout {
            shape,
            focused,
            terms,
        }
    }

    /// The resume line per surface (plan V3).
    pub(super) fn inputs(&self, regs: &[Registration]) -> Vec<Option<String>> {
        self.terms.iter().map(|t| t.resume_line(regs)).collect()
    }

    pub(super) fn layout(&self) -> Layout {
        Layout::from_shape(&self.shape, self.focused)
    }
}

impl WindowView {
    /// Remembers the window's size for the next save (called from render).
    pub(super) fn note_window_bounds(&mut self, window: &Window) {
        self.window_bounds = Some(match window.window_bounds() {
            WindowBounds::Windowed(b) => (b.size.width.into(), b.size.height.into(), false),
            WindowBounds::Maximized(b) | WindowBounds::Fullscreen(b) => {
                (b.size.width.into(), b.size.height.into(), true)
            }
        });
    }

    fn capture(&self, cx: &App) -> Saved {
        let list = self.workspaces.list();
        let groups: Vec<&Group> = self.workspaces.groups();
        let index = |id: WorkspaceId| list.iter().position(|w| w.id == id);
        let (width, height, maximized) = self.window_bounds.unwrap_or((1100., 700., false));
        Saved {
            width,
            height,
            maximized,
            sidebar_visible: self.sidebar_visible,
            sidebar_width: self.sidebar_width,
            right_open: self.right_sidebar_open,
            right_dock: self.right_mode == RightMode::Dock,
            right_width: self.right_width,
            selected: self.workspaces.selected_index(),
            groups: groups
                .iter()
                .map(|g| SavedGroup {
                    name: g.name.clone(),
                    collapsed: g.collapsed,
                    pinned: g.pinned,
                    generated_anchor: g.generated_anchor,
                    anchor: index(g.anchor).unwrap_or(0),
                })
                .collect(),
            workspaces: list
                .iter()
                .filter_map(|w| {
                    Some(SavedWorkspace {
                        title: w.custom_title.clone(),
                        auto_title: w.auto_title,
                        group: w.group.and_then(|g| groups.iter().position(|x| x.id == g)),
                        layout: SavedLayout::of(self.views.get(&w.id)?, &self.vault_agents, cx),
                    })
                })
                .collect(),
            dock: self
                .dock
                .as_ref()
                .map(|d| SavedLayout::of(d, &self.vault_agents, cx)),
        }
    }

    /// Writes `session.json` if it differs from the last write. Errors go to stderr.
    pub(super) fn save_session(&mut self, cx: &App) {
        let Some(path) = path() else { return };
        let text = to_json(&self.capture(cx)).to_string();
        if self.last_saved.as_deref() == Some(text.as_str()) {
            return;
        }
        match write_atomic(&path, &text) {
            Ok(()) => self.last_saved = Some(text),
            Err(e) => eprintln!("cmux-linux: could not save {}: {e}", path.display()),
        }
    }

    /// The 8 s save loop, plus a save on quit and when the window goes away.
    pub(super) fn start_session_saving(&mut self, cx: &mut Context<Self>) {
        let loop_task: Task<()> = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(SAVE_EVERY).await;
                if this.update(cx, |this, cx| this.save_session(cx)).is_err() {
                    break;
                }
            }
        });
        self._tasks.push(loop_task);
        self._session_subs = vec![
            cx.on_app_quit(|this, cx| {
                this.save_session(cx);
                async {}
            }),
            cx.on_release(|this, cx| this.save_session(cx)),
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Saved {
        let layout = SavedLayout {
            shape: Shape::Split {
                orientation: Orientation::Vertical,
                ratio: 0.25,
                first: Box::new(Shape::Pane {
                    tabs: 2,
                    selected: 1,
                }),
                second: Box::new(Shape::Pane {
                    tabs: 1,
                    selected: 0,
                }),
            },
            focused: 1,
            terms: vec![
                SavedTerm {
                    cwd: Some("/tmp".into()),
                    agent: Some(Agent {
                        kind: "claude".into(),
                        session_id: "abc-123".into(),
                        executable: None,
                    }),
                },
                SavedTerm {
                    cwd: None,
                    agent: None,
                },
                SavedTerm {
                    cwd: Some("/".into()),
                    agent: None,
                },
            ],
        };
        Saved {
            width: 1200.,
            height: 800.,
            maximized: true,
            sidebar_visible: false,
            sidebar_width: 300.,
            right_open: true,
            right_dock: true,
            right_width: 320.,
            selected: 1,
            groups: vec![SavedGroup {
                name: "g".into(),
                collapsed: true,
                pinned: false,
                generated_anchor: false,
                anchor: 1,
            }],
            workspaces: vec![
                SavedWorkspace {
                    title: Some("a".into()),
                    auto_title: true,
                    group: None,
                    layout: layout.clone(),
                },
                SavedWorkspace {
                    title: None,
                    auto_title: false,
                    group: Some(0),
                    layout: layout.clone(),
                },
            ],
            dock: Some(layout),
        }
    }

    #[test]
    fn json_round_trips() {
        let s = sample();
        assert_eq!(from_json(&to_json(&s)), Some(s));
    }

    #[test]
    fn other_versions_and_bad_shapes_are_rejected() {
        let mut v = to_json(&sample());
        v["version"] = json!(2);
        assert_eq!(from_json(&v), None);
        let mut v = to_json(&sample());
        v["workspaces"][0]["layout"] = json!({"split": {"orientation": "diagonal"}});
        assert_eq!(from_json(&v), None);
    }

    #[test]
    fn a_bad_agent_is_dropped_and_resume_lines_follow_agents() {
        let mut v = to_json(&sample());
        v["workspaces"][0]["terminals"][0]["agent"]["session_id"] = json!("x; rm -rf ~");
        let s = from_json(&v).unwrap();
        assert_eq!(s.workspaces[0].layout.terms[0].agent, None);
        let inputs = sample().workspaces[0].layout.inputs(&[]);
        assert_eq!(
            inputs,
            vec![Some("claude --resume abc-123".to_string()), None, None]
        );
    }

    #[test]
    fn workspaces_rebuild_with_groups() {
        let w = workspaces_from(&sample());
        assert_eq!(w.len(), 2);
        assert_eq!(w.selected(), WorkspaceId(2));
        let g = w.groups();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].anchor, WorkspaceId(2));
        assert!(g[0].collapsed);
        assert_eq!(w.list()[0].custom_title.as_deref(), Some("a"));
        assert!(w.list()[0].auto_title);
    }

    #[test]
    fn atomic_write_replaces_the_file() {
        let dir = std::env::temp_dir().join(format!("cmux-session-{}", std::process::id()));
        let p = dir.join("session.json");
        write_atomic(&p, "1").unwrap();
        write_atomic(&p, "2").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "2");
        assert!(!p.with_extension("json.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
