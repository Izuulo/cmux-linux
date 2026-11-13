//! A glass popover with a soft arrow pointing at its anchor (NSPopover; plan K3-K5).

use std::time::Instant;

use gpui::{
    AnyElement, IntoElement, ParentElement, PathBuilder, Pixels, Point, Styled, canvas, div, point,
    prelude::FluentBuilder,
};

use super::{Fade, Theme, glass_panel, metric, overlay};
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

/// Body left edge and arrow centre (design px): the body is centred on `tip_x` but slides to
/// stay inside the window; the arrow keeps pointing at `tip_x`, within the body's straight edge.
pub fn popover_left(tip_x: f32, width: f32, window_w: f32, margin: f32) -> (f32, f32) {
    let left = (tip_x - width / 2.0)
        .min(window_w - margin - width)
        .max(margin);
    let inset = metric::POPOVER_RADIUS + metric::ARROW_W / 2.0;
    // max then min, not clamp: a body narrower than two insets must not panic.
    (left, tip_x.max(left + inset).min(left + width - inset))
}

#[allow(clippy::too_many_arguments)]
pub fn popover(
    id: &'static str,
    t: &Theme,
    tip: Point<Pixels>,
    window_w: f32,
    width: f32,
    height: Option<f32>,
    fade: Fade,
    now: Instant,
    content: AnyElement,
) -> impl IntoElement + use<> {
    // `window_w` and `width` are design px (unscaled); `tip` is already window px.
    let (left, arrow_x) = popover_left(
        f32::from(tip.x) / s(1.0),
        width,
        window_w,
        metric::OVERLAY_MARGIN,
    );
    let (fill, edge) = (t.glass, t.glass_edge);
    // Drawn after the body, overlapping its top border by 1 px: the fill covers the border
    // across the base, so no line crosses the arrow.
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
    let inner = div().relative().pt(px(metric::ARROW_H)).child(body).child(
        div()
            .absolute()
            .top(px(0.))
            .left(px(arrow_x - left - metric::ARROW_W / 2.0))
            .child(arrow),
    );
    // Leaving, the overlay's cover takes clicks, so the caller's buttons (e.g. "Clear All")
    // can't fire during the fade.
    overlay(id, point(gpui::px(s(left)), tip.y), fade, now, inner)
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

    const M: f32 = metric::OVERLAY_MARGIN;
    const W: f32 = 560.0;
    const WIN: f32 = 1200.0;
    // Arrow centre must stay this far inside the body's edge (straight part of the edge).
    const INSET: f32 = metric::POPOVER_RADIUS + metric::ARROW_W / 2.0;

    #[test]
    fn centred_when_there_is_room() {
        let (left, arrow) = popover_left(600.0, W, WIN, M);
        assert_eq!((left, arrow), (600.0 - W / 2.0, 600.0));
    }

    #[test]
    fn clamped_at_left_edge_arrow_keeps_pointing() {
        let (left, arrow) = popover_left(100.0, W, WIN, M);
        assert_eq!((left, arrow), (M, 100.0));
    }

    #[test]
    fn clamped_at_right_edge_arrow_keeps_pointing() {
        let (left, arrow) = popover_left(1150.0, W, WIN, M);
        assert_eq!((left, arrow), (WIN - M - W, 1150.0));
    }

    #[test]
    fn arrow_stays_within_the_straight_edge() {
        let (left, arrow) = popover_left(0.0, W, WIN, M);
        assert_eq!(arrow, left + INSET);
        let (left, arrow) = popover_left(WIN, W, WIN, M);
        assert_eq!(arrow, left + W - INSET);
    }

    #[test]
    fn wider_than_window_pins_to_margin() {
        assert_eq!(popover_left(300.0, 2000.0, WIN, M).0, M);
    }
}
