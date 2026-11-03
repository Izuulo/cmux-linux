//! The workspace: one cmux-core `Layout` drawn as absolutely positioned panes and dividers,
//! each pane showing its selected tab's terminal. All geometry comes from `Layout::rects`, so
//! layout, focus and divider drag share one maths source (plan D2).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use cmux_core::{
    Closed, Direction, DividerRect, Layout, Orientation, PaneId, RESIZE_STEP, Rect, SurfaceId,
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
    _subscriptions: [Subscription; 2],
}

pub struct WorkspaceView {
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
    /// Only the active workspace pulls focus back after a focus loss.
    active: bool,
    has_terminals: bool,
    _subscriptions: Vec<Subscription>,
}

/// What the owner of this workspace needs to hear about.
pub enum WorkspaceEvent {
    /// The last surface closed.
    Empty,
    /// Something shown in the sidebar (a title) changed.
    Changed,
}

impl EventEmitter<WorkspaceEvent> for WorkspaceView {}

impl WorkspaceView {
    pub fn new(
        style: Rc<TermStyle>,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Nothing focused (a closed tab's handle is gone) means keys reach only the root:
        // put focus back on the active terminal (research-gpui.md §1.4).
        let lost = cx.on_focus_lost(window, |this, window, cx| {
            if this.active {
                this.focus_active(window, cx);
            }
        });
        let mut this = Self {
            layout: Layout::new(),
            surfaces: HashMap::new(),
            style,
            focus: cx.focus_handle(),
            window_title: String::new(),
            tab_bounds: Rc::default(),
            drop_target: None,
            left_inset: 0.0,
            active: true,
            has_terminals: true,
            _subscriptions: vec![lost],
        };
        let first = this.layout.active_surface();
        if this.add_terminal(first, cwd, window, cx) {
            this.focus_active(window, cx);
        } else {
            this.has_terminals = false;
        }
        this
    }

    /// False if the first shell failed to start.
    pub fn has_terminals(&self) -> bool {
        self.has_terminals
    }

    pub fn set_left_inset(&mut self, inset: f32) {
        self.left_inset = inset;
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
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
                    cwd: v.cwd(),
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

    fn add_terminal(
        &mut self,
        surface: SurfaceId,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let started = match TerminalView::start(&self.style, cwd.as_deref()) {
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
        });
        let handle = view.read(cx).focus_handle(cx);
        // A click focuses the terminal itself; follow it in the model.
        let focused = cx.on_focus_in(&handle, window, move |this, _, cx| {
            if this.layout.active_surface() != surface && this.layout.select_surface(surface) {
                cx.notify();
            }
        });
        self.surfaces.insert(
            surface,
            Surface {
                view,
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
            (f32::from(size.width) - self.left_inset).max(0.0),
            f32::from(size.height),
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
                cx.emit(WorkspaceEvent::Empty);
            }
            Closed::NotFound => {}
            Closed::Tab | Closed::Pane => {
                self.surfaces.remove(&surface);
                self.focus_active(window, cx);
            }
        }
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
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let pane = self.layout.pane(pane).expect("rects come from the layout");
        let terminal = self
            .surfaces
            .get(&pane.selected_surface())
            .map(|s| s.view.clone());
        let focused = pane.id == self.layout.focused();
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
            .child(self.render_tab_strip(pane, focused, self.left_inset + r.x, cx))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h(px(0.))
                    .children(terminal)
                    // No mouse handlers, so no hitbox: clicks go through to the terminal.
                    .children(dim.map(|c| div().absolute().inset_0().bg(c))),
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
        let dim = chrome::dim_overlay(&self.style.cfg).filter(|_| self.layout.pane_count() > 1);
        let divider = chrome::divider(&self.style.cfg);
        let panes: Vec<_> = rects
            .panes
            .iter()
            .map(|(p, r)| self.render_pane(*p, *r, dim.filter(|_| *p != focused), cx))
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
