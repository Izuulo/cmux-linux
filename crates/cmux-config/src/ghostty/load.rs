//! Finding and layering Ghostty config files: `config.ghostty` (or legacy `config`), its
//! `config-file` includes, and the `theme` (Ghostty's `config/file_load.zig`, `config/theme.zig`).

use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{Config, entries};

/// Where to look. `from_system` reads the real environment; tests build their own.
#[derive(Debug, Clone)]
pub struct Env {
    /// `$XDG_CONFIG_HOME/ghostty`.
    pub config_dir: PathBuf,
    /// Ghostty resource dirs, each holding a `themes/` directory.
    pub resources_dirs: Vec<PathBuf>,
    /// Desktop appearance, for `theme = light:X,dark:Y`.
    pub dark: bool,
    /// `$HOME`, for `config-file = ~/...`. `None` when unset or empty.
    pub home: Option<PathBuf>,
}

impl Env {
    pub fn from_system(dark: bool) -> Option<Env> {
        let env = Env::from_vars(
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
            std::env::var_os("GHOSTTY_RESOURCES_DIR"),
        )?;
        Some(Env { dark, ..env })
    }

    /// `$XDG_CONFIG_HOME/cmux-linux/config.toml`: cmux-linux's own settings (key bindings).
    pub fn cmux_config_file(&self) -> PathBuf {
        let config_home = self.config_dir.parent().unwrap_or(&self.config_dir);
        config_home.join("cmux-linux").join("config.toml")
    }

    /// Takes the variables as values so tests can cover the rules without the process env.
    /// `None` when neither a usable `XDG_CONFIG_HOME` nor `HOME` exists.
    fn from_vars(
        xdg: Option<OsString>,
        home: Option<OsString>,
        resources: Option<OsString>,
    ) -> Option<Env> {
        let home = home.filter(|h| !h.is_empty()).map(PathBuf::from);
        // The XDG spec says a relative XDG_CONFIG_HOME is invalid, so it falls back to ~/.config.
        let config_home = xdg
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| home.as_ref().map(|h| h.join(".config")))?;
        let mut resources_dirs: Vec<PathBuf> = resources
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .into_iter()
            .collect();
        resources_dirs.push("/usr/share/ghostty".into());
        resources_dirs.push("/usr/local/share/ghostty".into());
        Some(Env {
            config_dir: config_home.join("ghostty"),
            resources_dirs,
            dark: false,
            home,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Loaded {
    pub config: Config,
    /// One line each, `path:line: key: problem`. Never fatal.
    pub warnings: Vec<String>,
}

struct Entry {
    origin: String,
    key: String,
    value: String,
}

/// Loads the theme, then the user's file and its includes in Ghostty's breadth-first
/// order (later wins).
pub fn load(env: &Env) -> Loaded {
    let mut warnings = Vec::new();
    let main = ["config.ghostty", "config"]
        .iter()
        .map(|name| env.config_dir.join(name))
        .find(|p| p.is_file());
    let mut user = Vec::new();
    if let Some(main) = main {
        read_with_includes(env, &main, &mut user, &mut warnings);
    }

    let mut config = Config::default();
    if let Some(theme) = user.iter().rev().find(|e| e.key == "theme") {
        match resolve_theme(env, &theme.value) {
            Ok(path) => {
                let mut entries = Vec::new();
                read_file(&path, &mut entries, &mut Vec::new(), &mut warnings);
                apply_all(&mut config, &entries, &mut warnings);
            }
            Err(e) => warnings.push(format!("{}: theme: {e}", theme.origin)),
        }
    }
    apply_all(&mut config, &user, &mut warnings);
    Loaded { config, warnings }
}

fn apply_all(config: &mut Config, entries: &[Entry], warnings: &mut Vec<String>) {
    for e in entries.iter().filter(|e| e.key != "theme") {
        if let Err(msg) = config.apply(&e.key, &e.value) {
            warnings.push(format!("{}: {}: {msg}", e.origin, e.key));
        }
    }
}

/// Reads one file's entries; `config-file` values are returned in `includes`, not as entries.
fn read_file(
    path: &Path,
    out: &mut Vec<Entry>,
    includes: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return warnings.push(format!("{}: {e}", path.display())),
    };
    for (line, key, value) in entries(&text) {
        if key == "config-file" {
            includes.push(value.to_string());
        } else {
            let origin = format!("{}:{line}", path.display());
            out.push(Entry {
                origin,
                key: key.to_string(),
                value: value.to_string(),
            });
        }
    }
}

/// Ghostty's `loadRecursiveFiles`: a FIFO queue, so the main file's includes load before
/// the includes they add. Each file is read once; a repeat is reported as a cycle.
fn read_with_includes(env: &Env, main: &Path, out: &mut Vec<Entry>, warnings: &mut Vec<String>) {
    let mut queue = VecDeque::from([main.to_path_buf()]);
    // Canonical paths read so far. Never popped, so cycle detection covers the whole load.
    let mut visited: Vec<PathBuf> = Vec::new();
    while let Some(path) = queue.pop_front() {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if visited.contains(&canonical) {
            warnings.push(format!("{}: config-file cycle, skipped", path.display()));
            continue;
        }
        visited.push(canonical);
        let mut includes = Vec::new();
        read_file(&path, out, &mut includes, warnings);
        let dir = path.parent().unwrap_or(Path::new("."));
        for include in includes {
            let (optional, name) = match include.strip_prefix('?') {
                Some(rest) => (true, rest),
                None => (false, include.as_str()),
            };
            if name.is_empty() {
                continue;
            }
            // Ghostty expands `~/` to $HOME (path.zig) rather than resolving it against the file.
            let target = match name.strip_prefix("~/") {
                Some(rest) => match &env.home {
                    Some(home) => home.join(rest),
                    None => {
                        warnings.push(format!(
                            "{}: config-file: cannot expand ~/ without a home directory: {name}",
                            path.display()
                        ));
                        continue;
                    }
                },
                None => dir.join(name),
            };
            if target.is_file() {
                queue.push_back(target);
            } else if !optional {
                warnings.push(format!("{}: config-file not found: {name}", path.display()));
            }
        }
    }
}

/// `name`, an absolute path, or `light:A,dark:B`. Names are looked up in the user's
/// `themes/` first, then each resources dir.
fn resolve_theme(env: &Env, value: &str) -> Result<PathBuf, String> {
    let name = pick_theme(value, env.dark)?;
    let path = Path::new(name);
    if path.is_absolute() {
        return if path.is_file() {
            Ok(path.into())
        } else {
            Err(format!("not found: {name}"))
        };
    }
    if name.contains('/') || name.contains('\\') {
        return Err(format!(
            "theme names cannot contain path separators: {name}"
        ));
    }
    std::iter::once(env.config_dir.join("themes"))
        .chain(env.resources_dirs.iter().map(|d| d.join("themes")))
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
        .ok_or_else(|| format!("not found: {name}"))
}

fn pick_theme(value: &str, dark: bool) -> Result<&str, String> {
    let parts: Vec<&str> = value.split(',').map(str::trim).collect();
    // Only a `light:`/`dark:` prefix marks a pair, so an absolute path like `/x/dark:y/T` is not one.
    if !parts
        .iter()
        .any(|p| p.starts_with("light:") || p.starts_with("dark:"))
    {
        return Ok(value);
    }
    let want = if dark { "dark:" } else { "light:" };
    parts
        .iter()
        .find_map(|part| part.strip_prefix(want))
        .map(str::trim)
        .ok_or_else(|| format!("missing {want} in {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ghostty::Rgb;

    /// Removes its directory on drop, so a failing test still cleans up.
    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for TempDir {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    /// A fresh `<tmp>/cmux-config-<name>-<pid>` with `ghostty/`, `res/themes/` and `home/` inside.
    fn fixture(name: &str) -> (TempDir, Env) {
        let root = std::env::temp_dir().join(format!("cmux-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("ghostty/themes")).unwrap();
        std::fs::create_dir_all(root.join("res/themes")).unwrap();
        std::fs::create_dir_all(root.join("home")).unwrap();
        let env = Env {
            config_dir: root.join("ghostty"),
            resources_dirs: vec![root.join("res")],
            dark: true,
            home: Some(root.join("home")),
        };
        (TempDir(root), env)
    }

    fn write(root: &Path, rel: &str, text: &str) {
        std::fs::write(root.join(rel), text).unwrap();
    }

    const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
    const WHITE: Rgb = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };
    const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };

    #[test]
    fn no_config_file_gives_defaults_and_no_warnings() {
        let (_root, env) = fixture("none");
        let loaded = load(&env);
        assert_eq!(loaded.config, Config::default());
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    #[test]
    fn config_ghostty_wins_over_legacy_config() {
        let (root, env) = fixture("prefer");
        write(&root, "ghostty/config.ghostty", "background = #000000\n");
        write(&root, "ghostty/config", "background = #ffffff\n");
        assert_eq!(load(&env).config.background, BLACK);
    }

    #[test]
    fn legacy_config_is_the_fallback() {
        let (root, env) = fixture("legacy");
        write(&root, "ghostty/config", "background = #ffffff\n");
        assert_eq!(load(&env).config.background, WHITE);
    }

    #[test]
    fn includes_load_after_the_including_file() {
        let (root, env) = fixture("include");
        write(
            &root,
            "ghostty/config.ghostty",
            "config-file = extra\nbackground = #000000\nconfig-file = ?missing\n",
        );
        write(&root, "ghostty/extra", "background = #ff0000\n");
        let loaded = load(&env);
        assert_eq!(loaded.config.background, RED);
        assert!(
            loaded.warnings.is_empty(),
            "optional include is silent: {:?}",
            loaded.warnings
        );
    }

    #[test]
    fn missing_required_include_and_cycles_warn() {
        let (root, env) = fixture("cycle");
        write(
            &root,
            "ghostty/config.ghostty",
            "config-file = a\nconfig-file = nope\n",
        );
        write(&root, "ghostty/a", "config-file = config.ghostty\n");
        let warnings = load(&env).warnings;
        assert!(warnings.iter().any(|w| w.contains("cycle")), "{warnings:?}");
        assert!(
            warnings.iter().any(|w| w.contains("not found: nope")),
            "{warnings:?}"
        );
    }

    #[test]
    fn theme_loads_first_and_user_values_override_it() {
        let (root, env) = fixture("theme");
        write(
            &root,
            "res/themes/Night",
            "palette = 1=#ff0000\nbackground = #ffffff\nforeground = #000000\n",
        );
        write(
            &root,
            "ghostty/config.ghostty",
            "background = #000000\ntheme = Night\n",
        );
        let cfg = load(&env).config;
        assert_eq!(cfg.palette[1], Some(RED));
        assert_eq!(cfg.foreground, BLACK, "from theme");
        assert_eq!(cfg.background, BLACK, "user overrides theme");
    }

    #[test]
    fn user_themes_dir_wins_over_resources() {
        let (root, env) = fixture("theme-user");
        write(&root, "res/themes/Night", "background = #ffffff\n");
        write(&root, "ghostty/themes/Night", "background = #ff0000\n");
        write(&root, "ghostty/config.ghostty", "theme = Night\n");
        assert_eq!(load(&env).config.background, RED);
    }

    #[test]
    fn light_dark_pair_follows_appearance() {
        let (root, mut env) = fixture("pair");
        write(&root, "res/themes/Day", "background = #ffffff\n");
        write(&root, "res/themes/Night", "background = #000000\n");
        write(
            &root,
            "ghostty/config.ghostty",
            "theme = light:Day,dark:Night\n",
        );
        assert_eq!(load(&env).config.background, BLACK);
        env.dark = false;
        assert_eq!(load(&env).config.background, WHITE);
    }

    #[test]
    fn missing_theme_and_bad_values_warn_with_location() {
        let (root, env) = fixture("bad");
        write(
            &root,
            "ghostty/config.ghostty",
            "theme = Nope\nfont-size = big\n",
        );
        let loaded = load(&env);
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.warnings.len(), 2, "{:?}", loaded.warnings);
        assert!(
            loaded.warnings[0].contains("config.ghostty:1: theme: not found: Nope"),
            "{:?}",
            loaded.warnings
        );
        assert!(
            loaded.warnings[1].contains("config.ghostty:2: font-size:"),
            "{:?}",
            loaded.warnings
        );
    }

    #[test]
    fn include_tilde_resolves_against_home() {
        let (root, env) = fixture("tilde");
        write(&root, "home/extra", "background = #ff0000\n");
        write(&root, "ghostty/config.ghostty", "config-file = ~/extra\n");
        let loaded = load(&env);
        assert_eq!(loaded.config.background, RED);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    #[test]
    fn include_tilde_without_home_warns() {
        let (root, mut env) = fixture("tilde-nohome");
        env.home = None;
        write(&root, "ghostty/config.ghostty", "config-file = ~/extra\n");
        let warnings = load(&env).warnings;
        assert!(
            warnings.iter().any(|w| w.contains("cannot expand ~/")),
            "{warnings:?}"
        );
    }

    #[test]
    fn includes_load_breadth_first() {
        // Queue order is main, B, C, D, so D (queued by B) comes after C and wins.
        let (root, env) = fixture("bfs");
        write(
            &root,
            "ghostty/config.ghostty",
            "config-file = b\nconfig-file = c\n",
        );
        write(&root, "ghostty/b", "config-file = d\n");
        write(&root, "ghostty/c", "background = #ffffff\n");
        write(&root, "ghostty/d", "background = #ff0000\n");
        assert_eq!(load(&env).config.background, RED);
    }

    #[test]
    fn empty_config_file_value_is_skipped_silently() {
        let (root, env) = fixture("empty-include");
        write(
            &root,
            "ghostty/config.ghostty",
            "config-file =\nconfig-file = ?\nbackground = #ff0000\n",
        );
        let loaded = load(&env);
        assert_eq!(loaded.config.background, RED);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    #[test]
    fn absolute_theme_path_with_dark_colon_is_not_a_pair() {
        let (root, env) = fixture("abs-dark");
        std::fs::create_dir_all(root.join("res/dark:set")).unwrap();
        write(&root, "res/dark:set/Night", "background = #ff0000\n");
        let abs = root.join("res/dark:set/Night");
        write(
            &root,
            "ghostty/config.ghostty",
            &format!("theme = {}\n", abs.display()),
        );
        let loaded = load(&env);
        assert_eq!(loaded.config.background, RED);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    #[test]
    fn from_vars_uses_an_absolute_xdg_config_home() {
        let env = Env::from_vars(Some("/x".into()), Some("/h".into()), None).unwrap();
        assert_eq!(env.config_dir, PathBuf::from("/x/ghostty"));
        assert_eq!(env.home, Some(PathBuf::from("/h")));
        assert_eq!(
            env.cmux_config_file(),
            PathBuf::from("/x/cmux-linux/config.toml")
        );
    }

    #[test]
    fn from_vars_ignores_relative_or_empty_xdg_config_home() {
        for xdg in ["relative/dir", ""] {
            let env = Env::from_vars(Some(xdg.into()), Some("/h".into()), None).unwrap();
            assert_eq!(
                env.config_dir,
                PathBuf::from("/h/.config/ghostty"),
                "xdg={xdg:?}"
            );
        }
    }

    #[test]
    fn from_vars_ignores_empty_home() {
        let env = Env::from_vars(Some("/x".into()), Some("".into()), None).unwrap();
        assert_eq!(env.home, None);
        assert!(Env::from_vars(None, Some("".into()), None).is_none());
        assert!(Env::from_vars(Some("rel".into()), Some("".into()), None).is_none());
    }

    #[test]
    fn from_vars_ignores_empty_resources_dir() {
        let env = Env::from_vars(Some("/x".into()), Some("/h".into()), Some("".into())).unwrap();
        assert_eq!(
            env.resources_dirs,
            vec![
                PathBuf::from("/usr/share/ghostty"),
                PathBuf::from("/usr/local/share/ghostty")
            ]
        );
    }

    #[test]
    fn from_vars_keeps_a_set_resources_dir_first() {
        let env = Env::from_vars(Some("/x".into()), Some("/h".into()), Some("/r".into())).unwrap();
        assert_eq!(env.resources_dirs[0], PathBuf::from("/r"));
    }
}
