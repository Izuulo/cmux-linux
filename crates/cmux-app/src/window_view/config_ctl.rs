//! Live config apply (M12 plan A1–A4): one 1 s poll stats `config.toml` and every Ghostty file
//! the last load read; a change reloads what that file feeds. The desktop's light/dark flip
//! reloads the Ghostty side too (`theme = light:X,dark:Y`).

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use cmux_config::ghostty::{self, Env};
use cmux_config::keybindings;
use cmux_config::watch::Watched;
use gpui::{Context, Task, Window, WindowAppearance};

use super::{WindowView, vault_ctl};
use crate::style::{TermStyle, installed_families};

const CONFIG_EVERY: Duration = Duration::from_millis(1000);

#[derive(Default)]
pub(super) struct ConfigWatch {
    /// `None` without a home directory: nothing to watch.
    env: Option<Env>,
    ghostty: Watched,
    toml: Watched,
}

impl ConfigWatch {
    /// `ghostty_files`: what the startup load read (`Loaded::files`).
    pub(super) fn new(env: Option<Env>, ghostty_files: Vec<PathBuf>) -> Self {
        let toml = Watched::new(env.iter().map(Env::cmux_config_file).collect());
        Self {
            env,
            ghostty: Watched::new(ghostty_files),
            toml,
        }
    }
}

fn is_dark(a: WindowAppearance) -> bool {
    matches!(a, WindowAppearance::Dark | WindowAppearance::VibrantDark)
}

fn log(warnings: &[String]) {
    for w in warnings {
        eprintln!("cmux-linux: config: {w}");
    }
}

impl WindowView {
    pub(super) fn config_loop(window: &mut Window, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(CONFIG_EVERY).await;
                if this
                    .update_in(cx, |s, window, cx| s.check_config(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
    }

    /// A light/dark flip picks the other half of a theme pair (plan A3).
    pub(super) fn appearance_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dark = is_dark(window.appearance());
        if let Some(env) = self.config.env.as_mut().filter(|e| e.dark != dark) {
            env.dark = dark;
            self.reload_ghostty(window, cx);
        }
    }

    // Stats a handful of files on the UI thread: microseconds, cheaper than a background hop.
    fn check_config(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.config.ghostty.changed() {
            self.reload_ghostty(window, cx);
        }
        if self.config.toml.changed() {
            self.reload_toml(cx);
        }
    }

    fn reload_ghostty(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(env) = &self.config.env else { return };
        let loaded = ghostty::load(env);
        log(&loaded.warnings);
        self.config.ghostty = Watched::new(loaded.files);
        let style = Rc::new(TermStyle::new(&loaded.config, &installed_families(cx)));
        self.set_style(style, window, cx);
    }

    /// Every view, the Dock's too, gets the new style; the window's fill and blur follow the
    /// new opacity (plan A2).
    fn set_style(&mut self, style: Rc<TermStyle>, window: &mut Window, cx: &mut Context<Self>) {
        for view in self.views.values().chain(&self.dock) {
            view.update(cx, |v, cx| v.set_style(style.clone(), cx));
        }
        window.set_background_appearance(style.background_appearance());
        #[cfg(target_os = "linux")]
        {
            let want = style.cfg.background_blur && style.cfg.background_opacity < 1.0;
            if !want {
                self._blur = None;
            } else if self._blur.is_none() {
                self._blur = crate::blur::WindowBlur::enable(window, cx);
            }
        }
        self.style = style;
        cx.notify();
    }

    /// `ui_scale`, key bindings and `[[vault.agents]]` (plan A4).
    fn reload_toml(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.config.env.as_ref().map(Env::cmux_config_file) else {
            return;
        };
        self.config.toml = Watched::new(vec![path.clone()]);
        let new = cmux_config::ui::load(&path);
        let old = crate::ui::scale();
        if new != old {
            crate::ui::set_scale(new);
            // Dragged widths keep their look relative to the chrome around them.
            self.sidebar_width *= new / old;
            self.right_width *= new / old;
            cx.refresh_windows();
        }
        let keys = keybindings::load(&path);
        cx.clear_key_bindings();
        let bad = crate::actions::bind(cx, &keys);
        log(&keys.warnings);
        log(&bad);
        self.vault_agents = vault_ctl::load();
    }
}
