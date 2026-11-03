use gpui::{
    App, Application, Bounds, Context, Window, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, div, prelude::*, px, rgba, size,
};

struct Root;

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Translucent fill so KWin's blur behind the window is visible.
        div().size_full().bg(rgba(0x1e1e2eb3))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.), px(700.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background: WindowBackgroundAppearance::Blurred,
                app_id: Some("cmux-linux".into()),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Root),
        )
        .expect("open window");
        cx.activate(true);
    });
}
