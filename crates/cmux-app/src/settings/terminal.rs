//! The Terminal section (M12 plan S5): Ghostty options, after sui
//! `TerminalGhosttyOptionsCard.swift`; the split rows are ours. Each row shows the value in
//! effect and writes the user's main Ghostty file.

use std::rc::Rc;

use cmux_config::ghostty::{CursorStyle, Rgb, parse_rgb};
use gpui::{AnyElement, Context, IntoElement, TextSystem, Window, prelude::*};

use super::{FieldId, SettingsView, step};
use crate::chrome::rgba;
use crate::mac::Theme;
use crate::mac::controls;

/// Every key this section writes, for the "Set in <file>" notes.
pub(super) const KEYS: &[&str] = &[
    "font-family",
    "font-size",
    "cursor-style",
    "cursor-style-blink",
    "background-opacity",
    "background-blur",
    "unfocused-split-opacity",
    "unfocused-split-fill",
    "split-divider-color",
];

pub(super) const SLIDER_STEP: f32 = 0.05;
const FONT_SIZE_MIN: f32 = 4.0;
const FONT_SIZE_MAX: f32 = 96.0;
const CURSORS: &[(CursorStyle, &str, &str)] = &[
    (CursorStyle::Block, "block", "Block"),
    (CursorStyle::Bar, "bar", "Bar"),
    (CursorStyle::Underline, "underline", "Underline"),
    (CursorStyle::BlockHollow, "block_hollow", "Hollow"),
];
const HEX_FIELD_W: f32 = 120.0;

/// The installed families whose `M` and `i` are equally wide, sorted.
fn monospace_families(ts: &TextSystem) -> Vec<String> {
    let size = gpui::px(16.);
    let mut out: Vec<String> = ts
        .all_font_names()
        .into_iter()
        .filter(|name| !name.starts_with('.'))
        .filter(|name| {
            let id = ts.resolve_font(&gpui::font(name.clone()));
            // resolve_font falls back to another family when this one can't load.
            if ts
                .get_font_for_id(id)
                .is_none_or(|f| f.family.as_ref() != name)
            {
                return false;
            }
            match (ts.advance(id, size, 'M'), ts.advance(id, size, 'i')) {
                (Ok(m), Ok(i)) => (f32::from(m.width) - f32::from(i.width)).abs() < 0.01,
                _ => false,
            }
        })
        .collect();
    out.sort_by_key(|f| f.to_lowercase());
    out
}

/// A Ghostty number as written: `14`, `13.5`.
fn number(v: f32) -> String {
    format!("{}", (v * 100.0).round() / 100.0)
}

fn hex(c: Option<Rgb>) -> String {
    c.map_or_else(String::new, |c| {
        format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
    })
}

/// What a colour field's text writes: `""` resets the key, else `#rrggbb`. `Err` for text
/// that isn't a colour.
pub(super) fn color_value(text: &str) -> Result<String, String> {
    if text.is_empty() {
        return Ok(String::new());
    }
    parse_rgb(text).map(|c| hex(Some(c)))
}

impl SettingsView {
    pub(super) fn commit_color(&mut self, key: &'static str, text: &str, cx: &mut Context<Self>) {
        match color_value(text) {
            Ok(v) => self.set_ghostty(key, v, cx),
            Err(e) => self.error = Some(format!("{key}: {e}")),
        }
    }

    fn open_fonts(&mut self, cx: &mut Context<Self>) {
        if self.dropdown.take().is_none() {
            // Loading every family takes about a second, so it runs off the UI thread.
            if self.fonts.is_none() && self.font_scan.is_none() {
                let ts = cx.text_system().clone();
                self.font_scan = Some(cx.spawn(async move |this, cx| {
                    let scan = async move { monospace_families(&ts) };
                    let fonts = cx.background_executor().spawn(scan).await;
                    this.update(cx, |this, cx| {
                        this.fonts = Some(fonts);
                        cx.notify();
                    })
                    .ok();
                }));
            }
            self.dropdown = Some("font-family");
        }
        cx.notify();
    }

    fn font_row(&self, c: Theme, cx: &Context<Self>) -> AnyElement {
        let current = self.cfg.font_family.first().cloned();
        // "Default" first, then the families; the configured one is kept even if not found.
        let mut items = vec!["Default".to_string()];
        if let Some(fonts) = &self.fonts {
            items.extend(fonts.iter().cloned());
        }
        if let Some(f) = current.as_ref().filter(|f| !items.contains(f)) {
            items.insert(1, f.clone());
        }
        let selected = match &current {
            None => Some(0),
            Some(f) => items.iter().position(|i| i == f),
        };
        let open = self.dropdown == Some("font-family");
        let pick_items = items.clone();
        let this = cx.entity().downgrade();
        let pick: controls::Pick = Rc::new(move |k, _, cx| {
            let value = if k == 0 {
                String::new()
            } else {
                pick_items[k].clone()
            };
            this.update(cx, |this, cx| {
                this.dropdown = None;
                this.set_ghostty("font-family", value, cx);
            })
            .ok();
        });
        let button = self.font_button.clone();
        let control = gpui::div()
            .relative()
            .flex()
            .flex_col()
            .child(
                gpui::canvas(move |b, _, _| button.set(b), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            )
            .child(controls::dropdown_button(
                "font-family",
                current.unwrap_or_else(|| "Default".into()),
                c,
                self.handler(cx, |this, _, cx| this.open_fonts(cx)),
            ))
            .when(open, |d| {
                d.child(controls::dropdown_list(
                    "font-list",
                    &items,
                    selected,
                    c,
                    pick,
                    self.handler(cx, |this, window, cx| {
                        // The button toggles the list itself.
                        if !this.font_button.get().contains(&window.mouse_position()) {
                            this.dropdown = None;
                            cx.notify();
                        }
                    }),
                ))
            })
            .into_any_element();
        self.ghostty_row("Font", "font-family", control, c)
    }

    fn ghostty_row(&self, title: &str, key: &str, control: AnyElement, c: Theme) -> AnyElement {
        controls::row(title, Some(key), control, self.origin_note(key, c), c)
    }

    fn slider_row(
        &mut self,
        title: &str,
        key: &'static str,
        current: f32,
        (min, max): (f32, f32),
        c: Theme,
        cx: &Context<Self>,
    ) -> AnyElement {
        let v = self.slider_value(key, current);
        let track = self.track(key);
        let this = cx.entity().downgrade();
        let control = controls::slider(
            key,
            (v - min) / (max - min),
            format!("{:.0}%", v * 100.0),
            track,
            c,
            move |ev, _, cx| {
                this.update(cx, |this, cx| {
                    this.start_drag(key, min, max, ev.position.x, cx)
                })
                .ok();
            },
        );
        self.ghostty_row(title, key, control, c)
    }

    fn toggle_row(
        &self,
        title: &str,
        key: &'static str,
        on: bool,
        c: Theme,
        cx: &Context<Self>,
    ) -> AnyElement {
        let click = self.handler(cx, move |this, _, cx| {
            this.set_ghostty(key, (!on).to_string(), cx)
        });
        self.ghostty_row(title, key, controls::toggle(key, on, c, click), c)
    }

    fn color_row(
        &self,
        title: &str,
        key: &'static str,
        current: Option<Rgb>,
        window: &Window,
        c: Theme,
        cx: &Context<Self>,
    ) -> AnyElement {
        let swatch = current.map(|c| rgba(c, 1.0));
        let field = self.field(
            FieldId::Ghostty(key),
            key,
            &hex(current),
            "Default",
            swatch,
            HEX_FIELD_W,
            window,
            cx,
        );
        self.ghostty_row(title, key, field, c)
    }

    pub(super) fn render_terminal(
        &mut self,
        c: Theme,
        window: &Window,
        cx: &Context<Self>,
    ) -> Vec<AnyElement> {
        let cfg = self.cfg.clone();
        let size_by = |dir: f32| {
            self.handler(cx, move |this, _, cx| {
                if let Some(v) = step(this.cfg.font_size, dir, 1.0, FONT_SIZE_MIN, FONT_SIZE_MAX) {
                    this.set_ghostty("font-size", number(v), cx);
                }
            })
        };
        let font_size = self.ghostty_row(
            "Font Size",
            "font-size",
            controls::stepper(
                "font-size",
                format!("{} pt", number(cfg.font_size)),
                c,
                size_by(-1.0),
                size_by(1.0),
            ),
            c,
        );
        let selected = CURSORS
            .iter()
            .position(|(s, _, _)| *s == cfg.cursor_style)
            .unwrap_or(0);
        let this = cx.entity().downgrade();
        let pick: controls::Pick = Rc::new(move |k, _, cx| {
            this.update(cx, |this, cx| {
                this.set_ghostty("cursor-style", CURSORS[k].1.to_string(), cx)
            })
            .ok();
        });
        let titles: Vec<&str> = CURSORS.iter().map(|(_, _, t)| *t).collect();
        let cursor = self.ghostty_row(
            "Cursor Style",
            "cursor-style",
            controls::segmented("cursor-style", &titles, selected, c, pick),
            c,
        );
        let rows = vec![
            self.font_row(c, cx),
            font_size,
            cursor,
            // Ghostty blinks unless told otherwise.
            self.toggle_row(
                "Blinking Cursor",
                "cursor-style-blink",
                cfg.cursor_style_blink.unwrap_or(true),
                c,
                cx,
            ),
            self.slider_row(
                "Background Opacity",
                "background-opacity",
                cfg.background_opacity,
                (0.0, 1.0),
                c,
                cx,
            ),
            self.toggle_row(
                "Background Blur",
                "background-blur",
                cfg.background_blur,
                c,
                cx,
            ),
            self.slider_row(
                "Unfocused Split Opacity",
                "unfocused-split-opacity",
                cfg.unfocused_split_opacity,
                (0.15, 1.0),
                c,
                cx,
            ),
            self.color_row(
                "Unfocused Split Fill",
                "unfocused-split-fill",
                cfg.unfocused_split_fill,
                window,
                c,
                cx,
            ),
            self.color_row(
                "Split Divider Color",
                "split-divider-color",
                cfg.split_divider_color,
                window,
                c,
                cx,
            ),
        ];
        let main = cmux_config::ghostty::edit::main_file(&self.env);
        let mut all = vec![controls::note(
            format!(
                "These rows show the value in effect and save to your Ghostty config ({}). \
                 The caption under each row is its config key.",
                super::tilde(&main, self.env.home.as_deref())
            ),
            c.secondary,
        )];
        all.extend(rows);
        vec![controls::card(all, c).into_any_element()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_fields_write_hex_or_reset() {
        assert_eq!(color_value(""), Ok(String::new()));
        assert_eq!(color_value("#ABC").as_deref(), Ok("#aabbcc"));
        assert_eq!(color_value("102030").as_deref(), Ok("#102030"));
        assert!(color_value("blue!").is_err());
    }

    #[test]
    fn numbers_are_written_short() {
        assert_eq!(number(14.0), "14");
        assert_eq!(number(13.5), "13.5");
    }
}
