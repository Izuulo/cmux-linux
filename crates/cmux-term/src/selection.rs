//! Mouse selection in absolute screen rows (scrollback + screen), so it stays put while the
//! viewport scrolls. The vendored wrapper does not expose Ghostty's terminal-owned selection,
//! so the selection lives here and the renderer paints it; word and line rules come from Ghostty.
//!
//! Endpoints are Ghostty tracked points, so they follow their cells when rows shift (scrollback
//! trim, resize or reflow). When they are lost (trimmed out of history, scrollback erased, screen
//! switched, reset) [`Selection::range`] and [`Selection::text`] return `None` instead of the
//! wrong rows. Ghostty does not flag the last two cases, so [`Term::epoch`] covers them.

use ghostty_vt::{SelectionPoint, SelectionRange, TrackedScreenPoint};

use crate::Term;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectMode {
    /// Single click and drag.
    Cell,
    /// Double click: whole words.
    Word,
    /// Triple click: whole lines.
    Line,
}

/// Owns tracked points registered with the [`Term`] it was created from; use it only with
/// that `Term`. Another terminal resolves to `None`.
pub struct Selection {
    mode: SelectMode,
    /// [`Term::epoch`] at creation; a different epoch means the rows were replaced.
    epoch: u64,
    /// The word or line (or cell) under the initial click: `[start, end]`.
    anchor: [TrackedScreenPoint; 2],
    /// Anchor extended to the drag head: `[start, end]`, inclusive.
    range: [TrackedScreenPoint; 2],
}

fn key(p: SelectionPoint) -> (u32, u16) {
    (p.row, p.column)
}

fn earlier(a: SelectionPoint, b: SelectionPoint) -> SelectionPoint {
    if key(a) <= key(b) { a } else { b }
}

fn later(a: SelectionPoint, b: SelectionPoint) -> SelectionPoint {
    if key(a) >= key(b) { a } else { b }
}

/// The cell, word or line at `point`, ordered.
fn unit(term: &Term, point: SelectionPoint, mode: SelectMode) -> SelectionRange {
    let vt = term.vt();
    let point = vt.normalize_selection_point_screen(point).unwrap_or(point);
    let found = match mode {
        SelectMode::Cell => None,
        SelectMode::Word => vt.select_word_screen(point).ok().flatten(),
        // A blank line has no text to trim to, so fall back to the whole row.
        SelectMode::Line => vt
            .select_line_screen(point)
            .ok()
            .flatten()
            .or_else(|| vt.select_line_screen_untrimmed(point).ok().flatten()),
    };
    found
        .map(|r| SelectionRange {
            start: earlier(r.start, r.end),
            end: later(r.start, r.end),
        })
        .unwrap_or(SelectionRange {
            start: point,
            end: point,
        })
}

fn track(term: &mut Term, r: SelectionRange) -> Option<[TrackedScreenPoint; 2]> {
    let vt = term.vt_mut();
    Some([
        vt.track_screen_point(r.start.column, r.start.row).ok()?,
        vt.track_screen_point(r.end.column, r.end.row).ok()?,
    ])
}

fn resolve(term: &Term, pts: &[TrackedScreenPoint; 2]) -> Option<SelectionRange> {
    let at = |p: &TrackedScreenPoint| {
        let (column, row) = term.vt().tracked_screen_point(p)?;
        Some(SelectionPoint { column, row })
    };
    let (a, b) = (at(&pts[0])?, at(&pts[1])?);
    Some(SelectionRange {
        start: earlier(a, b),
        end: later(a, b),
    })
}

/// Whether `range` covers the cell, as a linear (not rectangular) range.
///
/// The range ends at a wide character's lead cell, so the renderer must also highlight that
/// character's spacer tail cell (the column after the range end).
pub fn range_contains(range: SelectionRange, column: u16, row: u32) -> bool {
    let p = (row, column);
    key(range.start) <= p && p <= key(range.end)
}

impl Selection {
    /// `None` when `point` is outside the screen and scrollback.
    pub fn new(term: &mut Term, point: SelectionPoint, mode: SelectMode) -> Option<Self> {
        let anchor = unit(term, point, mode);
        Some(Self {
            mode,
            epoch: term.epoch(),
            anchor: track(term, anchor)?,
            range: track(term, anchor)?,
        })
    }

    /// Drag to `point`: the selection spans the anchor and the unit under `point`. The head
    /// may be before the anchor; the whole anchor unit stays selected. Does nothing if the
    /// anchor was lost.
    pub fn extend(&mut self, term: &mut Term, point: SelectionPoint) {
        if self.epoch != term.epoch() {
            return;
        }
        let Some(anchor) = resolve(term, &self.anchor) else {
            return;
        };
        let head = unit(term, point, self.mode);
        let start = earlier(anchor.start, head.start);
        let end = later(anchor.end, head.end);
        let vt = term.vt_mut();
        let [rs, re] = &mut self.range;
        // Out-of-grid heads leave the previous range in place.
        let _ = vt
            .set_tracked_screen_point(rs, start.column, start.row)
            .and_then(|()| vt.set_tracked_screen_point(re, end.column, end.row));
    }

    /// The current ordered range, or `None` if either end was lost.
    pub fn range(&self, term: &Term) -> Option<SelectionRange> {
        if self.epoch != term.epoch() {
            return None;
        }
        resolve(term, &self.range)
    }

    /// A plain click that never dragged: not a selection. True only for a cell-mode selection
    /// covering a single cell (including a drag that came back to its start). Word and line
    /// clicks always select something, so they are never empty.
    pub fn is_empty_click(&self, term: &Term) -> bool {
        self.mode == SelectMode::Cell && self.range(term).is_some_and(|r| r.start == r.end)
    }

    /// Per-cell test; for a whole frame call [`Selection::range`] once and use [`range_contains`].
    /// See [`range_contains`] for the wide-character caveat.
    pub fn contains(&self, term: &Term, column: u16, row: u32) -> bool {
        self.range(term)
            .is_some_and(|r| range_contains(r, column, row))
    }

    /// The selected text, with soft-wrapped lines joined (Ghostty's formatter). `None` if the
    /// selection was lost.
    pub fn text(&self, term: &mut Term) -> Option<String> {
        let r = self.range(term)?;
        let (s, e) = (r.start, r.end);
        term.vt_mut()
            .selection_text_absolute((s.column, u64::from(s.row)), (e.column, u64::from(e.row)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(text: &[u8]) -> Term {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut t = Term::new(20, 3, tx).unwrap();
        t.feed(text);
        t
    }

    fn p(column: u16, row: u32) -> SelectionPoint {
        SelectionPoint { column, row }
    }

    fn sel(t: &mut Term, point: SelectionPoint, mode: SelectMode) -> Selection {
        Selection::new(t, point, mode).unwrap()
    }

    #[test]
    fn drag_left_selects_cells_in_order() {
        let mut t = term(b"hello world");
        let mut s = sel(&mut t, p(4, 0), SelectMode::Cell);
        s.extend(&mut t, p(0, 0));
        assert_eq!(s.text(&mut t).as_deref(), Some("hello"));
        assert!(s.contains(&t, 2, 0));
        assert!(!s.contains(&t, 5, 0));
        assert!(!s.contains(&t, 2, 1));
    }

    #[test]
    fn contains_spans_rows_linearly() {
        let mut t = term(b"hello\r\nworld\r\nagain");
        let mut s = sel(&mut t, p(3, 0), SelectMode::Cell);
        s.extend(&mut t, p(1, 2));
        assert!(!s.contains(&t, 2, 0));
        assert!(s.contains(&t, 3, 0));
        assert!(s.contains(&t, 19, 0));
        assert!(s.contains(&t, 0, 1));
        assert!(s.contains(&t, 19, 1));
        assert!(s.contains(&t, 0, 2));
        assert!(s.contains(&t, 1, 2));
        assert!(!s.contains(&t, 2, 2));
    }

    #[test]
    fn double_click_selects_a_word() {
        let mut t = term(b"hello world");
        let s = sel(&mut t, p(7, 0), SelectMode::Word);
        assert_eq!(
            s.range(&t),
            Some(SelectionRange {
                start: p(6, 0),
                end: p(10, 0)
            })
        );
        assert_eq!(s.text(&mut t).as_deref(), Some("world"));
    }

    #[test]
    fn word_drag_extends_by_whole_words() {
        let mut t = term(b"hello world");
        let mut s = sel(&mut t, p(1, 0), SelectMode::Word);
        s.extend(&mut t, p(7, 0));
        assert_eq!(s.text(&mut t).as_deref(), Some("hello world"));
    }

    #[test]
    fn reversed_word_drag_keeps_the_whole_anchor_word() {
        let mut t = term(b"hello world");
        let mut s = sel(&mut t, p(7, 0), SelectMode::Word);
        s.extend(&mut t, p(1, 0));
        assert_eq!(s.text(&mut t).as_deref(), Some("hello world"));
        s.extend(&mut t, p(8, 0));
        assert_eq!(s.text(&mut t).as_deref(), Some("world"));
    }

    #[test]
    fn triple_click_selects_a_line() {
        let mut t = term(b"hello world\r\none\r\nfoo bar");
        let s = sel(&mut t, p(1, 2), SelectMode::Line);
        assert_eq!(s.text(&mut t).as_deref(), Some("foo bar"));
    }

    #[test]
    fn triple_click_on_a_blank_line_selects_the_whole_line() {
        let mut t = term(b"hello\r\n\r\nfoo");
        let s = sel(&mut t, p(3, 1), SelectMode::Line);
        assert_eq!(
            s.range(&t),
            Some(SelectionRange {
                start: p(0, 1),
                end: p(19, 1)
            })
        );
    }

    #[test]
    fn click_without_drag_is_empty() {
        let mut t = term(b"hello");
        let mut s = sel(&mut t, p(1, 0), SelectMode::Cell);
        assert!(s.is_empty_click(&t));
        s.extend(&mut t, p(3, 0));
        assert!(!s.is_empty_click(&t));
        s.extend(&mut t, p(1, 0));
        assert!(s.is_empty_click(&t));
        assert!(!sel(&mut t, p(1, 0), SelectMode::Word).is_empty_click(&t));
    }

    #[test]
    fn point_outside_the_grid_makes_no_selection() {
        let mut t = term(b"hello");
        assert!(Selection::new(&mut t, p(0, 100), SelectMode::Cell).is_none());
    }

    #[test]
    fn selection_in_scrollback_survives_trim() {
        let mut t = term(b"");
        for i in 0..10_100 {
            t.feed(format!("line{i}\r\n").as_bytes());
        }
        // Scrollback is full, so old rows are already being trimmed.
        let mid = (t.vt().scrollbar().unwrap().total / 2) as u32;
        let s = sel(&mut t, p(0, mid), SelectMode::Line);
        let before = s.text(&mut t).unwrap();
        for i in 10_100..12_000 {
            t.feed(format!("line{i}\r\n").as_bytes());
        }
        let after = s.text(&mut t);
        assert!(
            after.is_none() || after.as_deref() == Some(before.as_str()),
            "{before} -> {after:?}"
        );
    }

    #[test]
    fn selection_follows_its_row_as_history_grows() {
        let mut t = term(b"");
        for i in 0..8 {
            t.feed(format!("line{i}\r\n").as_bytes());
        }
        let s = sel(&mut t, p(0, 2), SelectMode::Line);
        assert_eq!(s.text(&mut t).as_deref(), Some("line2"));
        for i in 8..40 {
            t.feed(format!("line{i}\r\n").as_bytes());
        }
        assert_eq!(s.text(&mut t).as_deref(), Some("line2"));
    }

    #[test]
    fn selection_after_resize_is_correct_or_none() {
        let mut t = term(b"");
        for i in 0..8 {
            t.feed(format!("line{i}\r\n").as_bytes());
        }
        let s = sel(&mut t, p(0, 2), SelectMode::Line);
        for (cols, rows) in [(5, 3), (3, 3), (40, 6)] {
            t.resize(cols, rows, 8, 16).unwrap();
            let got = s.text(&mut t);
            assert!(got.is_none() || got.as_deref() == Some("line2"), "{got:?}");
        }
    }

    #[test]
    fn selection_lost_to_clear_or_alt_screen_is_none_never_other_text() {
        // ED 3 relocates tracked points to the top and the alternate screen reuses
        // coordinates, so these must read as lost.
        for seq in [&b"\x1b[3J"[..], b"\x1b[?1049h", b"\x1b[?47h", b"\x1bc"] {
            let mut t = term(b"");
            for i in 0..8 {
                t.feed(format!("line{i}\r\n").as_bytes());
            }
            let s = sel(&mut t, p(0, 2), SelectMode::Line);
            t.feed(seq);
            t.feed(b"zzz\r\nyyy\r\nxxx\r\nwww\r\n");
            assert_eq!(s.text(&mut t), None, "{seq:?}");
            assert_eq!(s.range(&t), None, "{seq:?}");
        }
    }

    #[test]
    fn selection_survives_plain_clear_screen() {
        let mut t = term(b"");
        for i in 0..8 {
            t.feed(format!("line{i}\r\n").as_bytes());
        }
        let s = sel(&mut t, p(0, 2), SelectMode::Line);
        t.feed(b"\x1b[2J");
        assert_eq!(s.text(&mut t).as_deref(), Some("line2"));
    }
}
