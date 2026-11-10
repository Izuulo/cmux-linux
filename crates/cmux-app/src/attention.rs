//! What a notification does to the panes: the focused-arrival rule (rn §3) and, from Task 5,
//! the pane ring and flash.

use std::time::Duration;

use gpui::{Animation, AnimationExt, Div, IntoElement, ParentElement, Rgba, Styled, div, px};

/// How long one flash runs (rn §4 double blink).
pub const FLASH: Duration = Duration::from_millis(900);

/// A notification "arrives focused" only on the focused surface of the selected workspace of
/// the active window. Such an arrival posts no desktop notification, doesn't flash and doesn't
/// reorder the sidebar (rn §3, §9).
pub fn is_focused_arrival(
    window_active: bool,
    workspace_selected: bool,
    surface_is_active: bool,
) -> bool {
    window_active && workspace_selected && surface_is_active
}

/// Ring geometry (rn §4): inset from the pane, corner radius, stroke width.
const INSET: f32 = 2.0;
const RADIUS: f32 = 6.0;
const STROKE: f32 = 2.0;

/// Opacity of the double-blink flash at `t` in [0, 1]: keyframes 0→1→0→1→0 at 0, .25, .5,
/// .75, 1; quadratic ease-out going up, ease-in coming down (rn §4).
pub fn flash_opacity(t: f32) -> f32 {
    if !(0.0..1.0).contains(&t) {
        return 0.0;
    }
    let seg = (t * 4.0).floor();
    let u = t * 4.0 - seg;
    if (seg as u32).is_multiple_of(2) {
        1.0 - (1.0 - u) * (1.0 - u)
    } else {
        1.0 - u * u
    }
}

/// A stroke `width` wide centred on the ring line, in `color` at `alpha`.
fn stroke(color: Rgba, alpha: f32, width: f32) -> Div {
    let inset = INSET + STROKE / 2.0 - width / 2.0;
    div()
        .absolute()
        .top(px(inset))
        .left(px(inset))
        .right(px(inset))
        .bottom(px(inset))
        .rounded(px(RADIUS + (STROKE - width) / 2.0))
        .border(px(width))
        .border_color(Rgba { a: alpha, ..color })
}

/// The steady unread ring over a pane: a 2 px accent stroke with a soft glow. gpui shadows fill
/// the whole rect, so the glow (cmux: 0.35 alpha, 3 blur) is a wider faint stroke underneath.
pub fn ring(accent: Rgba) -> Div {
    div()
        .absolute()
        .inset_0()
        .child(stroke(accent, 0.35 / 2.0, STROKE + 2.0))
        .child(stroke(accent, 1.0, STROKE))
}

/// The flash drawn over the ring (glow 0.6/6 in cmux), animated once per `token`.
pub fn flash(accent: Rgba, token: u64) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .child(stroke(accent, 0.6 / 2.0, STROKE + 4.0))
        .child(stroke(accent, 1.0, STROKE))
        .with_animation(("flash", token as usize), Animation::new(FLASH), |el, t| {
            el.opacity(flash_opacity(t))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flash_opacity_double_blink() {
        let cases = [
            (0.0, 0.0),
            (0.125, 0.75),
            (0.25, 1.0),
            (0.375, 0.75),
            (0.5, 0.0),
            (0.75, 1.0),
            (1.0, 0.0),
        ];
        for (t, want) in cases {
            assert!(
                (flash_opacity(t) - want).abs() < 1e-4,
                "t={t}: {}",
                flash_opacity(t)
            );
        }
        assert_eq!(flash_opacity(-1.0), 0.0);
        assert_eq!(flash_opacity(2.0), 0.0);
    }

    #[test]
    fn inactive_window_is_never_a_focused_arrival() {
        for w in [false, true] {
            for ws in [false, true] {
                for s in [false, true] {
                    assert_eq!(is_focused_arrival(w, ws, s), w && ws && s, "{w} {ws} {s}");
                }
            }
        }
        assert!(!is_focused_arrival(false, true, true));
    }
}
