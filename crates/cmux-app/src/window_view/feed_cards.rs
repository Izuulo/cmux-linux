//! The Feed's plan and question cards (plan M7 D8, rf §5): plan feedback / Manual / Auto, and
//! question picks with Submit All Answers. Split from `feed.rs` for size.

use std::collections::BTreeSet;

use crate::ui::px;
use cmux_core::{Decision, FeedItemId, FeedStatus, PlanMode, QuestionPrompt};
use gpui::{
    ClickEvent, Context, Div, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Rgba, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::WindowView;
use super::feed::{
    BUTTON_RADIUS, CLAUDE_CHIP_ALPHA, CODE_FILL_ALPHA, CODE_PAD, CODE_RADIUS, CODE_STROKE_ALPHA,
    FeedColors, ROW_GAP, TITLE_ALPHA, button_row, card_footer, chip, feed_button, with_alpha,
};
use super::rename::{RenameKey, RenameOutcome, RenameState};

/// rf §5 / D8: the plan scrolls past this height.
const PLAN_MAX_H: f32 = 240.0;
const FIELD_PAD: f32 = 10.0;
const FIELD_RADIUS: f32 = 6.0;
/// Not from cmux: the soft (Manual) button fill, a picked option's accent fill.
const SOFT_ALPHA: f32 = 0.10;
const PICKED_ALPHA: f32 = 0.15;

/// A question card's picks: option indices per question (D8).
#[derive(Debug, Default)]
pub(super) struct QuestionPicks(Vec<BTreeSet<usize>>);

impl QuestionPicks {
    /// Single-select: `opt` becomes the pick. Multi-select: `opt` toggles.
    pub(super) fn toggle(&mut self, q: usize, opt: usize, multi: bool) {
        if self.0.len() <= q {
            self.0.resize_with(q + 1, BTreeSet::new);
        }
        let set = &mut self.0[q];
        if !multi {
            set.clear();
            set.insert(opt);
        } else if !set.remove(&opt) {
            set.insert(opt);
        }
    }

    pub(super) fn is_picked(&self, q: usize, opt: usize) -> bool {
        self.0.get(q).is_some_and(|s| s.contains(&opt))
    }

    /// One entry per question, in order, multi-select labels joined `", "`; `None` until every
    /// question has a pick (an entry is never skipped: `hook_output` keys answers by index).
    pub(super) fn selections(&self, prompts: &[QuestionPrompt]) -> Option<Vec<String>> {
        prompts
            .iter()
            .enumerate()
            .map(|(q, p)| {
                let labels: Vec<&str> = self
                    .0
                    .get(q)?
                    .iter()
                    .filter_map(|&o| p.options.get(o).map(|o| o.label.as_str()))
                    .collect();
                (!labels.is_empty()).then(|| labels.join(", "))
            })
            .collect()
    }
}

/// A small titled block under the plan (`Allowed prompts`, `Plan file`).
fn plan_section(title: &'static str, c: FeedColors) -> Div {
    div().flex().flex_col().gap(px(2.)).child(
        div()
            .text_size(px(10.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c.secondary)
            .child(title),
    )
}

impl WindowView {
    /// The plan feedback field's keys: Enter sends feedback when there is text, Escape goes back
    /// to the list (text kept), the rest edits.
    fn on_feedback_key(
        &mut self,
        id: FeedItemId,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let state = self
            .feed_feedback
            .entry(id)
            .or_insert_with(|| RenameState::new("", true));
        match state.apply(RenameKey::from(&ev.keystroke)) {
            RenameOutcome::Editing => {}
            RenameOutcome::Commit(t) => {
                if let Some(d) = feedback_decision(&t) {
                    self.feed_answer(id, d, cx);
                    window.focus(&self.feed_focus);
                }
            }
            RenameOutcome::Cancel => window.focus(&self.feed_focus),
        }
        cx.notify();
    }

    /// Plan text (plain, scrolls past 240), `Allowed prompts`, `Plan file`; while pending the
    /// feedback field and Send feedback / Manual / Auto (D8; no Ultraplan).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_plan(
        &self,
        id: FeedItemId,
        plan: &str,
        allowed_prompts: &[String],
        plan_file: Option<&str>,
        status: &FeedStatus,
        c: FeedColors,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let plan_box = div()
            .id("plan")
            .max_h(px(PLAN_MAX_H))
            .overflow_y_scroll()
            .p(px(CODE_PAD))
            .rounded(px(CODE_RADIUS))
            .bg(with_alpha(c.primary, CODE_FILL_ALPHA))
            .border_1()
            .border_color(with_alpha(c.primary, CODE_STROKE_ALPHA))
            .text_size(px(11.))
            .text_color(with_alpha(c.primary, TITLE_ALPHA))
            .child(plan.to_string());
        let prompts = (!allowed_prompts.is_empty()).then(|| {
            plan_section("Allowed prompts", c).children(allowed_prompts.iter().map(|p| {
                div()
                    .text_size(px(11.))
                    .text_color(c.secondary)
                    .child(format!("• {p}"))
            }))
        });
        let file = plan_file.filter(|f| !f.is_empty()).map(|f| {
            plan_section("Plan file", c).child(
                div()
                    .font_family(self.style.font.family.clone())
                    .text_size(px(10.))
                    .text_color(c.secondary)
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(f.to_string()),
            )
        });
        let footer = card_footer(status, c, || {
            div()
                .flex()
                .flex_col()
                .gap(px(ROW_GAP))
                .child(self.feedback_field(id, c, window, cx))
                .child(self.plan_buttons(id, c, cx))
                .into_any_element()
        });
        div()
            .flex()
            .flex_col()
            .gap(px(ROW_GAP))
            .child(plan_box)
            .children(prompts)
            .children(file)
            .children(footer)
    }

    fn feedback_field(
        &self,
        id: FeedItemId,
        c: FeedColors,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let editing = self.feed_editing == Some(id);
        let focused = editing && self.feed_field_focus.is_focused(window);
        let text = self.feed_feedback.get(&id).map_or("", |s| s.text.as_str());
        // shortcut: the caret is a glyph after the text (no cursor movement, no IME), as in the
        // rename field; a real text input needs gpui's InputHandler.
        let shown = match (text.is_empty(), focused) {
            (true, false) => None,
            (_, true) => Some(format!("{text}▏")),
            (false, false) => Some(text.to_string()),
        };
        div()
            .id("feedback")
            .when(editing, |d| d.track_focus(&self.feed_field_focus))
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
                this.on_feedback_key(id, ev, window, cx)
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.feed_selected = Some(id);
                this.feed_editing = Some(id);
                window.focus(&this.feed_field_focus);
                cx.notify();
            }))
            .p(px(FIELD_PAD))
            .rounded(px(FIELD_RADIUS))
            .bg(with_alpha(c.primary, CODE_FILL_ALPHA))
            .border_1()
            .border_color(if focused {
                c.accent
            } else {
                with_alpha(c.primary, CODE_STROKE_ALPHA)
            })
            .text_size(px(12.))
            .text_color(with_alpha(c.primary, TITLE_ALPHA))
            .child(match shown {
                Some(t) => div().child(t),
                None => div()
                    .text_color(c.secondary)
                    .child("Tell Claude what to change…"),
            })
    }

    fn plan_buttons(&self, id: FeedItemId, c: FeedColors, cx: &Context<Self>) -> impl IntoElement {
        let white = Rgba {
            r: 1.,
            g: 1.,
            b: 1.,
            a: 1.,
        };
        let feedback = self
            .feed_feedback
            .get(&id)
            .and_then(|s| feedback_decision(&s.text));
        let answer = move |d: Decision| {
            cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.feed_answer(id, d.clone(), cx);
            })
        };
        let mode = |mode| Decision::ExitPlan {
            mode,
            feedback: None,
        };
        button_row()
            .child(
                feed_button("Send feedback", white, c.accent, feedback.is_some())
                    .when_some(feedback, |b, d| b.on_click(answer(d))),
            )
            .child(
                feed_button("Manual", c.primary, with_alpha(c.primary, SOFT_ALPHA), true)
                    .on_click(answer(mode(PlanMode::Manual))),
            )
            .child(
                feed_button("Auto", white, c.green, true)
                    .on_click(answer(mode(PlanMode::AutoAccept))),
            )
    }

    /// Each question's header, text and options (multi-select toggles), then Submit All
    /// Answers once every question has a pick (D8; no free-text answer).
    pub(super) fn render_question(
        &self,
        id: FeedItemId,
        prompts: &[QuestionPrompt],
        status: &FeedStatus,
        c: FeedColors,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let pending = *status == FeedStatus::Pending;
        let picks = self.feed_picks.get(&id);
        let questions = prompts.iter().enumerate().map(|(q, p)| {
            let multi = p.multi_select;
            let head = div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .when(!p.header.is_empty(), |d| {
                    d.child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c.secondary)
                            .child(p.header.clone()),
                    )
                })
                .when(multi, |d| {
                    d.child(chip(
                        "Multi-select",
                        c.blue,
                        with_alpha(c.blue, CLAUDE_CHIP_ALPHA),
                    ))
                });
            let options = p.options.iter().enumerate().map(move |(o, opt)| {
                let picked = picks.is_some_and(|s| s.is_picked(q, o));
                div()
                    .id(("option", o))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(px(BUTTON_RADIUS))
                    .border_1()
                    .border_color(if picked {
                        c.accent
                    } else {
                        with_alpha(c.primary, CODE_STROKE_ALPHA)
                    })
                    .bg(if picked {
                        with_alpha(c.accent, PICKED_ALPHA)
                    } else {
                        with_alpha(c.primary, CODE_FILL_ALPHA)
                    })
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(with_alpha(c.primary, TITLE_ALPHA))
                            .child(opt.label.clone()),
                    )
                    .when(!opt.description.is_empty(), |d| {
                        d.child(
                            div()
                                .text_size(px(10.))
                                .text_color(c.secondary)
                                .child(opt.description.clone()),
                        )
                    })
                    .when(pending, |d| {
                        d.hover(|s| s.opacity(0.85)).on_click(cx.listener(
                            move |this, _: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                this.feed_selected = Some(id);
                                this.feed_picks.entry(id).or_default().toggle(q, o, multi);
                                cx.notify();
                            },
                        ))
                    })
            });
            div()
                .id(("question", q))
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(head)
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(with_alpha(c.primary, TITLE_ALPHA))
                        .child(p.question.clone()),
                )
                .children(options)
        });
        let answers = picks.and_then(|p| p.selections(prompts));
        let footer = card_footer(status, c, || {
            let white = Rgba {
                r: 1.,
                g: 1.,
                b: 1.,
                a: 1.,
            };
            button_row()
                .child(
                    feed_button("Submit All Answers", white, c.accent, answers.is_some())
                        .when_some(answers, |b, selections| {
                            b.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                let d = Decision::Question {
                                    selections: selections.clone(),
                                };
                                this.feed_answer(id, d, cx);
                            }))
                        }),
                )
                .into_any_element()
        });
        div()
            .flex()
            .flex_col()
            .gap(px(ROW_GAP))
            .children(questions)
            .children(footer)
    }
}

/// Send feedback's decision: manual with the trimmed text, nothing without text.
fn feedback_decision(text: &str) -> Option<Decision> {
    let t = text.trim();
    (!t.is_empty()).then(|| Decision::ExitPlan {
        mode: PlanMode::Manual,
        feedback: Some(t.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use cmux_core::QuestionOption;

    use super::*;

    fn prompt(multi: bool, labels: &[&str]) -> QuestionPrompt {
        QuestionPrompt {
            header: String::new(),
            question: String::new(),
            multi_select: multi,
            options: labels
                .iter()
                .map(|l| QuestionOption {
                    label: (*l).into(),
                    description: String::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn selections_join_multi_select() {
        let prompts = [prompt(false, &["A", "B"]), prompt(true, &["X", "Y", "Z"])];
        let mut p = QuestionPicks::default();
        p.toggle(0, 0, false);
        p.toggle(0, 1, false);
        p.toggle(1, 2, true);
        p.toggle(1, 1, true);
        p.toggle(1, 0, true);
        p.toggle(1, 1, true);
        assert!(
            p.is_picked(0, 1) && !p.is_picked(0, 0),
            "single-select keeps one"
        );
        assert_eq!(
            p.selections(&prompts),
            Some(vec!["B".into(), "X, Z".into()]),
            "option order, toggled-off Y dropped"
        );
    }

    #[test]
    fn submit_needs_every_question() {
        let prompts = [prompt(false, &["A"]), prompt(true, &["X", "Y"])];
        let mut p = QuestionPicks::default();
        assert_eq!(p.selections(&prompts), None);
        p.toggle(1, 1, true);
        assert_eq!(p.selections(&prompts), None, "question 0 unanswered");
        p.toggle(0, 0, false);
        assert_eq!(p.selections(&prompts), Some(vec!["A".into(), "Y".into()]));
        p.toggle(1, 1, true);
        assert_eq!(
            p.selections(&prompts),
            None,
            "untoggling empties question 1"
        );
    }
}
