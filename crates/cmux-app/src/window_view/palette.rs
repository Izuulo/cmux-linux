//! The command palette (M11 plan P1–P6): `>` lists commands, anything else switches workspaces
//! and the selected workspace's tabs. Rows come from `rows`; other sources (cmux.json commands)
//! add a `Run` kind and push rows there.

use crate::ui::{px, s};
use cmux_config::keybindings::Binding;
use cmux_core::{SurfaceId, WorkspaceId};
use gpui::{
    BoxShadow, ClickEvent, Context, FocusHandle, FontWeight, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Rgba, ScrollHandle, SharedString, StatefulInteractiveElement,
    Styled, Subscription, Window, anchored, deferred, div, point, prelude::*,
};

use super::WindowView;
use super::rename::{RenameKey, RenameOutcome, RenameState};
use crate::actions::{Hints, PALETTE, action_for};
use crate::chrome;

const MAX_W: f32 = 560.0;
const MIN_W: f32 = 340.0;
const SIDE_ROOM: f32 = 260.0;
const TOP: f32 = 40.0;
const RADIUS: f32 = 8.0;
const FIELD_PX: f32 = 13.0;
const FIELD_PAD_X: f32 = 9.0;
const FIELD_PAD_Y: f32 = 7.0;
const ROW_H: f32 = 24.0;
const ROW_PAD_X: f32 = 9.0;
const ROW_PAD_Y: f32 = 2.0;
const LIST_MAX_H: f32 = 450.0;
const HINT_PX: f32 = 11.0;

/// What a row does when run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Run {
    /// A config action name from `PALETTE`.
    Action(&'static str),
    Workspace(WorkspaceId),
    Surface(WorkspaceId, SurfaceId),
}

pub(super) struct Row {
    pub title: String,
    /// Searched after the title.
    pub keywords: Vec<String>,
    pub hint: Option<String>,
    pub run: Run,
}

pub(super) struct Palette {
    field: RenameState,
    /// Index into the filtered rows.
    selected: usize,
    focus: FocusHandle,
    /// Focus before opening, given back on close.
    restore: FocusHandle,
    scroll: ScrollHandle,
    _blur: Subscription,
}

impl Palette {
    fn commands(&self) -> bool {
        self.field.text.starts_with('>')
    }

    fn query(&self) -> &str {
        self.field
            .text
            .strip_prefix('>')
            .unwrap_or(&self.field.text)
    }
}

/// `>` = command mode.
fn field(text: &str) -> RenameState {
    let mut f = RenameState::new(text, true);
    f.replace_on_type = false;
    f
}

impl WindowView {
    /// The `commandPalette` (`commands`) or `goToWorkspace` binding: opens in that mode,
    /// switches to it, or closes when already in it.
    pub(super) fn toggle_palette(
        &mut self,
        commands: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = if commands { ">" } else { "" };
        if let Some(p) = &mut self.palette {
            if p.commands() == commands {
                self.close_palette(window, cx);
            } else {
                p.field = field(text);
                p.selected = 0;
                cx.notify();
            }
            return;
        }
        let focus = cx.focus_handle();
        // Nothing focused: the root view, so actions dispatched after closing still reach it.
        let restore = window.focused(cx).unwrap_or_else(|| self.focus.clone());
        window.focus(&focus);
        // Focus moved elsewhere (a click, a shortcut that focuses a terminal): close.
        let _blur = cx.on_blur(&focus, window, |this, window, cx| {
            if window.is_window_active() && this.palette.take().is_some() {
                cx.notify();
            }
        });
        self.palette = Some(Palette {
            field: field(text),
            selected: 0,
            focus,
            restore,
            scroll: ScrollHandle::new(),
            _blur,
        });
        cx.notify();
    }

    /// False if it was not open.
    pub(super) fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(p) = self.palette.take() else {
            return false;
        };
        window.focus(&p.restore);
        cx.notify();
        true
    }

    /// Every row of the current mode, unfiltered, in table order.
    fn palette_rows(&self, commands: bool, cx: &Context<Self>) -> Vec<Row> {
        if commands {
            let hints = cx.try_global::<Hints>();
            return PALETTE
                .iter()
                .map(|(action, title)| Row {
                    title: title.to_string(),
                    keywords: vec![action.to_string()],
                    hint: hints.and_then(|h| h.0.get(action).cloned()),
                    run: Run::Action(action),
                })
                .collect();
        }
        let mut rows: Vec<Row> = self
            .workspaces
            .list()
            .iter()
            .map(|w| Row {
                title: self.title_of(w.id),
                keywords: vec!["workspace".into()],
                hint: None,
                run: Run::Workspace(w.id),
            })
            .collect();
        let sel = self.workspaces.selected();
        if let Some(v) = self.views.get(&sel) {
            let v = v.read(cx);
            rows.extend(v.layout().surfaces().into_iter().map(|s| Row {
                title: v.tab_title(s, cx).to_string(),
                keywords: vec!["surface".into(), "tab".into()],
                hint: None,
                run: Run::Surface(sel, s),
            }));
        }
        rows
    }

    /// The rows shown: matches of the query, best first.
    fn palette_matches(&self, cx: &Context<Self>) -> Vec<Row> {
        let Some(p) = &self.palette else {
            return Vec::new();
        };
        let mut rows: Vec<Option<Row>> = self
            .palette_rows(p.commands(), cx)
            .into_iter()
            .map(Some)
            .collect();
        let order = cmux_core::palette::rank(
            p.query(),
            rows.iter().flatten().map(|r| {
                std::iter::once(r.title.as_str())
                    .chain(r.keywords.iter().map(String::as_str))
                    .collect()
            }),
        );
        order.into_iter().filter_map(|i| rows[i].take()).collect()
    }

    fn palette_move(&mut self, delta: isize, cx: &mut Context<Self>) {
        let n = self.palette_matches(cx).len();
        let Some(p) = &mut self.palette else { return };
        p.selected = p
            .selected
            .saturating_add_signed(delta)
            .min(n.saturating_sub(1));
        p.scroll.scroll_to_item(p.selected);
        cx.notify();
    }

    fn palette_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "up" => return self.palette_move(-1, cx),
            "down" => return self.palette_move(1, cx),
            _ => {}
        }
        let Some(p) = &mut self.palette else { return };
        let before = p.field.text.clone();
        match p.field.apply(RenameKey::from(&ev.keystroke)) {
            RenameOutcome::Editing if p.field.text == before => {}
            RenameOutcome::Editing => {
                p.selected = 0;
                p.scroll.scroll_to_item(0);
                cx.notify();
            }
            // Enter: `apply` reports Commit or Cancel by text, which means nothing here.
            _ if ev.keystroke.key == "enter" => {
                let ix = p.selected;
                self.palette_run(ix, window, cx);
            }
            _ => {
                self.close_palette(window, cx);
            }
        }
    }

    /// Closes, gives focus back, then runs the `ix`th shown row (plan P4).
    fn palette_run(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let mut rows = self.palette_matches(cx);
        // As drawn: the highlight is clamped to the last row.
        let row = (!rows.is_empty()).then(|| rows.swap_remove(ix.min(rows.len() - 1)));
        self.close_palette(window, cx);
        let Some(row) = row else { return };
        match row.run {
            Run::Action(action) => {
                let b = Binding {
                    action,
                    keystroke: String::new(),
                    hint: String::new(),
                    number: None,
                    context: None,
                };
                if let Some(a) = action_for(&b) {
                    window.dispatch_action(a, cx);
                }
            }
            Run::Workspace(id) => self.select(id, window, cx),
            Run::Surface(id, s) => {
                if let Some(v) = self.views.get(&id).cloned() {
                    v.update(cx, |v, cx| v.focus_surface(s, window, cx));
                }
            }
        }
    }

    pub(super) fn render_palette(
        &self,
        window: &Window,
        cx: &Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let p = self.palette.as_ref()?;
        let c = chrome::sidebar_palette(chrome::rgba(self.style.cfg.background, 1.0));
        let primary = |a: f32| Rgba { a, ..c.title };
        let win_w = f32::from(window.viewport_size().width);
        let w = s(MAX_W).min(s(MIN_W).max(win_w - s(SIDE_ROOM)));
        let rows = self.palette_matches(cx);
        let sel = p.selected.min(rows.len().saturating_sub(1));
        let commands = p.commands();
        let (placeholder, empty) = if commands {
            ("Type a command", "No commands match your search.")
        } else {
            (
                "Search workspaces and surfaces",
                "No workspaces match your search.",
            )
        };
        let query = p.field.text.clone();
        let field = div()
            .flex()
            .flex_row()
            .items_center()
            .px(px(FIELD_PAD_X))
            .py(px(FIELD_PAD_Y))
            .text_size(px(FIELD_PX))
            .text_color(c.title)
            .whitespace_nowrap()
            .overflow_hidden()
            .when(!query.is_empty(), |d| d.child(query))
            .child(div().flex_none().w(px(1.)).h(px(FIELD_PX + 2.)).bg(c.title))
            .when(p.field.text.is_empty() || p.field.text == ">", |d| {
                d.child(div().ml(px(2.)).text_color(c.detail).child(placeholder))
            });
        let list = div()
            .id("palette-list")
            .flex()
            .flex_col()
            .max_h(px(LIST_MAX_H))
            .overflow_y_scroll()
            .track_scroll(&p.scroll)
            .when(rows.is_empty(), |d| {
                d.child(
                    div()
                        .px(px(ROW_PAD_X))
                        .py(px(FIELD_PAD_Y))
                        .text_size(px(FIELD_PX))
                        .text_color(c.detail)
                        .child(empty),
                )
            })
            .children(rows.into_iter().enumerate().map(|(k, row)| {
                div()
                    .id(("palette-row", k))
                    .flex_none()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .h(px(ROW_H))
                    .px(px(ROW_PAD_X))
                    .py(px(ROW_PAD_Y))
                    .text_size(px(FIELD_PX))
                    .text_color(c.title)
                    .when(k == sel, |d| {
                        d.bg(Rgba {
                            a: 0.12,
                            ..c.accent
                        })
                    })
                    .when(k != sel, |d| d.hover(|s| s.bg(primary(0.04))))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(SharedString::from(row.title)),
                    )
                    .children(row.hint.map(|h| {
                        div()
                            .flex_none()
                            .px(px(4.))
                            .py(px(1.))
                            .rounded(px(4.))
                            .bg(primary(0.08))
                            .text_size(px(HINT_PX))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c.detail)
                            .child(h)
                    }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.palette_run(k, window, cx);
                    }))
            }));
        Some(deferred(
            anchored()
                .position(point(gpui::px((win_w - w) / 2.), px(TOP)))
                .child(
                    div()
                        .id("command-palette")
                        .key_context("CommandPalette")
                        .track_focus(&p.focus)
                        .occlude()
                        .flex()
                        .flex_col()
                        .w(gpui::px(w))
                        .overflow_hidden()
                        .rounded(px(RADIUS))
                        .border_1()
                        .border_color(c.border)
                        .bg(c.bg)
                        .shadow(vec![BoxShadow {
                            color: Rgba {
                                a: 0.24,
                                ..gpui::black().into()
                            }
                            .into(),
                            offset: point(px(0.), px(5.)),
                            blur_radius: px(10.),
                            spread_radius: px(0.),
                        }])
                        .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                            this.palette_key(ev, window, cx);
                            cx.stop_propagation();
                        }))
                        .on_action(cx.listener(
                            |this, _: &crate::actions::CommandPaletteNext, _, cx| {
                                this.palette_move(1, cx)
                            },
                        ))
                        .on_action(cx.listener(
                            |this, _: &crate::actions::CommandPalettePrevious, _, cx| {
                                this.palette_move(-1, cx)
                            },
                        ))
                        .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                            this.close_palette(window, cx);
                        }))
                        .child(field)
                        .child(div().flex_none().h(px(1.)).bg(c.border))
                        .child(list),
                ),
        ))
    }
}
