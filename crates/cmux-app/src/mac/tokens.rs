use crate::chrome;
use crate::ui::px;
use gpui::{BoxShadow, Rgba, point};

/// Colours, derived from the terminal background like the rest of the chrome.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Rgba,
    pub sidebar: Rgba,
    pub card: Rgba,
    pub separator: Rgba,
    pub text: Rgba,
    pub secondary: Rgba,
    pub accent: Rgba,
    pub hover: Rgba,
    pub control: Rgba,
    pub warning: Rgba,
    pub error: Rgba,
    /// Notification popover fill (opaque: without blur any see-through shows text behind).
    pub popover: Rgba,
    /// NSMenu fill (opaque, lighter than the popover).
    pub menu: Rgba,
    /// Outer 1 px line of popovers and menus.
    pub edge_outer: Rgba,
    /// Inner 1 px light line of popovers and menus.
    pub edge_inner: Rgba,
    /// Command palette fill: the terminal background itself.
    pub palette: Rgba,
    pub palette_border: Rgba,
    /// Command palette selected row: a tint over the fill, the text keeps its colour.
    pub palette_select: Rgba,
    /// Card outline.
    pub card_edge: Rgba,
    /// Outline of an inline text field (omnibar).
    pub field_border: Rgba,
    /// Menu row under the pointer: accent fill, white text (NSMenu).
    pub menu_hover: Rgba,
    pub menu_hover_text: Rgba,
}

const WHITE: Rgba = Rgba {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

impl Theme {
    pub fn new(bg: Rgba) -> Self {
        let p = chrome::sidebar_palette(bg);
        let light = chrome::wcag_is_light(bg);
        let ink = |a: f32| {
            let v = if light { 0.0 } else { 1.0 };
            Rgba {
                r: v,
                g: v,
                b: v,
                a,
            }
        };
        // Measured: dark cards are darker than the window; light cards are white on white,
        // set off by their edge.
        let card = if light {
            WHITE
        } else {
            chrome::shift(bg, -0.03, 1.0)
        };
        let grey = |r: f32, g: f32, b: f32| Rgba {
            r: r / 255.0,
            g: g / 255.0,
            b: b / 255.0,
            a: 1.0,
        };
        // Light: NSPopover fill measured on macOS 27; menus not measured, assumed the same.
        let light_fill = grey(235., 236., 237.);
        let black = |a: f32| Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a,
        };
        let separator = Rgba {
            a: p.border.a * 0.5,
            ..p.border
        };
        Theme {
            bg,
            sidebar: chrome::shift(bg, if light { -0.03 } else { 0.025 }, 1.0),
            card,
            separator,
            text: p.title,
            secondary: p.detail,
            accent: p.accent,
            hover: ink(0.07),
            control: ink(0.12),
            warning: Rgba {
                r: 1.0,
                g: 0.62,
                b: 0.04,
                a: 1.0,
            },
            error: Rgba {
                r: 1.0,
                g: 0.27,
                b: 0.23,
                a: 1.0,
            },
            popover: if light {
                light_fill
            } else {
                grey(30., 30., 30.)
            },
            menu: if light {
                light_fill
            } else {
                grey(61., 59., 62.)
            },
            // Light measured near-black too; 0.5 because at 1x a 1 px line is twice as heavy.
            edge_outer: if light { black(0.5) } else { black(0.9) },
            edge_inner: if light { WHITE } else { alpha(WHITE, 0.10) },
            palette: bg,
            palette_border: if light { ink(0.12) } else { ink(0.18) },
            palette_select: alpha(p.accent, 0.14),
            card_edge: if light { separator } else { ink(0.08) },
            field_border: if light { ink(0.10) } else { ink(0.14) },
            menu_hover: p.accent,
            menu_hover_text: WHITE,
        }
    }
}

/// Tahoe metrics in logical pixels before UI scale (plan K3). Calibrated against cmux on
/// macOS 27 (2026-11-12, dark).
pub mod metric {
    pub const WINDOW_RADIUS: f32 = 16.0;
    pub const TITLEBAR_H: f32 = 28.0;
    pub const TITLE_TEXT: f32 = 13.0;
    pub const BODY_TEXT: f32 = 13.0;
    pub const SMALL_TEXT: f32 = 11.0;
    pub const MENU_RADIUS: f32 = 13.0;
    pub const MENU_INSET: f32 = 5.0;
    pub const MENU_ITEM_H: f32 = 24.0;
    pub const MENU_ITEM_RADIUS: f32 = 8.0;
    pub const MENU_ITEM_PAD_X: f32 = 10.0;
    pub const MENU_W: f32 = 220.0;
    pub const POPOVER_RADIUS: f32 = 20.0;
    pub const ARROW_W: f32 = 30.0;
    pub const ARROW_H: f32 = 12.0;
    pub const ARROW_TIP_R: f32 = 4.0;
    pub const PALETTE_RADIUS: f32 = 9.0;
    pub const PALETTE_FIELD_H: f32 = 30.0;
    pub const PALETTE_ROW_H: f32 = 20.0;
    pub const CARD_RADIUS: f32 = 12.0;
    pub const CONTROL_RADIUS: f32 = 7.0;
    /// Small key-hint and count badges.
    pub const BADGE_RADIUS: f32 = 4.0;
    pub const SWITCH_W: f32 = 44.0;
    pub const SWITCH_H: f32 = 22.0;
    pub const SIDEBAR_ROW_H: f32 = 32.0;
    pub const SIDEBAR_GLYPH: f32 = 16.0;
    pub const TOOLBAR_BUTTON: f32 = 26.0;
    pub const TOOLBAR_GLYPH: f32 = 14.0;
    pub const OMNIBAR_H: f32 = 27.0; // measured on macOS 27 (cmux source says 18 + padding)
    pub const OMNIBAR_RADIUS: f32 = 10.0;
    pub const OMNIBAR_TEXT: f32 = 12.0;
    pub const OMNIBAR_PAD_X: f32 = 8.0;
    /// The https lock before the omnibar text.
    pub const LOCK_GLYPH: f32 = 11.0;
    /// Gap kept between an overlay and the window edge.
    pub const OVERLAY_MARGIN: f32 = 8.0;
}

/// Enter/leave timing for popovers, menus and the palette (plan K5).
pub mod motion {
    use std::time::Duration;

    pub const ENTER: Duration = Duration::from_millis(180);
    pub const LEAVE: Duration = Duration::from_millis(120);
    /// How far an entering overlay slides in from its anchor side.
    pub const SLIDE: f32 = 6.0;

    /// Cubic ease-out, clamped to 0..=1.
    pub fn ease_out(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }
}

/// `c` with its alpha multiplied by `k` (SwiftUI `color.opacity(k)`).
pub fn scale_alpha(c: Rgba, k: f32) -> Rgba {
    Rgba { a: c.a * k, ..c }
}

/// `c` with its alpha replaced.
pub fn alpha(c: Rgba, a: f32) -> Rgba {
    Rgba { a, ..c }
}

/// Soft two-layer shadow under popovers and menus (plan K4).
pub fn popover_shadow() -> Vec<BoxShadow> {
    let black = |a: f32| Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a,
    };
    vec![
        BoxShadow {
            color: black(0.30).into(),
            offset: point(px(0.), px(10.)),
            blur_radius: px(30.),
            spread_radius: px(0.),
        },
        BoxShadow {
            color: black(0.30).into(),
            offset: point(px(0.), px(0.)),
            blur_radius: px(1.),
            spread_radius: px(0.),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(r: f32, g: f32, b: f32) -> Rgba {
        Rgba { r, g, b, a: 1.0 }
    }

    #[test]
    fn surfaces_are_opaque_and_edges_follow_appearance() {
        let dark = Theme::new(rgba(0.1, 0.1, 0.1));
        let light = Theme::new(rgba(0.95, 0.95, 0.95));
        assert_eq!((dark.popover.a, dark.menu.a), (1.0, 1.0));
        assert_eq!((dark.popover.r * 255.0).round(), 30.0);
        assert_eq!((dark.menu.r * 255.0).round(), 61.0);
        assert_eq!((dark.edge_outer.r, dark.edge_outer.a), (0.0, 0.9));
        assert_eq!((dark.edge_inner.r, dark.edge_inner.a), (1.0, 0.10));
        assert_eq!((light.edge_outer.a, light.edge_inner.a), (0.5, 1.0));
        assert_eq!(dark.palette, dark.bg);
        assert_eq!(dark.palette_select.a, 0.14);
        assert!(
            dark.card.r < dark.bg.r,
            "dark cards are darker than the window"
        );
        assert_eq!(dark.menu_hover, dark.accent);
    }

    #[test]
    fn ease_out_starts_fast_and_ends_at_one() {
        assert_eq!(motion::ease_out(0.0), 0.0);
        assert_eq!(motion::ease_out(1.0), 1.0);
        assert!(motion::ease_out(0.5) > 0.8);
        assert_eq!(motion::ease_out(2.0), 1.0);
    }
}
