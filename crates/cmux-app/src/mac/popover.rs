//! A glass popover with a soft arrow pointing at its anchor (NSPopover; plan K3-K5).

use std::time::Instant;

use gpui::{
    AnyElement, IntoElement, ParentElement, PathBuilder, Pixels, Point, Styled, canvas, div, point,
    prelude::FluentBuilder,
};

use super::{Fade, Theme, glass_panel, metric, overlay};
use crate::ui::{px, s};

/// Up-pointing arrow outline: a rounded tip, then flanks that are steep at the tip and flatten
/// into the base (one cubic per side whose last control sits on the base line).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrowPts {
    pub base_l: (f32, f32),
    pub ctrl_l: [(f32, f32); 2],
    pub tip_l: (f32, f32),
    pub tip_ctrl: (f32, f32),
    pub tip_r: (f32, f32),
    pub ctrl_r: [(f32, f32); 2],
    pub base_r: (f32, f32),
}

/// Slope (dy/dx) of the flank where it leaves the tip: about 56 degrees.
const TIP_SLOPE: f32 = 1.5;

pub fn arrow_points(tip_x: f32, tip_y: f32) -> ArrowPts {
    let (w, h, r) = (metric::ARROW_W, metric::ARROW_H, metric::ARROW_TIP_R);
    let base_y = tip_y + h;
    // The tip is a quadratic from (-dx, dy) to (dx, dy) with its control at (0, -dy), so its
    // apex is `tip_y` and its end tangent has slope 2 dy / dx = TIP_SLOPE. Each flank starts on
    // that tangent (first control), and arrives level with the base (second control).
    let dx = r * 0.55;
    let dy = dx * TIP_SLOPE / 2.0;
    let run = (h - dy) * 0.4;
    let c1 = (dx + run / TIP_SLOPE, dy + run);
    let c2 = (w / 2.0 - w * 0.2, h);
    let (l, r_) = (
        |(x, y): (f32, f32)| (tip_x - x, tip_y + y),
        |(x, y): (f32, f32)| (tip_x + x, tip_y + y),
    );
    ArrowPts {
        base_l: (tip_x - w / 2.0, base_y),
        ctrl_l: [l(c2), l(c1)],
        tip_l: l((dx, dy)),
        tip_ctrl: (tip_x, tip_y - dy),
        tip_r: r_((dx, dy)),
        ctrl_r: [r_(c1), r_(c2)],
        base_r: (tip_x + w / 2.0, base_y),
    }
}

/// gpui path for `a`.
fn arrow_path(a: ArrowPts, origin: Point<Pixels>) -> PathBuilder {
    let at = |(x, y): (f32, f32)| point(origin.x + px(x), origin.y + px(y));
    let mut b = PathBuilder::fill();
    b.move_to(at(a.base_l));
    b.cubic_bezier_to(at(a.tip_l), at(a.ctrl_l[0]), at(a.ctrl_l[1]));
    b.curve_to(at(a.tip_r), at(a.tip_ctrl));
    b.cubic_bezier_to(at(a.base_r), at(a.ctrl_r[0]), at(a.ctrl_r[1]));
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
    let (fill, outer, inner) = (t.popover, t.edge_outer, t.edge_inner);
    // Drawn after the body, overlapping its two top lines (2 px): each layer sits 1 px lower
    // than the last, so the outer line, the inner line and the fill show as on the body, and
    // the last fill covers both lines across the base.
    let arrow = canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let layers = [(0.0, outer), (1.0, fill), (1.0, inner), (2.0, fill)];
            for (tip_y, color) in layers {
                let a = arrow_points(metric::ARROW_W / 2.0, tip_y);
                if let Ok(p) = arrow_path(a, bounds.origin).build() {
                    window.paint_path(p, color);
                }
            }
        },
    )
    .w(px(metric::ARROW_W))
    .h(px(metric::ARROW_H + 2.0));
    let body = glass_panel(t, metric::POPOVER_RADIUS, fill)
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
        for p in [
            a.tip_l,
            a.tip_r,
            a.ctrl_l[0],
            a.ctrl_l[1],
            a.ctrl_r[0],
            a.ctrl_r[1],
        ] {
            assert!(
                p.1 >= 50.0 && p.1 <= 50.0 + h,
                "{p:?} outside the arrow's box"
            );
        }
        assert!(a.tip_ctrl.1 < a.tip_l.1, "the tip curve bulges upward");
    }

    #[test]
    fn flanks_are_steep_at_the_tip_and_flatten_into_the_base() {
        let a = arrow_points(100.0, 50.0);
        // The last control is level with the base, so the flank arrives horizontally (concave
        // flare), and the first control continues the tip's tangent (no kink).
        assert_eq!(a.ctrl_l[0].1, a.base_l.1);
        assert!(a.base_l.0 < a.ctrl_l[0].0 && a.ctrl_l[0].0 < 100.0);
        let slope = |p: (f32, f32), q: (f32, f32)| (q.1 - p.1) / (q.0 - p.0);
        let tip_tangent = slope(a.tip_ctrl, a.tip_l);
        assert!((tip_tangent + TIP_SLOPE).abs() < 1e-4);
        assert!((slope(a.tip_l, a.ctrl_l[1]) + TIP_SLOPE).abs() < 1e-4);
        assert!(
            a.ctrl_l[1].0 > a.ctrl_l[0].0,
            "steep part sits nearer the tip"
        );
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
