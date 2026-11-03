//! Cell colour resolution. libghostty-vt resolves palette indices but leaves the defaults,
//! inverse, faint and invisible to the renderer (vendor/ghostty/include/ghostty/vt/render.h).

use cmux_config::ghostty::{self as config, DynColor};
use cmux_term::ghostty_vt::{Cell, Rgb};
use gpui::{Hsla, Rgba};

pub fn to_vt(c: config::Rgb) -> Rgb {
    Rgb {
        r: c.r,
        g: c.g,
        b: c.b,
    }
}

pub fn hsla(c: Rgb, alpha: f32) -> Hsla {
    Rgba {
        r: f32::from(c.r) / 255.0,
        g: f32::from(c.g) / 255.0,
        b: f32::from(c.b) / 255.0,
        a: alpha,
    }
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellColors {
    pub fg: Rgb,
    /// `None` = the default background: no quad, so window opacity shows through.
    pub bg: Option<Rgb>,
    /// Alpha of the `bg` quad (generic.zig ~3414: selected and inverse cells are opaque).
    pub bg_alpha: f32,
    /// Glyph alpha. 0.0 for invisible text, which hides only the glyph: the painter must still
    /// draw underline, strikethrough and the other decorations, as Ghostty does.
    pub fg_alpha: f32,
}

/// `explicit_bg_alpha`: alpha for cells with an explicit background, i.e. `background-opacity`
/// when `background-opacity-cells` is on (generic.zig ~3426), else 1.0.
pub fn resolve(
    cell: &Cell,
    default_fg: Rgb,
    default_bg: Rgb,
    faint_opacity: f32,
    explicit_bg_alpha: f32,
) -> CellColors {
    let mut fg = cell.resolved_fg.unwrap_or(default_fg);
    let mut bg = cell.resolved_bg;
    let mut bg_alpha = explicit_bg_alpha;
    if cell.inverse {
        let old_fg = fg;
        fg = bg.unwrap_or(default_bg);
        bg = Some(old_fg);
        bg_alpha = 1.0;
    }
    let fg_alpha = if cell.invisible {
        0.0
    } else if cell.faint {
        faint_opacity
    } else {
        1.0
    };
    CellColors {
        fg,
        bg,
        bg_alpha,
        fg_alpha,
    }
}

fn pick(c: DynColor, cell: CellColors, default_bg: Rgb) -> Rgb {
    match c {
        DynColor::Rgb(rgb) => to_vt(rgb),
        DynColor::CellForeground => cell.fg,
        DynColor::CellBackground => cell.bg.unwrap_or(default_bg),
    }
}

/// Ghostty's selection colours (generic.zig ~3342, ~3391): unset uses the default foreground
/// as the background and the default background as the foreground.
pub fn selected(
    c: CellColors,
    default_fg: Rgb,
    default_bg: Rgb,
    fg: Option<DynColor>,
    bg: Option<DynColor>,
) -> CellColors {
    CellColors {
        fg: fg.map_or(default_bg, |d| pick(d, c, default_bg)),
        bg: Some(bg.map_or(default_fg, |d| pick(d, c, default_bg))),
        bg_alpha: 1.0,
        fg_alpha: c.fg_alpha,
    }
}

/// (cursor fill, glyph colour under a block cursor). An OSC 12 colour beats the config; unset
/// config uses the default foreground for the fill (generic.zig ~3034) and the default
/// background for the glyph (~3101), also when OSC 12 sets the fill.
pub fn cursor(
    c: CellColors,
    default_fg: Rgb,
    default_bg: Rgb,
    osc: Option<Rgb>,
    color: Option<DynColor>,
    text: Option<DynColor>,
) -> (Rgb, Rgb) {
    let fill = osc
        .or(color.map(|d| pick(d, c, default_bg)))
        .unwrap_or(default_fg);
    let glyph = text.map_or(default_bg, |d| pick(d, c, default_bg));
    (fill, glyph)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FG: Rgb = Rgb {
        r: 200,
        g: 200,
        b: 200,
    };
    const BG: Rgb = Rgb {
        r: 10,
        g: 10,
        b: 10,
    };
    const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };
    const BLUE: Rgb = Rgb { r: 0, g: 0, b: 255 };

    fn colors(cell: Cell) -> CellColors {
        resolve(&cell, FG, BG, 0.5, 1.0)
    }

    #[test]
    fn default_cell_uses_default_fg_and_no_bg() {
        assert_eq!(
            colors(Cell::default()),
            CellColors {
                fg: FG,
                bg: None,
                bg_alpha: 1.0,
                fg_alpha: 1.0
            }
        );
    }

    #[test]
    fn explicit_colours_pass_through() {
        let c = colors(Cell {
            resolved_fg: Some(RED),
            resolved_bg: Some(BLUE),
            ..Default::default()
        });
        assert_eq!((c.fg, c.bg), (RED, Some(BLUE)));
    }

    #[test]
    fn inverse_swaps_including_defaults() {
        let c = colors(Cell {
            inverse: true,
            ..Default::default()
        });
        assert_eq!((c.fg, c.bg), (BG, Some(FG)));
        let c = colors(Cell {
            inverse: true,
            resolved_fg: Some(RED),
            resolved_bg: Some(BLUE),
            ..Default::default()
        });
        assert_eq!((c.fg, c.bg), (BLUE, Some(RED)));
    }

    #[test]
    fn faint_and_invisible_set_alpha() {
        assert_eq!(
            colors(Cell {
                faint: true,
                ..Default::default()
            })
            .fg_alpha,
            0.5
        );
        assert_eq!(
            colors(Cell {
                invisible: true,
                faint: true,
                ..Default::default()
            })
            .fg_alpha,
            0.0
        );
    }

    fn red_on_blue() -> CellColors {
        colors(Cell {
            resolved_fg: Some(RED),
            resolved_bg: Some(BLUE),
            ..Default::default()
        })
    }

    #[test]
    fn unset_selection_uses_default_fg_as_bg_and_default_bg_as_fg() {
        let c = selected(red_on_blue(), FG, BG, None, None);
        assert_eq!((c.fg, c.bg), (BG, Some(FG)));
        assert_eq!(c.bg_alpha, 1.0);
    }

    #[test]
    fn selection_colours_from_config() {
        let red = DynColor::Rgb(config::Rgb { r: 255, g: 0, b: 0 });
        let base = colors(Cell {
            resolved_fg: Some(BLUE),
            ..Default::default()
        });
        let c = selected(base, FG, BG, Some(DynColor::CellForeground), Some(red));
        assert_eq!((c.fg, c.bg), (BLUE, Some(RED)));
        let c = selected(
            red_on_blue(),
            FG,
            BG,
            Some(DynColor::CellBackground),
            Some(DynColor::CellForeground),
        );
        assert_eq!((c.fg, c.bg), (BLUE, Some(RED)));
    }

    #[test]
    fn unset_cursor_uses_default_fg_fill_and_default_bg_glyph() {
        let base = red_on_blue();
        assert_eq!(cursor(base, FG, BG, None, None, None), (FG, BG));
        let osc = Rgb { r: 1, g: 2, b: 3 };
        assert_eq!(
            cursor(base, FG, BG, Some(osc), None, None),
            (osc, BG),
            "OSC 12 leaves the glyph at the default bg"
        );
    }

    #[test]
    fn configured_cursor_colours() {
        let base = red_on_blue();
        let green = DynColor::Rgb(config::Rgb { r: 0, g: 255, b: 0 });
        let green_vt = Rgb { r: 0, g: 255, b: 0 };
        assert_eq!(
            cursor(
                base,
                FG,
                BG,
                None,
                Some(green),
                Some(DynColor::CellForeground)
            ),
            (green_vt, RED)
        );
        assert_eq!(
            cursor(base, FG, BG, Some(BLUE), Some(green), None),
            (BLUE, BG),
            "OSC 12 wins"
        );
    }

    #[test]
    fn opacity_cells_applies_to_explicit_backgrounds_only() {
        let cell = |bg, inverse| Cell {
            resolved_bg: bg,
            inverse,
            ..Default::default()
        };
        let at = |cell: Cell| resolve(&cell, FG, BG, 0.5, 0.25);
        assert_eq!(at(cell(Some(BLUE), false)).bg_alpha, 0.25);
        assert_eq!(at(cell(None, false)).bg, None);
        assert_eq!(
            at(cell(Some(BLUE), true)).bg_alpha,
            1.0,
            "inverse is opaque"
        );
        assert_eq!(
            colors(cell(Some(BLUE), false)).bg_alpha,
            1.0,
            "off = opaque"
        );
    }

    #[test]
    fn hsla_keeps_alpha() {
        assert_eq!(hsla(RED, 0.25).a, 0.25);
    }
}
