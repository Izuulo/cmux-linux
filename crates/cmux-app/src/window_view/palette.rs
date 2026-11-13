//! The command palette (M11 plan P1–P6): `>` lists commands, anything else switches workspaces
//! and the selected workspace's tabs. Rows come from `palette_rows`; `cmux.json` commands are
//! read when it opens (plan C1, C3).

use crate::ui::{px, s};
use cmux_config::commands::sanitize;
use cmux_config::keybindings::Binding;
use cmux_core::{SurfaceId, WorkspaceId};
use gpui::{
    ClickEvent, Context, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Subscription,
    Window, div, point, prelude::*,
};
use std::time::Instant;

use super::WindowView;
use super::rename::{RenameKey, RenameOutcome, RenameState};
use crate::actions::{Hints, PALETTE, action_for};
use crate::chrome;
use crate::mac::{self, Theme, metric};
use cmux_config::commands::Loaded;

const MAX_W: f32 = 560.0;
const MIN_W: f32 = 340.0;
const SIDE_ROOM: f32 = 260.0;
const TOP: f32 = 40.0;
const FIELD_PAD_X: f32 = 9.0;
const EMPTY_PAD_Y: f32 = 7.0;
const ROW_PAD_X: f32 = 9.0;
const LIST_MAX_H: f32 = 450.0;

/// What a row does when run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Run {
    /// A config action name from `PALETTE`.
    Action(&'static str),
    Workspace(WorkspaceId),
    Surface(WorkspaceId, SurfaceId),
    /// Index into the palette's `cmux.json` commands.
    Custom(usize),
    /// A `cmux.json` error row.
    Nothing,
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
    custom: Loaded,
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
        if let Some(p) = self.palette.live_mut() {
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
            if window.is_window_active() && this.palette.close(Instant::now()).is_some() {
                cx.notify();
            }
        });
        let custom = self.load_commands(cx);
        self.palette.open(Palette {
            field: field(text),
            selected: 0,
            focus,
            restore,
            scroll: ScrollHandle::new(),
            custom,
            _blur,
        });
        cx.notify();
    }

    /// False if it was not open.
    pub(super) fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(restore) = self
            .palette
            .close(Instant::now())
            .map(|p| p.restore.clone())
        else {
            return false;
        };
        window.focus(&restore);
        cx.notify();
        true
    }

    /// Every row of `p`'s mode, unfiltered, in table order.
    fn palette_rows(&self, p: &Palette, cx: &Context<Self>) -> Vec<Row> {
        if p.commands() {
            let hints = cx.try_global::<Hints>();
            let mut rows: Vec<Row> = PALETTE
                .iter()
                .map(|(action, title)| Row {
                    title: title.to_string(),
                    keywords: vec![action.to_string()],
                    hint: hints.and_then(|h| h.0.get(action).cloned()),
                    run: Run::Action(action),
                })
                .collect();
            rows.extend(p.custom.commands.iter().enumerate().map(|(i, c)| Row {
                title: format!("Custom: {}", sanitize(&c.name)),
                keywords: c.keywords.clone(),
                hint: None,
                run: Run::Custom(i),
            }));
            rows.extend(p.custom.errors.iter().map(|e| Row {
                title: format!("cmux.json: {e}"),
                keywords: Vec::new(),
                hint: None,
                run: Run::Nothing,
            }));
            return rows;
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

    /// The rows `p` shows: matches of its query, best first.
    fn palette_matches(&self, p: &Palette, cx: &Context<Self>) -> Vec<Row> {
        let mut rows: Vec<Option<Row>> = self.palette_rows(p, cx).into_iter().map(Some).collect();
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
        let Some(n) = self
            .palette
            .live()
            .map(|p| self.palette_matches(p, cx).len())
        else {
            return;
        };
        let Some(p) = self.palette.live_mut() else {
            return;
        };
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
        let Some(p) = self.palette.live_mut() else {
            return;
        };
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
        let Some(p) = self.palette.live() else { return };
        let mut rows = self.palette_matches(p, cx);
        // As drawn: the highlight is clamped to the last row.
        let row = (!rows.is_empty()).then(|| rows.swap_remove(ix.min(rows.len() - 1)));
        // Cloned: the palette still shows its rows while it fades out.
        let custom = match row.as_ref().map(|r| r.run) {
            Some(Run::Custom(i)) => p.custom.commands.get(i).cloned(),
            _ => None,
        };
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
            Run::Custom(_) => {
                if let Some(c) = custom {
                    self.run_custom(c, window, cx)
                }
            }
            Run::Nothing => {}
        }
    }

    /// Window width and palette width, window px.
    fn palette_size(window: &Window) -> (f32, f32) {
        let win_w = f32::from(window.viewport_size().width);
        (win_w, s(MAX_W).min(s(MIN_W).max(win_w - s(SIDE_ROOM))))
    }

    pub(super) fn render_palette(
        &self,
        window: &Window,
        cx: &Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let (p, fade) = self.palette.shown()?;
        // Leaving: drawn as it was, without focus or handlers.
        let live = !fade.leaving();
        let t = Theme::new(chrome::rgba(self.style.cfg.background, 1.0));
        let (win_w, w) = Self::palette_size(window);
        let rows = self.palette_matches(p, cx);
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
            .h(px(metric::PALETTE_FIELD_H))
            .px(px(FIELD_PAD_X))
            .text_size(px(metric::BODY_TEXT))
            .text_color(t.text)
            .whitespace_nowrap()
            .overflow_hidden()
            .when(!query.is_empty(), |d| d.child(query))
            .child(
                div()
                    .flex_none()
                    .w(px(1.))
                    .h(px(metric::BODY_TEXT + 2.))
                    .bg(t.text),
            )
            .when(p.field.text.is_empty() || p.field.text == ">", |d| {
                d.child(div().ml(px(2.)).text_color(t.secondary).child(placeholder))
            });
        let list = div()
            .id("palette-list")
            .flex()
            .flex_col()
            .max_h(px(LIST_MAX_H))
            .overflow_y_scroll()
            .track_scroll(&p.scroll)
            .p(px(metric::MENU_INSET))
            .when(rows.is_empty(), |d| {
                d.child(
                    div()
                        .px(px(ROW_PAD_X))
                        .py(px(EMPTY_PAD_Y))
                        .text_size(px(metric::BODY_TEXT))
                        .text_color(t.secondary)
                        .child(empty),
                )
            })
            .children(rows.into_iter().enumerate().map(|(k, row)| {
                let chosen = k == sel;
                div()
                    .id(("palette-row", k))
                    .flex_none()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .h(px(metric::PALETTE_ROW_H))
                    .px(px(ROW_PAD_X))
                    .rounded(px(metric::MENU_ITEM_RADIUS))
                    .text_size(px(metric::BODY_TEXT))
                    .text_color(t.text)
                    .when(chosen, |d| d.bg(t.palette_select))
                    .when(!chosen, |d| d.hover(|s| s.bg(t.hover)))
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
                            .text_size(px(metric::SMALL_TEXT))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.secondary)
                            .child(h)
                    }))
                    .when(live, |d| {
                        d.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.palette_run(k, window, cx);
                        }))
                    })
            }));
        let panel = mac::palette_panel(&t)
            .id("command-palette")
            .relative()
            .flex()
            .flex_col()
            .w(gpui::px(w))
            .when(live, |d| {
                d.key_context("CommandPalette")
                    .track_focus(&p.focus)
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
            })
            .child(field)
            .child(div().flex_none().h(px(1.)).bg(t.separator))
            .child(list);
        Some(mac::overlay(
            "palette",
            point(gpui::px((win_w - w) / 2.), px(TOP)),
            fade,
            Instant::now(),
            panel,
        ))
    }
}
