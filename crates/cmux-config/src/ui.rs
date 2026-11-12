//! `ui_scale` in `~/.config/cmux-linux/config.toml`: multiplies the app's own text, icons and
//! buttons (the terminal font stays Ghostty's `font-size`).

use std::path::Path;

/// 1.0 when missing, not a number, or the file doesn't parse; else clamped to 0.5..=3.0.
pub fn ui_scale(text: &str) -> f32 {
    let Ok(table) = text.parse::<toml::Table>() else {
        return 1.0;
    };
    let v = match table.get("ui_scale") {
        Some(toml::Value::Float(f)) => *f as f32,
        Some(toml::Value::Integer(i)) => *i as f32,
        _ => return 1.0,
    };
    if v.is_finite() {
        v.clamp(0.5, 3.0)
    } else {
        1.0
    }
}

/// Read once at startup.
pub fn load(path: &Path) -> f32 {
    ui_scale(&std::fs::read_to_string(path).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_clamps() {
        assert_eq!(ui_scale(""), 1.0);
        assert_eq!(ui_scale("ui_scale = \"big\""), 1.0);
        assert_eq!(ui_scale("ui_scale = 1.25"), 1.25);
        assert_eq!(ui_scale("ui_scale = 2"), 2.0);
        assert_eq!(ui_scale("ui_scale = 9.0"), 3.0);
        assert_eq!(ui_scale("ui_scale = 0.1"), 0.5);
        assert_eq!(ui_scale("ui_scale = nan"), 1.0);
        assert_eq!(load(Path::new("/nonexistent/config.toml")), 1.0);
    }
}
