//! Chrome colours derived from the terminal background with cmux's formulas
//! (research-cmux.md §4): separators and dividers (`WindowChromeColorResolver.separatorColor`),
//! the unfocused-pane dim (Ghostty `unfocused-split-*`).

use cmux_config::ghostty::{Config, Rgb};
use gpui::Rgba;

pub fn rgba(c: Rgb, a: f32) -> Rgba {
    Rgba {
        r: f32::from(c.r) / 255.0,
        g: f32::from(c.g) / 255.0,
        b: f32::from(c.b) / 255.0,
        a,
    }
}

/// Each RGB channel moved by `amount` and clamped; alpha replaced.
pub fn shift(c: Rgba, amount: f32, alpha: f32) -> Rgba {
    let f = |v: f32| (v + amount).clamp(0.0, 1.0);
    Rgba {
        r: f(c.r),
        g: f(c.g),
        b: f(c.b),
        a: alpha,
    }
}

/// cmux's separator test: Rec. 601 luma above 0.5.
fn luma_is_light(c: Rgba) -> bool {
    0.299 * c.r + 0.587 * c.g + 0.114 * c.b > 0.5
}

/// Tab-bar separators and the default divider: light bg -0.30 @ 0.26, dark bg +0.16 @ 0.36.
pub fn separator(bg: Rgba) -> Rgba {
    if luma_is_light(bg) {
        shift(bg, -0.30, 0.26)
    } else {
        shift(bg, 0.16, 0.36)
    }
}

/// Ghostty `split-divider-color` if set, else the separator colour (cmux `Workspace.swift:3963`).
pub fn divider(cfg: &Config) -> Rgba {
    cfg.split_divider_color
        .map_or_else(|| separator(rgba(cfg.background, 1.0)), |c| rgba(c, 1.0))
}

/// The overlay on unfocused panes: `unfocused-split-fill` (else the background) at alpha
/// `1 - unfocused-split-opacity`. `None` when that is fully transparent.
pub fn dim_overlay(cfg: &Config) -> Option<Rgba> {
    let alpha = 1.0 - cfg.unfocused_split_opacity.clamp(0.15, 1.0);
    (alpha > 0.0).then(|| rgba(cfg.unfocused_split_fill.unwrap_or(cfg.background), alpha))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Rgba, b: Rgba) -> bool {
        [a.r - b.r, a.g - b.g, a.b - b.b, a.a - b.a]
            .iter()
            .all(|d| d.abs() < 1e-4)
    }

    #[test]
    fn separator_steps_away_from_the_background() {
        let dark = rgba(
            Rgb {
                r: 0x28,
                g: 0x2c,
                b: 0x34,
            },
            1.0,
        );
        let want = Rgba {
            r: 0x28 as f32 / 255.0 + 0.16,
            g: 0x2c as f32 / 255.0 + 0.16,
            b: 0x34 as f32 / 255.0 + 0.16,
            a: 0.36,
        };
        assert!(close(separator(dark), want));
        let light = rgba(
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            1.0,
        );
        assert!(close(
            separator(light),
            Rgba {
                r: 0.7,
                g: 0.7,
                b: 0.7,
                a: 0.26
            }
        ));
    }

    #[test]
    fn divider_prefers_the_ghostty_colour() {
        let mut cfg = Config::default();
        assert!(close(divider(&cfg), separator(rgba(cfg.background, 1.0))));
        cfg.split_divider_color = Some(Rgb { r: 255, g: 0, b: 0 });
        assert!(close(
            divider(&cfg),
            Rgba {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0
            }
        ));
    }

    #[test]
    fn dim_overlay_uses_ghostty_opacity_and_fill() {
        let mut cfg = Config::default();
        let dim = dim_overlay(&cfg).unwrap();
        assert!((dim.a - 0.3).abs() < 1e-6);
        assert!(close(Rgba { a: 1.0, ..dim }, rgba(cfg.background, 1.0)));
        cfg.unfocused_split_fill = Some(Rgb { r: 0, g: 0, b: 0 });
        assert!(close(
            dim_overlay(&cfg).unwrap(),
            Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.3
            }
        ));
        cfg.unfocused_split_opacity = 1.0;
        assert_eq!(dim_overlay(&cfg), None);
    }
}
