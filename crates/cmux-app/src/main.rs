use std::rc::Rc;

use cmux_app::actions;
use cmux_app::assets::Assets;
use cmux_app::style::{TermStyle, installed_families, load_bundled_fonts};
use cmux_app::window_view::WindowView;
use cmux_config::ghostty::{Env, load};
use cmux_config::keybindings;
use gpui::{
    App, AppContext, Application, Bounds, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, px, size,
};

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        let dark = matches!(
            cx.window_appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let env = Env::from_system(dark);
        let loaded = env.as_ref().map(load).unwrap_or_default();
        let keys = match &env {
            Some(env) => keybindings::load(&env.cmux_config_file()),
            None => keybindings::keybindings(None, "config.toml"),
        };
        let bind_warnings = actions::bind(cx, &keys);
        // shortcut: warnings go to stderr; an in-window warning line is not planned before M6.
        for warning in loaded
            .warnings
            .iter()
            .chain(&keys.warnings)
            .chain(&bind_warnings)
        {
            eprintln!("cmux-linux: config: {warning}");
        }
        // Order matters: gpui caches failed font lookups, so the bundled font is registered and
        // the family picked before anything resolves a font_id.
        load_bundled_fonts(cx);
        let installed = installed_families(cx);
        let style = Rc::new(TermStyle::new(&loaded.config, &installed));
        // Translucent only when asked. Blurred on macOS (NSVisualEffectView); on Linux KWin blur
        // comes from our own shim (blur.rs), so Transparent there.
        let window_background = if loaded.config.background_opacity >= 1.0 {
            WindowBackgroundAppearance::Opaque
        } else if cfg!(target_os = "macos") && loaded.config.background_blur {
            WindowBackgroundAppearance::Blurred
        } else {
            WindowBackgroundAppearance::Transparent
        };
        let bounds = Bounds::centered(None, size(px(1100.), px(700.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background,
                app_id: Some("cmux-linux".into()),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| WindowView::new(style, window, cx)),
        )
        .expect("open window");
        cx.activate(true);
    });
}
