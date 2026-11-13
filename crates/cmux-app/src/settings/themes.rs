//! The Themes section (M12 plan S4), after sui `TerminalThemeGallery.swift`: the current
//! light/dark pair, a slot toggle, a search field and a card per theme. A click writes `theme`.

use std::rc::Rc;

use cmux_config::ghostty::{ThemePreview, edit, theme_pair};
use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Rgba, SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*,
};

use super::{FieldId, SettingsView};
use crate::chrome::{rgba, wcag_is_light};
use crate::mac::controls;
use crate::mac::{Theme, alpha, metric};
use crate::ui::{px, s};

const CARD_MIN: f32 = 128.0;
const CARD_MAX: f32 = 220.0;
const GAP: f32 = 10.0;
const PREVIEW_H: f32 = 54.0;
const SEARCH_W: f32 = 200.0;
/// The detail column's and the card's horizontal padding, for the grid's width.
const INSET: f32 = 2.0 * 20.0 + 2.0 * 14.0 + 2.0;

/// Card width for `avail` px, as SwiftUI's `.adaptive(minimum:maximum:)`: as many columns as
/// fit `min`, sharing the width, at most `max` each.
fn card_width(avail: f32, min: f32, max: f32, gap: f32) -> f32 {
    let cols = (((avail + gap) / (min + gap)).floor()).max(1.0);
    ((avail - gap * (cols - 1.0)) / cols).min(max)
}

/// The cards for `query`, split by appearance: those suiting the `dark` slot first.
fn results<'a>(
    themes: &'a [ThemePreview],
    query: &str,
    dark: bool,
) -> (Vec<&'a ThemePreview>, Vec<&'a ThemePreview>) {
    let q = query.trim().to_lowercase();
    themes
        .iter()
        .filter(|t| q.is_empty() || t.name.to_lowercase().contains(&q))
        .partition(|t| !wcag_is_light(rgba(t.background, 1.0)) == dark)
}

/// The `theme` value after picking `name` for one slot. A side that has no theme yet takes it
/// too (a pair with a missing side fails to load).
pub(super) fn picked(current: Option<&str>, name: &str, dark: bool) -> String {
    let (light, dk) = current.map(theme_pair).unwrap_or_default();
    let light = light.filter(|_| dark).unwrap_or_else(|| name.to_string());
    let dk = dk.filter(|_| !dark).unwrap_or_else(|| name.to_string());
    edit::theme_value(&light, &dk)
}

impl SettingsView {
    fn load_themes(&mut self, cx: &mut Context<Self>) {
        if self.themes.is_some() || self.theme_scan.is_some() {
            return;
        }
        let env = self.env.clone();
        self.theme_scan = Some(cx.spawn(async move |this, cx| {
            let scan = async move { cmux_config::ghostty::themes(&env) };
            let themes = cx.background_executor().spawn(scan).await;
            this.update(cx, |this, cx| {
                this.themes = Some(Rc::new(themes));
                cx.notify();
            })
            .ok();
        }));
    }

    /// Writes at once (plan S4), not after the Terminal rows' debounce.
    fn pick_theme(&mut self, name: &str, cx: &mut Context<Self>) {
        let value = picked(self.theme.as_deref(), name, self.theme_dark);
        let path = edit::main_file(&self.env);
        match edit::set(&path, "theme", &value) {
            Ok(()) => {
                self.theme = Some(value);
                self.error = None;
            }
            Err(e) => self.error = Some(format!("Couldn't save {}: {e}", path.display())),
        }
        cx.notify();
    }

    fn theme_card(
        &self,
        t: &ThemePreview,
        selected: bool,
        w: f32,
        c: Theme,
        cx: &Context<Self>,
    ) -> AnyElement {
        let fg = rgba(t.foreground, 1.0);
        let palette: Vec<Option<Rgba>> =
            t.palette.iter().map(|p| p.map(|p| rgba(p, 1.0))).collect();
        let name = t.name.clone();
        let accent = c.accent;
        div()
            .id(SharedString::from(format!("theme-{}", t.name)))
            .w(gpui::px(w))
            .p(px(5.))
            .rounded(px(metric::CONTROL_RADIUS))
            .border_2()
            .border_color(if selected {
                accent
            } else {
                gpui::transparent_black().into()
            })
            .when(selected, |d| d.bg(alpha(accent, 0.12)))
            .child(
                div()
                    .relative()
                    .h(px(PREVIEW_H))
                    .rounded(px(metric::CONTROL_RADIUS - 2.))
                    .bg(rgba(t.background, 1.0))
                    .border_1()
                    .border_color(alpha(c.text, 0.1))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.))
                            .px(px(8.))
                            .pt(px(8.))
                            .child(
                                div()
                                    .min_w(px(0.))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .font_family(self.style.font.family.clone())
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(fg)
                                    .child(SharedString::from(t.name.clone())),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .w(px(5.))
                                    .h(px(11.))
                                    .rounded(px(1.)) // mac-ok: the theme's cursor shape
                                    .bg(t.cursor.map_or(fg, |c| rgba(c, 1.0))),
                            ),
                    )
                    // Two rows of eight ANSI swatches, painted in one pass (as cmux's Canvas).
                    .child(
                        canvas(
                            |_, _, _| {},
                            move |b, _, window, _| {
                                let (inset, gap, h) = (s(8.), s(2.), s(8.));
                                let sw = (f32::from(b.size.width) - inset * 2. - gap * 7.) / 8.;
                                let bottom = f32::from(b.size.height) - inset - h;
                                for (k, color) in palette.iter().enumerate() {
                                    let Some(color) = color else { continue };
                                    let (row, col) = (k / 8, k % 8);
                                    let y = bottom - (1 - row) as f32 * (h + s(7.));
                                    let x = inset + col as f32 * (sw + gap);
                                    let r = gpui::Bounds::new(
                                        b.origin + gpui::point(gpui::px(x), gpui::px(y)),
                                        gpui::size(gpui::px(sw), gpui::px(h)),
                                    );
                                    window.paint_quad(
                                        gpui::fill(r, *color).corner_radii(gpui::px(s(2.))),
                                    );
                                }
                            },
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    ),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.pick_theme(&name, cx)))
            .into_any_element()
    }

    pub(super) fn render_themes(
        &mut self,
        c: Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        self.load_themes(cx);
        let (light, dark) = self.theme.as_deref().map(theme_pair).unwrap_or_default();
        let default = "Ghostty default colors".to_string();
        let (light, dark) = (light.unwrap_or(default.clone()), dark.unwrap_or(default));
        let subtitle = if light.eq_ignore_ascii_case(&dark) {
            light.clone()
        } else {
            format!("Light: {light} · Dark: {dark}")
        };
        let this = cx.entity().downgrade();
        let slot: controls::Pick = Rc::new(move |k, _, cx| {
            this.update(cx, |this, cx| {
                this.theme_dark = k == 1;
                cx.notify();
            })
            .ok();
        });
        let current = controls::row(
            "Terminal Theme",
            Some(&subtitle),
            controls::segmented(
                "theme-slot",
                &["Light", "Dark"],
                self.theme_dark as usize,
                c,
                slot,
            ),
            self.origin_note("theme", c),
            c,
        );
        let query = self
            .editing(FieldId::ThemeSearch)
            .unwrap_or(&self.theme_query)
            .to_string();
        let search = div().flex().flex_row().justify_end().child(self.field(
            FieldId::ThemeSearch,
            "theme-search",
            &self.theme_query,
            "Search all themes",
            None,
            SEARCH_W,
            window,
            cx,
        ));
        let mut gallery = vec![search.into_any_element()];
        match self.themes.clone() {
            None => gallery.push(controls::note("Loading themes…", c.secondary)),
            Some(themes) => {
                let avail = f32::from(window.viewport_size().width) - s(super::SIDEBAR_W + INSET);
                let w = card_width(avail, s(CARD_MIN), s(CARD_MAX), s(GAP));
                let in_use = if self.theme_dark { &dark } else { &light };
                let (mine, other) = results(&themes, &query, self.theme_dark);
                if themes.is_empty() {
                    gallery.push(controls::note("No themes found.", c.secondary));
                } else if mine.is_empty() && other.is_empty() {
                    gallery.push(controls::note("No themes match your search.", c.secondary));
                }
                for (list, is_dark) in [(mine, self.theme_dark), (other, !self.theme_dark)] {
                    if list.is_empty() {
                        continue;
                    }
                    gallery.push(
                        div()
                            .pt(px(4.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c.secondary)
                            .child(if is_dark {
                                "Dark Themes"
                            } else {
                                "Light Themes"
                            })
                            .into_any_element(),
                    );
                    gallery.push(
                        div()
                            .flex()
                            .flex_row()
                            .flex_wrap()
                            .gap(px(GAP))
                            .children(list.into_iter().map(|t| {
                                let on = t.name.eq_ignore_ascii_case(in_use);
                                self.theme_card(t, on, w, c, cx)
                            }))
                            .into_any_element(),
                    );
                }
            }
        }
        let gallery = div()
            .flex()
            .flex_col()
            .gap(px(GAP))
            .px(px(14.))
            .pb(px(12.))
            .children(gallery)
            .into_any_element();
        vec![controls::card(vec![current, gallery], c).into_any_element()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cards_share_the_width_between_min_and_max() {
        assert_eq!(card_width(100.0, 128.0, 220.0, 10.0), 100.0);
        assert_eq!(card_width(276.0, 128.0, 220.0, 10.0), 133.0);
        assert_eq!(card_width(700.0, 128.0, 220.0, 10.0), 132.0);
        assert_eq!(card_width(200.0, 50.0, 60.0, 0.0), 50.0);
    }

    #[test]
    fn a_pick_writes_one_slot_of_a_pair() {
        assert_eq!(picked(None, "Night", true), "Night");
        assert_eq!(picked(Some("Day"), "Night", true), "light:Day,dark:Night");
        assert_eq!(
            picked(Some("light:Day,dark:Night"), "Paper", false),
            "light:Paper,dark:Night"
        );
        assert_eq!(picked(Some("light:Day,dark:Night"), "Day", true), "Day");
        // A half pair gets its missing side.
        assert_eq!(
            picked(Some("dark:Night"), "Day", false),
            "light:Day,dark:Night"
        );
    }
}
