//! gpui input → CEF events (M13 plan B2): modifier flags, Windows virtual-key codes (what CEF's
//! key events are keyed on) and cursor shapes. Pure, so it is unit-tested.

use cef::{KeyEvent, KeyEventType};
use gpui::{CursorStyle, Keystroke, Modifiers, MouseButton};

// cef_event_flags_t.
const SHIFT: u32 = 1 << 1;
const CONTROL: u32 = 1 << 2;
const ALT: u32 = 1 << 3;
const LEFT_BUTTON: u32 = 1 << 4;
const MIDDLE_BUTTON: u32 = 1 << 5;
const RIGHT_BUTTON: u32 = 1 << 6;
const COMMAND: u32 = 1 << 7;

pub fn mods(m: &Modifiers) -> u32 {
    (m.shift as u32 * SHIFT)
        | (m.control as u32 * CONTROL)
        | (m.alt as u32 * ALT)
        | (m.platform as u32 * COMMAND)
}

pub fn button_flag(b: Option<MouseButton>) -> u32 {
    match b {
        Some(MouseButton::Left) => LEFT_BUTTON,
        Some(MouseButton::Middle) => MIDDLE_BUTTON,
        Some(MouseButton::Right) => RIGHT_BUTTON,
        _ => 0,
    }
}

/// gpui key name → Windows virtual-key code. Shifted symbols (Linux reports Shift+] as `}`)
/// map to their key's code.
pub fn vk(key: &str) -> i32 {
    match key {
        "backspace" => 0x08,
        "tab" => 0x09,
        "enter" => 0x0D,
        "escape" => 0x1B,
        "space" | " " => 0x20,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "end" => 0x23,
        "home" => 0x24,
        "left" => 0x25,
        "up" => 0x26,
        "right" => 0x27,
        "down" => 0x28,
        "insert" => 0x2D,
        "delete" => 0x2E,
        ";" | ":" => 0xBA,
        "=" | "+" => 0xBB,
        "," | "<" => 0xBC,
        "-" | "_" => 0xBD,
        "." | ">" => 0xBE,
        "/" | "?" => 0xBF,
        "`" | "~" => 0xC0,
        "[" | "{" => 0xDB,
        "\\" | "|" => 0xDC,
        "]" | "}" => 0xDD,
        "'" | "\"" => 0xDE,
        k => {
            let b = k.as_bytes();
            if b.len() == 1 && b[0].is_ascii_alphanumeric() {
                return b[0].to_ascii_uppercase() as i32;
            }
            if let Some(i) = k.strip_prefix('f').and_then(|n| n.parse::<i32>().ok())
                && (1..=24).contains(&i)
            {
                return 0x6F + i;
            }
            // Shift+digit symbols, US layout.
            match ")!@#$%^&*(".find(k) {
                Some(i) if k.len() == 1 => 0x30 + i as i32,
                _ => 0,
            }
        }
    }
}

/// The CEF events for one key press (RAWKEYDOWN, then CHAR when it types a character) or
/// release (KEYUP).
pub fn key_events(k: &Keystroke, down: bool) -> Vec<KeyEvent> {
    let m = mods(&k.modifiers);
    let code = vk(&k.key);
    let ev = |type_, ch: u16| KeyEvent {
        type_,
        modifiers: m,
        windows_key_code: code,
        character: ch,
        unmodified_character: ch,
        ..Default::default()
    };
    if !down {
        return vec![ev(KeyEventType::KEYUP, 0)];
    }
    let ch: Option<u16> = match k.key.as_str() {
        "enter" => Some(13),
        "backspace" | "tab" | "escape" | "delete" => None,
        _ => k.key_char.as_deref().and_then(|c| c.encode_utf16().next()),
    };
    let mut out = vec![ev(KeyEventType::RAWKEYDOWN, 0)];
    if let Some(ch) = ch.filter(|_| !k.modifiers.control && !k.modifiers.platform) {
        out.push(ev(KeyEventType::CHAR, ch));
    }
    out
}

/// cef_cursor_type_t → gpui cursor; `None` = the default arrow.
pub fn cursor(t: i32) -> Option<CursorStyle> {
    use cef::sys::cef_cursor_type_t as T;
    let is = |c: T| t == c as i32;
    Some(if is(T::CT_HAND) {
        CursorStyle::PointingHand
    } else if is(T::CT_IBEAM) {
        CursorStyle::IBeam
    } else if is(T::CT_VERTICALTEXT) {
        CursorStyle::IBeamCursorForVerticalLayout
    } else if is(T::CT_CROSS) || is(T::CT_CELL) {
        CursorStyle::Crosshair
    } else if is(T::CT_EASTWESTRESIZE) || is(T::CT_EASTRESIZE) || is(T::CT_WESTRESIZE) {
        CursorStyle::ResizeLeftRight
    } else if is(T::CT_COLUMNRESIZE) {
        CursorStyle::ResizeColumn
    } else if is(T::CT_NORTHSOUTHRESIZE) || is(T::CT_NORTHRESIZE) || is(T::CT_SOUTHRESIZE) {
        CursorStyle::ResizeUpDown
    } else if is(T::CT_ROWRESIZE) {
        CursorStyle::ResizeRow
    } else if is(T::CT_NORTHWESTSOUTHEASTRESIZE) {
        CursorStyle::ResizeUpLeftDownRight
    } else if is(T::CT_NORTHEASTSOUTHWESTRESIZE) {
        CursorStyle::ResizeUpRightDownLeft
    } else if is(T::CT_GRAB) {
        CursorStyle::OpenHand
    } else if is(T::CT_GRABBING) {
        CursorStyle::ClosedHand
    } else if is(T::CT_NOTALLOWED) || is(T::CT_NODROP) {
        CursorStyle::OperationNotAllowed
    } else if is(T::CT_CONTEXTMENU) {
        CursorStyle::ContextualMenu
    } else if is(T::CT_ALIAS) {
        CursorStyle::DragLink
    } else if is(T::CT_COPY) {
        CursorStyle::DragCopy
    } else if is(T::CT_NONE) {
        CursorStyle::None
    } else {
        return None;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ks(s: &str) -> Keystroke {
        Keystroke::parse(s).unwrap()
    }

    #[test]
    fn key_codes() {
        assert_eq!(vk("a"), 0x41);
        assert_eq!(vk("7"), 0x37);
        assert_eq!(vk("&"), 0x37);
        assert_eq!(vk("}"), 0xDD);
        assert_eq!(vk("f5"), 0x74);
        assert_eq!(vk("f25"), 0);
        assert_eq!(vk("left"), 0x25);
        assert_eq!(vk("ü"), 0);
    }

    #[test]
    fn typing_sends_a_char_and_shortcuts_do_not() {
        let typed = key_events(&ks("shift-x->X"), true);
        assert_eq!(typed.len(), 2);
        assert_eq!(typed[1].type_.get_raw(), KeyEventType::CHAR.get_raw());
        assert_eq!(typed[1].character, 'X' as u16);
        assert_eq!(typed[0].modifiers, SHIFT);
        // Ctrl+A selects all in the page: key down only.
        let ctrl_a = key_events(&ks("ctrl-a->a"), true);
        assert_eq!(ctrl_a.len(), 1);
        assert_eq!(ctrl_a[0].windows_key_code, 0x41);
        assert_eq!(key_events(&ks("enter"), true)[1].character, 13);
        assert_eq!(key_events(&ks("backspace"), true).len(), 1);
        let up = key_events(&ks("a"), false);
        assert_eq!(up[0].type_.get_raw(), KeyEventType::KEYUP.get_raw());
    }

    #[test]
    fn cursors() {
        use cef::sys::cef_cursor_type_t as T;
        assert_eq!(cursor(T::CT_HAND as i32), Some(CursorStyle::PointingHand));
        assert_eq!(cursor(T::CT_POINTER as i32), None);
    }
}
