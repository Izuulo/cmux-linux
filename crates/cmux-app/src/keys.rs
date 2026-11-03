//! gpui keystrokes to libghostty-vt key events, and the app's default key bindings.

use cmux_term::ghostty_vt::{KeyAction, KeyInput, Mods, sys};
use gpui::{Keystroke, Modifiers};

/// Things a key can do besides typing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Copy,
    Paste,
}

/// Default bindings, as `ctrl-alt-shift-super-key` chords (⌘ maps to Super).
/// Remapping through config arrives in M2; until then this table is the one place to change.
pub const BINDINGS: &[(&str, AppAction)] = &[
    ("super-c", AppAction::Copy),
    ("super-v", AppAction::Paste),
    ("ctrl-shift-c", AppAction::Copy),
    ("ctrl-shift-v", AppAction::Paste),
];

pub fn chord(ks: &Keystroke) -> String {
    let m = &ks.modifiers;
    let mut s = String::new();
    for (on, name) in [
        (m.control, "ctrl-"),
        (m.alt, "alt-"),
        (m.shift, "shift-"),
        (m.platform, "super-"),
    ] {
        if on {
            s.push_str(name);
        }
    }
    s.push_str(&ks.key);
    s
}

pub fn binding(ks: &Keystroke) -> Option<AppAction> {
    let chord = chord(ks);
    BINDINGS
        .iter()
        .find(|(b, _)| *b == chord)
        .map(|(_, action)| *action)
}

pub fn mods(m: &Modifiers) -> Mods {
    let mut out = Mods::default();
    for (on, bit) in [
        (m.shift, Mods::SHIFT),
        (m.control, Mods::CTRL),
        (m.alt, Mods::ALT),
        (m.platform, Mods::SUPER),
    ] {
        if on {
            out = out | bit;
        }
    }
    out
}

/// One printable US-layout key: its physical key and unshifted character.
fn ascii_key(c: char) -> Option<(sys::GhosttyKey, char)> {
    let c = c.to_ascii_lowercase();
    let offset = |base: char| c as u32 - base as u32;
    Some(match c {
        'a'..='z' => (sys::GHOSTTY_KEY_A + offset('a'), c),
        '0'..='9' => (sys::GHOSTTY_KEY_DIGIT_0 + offset('0'), c),
        ' ' => (sys::GHOSTTY_KEY_SPACE, ' '),
        '`' | '~' => (sys::GHOSTTY_KEY_BACKQUOTE, '`'),
        '-' | '_' => (sys::GHOSTTY_KEY_MINUS, '-'),
        '=' | '+' => (sys::GHOSTTY_KEY_EQUAL, '='),
        '[' | '{' => (sys::GHOSTTY_KEY_BRACKET_LEFT, '['),
        ']' | '}' => (sys::GHOSTTY_KEY_BRACKET_RIGHT, ']'),
        '\\' | '|' => (sys::GHOSTTY_KEY_BACKSLASH, '\\'),
        ';' | ':' => (sys::GHOSTTY_KEY_SEMICOLON, ';'),
        '\'' | '"' => (sys::GHOSTTY_KEY_QUOTE, '\''),
        ',' | '<' => (sys::GHOSTTY_KEY_COMMA, ','),
        '.' | '>' => (sys::GHOSTTY_KEY_PERIOD, '.'),
        '/' | '?' => (sys::GHOSTTY_KEY_SLASH, '/'),
        '!' => (sys::GHOSTTY_KEY_DIGIT_1, '1'),
        '@' => (sys::GHOSTTY_KEY_DIGIT_2, '2'),
        '#' => (sys::GHOSTTY_KEY_DIGIT_3, '3'),
        '$' => (sys::GHOSTTY_KEY_DIGIT_4, '4'),
        '%' => (sys::GHOSTTY_KEY_DIGIT_5, '5'),
        '^' => (sys::GHOSTTY_KEY_DIGIT_6, '6'),
        '&' => (sys::GHOSTTY_KEY_DIGIT_7, '7'),
        '*' => (sys::GHOSTTY_KEY_DIGIT_8, '8'),
        '(' => (sys::GHOSTTY_KEY_DIGIT_9, '9'),
        ')' => (sys::GHOSTTY_KEY_DIGIT_0, '0'),
        _ => return None,
    })
}

/// gpui's layout-independent key name (gpui platform/linux/platform.rs `from_xkb`) to
/// Ghostty's physical key and unshifted codepoint.
// shortcut: US-layout logical keys only, so non-US layouts report the wrong physical key for kitty and Ctrl combos, upgrade when kitty-protocol users report layout bugs (M6)
pub fn physical_key(key: &str) -> (sys::GhosttyKey, Option<char>) {
    let mut chars = key.chars();
    if let (Some(c), None) = (chars.next(), chars.next())
        && let Some((k, base)) = ascii_key(c)
    {
        return (k, Some(base));
    }
    let k = match key {
        "enter" => sys::GHOSTTY_KEY_ENTER,
        "tab" => sys::GHOSTTY_KEY_TAB,
        "backspace" => sys::GHOSTTY_KEY_BACKSPACE,
        "escape" => sys::GHOSTTY_KEY_ESCAPE,
        "space" => return (sys::GHOSTTY_KEY_SPACE, Some(' ')),
        "delete" => sys::GHOSTTY_KEY_DELETE,
        "insert" => sys::GHOSTTY_KEY_INSERT,
        "left" => sys::GHOSTTY_KEY_ARROW_LEFT,
        "right" => sys::GHOSTTY_KEY_ARROW_RIGHT,
        "up" => sys::GHOSTTY_KEY_ARROW_UP,
        "down" => sys::GHOSTTY_KEY_ARROW_DOWN,
        "home" => sys::GHOSTTY_KEY_HOME,
        "end" => sys::GHOSTTY_KEY_END,
        "pageup" => sys::GHOSTTY_KEY_PAGE_UP,
        "pagedown" => sys::GHOSTTY_KEY_PAGE_DOWN,
        f if f.len() > 1 && f.starts_with('f') => match f[1..].parse::<u32>() {
            Ok(n @ 1..=24) => sys::GHOSTTY_KEY_F1 + (n - 1),
            _ => sys::GHOSTTY_KEY_UNIDENTIFIED,
        },
        _ => sys::GHOSTTY_KEY_UNIDENTIFIED,
    };
    (k, None)
}

/// The encoder input for a gpui keystroke. `utf8` is the typed text (`key_char`), dropped
/// under Ctrl or when it holds control characters, as the encoder requires. Super
/// (`platform`) keystrokes must be filtered out by the caller before encoding; the view does this.
pub fn key_input(ks: &Keystroke, action: KeyAction) -> KeyInput {
    let m = &ks.modifiers;
    let (key, unshifted) = physical_key(&ks.key);
    let utf8 = match &ks.key_char {
        Some(text) if !m.control && !text.chars().any(char::is_control) => text.clone(),
        _ => String::new(),
    };
    let consumed_mods = if m.shift && !utf8.is_empty() {
        Mods::SHIFT
    } else {
        Mods::default()
    };
    KeyInput {
        key,
        mods: mods(m),
        consumed_mods,
        utf8,
        unshifted_codepoint: unshifted.map_or(0, u32::from),
        action: Some(action),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cmux_term::ghostty_vt::{Callbacks, KeyEncoder, Terminal};

    fn ks(m: &str, key: &str, key_char: Option<&str>) -> Keystroke {
        let modifiers = Modifiers {
            control: m.contains('c'),
            alt: m.contains('a'),
            shift: m.contains('s'),
            platform: m.contains('p'),
            function: false,
        };
        Keystroke {
            modifiers,
            key: key.into(),
            key_char: key_char.map(Into::into),
        }
    }

    fn encode(k: &Keystroke) -> Vec<u8> {
        encode_as(k, KeyAction::Press)
    }

    fn encode_as(k: &Keystroke, action: KeyAction) -> Vec<u8> {
        let term = Terminal::new(80, 24, 0, Callbacks::default()).unwrap();
        let mut enc = KeyEncoder::new().unwrap();
        enc.sync_from_terminal(&term);
        let mut out = Vec::new();
        enc.encode(&key_input(k, action), &mut out).unwrap();
        out
    }

    #[test]
    fn encodes_the_key_table() {
        let cases: &[(&str, &str, Option<&str>, &[u8])] = &[
            ("", "a", Some("a"), b"a"),
            ("s", "a", Some("A"), b"A"),
            ("c", "c", None, b"\x03"),
            ("a", "b", Some("b"), b"\x1bb"),
            ("", "1", Some("1"), b"1"),
            ("", "!", Some("!"), b"!"),
            ("", "space", Some(" "), b" "),
            ("c", "space", None, b"\x00"),
            ("", "enter", Some("\r"), b"\r"),
            ("", "tab", None, b"\t"),
            ("s", "tab", None, b"\x1b[Z"),
            ("", "backspace", None, b"\x7f"),
            ("", "escape", None, b"\x1b"),
            ("", "up", None, b"\x1b[A"),
            ("c", "left", None, b"\x1b[1;5D"),
            ("", "home", None, b"\x1b[H"),
            ("", "end", None, b"\x1b[F"),
            ("", "pageup", None, b"\x1b[5~"),
            ("", "pagedown", None, b"\x1b[6~"),
            ("", "delete", None, b"\x1b[3~"),
            ("", "f1", None, b"\x1bOP"),
            ("", "f5", None, b"\x1b[15~"),
            ("", "f12", None, b"\x1b[24~"),
            ("", "e", Some("é"), "é".as_bytes()),
        ];
        for (m, key, ch, want) in cases {
            assert_eq!(encode(&ks(m, key, *ch)), *want, "mods {m:?} key {key:?}");
        }
    }

    #[test]
    fn physical_keys() {
        assert_eq!(physical_key("a"), (sys::GHOSTTY_KEY_A, Some('a')));
        assert_eq!(physical_key("z"), (sys::GHOSTTY_KEY_Z, Some('z')));
        assert_eq!(physical_key("9"), (sys::GHOSTTY_KEY_DIGIT_9, Some('9')));
        assert_eq!(physical_key("?"), (sys::GHOSTTY_KEY_SLASH, Some('/')));
        assert_eq!(physical_key("f24"), (sys::GHOSTTY_KEY_F24, None));
        assert_eq!(physical_key("f25"), (sys::GHOSTTY_KEY_UNIDENTIFIED, None));
        assert_eq!(physical_key("f"), (sys::GHOSTTY_KEY_F, Some('f')));
        assert_eq!(physical_key("pagedown"), (sys::GHOSTTY_KEY_PAGE_DOWN, None));
        assert_eq!(physical_key("é"), (sys::GHOSTTY_KEY_UNIDENTIFIED, None));
    }

    #[test]
    fn shifted_symbol_with_shift_still_set() {
        // Robustness only: gpui clears shift for symbols, so this shape is not emitted.
        // shift stays in consumed_mods, and the text is still written as-is.
        assert_eq!(encode(&ks("s", "!", Some("!"))), b"!");
    }

    #[test]
    fn composed_text_without_key_entry_passes_through() {
        // gpui delivers dead-key/compose results as an unknown key name with key_char set;
        // no key entry matches, so the encoder writes the utf8 (key_encode.zig ~228-231).
        assert_eq!(encode(&ks("", "eacute", Some("é"))), "é".as_bytes());
    }

    #[test]
    fn ime_commit_single_char() {
        assert_eq!(encode(&ks("", "a", Some("a"))), b"a");
    }

    #[test]
    fn ctrl_symbols_produce_c0_bytes() {
        // key_input drops utf8 under Ctrl, so ctrlSeq (key_encode.zig ~710-800) uses the
        // physical key's codepoint. Ghostty does NOT map Ctrl+[ to ESC: the fixterms rule
        // (key_encode.zig:852) leaves '[' out of the C0 table, and with no utf8 the CSI-u
        // fallback is skipped too. So Ctrl+[ currently sends nothing; xterm sends \x1b.
        assert_eq!(encode(&ks("c", "[", None)), b"");
        assert_eq!(encode(&ks("c", "\\", None)), b"\x1c");
        assert_eq!(encode(&ks("c", "]", None)), b"\x1d");
        assert_eq!(encode(&ks("c", "space", None)), b"\x00");
    }

    #[test]
    fn alt_shift_letter_prefixes_esc() {
        // gpui: key "b", shift set, key_char "B". Alt prefix applies to the utf8 text.
        assert_eq!(encode(&ks("as", "b", Some("B"))), b"\x1bB");
    }

    #[test]
    fn keypad_names_fall_through_to_text() {
        // gpui platform/linux/platform.rs (~line 883) strips "kp_" from keypad keysym names,
        // so KP_Decimal arrives as "decimal", which physical_key does not know. With key_char
        // set, the encoder writes the text. Digit keypad keys reuse the normal digit names.
        assert_eq!(encode(&ks("", "decimal", Some("."))), b".");
    }

    #[test]
    fn legacy_release_encodes_nothing() {
        assert_eq!(
            encode_as(&ks("", "a", Some("a")), KeyAction::Release),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn letters_and_digits_map_to_contiguous_physical_keys() {
        for (i, c) in ('a'..='z').enumerate() {
            assert_eq!(
                physical_key(&c.to_string()).0,
                sys::GHOSTTY_KEY_A + i as u32,
                "letter {c}"
            );
        }
        for (i, c) in ('0'..='9').enumerate() {
            assert_eq!(
                physical_key(&c.to_string()).0,
                sys::GHOSTTY_KEY_DIGIT_0 + i as u32,
                "digit {c}"
            );
        }
    }

    #[test]
    fn modifiers_map_to_ghostty_bits() {
        let m = mods(&ks("csap", "a", None).modifiers);
        assert!(m.contains(Mods::CTRL | Mods::SHIFT | Mods::ALT | Mods::SUPER));
        assert_eq!(mods(&Modifiers::default()), Mods::default());
    }

    #[test]
    fn control_text_and_ctrl_never_reach_utf8() {
        assert_eq!(
            key_input(&ks("", "enter", Some("\r")), KeyAction::Press).utf8,
            ""
        );
        assert_eq!(
            key_input(&ks("c", "a", Some("a")), KeyAction::Press).utf8,
            ""
        );
        let shifted = key_input(&ks("s", "a", Some("A")), KeyAction::Repeat);
        assert_eq!(
            (shifted.utf8.as_str(), shifted.consumed_mods),
            ("A", Mods::SHIFT)
        );
        assert_eq!(shifted.action, Some(KeyAction::Repeat));
    }

    #[test]
    fn default_bindings() {
        assert_eq!(binding(&ks("p", "c", None)), Some(AppAction::Copy));
        assert_eq!(binding(&ks("p", "v", None)), Some(AppAction::Paste));
        assert_eq!(binding(&ks("cs", "c", Some("C"))), Some(AppAction::Copy));
        assert_eq!(binding(&ks("cs", "v", Some("V"))), Some(AppAction::Paste));
        assert_eq!(binding(&ks("c", "c", None)), None);
        assert_eq!(chord(&ks("csap", "x", None)), "ctrl-alt-shift-super-x");
    }
}
