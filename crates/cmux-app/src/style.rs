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
    pub fn new(cfg: &Config) -> Self {
        let mut families = cfg.font_family.clone();
        let primary = if families.is_empty() {
            DEFAULT_FONT_FAMILY.to_string()
        } else {
            families.remove(0)
        };
        // Like Ghostty, fall back to the bundled font after the user's own fallbacks.
        families.push(DEFAULT_FONT_FAMILY.to_string());
        let font = Font {
            family: primary.into(),
            // Ligatures stay off: force_width snaps glyphs, so a ligature would shift the grid.
            features: FontFeatures::disable_ligatures(),
            fallbacks: Some(FontFallbacks::from_fonts(families)),
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

/// Cell size in logical pixels, snapped to whole device pixels the way Ghostty rounds
/// (font/Metrics.zig: `cell_width = @round(face_width)`, `cell_height = @round(ascent - descent + line_gap)`).
/// gpui hides `line_gap` (0 for JetBrains Mono) and negates `descent` on Linux, hence `abs`.
pub fn cell_size(
    advance: f32,
    ascent: f32,
    descent: f32,
    scale: f32,
    adjust: Option<MetricModifier>,
) -> (f32, f32) {
    let w = (advance * scale).round().max(1.0);
    let mut h = ((ascent + descent.abs()) * scale).round();
    if let Some(m) = adjust {
        h = m.apply(h);
    }
    (w / scale, h.max(1.0) / scale)
}

/// Registers the bundled JetBrains Mono faces with gpui.
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
        let style = TermStyle::new(&Config::default());
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
        let style = TermStyle::new(&cfg);
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
        TermStyle::new(&cfg).apply_to(&mut t);
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
        TermStyle::new(&cfg).apply_to(&mut t);
        assert_eq!(
            t.update_frame().unwrap().cursor.unwrap().shape,
            CursorShape::Bar
        );
    }

    #[test]
    fn cell_size_snaps_to_device_pixels() {
        assert_eq!(cell_size(9.6, 13.0, -3.0, 1.0, None), (10.0, 16.0));
        assert_eq!(cell_size(9.6, 13.0, 3.0, 2.0, None), (9.5, 16.0));
        assert_eq!(
            cell_size(9.6, 13.0, -3.0, 1.0, Some(MetricModifier::Absolute(2))),
            (10.0, 18.0)
        );
        assert_eq!(
            cell_size(9.6, 13.0, -3.0, 1.0, Some(MetricModifier::Percent(1.25))),
            (10.0, 20.0)
        );
    }
}
