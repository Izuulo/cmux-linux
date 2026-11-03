//! Mouse selection in absolute screen rows (scrollback + screen), so it stays put while the
//! viewport scrolls. The vendored wrapper does not expose Ghostty's terminal-owned selection,
//! so the range lives here and the renderer paints it; word and line rules come from Ghostty.

use ghostty_vt::{SelectionPoint, SelectionRange};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    mode: SelectMode,
    /// The word or line (or cell) under the initial click.
    anchor: SelectionRange,
    /// Ordered: `start` is never after `end`. Both ends are inclusive.
    range: SelectionRange,
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
        SelectMode::Line => vt.select_line_screen(point).ok().flatten(),
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

impl Selection {
    pub fn new(term: &Term, point: SelectionPoint, mode: SelectMode) -> Self {
        let anchor = unit(term, point, mode);
        Self {
            mode,
            anchor,
            range: anchor,
        }
    }

    /// Drag to `point`: the selection spans the anchor and the unit under `point`.
    pub fn extend(&mut self, term: &Term, point: SelectionPoint) {
        let head = unit(term, point, self.mode);
        self.range = SelectionRange {
            start: earlier(self.anchor.start, head.start),
            end: later(self.anchor.end, head.end),
        };
    }

    pub fn range(&self) -> SelectionRange {
        self.range
    }

    /// A plain click that never dragged: not a selection.
    pub fn is_empty_click(&self) -> bool {
        self.mode == SelectMode::Cell && self.range.start == self.range.end
    }

    pub fn contains(&self, column: u16, row: u32) -> bool {
        let p = (row, column);
        key(self.range.start) <= p && p <= key(self.range.end)
    }

    /// The selected text, with soft-wrapped lines joined (Ghostty's formatter).
    pub fn text(&self, term: &mut Term) -> Option<String> {
        let (s, e) = (self.range.start, self.range.end);
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

    #[test]
    fn drag_left_selects_cells_in_order() {
        let mut t = term(b"hello world");
        let mut s = Selection::new(&t, p(4, 0), SelectMode::Cell);
        s.extend(&t, p(0, 0));
        assert_eq!(s.text(&mut t).as_deref(), Some("hello"));
        assert!(s.contains(2, 0));
        assert!(!s.contains(5, 0));
        assert!(!s.contains(2, 1));
    }

    #[test]
    fn double_click_selects_a_word() {
        let mut t = term(b"hello world");
        let s = Selection::new(&t, p(7, 0), SelectMode::Word);
        assert_eq!(
            s.range(),
            SelectionRange {
                start: p(6, 0),
                end: p(10, 0)
            }
        );
        assert_eq!(s.text(&mut t).as_deref(), Some("world"));
    }

    #[test]
    fn word_drag_extends_by_whole_words() {
        let mut t = term(b"hello world");
        let mut s = Selection::new(&t, p(1, 0), SelectMode::Word);
        s.extend(&t, p(7, 0));
        assert_eq!(s.text(&mut t).as_deref(), Some("hello world"));
    }

    #[test]
    fn triple_click_selects_a_line() {
        let mut t = term(b"hello world\r\none\r\nfoo bar");
        let s = Selection::new(&t, p(1, 2), SelectMode::Line);
        assert_eq!(s.text(&mut t).as_deref(), Some("foo bar"));
    }

    #[test]
    fn click_without_drag_is_empty() {
        let t = term(b"hello");
        assert!(Selection::new(&t, p(1, 0), SelectMode::Cell).is_empty_click());
        assert!(!Selection::new(&t, p(1, 0), SelectMode::Word).is_empty_click());
    }
}
