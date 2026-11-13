//! The strip over the content column (cmux `ContentView.customTitlebar`, plan K6): folder icon
//! and the selected workspace's title, drag to move, double-click to maximise.

use gpui::{
    Context, FontWeight, InteractiveElement, IntoElement, ParentElement, SharedString, Styled,
    Window, div, svg,
};

use super::WindowView;
use crate::chrome;
use crate::mac::{Theme, metric};
use crate::ui::{px, s};

/// `title · host` for SSH workspaces (cmux `resolvedWorkspaceWindowTitle`).
pub(super) fn titlebar_text(title: &str, host: Option<&str>) -> String {
    match host {
        Some(h) if title.is_empty() => h.to_string(),
        Some(h) => format!("{title} · {h}"),
        None => title.to_string(),
    }
}

impl WindowView {
    pub(super) fn render_titlebar(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let t = Theme::new(chrome::rgba(self.style.cfg.background, 1.0));
        let id = self.workspaces.selected();
        let text = titlebar_text(&self.title_of(id), self.remote_host(id, cx).as_deref());
        // Left: the reopen button floats here while the sidebar is hidden. Right: the window
        // buttons float here unless the right sidebar is what sits in the corner.
        let lead = match self.sidebar_px(window) {
            Some(_) => s(12.),
            None => s(crate::csd::REOPEN_LEAD),
        };
        let trail = match self.right_sidebar_open {
            true => s(12.),
            false => crate::csd::controls_width(window) + s(8.),
        };
        let strip = div()
            .id("titlebar")
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap(px(6.))
            .h(px(metric::TITLEBAR_H))
            .pl(gpui::px(lead))
            .pr(gpui::px(trail))
            .border_b_1()
            .border_color(t.separator)
            .child(
                svg()
                    .path("icons/folder.svg")
                    .flex_none()
                    .size(px(14.))
                    .text_color(t.secondary),
            )
            .child(
                div()
                    .min_w(gpui::px(0.))
                    .text_size(px(metric::TITLE_TEXT))
                    .font_weight(FontWeight::BOLD)
                    .text_color(t.text)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(SharedString::from(text)),
            );
        crate::csd::window_drag(strip, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_workspaces_show_their_host() {
        assert_eq!(titlebar_text("api", None), "api");
        assert_eq!(titlebar_text("api", Some("box")), "api · box");
        assert_eq!(titlebar_text("", Some("box")), "box");
    }
}
