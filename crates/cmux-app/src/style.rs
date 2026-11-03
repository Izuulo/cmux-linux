//! The Ghostty config turned into what the renderer and the terminal need.

use std::borrow::Cow;

use cmux_config::ghostty::{Config, CursorStyle, DynColor, MetricModifier};
use cmux_term::Term;
use cmux_term::ghostty_vt::CursorShape;
use gpui::{App, Font, FontFallbacks, FontFeatures, FontStyle, FontWeight, Hsla};

use crate::colors::{hsla, to_vt};

/// Ghostty's default font (font/embedded.zig), bundled in assets/fonts.
pub const DEFAULT_FONT_FAMILY: &str = "JetBrains Mono";

/// Ghostty sizes fonts in points at 96 dpi on Linux (font/face.zig `default_dpi`).
// shortcut: Linux only; macOS uses 72 dpi. Switch on target_os in the M6 macOS build.
const POINTS_TO_PX: f32 = 96.0 / 72.0;

pub struct TermStyle {
    pub cfg: Config,
    /// Regular face; bold and italic are derived per cell.
    pub font: Font,
    /// Logical pixels.
    pub font_px: f32,
}

impl TermStyle {
    /// `installed`: the loadable family names, from [`installed_families`] (call that after
    /// [`load_bundled_fonts`]).
    pub fn new(cfg: &Config, installed: &[String]) -> Self {
        let primary = pick_family(&cfg.font_family, installed);
        // gpui's Linux text system ignores `fallbacks` (text_system.rs "TODO: Remove this when
        // Linux have implemented setting fallbacks"), so `pick_family` does the real work there.
        let mut fallbacks: Vec<String> = cfg
            .font_family
            .iter()
            .filter(|f| **f != primary)
            .cloned()
            .collect();
        fallbacks.push(DEFAULT_FONT_FAMILY.to_string());
        let font = Font {
            family: primary.into(),
            // Ligatures stay off: force_width snaps glyphs, so a ligature would shift the grid.
            features: FontFeatures::disable_ligatures(),
            fallbacks: Some(FontFallbacks::from_fonts(fallbacks)),
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        };
        Self {
            cfg: cfg.clone(),
            font,
            font_px: cfg.font_size * POINTS_TO_PX,
        }
    }

    /// Window fill: the configured background at `background-opacity`.
    // shortcut: ignores an app's OSC 11 background change; repaint from FrameInfo.default_bg if needed.
    pub fn window_background(&self) -> Hsla {
        hsla(to_vt(self.cfg.background), self.cfg.background_opacity)
    }

    /// Sets the terminal's default colours, palette and cursor. Call before the first frame
    /// so OSC 4/10/11 queries already answer with the config's colours.
    pub fn apply_to(&self, term: &mut Term) {
        let cfg = &self.cfg;
        let vt = term.vt_mut();
        let cursor = match cfg.cursor_color {
            Some(DynColor::Rgb(c)) => Some(to_vt(c)),
            _ => None,
        };
        vt.replace_default_colors(
            Some(to_vt(cfg.foreground)),
            Some(to_vt(cfg.background)),
            cursor,
        );
        let palette = cfg.palette.map(|c| c.map(to_vt));
        vt.set_default_palette(&palette);
        let shape = match cfg.cursor_style {
            CursorStyle::Block => CursorShape::Block,
            CursorStyle::Bar => CursorShape::Bar,
            CursorStyle::Underline => CursorShape::Underline,
            CursorStyle::BlockHollow => CursorShape::BlockHollow,
        };
        vt.set_default_cursor(Some(shape), cfg.cursor_style_blink);
    }
}

/// The first configured family that is installed, else the bundled [`DEFAULT_FONT_FAMILY`].
/// gpui's Linux `resolve_font` would otherwise walk its own `.ZedMono/Helvetica/...` stack
/// and never reach the bundled font.
pub fn pick_family(configured: &[String], installed: &[String]) -> String {
    configured
        .iter()
        .find(|f| installed.contains(f))
        .map_or_else(|| DEFAULT_FONT_FAMILY.to_string(), Clone::clone)
}

/// Every family name gpui's text system knows. Call after [`load_bundled_fonts`], which must
/// itself run before the first `font_id` lookup because gpui caches failed lookups
/// (text_system.rs `font_id`), so an early "JetBrains Mono" lookup would stay failed.
// shortcut: gpui also lists its own fallback stack (Helvetica, Ubuntu, ...) whether or not it is
// installed, so configuring one of those uninstalled picks it; probe `resolve_font` if that bites.
pub fn installed_families(cx: &App) -> Vec<String> {
    cx.text_system().all_font_names()
}

/// Cell size in logical pixels, snapped to whole device pixels the way Ghostty rounds
/// (font/Metrics.zig: `cell_width = @round(face_width)`, `cell_height = @round(ascent - descent + line_gap)`).
/// gpui's Linux `font_metrics` negates swash's descent (platform/linux/text_system.rs), so it is
/// <= 0 as in Ghostty; `abs` also accepts the positive convention. `line_gap` is
/// `FontMetrics::line_gap(font_size)`, the font's leading.
pub fn cell_size(
    advance: f32,
    ascent: f32,
    descent: f32,
    line_gap: f32,
    scale: f32,
    adjust: Option<MetricModifier>,
) -> (f32, f32) {
    let w = (advance * scale).round().max(1.0);
    let mut h = ((ascent + descent.abs() + line_gap) * scale).round();
    if let Some(m) = adjust {
        h = m.apply(h);
    }
    (w / scale, h.max(1.0) / scale)
}

/// Registers the bundled JetBrains Mono faces with gpui. Call before any `font_id` lookup.
pub fn load_bundled_fonts(cx: &App) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Italic.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-BoldItalic.ttf").as_slice()),
    ];
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        eprintln!("cmux-linux: could not load the bundled font: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cmux_config::ghostty::Rgb;

    fn term() -> (Term, std::sync::mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = std::sync::mpsc::channel();
        (Term::new(20, 3, tx).unwrap(), rx)
    }

    fn reply(term: &mut Term, rx: &std::sync::mpsc::Receiver<Vec<u8>>, query: &[u8]) -> String {
        term.feed(query);
        String::from_utf8(rx.try_recv().unwrap_or_default()).unwrap()
    }

    #[test]
    fn default_font_is_bundled_jetbrains_mono_at_12pt() {
        let style = TermStyle::new(&Config::default(), &[]);
        assert_eq!(style.font.family.as_ref(), DEFAULT_FONT_FAMILY);
        assert_eq!(style.font.features.is_calt_enabled(), Some(false));
        assert_eq!(style.font_px, 16.0);
    }

    #[test]
    fn user_fonts_come_first_then_the_bundled_fallback() {
        let cfg = Config {
            font_family: vec!["Fira Code".into(), "Noto Emoji".into()],
            ..Default::default()
        };
        let style = TermStyle::new(&cfg, &names(&["Fira Code"]));
        assert_eq!(style.font.family.as_ref(), "Fira Code");
        let fallbacks = style.font.fallbacks.unwrap();
        assert_eq!(
            fallbacks.fallback_list(),
            ["Noto Emoji", DEFAULT_FONT_FAMILY]
        );
    }

    #[test]
    fn apply_to_answers_colour_queries_with_the_config() {
        let mut cfg = Config::default();
        cfg.palette[1] = Some(Rgb { r: 255, g: 0, b: 0 });
        let (mut t, rx) = term();
        TermStyle::new(&cfg, &[]).apply_to(&mut t);
        assert!(reply(&mut t, &rx, b"\x1b]11;?\x07").contains("rgb:2828/2c2c/3434"));
        assert!(reply(&mut t, &rx, b"\x1b]10;?\x07").contains("rgb:ffff/ffff/ffff"));
        assert!(reply(&mut t, &rx, b"\x1b]4;1;?\x07").contains("rgb:ffff/0000/0000"));
    }

    #[test]
    fn apply_to_sets_the_cursor_shape() {
        let cfg = Config {
            cursor_style: CursorStyle::Bar,
            ..Default::default()
        };
        let (mut t, _rx) = term();
        TermStyle::new(&cfg, &[]).apply_to(&mut t);
        assert_eq!(
            t.update_frame().unwrap().cursor.unwrap().shape,
            CursorShape::Bar
        );
    }

    #[test]
    fn cell_size_snaps_to_device_pixels() {
        assert_eq!(cell_size(9.6, 13.0, -3.0, 0.0, 1.0, None), (10.0, 16.0));
        assert_eq!(cell_size(9.6, 13.0, 3.0, 0.0, 2.0, None), (9.5, 16.0));
        assert_eq!(
            cell_size(9.6, 13.0, -3.0, 0.0, 1.0, Some(MetricModifier::Absolute(2))),
            (10.0, 18.0)
        );
        assert_eq!(
            cell_size(
                9.6,
                13.0,
                -3.0,
                0.0,
                1.0,
                Some(MetricModifier::Percent(1.25))
            ),
            (10.0, 20.0)
        );
    }

    #[test]
    fn cell_size_rounds_to_nearest() {
        assert_eq!(cell_size(9.4, 13.0, -3.0, 0.0, 1.0, None).0, 9.0);
        assert_eq!(cell_size(9.5, 13.0, -3.0, 0.0, 1.0, None).0, 10.0);
        assert_eq!(cell_size(9.6, 13.4, -3.0, 0.0, 1.0, None).1, 16.0);
        assert_eq!(cell_size(9.6, 13.5, -3.0, 0.0, 1.0, None).1, 17.0);
    }

    #[test]
    fn cell_size_clamps_a_collapsing_modifier_to_one_pixel() {
        assert_eq!(
            cell_size(
                9.6,
                13.0,
                -3.0,
                0.0,
                1.0,
                Some(MetricModifier::Absolute(-40))
            ),
            (10.0, 1.0)
        );
    }

    #[test]
    fn cell_height_includes_the_line_gap() {
        // 13 + 3 + 2.6 = 18.6 -> 19; gpui's Linux descent is negative.
        assert_eq!(cell_size(9.6, 13.0, -3.0, 2.6, 1.0, None), (10.0, 19.0));
    }

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn pick_family_takes_the_first_installed_configured_family() {
        let available = names(&["Noto Sans", "Fira Code", DEFAULT_FONT_FAMILY]);
        assert_eq!(
            pick_family(&names(&["Missing", "Fira Code", "Noto Sans"]), &available),
            "Fira Code"
        );
    }

    #[test]
    fn pick_family_falls_back_to_the_bundled_font() {
        let available = names(&[DEFAULT_FONT_FAMILY]);
        assert_eq!(
            pick_family(&names(&["Missing"]), &available),
            DEFAULT_FONT_FAMILY
        );
        assert_eq!(pick_family(&[], &available), DEFAULT_FONT_FAMILY);
    }
}
