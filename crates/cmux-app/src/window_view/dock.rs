//! The right sidebar's frame and its Dock (M8 plan D12-D15, research-dock.md): a mode bar
//! with Feed and Dock, a draggable width, and the Dock itself, which is one `WorkspaceView`
//! owned by the window, so it has the main area's panes, tabs, splits and shortcuts and
//! outlives workspace switches.

use std::path::{Path, PathBuf};

use crate::ui::{px, s};
use cmux_core::{Direction, WorkspaceId};
use gpui::{
    AppContext, ClickEvent, Context, CursorStyle, DragMoveEvent, EmptyView, Focusable, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Rgba, StatefulInteractiveElement, Styled,
    Window, div, prelude::*, svg,
};
use serde_json::Value;

use super::WindowView;
use super::feed::{BADGE_ALPHA, FeedColors, badge_label, with_alpha};
use crate::workspace::{WorkspaceEvent, WorkspaceView};

/// cmux's floor for the right sidebar (rd §2); the main area keeps `MAIN_MIN` beside it.
pub const RIGHT_SIDEBAR_MIN: f32 = 295.0;
const RIGHT_SIDEBAR_MAX: f32 = 1200.0;
const MAIN_MIN: f32 = 360.0;
// Mode bar (rd §1.5).
const BAR_H: f32 = 28.0;
const BAR_PAD_L: f32 = 4.0;
const BAR_PAD_R: f32 = 6.0;
const BAR_GAP: f32 = 4.0;
const TAB_H: f32 = 20.0;
const TAB_PAD_X: f32 = 8.0;
const TAB_RADIUS: f32 = 6.0;
const TAB_PX: f32 = 11.0;
const TAB_ICON: f32 = 11.0;
const TAB_ALPHA: f32 = 0.86;
const TAB_HOVER_ALPHA: f32 = 0.96;
const TAB_SELECTED_FILL: f32 = 0.10;
const TAB_HOVER_FILL: f32 = 0.05;
/// The resize band reaches this far into the main area and into the sidebar.
const RESIZE_OUT: f32 = 4.0;
const RESIZE_IN: f32 = 6.0;
// Empty Dock pane (rd §4.4).
const EMPTY_ICON: f32 = 30.0;
const EMPTY_ICON_ALPHA: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum RightMode {
    #[default]
    Feed,
    Dock,
}

/// The right sidebar's edge being dragged.
#[derive(Clone, Copy)]
pub(super) struct RightSidebarDrag;

/// [MIN, MAX], leaving the main area `MAIN_MIN`, but never below MIN (rd §2.2).
pub(crate) fn clamp_right_width(w: f32, window_w: f32) -> f32 {
    w.clamp(s(RIGHT_SIDEBAR_MIN), s(RIGHT_SIDEBAR_MAX))
        .min(window_w - s(MAIN_MIN))
        .max(s(RIGHT_SIDEBAR_MIN))
}

/// One terminal `dock.json` asks for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DockControl {
    pub command: String,
    /// Absolute: a relative `cwd` is joined onto the config's base directory.
    pub cwd: String,
}

/// `{"controls": [{"id", "command", "cwd"?, "type"?}]}` (rd §7.1) with cmux's error texts.
/// Browser controls are skipped (terminals only); `title`, `height` and `env` are read past.
// shortcut: no height ratios, env or tab titles; add them when someone's dock.json needs them.
pub(crate) fn parse_dock_config(json: &str, base: &Path) -> Result<Vec<DockControl>, String> {
    let generic = || "Could not load the Dock config. Check dock.json and try again.".to_string();
    let root: Value = serde_json::from_str(json).map_err(|_| generic())?;
    let controls = root
        .get("controls")
        .and_then(Value::as_array)
        .ok_or_else(generic)?;
    let mut ids: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for c in controls {
        let text = |k: &str| c.get(k).and_then(Value::as_str).map(str::trim);
        let id = text("id").ok_or_else(generic)?;
        if id.is_empty() {
            return Err("Dock control id must not be blank.".into());
        }
        if ids.contains(&id) {
            return Err("Dock control ids must be unique.".into());
        }
        ids.push(id);
        match text("type").unwrap_or("terminal").to_lowercase().as_str() {
            "terminal" => {}
            "browser" | "chief" => continue,
            _ => return Err("Dock control type must be terminal, browser, or Chief.".into()),
        }
        let command = text("command").unwrap_or("");
        if command.is_empty() {
            return Err("Dock control command must not be blank.".into());
        }
        let cwd = match text("cwd") {
            Some(d) if !d.is_empty() => base.join(d),
            _ => base.to_path_buf(),
        };
        out.push(DockControl {
            command: command.to_string(),
            cwd: cwd.to_string_lossy().into_owned(),
        });
    }
    Ok(out)
}

/// `~/.config/cmux-linux/dock.json` (`$XDG_CONFIG_HOME` first) and its base, the home directory.
pub(super) fn dock_config_path() -> Option<(PathBuf, PathBuf)> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    Some((config.join("cmux-linux/dock.json"), home))
}

impl WindowView {
    /// The drawn right sidebar width; 0 when closed.
    pub(super) fn right_sidebar_px(&self, window: &Window) -> f32 {
        if self.right_sidebar_open {
            clamp_right_width(self.right_width, f32::from(window.viewport_size().width))
        } else {
            0.
        }
    }

    fn right_has_focus(&self, window: &Window, cx: &Context<Self>) -> bool {
        self.feed_focus.contains_focused(window, cx)
            || self
                .dock
                .as_ref()
                .is_some_and(|d| d.focus_handle(cx).contains_focused(window, cx))
    }

    /// Only a Dock on screen may hold keyboard focus.
    fn sync_dock_active(&self, cx: &mut Context<Self>) {
        let shown = self.right_sidebar_open && self.right_mode == RightMode::Dock;
        if let Some(d) = &self.dock {
            d.update(cx, |d, _| d.set_active(shown));
        }
    }

    /// `toggleRightSidebar`: closing it hands focus back to the terminal. Dock terminals keep
    /// running while hidden (rd §5.8).
    pub(super) fn toggle_right_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let had_focus = self.right_has_focus(window, cx);
        self.right_sidebar_open = !self.right_sidebar_open;
        self.sync_dock_active(cx);
        if self.right_sidebar_open && self.right_mode == RightMode::Dock {
            self.seed_dock(window, cx);
        }
        if !self.right_sidebar_open && had_focus {
            self.focus_terminal(window, cx);
        }
        cx.notify();
    }

    /// Shows the sidebar in `mode` (`None`: the current one) and focuses it: the Feed's first
    /// card, or the Dock's active terminal. Never toggles off.
    pub(super) fn show_right(
        &mut self,
        mode: Option<RightMode>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.right_sidebar_open = true;
        self.right_mode = mode.unwrap_or(self.right_mode);
        self.sync_dock_active(cx);
        match self.right_mode {
            RightMode::Feed => {
                self.feed_selected = self.feed.items().next().map(|i| i.id);
                self.feed_scroll.scroll_to_item(0);
                window.focus(&self.feed_focus);
            }
            RightMode::Dock => {
                self.seed_dock(window, cx);
                match self.dock.clone() {
                    Some(d) => d.update(cx, |d, cx| d.focus_active(window, cx)),
                    // Nothing to focus in an empty Dock; keys stay with the terminal.
                    None => self.focus_terminal(window, cx),
                }
            }
        }
        cx.notify();
    }

    pub(super) fn on_right_sidebar_drag(
        &mut self,
        ev: &DragMoveEvent<RightSidebarDrag>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let window_w = f32::from(window.viewport_size().width);
        let w = clamp_right_width(window_w - f32::from(ev.event.position.x), window_w);
        if w != self.right_width {
            self.right_width = w;
            cx.notify();
        }
    }

    /// Straddles the right sidebar's leading edge, above the content (as the left one).
    pub(super) fn render_right_resize_band(&self, width: f32) -> impl IntoElement + use<> {
        div()
            .id("right-sidebar-resize")
            .absolute()
            .top_0()
            .right(gpui::px(width - s(RESIZE_IN)))
            .w(px(RESIZE_IN + RESIZE_OUT))
            .h_full()
            .cursor(CursorStyle::ResizeLeftRight)
            .occlude()
            .on_drag(RightSidebarDrag, |_, _, _, cx| cx.new(|_| EmptyView))
    }

    /// The Dock's splits are laid out in window coordinates, like the main area's: tell it
    /// where it sits (called before each draw).
    pub(super) fn place_dock(&self, window: &Window, cx: &mut Context<Self>) {
        if let Some(d) = &self.dock {
            // 1: the sidebar's leading border.
            let left = f32::from(window.viewport_size().width) - self.right_sidebar_px(window) + 1.;
            d.update(cx, |d, _| d.set_insets(left, 0.));
        }
    }

    /// Starts the Dock's view with one terminal in `cwd` (home when `None`). False if the
    /// shell did not start.
    fn start_dock(
        &mut self,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let (style, shell) = (self.style.clone(), self.shell.clone());
        let view = cx.new(|cx| WorkspaceView::new_dock(shell, style, cwd, BAR_H, window, cx));
        self.attach_dock(view, window, cx)
    }

    /// The Dock from `session.json` (M10 plan L3).
    pub(super) fn restore_dock(
        &mut self,
        saved: &super::session::SavedLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let (style, shell) = (self.style.clone(), self.shell.clone());
        let (layout, terms) = (saved.layout(), saved.terms.clone());
        let inputs = saved.inputs(&self.vault_agents);
        let view = cx.new(|cx| {
            WorkspaceView::restore(
                WorkspaceId(0),
                true,
                shell,
                style,
                layout,
                terms,
                inputs,
                BAR_H,
                window,
                cx,
            )
        });
        self.attach_dock(view, window, cx)
    }

    fn attach_dock(
        &mut self,
        view: gpui::Entity<WorkspaceView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !view.read(cx).has_terminals() {
            return false;
        }
        let sub = cx.subscribe_in(&view, window, |this, _, ev, window, cx| {
            // shortcut: Dock notifications are dropped (rd §8 gives them rings and unread
            // state); they need a notification owner that is not a workspace.
            if matches!(ev, WorkspaceEvent::Empty) {
                // The last terminal closed: back to the empty pane (rd §4.7).
                this.dock = None;
                this._dock_sub = None;
                this.focus_terminal(window, cx);
                cx.notify();
            }
        });
        self.dock = Some(view);
        self._dock_sub = Some(sub);
        // Before any split: the Dock is far narrower than the window.
        self.place_dock(window, cx);
        self.sync_dock_active(cx);
        true
    }

    /// The empty pane's "New Terminal": a first terminal, focused.
    fn new_dock_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dock.is_none() && self.start_dock(None, window, cx) {
            cx.notify();
        }
    }

    /// The first time the Dock is shown, starts the controls of `dock.json` (plan D15): one
    /// pane below the other, each command typed into a shell in its directory.
    fn seed_dock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::replace(&mut self.dock_seeded, true) || self.dock.is_some() {
            return;
        }
        let Some((path, base)) = dock_config_path() else {
            return;
        };
        let Ok(json) = std::fs::read_to_string(&path) else {
            return; // no config: the Dock starts empty
        };
        let controls = match parse_dock_config(&json, &base) {
            Ok(c) => c,
            Err(e) => {
                // shortcut: cmux shows this in a "Dock Config Error" view; ours is one line.
                eprintln!("cmux-linux: {}: {e}", path.display());
                return;
            }
        };
        let mut controls = controls.into_iter();
        let Some(first) = controls.next() else {
            return;
        };
        if !self.start_dock(Some(first.cwd), window, cx) {
            return;
        }
        let dock = self.dock.clone().expect("just started");
        dock.update(cx, |d, cx| {
            let mut last = d.layout().active_surface();
            if let Some(t) = d.terminal(last).cloned() {
                let input = format!("{}\n", first.command).into_bytes();
                t.update(cx, |t, cx| t.send_scripted(input, cx));
            }
            for c in controls {
                let input = format!("{}\n", c.command);
                match d.split_surface(
                    last,
                    Direction::Down,
                    false,
                    Some(&input),
                    Some(c.cwd),
                    window,
                    cx,
                ) {
                    Ok(s) => last = s,
                    Err(_) => {
                        eprintln!("cmux-linux: dock.json: no room for `{}`", c.command);
                        break;
                    }
                }
            }
        });
    }

    pub(super) fn render_right_sidebar(
        &self,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement + use<> {
        let p = self.palette();
        let c = FeedColors::new(p);
        let width = self.right_sidebar_px(window);
        let radius = crate::csd::corner_radius(window);
        let tab = |id: &'static str, icon: &'static str, label: &'static str, mode: RightMode| {
            let selected = self.right_mode == mode;
            let fg = |a: f32| with_alpha(c.secondary, c.secondary.a * a);
            div()
                .id(id)
                .flex()
                .flex_row()
                .flex_none()
                .items_center()
                .gap(px(4.))
                .h(px(TAB_H))
                .px(px(TAB_PAD_X))
                .rounded(px(TAB_RADIUS))
                .text_size(px(TAB_PX))
                .text_color(if selected { c.secondary } else { fg(TAB_ALPHA) })
                .when(selected, |d| d.bg(with_alpha(c.primary, TAB_SELECTED_FILL)))
                .when(!selected, |d| {
                    d.hover(|s| {
                        s.bg(with_alpha(c.primary, TAB_HOVER_FILL))
                            .text_color(fg(TAB_HOVER_ALPHA))
                    })
                })
                .child(
                    svg()
                        .path(icon)
                        .flex_none()
                        .size(px(TAB_ICON))
                        .text_color(if selected { c.secondary } else { fg(TAB_ALPHA) }),
                )
                .child(label)
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.show_right(Some(mode), window, cx);
                }))
        };
        let badge = badge_label(self.feed.pending_count()).map(|label| {
            div()
                .px(px(5.))
                .py(px(1.))
                .rounded_full()
                .bg(with_alpha(c.orange, BADGE_ALPHA))
                .text_color(c.orange)
                .text_size(px(10.))
                .font_weight(FontWeight::BOLD)
                .font_family(self.style.font.family.clone())
                .child(label)
        });
        let bar = div()
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap(px(BAR_GAP))
            .h(px(BAR_H))
            .pl(px(BAR_PAD_L))
            .pr(px(BAR_PAD_R))
            .border_b_1()
            .border_color(p.border)
            .child(tab("right-tab-feed", "icons/rss.svg", "Feed", RightMode::Feed).children(badge))
            .child(tab(
                "right-tab-dock",
                "icons/panel-right.svg",
                "Dock",
                RightMode::Dock,
            ));
        let body = match (self.right_mode, &self.dock) {
            (RightMode::Feed, _) => self.render_feed(window, cx).into_any_element(),
            (RightMode::Dock, Some(dock)) => div()
                .flex_1()
                .min_h(px(0.))
                .child(dock.clone())
                .into_any_element(),
            (RightMode::Dock, None) => self.render_empty_dock(c, cx).into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(gpui::px(width))
            .h_full()
            .bg(p.bg)
            .rounded_tr(gpui::px(radius))
            .rounded_br(gpui::px(radius))
            .border_l_1()
            .border_color(p.border)
            .child(bar)
            .child(body)
    }

    fn render_empty_dock(&self, c: FeedColors, cx: &Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .items_center()
            .justify_center()
            .gap(px(10.))
            .child(
                svg()
                    .path("icons/panel-right.svg")
                    .size(px(EMPTY_ICON))
                    .text_color(with_alpha(c.secondary, c.secondary.a * EMPTY_ICON_ALPHA)),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.secondary)
                    .child("Empty Dock Pane"),
            )
            .child(
                div()
                    .id("dock-new-terminal")
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(5.))
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(5.))
                    .border_1()
                    .border_color(with_alpha(c.primary, 0.15))
                    .text_size(px(11.))
                    .text_color(c.primary)
                    .hover(|s| s.bg(with_alpha(c.primary, TAB_HOVER_FILL)))
                    .child(
                        svg()
                            .path("icons/square-terminal.svg")
                            .size(px(11.))
                            .text_color(Rgba { ..c.primary }),
                    )
                    .child("New Terminal")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.new_dock_terminal(window, cx);
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_width_keeps_the_main_area() {
        assert_eq!(clamp_right_width(100.0, 2000.0), 295.0);
        assert_eq!(clamp_right_width(900.0, 2000.0), 900.0);
        assert_eq!(clamp_right_width(1500.0, 3000.0), 1200.0);
        assert_eq!(clamp_right_width(900.0, 1000.0), 640.0);
        assert_eq!(
            clamp_right_width(900.0, 500.0),
            295.0,
            "never below the minimum"
        );
    }

    #[test]
    fn dock_config_resolves_cwd_and_skips_browsers() {
        let json = r#"{"controls": [
            {"id": "git", "title": "Git", "command": " lazygit ", "cwd": ".", "height": 300},
            {"id": "docs", "type": "browser", "url": "http://localhost"},
            {"id": "logs", "command": "tail -f x.log", "cwd": "/var/log", "env": {"A": "1"}},
            {"id": "sub", "command": "ls", "cwd": "src/app"}
        ]}"#;
        let c = parse_dock_config(json, Path::new("/home/u")).unwrap();
        let got: Vec<(&str, &str)> = c
            .iter()
            .map(|c| (c.command.as_str(), c.cwd.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                ("lazygit", "/home/u/."),
                ("tail -f x.log", "/var/log"),
                ("ls", "/home/u/src/app")
            ]
        );
        assert_eq!(
            parse_dock_config(r#"{"controls": []}"#, Path::new("/")),
            Ok(vec![])
        );
    }

    #[test]
    fn dock_config_errors_are_cmux_texts() {
        let e = |json: &str| parse_dock_config(json, Path::new("/")).unwrap_err();
        let generic = "Could not load the Dock config. Check dock.json and try again.";
        assert_eq!(e("not json"), generic);
        assert_eq!(e("{}"), generic);
        assert_eq!(e(r#"{"controls": [{"command": "ls"}]}"#), generic);
        assert_eq!(
            e(r#"{"controls": [{"id": " ", "command": "ls"}]}"#),
            "Dock control id must not be blank."
        );
        assert_eq!(
            e(r#"{"controls": [{"id": "a", "command": "ls"}, {"id": "a", "command": "ls"}]}"#),
            "Dock control ids must be unique."
        );
        assert_eq!(
            e(r#"{"controls": [{"id": "a", "type": "widget", "command": "ls"}]}"#),
            "Dock control type must be terminal, browser, or Chief."
        );
        assert_eq!(
            e(r#"{"controls": [{"id": "a", "command": "  "}]}"#),
            "Dock control command must not be blank."
        );
    }
}
