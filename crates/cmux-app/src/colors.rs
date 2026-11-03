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
    pub fg_alpha: f32,
}

pub fn resolve(cell: &Cell, default_fg: Rgb, default_bg: Rgb, faint_opacity: f32) -> CellColors {
    let mut fg = cell.resolved_fg.unwrap_or(default_fg);
    let mut bg = cell.resolved_bg;
    if cell.inverse {
        let old_fg = fg;
        fg = bg.unwrap_or(default_bg);
        bg = Some(old_fg);
    }
    let fg_alpha = if cell.invisible {
        0.0
    } else if cell.faint {
        faint_opacity
    } else {
        1.0
    };
    CellColors { fg, bg, fg_alpha }
}

fn pick(c: DynColor, cell: CellColors, default_bg: Rgb) -> Rgb {
    match c {
        DynColor::Rgb(rgb) => to_vt(rgb),
        DynColor::CellForeground => cell.fg,
        DynColor::CellBackground => cell.bg.unwrap_or(default_bg),
    }
}

/// Ghostty's selection colours: unset swaps the cell's fg and bg.
pub fn selected(
    c: CellColors,
    default_bg: Rgb,
    fg: Option<DynColor>,
    bg: Option<DynColor>,
) -> CellColors {
    let cell_bg = c.bg.unwrap_or(default_bg);
    CellColors {
        fg: fg.map_or(cell_bg, |d| pick(d, c, default_bg)),
        bg: Some(bg.map_or(c.fg, |d| pick(d, c, default_bg))),
        fg_alpha: c.fg_alpha,
    }
}

/// (cursor fill, glyph colour under a block cursor). An OSC 12 colour beats the config;
/// unset config uses the cell's foreground for the fill and its background for the glyph.
pub fn cursor(
    c: CellColors,
    default_bg: Rgb,
    osc: Option<Rgb>,
    color: Option<DynColor>,
    text: Option<DynColor>,
) -> (Rgb, Rgb) {
    let fill = osc
        .or(color.map(|d| pick(d, c, default_bg)))
        .unwrap_or(c.fg);
    let glyph = text.map_or(c.bg.unwrap_or(default_bg), |d| pick(d, c, default_bg));
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
        resolve(&cell, FG, BG, 0.5)
    }

    #[test]
    fn default_cell_uses_default_fg_and_no_bg() {
        assert_eq!(
            colors(Cell::default()),
            CellColors {
                fg: FG,
                bg: None,
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

    #[test]
    fn selection_defaults_to_swapping() {
        let c = selected(colors(Cell::default()), BG, None, None);
        assert_eq!((c.fg, c.bg), (BG, Some(FG)));
    }

    #[test]
    fn selection_colours_from_config() {
        let red = DynColor::Rgb(config::Rgb { r: 255, g: 0, b: 0 });
        let base = colors(Cell {
            resolved_fg: Some(BLUE),
            ..Default::default()
        });
        let c = selected(base, BG, Some(DynColor::CellForeground), Some(red));
        assert_eq!((c.fg, c.bg), (BLUE, Some(RED)));
    }

    #[test]
    fn cursor_colours() {
        let base = colors(Cell::default());
        assert_eq!(cursor(base, BG, None, None, None), (FG, BG));
        let red = DynColor::Rgb(config::Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(
            cursor(base, BG, None, Some(red), Some(DynColor::CellForeground)),
            (RED, FG)
        );
        assert_eq!(
            cursor(base, BG, Some(BLUE), Some(red), None),
            (BLUE, BG),
            "OSC 12 wins"
        );
    }

    #[test]
    fn hsla_keeps_alpha() {
        assert_eq!(hsla(RED, 0.25).a, 0.25);
    }
}
