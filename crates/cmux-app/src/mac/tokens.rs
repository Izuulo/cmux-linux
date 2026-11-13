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
    /// Popover, menu and palette fill (plan K4: opaque; without blur any see-through shows text behind).
    pub glass: Rgba,
    pub glass_edge: Rgba,
    /// Menu row under the pointer: accent fill, white text (NSMenu).
    pub menu_hover: Rgba,
    pub menu_hover_text: Rgba,
}

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
        // macOS controlBackgroundColor at 0.76 (sui `SettingsCard`): a step off the backdrop.
        let card = chrome::shift(bg, if light { 0.06 } else { 0.05 }, 0.76);
        Theme {
            bg,
            sidebar: chrome::shift(bg, if light { -0.03 } else { 0.025 }, 1.0),
            card,
            separator: Rgba {
                a: p.border.a * 0.5,
                ..p.border
            },
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
            glass: chrome::shift(bg, if light { 0.04 } else { 0.06 }, 1.0),
            glass_edge: if light { ink(0.10) } else { ink(0.14) },
            menu_hover: p.accent,
            menu_hover_text: Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
        }
    }
}

/// Tahoe metrics in logical pixels before UI scale (plan K3). Calibrated in Task 11.
pub mod metric {
    pub const WINDOW_RADIUS: f32 = 16.0;
    pub const TITLEBAR_H: f32 = 28.0;
    pub const TITLE_TEXT: f32 = 13.0;
    pub const BODY_TEXT: f32 = 13.0;
    pub const SMALL_TEXT: f32 = 11.0;
    pub const MENU_RADIUS: f32 = 12.0;
    pub const MENU_INSET: f32 = 5.0;
    pub const MENU_ITEM_H: f32 = 24.0;
    pub const MENU_ITEM_RADIUS: f32 = 7.0;
    pub const MENU_ITEM_PAD_X: f32 = 10.0;
    pub const MENU_W: f32 = 220.0;
    pub const POPOVER_RADIUS: f32 = 16.0;
    pub const ARROW_W: f32 = 26.0;
    pub const ARROW_H: f32 = 11.0;
    pub const ARROW_TIP_R: f32 = 3.0;
    pub const PALETTE_RADIUS: f32 = 16.0;
    pub const CARD_RADIUS: f32 = 13.0;
    pub const CONTROL_RADIUS: f32 = 7.0;
    /// Small key-hint and count badges.
    pub const BADGE_RADIUS: f32 = 4.0;
    pub const SWITCH_W: f32 = 38.0;
    pub const SWITCH_H: f32 = 22.0;
    pub const TOOLBAR_BUTTON: f32 = 26.0;
    pub const TOOLBAR_GLYPH: f32 = 14.0;
    pub const OMNIBAR_H: f32 = 22.0;
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

/// Soft two-layer shadow under glass (plan K4).
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
    fn glass_is_opaque_and_edge_follows_appearance() {
        let dark = Theme::new(rgba(0.1, 0.1, 0.1));
        let light = Theme::new(rgba(0.95, 0.95, 0.95));
        assert_eq!(dark.glass.a, 1.0);
        assert_eq!((dark.glass_edge.r, dark.glass_edge.a), (1.0, 0.14));
        assert_eq!((light.glass_edge.r, light.glass_edge.a), (0.0, 0.10));
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
