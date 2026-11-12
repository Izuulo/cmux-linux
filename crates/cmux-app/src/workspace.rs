//! The workspace: one cmux-core `Layout` drawn as absolutely positioned panes and dividers,
//! each pane showing its selected tab's terminal. All geometry comes from `Layout::rects`, so
//! layout, focus and divider drag share one maths source (plan D2).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Instant;

use cmux_core::{
    Closed, Direction, DividerRect, Layout, Orientation, PaneId, RESIZE_STEP, Rect, SurfaceId,
    WorkspaceId,
};
use cmux_meta::PanelInput;
use gpui::{
    App, AppContext, Bounds, ClickEvent, Context, CursorStyle, DragMoveEvent, EmptyView, Entity,
    EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Pixels,
    Render, Rgba, StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};

mod tab_strip;

use crate::actions::*;
use crate::chrome;
use crate::style::TermStyle;
use crate::terminal_view::{TerminalEvent, TerminalView};
use crate::window_view::control::ShellEnv;
use crate::window_view::session::{Agent, SavedTerm, detect_agent};
use cmux_config::vault::Registration;

/// Hit band each side of a 1 px divider (cmux `dividerHitExpansion`).
const DIVIDER_HIT: f32 = 8.0;

/// Drag payload for a divider. Plain ids: nothing here keeps a view alive.
#[derive(Debug, Clone, Copy)]
struct DividerDrag {
    split: cmux_core::SplitId,
    orientation: Orientation,
}

struct Surface {
    view: Entity<TerminalView>,
    /// The agent session running here, for resume after a restart (M10 Part 2).
    agent: Option<Agent>,
    _subscriptions: [Subscription; 2],
}

pub struct WorkspaceView {
    id: WorkspaceId,
    shell: Rc<ShellEnv>,
    layout: Layout,
    surfaces: HashMap<SurfaceId, Surface>,
    style: Rc<TermStyle>,
    focus: FocusHandle,
    window_title: String,
    /// Each pane's tab bounds from the last prepaint (window coordinates).
    tab_bounds: Rc<RefCell<HashMap<PaneId, Vec<Bounds<Pixels>>>>>,
    /// Where a dragged tab would land: pane and insertion index.
    drop_target: Option<(PaneId, usize)>,
    /// Width the sidebar takes from the left of the window.
    left_inset: f32,
    /// Width the right sidebar takes from the right.
    right_inset: f32,
    /// Height above the view (the Dock sits under the right sidebar's mode bar).
    top_inset: f32,
    /// This is the window's Dock (M8 plan D13), not a listed workspace: it leaves the window
    /// title alone, never pulls lost focus back, and its shells carry the window's id.
    dock: bool,
    /// Only the active workspace pulls focus back after a focus loss.
    active: bool,
    has_terminals: bool,
    /// Surfaces whose pane shows the notification ring (pushed by the window).
    attention: HashSet<SurfaceId>,
    /// Running flashes: token (animation id) and start.
    flashes: HashMap<SurfaceId, (u64, Instant)>,
    flash_seq: u64,
    _subscriptions: Vec<Subscription>,
}

/// Why `split_surface` made nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SplitFailed {
    NoSpace,
    Spawn,
}

/// What the owner of this workspace needs to hear about.
pub enum WorkspaceEvent {
    /// The last surface closed.
    Empty,
    /// Something shown in the sidebar (a title) changed.
    Changed,
    /// A terminal posted a notification (OSC 9/99/777).
    Notify {
        surface: SurfaceId,
        title: String,
        body: String,
    },
    /// A terminal gained keyboard focus.
    Focused(SurfaceId),
    /// The user typed or pasted into a terminal.
    Typed(SurfaceId),
    /// A surface closed (sent before `Empty` for the last one).
    Closed(SurfaceId),
}

impl EventEmitter<WorkspaceEvent> for WorkspaceView {}

impl WorkspaceView {
    pub(crate) fn new(
        id: WorkspaceId,
        shell: Rc<ShellEnv>,
        style: Rc<TermStyle>,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::build(
            id,
            false,
            shell,
            style,
            Layout::new(),
            vec![cwd],
            window,
            cx,
        )
    }

    /// A workspace (or, with `dock`, the Dock) rebuilt from `session.json` (M10 plan L5, L6,
    /// V3): `terms` and `inputs` are per surface in `Layout::surfaces` order; each input line is
    /// typed into its new shell. Surfaces whose shell fails are dropped; `has_terminals` is
    /// false when none started.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn restore(
        id: WorkspaceId,
        dock: bool,
        shell: Rc<ShellEnv>,
        style: Rc<TermStyle>,
        layout: Layout,
        terms: Vec<SavedTerm>,
        inputs: Vec<Option<String>>,
        top_inset: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let ids = layout.surfaces();
        let cwds = terms.iter().map(|t| t.cwd.clone()).collect();
        let mut this = Self::build(id, dock, shell, style, layout, cwds, window, cx);
        this.top_inset = top_inset;
        for ((id, term), input) in ids.iter().zip(terms).zip(inputs) {
            let Some(s) = this.surfaces.get_mut(id) else {
                continue;
            };
            s.agent = term.agent;
            if let Some(line) = input {
                let bytes = format!("{line}\n").into_bytes();
                s.view.update(cx, |v, cx| v.send_scripted(bytes, cx));
            }
        }
        this
    }

    /// The tree's shape, the focused pane (tree order) and each surface's cwd and agent in
    /// `Layout::surfaces` order (M10 plan L3, V1, V2). A surface without a hook-reported agent
    /// is checked against `regs` by its foreground process.
    pub(crate) fn saved(
        &self,
        regs: &[Registration],
        cx: &App,
    ) -> (cmux_core::Shape, usize, Vec<SavedTerm>) {
        let (shape, focused) = self.layout.shape();
        let terms = self
            .layout
            .surfaces()
            .iter()
            .filter_map(|s| self.surfaces.get(s))
            .map(|s| {
                let v = s.view.read(cx);
                let agent = s
                    .agent
                    .clone()
                    .or_else(|| detect_agent(regs, v.foreground_pid()?));
                SavedTerm {
                    cwd: v.cwd(),
                    agent,
                }
            })
            .collect();
        (shape, focused, terms)
    }

    /// Sets (or with `None` clears) the agent session of `surface`. False if it is not here.
    pub(crate) fn set_agent(&mut self, surface: SurfaceId, agent: Option<Agent>) -> bool {
        match self.surfaces.get_mut(&surface) {
            Some(s) => {
                s.agent = agent;
                true
            }
            None => false,
        }
    }

    pub(crate) fn agent(&self, surface: SurfaceId) -> Option<&Agent> {
        self.surfaces.get(&surface)?.agent.as_ref()
    }

    /// The window's Dock: the same panes, tabs and splits under the right sidebar's bar.
    pub(crate) fn new_dock(
        shell: Rc<ShellEnv>,
        style: Rc<TermStyle>,
        cwd: Option<String>,
        top_inset: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // No listed workspace has id 0.
        let mut this = Self::build(
            WorkspaceId(0),
            true,
            shell,
            style,
            Layout::new(),
            vec![cwd],
            window,
            cx,
        );
        this.top_inset = top_inset;
        this
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        id: WorkspaceId,
        dock: bool,
        shell: Rc<ShellEnv>,
        style: Rc<TermStyle>,
        layout: Layout,
        cwds: Vec<Option<String>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Nothing focused (a closed tab's handle is gone) means keys reach only the root:
        // put focus back on the active terminal (research-gpui.md §1.4).
        let lost = cx.on_focus_lost(window, |this, window, cx| {
            if this.active && !this.dock {
                this.focus_active(window, cx);
            }
        });
        let mut this = Self {
            id,
            shell,
            layout,
            surfaces: HashMap::new(),
            style,
            focus: cx.focus_handle(),
            window_title: String::new(),
            tab_bounds: Rc::default(),
            drop_target: None,
            left_inset: 0.0,
            right_inset: 0.0,
            top_inset: 0.0,
            dock,
            active: true,
            has_terminals: true,
            attention: HashSet::new(),
            flashes: HashMap::new(),
            flash_seq: 0,
            _subscriptions: vec![lost],
        };
        let mut cwds = cwds.into_iter();
        for surface in this.layout.surfaces() {
            let cwd = cwds.next().flatten();
            if !this.add_terminal(surface, cwd, window, cx)
                && this.layout.close_surface(surface) == Closed::LastSurface
            {
                // The last one left failed: nothing started at all.
                this.has_terminals = false;
                return this;
            }
        }
        this.focus_active(window, cx);
        this
    }

    /// False if the first shell failed to start.
    pub fn has_terminals(&self) -> bool {
        self.has_terminals
    }

    pub fn set_insets(&mut self, left: f32, right: f32) {
        self.left_inset = left;
        self.right_inset = right;
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        if active {
            // Another workspace may have set the window title meanwhile; force a resend.
            self.window_title.clear();
        }
    }

    /// The focused surface's OSC title.
    pub fn title(&self, cx: &App) -> Option<String> {
        let s = self.surfaces.get(&self.layout.active_surface())?;
        s.view.read(cx).title().map(str::to_string)
    }

    pub fn active_cwd(&self, cx: &App) -> Option<String> {
        self.surfaces
            .get(&self.layout.active_surface())?
            .view
            .read(cx)
            .cwd()
    }

    /// One entry per surface, in `layout.surfaces()` order.
    pub fn panels(&self, cx: &App) -> Vec<PanelInput> {
        self.layout
            .surfaces()
            .iter()
            .filter_map(|id| self.surfaces.get(id))
            .map(|s| {
                let v = s.view.read(cx);
                PanelInput {
                    cwd: v.osc_cwd(),
                    pid: v.pid(),
                }
            })
            .collect()
    }

    /// Whether any surface has a foreground job running.
    pub fn busy(&self, cx: &App) -> bool {
        self.surfaces.values().any(|s| s.view.read(cx).busy())
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn terminal(&self, surface: SurfaceId) -> Option<&Entity<TerminalView>> {
        self.surfaces.get(&surface).map(|s| &s.view)
    }

    /// Focuses `pane` (the socket's `pane.focus`). False if it is gone.
    pub fn focus_pane(
        &mut self,
        pane: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.layout.focus_pane(pane) {
            return false;
        }
        self.focus_active(window, cx);
        true
    }

    /// Selects `surface`'s tab and focuses it (a jump). False if it is gone.
    pub fn focus_surface(
        &mut self,
        surface: SurfaceId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.surfaces.contains_key(&surface) || !self.layout.select_surface(surface) {
            return false;
        }
        self.focus_active(window, cx);
        true
    }

    /// The notification ring, flash and tab dot colour: cmux's accent for this background.
    pub(crate) fn attention_color(&self) -> Rgba {
        let cfg = &self.style.cfg;
        chrome::cmux_accent(chrome::wcag_is_light(chrome::rgba(cfg.background, 1.0)))
    }

    /// The surfaces whose pane shows the notification ring.
    pub fn set_attention(&mut self, ring: HashSet<SurfaceId>, cx: &mut Context<Self>) {
        if self.attention != ring {
            self.attention = ring;
            cx.notify();
        }
    }

    /// Starts the arrival/dismissal flash on `surface` (rn §4). A flash already running there
    /// keeps going; a new one is not stacked on it.
    pub fn flash(&mut self, surface: SurfaceId, cx: &mut Context<Self>) {
        let now = Instant::now();
        if self
            .flashes
            .get(&surface)
            .is_some_and(|(_, at)| now.duration_since(*at) < crate::attention::FLASH)
        {
            return;
        }
        self.flash_seq += 1;
        self.flashes.insert(surface, (self.flash_seq, now));
        // Redraw once it has run so the finished overlay goes.
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(crate::attention::FLASH)
                .await;
            let _ = this.update(cx, |this, cx| {
                let now = Instant::now();
                this.flashes
                    .retain(|_, (_, at)| now.duration_since(*at) < crate::attention::FLASH);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn add_terminal(
        &mut self,
        surface: SurfaceId,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let env = if self.dock {
            self.shell.for_dock(surface)
        } else {
            self.shell.for_surface(self.id, surface)
        };
        let started = match TerminalView::start(&self.style, cwd.as_deref(), &env) {
            Ok(started) => started,
            Err(e) => {
                eprintln!("cmux-linux: could not start a shell: {e}");
                return false;
            }
        };
        let style = self.style.clone();
        let view = cx.new(|cx| TerminalView::new(style, cwd, started, window, cx));
        let events = cx.subscribe_in(&view, window, move |this, _, ev, window, cx| match ev {
            TerminalEvent::Exited => this.close_surface(surface, window, cx),
            TerminalEvent::TitleChanged => {
                cx.notify();
                cx.emit(WorkspaceEvent::Changed);
            }
            TerminalEvent::Notify { title, body } => cx.emit(WorkspaceEvent::Notify {
                surface,
                title: title.clone(),
                body: body.clone(),
            }),
            TerminalEvent::Typed => cx.emit(WorkspaceEvent::Typed(surface)),
        });
        let handle = view.read(cx).focus_handle(cx);
        // A click focuses the terminal itself; follow it in the model.
        let focused = cx.on_focus_in(&handle, window, move |this, _, cx| {
            if this.layout.active_surface() != surface && this.layout.select_surface(surface) {
                cx.notify();
            }
            cx.emit(WorkspaceEvent::Focused(surface));
        });
        self.surfaces.insert(
            surface,
            Surface {
                view,
                agent: None,
                _subscriptions: [events, focused],
            },
        );
        true
    }

    /// Starts a shell for a surface the layout just added; on failure removes the surface
    /// again so focus stays where it was.
    fn add_terminal_or_undo(
        &mut self,
        surface: SurfaceId,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.add_terminal(surface, cwd, window, cx) {
            self.layout.close_surface(surface);
        }
        self.focus_active(window, cx);
    }

    /// Focuses the active surface's terminal (after any change to focus or tabs).
    pub fn focus_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // A hidden workspace must never take keyboard focus.
        if let Some(s) = self
            .surfaces
            .get(&self.layout.active_surface())
            .filter(|_| self.active)
        {
            let handle = s.view.read(cx).focus_handle(cx);
            window.focus(&handle);
        }
        cx.notify();
    }

    /// The directory a new terminal starts in: the active terminal's OSC 7 cwd, if allowed.
    fn inherited_cwd(&self, inherit: bool, cx: &Context<Self>) -> Option<String> {
        let s = self.surfaces.get(&self.layout.active_surface())?;
        inherit.then(|| s.view.read(cx).cwd()).flatten()
    }

    fn container(&self, window: &Window) -> (f32, f32, f32) {
        let size = window.viewport_size();
        (
            (f32::from(size.width) - self.left_inset - self.right_inset).max(0.0),
            (f32::from(size.height) - self.top_inset).max(0.0),
            window.scale_factor(),
        )
    }

    pub fn close_surface(
        &mut self,
        surface: SurfaceId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.layout.close_surface(surface) {
            Closed::LastSurface => {
                self.surfaces.remove(&surface);
                cx.emit(WorkspaceEvent::Closed(surface));
                cx.emit(WorkspaceEvent::Empty);
            }
            Closed::NotFound => {}
            Closed::Tab | Closed::Pane => {
                self.surfaces.remove(&surface);
                self.flashes.remove(&surface);
                cx.emit(WorkspaceEvent::Closed(surface));
                self.focus_active(window, cx);
                // The active surface, and so the sidebar title, may have changed.
                cx.emit(WorkspaceEvent::Changed);
            }
        }
    }

    /// The socket's `surface.split` (M5 plan D6): splits `surface`'s pane towards `dir`, moving
    /// focus only when `focus`, and types `input` into the new shell, which starts in `cwd` (else
    /// the source's directory, if the config inherits it). The new surface, or why not.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn split_surface(
        &mut self,
        surface: SurfaceId,
        dir: Direction,
        focus: bool,
        input: Option<&str>,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<SurfaceId, SplitFailed> {
        let (w, h, scale) = self.container(window);
        let cwd = cwd.or_else(|| {
            self.surfaces
                .get(&surface)
                .filter(|_| self.style.cfg.split_inherit_working_directory)
                .and_then(|s| s.view.read(cx).cwd())
        });
        let new = self
            .layout
            .try_split_surface(surface, dir, focus, w, h, scale)
            .map_err(|_| SplitFailed::NoSpace)?;
        if !self.add_terminal(new, cwd, window, cx) {
            self.layout.close_surface(new);
            return Err(SplitFailed::Spawn);
        }
        if let (Some(input), Some(s)) = (input, self.surfaces.get(&new)) {
            s.view
                .update(cx, |v, cx| v.send_scripted(input.as_bytes().to_vec(), cx));
        }
        if focus {
            self.focus_active(window, cx);
        } else {
            cx.notify();
        }
        Ok(new)
    }

    fn split(&mut self, o: Orientation, window: &mut Window, cx: &mut Context<Self>) {
        let (w, h, scale) = self.container(window);
        let cwd = self.inherited_cwd(self.style.cfg.split_inherit_working_directory, cx);
        // shortcut: cmux beeps on a refused split; gpui 0.2.2 has no beep API, so nothing happens.
        if let Ok(surface) = self.layout.try_split(o, w, h, scale) {
            self.add_terminal_or_undo(surface, cwd, window, cx);
        }
    }

    fn new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self.inherited_cwd(self.style.cfg.tab_inherit_working_directory, cx);
        let surface = self.layout.new_tab();
        self.add_terminal_or_undo(surface, cwd, window, cx);
    }

    fn focus_dir(&mut self, dir: Direction, window: &mut Window, cx: &mut Context<Self>) {
        if self.layout.focus_direction(dir) {
            self.focus_active(window, cx);
        }
    }

    fn resize(&mut self, dir: Direction, window: &mut Window, cx: &mut Context<Self>) {
        let (w, h, scale) = self.container(window);
        if self.layout.resize_pane(dir, RESIZE_STEP, w, h, scale) {
            cx.notify();
        }
    }

    fn close_other_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for s in self.layout.close_other_tabs() {
            self.surfaces.remove(&s);
            self.flashes.remove(&s);
            cx.emit(WorkspaceEvent::Closed(s));
        }
        self.focus_active(window, cx);
    }

    fn on_divider_drag(
        &mut self,
        ev: &DragMoveEvent<DividerDrag>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let d = *ev.drag(cx);
        let pos = match d.orientation {
            Orientation::Horizontal => ev.event.position.x - ev.bounds.origin.x,
            Orientation::Vertical => ev.event.position.y - ev.bounds.origin.y,
        };
        let (w, h, scale) = self.container(window);
        // shortcut: the divider jumps to the pointer (at most the 8 px band half) when the drag
        // starts; NSSplitView keeps the grab offset. Store it in DividerDrag if it bothers anyone.
        if self
            .layout
            .drag_divider(d.split, f32::from(pos), w, h, scale)
        {
            cx.notify();
        }
    }

    fn update_window_title(&mut self, window: &mut Window, cx: &Context<Self>) {
        if self.dock {
            return;
        }
        let title = self
            .surfaces
            .get(&self.layout.active_surface())
            .and_then(|s| s.view.read(cx).title().map(str::to_string))
            .unwrap_or_else(|| "Terminal".into());
        if title != self.window_title {
            window.set_window_title(&title);
            self.window_title = title;
        }
    }

    fn render_pane(
        &self,
        pane: PaneId,
        r: Rect,
        dim: Option<Rgba>,
        chrome: PaneChrome,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let pane = self.layout.pane(pane).expect("rects come from the layout");
        let terminal = self
            .surfaces
            .get(&pane.selected_surface())
            .map(|s| s.view.clone());
        let focused = pane.id == self.layout.focused();
        let shown = pane.selected_surface();
        let accent = self.attention_color();
        let ring = self
            .attention
            .contains(&shown)
            .then(|| crate::attention::ring(accent));
        let flash = self
            .flashes
            .get(&shown)
            .map(|(token, _)| crate::attention::flash(accent, *token));
        div()
            .absolute()
            .left(px(r.x))
            .top(px(r.y))
            .w(px(r.w))
            .h(px(r.h))
            .flex()
            .flex_col()
            .overflow_hidden()
            .key_context("Pane")
            // Tab bounds are window-space; the content area starts at left_inset.
            .child(self.render_tab_strip(pane, focused, self.left_inset + r.x, chrome, cx))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h(px(0.))
                    .children(terminal)
                    // No mouse handlers, so no hitbox: clicks go through to the terminal.
                    .children(dim.map(|c| {
                        div()
                            .absolute()
                            .inset_0()
                            .bg(c)
                            .rounded_bl(px(chrome.bl))
                            .rounded_br(px(chrome.br))
                    }))
                    .children(ring)
                    .children(flash),
            )
    }

    fn render_divider(&self, d: DividerRect, color: Rgba, cx: &Context<Self>) -> impl IntoElement {
        let r = d.rect;
        let (band, line, cursor) = match d.orientation {
            Orientation::Horizontal => (
                Rect {
                    x: r.x - DIVIDER_HIT,
                    w: r.w + 2.0 * DIVIDER_HIT,
                    ..r
                },
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(DIVIDER_HIT))
                    .w(px(r.w)),
                CursorStyle::ResizeLeftRight,
            ),
            Orientation::Vertical => (
                Rect {
                    y: r.y - DIVIDER_HIT,
                    h: r.h + 2.0 * DIVIDER_HIT,
                    ..r
                },
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(DIVIDER_HIT))
                    .h(px(r.h)),
                CursorStyle::ResizeUpDown,
            ),
        };
        let drag = DividerDrag {
            split: d.split,
            orientation: d.orientation,
        };
        div()
            .id(("divider", d.split.0 as usize))
            .absolute()
            .left(px(band.x))
            .top(px(band.y))
            .w(px(band.w))
            .h(px(band.h))
            .cursor(cursor)
            // Above the terminals: the band must win the hover test over them.
            .occlude()
            .on_drag(drag, |_, _, _, cx| cx.new(|_| EmptyView))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                // cmux equalizeSplitsOnDividerDoubleClick (default on): the run, not all splits.
                if ev.click_count() == 2 && this.layout.equalize_run(drag.split) {
                    cx.notify();
                }
            }))
            .child(line.bg(color))
    }
}

impl Focusable for WorkspaceView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

/// How a pane gives way to window chrome: its strip's indents for the reopen button and the
/// window buttons, and the radius of each rounded window corner it sits in (else 0).
#[derive(Clone, Copy)]
pub(crate) struct PaneChrome {
    pub lead: f32,
    pub trail: f32,
    pub tl: f32,
    pub tr: f32,
    pub bl: f32,
    pub br: f32,
}

impl Render for WorkspaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.update_window_title(window, cx);
        // A drag released outside every strip leaves no drop target behind.
        if !cx.has_active_drag() {
            self.drop_target = None;
        }
        let (w, h, scale) = self.container(window);
        let rects = self.layout.rects(w, h, scale);
        let focused = self.layout.focused();
        let controls = crate::csd::controls_width(window);
        // No sidebar left of us: the window draws its reopen button in our top-left corner.
        let reopen = match self.left_inset == 0.0 {
            true => crate::csd::REOPEN_LEAD,
            false => 0.0,
        };
        let radius = crate::csd::corner_radius(window);
        let (left, top, right) = (self.left_inset, self.top_inset, self.right_inset);
        let chrome_of = move |r: Rect| {
            let (x, y) = (left + r.x, top + r.y);
            let (right_gap, bottom_gap) = (right + w - r.x - r.w, h - r.y - r.h);
            let at = |a: f32, b: f32| match a.abs() < 1.0 && b.abs() < 1.0 {
                true => radius,
                false => 0.0,
            };
            PaneChrome {
                lead: crate::csd::inset(reopen, x, y),
                trail: crate::csd::inset(controls, right_gap, y),
                tl: at(x, y),
                tr: at(right_gap, y),
                bl: at(x, bottom_gap),
                br: at(right_gap, bottom_gap),
            }
        };
        for (p, r) in &rects.panes {
            let c = chrome_of(*r);
            let view = self
                .layout
                .pane(*p)
                .and_then(|p| self.surfaces.get(&p.selected_surface()));
            if let Some(s) = view {
                s.view.update(cx, |v, _| v.corners = (c.bl, c.br));
            }
        }
        let dim = chrome::dim_overlay(&self.style.cfg).filter(|_| self.layout.pane_count() > 1);
        let divider = chrome::divider(&self.style.cfg);
        let panes: Vec<_> = rects
            .panes
            .iter()
            .map(|(p, r)| {
                self.render_pane(*p, *r, dim.filter(|_| *p != focused), chrome_of(*r), cx)
            })
            .collect();
        let dividers: Vec<_> = rects
            .dividers
            .iter()
            .map(|d| self.render_divider(*d, divider, cx))
            .collect();
        div()
            .id("workspace")
            .relative()
            .size_full()
            // No background here: each terminal paints its own (with opacity); a second layer
            // would make panes more opaque than the divider gaps.
            .key_context("Workspace")
            .track_focus(&self.focus)
            .on_drag_move(cx.listener(Self::on_divider_drag))
            .on_action(
                cx.listener(|this, _: &SplitRight, w, cx| {
                    this.split(Orientation::Horizontal, w, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &SplitDown, w, cx| this.split(Orientation::Vertical, w, cx)),
            )
            .on_action(cx.listener(|this, _: &ToggleSplitZoom, w, cx| {
                if this.layout.toggle_zoom() {
                    this.focus_active(w, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &EqualizeSplits, _, cx| {
                this.layout.equalize_all();
                cx.notify();
            }))
            .on_action(
                cx.listener(|this, _: &ResizePaneLeft, w, cx| this.resize(Direction::Left, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ResizePaneRight, w, cx| {
                    this.resize(Direction::Right, w, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &ResizePaneUp, w, cx| this.resize(Direction::Up, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ResizePaneDown, w, cx| this.resize(Direction::Down, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &FocusLeft, w, cx| this.focus_dir(Direction::Left, w, cx)),
            )
            .on_action(
                cx.listener(|this, _: &FocusRight, w, cx| this.focus_dir(Direction::Right, w, cx)),
            )
            .on_action(cx.listener(|this, _: &FocusUp, w, cx| this.focus_dir(Direction::Up, w, cx)))
            .on_action(
                cx.listener(|this, _: &FocusDown, w, cx| this.focus_dir(Direction::Down, w, cx)),
            )
            .on_action(cx.listener(|this, _: &FocusNextPane, w, cx| {
                if this.layout.focus_cycle(true) {
                    this.focus_active(w, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &FocusPreviousPane, w, cx| {
                if this.layout.focus_cycle(false) {
                    this.focus_active(w, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &NewSurface, w, cx| this.new_tab(w, cx)))
            .on_action(cx.listener(|this, _: &CloseTab, w, cx| {
                let s = this.layout.active_surface();
                this.close_surface(s, w, cx);
            }))
            .on_action(
                cx.listener(|this, _: &CloseOtherTabsInPane, w, cx| this.close_other_tabs(w, cx)),
            )
            .on_action(cx.listener(|this, _: &NextSurface, w, cx| {
                this.layout.cycle_tab(true);
                this.focus_active(w, cx);
            }))
            .on_action(cx.listener(|this, _: &PrevSurface, w, cx| {
                this.layout.cycle_tab(false);
                this.focus_active(w, cx);
            }))
            .on_action(cx.listener(|this, a: &SelectSurfaceByNumber, w, cx| {
                if this.layout.select_tab_number(usize::from(a.0)) {
                    this.focus_active(w, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &MoveSurfaceLeft, _, cx| {
                if this.layout.move_tab(-1) {
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &MoveSurfaceRight, _, cx| {
                if this.layout.move_tab(1) {
                    cx.notify();
                }
            }))
            .children(panes)
            .children(dividers)
    }
}
