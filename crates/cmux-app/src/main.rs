use std::rc::Rc;

use cmux_app::style::{TermStyle, installed_families, load_bundled_fonts};
use cmux_app::terminal_view::TerminalView;
use cmux_config::ghostty::{Env, load};
use gpui::{
    App, AppContext, Application, Bounds, Focusable, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, px, size,
};

fn main() {
    Application::new().run(|cx: &mut App| {
        let dark = matches!(
            cx.window_appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let loaded = Env::from_system(dark)
            .map(|env| load(&env))
            .unwrap_or_default();
        // shortcut: warnings go to stderr; an in-window warning line comes with the M2 config work.
        for warning in &loaded.warnings {
            eprintln!("cmux-linux: config: {warning}");
        }
        // Order matters: gpui caches failed font lookups, so the bundled font is registered and
        // the family picked before anything resolves a font_id.
        load_bundled_fonts(cx);
        let installed = installed_families(cx);
        let style = Rc::new(TermStyle::new(&loaded.config, &installed));
        // Translucent only when asked. KWin blur needs ext_background_effect_v1, which gpui 0.2.2
        // lacks (M0 journal); Blurred is kept so it works once gpui supports it.
        let window_background = if loaded.config.background_opacity < 1.0 {
            WindowBackgroundAppearance::Blurred
        } else {
            WindowBackgroundAppearance::Opaque
        };
        let bounds = Bounds::centered(None, size(px(1100.), px(700.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background,
                app_id: Some("cmux-linux".into()),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| TerminalView::new(style, window, cx));
                let focus = view.read(cx).focus_handle(cx);
                window.focus(&focus);
                view
            },
        )
        .expect("open window");
        cx.activate(true);
    });
}
