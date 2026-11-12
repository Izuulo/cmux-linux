//! The Settings window (M12 plan S1–S3, S5, S8): a second gpui window with grouped sections.
//! Controls only write the user's files (`config.toml`, the Ghostty main file); the main
//! window's config poll applies them and hands the new style back here (`set_style`).

mod controls;
mod terminal;

use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use cmux_config::ghostty::{self, Config, Env, edit};
use gpui::{
    App, AppContext, Bounds, ClickEvent, Context, FocusHandle, FontWeight, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseMoveEvent, ParentElement, Pixels, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Subscription, Task,
    TitlebarOptions, Window, WindowBounds, WindowHandle, WindowOptions, div, prelude::*, svg,
};

use crate::actions::{CloseTab, OpenSettings, Quit};
use crate::style::TermStyle;
use crate::ui::px;
use crate::window_view::rename::{RenameKey, RenameOutcome, RenameState};
use controls::{Colors, Handler};

const WINDOW_W: f32 = 980.0;
const WINDOW_H: f32 = 680.0;
const SIDEBAR_W: f32 = 210.0;
/// Ghostty writes wait this long for more changes (plan S5).
const WRITE_DELAY: Duration = Duration::from_millis(250);
const UI_SCALE_MIN: f32 = 0.5;
const UI_SCALE_MAX: f32 = 2.0;
const UI_SCALE_STEP: f32 = 0.05;

/// cmux's section ids, only those this port has (plan S3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    App,
    Themes,
    Terminal,
    Automation,
    KeyboardShortcuts,
}

/// Sidebar groups in order (cmux `SettingsTaxonomyGroup`).
const GROUPS: &[(&str, &[Section])] = &[
    ("General", &[Section::App, Section::Themes]),
    ("Terminal", &[Section::Terminal]),
    ("Agents & Automation", &[Section::Automation]),
    ("Keyboard & Advanced", &[Section::KeyboardShortcuts]),
];

impl Section {
    pub fn title(self) -> &'static str {
        match self {
            Section::App => "App",
            Section::Themes => "Themes",
            Section::Terminal => "Terminal",
            Section::Automation => "Automation",
            Section::KeyboardShortcuts => "Keyboard Shortcuts",
        }
    }

    /// Lucide stand-ins for cmux's SF Symbols.
    fn icon(self) -> &'static str {
        match self {
            Section::App => "icons/settings.svg",
            Section::Themes => "icons/paintbrush.svg",
            Section::Terminal => "icons/terminal.svg",
            Section::Automation => "icons/wand-sparkles.svg",
            Section::KeyboardShortcuts => "icons/keyboard.svg",
        }
    }
}

/// A text field being edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldId {
    /// A Ghostty colour key.
    Ghostty(&'static str),
}

/// A slider being dragged: its key, range and the value under the pointer.
struct Drag {
    key: &'static str,
    min: f32,
    max: f32,
    value: f32,
}

pub struct SettingsView {
    env: Env,
    style: Rc<TermStyle>,
    /// What the rows show: the last loaded config plus this window's own changes since.
    cfg: Config,
    section: Section,
    ui_scale: f32,
    /// Ghostty changes waiting for the debounce, in order, one per key.
    pending: Vec<(&'static str, String)>,
    write_task: Option<Task<()>>,
    error: Option<String>,
    /// Keys whose winning line is in a file other than the main one (plan W2).
    origins: HashMap<&'static str, PathBuf>,
    field: Option<(FieldId, RenameState)>,
    field_focus: FocusHandle,
    /// The open dropdown's id.
    dropdown: Option<&'static str>,
    /// Installed monospace families, found on the first open of the font list.
    fonts: Option<Vec<String>>,
    font_scan: Option<Task<()>>,
    drag: Option<Drag>,
    /// Each slider's track, from its last paint.
    tracks: HashMap<&'static str, Rc<Cell<Bounds<Pixels>>>>,
    focus: FocusHandle,
    scroll: ScrollHandle,
    _subs: Vec<Subscription>,
}

/// Focuses the open Settings window, or opens one (plan S1).
pub fn open(
    existing: Option<WindowHandle<SettingsView>>,
    env: Env,
    style: Rc<TermStyle>,
    cx: &mut App,
) -> Option<WindowHandle<SettingsView>> {
    if let Some(h) = existing
        && h.update(cx, |_, window, _| window.activate_window())
            .is_ok()
    {
        return Some(h);
    }
    let bounds = Bounds::centered(None, gpui::size(gpui::px(WINDOW_W), gpui::px(WINDOW_H)), cx);
    let opened = cx.open_window(
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("Settings".into()),
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            app_id: Some("cmux-linux".into()),
            window_decorations: Some(gpui::WindowDecorations::Server),
            ..Default::default()
        },
        |window, cx| cx.new(|cx| SettingsView::new(env, style, window, cx)),
    );
    opened
        .inspect_err(|e| eprintln!("cmux-linux: could not open Settings: {e}"))
        .ok()
}

/// `v` moved one `inc` in direction `dir` (±1) on the `inc` grid, clamped to `min..=max` and
/// rounded to 2 decimals. `None` when that doesn't move it that way (already at the end).
pub(crate) fn step(v: f32, dir: f32, inc: f32, min: f32, max: f32) -> Option<f32> {
    let k = v / inc;
    let n = if dir > 0.0 {
        (k + 1e-3).floor() + 1.0
    } else {
        (k - 1e-3).ceil() - 1.0
    };
    let n = ((n * inc).clamp(min, max) * 100.0).round() / 100.0;
    ((n - v) * dir > 1e-4).then_some(n)
}

/// A slider position as a value on the `inc` grid.
pub(crate) fn snap(frac: f32, min: f32, max: f32, inc: f32) -> f32 {
    let v = min + frac.clamp(0.0, 1.0) * (max - min);
    (((v / inc).round() * inc).clamp(min, max) * 100.0).round() / 100.0
}

/// `path` with the home directory as `~`.
fn tilde(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|h| path.strip_prefix(h).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

impl SettingsView {
    fn new(env: Env, style: Rc<TermStyle>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // gpui on Wayland ignores `TitlebarOptions::title`.
        window.set_window_title("Settings");
        let focus = cx.focus_handle();
        window.focus(&focus);
        let field_focus = cx.focus_handle();
        let subs = vec![
            cx.on_blur(&field_focus, window, |this, window, cx| {
                // Alt-tab clears focus too; a half-typed value must survive a window switch.
                if window.is_window_active() {
                    this.commit_field(window, cx);
                }
            }),
            // Closing within the debounce must not lose the last change.
            cx.on_release(|this, _| this.write_pending()),
        ];
        let mut this = Self {
            cfg: style.cfg.clone(),
            env,
            style,
            section: Section::App,
            ui_scale: crate::ui::scale(),
            pending: Vec::new(),
            write_task: None,
            error: None,
            origins: HashMap::new(),
            field: None,
            field_focus,
            dropdown: None,
            fonts: None,
            font_scan: None,
            drag: None,
            tracks: HashMap::new(),
            focus,
            scroll: ScrollHandle::new(),
            _subs: subs,
        };
        this.origins = this.find_origins();
        this
    }

    /// The main window reloaded the Ghostty config: show what is in effect now, unless a change
    /// of ours is still on its way to the file.
    pub fn set_style(&mut self, style: Rc<TermStyle>, cx: &mut Context<Self>) {
        if self.pending.is_empty() && self.drag.is_none() {
            self.cfg = style.cfg.clone();
        }
        self.style = style;
        self.origins = self.find_origins();
        cx.notify();
    }

    fn find_origins(&self) -> HashMap<&'static str, PathBuf> {
        let main = edit::main_file(&self.env);
        terminal::KEYS
            .iter()
            .filter_map(|&k| {
                let (path, _) = ghostty::origin(&self.env, k)?;
                (path != main).then_some((k, path))
            })
            .collect()
    }

    /// W2's note for a row whose value comes from another file.
    fn origin_note(&self, key: &str, c: Colors) -> Option<(String, gpui::Rgba)> {
        let path = self.origins.get(key)?;
        let main = edit::main_file(&self.env);
        let name = main
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let home = self.env.home.as_deref();
        Some((
            format!(
                "Set in {}. Changes here go to {name}; that file wins if it loads later.",
                tilde(path, home)
            ),
            c.warning,
        ))
    }

    /// Records a Ghostty change and writes it after `WRITE_DELAY` with any that follow.
    fn set_ghostty(&mut self, key: &'static str, value: String, cx: &mut Context<Self>) {
        if key == "font-family" {
            // The write leaves one font-family line, so the list is just this one.
            self.cfg.font_family.clear();
        }
        if let Err(e) = self.cfg.apply(key, &value) {
            self.error = Some(format!("{key}: {e}"));
            cx.notify();
            return;
        }
        self.pending.retain(|(k, _)| *k != key);
        self.pending.push((key, value));
        self.write_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(WRITE_DELAY).await;
            this.update(cx, |this, cx| {
                this.write_pending();
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn write_pending(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let path = edit::main_file(&self.env);
        let changes: Vec<(&str, &str)> =
            self.pending.iter().map(|(k, v)| (*k, v.as_str())).collect();
        self.error = edit::set_all(&path, &changes)
            .err()
            .map(|e| format!("Couldn't save {}: {e}", path.display()));
        self.pending.clear();
    }

    fn set_ui_scale(&mut self, v: f32, cx: &mut Context<Self>) {
        let v = v.clamp(UI_SCALE_MIN, UI_SCALE_MAX);
        let path = self.env.cmux_config_file();
        match cmux_config::write::set_ui_scale(&path, v) {
            Ok(()) => {
                self.ui_scale = v;
                self.error = None;
            }
            Err(e) => self.error = Some(format!("Couldn't save {}: {e}", path.display())),
        }
        cx.notify();
    }

    /// A click handler that runs `f` on this view.
    fn handler(
        &self,
        cx: &Context<Self>,
        f: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> Handler {
        let this = cx.entity().downgrade();
        Rc::new(move |window, cx| {
            this.update(cx, |this, cx| f(this, window, cx)).ok();
        })
    }

    /// config.toml changed on disk (the main window's poll).
    pub fn toml_changed(&mut self, cx: &mut Context<Self>) {
        self.ui_scale = crate::ui::scale();
        cx.notify();
    }

    /// Up/Down move through the sidebar, as in cmux's sidebar list.
    fn on_key(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let all: Vec<Section> = GROUPS.iter().flat_map(|(_, s)| s.iter().copied()).collect();
        let at = all.iter().position(|s| *s == self.section).unwrap_or(0);
        let to = match ev.keystroke.key.as_str() {
            "up" => at.checked_sub(1),
            "down" => Some(at + 1).filter(|k| *k < all.len()),
            _ => return,
        };
        if let Some(k) = to {
            self.select(all[k], cx);
        }
    }

    fn select(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        self.dropdown = None;
        self.scroll.set_offset(Default::default());
        cx.notify();
    }

    // ---- text fields ----

    fn start_field(
        &mut self,
        id: FieldId,
        current: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.field = Some((id, RenameState::new(current, true)));
        window.focus(&self.field_focus);
        cx.notify();
    }

    fn on_field_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        let Some((_, state)) = self.field.as_mut() else {
            return;
        };
        match state.apply(RenameKey::from(&ev.keystroke)) {
            RenameOutcome::Editing => {}
            RenameOutcome::Commit(_) => {
                self.commit_field(window, cx);
                window.focus(&self.focus);
            }
            RenameOutcome::Cancel => {
                self.field = None;
                window.focus(&self.focus);
            }
        }
        cx.notify();
    }

    fn commit_field(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some((id, state)) = self.field.take() else {
            return;
        };
        match id {
            FieldId::Ghostty(key) => self.commit_color(key, state.text.trim(), cx),
        }
        cx.notify();
    }

    /// The editing text of field `id`, if it is the one being edited.
    fn editing(&self, id: FieldId) -> Option<&str> {
        self.field
            .as_ref()
            .filter(|(f, _)| *f == id)
            .map(|(_, s)| s.text.as_str())
    }

    // ---- sliders ----

    fn track(&mut self, key: &'static str) -> Rc<Cell<Bounds<Pixels>>> {
        self.tracks.entry(key).or_default().clone()
    }

    fn start_drag(
        &mut self,
        key: &'static str,
        min: f32,
        max: f32,
        x: Pixels,
        cx: &mut Context<Self>,
    ) {
        let mut d = Drag {
            key,
            min,
            max,
            value: min,
        };
        self.drag_to(&mut d, x);
        self.drag = Some(d);
        cx.notify();
    }

    fn drag_to(&self, d: &mut Drag, x: Pixels) {
        let Some(b) = self.tracks.get(d.key).map(|t| t.get()) else {
            return;
        };
        let frac = controls::fraction(f32::from(x), f32::from(b.origin.x), f32::from(b.size.width));
        d.value = snap(frac, d.min, d.max, terminal::SLIDER_STEP);
    }

    fn on_mouse_move(&mut self, ev: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(mut d) = self.drag.take() else {
            return;
        };
        if ev.pressed_button != Some(MouseButton::Left) {
            self.drag = Some(d);
            return self.end_drag(cx);
        }
        self.drag_to(&mut d, ev.position.x);
        self.drag = Some(d);
        cx.notify();
    }

    /// The slider writes on release (plan S5).
    fn end_drag(&mut self, cx: &mut Context<Self>) {
        if let Some(d) = self.drag.take() {
            self.set_ghostty(d.key, format!("{:.2}", d.value), cx);
        }
    }

    /// A slider's shown value: the dragged one while dragging.
    fn slider_value(&self, key: &str, current: f32) -> f32 {
        self.drag
            .as_ref()
            .filter(|d| d.key == key)
            .map_or(current, |d| d.value)
    }

    // ---- render ----

    fn colors(&self) -> Colors {
        Colors::new(crate::chrome::rgba(self.style.cfg.background, 1.0))
    }

    fn render_sidebar(&self, c: Colors, cx: &Context<Self>) -> impl IntoElement {
        let mut items = Vec::new();
        for (group, sections) in GROUPS {
            items.push(
                div()
                    .px(px(10.))
                    .pt(px(12.))
                    .pb(px(4.))
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c.secondary)
                    .child(*group)
                    .into_any_element(),
            );
            for &s in *sections {
                let on = s == self.section;
                let fg = if on { gpui::white().into() } else { c.text };
                items.push(
                    div()
                        .id(SharedString::from(s.title()))
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(10.))
                        .px(px(10.))
                        .py(px(5.))
                        .rounded(px(6.))
                        .when(on, |d| d.bg(c.accent))
                        .when(!on, |d| d.hover(move |st| st.bg(c.hover)))
                        .child(
                            svg()
                                .path(s.icon())
                                .size(px(16.))
                                .flex_none()
                                .text_color(fg),
                        )
                        .child(div().text_size(px(13.)).text_color(fg).child(s.title()))
                        .on_click(
                            cx.listener(move |this, _: &ClickEvent, _, cx| this.select(s, cx)),
                        )
                        .into_any_element(),
                );
            }
        }
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(SIDEBAR_W))
            .h_full()
            .px(px(8.))
            .py(px(8.))
            .bg(c.sidebar)
            .border_r_1()
            .border_color(c.separator)
            .children(items)
    }

    fn render_app(&self, c: Colors, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        let scale = self.ui_scale;
        let by = |dir: f32| {
            self.handler(cx, move |this, _, cx| {
                if let Some(v) = step(
                    this.ui_scale,
                    dir,
                    UI_SCALE_STEP,
                    UI_SCALE_MIN,
                    UI_SCALE_MAX,
                ) {
                    this.set_ui_scale(v, cx);
                }
            })
        };
        let row = controls::row(
            "Interface Scale",
            Some("ui_scale · the size of the app's own text and controls"),
            controls::stepper(
                "ui-scale",
                format!("{:.0}%", scale * 100.0),
                c,
                by(-1.0),
                by(1.0),
            ),
            None,
            c,
        );
        vec![controls::card(vec![row], c).into_any_element()]
    }

    fn render_placeholder(&self, c: Colors) -> Vec<gpui::AnyElement> {
        vec![
            controls::card(vec![controls::note("Coming next.", c.secondary)], c).into_any_element(),
        ]
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = self.colors();
        let body = match self.section {
            Section::App => self.render_app(c, cx),
            Section::Terminal => self.render_terminal(c, window, cx),
            Section::Themes | Section::Automation | Section::KeyboardShortcuts => {
                self.render_placeholder(c)
            }
        };
        let detail = div()
            .id("settings-detail")
            .flex_1()
            .min_w(px(0.))
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .p(px(20.))
                    .child(controls::header(self.section.title(), c))
                    .children(
                        self.error
                            .clone()
                            .map(|e| div().text_size(px(11.)).text_color(c.error).child(e)),
                    )
                    .children(body),
            );
        div()
            .id("settings")
            .key_context("Settings")
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &Quit, _, cx| cx.quit()))
            .on_action(|_: &OpenSettings, _, _| {})
            .on_action(|_: &CloseTab, window, _| window.remove_window())
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| this.on_key(ev, cx)))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.end_drag(cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.end_drag(cx)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    // A click anywhere else ends a text edit.
                    if this.field.is_some() {
                        this.commit_field(window, cx);
                        window.focus(&this.focus);
                    }
                }),
            )
            .flex()
            .flex_row()
            .size_full()
            .bg(c.bg)
            .text_color(c.text)
            .child(self.render_sidebar(c, cx))
            .child(detail)
            .children(crate::csd::controls(window))
            .children(crate::csd::resize_grips(window))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_moves_on_the_grid_and_stops_at_the_ends() {
        assert_eq!(step(1.0, 1.0, 0.05, 0.5, 2.0), Some(1.05));
        assert_eq!(step(1.0, -1.0, 0.05, 0.5, 2.0), Some(0.95));
        assert_eq!(step(2.0, 1.0, 0.05, 0.5, 2.0), None);
        assert_eq!(step(0.5, -1.0, 0.05, 0.5, 2.0), None);
        // A value from the file past the range: + does nothing, - lands on the end.
        assert_eq!(step(2.5, 1.0, 0.05, 0.5, 2.0), None);
        assert_eq!(step(2.5, -1.0, 0.05, 0.5, 2.0), Some(2.0));
        // Off the grid: the next grid point, not a full step past it.
        assert_eq!(step(13.5, 1.0, 1.0, 4.0, 96.0), Some(14.0));
        assert_eq!(step(13.5, -1.0, 1.0, 4.0, 96.0), Some(13.0));
        assert_eq!(step(1.02, 1.0, 0.05, 0.5, 2.0), Some(1.05));
    }

    #[test]
    fn snap_rounds_to_the_step_inside_the_range() {
        assert_eq!(snap(0.0, 0.0, 1.0, 0.05), 0.0);
        assert_eq!(snap(0.51, 0.0, 1.0, 0.05), 0.5);
        assert_eq!(snap(1.2, 0.0, 1.0, 0.05), 1.0);
        assert_eq!(snap(0.0, 0.15, 1.0, 0.05), 0.15);
        assert_eq!(snap(1.0, 0.15, 1.0, 0.05), 1.0);
    }

    #[test]
    fn tilde_shortens_home_paths() {
        let home = Path::new("/home/u");
        assert_eq!(
            tilde(Path::new("/home/u/.config/x"), Some(home)),
            "~/.config/x"
        );
        assert_eq!(tilde(Path::new("/etc/x"), Some(home)), "/etc/x");
        assert_eq!(tilde(Path::new("/etc/x"), None), "/etc/x");
    }
}
