//! A glass popover with a soft arrow pointing at its anchor (NSPopover; plan K3-K5).

use std::time::Instant;

use gpui::{
    AnyElement, InteractiveElement, IntoElement, ParentElement, PathBuilder, Pixels, Point, Styled,
    anchored, canvas, deferred, div, point, prelude::FluentBuilder,
};

use super::{Fade, Theme, glass_panel, metric};
use crate::ui::{px, s};

/// Up-pointing arrow outline: flared shoulders into a rounded tip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrowPts {
    pub base_l: (f32, f32),
    pub shoulder_l: (f32, f32),
    pub tip_l: (f32, f32),
    pub tip_ctrl: (f32, f32),
    pub tip_r: (f32, f32),
    pub shoulder_r: (f32, f32),
    pub base_r: (f32, f32),
}

pub fn arrow_points(tip_x: f32, tip_y: f32) -> ArrowPts {
    let (w, h, r) = (metric::ARROW_W, metric::ARROW_H, metric::ARROW_TIP_R);
    let base_y = tip_y + h;
    ArrowPts {
        base_l: (tip_x - w / 2.0, base_y),
        shoulder_l: (tip_x - w * 0.2, tip_y + h * 0.45),
        tip_l: (tip_x - r, tip_y + r),
        tip_ctrl: (tip_x, tip_y - r * 0.2),
        tip_r: (tip_x + r, tip_y + r),
        shoulder_r: (tip_x + w * 0.2, tip_y + h * 0.45),
        base_r: (tip_x + w / 2.0, base_y),
    }
}

/// Scaled gpui path for `a`; the shoulders curve out of the base like NSPopover's.
fn arrow_path(a: ArrowPts, origin: Point<Pixels>) -> PathBuilder {
    let at = |(x, y): (f32, f32)| point(origin.x + px(x), origin.y + px(y));
    let w = metric::ARROW_W;
    let mut b = PathBuilder::fill();
    b.move_to(at(a.base_l));
    b.curve_to(at(a.shoulder_l), at((a.base_l.0 + w * 0.18, a.base_l.1)));
    b.line_to(at(a.tip_l));
    b.curve_to(at(a.tip_r), at(a.tip_ctrl));
    b.line_to(at(a.shoulder_r));
    b.curve_to(at(a.base_r), at((a.base_r.0 - w * 0.18, a.base_r.1)));
    b.close();
    b
}

#[allow(clippy::too_many_arguments)]
pub fn popover(
    id: &'static str,
    t: &Theme,
    tip: Point<Pixels>,
    width: f32,
    height: Option<f32>,
    fade: Fade,
    now: Instant,
    content: AnyElement,
) -> impl IntoElement {
    let amount = fade.amount_at(now);
    let (fill, edge) = (t.glass, t.glass_edge);
    // The arrow is drawn in its own box above the body; the body's top edge hides its base.
    let arrow = canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let a = arrow_points(metric::ARROW_W / 2.0, 0.0);
            if let Ok(p) = arrow_path(a, bounds.origin).build() {
                window.paint_path(p, edge);
            }
            // Inset 1 px so the edge colour shows as a hairline outline.
            let inner = arrow_points(metric::ARROW_W / 2.0, 1.0);
            if let Ok(p) = arrow_path(inner, bounds.origin).build() {
                window.paint_path(p, fill);
            }
        },
    )
    .w(px(metric::ARROW_W))
    .h(px(metric::ARROW_H + 1.0));
    let body = glass_panel(t, metric::POPOVER_RADIUS)
        .w(px(width))
        .when_some(height, |d, h| d.h(px(h)))
        .flex()
        .flex_col()
        .child(content);
    deferred(
        anchored()
            .position(point(tip.x - gpui::px(s(width) / 2.0), tip.y))
            .snap_to_window_with_margin(px(metric::OVERLAY_MARGIN))
            .child(
                div()
                    .id(id)
                    .occlude()
                    .when(fade.leaving(), |d| d.block_mouse_except_scroll())
                    .opacity(amount)
                    .mt(px(fade.slide(now)))
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(arrow)
                    // Overlap the arrow's base by 1 px so no seam shows.
                    .child(div().mt(px(-1.0)).child(body)),
            ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_is_symmetric_and_fits_its_box() {
        let a = arrow_points(100.0, 50.0);
        let (w, h) = (metric::ARROW_W, metric::ARROW_H);
        assert_eq!(a.base_l, (100.0 - w / 2.0, 50.0 + h));
        assert_eq!(a.base_r, (100.0 + w / 2.0, 50.0 + h));
        assert_eq!(100.0 - a.tip_l.0, a.tip_r.0 - 100.0);
        assert_eq!(100.0 - a.shoulder_l.0, a.shoulder_r.0 - 100.0);
        for p in [a.shoulder_l, a.tip_l, a.tip_r, a.shoulder_r] {
            assert!(
                p.1 >= 50.0 && p.1 <= 50.0 + h,
                "{p:?} outside the arrow's box"
            );
        }
        assert!(a.tip_ctrl.1 < a.tip_l.1, "the tip curve bulges upward");
    }
}
