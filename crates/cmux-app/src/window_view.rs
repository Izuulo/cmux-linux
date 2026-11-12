//! The window's root view: the workspace list and one `WorkspaceView` per workspace. Only the
//! selected workspace is rendered; the others keep their shells running (plan D1, D6).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cmux_core::{
    FeedItemId, FeedStore, GroupId, MessageStore, Notifications, Placement, WorkspaceClosed,
    WorkspaceId, Workspaces,
};
use cmux_meta::pr::{self, PrKey, PrPoller};
use cmux_meta::{WorkspaceInput, WorkspaceMeta};
use gpui::{
    App, AppContext, Bounds, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Pixels, PromptLevel, Render, ScrollHandle, Styled, Subscription,
    Task, Window, div, prelude::*, px,
};

use crate::actions::*;
use crate::style::TermStyle;
use crate::workspace::{WorkspaceEvent, WorkspaceView};

mod automation_ctl;
mod autoname_ctl;
pub(crate) mod control;
mod dock;
mod feed;
mod feed_cards;
mod feed_ctl;
mod group_ctl;
mod groups;
mod menu;
mod message_ctl;
mod notify;
mod panel;
mod rename;
pub mod session;
mod sidebar;
mod vault_ctl;

use rename::RenameInput;

pub const SIDEBAR_DEFAULT: f32 = 240.0;
pub const SIDEBAR_MIN: f32 = 240.0;
pub const SIDEBAR_MAX: f32 = 600.0;
pub const CONTENT_MIN: f32 = 300.0;

pub struct WindowView {
    workspaces: Workspaces,
    views: HashMap<WorkspaceId, Entity<WorkspaceView>>,
    style: Rc<TermStyle>,
    sidebar_visible: bool,
    /// The requested width; drawn clamped to the window (`sidebar::clamp_width`).
    sidebar_width: f32,
    sidebar_scroll: ScrollHandle,
    rename: Option<Entity<RenameInput>>,
    /// The open rename edits this group's name, not a workspace title (M8 plan D9).
    rename_group: Option<GroupId>,
    /// The sidebar's context menu, while open.
    menu: Option<menu::Menu>,
    /// Workspaces something was typed into (the "untouched anchor" rule, plan D5).
    typed: HashSet<WorkspaceId>,
    /// Row bounds in list order, from the last paint (drop-gap maths).
    row_bounds: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    drop_gap: Option<usize>,
    dragging: Option<WorkspaceId>,
    focus: FocusHandle,
    subs: HashMap<WorkspaceId, Subscription>,
    meta: HashMap<WorkspaceId, WorkspaceMeta>,
    /// Each workspace's OSC title, so render never reads (and so never tracks) hidden views.
    titles: HashMap<WorkspaceId, Option<String>>,
    prs: PrPoller,
    pr_in_flight: bool,
    notifications: Notifications,
    /// Agent prompts (M7) and the `feed.push` requests still waiting on them, by request id.
    feed: FeedStore,
    feed_waiters: HashMap<String, feed_ctl::Waiter>,
    /// Agent-to-agent messages (M9), claimed by the recipients' hooks.
    messages: MessageStore,
    /// Auto-naming's throttle state per workspace (M9 Part 2).
    naming: HashMap<WorkspaceId, autoname_ctl::Naming>,
    /// Automation rules (M9 Part 3) and whether one of their notifications is being made.
    automations: cmux_ipc::automation::Rules,
    automating: bool,
    window_handle: gpui::AnyWindowHandle,
    /// The right sidebar (plan M7 D7, M8 D12): closed at launch.
    right_sidebar_open: bool,
    right_mode: dock::RightMode,
    /// The requested width; drawn clamped to the window (`dock::clamp_right_width`).
    right_width: f32,
    /// The Dock (M8 plan D13), once it has a terminal; `None` shows the empty pane.
    dock: Option<Entity<WorkspaceView>>,
    _dock_sub: Option<Subscription>,
    /// `dock.json` was looked at (it only seeds the first showing).
    dock_seeded: bool,
    feed_focus: FocusHandle,
    /// The card j/k moved to; Enter jumps to its surface.
    feed_selected: Option<FeedItemId>,
    feed_scroll: ScrollHandle,
    /// Plan feedback text and question picks of pending cards (D8), dropped when a card ends.
    feed_feedback: HashMap<FeedItemId, rename::RenameState>,
    feed_picks: HashMap<FeedItemId, feed_cards::QuestionPicks>,
    /// The plan card whose feedback field holds (or last held) `feed_field_focus`.
    feed_editing: Option<FeedItemId>,
    feed_field_focus: FocusHandle,
    /// Handles and env vars for new shells; shared with every `WorkspaceView`.
    shell: Rc<control::ShellEnv>,
    /// The control socket (M5), while it runs.
    _server: Option<cmux_ipc::server::Server>,
    /// The notification panel is showing (Super+I, the bell).
    panel_open: bool,
    /// When a click outside last closed the panel (panel.rs REOPEN_GUARD).
    panel_dismissed: Option<Instant>,
    _tasks: Vec<Task<()>>,
    _activation: Option<Subscription>,
    /// The notification server, once connected (Linux; plan M4 D9).
    #[cfg(target_os = "linux")]
    desktop: Option<Rc<crate::desktop_notify::DesktopNotifier>>,
    #[cfg(target_os = "linux")]
    desktop_ids: crate::desktop_notify::DesktopIds,
    /// Whole-window KWin blur, dropped with the view. gpui may destroy the surface first; the
    /// effect is then inert and its destroy is still allowed (blur.rs).
    #[cfg(target_os = "linux")]
    _blur: Option<crate::blur::WindowBlur>,
    /// Width, height and maximized, from the last render (M10 `session.json`).
    window_bounds: Option<(f32, f32, bool)>,
    /// The last text written to `session.json`.
    last_saved: Option<String>,
    _session_subs: Vec<Subscription>,
    /// `[[vault.agents]]` from `config.toml`, read at start (M10 plan V2).
    vault_agents: Vec<cmux_config::vault::Registration>,
}

/// Metadata rescan period (plan D2) and PR loop tick (D5).
const META_EVERY: Duration = Duration::from_millis(1500);
const PR_EVERY: Duration = Duration::from_millis(2000);
const PR_FETCHES_PER_TICK: usize = 3;
const GH_DEADLINE: Duration = Duration::from_secs(15);

impl WindowView {
    /// Quits if the first shell fails to start. `saved`: the session to restore (M10).
    pub fn new(
        style: Rc<TermStyle>,
        saved: Option<session::Saved>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Blur only when Ghostty asks for it and there is something to see through (plan D4).
        #[cfg(target_os = "linux")]
        let blur = if style.cfg.background_blur && style.cfg.background_opacity < 1.0 {
            crate::blur::WindowBlur::enable(window, cx)
        } else {
            None
        };
        // Before the first shell starts: its env names the socket.
        let (server, requests) = control::start_server().unzip();
        let shell = Rc::new(control::ShellEnv::new(
            server.as_ref().map(|s| s.path().to_path_buf()),
        ));
        let mut this = Self {
            workspaces: Workspaces::new(),
            views: HashMap::new(),
            style,
            sidebar_visible: true,
            sidebar_width: crate::ui::s(SIDEBAR_DEFAULT),
            sidebar_scroll: ScrollHandle::new(),
            rename: None,
            rename_group: None,
            menu: None,
            typed: HashSet::new(),
            row_bounds: Rc::default(),
            drop_gap: None,
            dragging: None,
            focus: cx.focus_handle(),
            subs: HashMap::new(),
            meta: HashMap::new(),
            titles: HashMap::new(),
            prs: PrPoller::new(),
            pr_in_flight: false,
            notifications: Notifications::new(),
            feed: FeedStore::default(),
            feed_waiters: HashMap::new(),
            messages: MessageStore::default(),
            naming: HashMap::new(),
            automations: automation_ctl::load(),
            automating: false,
            window_handle: window.window_handle(),
            right_sidebar_open: false,
            right_mode: Default::default(),
            right_width: crate::ui::s(dock::RIGHT_SIDEBAR_MIN),
            dock: None,
            _dock_sub: None,
            dock_seeded: false,
            feed_focus: cx.focus_handle(),
            feed_selected: None,
            feed_scroll: ScrollHandle::new(),
            feed_feedback: HashMap::new(),
            feed_picks: HashMap::new(),
            feed_editing: None,
            feed_field_focus: cx.focus_handle(),
            shell,
            _server: server,
            panel_open: false,
            panel_dismissed: None,
            _tasks: Vec::new(),
            _activation: None,
            #[cfg(target_os = "linux")]
            desktop: None,
            #[cfg(target_os = "linux")]
            desktop_ids: Default::default(),
            #[cfg(target_os = "linux")]
            _blur: blur,
            window_bounds: None,
            last_saved: None,
            _session_subs: Vec::new(),
            vault_agents: vault_ctl::load(),
        };
        this._tasks = vec![Self::meta_loop(cx), Self::pr_loop(cx)];
        if let Some(mut requests) = requests {
            this._tasks.push(cx.spawn_in(window, async move |this, cx| {
                use futures::StreamExt;
                while let Some(req) = requests.next().await {
                    if req.method == "feed.push" {
                        // Held until answered (plan M7 D5).
                        let held =
                            this.update_in(cx, |this, window, cx| this.feed_push(req, window, cx));
                        if held.is_err() {
                            return;
                        }
                        continue;
                    }
                    let reply = this.update_in(cx, |this, window, cx| {
                        this.control(&req.method, &req.params, window, cx)
                    });
                    // The view is gone: dropping `req` fails the client with internal_error.
                    let Ok(reply) = reply else { return };
                    req.respond(reply);
                }
            }));
        }
        #[cfg(target_os = "linux")]
        this._tasks.push(Self::desktop_loop(window, cx));
        // Coming back to the window reads what is in front of the user (rn §3).
        this._activation = Some(cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.read_selected(window, cx);
            }
        }));
        if !saved.is_some_and(|s| this.restore(s, window, cx)) {
            this.workspaces = Workspaces::new();
            let id = this.workspaces.selected();
            if !this.make_view(id, None, window, cx) {
                cx.quit();
            }
        }
        this.start_session_saving(cx);
        this
    }

    /// Rebuilds the saved workspaces, groups, sidebars and Dock. False (nothing kept) when no
    /// workspace's shell started.
    fn restore(
        &mut self,
        saved: session::Saved,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.workspaces = session::workspaces_from(&saved);
        let ids: Vec<WorkspaceId> = self.workspaces.list().iter().map(|w| w.id).collect();
        for (id, ws) in ids.iter().zip(&saved.workspaces) {
            let (style, shell) = (self.style.clone(), self.shell.clone());
            let (layout, terms) = (ws.layout.layout(), ws.layout.terms.clone());
            let inputs = ws.layout.inputs(&self.vault_agents);
            let view = cx.new(|cx| {
                WorkspaceView::restore(
                    *id, false, shell, style, layout, terms, inputs, 0., window, cx,
                )
            });
            if !self.register_view(*id, view, window, cx) && self.workspaces.len() > 1 {
                self.workspaces.close(*id);
            }
        }
        if self.views.is_empty() {
            return false;
        }
        self.sidebar_visible = saved.sidebar_visible;
        self.sidebar_width = saved.sidebar_width;
        self.right_sidebar_open = saved.right_open;
        self.right_mode = if saved.right_dock {
            dock::RightMode::Dock
        } else {
            dock::RightMode::Feed
        };
        self.right_width = saved.right_width;
        if let Some(d) = &saved.dock {
            self.dock_seeded = true;
            self.restore_dock(d, window, cx);
        }
        let sel = self.workspaces.selected();
        self.select(sel, window, cx);
        true
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
        let (style, shell) = (self.style.clone(), self.shell.clone());
        let view = cx.new(|cx| WorkspaceView::new(id, shell, style, cwd, window, cx));
        self.register_view(id, view, window, cx)
    }

    /// Subscribes to and lists a new view; false if its shell failed to start.
    fn register_view(
        &mut self,
        id: WorkspaceId,
        view: Entity<WorkspaceView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !view.read(cx).has_terminals() {
            return false;
        }
        let sub = cx.subscribe_in(&view, window, move |this, _, ev, window, cx| match ev {
            WorkspaceEvent::Empty => this.close_now(id, window, cx),
            WorkspaceEvent::Changed => {
                this.refresh_title(id, cx);
                cx.notify();
            }
            WorkspaceEvent::Notify {
                surface,
                title,
                body,
            } => {
                let (title, body) = (title.clone(), body.clone());
                this.on_notify(id, *surface, title, String::new(), body, true, window, cx);
            }
            WorkspaceEvent::Focused(s) => this.on_focused(id, *s, window, cx),
            WorkspaceEvent::Typed(s) => {
                this.typed.insert(id);
                this.on_typed(id, *s, cx);
            }
            WorkspaceEvent::Closed(s) => {
                this.feed_close_surface(id, *s, cx);
                this.messages.forget(id, Some(*s));
                this.on_closed(id, *s, cx);
            }
        });
        self.subs.insert(id, sub);
        self.views.insert(id, view);
        self.refresh_title(id, cx);
        self.emit("workspace.created", Some(id), None, Default::default(), cx);
        true
    }

    fn refresh_title(&mut self, id: WorkspaceId, cx: &App) {
        if let Some(v) = self.views.get(&id) {
            let t = v.read(cx).title(cx);
            self.titles.insert(id, t);
        }
    }

    /// `at_end`: appended as a plain row. Otherwise next to the selected workspace, inside its
    /// group if it has one (rg §7.2).
    fn new_workspace(&mut self, at_end: bool, window: &mut Window, cx: &mut Context<Self>) {
        let old = self.workspaces.selected();
        if let Some(g) = self.workspaces.group_of(old).filter(|_| !at_end) {
            self.spawn_in_group(g, Placement::AfterCurrent, Some(old), true, window, cx);
            return;
        }
        let cwd = self.views.get(&old).and_then(|v| v.read(cx).active_cwd(cx));
        let id = if at_end {
            self.workspaces.add_at_end()
        } else {
            self.workspaces.add_after_selected()
        };
        self.adopt(id, old, cwd, true, window, cx);
    }

    fn select(&mut self, id: WorkspaceId, window: &mut Window, cx: &mut Context<Self>) {
        self.workspaces.select(id);
        let inset = self.sidebar_px(window).unwrap_or(0.);
        let right = self.right_sidebar_px(window);
        for (wid, view) in &self.views {
            view.update(cx, |v, _| v.set_active(*wid == id));
        }
        // Splits right after the switch need this workspace's width now, not next frame.
        if let Some(view) = self.views.get(&id) {
            view.update(cx, |v, _| v.set_insets(inset, right));
        }
        if let Some(view) = self.views.get(&id) {
            view.update(cx, |v, cx| v.focus_active(window, cx));
        }
        let rows = self.workspaces.rows();
        if let Some(ix) = rows.iter().position(|r| r.workspace() == id) {
            self.sidebar_scroll.scroll_to_item(ix);
        }
        self.read_selected(window, cx);
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
        let was_selected = id == self.workspaces.selected();
        // Before it goes: the event still names it. (Closing the last one quits instead.)
        if self.workspaces.len() > 1 && self.workspaces.index_of(id).is_some() {
            self.emit("workspace.closed", Some(id), None, Default::default(), cx);
        }
        match self.workspaces.close(id) {
            WorkspaceClosed::Last => cx.quit(),
            WorkspaceClosed::Closed => {
                if self.rename.as_ref().is_some_and(|r| r.read(cx).id() == id) {
                    self.rename = None;
                    self.rename_group = None;
                }
                self.typed.remove(&id);
                self.subs.remove(&id);
                self.views.remove(&id);
                self.titles.remove(&id);
                self.feed_close_workspace(id, cx);
                self.messages.forget(id, None);
                self.naming.remove(&id);
                let gone = self.notifications.remove_workspace(id);
                self.after_change(gone, cx);
                if was_selected {
                    let sel = self.workspaces.selected();
                    self.select(sel, window, cx);
                } else {
                    // Selecting would refocus the terminal and blur an open rename.
                    cx.notify();
                }
            }
            WorkspaceClosed::NotFound => {}
        }
    }

    /// Custom title, else the cached OSC title, else "Terminal".
    pub(crate) fn title_of(&self, id: WorkspaceId) -> String {
        self.workspaces
            .list()
            .iter()
            .find(|w| w.id == id)
            .and_then(|w| w.custom_title.clone())
            .or_else(|| self.titles.get(&id).cloned().flatten())
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.note_window_bounds(window);
        if !cx.has_active_drag() {
            self.drop_gap = None;
            self.dragging = None;
        }
        // Only the selected view is drawn, so only it is read here: touching a hidden one would
        // make its every notify redraw this window. Refreshing here catches focus moves, which
        // don't emit Changed.
        self.refresh_title(self.workspaces.selected(), cx);
        let selected = self.views.get(&self.workspaces.selected()).cloned();
        let width = self.sidebar_px(window);
        let right = self.right_sidebar_px(window);
        if let Some(v) = &selected {
            v.update(cx, |v, _| v.set_insets(width.unwrap_or(0.), right));
        }
        self.place_dock(window, cx);
        let radius = crate::csd::corner_radius(window);
        div()
            .relative()
            // Fills hairline gaps between children; a translucent window must not be painted twice.
            .when(self.style.cfg.background_opacity >= 1.0, |d| {
                d.bg(self.style.window_background()).rounded(px(radius))
            })
            .flex()
            .flex_row()
            .size_full()
            .on_drag_move(cx.listener(Self::on_sidebar_drag))
            .on_drag_move(cx.listener(Self::on_right_sidebar_drag))
            .on_action(
                cx.listener(|this, _: &NewWorkspace, w, cx| this.new_workspace(false, w, cx)),
            )
            .on_action(cx.listener(|_, _: &Quit, _, cx| cx.quit()))
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
            .on_action(
                cx.listener(|this, _: &NewWorkspaceGroup, w, cx| this.new_empty_group(w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ToggleWorkspaceGroupCollapsed, w, cx| {
                    match this.workspaces.group_of(this.workspaces.selected()) {
                        Some(g) => this.toggle_group(g, w, cx),
                        // Not ours to consume (rg §10.1): the key goes on to the terminal.
                        None => cx.propagate(),
                    }
                }),
            )
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(
                cx.listener(|this, _: &ToggleRightSidebar, w, cx| this.toggle_right_sidebar(w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &FocusRightSidebar, w, cx| this.show_right(None, w, cx)),
            )
            .on_action(cx.listener(|this, _: &SwitchRightSidebarToFeed, w, cx| {
                this.show_right(Some(dock::RightMode::Feed), w, cx)
            }))
            .on_action(cx.listener(|this, _: &SwitchRightSidebarToDock, w, cx| {
                this.show_right(Some(dock::RightMode::Dock), w, cx)
            }))
            .on_action(cx.listener(|this, _: &ShowNotifications, _, cx| {
                this.panel_open = !this.panel_open;
                cx.notify();
            }))
            // The terminal keeps focus while the panel is up, so Escape is caught here first.
            .capture_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _, cx| {
                if ev.keystroke.key != "escape" {
                    return;
                }
                if this.close_menu(cx) {
                    cx.stop_propagation();
                } else if this.panel_open {
                    this.close_panel(cx);
                    cx.stop_propagation();
                }
            }))
            .on_action(
                cx.listener(|this, _: &JumpToUnread, w, cx| this.jump_to_latest_unread(w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &MarkAllNotificationsRead, _, cx| this.mark_all_read(cx)),
            )
            .on_action(cx.listener(|this, _: &ClearAllNotifications, _, cx| this.clear_all(cx)))
            .on_action(cx.listener(|this, _: &RenameWorkspace, w, cx| {
                let id = this.workspaces.selected();
                this.start_rename(id, w, cx);
            }))
            .when_some(width, |d, w| d.child(self.render_sidebar(w, radius, cx)))
            .child(div().flex_1().min_w(px(0.)).h_full().children(selected))
            .when(self.right_sidebar_open, |d| {
                d.child(self.render_right_sidebar(window, cx))
            })
            .when_some(width, |d, w| d.child(self.render_resize_band(w)))
            .when(self.right_sidebar_open, |d| {
                d.child(self.render_right_resize_band(right))
            })
            .children(self.render_panel(window, cx))
            .children(self.render_menu(cx))
            .when(width.is_none(), |d| d.child(self.render_reopen(cx)))
            .children(crate::csd::controls(window))
            .children(crate::csd::resize_grips(window))
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
