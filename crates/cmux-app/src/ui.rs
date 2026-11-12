//! The app chrome's size multiplier (`ui_scale` in config.toml). Chrome files import `px` from
//! here; measured values (widths from drags, window corners, mouse positions) use `gpui::px`.

use std::sync::atomic::{AtomicU32, Ordering};

use gpui::Pixels;

static SCALE: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0f32

/// shortcut: set once at startup, so a change needs a restart; live reload comes with the settings page (M12).
pub fn set_scale(v: f32) {
    SCALE.store(v.to_bits(), Ordering::Relaxed);
}

/// A design-time length, scaled.
pub fn s(v: f32) -> f32 {
    v * f32::from_bits(SCALE.load(Ordering::Relaxed))
}

pub fn px(v: f32) -> Pixels {
    gpui::px(s(v))
}
