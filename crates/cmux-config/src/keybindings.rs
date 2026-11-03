//! Key bindings: cmux's default shortcut table with ⌘ as Super, overridable from the
//! `[keybindings]` table of `~/.config/cmux-linux/config.toml`.
//!
//! Keys are cmux action names (`KeyboardShortcutSettings.swift` `Action` rawValues), values a
//! chord or a list of chords in cmux's syntax (`KeyboardShortcutSettings.swift`
//! `parseConfig`): modifiers `cmd`/`command`/`super`, `shift`, `opt`/`option`/`alt`,
//! `ctrl`/`control`, then one key, joined by `+`. `""` or `"none"` unbinds. Output is gpui
//! keystroke syntax (`super-shift-d`); the app validates it with gpui's own parser.

use std::path::Path;

/// One gpui binding: `keystroke` runs `action`; `number` is the digit for
/// `selectSurfaceByNumber`, whose chord binds the same modifiers with each of 1-9.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub action: &'static str,
    pub keystroke: String,
    pub number: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Keybindings {
    pub bindings: Vec<Binding>,
    /// One line each, `path: problem`. Never fatal.
    pub warnings: Vec<String>,
}

/// Every action M2 handles with cmux's default chords (research-cmux.md §3; Cmd → super).
/// Ctrl+Tab / Ctrl+Shift+Tab are cmux's hard-wired legacy tab keys; here they are ordinary
/// remappable defaults. Copy and paste are the M1 bindings.
pub const DEFAULTS: &[(&str, &[&str])] = &[
    ("copy", &["super+c", "ctrl+shift+c"]),
    ("paste", &["super+v", "ctrl+shift+v"]),
    ("splitRight", &["super+d"]),
    ("splitDown", &["super+shift+d"]),
    ("toggleSplitZoom", &["super+shift+return"]),
    ("equalizeSplits", &["super+ctrl+shift+="]),
    ("resizePaneLeft", &["ctrl+shift+h"]),
    ("resizePaneRight", &["ctrl+shift+l"]),
    ("resizePaneUp", &["ctrl+shift+k"]),
    ("resizePaneDown", &["ctrl+shift+j"]),
    ("focusLeft", &["super+opt+left"]),
    ("focusRight", &["super+opt+right"]),
    ("focusUp", &["super+opt+up"]),
    ("focusDown", &["super+opt+down"]),
    ("focusPreviousPane", &[]),
    ("focusNextPane", &[]),
    ("newSurface", &["super+t"]),
    ("closeTab", &["super+w"]),
    ("closeOtherTabsInPane", &["super+opt+t"]),
    ("nextSurface", &["super+shift+]", "ctrl+tab"]),
    ("prevSurface", &["super+shift+[", "ctrl+shift+tab"]),
    ("selectSurfaceByNumber", &["ctrl+1"]),
    ("moveSurfaceLeft", &["super+shift+opt+["]),
    ("moveSurfaceRight", &["super+shift+opt+]"]),
];

const NUMBERED: &str = "selectSurfaceByNumber";

/// What a US-layout Shift turns a key into. gpui on Linux reports Shift+] as key `}` with
/// Shift cleared (platform/linux/platform.rs `from_xkb`), so the binding must say `}` too.
// shortcut: US layout only; other layouts need the real keymap. Revisit with M6 layout work.
fn shifted(key: &str) -> Option<&'static str> {
    Some(match key {
        "1" => "!",
        "2" => "@",
        "3" => "#",
        "4" => "$",
        "5" => "%",
        "6" => "^",
        "7" => "&",
        "8" => "*",
        "9" => "(",
        "0" => ")",
        "-" => "_",
        "=" => "+",
        "[" => "{",
        "]" => "}",
        "\\" => "|",
        ";" => ":",
        "'" => "\"",
        "," => "<",
        "." => ">",
        "/" => "?",
        "`" => "~",
        _ => return None,
    })
}

/// cmux's key tokens (`parseConfigKeyToken`) in gpui's key names.
fn key_name(token: &str) -> Option<String> {
    let t = token.trim().to_lowercase();
    let named = match t.as_str() {
        "left" | "arrowleft" | "leftarrow" | "←" => "left",
        "right" | "arrowright" | "rightarrow" | "→" => "right",
        "up" | "arrowup" | "uparrow" | "↑" => "up",
        "down" | "arrowdown" | "downarrow" | "↓" => "down",
        "tab" => "tab",
        "return" | "enter" | "↩" => "enter",
        "space" | "spacebar" | "<space>" => "space",
        "escape" | "esc" => "escape",
        "backspace" => "backspace",
        "delete" => "delete",
        "home" => "home",
        "end" => "end",
        "pageup" => "pageup",
        "pagedown" => "pagedown",
        "comma" => ",",
        "period" | "dot" => ".",
        "slash" => "/",
        "backslash" => "\\",
        "semicolon" => ";",
        "quote" | "apostrophe" => "'",
        "backtick" | "grave" => "`",
        "minus" | "hyphen" => "-",
        "plus" | "equals" => "=",
        "leftbracket" | "openbracket" => "[",
        "rightbracket" | "closebracket" => "]",
        f if f.len() > 1
            && f.starts_with('f')
            && f[1..].parse::<u8>().is_ok_and(|n| (1..=20).contains(&n)) =>
        {
            return Some(f.to_string());
        }
        c if c.chars().count() == 1 && !c.chars().any(char::is_whitespace) => {
            return Some(c.to_string());
        }
        _ => return None,
    };
    Some(named.to_string())
}

/// One cmux chord (`super+shift+]`) as a gpui keystroke (`super-}`).
pub fn chord_to_gpui(chord: &str) -> Result<String, String> {
    let parts: Vec<&str> = chord.split('+').collect();
    let (key, mods) = parts.split_last().expect("split yields one part");
    let (mut ctrl, mut alt, mut shift, mut sup) = (false, false, false, false);
    for m in mods {
        match m.trim().to_lowercase().as_str() {
            "cmd" | "command" | "super" | "⌘" => sup = true,
            "shift" | "⇧" => shift = true,
            "opt" | "option" | "alt" | "⌥" => alt = true,
            "ctrl" | "control" | "ctl" | "⌃" => ctrl = true,
            other => return Err(format!("unknown modifier `{other}` in `{chord}`")),
        }
    }
    let mut key = key_name(key).ok_or_else(|| format!("unknown key in `{chord}`"))?;
    if shift && let Some(s) = shifted(&key) {
        key = s.to_string();
        shift = false;
    } else if shift && key.chars().count() == 1 && "!@#$%^&*()_+{}|:\"<>?~".contains(key.as_str()) {
        shift = false; // already the shifted symbol, which gpui reports without Shift
    }
    let mut out = String::new();
    for (on, name) in [
        (ctrl, "ctrl-"),
        (alt, "alt-"),
        (shift, "shift-"),
        (sup, "super-"),
    ] {
        if on {
            out.push_str(name);
        }
    }
    out.push_str(&key);
    Ok(out)
}

/// The chords for one action as gpui bindings. `selectSurfaceByNumber` chords must end in a
/// digit; their modifiers are bound with each of 1-9.
fn expand(action: &'static str, chords: &[String]) -> Result<Vec<Binding>, String> {
    let mut out = Vec::new();
    for chord in chords {
        let chord = chord.trim();
        if action == NUMBERED {
            let last = chord.rsplit('+').next().unwrap_or_default();
            if last.len() != 1 || !last.chars().all(|c| c.is_ascii_digit()) {
                return Err(format!("`{chord}` must end in a digit (it binds 1-9)"));
            }
            let mods = &chord[..chord.len() - last.len()];
            for n in 1..=9u8 {
                out.push(Binding {
                    action,
                    keystroke: chord_to_gpui(&format!("{mods}{n}"))?,
                    number: Some(n),
                });
            }
        } else {
            out.push(Binding {
                action,
                keystroke: chord_to_gpui(chord)?,
                number: None,
            });
        }
    }
    Ok(out)
}

/// A TOML value as a chord list: a string, or an array of strings. `""` / `"none"` = none.
fn chords(value: &toml::Value) -> Result<Vec<String>, String> {
    let list = match value {
        toml::Value::String(s) => vec![s.clone()],
        toml::Value::Array(items) => items
            .iter()
            .map(|v| v.as_str().map(str::to_string).ok_or("expected a string"))
            .collect::<Result<_, _>>()?,
        _ => return Err("expected a chord string or a list of them".into()),
    };
    Ok(list
        .into_iter()
        .filter(|c| !c.trim().is_empty() && !c.trim().eq_ignore_ascii_case("none"))
        .collect())
}

/// Defaults, with each action named in `user`'s `[keybindings]` replaced by the user's chords.
/// `origin` prefixes warnings. A bad entry is reported and its action keeps its defaults.
pub fn keybindings(user: Option<&str>, origin: &str) -> Keybindings {
    let mut warnings = Vec::new();
    let mut table = toml::Table::new();
    if let Some(text) = user {
        match text.parse::<toml::Table>() {
            Ok(t) => table = t,
            Err(e) => warnings.push(format!("{origin}: {e}")),
        }
    }
    let user = match table.get("keybindings") {
        None => toml::Table::new(),
        Some(toml::Value::Table(t)) => t.clone(),
        Some(_) => {
            warnings.push(format!("{origin}: keybindings: expected a table"));
            toml::Table::new()
        }
    };
    for name in user.keys() {
        if !DEFAULTS.iter().any(|(a, _)| a == name) {
            warnings.push(format!("{origin}: keybindings: unknown action `{name}`"));
        }
    }
    let mut bindings = Vec::new();
    for (action, defaults) in DEFAULTS {
        let defaults: Vec<String> = defaults.iter().map(|c| c.to_string()).collect();
        let expanded = match user.get(*action).map(chords) {
            None => expand(action, &defaults),
            Some(Ok(list)) => expand(action, &list).or_else(|e| {
                warnings.push(format!("{origin}: keybindings.{action}: {e}"));
                expand(action, &defaults)
            }),
            Some(Err(e)) => {
                warnings.push(format!("{origin}: keybindings.{action}: {e}"));
                expand(action, &defaults)
            }
        };
        bindings.extend(expanded.expect("the default table is valid"));
    }
    Keybindings { bindings, warnings }
}

/// Reads `path` (a missing file is fine) and builds the bindings.
pub fn load(path: &Path) -> Keybindings {
    let origin = path.display().to_string();
    match std::fs::read_to_string(path) {
        Ok(text) => keybindings(Some(&text), &origin),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => keybindings(None, &origin),
        Err(e) => {
            let mut k = keybindings(None, &origin);
            k.warnings.push(format!("{origin}: {e}"));
            k
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys_for(k: &Keybindings, action: &str) -> Vec<String> {
        k.bindings
            .iter()
            .filter(|b| b.action == action)
            .map(|b| b.keystroke.clone())
            .collect()
    }

    #[test]
    fn chords_translate_to_gpui_keystrokes() {
        assert_eq!(chord_to_gpui("super+d").unwrap(), "super-d");
        assert_eq!(chord_to_gpui("cmd+shift+d").unwrap(), "shift-super-d");
        assert_eq!(
            chord_to_gpui("super+shift+return").unwrap(),
            "shift-super-enter"
        );
        assert_eq!(chord_to_gpui("super+opt+left").unwrap(), "alt-super-left");
        assert_eq!(chord_to_gpui("ctrl+shift+tab").unwrap(), "ctrl-shift-tab");
        assert_eq!(chord_to_gpui("Ctrl+Shift+H").unwrap(), "ctrl-shift-h");
        assert_eq!(chord_to_gpui("ctrl+F5").unwrap(), "ctrl-f5");
        assert_eq!(chord_to_gpui("super+comma").unwrap(), "super-,");
    }

    #[test]
    fn shifted_symbols_become_the_shifted_key() {
        assert_eq!(chord_to_gpui("super+shift+]").unwrap(), "super-}");
        assert_eq!(chord_to_gpui("super+shift+opt+[").unwrap(), "alt-super-{");
        assert_eq!(chord_to_gpui("super+ctrl+shift+=").unwrap(), "ctrl-super-+");
        assert_eq!(chord_to_gpui("shift+plus").unwrap(), "+");
    }

    #[test]
    fn bad_chords_are_errors() {
        assert!(chord_to_gpui("hyper+d").is_err());
        assert!(chord_to_gpui("super+").is_err());
        assert!(chord_to_gpui("super+notakey").is_err());
    }

    #[test]
    fn defaults_cover_cmux_table() {
        let k = keybindings(None, "test");
        assert_eq!(k.warnings, Vec::<String>::new());
        assert_eq!(keys_for(&k, "splitRight"), vec!["super-d"]);
        assert_eq!(keys_for(&k, "nextSurface"), vec!["super-}", "ctrl-tab"]);
        assert_eq!(keys_for(&k, "resizePaneDown"), vec!["ctrl-shift-j"]);
        assert!(keys_for(&k, "focusNextPane").is_empty());
        let numbered: Vec<_> = k
            .bindings
            .iter()
            .filter(|b| b.action == "selectSurfaceByNumber")
            .map(|b| (b.keystroke.as_str(), b.number))
            .collect();
        assert_eq!(numbered.len(), 9);
        assert_eq!(numbered[0], ("ctrl-1", Some(1)));
        assert_eq!(numbered[8], ("ctrl-9", Some(9)));
    }

    #[test]
    fn user_entries_replace_or_unbind_an_action() {
        let k = keybindings(
            Some(
                "[keybindings]\nsplitRight = \"ctrl+alt+d\"\nsplitDown = [\"super+shift+d\", \"ctrl+alt+shift+d\"]\ncloseTab = \"none\"\nnewSurface = \"\"\nselectSurfaceByNumber = \"alt+1\"\n",
            ),
            "test",
        );
        assert_eq!(k.warnings, Vec::<String>::new());
        assert_eq!(keys_for(&k, "splitRight"), vec!["ctrl-alt-d"]);
        assert_eq!(
            keys_for(&k, "splitDown"),
            vec!["shift-super-d", "ctrl-alt-shift-d"]
        );
        assert!(keys_for(&k, "closeTab").is_empty());
        assert!(keys_for(&k, "newSurface").is_empty());
        assert_eq!(keys_for(&k, "selectSurfaceByNumber")[4], "alt-5");
    }

    #[test]
    fn bad_entries_warn_and_keep_defaults() {
        let k = keybindings(
            Some(
                "[keybindings]\nsplitRight = \"hyper+d\"\nnoSuchAction = \"super+x\"\nsplitDown = 3\nselectSurfaceByNumber = \"ctrl+x\"\n",
            ),
            "cfg",
        );
        assert_eq!(k.warnings.len(), 4, "{:?}", k.warnings);
        assert!(k.warnings.iter().any(|w| w.contains("noSuchAction")));
        assert_eq!(keys_for(&k, "splitRight"), vec!["super-d"]);
        assert_eq!(keys_for(&k, "splitDown"), vec!["shift-super-d"]);
        assert_eq!(keys_for(&k, "selectSurfaceByNumber")[0], "ctrl-1");
    }

    #[test]
    fn malformed_toml_warns_and_uses_defaults() {
        let k = keybindings(Some("[keybindings\nsplitRight = "), "cfg");
        assert_eq!(k.warnings.len(), 1, "{:?}", k.warnings);
        assert!(k.warnings[0].starts_with("cfg: "));
        assert_eq!(keys_for(&k, "splitRight"), vec!["super-d"]);
    }

    #[test]
    fn a_missing_file_is_not_a_warning() {
        let k = load(Path::new("/nonexistent/cmux-linux/config.toml"));
        assert_eq!(k.warnings, Vec::<String>::new());
        assert!(!k.bindings.is_empty());
    }

    #[test]
    fn already_shifted_symbols_drop_shift() {
        assert_eq!(chord_to_gpui("super+shift+}").unwrap(), "super-}");
    }

    #[test]
    fn numbered_chords_are_trimmed_and_take_one_digit() {
        let ok = expand(NUMBERED, &["ctrl+1\u{a0}".to_string()]).unwrap();
        assert_eq!(ok[8].keystroke, "ctrl-9");
        assert!(expand(NUMBERED, &["ctrl+12".to_string()]).is_err());
    }
    #[test]
    fn workspace_defaults_bind_like_cmux() {
        let k = keybindings(None, "config.toml");
        let chord = |a: &str| {
            k.bindings
                .iter()
                .filter(|b| b.action == a)
                .map(|b| b.keystroke.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(chord("newTab"), vec!["super-n"]);
        assert_eq!(chord("closeWorkspace"), vec!["shift-super-w"]);
        assert_eq!(chord("nextSidebarTab"), vec!["ctrl-super-]"]);
        assert_eq!(chord("moveWorkspaceDown"), vec!["ctrl-alt-super-]"]);
        assert_eq!(chord("toggleSidebar"), vec!["super-b"]);
        let numbered: Vec<_> = k
            .bindings
            .iter()
            .filter(|b| b.action == "selectWorkspaceByNumber")
            .map(|b| (b.keystroke.as_str(), b.number))
            .collect();
        assert_eq!(numbered.len(), 9);
        assert_eq!(numbered[0], ("super-1", Some(1)));
        assert_eq!(numbered[8], ("super-9", Some(9)));
    }

    #[test]
    fn workspace_number_remap_keeps_digit_family() {
        let k = keybindings(
            Some("[keybindings]\nselectWorkspaceByNumber = \"alt+1\"\n"),
            "config.toml",
        );
        assert!(k.warnings.is_empty(), "{:?}", k.warnings);
        assert!(k.bindings.iter().any(|b| b.action == "selectWorkspaceByNumber"
            && b.keystroke == "alt-9"
            && b.number == Some(9)));
    }
}
