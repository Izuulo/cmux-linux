//! Finding and layering Ghostty config files: `config.ghostty` (or legacy `config`), its
//! `config-file` includes, and the `theme` (Ghostty's `config/file_load.zig`, `config/theme.zig`).

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
}

impl Env {
    pub fn from_system(dark: bool) -> Option<Env> {
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        let mut resources_dirs: Vec<PathBuf> = std::env::var_os("GHOSTTY_RESOURCES_DIR")
            .map(PathBuf::from)
            .into_iter()
            .collect();
        resources_dirs.push("/usr/share/ghostty".into());
        resources_dirs.push("/usr/local/share/ghostty".into());
        Some(Env {
            config_dir: config_home.join("ghostty"),
            resources_dirs,
            dark,
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

/// Loads theme, then the user's file, then its includes (later wins), as Ghostty does.
pub fn load(env: &Env) -> Loaded {
    let mut warnings = Vec::new();
    let main = ["config.ghostty", "config"]
        .iter()
        .map(|name| env.config_dir.join(name))
        .find(|p| p.is_file());
    let mut user = Vec::new();
    if let Some(main) = main {
        let mut visited = Vec::new();
        read_with_includes(&main, &mut visited, &mut user, &mut warnings);
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

fn read_with_includes(
    path: &Path,
    visited: &mut Vec<PathBuf>,
    out: &mut Vec<Entry>,
    warnings: &mut Vec<String>,
) {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if visited.contains(&canonical) {
        return warnings.push(format!("{}: config-file cycle, skipped", path.display()));
    }
    visited.push(canonical);
    let mut includes = Vec::new();
    read_file(path, out, &mut includes, warnings);
    let dir = path.parent().unwrap_or(Path::new("."));
    for include in includes {
        let (optional, name) = match include.strip_prefix('?') {
            Some(rest) => (true, rest),
            None => (false, include.as_str()),
        };
        let target = dir.join(name);
        if target.is_file() {
            read_with_includes(&target, visited, out, warnings);
        } else if !optional {
            warnings.push(format!("{}: config-file not found: {name}", path.display()));
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
    if !value.contains("light:") && !value.contains("dark:") {
        return Ok(value);
    }
    let want = if dark { "dark:" } else { "light:" };
    value
        .split(',')
        .find_map(|part| part.trim().strip_prefix(want))
        .map(str::trim)
        .ok_or_else(|| format!("missing {want} in {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ghostty::Rgb;

    /// A fresh `<tmp>/cmux-config-<name>-<pid>` with `ghostty/` and `res/themes/` inside.
    fn fixture(name: &str) -> (PathBuf, Env) {
        let root = std::env::temp_dir().join(format!("cmux-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("ghostty/themes")).unwrap();
        std::fs::create_dir_all(root.join("res/themes")).unwrap();
        let env = Env {
            config_dir: root.join("ghostty"),
            resources_dirs: vec![root.join("res")],
            dark: true,
        };
        (root, env)
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
}
