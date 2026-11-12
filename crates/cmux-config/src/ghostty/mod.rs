//! The subset of Ghostty's config that cmux-linux uses.
//!
//! Syntax and semantics follow Ghostty's `src/cli/args.zig` (line format) and
//! `src/config/Config.zig` (keys); see docs/milestones/m1-terminal/research.md §7.
//! Unknown keys are ignored: Ghostty has hundreds and most are not ours.

pub mod edit;
mod load;

pub use load::{
    Env, Loaded, ThemePreview, load, origin, theme_dirs, theme_pair, theme_setting, themes,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A colour that may instead follow the cell under it (`cell-foreground` / `cell-background`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynColor {
    Rgb(Rgb),
    CellForeground,
    CellBackground,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    #[default]
    Block,
    Bar,
    Underline,
    BlockHollow,
}

/// `adjust-cell-height`: whole device pixels (`2`, `-1`) or a percentage (`20%`).
/// Percentages are stored as a factor, as Ghostty does (font/Metrics.zig `Modifier.parse`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricModifier {
    Absolute(i32),
    Percent(f32),
}

impl MetricModifier {
    pub fn parse(s: &str) -> Result<Self, String> {
        if let Some(p) = s.strip_suffix('%') {
            let p: f32 = p
                .trim()
                .parse()
                .map_err(|_| format!("not a percentage: {s}"))?;
            if !p.is_finite() {
                return Err(format!("not a finite percentage: {s}"));
            }
            return Ok(Self::Percent((1.0 + p / 100.0).max(0.0)));
        }
        s.parse()
            .map(Self::Absolute)
            .map_err(|_| format!("not a whole number or percentage: {s}"))
    }

    /// Applies the modifier to a height in device pixels, rounding like Ghostty.
    /// Clamped to one pixel so a large negative modifier cannot zero the cell height.
    pub fn apply(self, px: f32) -> f32 {
        let h = match self {
            Self::Absolute(d) => px + d as f32,
            Self::Percent(f) => (px * f).round(),
        };
        h.max(1.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// First entry is the primary font, the rest are fallbacks. Empty = Ghostty's default.
    pub font_family: Vec<String>,
    /// Points. Ghostty's Linux default is 12.
    pub font_size: f32,
    pub adjust_cell_height: Option<MetricModifier>,
    pub background: Rgb,
    pub foreground: Rgb,
    pub palette: [Option<Rgb>; 256],
    pub background_opacity: f32,
    pub background_opacity_cells: bool,
    /// `background-blur`: any radius above zero (or `true`) turns the blur on.
    pub background_blur: bool,
    pub cursor_style: CursorStyle,
    /// `None` lets the application (DEC mode 12) decide.
    pub cursor_style_blink: Option<bool>,
    pub cursor_color: Option<DynColor>,
    pub cursor_text: Option<DynColor>,
    pub selection_background: Option<DynColor>,
    pub selection_foreground: Option<DynColor>,
    pub faint_opacity: f32,
    /// Clamped to 0.15..=1 as Ghostty does; the dim overlay alpha is `1 - this`.
    pub unfocused_split_opacity: f32,
    /// `None` dims with the background colour.
    pub unfocused_split_fill: Option<Rgb>,
    /// `None` uses the chrome separator colour (cmux).
    pub split_divider_color: Option<Rgb>,
    pub split_inherit_working_directory: bool,
    pub tab_inherit_working_directory: bool,
    /// Confirm before pasting text with a newline (or a bracketed-paste end) unless the
    /// program enabled bracketed paste.
    pub clipboard_paste_protection: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_family: Vec::new(),
            font_size: 12.0,
            adjust_cell_height: None,
            background: Rgb {
                r: 0x28,
                g: 0x2c,
                b: 0x34,
            },
            foreground: Rgb {
                r: 0xff,
                g: 0xff,
                b: 0xff,
            },
            palette: [None; 256],
            background_opacity: 1.0,
            background_opacity_cells: false,
            background_blur: false,
            cursor_style: CursorStyle::Block,
            cursor_style_blink: None,
            cursor_color: None,
            cursor_text: None,
            selection_background: None,
            selection_foreground: None,
            faint_opacity: 0.5,
            unfocused_split_opacity: 0.7,
            unfocused_split_fill: None,
            split_divider_color: None,
            split_inherit_working_directory: true,
            tab_inherit_working_directory: true,
            clipboard_paste_protection: true,
        }
    }
}

impl Config {
    /// Applies one `key = value` entry. `theme` and `config-file` are handled by load.
    pub fn apply(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "font-family" if value.is_empty() => self.font_family.clear(),
            "font-family" => self.font_family.push(value.to_string()),
            "font-size" => {
                self.font_size = number(value).and_then(|v| {
                    if v > 0.0 {
                        Ok(v)
                    } else {
                        Err(format!("must be positive: {value}"))
                    }
                })?
            }
            "adjust-cell-height" if value.is_empty() => self.adjust_cell_height = None,
            "adjust-cell-height" => self.adjust_cell_height = Some(MetricModifier::parse(value)?),
            "background" => self.background = parse_rgb(value)?,
            "foreground" => self.foreground = parse_rgb(value)?,
            "palette" => {
                let (index, color) = parse_palette(value)?;
                self.palette[index as usize] = Some(color);
            }
            "background-opacity" => self.background_opacity = number(value)?.clamp(0.0, 1.0),
            "background-opacity-cells" => self.background_opacity_cells = boolean(value)?,
            "background-blur" => {
                self.background_blur = boolean(value)
                    .or_else(|_| number(value).map(|v| v > 0.0))
                    .map_err(|_| format!("expected true, false or a radius: {value}"))?
            }
            "cursor-style" => {
                self.cursor_style = match value {
                    "block" => CursorStyle::Block,
                    "bar" => CursorStyle::Bar,
                    "underline" => CursorStyle::Underline,
                    "block_hollow" => CursorStyle::BlockHollow,
                    _ => return Err(format!("unknown cursor style: {value}")),
                }
            }
            "cursor-style-blink" if value.is_empty() => self.cursor_style_blink = None,
            "cursor-style-blink" => self.cursor_style_blink = Some(boolean(value)?),
            "cursor-color" => self.cursor_color = dyn_color(value)?,
            "cursor-text" => self.cursor_text = dyn_color(value)?,
            "selection-background" => self.selection_background = dyn_color(value)?,
            "selection-foreground" => self.selection_foreground = dyn_color(value)?,
            "faint-opacity" => self.faint_opacity = number(value)?.clamp(0.0, 1.0),
            "unfocused-split-opacity" => {
                self.unfocused_split_opacity = number(value)?.clamp(0.15, 1.0)
            }
            "unfocused-split-fill" => self.unfocused_split_fill = optional_rgb(value)?,
            "split-divider-color" => self.split_divider_color = optional_rgb(value)?,
            "split-inherit-working-directory" => {
                self.split_inherit_working_directory = boolean(value)?
            }
            "tab-inherit-working-directory" => self.tab_inherit_working_directory = boolean(value)?,
            "clipboard-paste-protection" => self.clipboard_paste_protection = boolean(value)?,
            _ => {}
        }
        Ok(())
    }
}

/// `key = value` entries of one file as (1-based line, key, value). Ghostty has no inline
/// comments; only lines starting with `#` are comments. A bare `key` has an empty value.
pub fn entries(text: &str) -> impl Iterator<Item = (usize, &str, &str)> {
    // A UTF-8 BOM is not whitespace, so trim() would leave it on the first key.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.lines().enumerate().filter_map(|(i, line)| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (key, value) = line.split_once('=').unwrap_or((line, ""));
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value);
        Some((i + 1, key.trim(), value))
    })
}

/// `#rrggbb`, `rrggbb`, `#rgb` or `rgb`. Every theme Ghostty ships uses `#rrggbb`.
// shortcut: X11 colour names (`red`) are rejected with a warning; add a name table if users hit it.
pub fn parse_rgb(s: &str) -> Result<Rgb, String> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    let digit = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16);
    let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16);
    // from_str_radix accepts a leading `+`, so check the characters first.
    let all_hex = hex.bytes().all(|b| b.is_ascii_hexdigit());
    let rgb = match hex.len() {
        6 if all_hex => (pair(0), pair(2), pair(4)),
        3 if all_hex => {
            let d = |i| digit(i).map(|v| v * 17);
            (d(0), d(1), d(2))
        }
        _ => return Err(format!("not a hex colour (#rrggbb): {s}")),
    };
    match rgb {
        (Ok(r), Ok(g), Ok(b)) => Ok(Rgb { r, g, b }),
        _ => Err(format!("not a hex colour (#rrggbb): {s}")),
    }
}

/// `N=COLOR` with N decimal, `0x`, `0o` or `0b`.
fn parse_palette(s: &str) -> Result<(u8, Rgb), String> {
    let (index, color) = s
        .split_once('=')
        .ok_or_else(|| format!("expected N=COLOR: {s}"))?;
    let index = index.trim();
    let parsed = if let Some(h) = index.strip_prefix("0x") {
        digits(h, 16)
    } else if let Some(o) = index.strip_prefix("0o") {
        digits(o, 8)
    } else if let Some(b) = index.strip_prefix("0b") {
        digits(b, 2)
    } else {
        digits(index, 10)
    };
    let index = parsed.ok_or_else(|| format!("palette index must be 0-255: {index}"))?;
    Ok((index, parse_rgb(color.trim())?))
}

/// `from_str_radix` accepts a leading `+`, so every character is checked first.
fn digits(s: &str, radix: u32) -> Option<u8> {
    if s.chars().all(|c| c.is_digit(radix)) {
        u8::from_str_radix(s, radix).ok()
    } else {
        None
    }
}

fn dyn_color(s: &str) -> Result<Option<DynColor>, String> {
    Ok(Some(match s {
        "" => return Ok(None),
        "cell-foreground" => DynColor::CellForeground,
        "cell-background" => DynColor::CellBackground,
        _ => DynColor::Rgb(parse_rgb(s)?),
    }))
}

/// An optional colour; an empty value resets it to unset.
fn optional_rgb(s: &str) -> Result<Option<Rgb>, String> {
    if s.is_empty() {
        Ok(None)
    } else {
        parse_rgb(s).map(Some)
    }
}

/// `f32::from_str` also accepts `nan` and `inf`, which no config value means.
fn number(s: &str) -> Result<f32, String> {
    s.parse()
        .ok()
        .filter(|v: &f32| v.is_finite())
        .ok_or_else(|| format!("not a finite number: {s}"))
}

/// Spellings from Ghostty's `parseBool` (`vendor/ghostty/src/cli/args.zig`). A bare key means true.
fn boolean(s: &str) -> Result<bool, String> {
    match s {
        "" | "1" | "t" | "T" | "true" => Ok(true),
        "0" | "f" | "F" | "false" => Ok(false),
        _ => Err(format!("expected true or false: {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> (Config, Vec<String>) {
        let mut cfg = Config::default();
        let mut warnings = Vec::new();
        for (line, key, value) in entries(text) {
            if let Err(e) = cfg.apply(key, value) {
                warnings.push(format!("{line}: {key}: {e}"));
            }
        }
        (cfg, warnings)
    }

    #[test]
    fn entries_skip_comments_trim_and_unquote() {
        let text = "# comment\n\n  font-family = \"JetBrains Mono\"  \r\nfont-size=13\nbare-key\n";
        let got: Vec<_> = entries(text).collect();
        assert_eq!(
            got,
            vec![
                (3, "font-family", "JetBrains Mono"),
                (4, "font-size", "13"),
                (5, "bare-key", "")
            ]
        );
    }

    #[test]
    fn value_keeps_text_after_first_equals() {
        let got: Vec<_> = entries("palette = 1=#ff0000").collect();
        assert_eq!(got, vec![(1, "palette", "1=#ff0000")]);
    }

    #[test]
    fn hex_colours() {
        assert_eq!(
            parse_rgb("#282c34"),
            Ok(Rgb {
                r: 0x28,
                g: 0x2c,
                b: 0x34
            })
        );
        assert_eq!(
            parse_rgb("FFFFFF"),
            Ok(Rgb {
                r: 255,
                g: 255,
                b: 255
            })
        );
        assert_eq!(
            parse_rgb("#f80"),
            Ok(Rgb {
                r: 0xff,
                g: 0x88,
                b: 0x00
            })
        );
        assert!(parse_rgb("red").is_err());
        assert!(parse_rgb("#12345g").is_err());
        assert!(parse_rgb("#ééé").is_err());
    }

    #[test]
    fn spec_keys_apply() {
        let (cfg, warnings) = parse(
            "font-family = Fira Code\nfont-family = Noto Color Emoji\nfont-size = 14.5\n\
             adjust-cell-height = 20%\nbackground = #000000\nforeground = #eeeeee\n\
             palette = 0=#111111\npalette = 0x0f=#222222\nbackground-opacity = 0.85\n\
             background-opacity-cells\ncursor-style = bar\ncursor-style-blink = false\n\
             cursor-color = #ff0000\ncursor-text = cell-background\n\
             selection-background = #333333\nselection-foreground = cell-foreground\n\
             faint-opacity = 0.4\n",
        );
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(cfg.font_family, vec!["Fira Code", "Noto Color Emoji"]);
        assert_eq!(cfg.font_size, 14.5);
        assert_eq!(cfg.adjust_cell_height, Some(MetricModifier::Percent(1.2)));
        assert_eq!(cfg.background, Rgb { r: 0, g: 0, b: 0 });
        assert_eq!(
            cfg.foreground,
            Rgb {
                r: 0xee,
                g: 0xee,
                b: 0xee
            }
        );
        assert_eq!(
            cfg.palette[0],
            Some(Rgb {
                r: 0x11,
                g: 0x11,
                b: 0x11
            })
        );
        assert_eq!(
            cfg.palette[15],
            Some(Rgb {
                r: 0x22,
                g: 0x22,
                b: 0x22
            })
        );
        assert_eq!(cfg.background_opacity, 0.85);
        assert!(cfg.background_opacity_cells);
        assert_eq!(cfg.cursor_style, CursorStyle::Bar);
        assert_eq!(cfg.cursor_style_blink, Some(false));
        assert_eq!(
            cfg.cursor_color,
            Some(DynColor::Rgb(Rgb { r: 255, g: 0, b: 0 }))
        );
        assert_eq!(cfg.cursor_text, Some(DynColor::CellBackground));
        assert_eq!(
            cfg.selection_background,
            Some(DynColor::Rgb(Rgb {
                r: 0x33,
                g: 0x33,
                b: 0x33
            }))
        );
        assert_eq!(cfg.selection_foreground, Some(DynColor::CellForeground));
        assert_eq!(cfg.faint_opacity, 0.4);
    }

    #[test]
    fn split_keys_apply() {
        let (cfg, warnings) = parse(
            "unfocused-split-opacity = 0.05\nunfocused-split-fill = #102030\n\
             split-divider-color = 445566\nsplit-inherit-working-directory = false\n\
             tab-inherit-working-directory = false\nclipboard-paste-protection = false\n",
        );
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(cfg.unfocused_split_opacity, 0.15, "clamped like Ghostty");
        assert_eq!(
            cfg.unfocused_split_fill,
            Some(Rgb {
                r: 0x10,
                g: 0x20,
                b: 0x30
            })
        );
        assert_eq!(
            cfg.split_divider_color,
            Some(Rgb {
                r: 0x44,
                g: 0x55,
                b: 0x66
            })
        );
        assert!(!cfg.split_inherit_working_directory);
        assert!(!cfg.tab_inherit_working_directory);
        assert!(!cfg.clipboard_paste_protection);
        let (cfg, _) = parse("split-divider-color = #445566\nsplit-divider-color =\n");
        assert_eq!(cfg.split_divider_color, None, "empty resets");
    }

    #[test]
    fn split_key_defaults_match_ghostty() {
        let cfg = Config::default();
        assert_eq!(cfg.unfocused_split_opacity, 0.7);
        assert_eq!(cfg.unfocused_split_fill, None);
        assert_eq!(cfg.split_divider_color, None);
        assert!(cfg.split_inherit_working_directory);
        assert!(cfg.tab_inherit_working_directory);
        assert!(cfg.clipboard_paste_protection);
    }

    #[test]
    fn empty_font_family_resets_the_list() {
        let (cfg, _) = parse("font-family = A\nfont-family = \"\"\nfont-family = B\n");
        assert_eq!(cfg.font_family, vec!["B"]);
    }

    #[test]
    fn metric_modifiers() {
        assert_eq!(MetricModifier::parse("2"), Ok(MetricModifier::Absolute(2)));
        assert_eq!(
            MetricModifier::parse("-20%"),
            Ok(MetricModifier::Percent(0.8))
        );
        assert_eq!(
            MetricModifier::parse("-150%"),
            Ok(MetricModifier::Percent(0.0))
        );
        assert!(MetricModifier::parse("2.5").is_err());
        assert_eq!(MetricModifier::Absolute(2).apply(17.0), 19.0);
        assert_eq!(MetricModifier::Percent(1.2).apply(17.0), 20.0);
    }

    #[test]
    fn malformed_values_warn_and_keep_defaults() {
        let (cfg, warnings) = parse(
            "font-size = big\nbackground = red\npalette = 300=#000000\ncursor-style = triangle\n\
             background-opacity = 2\nunknown-key = whatever\n",
        );
        assert_eq!(warnings.len(), 4, "{warnings:?}");
        assert!(warnings[0].starts_with("1: font-size:"), "{warnings:?}");
        let defaults = Config::default();
        assert_eq!(cfg.font_size, defaults.font_size);
        assert_eq!(cfg.background, defaults.background);
        assert_eq!(cfg.palette, defaults.palette);
        assert_eq!(cfg.cursor_style, defaults.cursor_style);
        assert_eq!(cfg.background_opacity, 1.0, "clamped, not rejected");
    }

    #[test]
    fn non_finite_numbers_warn_and_keep_defaults() {
        let (cfg, warnings) =
            parse("background-opacity = nan\nfont-size = inf\nfaint-opacity = infinity\n");
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        let defaults = Config::default();
        assert_eq!(cfg.background_opacity, defaults.background_opacity);
        assert_eq!(cfg.font_size, defaults.font_size);
        assert_eq!(cfg.faint_opacity, defaults.faint_opacity);
    }

    #[test]
    fn plus_sign_is_not_hex() {
        assert!(parse_rgb("#+f0000").is_err());
        let (cfg, warnings) = parse("palette = 0x+f=#000000\n");
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(cfg.palette[15], None);
    }

    #[test]
    fn leading_bom_is_ignored() {
        let (cfg, warnings) = parse("\u{feff}font-family = X\n");
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(cfg.font_family, vec!["X"]);
    }

    #[test]
    fn ghostty_boolean_spellings() {
        for (text, want) in [
            ("1", true),
            ("t", true),
            ("T", true),
            ("true", true),
            ("0", false),
            ("f", false),
            ("F", false),
            ("false", false),
        ] {
            let (cfg, warnings) = parse(&format!("background-opacity-cells = {text}\n"));
            assert_eq!(warnings, Vec::<String>::new(), "{text}");
            assert_eq!(cfg.background_opacity_cells, want, "{text}");
        }
    }

    #[test]
    fn metric_modifier_never_drops_below_one_pixel() {
        assert_eq!(MetricModifier::Absolute(-1000).apply(17.0), 1.0);
        assert_eq!(MetricModifier::Percent(-100.0).apply(17.0), 1.0);
        assert!(MetricModifier::parse("inf%").is_err());
        assert!(MetricModifier::parse("nan%").is_err());
    }
    #[test]
    fn background_blur_accepts_bool_and_radius() {
        for (v, on) in [("true", true), ("false", false), ("20", true), ("0", false)] {
            let mut cfg = Config::default();
            cfg.apply("background-blur", v).unwrap();
            assert_eq!(cfg.background_blur, on, "{v}");
        }
        assert!(Config::default().apply("background-blur", "maybe").is_err());
    }
}
