//! Inline workspace rename (plan D1): the key-to-text state machine and the input view.

use cmux_core::WorkspaceId;
use gpui::{
    Context, EventEmitter, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Render, Rgba, StatefulInteractiveElement, Styled, Subscription, Window, div,
    prelude::*, px,
};

use crate::actions::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenameKey<'a> {
    Char(&'a str),
    Backspace,
    Enter,
    Escape,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenameOutcome {
    Editing,
    Commit(String),
    Cancel,
}

pub(crate) struct RenameState {
    pub text: String,
    /// The prefill is "selected": the next edit replaces it whole.
    pub replace_on_type: bool,
}

impl RenameState {
    pub fn new(current: &str) -> Self {
        Self {
            text: current.to_string(),
            replace_on_type: true,
        }
    }

    pub fn apply(&mut self, key: RenameKey) -> RenameOutcome {
        match key {
            RenameKey::Char(c) => {
                if std::mem::take(&mut self.replace_on_type) {
                    self.text.clear();
                }
                self.text.push_str(c);
            }
            RenameKey::Backspace => {
                if std::mem::take(&mut self.replace_on_type) {
                    self.text.clear();
                } else {
                    self.text.pop();
                }
            }
            RenameKey::Enter => return RenameOutcome::Commit(self.text.clone()),
            RenameKey::Escape => return RenameOutcome::Cancel,
            RenameKey::Other => {}
        }
        RenameOutcome::Editing
    }
}

pub(crate) enum RenameEvent {
    /// `None` = cancelled.
    Done(WorkspaceId, Option<String>),
}

/// Look of the title slot the input replaces.
pub(crate) struct RenameStyle {
    pub size: f32,
    pub text: Rgba,
    pub hint: Rgba,
}

pub(crate) struct RenameInput {
    id: WorkspaceId,
    state: RenameState,
    focus: FocusHandle,
    style: RenameStyle,
    done: bool,
    _blur: Subscription,
}

impl EventEmitter<RenameEvent> for RenameInput {}

impl RenameInput {
    pub fn new(
        id: WorkspaceId,
        current: &str,
        style: RenameStyle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);
        let _blur = cx.on_blur(&focus, window, |this, window, cx| {
            // Alt-tab clears focus too; a half-typed name must survive a window switch.
            if !window.is_window_active() {
                return;
            }
            let text = this.state.text.clone();
            this.finish(Some(text), cx);
        });
        Self {
            id,
            state: RenameState::new(current),
            focus,
            style,
            done: false,
            _blur,
        }
    }

    pub fn id(&self) -> WorkspaceId {
        self.id
    }

    pub fn snapshot(&self) -> (WorkspaceId, String) {
        (self.id, self.state.text.clone())
    }

    fn finish(&mut self, text: Option<String>, cx: &mut Context<Self>) {
        if !std::mem::replace(&mut self.done, true) {
            cx.emit(RenameEvent::Done(self.id, text));
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let key = match ks.key.as_str() {
            "backspace" => RenameKey::Backspace,
            "enter" => RenameKey::Enter,
            "escape" => RenameKey::Escape,
            _ => match &ks.key_char {
                Some(c) if !ks.modifiers.control && !ks.modifiers.platform => RenameKey::Char(c),
                _ => RenameKey::Other,
            },
        };
        match self.state.apply(key) {
            RenameOutcome::Editing => cx.notify(),
            RenameOutcome::Commit(t) => self.finish(Some(t), cx),
            RenameOutcome::Cancel => self.finish(None, cx),
        }
    }
}

impl Render for RenameInput {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let st = &self.style;
        let empty = self.state.text.is_empty();
        let selected = self.state.replace_on_type;
        // Key bindings fire before key-down listeners, so the workspace-level ones are swallowed
        // here while typing.
        div()
            .id("ws-rename")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                this.on_key(ev, cx);
                cx.stop_propagation();
            }))
            .on_action(|_: &NewWorkspace, _, cx| cx.stop_propagation())
            .on_action(|_: &CloseWorkspace, _, cx| cx.stop_propagation())
            .on_action(|_: &NextWorkspace, _, cx| cx.stop_propagation())
            .on_action(|_: &PrevWorkspace, _, cx| cx.stop_propagation())
            .on_action(|_: &SelectWorkspaceByNumber, _, cx| cx.stop_propagation())
            .on_action(|_: &MoveWorkspaceUp, _, cx| cx.stop_propagation())
            .on_action(|_: &MoveWorkspaceDown, _, cx| cx.stop_propagation())
            .on_action(|_: &ToggleSidebar, _, cx| cx.stop_propagation())
            // Clicks inside must not reach the row (selecting it would blur the input).
            .on_click(|_, _, cx| cx.stop_propagation())
            .flex()
            .flex_row()
            .items_center()
            .flex_1()
            .min_w(px(0.))
            .overflow_hidden()
            .text_size(px(st.size))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(st.text)
            .whitespace_nowrap()
            .child(if empty {
                div()
                    .text_color(st.hint)
                    .child("Workspace name")
                    .into_any_element()
            } else {
                div()
                    .when(selected, |d| d.bg(Rgba { a: 0.3, ..st.text }))
                    .child(self.state.text.clone())
                    .into_any_element()
            })
            .child(div().flex_none().w(px(1.)).h(px(st.size)).bg(st.text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_char_replaces_then_appends() {
        let mut s = RenameState::new("Terminal");
        assert_eq!(s.apply(RenameKey::Char("a")), RenameOutcome::Editing);
        assert_eq!(s.text, "a");
        s.apply(RenameKey::Char("pi"));
        assert_eq!(s.text, "api");
        s.apply(RenameKey::Backspace);
        assert_eq!(s.text, "ap");
        assert_eq!(
            s.apply(RenameKey::Enter),
            RenameOutcome::Commit("ap".into())
        );
    }

    #[test]
    fn backspace_first_clears_and_escape_cancels() {
        let mut s = RenameState::new("é名");
        s.apply(RenameKey::Backspace);
        assert_eq!(s.text, "", "preselected text is deleted whole");
        let mut s = RenameState::new("x");
        s.replace_on_type = false;
        s.text = "é名".into();
        s.apply(RenameKey::Backspace);
        assert_eq!(s.text, "é", "backspace removes one char, not one byte");
        assert_eq!(s.apply(RenameKey::Escape), RenameOutcome::Cancel);
    }
}
