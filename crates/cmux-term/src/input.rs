//! Paste and focus encoding. The vendored wrapper does not cover these, so this calls
//! libghostty-vt's C API (`vt/paste.h`, `vt/focus.h`) directly.

use ghostty_vt::sys;

/// Bytes to send for pasting `text`. Unsafe controls (ESC, NUL, DEL...) become spaces;
/// without bracketed paste newlines become CR; with it the text is wrapped in
/// `ESC[200~ ... ESC[201~`.
///
/// This control-byte stripping is what makes bracketed paste safe: an embedded `ESC[201~`
/// loses its ESC and so cannot end the bracket early and smuggle the rest in as typed input.
///
/// Whether to ask the user before pasting suspicious text (`ghostty_paste_is_safe`) is the
/// caller's decision; this function encodes whatever it is given.
pub fn encode_paste(text: &str, bracketed: bool) -> Vec<u8> {
    // Encoding never grows the text beyond the two 6-byte bracket markers.
    paste_with_capacity(text, bracketed, text.len() + 12)
}

fn paste_with_capacity(text: &str, bracketed: bool, capacity: usize) -> Vec<u8> {
    let mut out = vec![0u8; capacity];
    // The C side sanitises its input in place, so a retry needs a fresh copy.
    for _ in 0..2 {
        let mut data = text.as_bytes().to_vec();
        let mut written = 0usize;
        // SAFETY: `data` and `out` are live Vecs for the whole call; the lengths passed are
        // exactly their lengths. `data` is mutable because the C side sanitises it in place.
        // `written` is a valid out-pointer.
        let rc = unsafe {
            sys::ghostty_paste_encode(
                data.as_mut_ptr().cast(),
                data.len(),
                bracketed,
                out.as_mut_ptr().cast(),
                out.len(),
                &mut written,
            )
        };
        if rc == sys::GHOSTTY_SUCCESS {
            out.truncate(written);
            return out;
        }
        if rc != sys::GHOSTTY_OUT_OF_SPACE {
            debug_assert!(false, "ghostty_paste_encode failed: {rc}");
            return Vec::new();
        }
        // `written` is the required size.
        out = vec![0u8; written];
    }
    debug_assert!(false, "ghostty_paste_encode out of space after retry");
    Vec::new()
}

/// `ESC[I` / `ESC[O`, for applications that enabled focus reporting (DEC mode 1004).
pub fn encode_focus(gained: bool) -> Vec<u8> {
    let event = if gained {
        sys::GHOSTTY_FOCUS_GAINED
    } else {
        sys::GHOSTTY_FOCUS_LOST
    };
    let mut out = [0u8; 8];
    let mut written = 0usize;
    // SAFETY: `out` is a live 8-byte buffer and its length is passed as the capacity (the
    // sequence is 3 bytes); `written` is a valid out-pointer.
    let rc = unsafe {
        sys::ghostty_focus_encode(event, out.as_mut_ptr().cast(), out.len(), &mut written)
    };
    if rc != sys::GHOSTTY_SUCCESS {
        debug_assert!(false, "ghostty_focus_encode failed: {rc}");
        return Vec::new();
    }
    out[..written].to_vec()
}

/// Whether pasting `text` needs the user's OK under Ghostty's `clipboard-paste-protection`
/// (Surface.zig `completeClipboardPaste`): a newline or a bracketed-paste end
/// (`ghostty_paste_is_safe`), except that a bracketed paste without an embedded end marker is
/// trusted (`clipboard-paste-bracketed-safe`, default true).
pub fn paste_needs_confirmation(text: &str, bracketed: bool) -> bool {
    if bracketed && !text.contains("\x1b[201~") {
        return false;
    }
    // SAFETY: `text` is a live &str; pointer and length describe exactly its bytes.
    !unsafe { sys::ghostty_paste_is_safe(text.as_ptr().cast(), text.len()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_line_pastes_need_confirmation_unless_bracketed() {
        assert!(!paste_needs_confirmation("ls -la", false));
        assert!(paste_needs_confirmation("rm -rf ~\n", false));
        assert!(!paste_needs_confirmation("line1\nline2", true));
        assert!(paste_needs_confirmation("a\x1b[201~b", true));
        assert!(paste_needs_confirmation("a\x1b[201~b", false));
    }

    #[test]
    fn bracketed_paste_wraps_and_keeps_newlines() {
        assert_eq!(encode_paste("a\nb", true), b"\x1b[200~a\nb\x1b[201~");
    }

    #[test]
    fn embedded_bracket_end_cannot_break_out() {
        assert_eq!(
            encode_paste("a\x1b[201~rm", true),
            b"\x1b[200~a [201~rm\x1b[201~"
        );
    }

    #[test]
    fn undersized_buffer_is_retried_with_the_reported_size() {
        assert_eq!(
            paste_with_capacity("a\nb", true, 1),
            b"\x1b[200~a\nb\x1b[201~"
        );
    }

    #[test]
    fn plain_paste_turns_newlines_into_cr_and_strips_escape() {
        assert_eq!(encode_paste("a\nb\x1bc", false), b"a\rb c");
    }

    #[test]
    fn empty_paste_is_empty() {
        assert_eq!(encode_paste("", false), b"");
    }

    #[test]
    fn focus_reports() {
        assert_eq!(encode_focus(true), b"\x1b[I");
        assert_eq!(encode_focus(false), b"\x1b[O");
    }
}
