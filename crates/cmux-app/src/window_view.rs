//! The window's root view: the workspace list and one `WorkspaceView` per workspace. Only the
//! selected workspace is rendered; the others keep their shells running (plan D1, D6).

use std::collections::HashMap;
use std::rc::Rc;

use cmux_core::{WorkspaceClosed, WorkspaceId, Workspaces};
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, PromptLevel, Render, Styled, Subscription, Window, div, px,
};

use crate::actions::*;
use crate::style::TermStyle;
use crate::workspace::{WorkspaceEvent, WorkspaceView};

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
}

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
        };
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
        let id = if at_end {
            self.workspaces.add_at_end()
        } else {
            self.workspaces.add_after_selected()
        };
        if self.make_view(id, cwd, window, cx) {
            self.select(id, window, cx);
        } else {
            // The model selected the new entry; closing it returns to the previous one.
            let prev = self.workspaces.selected();
            self.workspaces.close(id);
            self.select(prev, window, cx);
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
