//! Paste and focus encoding. The vendored wrapper does not cover these, so this calls
//! libghostty-vt's C API (`vt/paste.h`, `vt/focus.h`) directly.

use ghostty_vt::sys;

/// Bytes to send for pasting `text`. Unsafe controls (ESC, NUL, DEL...) become spaces;
/// without bracketed paste newlines become CR; with it the text is wrapped in
/// `ESC[200~ ... ESC[201~`.
pub fn encode_paste(text: &str, bracketed: bool) -> Vec<u8> {
    let mut data = text.as_bytes().to_vec();
    // Encoding never grows the text beyond the two 6-byte bracket markers.
    let mut out = vec![0u8; data.len() + 12];
    let mut written = 0usize;
    // SAFETY: `data` and `out` are live Vecs for the whole call; the lengths passed are exactly
    // their lengths. `data` is mutable because the C side sanitises it in place. `written` is a
    // valid out-pointer, and `out` has room for the worst case (text plus both markers).
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
    if rc != sys::GHOSTTY_SUCCESS {
        return Vec::new();
    }
    out.truncate(written);
    out
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
        return Vec::new();
    }
    out[..written].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracketed_paste_wraps_and_keeps_newlines() {
        assert_eq!(encode_paste("a\nb", true), b"\x1b[200~a\nb\x1b[201~");
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
