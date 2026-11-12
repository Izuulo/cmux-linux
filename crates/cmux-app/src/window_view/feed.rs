//! The right sidebar's Feed (plan M7 D7-D10, rf §5): agent prompts as cards, newest first.
//! Permission cards are here; plan and question cards are in `feed_cards.rs` (D8).

use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::px;
use cmux_core::{
    Decision, FeedItem, FeedItemId, FeedKind, FeedStatus, PermissionMode, Preview, age_label,
};
use gpui::{
    AnyElement, ClickEvent, Context, Div, FontWeight, InteractiveElement, IntoElement,
    KeyDownEvent, Keystroke, Modifiers, ParentElement, Rgba, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, rgb, svg,
};

use super::WindowView;
use super::sidebar::clip_message;
use crate::chrome::{self, SidebarPalette};

// rf §5 metrics.
const ROW_PAD_X: f32 = 12.0;
const ROW_PAD_Y: f32 = 10.0;
pub(super) const ROW_GAP: f32 = 10.0;
const DIVIDER_ALPHA: f32 = 0.08;
/// Answered and expired cards (D10).
const DONE_OPACITY: f32 = 0.6;
pub(super) const TITLE_ALPHA: f32 = 0.92;
const EMPTY_ALPHA: f32 = 0.7;
const EMPTY_PAD_X: f32 = 16.0;
const KIND_ICON: f32 = 12.0;
const KIND_BOX: f32 = 14.0;
const CHIP_PX: f32 = 10.0;
const CHIP_RADIUS: f32 = 4.0;
pub(super) const CLAUDE_CHIP_ALPHA: f32 = 0.18;
pub(super) const BADGE_ALPHA: f32 = 0.2;
pub(super) const CODE_PAD: f32 = 10.0;
pub(super) const CODE_RADIUS: f32 = 6.0;
pub(super) const CODE_FILL_ALPHA: f32 = 0.05;
pub(super) const CODE_STROKE_ALPHA: f32 = 0.10;
/// Not from cmux: keeps a huge command or JSON from filling the sidebar.
const CODE_LINES: usize = 12;
const BUTTON_GAP: f32 = 6.0;
pub(super) const BUTTON_RADIUS: f32 = 6.0;
/// The compact FeedButton: four medium buttons (12×5 at 10.5 pt) don't fit in 295.
const BUTTON_PX: f32 = 10.0;
const BUTTON_PAD_X: f32 = 8.0;
const BUTTON_PAD_Y: f32 = 4.0;
/// Not from cmux: the keyboard-selected card, primary at this alpha.
const SELECTED_ALPHA: f32 = 0.06;
/// Not from cmux: "dimmed" green of the submitted line.
const SUBMITTED_ALPHA: f32 = 0.8;
/// Not from cmux: a disabled button (Send feedback without text, Submit before every answer).
const DISABLED_OPACITY: f32 = 0.4;

/// What a key does in the Feed (D9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FeedKey {
    Next,
    Prev,
    Jump,
    Leave,
}

pub(super) fn feed_key(k: &Keystroke) -> Option<FeedKey> {
    let plain = k.modifiers == Modifiers::none();
    let ctrl = k.modifiers == Modifiers::control();
    Some(match k.key.as_str() {
        "j" | "down" if plain => FeedKey::Next,
        "n" if ctrl => FeedKey::Next,
        "k" | "up" if plain => FeedKey::Prev,
        "p" if ctrl => FeedKey::Prev,
        "enter" if plain => FeedKey::Jump,
        "escape" if plain => FeedKey::Leave,
        _ => return None,
    })
}

/// The card after (or before) `cur` in `ids`, stopping at the ends; the first if none is.
pub(super) fn step(
    ids: &[FeedItemId],
    cur: Option<FeedItemId>,
    forward: bool,
) -> Option<FeedItemId> {
    let next = match cur.and_then(|c| ids.iter().position(|i| *i == c)) {
        None => 0,
        Some(i) if forward => (i + 1).min(ids.len() - 1),
        Some(i) => i.saturating_sub(1),
    };
    ids.get(next).copied()
}

/// The Feed badge: the pending count, `9+` past nine, nothing at 0.
pub(super) fn badge_label(pending: usize) -> Option<String> {
    match pending {
        0 => None,
        1..=9 => Some(pending.to_string()),
        _ => Some("9+".into()),
    }
}

/// Mono text for the code box and an optional secondary line under it.
fn preview_text(p: &Preview) -> (String, Option<String>) {
    match p {
        Preview::Command {
            command,
            description,
        } => (format!("$ {command}"), description.clone()),
        Preview::File { path, first_line } => (path.clone(), first_line.clone()),
        Preview::Raw(s) => (s.clone(), None),
    }
}

pub(super) fn with_alpha(c: Rgba, a: f32) -> Rgba {
    Rgba { a, ..c }
}

pub(super) fn chip(label: impl Into<SharedString>, fg: Rgba, bg: Rgba) -> Div {
    div()
        .flex_none()
        .px(px(5.))
        .py(px(2.))
        .rounded(px(CHIP_RADIUS))
        .bg(bg)
        .text_color(fg)
        .text_size(px(CHIP_PX))
        .font_weight(FontWeight::MEDIUM)
        .child(label.into())
}

/// The compact FeedButton; `enabled: false` dims it (the caller adds no click then).
pub(super) fn feed_button(label: &'static str, fg: Rgba, bg: Rgba, enabled: bool) -> Stateful<Div> {
    div()
        .id(label)
        .flex()
        .flex_1()
        .flex_basis(px(0.))
        .min_w(px(0.))
        .items_center()
        .justify_center()
        .px(px(BUTTON_PAD_X))
        .py(px(BUTTON_PAD_Y))
        .rounded(px(BUTTON_RADIUS))
        .bg(bg)
        .text_color(fg)
        .text_size(px(BUTTON_PX))
        .font_weight(FontWeight::SEMIBOLD)
        .text_center()
        .when(enabled, |d| d.hover(|s| s.opacity(0.85)))
        .when(!enabled, |d| d.opacity(DISABLED_OPACITY))
        .child(label)
}

pub(super) fn button_row() -> Div {
    div().flex().flex_row().gap(px(BUTTON_GAP))
}

/// Pending: `pending()`; answered: the submitted line; expired: nothing.
pub(super) fn card_footer(
    status: &FeedStatus,
    c: FeedColors,
    pending: impl FnOnce() -> AnyElement,
) -> Option<AnyElement> {
    match status {
        FeedStatus::Pending => Some(pending()),
        FeedStatus::Resolved(d) => Some(submitted(&d.summary(), c).into_any_element()),
        FeedStatus::Expired => None,
    }
}

/// SwiftUI's system colours (light, dark) and the label colours the Feed uses.
#[derive(Clone, Copy)]
pub(super) struct FeedColors {
    pub primary: Rgba,
    pub secondary: Rgba,
    pub orange: Rgba,
    pub purple: Rgba,
    pub blue: Rgba,
    pub green: Rgba,
    pub accent: Rgba,
}

impl FeedColors {
    pub(super) fn new(p: SidebarPalette) -> Self {
        let light = chrome::wcag_is_light(p.bg);
        let pick = |l: u32, d: u32| if light { rgb(l) } else { rgb(d) };
        Self {
            primary: with_alpha(p.title, 1.0),
            secondary: p.detail,
            orange: pick(0xFF9500, 0xFF9F0A),
            purple: pick(0xAF52DE, 0xBF5AF2),
            blue: chrome::accent(light),
            green: pick(0x34C759, 0x30D158),
            accent: p.accent,
        }
    }
}

impl WindowView {
    pub(super) fn focus_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(v) = self.views.get(&self.workspaces.selected()).cloned() {
            v.update(cx, |v, cx| v.focus_active(window, cx));
        }
    }

    /// The card's agent surface, as `surface.focus` does it.
    fn feed_jump(&mut self, id: FeedItemId, window: &mut Window, cx: &mut Context<Self>) {
        let Some((ws, surface)) = self.feed.get(id).map(|i| (i.workspace, i.surface)) else {
            return;
        };
        if let Some(view) = self.views.get(&ws).cloned() {
            view.update(cx, |v, cx| v.focus_surface(surface, window, cx));
        }
        if ws != self.workspaces.selected() {
            self.select(ws, window, cx);
        }
    }

    fn on_feed_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Only the list itself navigates; the feedback field stops its keys before here.
        if !self.feed_focus.is_focused(window) {
            return;
        }
        let Some(key) = feed_key(&ev.keystroke) else {
            return;
        };
        cx.stop_propagation();
        match key {
            FeedKey::Next | FeedKey::Prev => {
                let ids: Vec<FeedItemId> = self.feed.items().map(|i| i.id).collect();
                self.feed_selected = step(&ids, self.feed_selected, key == FeedKey::Next);
                if let Some(ix) = ids.iter().position(|i| Some(*i) == self.feed_selected) {
                    self.feed_scroll.scroll_to_item(ix);
                }
                cx.notify();
            }
            FeedKey::Jump => {
                if let Some(id) = self.feed_selected {
                    self.feed_jump(id, window, cx);
                }
            }
            FeedKey::Leave => self.focus_terminal(window, cx),
        }
    }

    /// The Feed's cards (the right sidebar's bar and frame are in `dock.rs`).
    pub(super) fn render_feed(
        &self,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let p = self.palette();
        let c = FeedColors::new(p);
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);
        let rows: Vec<_> = self
            .feed
            .items()
            .enumerate()
            .map(|(k, item)| self.render_card(item, k > 0, now_ms, c, window, cx))
            .collect();
        let body = if rows.is_empty() {
            empty_state(c).into_any_element()
        } else {
            div()
                .id("feed-list")
                .flex()
                .flex_col()
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .track_scroll(&self.feed_scroll)
                .children(rows)
                .into_any_element()
        };
        div()
            .key_context("RightSidebar")
            .track_focus(&self.feed_focus)
            .on_key_down(cx.listener(Self::on_feed_key))
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .child(body)
    }

    fn render_card(
        &self,
        item: &FeedItem,
        divider: bool,
        now_ms: u64,
        c: FeedColors,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let id = item.id;
        let (icon, tint) = match &item.kind {
            FeedKind::Permission { .. } => ("icons/shield.svg", c.orange),
            FeedKind::ExitPlan { .. } => ("icons/list.svg", c.purple),
            FeedKind::Question { .. } => ("icons/circle-help.svg", c.blue),
        };
        let claude = Rgba {
            r: 0.92,
            g: 0.54,
            b: 0.29,
            a: 1.0,
        };
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(KIND_BOX))
                    .child(svg().path(icon).size(px(KIND_ICON)).text_color(tint)),
            )
            .child(
                // shortcut: end-truncated; cmux truncates in the middle. gpui has no middle mode.
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(with_alpha(c.primary, TITLE_ALPHA))
                    .child(item.title()),
            )
            .when(item.source == "claude", |d| {
                d.child(chip(
                    "Claude",
                    claude,
                    with_alpha(claude, CLAUDE_CHIP_ALPHA),
                ))
            })
            .child(chip(
                age_label(now_ms, item.created_ms),
                c.secondary,
                with_alpha(c.primary, CODE_FILL_ALPHA),
            ));
        let status = &item.status;
        let body = match &item.kind {
            FeedKind::Permission { tool, preview } => self
                .render_permission(id, tool, preview, status, c, cx)
                .into_any_element(),
            FeedKind::ExitPlan {
                plan,
                allowed_prompts,
                plan_file,
            } => self
                .render_plan(
                    id,
                    plan,
                    allowed_prompts,
                    plan_file.as_deref(),
                    status,
                    c,
                    window,
                    cx,
                )
                .into_any_element(),
            FeedKind::Question { prompts } => self
                .render_question(id, prompts, status, c, cx)
                .into_any_element(),
        };
        let selected = self.feed_selected == Some(id);
        div()
            .id(("feed-card", id.0 as usize))
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(ROW_GAP))
            .px(px(ROW_PAD_X))
            .py(px(ROW_PAD_Y))
            .when(divider, |d| {
                d.border_t_1()
                    .border_color(with_alpha(c.primary, DIVIDER_ALPHA))
            })
            .when(selected, |d| d.bg(with_alpha(c.primary, SELECTED_ALPHA)))
            .when(item.status != FeedStatus::Pending, |d| {
                d.opacity(DONE_OPACITY)
            })
            .child(header)
            .child(body)
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                this.feed_selected = Some(id);
                if ev.click_count() == 2 {
                    this.feed_jump(id, window, cx);
                }
                cx.notify();
            }))
    }

    fn render_permission(
        &self,
        id: FeedItemId,
        tool: &str,
        preview: &Preview,
        status: &FeedStatus,
        c: FeedColors,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let (code, note) = preview_text(preview);
        let tool_line = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(5.))
            .text_size(px(11.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c.orange)
            .child(
                svg()
                    .path("icons/triangle-alert.svg")
                    .flex_none()
                    .size(px(11.))
                    .text_color(c.orange),
            )
            .child(
                div()
                    .min_w(px(0.))
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(if tool.is_empty() { "Tool" } else { tool }.to_string()),
            );
        let code_box = div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(CODE_PAD))
            .rounded(px(CODE_RADIUS))
            .bg(with_alpha(c.primary, CODE_FILL_ALPHA))
            .border_1()
            .border_color(with_alpha(c.primary, CODE_STROKE_ALPHA))
            .child(
                div()
                    .font_family(self.style.font.family.clone())
                    .text_size(px(11.))
                    .text_color(with_alpha(c.primary, TITLE_ALPHA))
                    .line_clamp(CODE_LINES)
                    .text_ellipsis()
                    .child(clip_message(&code).to_string()),
            )
            .children(note.filter(|n| !n.trim().is_empty()).map(|n| {
                div()
                    .text_size(px(10.))
                    .text_color(c.secondary)
                    .line_clamp(CODE_LINES)
                    .text_ellipsis()
                    .child(clip_message(&n).to_string())
            }));
        let footer = card_footer(status, c, || {
            self.permission_buttons(id, c, cx).into_any_element()
        });
        div()
            .flex()
            .flex_col()
            .gap(px(ROW_GAP))
            .child(tool_line)
            .child(code_box)
            .children(footer)
    }

    /// Deny / Allow Once / Always Allow / All tools (D8; Bypass is hidden for Claude).
    fn permission_buttons(
        &self,
        id: FeedItemId,
        c: FeedColors,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let white = |a: f32| Rgba {
            r: 1.,
            g: 1.,
            b: 1.,
            a,
        };
        let black = |a: f32| Rgba {
            r: 0.,
            g: 0.,
            b: 0.,
            a,
        };
        let buttons = [
            ("Deny", PermissionMode::Deny, white(1.), black(0.75)),
            ("Allow Once", PermissionMode::Once, black(1.), white(0.88)),
            ("Always Allow", PermissionMode::Always, white(1.), c.accent),
            ("All tools", PermissionMode::All, white(1.), c.accent),
        ];
        button_row().children(buttons.into_iter().map(|(label, mode, fg, bg)| {
            feed_button(label, fg, bg, true).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    this.feed_answer(id, Decision::Permission(mode), cx);
                },
            ))
        }))
    }
}

fn submitted(summary: &str, c: FeedColors) -> impl IntoElement + use<> {
    div()
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(with_alpha(c.green, SUBMITTED_ALPHA))
        .child(format!("✓ Submitted · {summary}"))
}

fn empty_state(c: FeedColors) -> impl IntoElement {
    let text = with_alpha(c.secondary, c.secondary.a * EMPTY_ALPHA);
    div()
        .flex()
        .flex_col()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(4.))
        .px(px(EMPTY_PAD_X))
        .text_center()
        .text_color(text)
        .child(div().text_size(px(12.)).child("No pending decisions"))
        .child(
            div()
                .text_size(px(11.))
                .child("Permission, plan, and question requests from AI agents will appear here."),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> Option<FeedKey> {
        feed_key(&Keystroke::parse(s).unwrap())
    }

    #[test]
    fn feed_keys_match_cmux() {
        for s in ["j", "down", "ctrl-n"] {
            assert_eq!(key(s), Some(FeedKey::Next), "{s}");
        }
        for s in ["k", "up", "ctrl-p"] {
            assert_eq!(key(s), Some(FeedKey::Prev), "{s}");
        }
        assert_eq!(key("enter"), Some(FeedKey::Jump));
        assert_eq!(key("escape"), Some(FeedKey::Leave));
        for s in ["n", "shift-j", "super-k", "ctrl-4", "ctrl-enter"] {
            assert_eq!(key(s), None, "{s}");
        }
    }

    #[test]
    fn step_moves_and_stops_at_the_ends() {
        let ids = [FeedItemId(3), FeedItemId(2), FeedItemId(1)];
        assert_eq!(step(&ids, None, true), Some(FeedItemId(3)));
        assert_eq!(step(&ids, None, false), Some(FeedItemId(3)));
        assert_eq!(step(&ids, Some(FeedItemId(3)), true), Some(FeedItemId(2)));
        assert_eq!(step(&ids, Some(FeedItemId(1)), true), Some(FeedItemId(1)));
        assert_eq!(step(&ids, Some(FeedItemId(3)), false), Some(FeedItemId(3)));
        assert_eq!(step(&ids, Some(FeedItemId(9)), true), Some(FeedItemId(3)));
        assert_eq!(step(&[], Some(FeedItemId(1)), true), None);
    }

    #[test]
    fn badge_caps_at_nine() {
        assert_eq!(badge_label(0), None);
        assert_eq!(badge_label(9).as_deref(), Some("9"));
        assert_eq!(badge_label(10).as_deref(), Some("9+"));
    }
}
