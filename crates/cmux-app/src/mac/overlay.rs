//! A floating layer (menu, palette, popover) at `at`, drawn above the window with its fade.
//! The caller wires handlers into `content` only while `fade` is not leaving; a leaving overlay
//! also gets a handler-less cover, so nothing in it can be clicked during the fade.

use std::time::Instant;

use gpui::{
    ElementId, InteractiveElement, IntoElement, ParentElement, Pixels, Point, Styled, anchored,
    deferred, div, prelude::FluentBuilder,
};

use super::{Fade, metric};
use crate::ui::px;

pub fn overlay(
    id: impl Into<ElementId>,
    at: Point<Pixels>,
    fade: Fade,
    now: Instant,
    content: impl IntoElement,
) -> impl IntoElement {
    let leaving = fade.leaving();
    deferred(
        anchored()
            .position(at)
            .snap_to_window_with_margin(px(metric::OVERLAY_MARGIN))
            .child(
                div()
                    .id(id)
                    .relative()
                    .when(!leaving, |d| d.occlude())
                    .opacity(fade.amount_at(now))
                    .mt(px(fade.slide(now)))
                    .child(content)
                    .when(leaving, |d| {
                        d.child(
                            div()
                                .id("overlay-leaving-cover")
                                .absolute()
                                .size_full()
                                .occlude(),
                        )
                    }),
            ),
    )
}
