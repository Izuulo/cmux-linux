//! Chrome colours derived from the terminal background with cmux's formulas
//! (research-cmux.md §4): separators and dividers (`WindowChromeColorResolver.separatorColor`),
//! the unfocused-pane dim (Ghostty `unfocused-split-*`), and the pane tab strip
//! (Bonsplit `TabBarColors.swift`).

use cmux_config::ghostty::{Config, Rgb};
use gpui::Rgba;

/// Hover overlay on a shared-backdrop tab bar (Bonsplit `TabBarColors.swift:194`).
const HOVER_LIGHT_ALPHA: f32 = 0.055;
const HOVER_DARK_ALPHA: f32 = 0.075;

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

/// Bonsplit `isBonsplitLightColor`: WCAG relative luminance, black text contrasts more.
pub fn wcag_is_light(c: Rgba) -> bool {
    let lin = |v: f32| {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let l = 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
    (l + 0.05) / 0.05 > 1.05 / (l + 0.05)
}

/// Saturation 0 (Bonsplit `bonsplitSaturating(by: 0)`): every channel becomes the Rec. 601 luma.
pub fn desaturate(c: Rgba) -> Rgba {
    let l = 0.299 * c.r + 0.587 * c.g + 0.114 * c.b;
    Rgba {
        r: l,
        g: l,
        b: l,
        a: c.a,
    }
}

/// macOS's default `controlAccentColor` (systemBlue): #007AFF in light, #0A84FF in dark
/// appearance. cmux draws the selected-tab indicator in it; Linux has no portable accent, so
/// we use Apple's value matched to the bar's lightness.
pub fn accent(light_bar: bool) -> Rgba {
    if light_bar {
        rgba(
            Rgb {
                r: 0x00,
                g: 0x7a,
                b: 0xff,
            },
            1.0,
        )
    } else {
        rgba(
            Rgb {
                r: 0x0a,
                g: 0x84,
                b: 0xff,
            },
            1.0,
        )
    }
}

/// cmux's own accent (`CmuxAccentColor.swift`): #0088FF on a light background, #0091FF on dark.
pub fn cmux_accent(light_bg: bool) -> Rgba {
    let g = if light_bg { 0x88 } else { 0x91 };
    rgba(Rgb { r: 0, g, b: 0xff }, 1.0)
}

/// Colours of the workspace sidebar (plan D3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidebarPalette {
    pub bg: Rgba,
    pub border: Rgba,
    pub selected_fill: Rgba,
    pub title: Rgba,
    pub detail: Rgba,
    pub selected_title: Rgba,
    pub selected_detail: Rgba,
    pub accent: Rgba,
}

/// The sidebar sits on the terminal backdrop `bg`; selected rows are solid accent with white
/// text, unselected text approximates the macOS label colours.
pub fn sidebar_palette(bg: Rgba) -> SidebarPalette {
    let light = wcag_is_light(bg);
    let v = if light { 0.0 } else { 1.0 };
    let label = |a: f32| Rgba {
        r: v,
        g: v,
        b: v,
        a,
    };
    let white = |a: f32| Rgba {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a,
    };
    SidebarPalette {
        bg,
        border: separator(bg),
        selected_fill: cmux_accent(light),
        title: label(0.85),
        detail: label(if light { 0.5 } else { 0.55 }),
        selected_title: white(1.0),
        selected_detail: white(0.75),
        accent: cmux_accent(light),
    }
}

/// Colours of one pane's tab strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabPalette {
    pub bar: Rgba,
    pub active_fill: Rgba,
    pub hover_fill: Rgba,
    pub active_text: Rgba,
    pub inactive_text: Rgba,
    pub separator: Rgba,
    pub accent: Rgba,
}

/// The strip palette for a bar drawn on `bg`; an unfocused pane's strip is greyscale
/// (Bonsplit `TabBarView.swift:923`). cmux always shares the window backdrop
/// (`Workspace.swift:3886`), so Bonsplit's `usesSharedBackdrop` fills apply: no active-tab
/// fill and a flat hover overlay (`TabBarColors.swift:177, 194`).
pub fn tab_palette(bg: Rgba, focused: bool) -> TabPalette {
    let light = wcag_is_light(bg);
    let white_or_black = |a: f32| {
        let v = if light { 0.0 } else { 1.0 };
        Rgba {
            r: v,
            g: v,
            b: v,
            a,
        }
    };
    let palette = TabPalette {
        bar: bg,
        active_fill: Rgba { a: 0.0, ..bg },
        hover_fill: if light {
            Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: HOVER_LIGHT_ALPHA,
            }
        } else {
            Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: HOVER_DARK_ALPHA,
            }
        },
        active_text: white_or_black(0.82),
        inactive_text: white_or_black(if light { 0.62 } else { 0.68 }),
        separator: separator(bg),
        accent: accent(light),
    };
    if focused {
        return palette;
    }
    TabPalette {
        bar: desaturate(palette.bar),
        active_fill: desaturate(palette.active_fill),
        hover_fill: desaturate(palette.hover_fill),
        active_text: palette.active_text,
        inactive_text: palette.inactive_text,
        separator: desaturate(palette.separator),
        accent: desaturate(palette.accent),
    }
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
    fn tab_palette_follows_bonsplit_on_dark_and_light_bars() {
        let dark = rgba(
            Rgb {
                r: 0x28,
                g: 0x2c,
                b: 0x34,
            },
            1.0,
        );
        let p = tab_palette(dark, true);
        assert_eq!(p.active_fill.a, 0.0);
        assert!(close(
            p.hover_fill,
            Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.075
            }
        ));
        assert!(close(
            p.active_text,
            Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.82
            }
        ));
        assert!(close(
            p.inactive_text,
            Rgba {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.68
            }
        ));
        assert!(close(p.accent, accent(false)));
        let light = rgba(
            Rgb {
                r: 0xfa,
                g: 0xfa,
                b: 0xfa,
            },
            1.0,
        );
        let p = tab_palette(light, true);
        assert_eq!(p.active_fill.a, 0.0);
        assert!(close(
            p.hover_fill,
            Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.055
            }
        ));
        assert!(close(
            p.inactive_text,
            Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.62
            }
        ));
    }

    #[test]
    fn an_unfocused_strip_is_greyscale() {
        let p = tab_palette(
            rgba(
                Rgb {
                    r: 0x28,
                    g: 0x2c,
                    b: 0x34,
                },
                1.0,
            ),
            false,
        );
        assert_eq!(p.accent.r, p.accent.g);
        assert_eq!(p.accent.g, p.accent.b);
        assert_eq!(p.active_fill.r, p.active_fill.b);
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
    fn sidebar_palette_uses_cmux_accent_and_label_alphas() {
        let dark = Rgba {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        };
        let p = sidebar_palette(dark);
        assert_eq!(p.selected_fill, cmux_accent(false));
        assert_eq!((p.title.r, p.title.a), (1.0, 0.85));
        assert_eq!(p.detail.a, 0.55);
        assert_eq!((p.selected_title.r, p.selected_title.a), (1.0, 1.0));
        assert_eq!(p.selected_detail.a, 0.75);
        let light = sidebar_palette(Rgba {
            r: 0.97,
            g: 0.97,
            b: 0.97,
            a: 1.0,
        });
        assert_eq!(light.selected_fill, cmux_accent(true));
        assert_eq!((light.title.r, light.detail.a), (0.0, 0.5));
        assert_eq!(
            light.border,
            separator(Rgba {
                r: 0.97,
                g: 0.97,
                b: 0.97,
                a: 1.0
            })
        );
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
