//! Writing settings into the user's own Ghostty file (M12 plan W1, W3): each key's last line
//! is rewritten in place, new keys are appended under a `# cmux-linux` line, and every other
//! byte is kept.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::Env;

const MARKER: &str = "# cmux-linux";

/// The file the loader reads first: `config.ghostty`, else `config`, else a new `config.ghostty`.
pub fn main_file(env: &Env) -> PathBuf {
    ["config.ghostty", "config"]
        .iter()
        .map(|name| env.config_dir.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| env.config_dir.join("config.ghostty"))
}

/// `theme`'s value: `light:X,dark:Y` when the slots differ, else `X`.
pub fn theme_value(light: &str, dark: &str) -> String {
    if light == dark {
        light.to_string()
    } else {
        format!("light:{light},dark:{dark}")
    }
}

/// Sets one key; see [`set_all`].
pub fn set(path: &Path, key: &str, value: &str) -> io::Result<()> {
    set_all(path, &[(key, value)])
}

/// Applies `changes` to the file at `path` (created if missing). The first write to a file in
/// this process keeps the old content as `<file>.cmux-bak`.
pub fn set_all(path: &Path, changes: &[(&str, &str)]) -> io::Result<()> {
    for &(key, value) in changes {
        let bad_key = key.is_empty()
            || key == "config-file"
            || key.contains(['=', '#', '\n', '\r'])
            || key.trim() != key;
        if bad_key || value.contains(['\n', '\r']) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("cannot write Ghostty setting {key:?} = {value:?}"),
            ));
        }
    }
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let new = edit(&text, changes);
    if new == text {
        return Ok(());
    }
    static BACKED_UP: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    let mut backed_up = BACKED_UP.lock().unwrap_or_else(|e| e.into_inner());
    if !backed_up.iter().any(|p| p == path) {
        if path.is_file() {
            let mut bak = path.as_os_str().to_owned();
            bak.push(".cmux-bak");
            std::fs::copy(path, PathBuf::from(bak))?;
        }
        backed_up.push(path.to_path_buf());
    }
    crate::write::replace_file(path, &new)
}

/// The key a line sets, as the loader's `entries` reads it.
fn key_of(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') {
        return None;
    }
    Some(t.split_once('=').map_or(t, |(k, _)| k).trim())
}

/// Pure edit behind [`set_all`]. Earlier `font-family` lines are commented out so the
/// font list is the one written; other repeated keys keep their earlier lines.
pub fn edit(text: &str, changes: &[(&str, &str)]) -> String {
    let (bom, body) = match text.strip_prefix('\u{feff}') {
        Some(rest) => ("\u{feff}", rest),
        None => ("", text),
    };
    let eol = if body.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = body.split_inclusive('\n').map(str::to_string).collect();
    for &(key, value) in changes {
        let hits: Vec<usize> = (0..lines.len())
            .filter(|&i| key_of(&lines[i]) == Some(key))
            .collect();
        match hits.split_last() {
            Some((&last, earlier)) => {
                let line = &lines[last];
                let ending = &line[line.trim_end_matches(['\r', '\n']).len()..];
                lines[last] = format!("{key} = {value}{ending}");
                if key == "font-family" {
                    for &i in earlier {
                        lines[i].insert_str(0, "# ");
                    }
                }
            }
            None => {
                if let Some(l) = lines.last_mut()
                    && !l.ends_with('\n')
                {
                    l.push_str(eol);
                }
                if !lines.iter().any(|l| l.trim() == MARKER) {
                    lines.push(format!("{MARKER}{eol}"));
                }
                lines.push(format!("{key} = {value}{eol}"));
            }
        }
    }
    format!("{bom}{}", lines.concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\u{feff}# my config\r\n\
        font-family = Iosevka\r\n\
        \r\n\
        config-file = ?themes.conf\r\n\
        some-unknown-key = 7\r\n\
        font-size = 11\r\n\
        font-family = \"Noto Color Emoji\"\r\n\
        keybind = ctrl+a=copy\r\n\
        font-size=13\r\n\
        # font-size = 99\r\n\
        keybind = ctrl+b=paste";

    #[test]
    fn rewrites_last_line_and_keeps_everything_else() {
        let out = edit(FIXTURE, &[("font-size", "14")]);
        assert_eq!(out, FIXTURE.replace("font-size=13", "font-size = 14"));
    }

    #[test]
    fn font_family_comments_out_earlier_lines() {
        let out = edit(FIXTURE, &[("font-family", "JetBrains Mono")]);
        let want = FIXTURE
            .replace("font-family = Iosevka", "# font-family = Iosevka")
            .replace(
                "font-family = \"Noto Color Emoji\"",
                "font-family = JetBrains Mono",
            );
        assert_eq!(out, want);
        let families: Vec<_> = crate::ghostty::entries(&out)
            .filter(|(_, k, _)| *k == "font-family")
            .map(|(_, _, v)| v)
            .collect();
        assert_eq!(families, ["JetBrains Mono"]);
    }

    #[test]
    fn new_keys_append_under_one_marker_with_file_line_endings() {
        let out = edit(FIXTURE, &[("theme", "light:A,dark:B")]);
        assert_eq!(
            out,
            format!("{FIXTURE}\r\n# cmux-linux\r\ntheme = light:A,dark:B\r\n")
        );
        let again = edit(&out, &[("background-blur", "true"), ("theme", "C")]);
        assert_eq!(
            again,
            format!("{FIXTURE}\r\n# cmux-linux\r\ntheme = C\r\nbackground-blur = true\r\n")
        );
    }

    #[test]
    fn empty_file_gets_marker_and_key() {
        assert_eq!(
            edit("", &[("font-size", "12")]),
            "# cmux-linux\nfont-size = 12\n"
        );
    }

    #[test]
    fn unchanged_keys_and_no_changes_are_identity() {
        assert_eq!(edit(FIXTURE, &[]), FIXTURE);
        assert_eq!(
            edit(FIXTURE, &[("font-size", "13")]).len(),
            FIXTURE.len() + 2
        );
    }

    #[test]
    fn theme_value_pairs_only_when_slots_differ() {
        assert_eq!(theme_value("A", "A"), "A");
        assert_eq!(theme_value("A", "B"), "light:A,dark:B");
    }

    #[test]
    fn set_all_writes_backs_up_once_and_rejects_bad_input() {
        let dir = std::env::temp_dir().join(format!("cmux-ghostty-edit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let env = Env {
            config_dir: dir.clone(),
            resources_dirs: vec![],
            dark: false,
            home: None,
        };
        let path = main_file(&env);
        assert_eq!(path, dir.join("config.ghostty"));
        std::fs::write(dir.join("config"), "font-size = 10\n").unwrap();
        let path = main_file(&env);
        assert_eq!(path, dir.join("config"));

        set(&path, "font-size", "12").unwrap();
        set(&path, "font-size", "13").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "font-size = 13\n");
        let bak = std::fs::read_to_string(dir.join("config.cmux-bak")).unwrap();
        assert_eq!(bak, "font-size = 10\n", "backup is the pre-session file");

        for (k, v) in [
            ("config-file", "x"),
            ("a=b", "1"),
            ("k", "1\nx = 2"),
            ("", "1"),
        ] {
            assert_eq!(
                set(&path, k, v).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
