//! IME (text-input-v3 on Wayland): committed text goes to the PTY, preedit is held for the
//! element to paint at the cursor. Shape follows Zed's `TerminalInputHandler`
//! (zed-industries/zed@69e2130, crates/terminal_view/src/terminal_element.rs, GPL-3.0-or-later).
//!
//! Input paths, so a key is never typed twice (gpui 0.2.2, Linux):
//! - Ordinary keys, and compose results (`Status::Composed` sets `key_char`, wayland/client.rs:1320),
//!   arrive as key-downs; `key_down` encodes them and stops propagation, which makes the window
//!   skip its `replace_text_in_range` fallback (wayland/window.rs:719-736).
//! - Pending dead keys and compose steps arrive with `key_char: None` (types nothing) plus a
//!   `SetMarkedText` preedit; the composed text then arrives once, as the key-down above.
//! - IME commits of one byte are replayed as a key-down (client.rs:1443); longer ones come here.

use std::ops::Range;

use gpui::{
    Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window, point, px, size,
};

use crate::terminal_view::TerminalView;

/// Ends composition and returns the bytes the committed text sends. An empty commit (gpui's
/// `DeleteText` cancel, wayland/window.rs:650) only drops the preedit.
pub(crate) fn commit(marked: &mut String, text: &str) -> Vec<u8> {
    marked.clear();
    text.as_bytes().to_vec()
}

impl EntityInputHandler for TerminalView {
    fn text_for_range(
        &mut self,
        _range: Range<usize>,
        _adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        // An empty caret, so the IME treats the terminal as an editable field.
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        (!self.marked_text.is_empty()).then(|| 0..self.marked_text.encode_utf16().count())
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_text.clear();
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Like typing: committed text drops the selection and snaps to the live screen.
        let bytes = commit(&mut self.marked_text, text);
        if !bytes.is_empty() {
            self.selection = None;
        }
        self.send_input(bytes);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked_text = new_text.to_string();
        cx.notify();
    }

    /// The candidate window sits at the cursor cell (offset by `range.start` cells).
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let g = self.geometry?;
        let (col, row) = self.cursor_cell?;
        let x = (f32::from(col) + range.start as f32) * g.cell_w;
        let origin = point(
            g.origin.x + px(x),
            g.origin.y + px(f32::from(row) * g.cell_h),
        );
        Some(Bounds::new(origin, size(px(g.cell_w), px(g.cell_h))))
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::key_input;
    use cmux_term::ghostty_vt::KeyAction;
    use gpui::{Keystroke, Modifiers};

    fn key(key: &str, key_char: Option<&str>) -> Keystroke {
        Keystroke {
            modifiers: Modifiers::default(),
            key: key.into(),
            key_char: key_char.map(Into::into),
        }
    }

    #[test]
    fn commit_sends_the_text_once_and_ends_composition() {
        let mut marked = "かん".to_string();
        assert_eq!(commit(&mut marked, "漢"), "漢".as_bytes());
        assert!(marked.is_empty());
    }

    #[test]
    fn cancelling_a_preedit_sends_nothing() {
        let mut marked = "´".to_string();
        assert!(commit(&mut marked, "").is_empty());
        assert!(marked.is_empty());
    }

    #[test]
    fn dead_key_then_composed_key_types_the_character_once() {
        // gpui Wayland: the dead key is a preedit and a key-down without key_char (types
        // nothing); the composed key-down carries the text. Only the latter reaches the PTY,
        // through the key path, and it stops propagation, so the IME path never repeats it.
        let dead = key("'", None);
        assert_eq!(key_input(&dead, KeyAction::Press).utf8, "");
        let composed = key("eacute", Some("é"));
        assert_eq!(key_input(&composed, KeyAction::Press).utf8, "é");
    }
}
