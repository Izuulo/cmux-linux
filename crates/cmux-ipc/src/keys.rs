//! `surface.send_key` key names and the bytes of `send`/`paste` (rc §8).

/// A key press: a gpui key name plus modifiers (`platform` = Super/Cmd).
#[derive(Debug, Clone, PartialEq)]
pub struct KeySpec {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub platform: bool,
}

fn spec(key: &str, ctrl: bool, shift: bool) -> KeySpec {
    KeySpec {
        key: key.into(),
        ctrl,
        shift,
        alt: false,
        platform: false,
    }
}

/// cmux's key names (`TerminalSurface+Input.swift:845-949`), case-insensitive.
pub fn parse_key(name: &str) -> Option<KeySpec> {
    let name = name.trim().to_lowercase();
    let named = match name.as_str() {
        "ctrl-c" | "ctrl+c" | "sigint" => spec("c", true, false),
        "ctrl-d" | "ctrl+d" | "eof" => spec("d", true, false),
        "ctrl-f" | "ctrl+f" => spec("f", true, false),
        "ctrl-z" | "ctrl+z" | "sigtstp" => spec("z", true, false),
        "ctrl-\\" | "ctrl+\\" | "sigquit" => spec("\\", true, false),
        "shift+tab" | "shift-tab" | "backtab" => spec("tab", false, true),
        "pageup" | "page_up" => spec("pageup", false, false),
        "pagedown" | "page_down" => spec("pagedown", false, false),
        "delete" | "del" | "forward_delete" => spec("delete", false, false),
        "home" | "end" => spec(&name, false, false),
        _ => {
            for dir in ["up", "down", "left", "right"] {
                if [format!("arrow_{dir}"), format!("arrow{dir}")].contains(&name) {
                    return Some(spec(dir, false, false));
                }
            }
            return chord(&name);
        }
    };
    Some(named)
}

/// `[mod(+|-)]*base`: modifiers ctrl/control, shift, alt/opt/option, cmd/command/super; base
/// enter return tab escape esc backspace delete space up down left right `\` or a–z.
fn chord(name: &str) -> Option<KeySpec> {
    let parts: Vec<&str> = name.split(['+', '-']).collect();
    let (base, mods) = parts.split_last()?;
    let mut k = spec("", false, false);
    for m in mods {
        match *m {
            "ctrl" | "control" => k.ctrl = true,
            "shift" => k.shift = true,
            "alt" | "opt" | "option" => k.alt = true,
            "cmd" | "command" | "super" => k.platform = true,
            _ => return None,
        }
    }
    k.key = match *base {
        "return" | "enter" => "enter".into(),
        "esc" | "escape" => "escape".into(),
        "tab" | "backspace" | "delete" | "space" | "up" | "down" | "left" | "right" | "\\" => {
            (*base).into()
        }
        b if b.len() == 1 && b.as_bytes()[0].is_ascii_lowercase() => b.into(),
        _ => return None,
    };
    Some(k)
}

/// Text typed as keystrokes: every newline (`\n`, `\r\n`) presses Enter (`\r`).
pub fn typed_bytes(text: &str) -> Vec<u8> {
    text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
}

/// cmux's paste path: control characters other than tab and newlines become spaces.
pub fn sanitize_paste(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\t' | '\n' | '\r' => c,
            c if c.is_control() => ' ',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(key: &str, ctrl: bool, shift: bool, alt: bool, platform: bool) -> Option<KeySpec> {
        Some(KeySpec {
            key: key.into(),
            ctrl,
            shift,
            alt,
            platform,
        })
    }

    fn plain(key: &str) -> Option<KeySpec> {
        k(key, false, false, false, false)
    }

    fn ctrl(key: &str) -> Option<KeySpec> {
        k(key, true, false, false, false)
    }

    #[test]
    fn named_keys() {
        for (name, want) in [
            ("ctrl-c", ctrl("c")),
            ("CTRL+C", ctrl("c")),
            ("SIGINT", ctrl("c")),
            ("eof", ctrl("d")),
            ("ctrl-f", ctrl("f")),
            ("sigtstp", ctrl("z")),
            ("ctrl-\\", ctrl("\\")),
            ("sigquit", ctrl("\\")),
            ("return", plain("enter")),
            ("Enter", plain("enter")),
            ("esc", plain("escape")),
            ("tab", plain("tab")),
            ("backspace", plain("backspace")),
            ("arrow_up", plain("up")),
            ("arrowleft", plain("left")),
            ("down", plain("down")),
            ("page_down", plain("pagedown")),
            ("pageup", plain("pageup")),
            ("home", plain("home")),
            ("end", plain("end")),
            ("forward_delete", plain("delete")),
            ("del", plain("delete")),
            ("backtab", k("tab", false, true, false, false)),
            ("shift-tab", k("tab", false, true, false, false)),
        ] {
            assert_eq!(parse_key(name), want, "{name}");
        }
    }

    #[test]
    fn chords() {
        assert_eq!(parse_key("ctrl+l"), ctrl("l"));
        assert_eq!(parse_key("alt+left"), k("left", false, false, true, false));
        assert_eq!(parse_key("opt-b"), k("b", false, false, true, false));
        assert_eq!(parse_key("cmd+k"), k("k", false, false, false, true));
        assert_eq!(
            parse_key("ctrl+shift+space"),
            k("space", true, true, false, false)
        );
        assert_eq!(parse_key("control-return"), ctrl("enter"));
    }

    #[test]
    fn bare_letter() {
        assert_eq!(parse_key("q"), plain("q"));
        assert_eq!(parse_key("Q"), plain("q"));
    }

    #[test]
    fn unknown_key_is_none() {
        for name in ["", "f13", "ctrl+", "hyper+a", "ctrl+ab", "1"] {
            assert_eq!(parse_key(name), None, "{name}");
        }
    }

    #[test]
    fn typed_bytes_turn_newlines_into_enter() {
        assert_eq!(typed_bytes("a\nb\r\nc\rd\t"), b"a\rb\rc\rd\t");
    }

    #[test]
    fn sanitize_paste_replaces_controls() {
        assert_eq!(
            sanitize_paste("a\x1b[31mb\x03c\nd\te\r\x7f\u{9b}"),
            "a [31mb c\nd\te\r  "
        );
    }
}
