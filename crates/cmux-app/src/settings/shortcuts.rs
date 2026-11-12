//! The Keyboard Shortcuts section (M12 plan S6), after sui `KeyboardShortcutsSection.swift`,
//! `ShortcutListRowView.swift` and `ShortcutRecorderView.swift`: a recorder per action that
//! writes `[keybindings]` in config.toml.

use cmux_config::keybindings::{self, DEFAULTS};
use cmux_config::write;
use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, Keystroke, ParentElement,
    Rgba, SharedString, StatefulInteractiveElement, Styled, div, prelude::*,
};

use super::SettingsView;
use super::controls::{self, Colors};
use crate::actions::PALETTE;
use crate::ui::px;

const RECORDER_W: f32 = 160.0;

/// cmux's `ShortcutAction.displayName`; the palette table has all but these four.
fn title(action: &str) -> &str {
    match action {
        "selectSurfaceByNumber" => "Select Surface 1…9",
        "selectWorkspaceByNumber" => "Select Workspace 1…9",
        "commandPaletteNext" => "Command Palette: Next",
        "commandPalettePrevious" => "Command Palette: Previous",
        _ => PALETTE
            .iter()
            .find(|(a, _)| *a == action)
            .map_or(action, |(_, t)| t),
    }
}

/// Keys that only change modifiers; the recorder waits for the key that goes with them.
fn modifier_only(key: &str) -> bool {
    matches!(
        key,
        "shift" | "control" | "alt" | "platform" | "super" | "function" | "capslock"
    )
}

impl SettingsView {
    /// Captures the next key press in this window before gpui matches it against the key
    /// bindings, so Super+, or Super+Q record instead of running (plan S6).
    fn start_recording(&mut self, action: &'static str, cx: &mut Context<Self>) {
        self.recording = Some(action);
        self.shortcut_note = None;
        let me = self.window;
        let this = cx.entity().downgrade();
        self.record_sub = Some(cx.intercept_keystrokes(move |ev, window, cx| {
            if window.window_handle() != me || modifier_only(&ev.keystroke.key) {
                return;
            }
            cx.stop_propagation();
            let ks = ev.keystroke.clone();
            this.update(cx, |this, cx| this.record(ks, cx)).ok();
        }));
        cx.notify();
    }

    pub(super) fn stop_recording(&mut self) {
        self.recording = None;
        self.record_sub = None;
    }

    fn record(&mut self, ks: Keystroke, cx: &mut Context<Self>) {
        let Some(action) = self.recording else {
            return;
        };
        let m = ks.modifiers;
        if ks.key == "escape" && !m.modified() {
            self.stop_recording();
            cx.notify();
            return;
        }
        let Some(chord) =
            keybindings::recorded_chord(&ks.key, m.control, m.alt, m.shift, m.platform)
        else {
            // A bare key keeps the recorder listening (cmux's pending rejection).
            self.shortcut_note = Some((
                action,
                "Shortcuts must include Super, Ctrl, Alt or Shift.".into(),
            ));
            cx.notify();
            return;
        };
        self.stop_recording();
        match keybindings::conflict(&self.keys, action, &chord) {
            Err(e) => self.shortcut_note = Some((action, e)),
            Ok(Some(b)) => {
                self.shortcut_note = Some((
                    action,
                    format!(
                        "This shortcut conflicts with {} ({}).",
                        title(b.action),
                        b.hint
                    ),
                ))
            }
            Ok(None) => self.write_toml(|p| write::set_keybinding(p, action, &[&chord]), cx),
        }
        cx.notify();
    }

    /// The action's chords as configured; a numbered action shows its 1 as `1…9`.
    fn hints(&self, action: &str) -> Vec<String> {
        self.keys
            .bindings
            .iter()
            .filter(|b| b.action == action && b.number.is_none_or(|n| n == 1))
            .map(|b| match b.number {
                Some(_) => format!("{}…9", b.hint),
                None => b.hint.clone(),
            })
            .collect()
    }

    fn shortcut_row(&self, action: &'static str, c: Colors, cx: &Context<Self>) -> AnyElement {
        let hints = self.hints(action);
        let recording = self.recording == Some(action);
        let (label, color) = if recording {
            ("Press shortcut…".to_string(), c.accent)
        } else if hints.is_empty() {
            ("None".to_string(), c.secondary)
        } else {
            (hints.join(", "), c.text)
        };
        let recorder = div()
            .id(SharedString::from(format!("record-{action}")))
            .flex()
            .items_center()
            .justify_center()
            .w(px(RECORDER_W))
            .h(px(22.))
            .px(px(6.))
            .rounded(px(5.))
            .bg(c.control)
            .border_1()
            .border_color(if recording { c.accent } else { c.separator })
            .text_size(px(12.))
            .text_color(color)
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if this.recording == Some(action) {
                    this.stop_recording();
                    cx.notify();
                } else {
                    this.start_recording(action, cx);
                }
            }));
        // × unbinds; once unbound, ↺ brings the defaults back (cmux's clear/restore button).
        let defaults = DEFAULTS
            .iter()
            .find(|(a, _)| *a == action)
            .is_some_and(|(_, d)| !d.is_empty());
        let restore = hints.is_empty() && defaults;
        let disabled = hints.is_empty() && !defaults;
        let button = div()
            .id(SharedString::from(format!("unbind-{action}")))
            .flex()
            .items_center()
            .justify_center()
            .size(px(22.))
            .rounded_full()
            .text_size(px(13.))
            .text_color(if disabled { c.control } else { c.secondary })
            .when(!disabled, |d| d.hover(move |s| s.bg(c.hover)))
            .child(if restore { "↺" } else { "×" })
            .when(!disabled, |d| {
                d.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.stop_recording();
                    this.shortcut_note = None;
                    if restore {
                        this.write_toml(|p| write::reset_keybinding(p, action), cx);
                    } else {
                        this.write_toml(|p| write::set_keybinding(p, action, &[]), cx);
                    }
                }))
            });
        let control = div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .child(recorder)
            .child(button)
            .into_any_element();
        let row = controls::row(title(action), Some(action), control, None, c);
        let Some((_, msg)) = self.shortcut_note.as_ref().filter(|(a, _)| *a == action) else {
            return row;
        };
        // sui `ShortcutListRowView`'s validation banner.
        let red = c.error;
        let banner = div()
            .mx(px(14.))
            .mb(px(9.))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.))
            .px(px(8.))
            .py(px(6.))
            .rounded(px(6.))
            .bg(Rgba { a: 0.12, ..red })
            .border_1()
            .border_color(Rgba { a: 0.35, ..red })
            .text_size(px(11.))
            .child(
                div()
                    .flex_1()
                    .text_color(red)
                    .child(SharedString::from(msg.clone())),
            )
            .child(
                div()
                    .id("shortcut-undo")
                    .text_color(c.accent)
                    .child("Undo")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.shortcut_note = None;
                        this.stop_recording();
                        cx.notify();
                    })),
            );
        div()
            .flex()
            .flex_col()
            .child(row)
            .child(banner)
            .into_any_element()
    }

    pub(super) fn render_shortcuts(&self, c: Colors, cx: &Context<Self>) -> Vec<AnyElement> {
        let reset = div()
            .id("reset-shortcuts")
            .flex()
            .items_center()
            .h(px(22.))
            .px(px(10.))
            .rounded(px(5.))
            .bg(c.control)
            .text_size(px(12.))
            .hover(move |s| s.bg(c.hover))
            .child("↺ Reset Defaults")
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.stop_recording();
                this.shortcut_note = None;
                this.write_toml(
                    |p| {
                        DEFAULTS
                            .iter()
                            .try_for_each(|(a, _)| write::reset_keybinding(p, a))
                    },
                    cx,
                );
            }))
            .into_any_element();
        let mut rows = vec![controls::row(
            "Reset Default Shortcuts",
            Some("Restore every shortcut set here or in config.toml to its default."),
            reset,
            None,
            c,
        )];
        rows.extend(DEFAULTS.iter().map(|(a, _)| self.shortcut_row(a, c, cx)));
        vec![
            controls::card(rows, c).into_any_element(),
            div()
                .pl(px(2.))
                .text_size(px(11.))
                .text_color(c.secondary)
                .child(
                    "Click a shortcut to record a new one; Escape cancels. × unbinds; it changes \
                     to restore after a clear.",
                )
                .into_any_element(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_has_a_title() {
        for (a, _) in DEFAULTS {
            assert_ne!(title(a), *a, "{a}");
        }
    }
}
