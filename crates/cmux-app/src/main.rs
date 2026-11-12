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
        cmux_app::ui::set_scale(
            env.as_ref()
                .map_or(1.0, |e| cmux_config::ui::load(&e.cmux_config_file())),
        );
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
        // Linux is always Transparent: the rounded window corners (csd.rs) must show through.
        let window_background =
            if cfg!(target_os = "macos") && loaded.config.background_opacity >= 1.0 {
                WindowBackgroundAppearance::Opaque
            } else if cfg!(target_os = "macos") && loaded.config.background_blur {
                WindowBackgroundAppearance::Blurred
            } else {
                WindowBackgroundAppearance::Transparent
            };
        let saved = cmux_app::window_view::session::load();
        let (w, h, maximized) = saved.as_ref().map_or((1100., 700., false), |s| {
            (s.width.max(300.), s.height.max(400.), s.maximized)
        });
        let bounds = Bounds::centered(None, size(px(w), px(h)), cx);
        let window_bounds = if maximized {
            WindowBounds::Maximized(bounds)
        } else {
            WindowBounds::Windowed(bounds)
        };
        cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                window_background,
                app_id: Some("cmux-linux".into()),
                // We draw the window buttons ourselves (csd.rs); macOS keeps its title bar.
                window_decorations: cfg!(target_os = "linux")
                    .then_some(gpui::WindowDecorations::Client),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| WindowView::new(style, saved, window, cx)),
        )
        .expect("open window");
        cx.activate(true);
    });
}
