//! The window's root view: the workspace list and one `WorkspaceView` per workspace. Only the
//! selected workspace is rendered; the others keep their shells running (plan D1, D6).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cmux_core::{WorkspaceClosed, WorkspaceId, Workspaces};
use cmux_meta::pr::{self, PrKey, PrPoller};
use cmux_meta::{WorkspaceInput, WorkspaceMeta};
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, PromptLevel, Render, Styled, Subscription, Task, Window, div, px,
};

use crate::actions::*;
use crate::style::TermStyle;
use crate::workspace::{WorkspaceEvent, WorkspaceView};

#[allow(dead_code)] // The sidebar render (next commits) uses the row model.
mod sidebar;

pub const SIDEBAR_DEFAULT: f32 = 240.0;
pub const SIDEBAR_MIN: f32 = 240.0;
pub const SIDEBAR_MAX: f32 = 600.0;
pub const CONTENT_MIN: f32 = 300.0;

pub struct WindowView {
    workspaces: Workspaces,
    views: HashMap<WorkspaceId, Entity<WorkspaceView>>,
    style: Rc<TermStyle>,
    // Task 9 draws the sidebar from these.
    #[allow(dead_code)]
    sidebar_visible: bool,
    #[allow(dead_code)]
    sidebar_width: f32,
    focus: FocusHandle,
    subs: HashMap<WorkspaceId, Subscription>,
    meta: HashMap<WorkspaceId, WorkspaceMeta>,
    prs: PrPoller,
    pr_in_flight: bool,
    _tasks: Vec<Task<()>>,
}

/// Metadata rescan period (plan D2) and PR loop tick (D5).
const META_EVERY: Duration = Duration::from_millis(1500);
const PR_EVERY: Duration = Duration::from_millis(2000);
const PR_FETCHES_PER_TICK: usize = 3;
const GH_DEADLINE: Duration = Duration::from_secs(15);

impl WindowView {
    /// Quits if the first shell fails to start.
    pub fn new(style: Rc<TermStyle>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            workspaces: Workspaces::new(),
            views: HashMap::new(),
            style,
            sidebar_visible: true,
            sidebar_width: SIDEBAR_DEFAULT,
            focus: cx.focus_handle(),
            subs: HashMap::new(),
            meta: HashMap::new(),
            prs: PrPoller::new(),
            pr_in_flight: false,
            _tasks: Vec::new(),
        };
        this._tasks = vec![Self::meta_loop(cx), Self::pr_loop(cx)];
        let id = this.workspaces.selected();
        if !this.make_view(id, None, window, cx) {
            cx.quit();
        }
        this
    }

    pub fn workspaces(&self) -> &Workspaces {
        &self.workspaces
    }

    /// Creates and registers the view for `id`; false if its shell failed to start.
    fn make_view(
        &mut self,
        id: WorkspaceId,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let style = self.style.clone();
        let view = cx.new(|cx| WorkspaceView::new(style, cwd, window, cx));
        if !view.read(cx).has_terminals() {
            return false;
        }
        let sub = cx.subscribe_in(&view, window, move |this, _, ev, window, cx| match ev {
            WorkspaceEvent::Empty => this.close_now(id, window, cx),
            WorkspaceEvent::Changed => cx.notify(),
        });
        self.subs.insert(id, sub);
        self.views.insert(id, view);
        true
    }

    fn new_workspace(&mut self, at_end: bool, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self
            .views
            .get(&self.workspaces.selected())
            .and_then(|v| v.read(cx).active_cwd(cx));
        let old = self.workspaces.selected();
        let id = if at_end {
            self.workspaces.add_at_end()
        } else {
            self.workspaces.add_after_selected()
        };
        if self.make_view(id, cwd, window, cx) {
            self.select(id, window, cx);
        } else {
            // make_view registered nothing for `id`; drop it from the model and go back.
            self.workspaces.close(id);
            self.select(old, window, cx);
        }
    }

    fn select(&mut self, id: WorkspaceId, window: &mut Window, cx: &mut Context<Self>) {
        self.workspaces.select(id);
        for (wid, view) in &self.views {
            view.update(cx, |v, _| v.set_active(*wid == id));
        }
        if let Some(view) = self.views.get(&id) {
            view.update(cx, |v, cx| v.focus_active(window, cx));
        }
        cx.notify();
    }

    fn request_close(&mut self, id: WorkspaceId, window: &mut Window, cx: &mut Context<Self>) {
        let busy = self.views.get(&id).is_some_and(|v| v.read(cx).busy(cx));
        if !busy {
            return self.close_now(id, window, cx);
        }
        let answer = window.prompt(
            PromptLevel::Warning,
            "Close workspace?",
            Some("This will close the workspace and all of its panels."),
            &["Close", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if answer.await == Ok(0) {
                let _ = this.update_in(cx, |this, window, cx| this.close_now(id, window, cx));
            }
        })
        .detach();
    }

    fn close_now(&mut self, id: WorkspaceId, window: &mut Window, cx: &mut Context<Self>) {
        match self.workspaces.close(id) {
            WorkspaceClosed::Last => cx.quit(),
            WorkspaceClosed::Closed => {
                self.subs.remove(&id);
                self.views.remove(&id);
                let sel = self.workspaces.selected();
                self.select(sel, window, cx);
            }
            WorkspaceClosed::NotFound => {}
        }
    }

    #[allow(dead_code)] // Task 9 (sidebar rows) uses it.
    /// Custom title, else the OSC title, else "Terminal".
    pub(crate) fn title_of(&self, id: WorkspaceId, cx: &App) -> String {
        self.workspaces
            .list()
            .iter()
            .find(|w| w.id == id)
            .and_then(|w| w.custom_title.clone())
            .or_else(|| self.views.get(&id)?.read(cx).title(cx))
            .unwrap_or_else(|| "Terminal".to_string())
    }
}

/// Background metadata (plan D5): inputs are read on the UI thread as plain data, the work
/// runs on the background pool, results come back only if something changed. Both loops end
/// when the view is dropped (its tasks are cancelled) or gone.
impl WindowView {
    fn meta_loop(cx: &mut Context<Self>) -> Task<()> {
        let home = std::env::var("HOME").ok();
        cx.spawn(async move |this, cx| {
            loop {
                let Ok((ids, inputs)) = this.read_with(cx, |s, cx| s.meta_inputs(cx)) else {
                    break;
                };
                let home = home.clone();
                let metas = cx
                    .background_executor()
                    .spawn(async move {
                        cmux_meta::collect(&inputs, Path::new("/proc"), home.as_deref())
                    })
                    .await;
                let new: HashMap<_, _> = ids.into_iter().zip(metas).collect();
                let applied = this.update(cx, |s, cx| {
                    if s.meta != new {
                        s.meta = new;
                        cx.notify();
                    }
                });
                if applied.is_err() {
                    break;
                }
                cx.background_executor().timer(META_EVERY).await;
            }
        })
    }

    fn meta_inputs(&self, cx: &App) -> (Vec<WorkspaceId>, Vec<WorkspaceInput>) {
        self.workspaces
            .list()
            .iter()
            .filter_map(|w| {
                let panels = self.views.get(&w.id)?.read(cx).panels(cx);
                Some((w.id, WorkspaceInput { panels }))
            })
            .unzip()
    }

    fn pr_loop(cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(PR_EVERY).await;
                let Ok(jobs) = this.update(cx, |s, _| s.due_prs()) else {
                    break;
                };
                if jobs.is_empty() {
                    continue;
                }
                let results = cx
                    .background_executor()
                    .spawn(async move {
                        let today = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map_or(0, |d| (d.as_secs() / 86_400) as i64);
                        jobs.into_iter()
                            .map(|(key, dir)| {
                                let r = pr::fetch("gh", &dir, &key.1, GH_DEADLINE)
                                    .map(|items| pr::select_pr(&items, &key.1, today));
                                (key, r)
                            })
                            .collect::<Vec<_>>()
                    })
                    .await;
                let applied = this.update(cx, |s, cx| {
                    let now = Instant::now();
                    for (key, r) in results {
                        s.prs.record(key, r, now);
                    }
                    s.pr_in_flight = false;
                    cx.notify();
                });
                if applied.is_err() {
                    break;
                }
            }
        })
    }

    /// Up to three due (key, directory) lookups, the selected workspace's first; marks them
    /// in flight.
    fn due_prs(&mut self) -> Vec<(PrKey, PathBuf)> {
        if self.prs.disabled() || self.pr_in_flight {
            return Vec::new();
        }
        let selected = self.workspaces.selected();
        let mut keys: Vec<(PrKey, PathBuf, bool)> = Vec::new();
        for (id, meta) in &self.meta {
            for d in &meta.dirs {
                let (Some(git), Some(branch)) = (&d.git_dir, &d.branch) else {
                    continue;
                };
                if pr::skip_branch(branch) {
                    continue;
                }
                let key = (git.clone(), branch.clone());
                let focused = *id == selected;
                match keys.iter_mut().find(|(k, _, _)| *k == key) {
                    Some(entry) => entry.2 |= focused,
                    None => keys.push((key, PathBuf::from(&d.path), focused)),
                }
            }
        }
        keys.sort_by_key(|(_, _, focused)| !focused);
        let now = Instant::now();
        let jobs: Vec<_> = keys
            .into_iter()
            .filter(|(k, _, focused)| self.prs.due(k, *focused, now))
            .take(PR_FETCHES_PER_TICK)
            .map(|(k, dir, _)| (k, dir))
            .collect();
        self.pr_in_flight = !jobs.is_empty();
        jobs
    }
}

impl Focusable for WindowView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for WindowView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.views.get(&self.workspaces.selected()).cloned();
        if let Some(v) = &selected {
            v.update(cx, |v, _| v.set_left_inset(0.));
        }
        div()
            .flex()
            .flex_row()
            .size_full()
            .on_action(
                cx.listener(|this, _: &NewWorkspace, w, cx| this.new_workspace(false, w, cx)),
            )
            .on_action(cx.listener(|this, _: &CloseWorkspace, w, cx| {
                let id = this.workspaces.selected();
                this.request_close(id, w, cx);
            }))
            .on_action(cx.listener(|this, _: &NextWorkspace, w, cx| this.cycle(true, w, cx)))
            .on_action(cx.listener(|this, _: &PrevWorkspace, w, cx| this.cycle(false, w, cx)))
            .on_action(cx.listener(|this, a: &SelectWorkspaceByNumber, w, cx| {
                if this.workspaces.select_number(usize::from(a.0)) {
                    let id = this.workspaces.selected();
                    this.select(id, w, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &MoveWorkspaceUp, _, cx| {
                if this.workspaces.move_selected(-1) {
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &MoveWorkspaceDown, _, cx| {
                if this.workspaces.move_selected(1) {
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|_, _: &ToggleSidebar, _, _| {}))
            .on_action(cx.listener(|_, _: &RenameWorkspace, _, _| {}))
            .child(div().flex_1().min_w(px(0.)).h_full().children(selected))
    }
}

impl WindowView {
    fn cycle(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.cycle(forward) {
            let id = self.workspaces.selected();
            self.select(id, window, cx);
        }
    }
}
