//! cmux-term: PTY + libghostty-vt wrapper.

pub mod osc;
pub mod pty;

pub use ghostty_vt;

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;

use ghostty_vt::{Callbacks, Cell, CursorInfo, Error, RenderState, Rgb, Scrollbar, Terminal};
use osc::{OscEvent, OscScanner};

/// Scrollback lines kept per terminal.
// shortcut: fixed; Ghostty's `scrollback-limit` (bytes) is not mapped yet. Add with M10 persistence.
const SCROLLBACK: usize = 10_000;

const TITLE_CHANGED: u8 = 1;
const BELL: u8 = 2;

/// Things the UI reacts to, in the order they were found in one `feed` call
/// (OSC events first, then title and bell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermEvent {
    Title(String),
    Bell,
    Cwd(String),
    Notify { title: Option<String>, body: String },
}

impl From<OscEvent> for TermEvent {
    fn from(e: OscEvent) -> Self {
        match e {
            OscEvent::Cwd(path) => TermEvent::Cwd(path),
            OscEvent::Notify { title, body } => TermEvent::Notify { title, body },
        }
    }
}

/// Per-frame facts the renderer needs before it walks the rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    pub cols: u16,
    pub rows: u16,
    /// `None` when hidden or scrolled out of view.
    pub cursor: Option<CursorInfo>,
    pub default_fg: Rgb,
    pub default_bg: Rgb,
    /// OSC 12 cursor colour, if the application set one.
    pub cursor_color: Option<Rgb>,
    pub scrollbar: Option<Scrollbar>,
}

pub struct Term {
    vt: Terminal,
    render: RenderState,
    osc: OscScanner,
    flags: Arc<AtomicU8>,
}

impl Term {
    /// `pty_tx` receives the terminal's replies (DA, DSR, OSC colour queries). The caller
    /// writes them to the PTY; they are produced inside `feed`, so never write from there.
    pub fn new(cols: u16, rows: u16, pty_tx: Sender<Vec<u8>>) -> Result<Self, Error> {
        let flags = Arc::new(AtomicU8::new(0));
        let (title, bell) = (flags.clone(), flags.clone());
        let callbacks = Callbacks {
            on_pty_write: Some(Box::new(move |bytes: &[u8]| {
                let _ = pty_tx.send(bytes.to_vec());
            })),
            on_title_changed: Some(Box::new(move || {
                title.fetch_or(TITLE_CHANGED, Ordering::Relaxed);
            })),
            on_bell: Some(Box::new(move || {
                bell.fetch_or(BELL, Ordering::Relaxed);
            })),
        };
        Ok(Self {
            vt: Terminal::new(cols, rows, SCROLLBACK, callbacks)?,
            render: RenderState::new()?,
            osc: OscScanner::new(),
            flags,
        })
    }

    /// Feeds PTY output and returns the events it contained.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<TermEvent> {
        // Scan what Ghostty parsed: 8-bit C1 controls (0x9d OSC, 0x9c ST) arrive as ESC ] / ESC \.
        let normalized = self.vt.vt_write_with_normalized(bytes);
        let mut events: Vec<TermEvent> = self
            .osc
            .feed(&normalized)
            .into_iter()
            .map(TermEvent::from)
            .collect();
        let flags = self.flags.swap(0, Ordering::Relaxed);
        if flags & TITLE_CHANGED != 0
            && let Some(title) = self.vt.title()
        {
            events.push(TermEvent::Title(title));
        }
        if flags & BELL != 0 {
            events.push(TermEvent::Bell);
        }
        events
    }

    pub fn title(&self) -> Option<String> {
        self.vt.title()
    }

    /// Cell pixel sizes feed pixel-size reports (CSI 14/16 t) and kitty graphics.
    pub fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        cell_w_px: u32,
        cell_h_px: u32,
    ) -> Result<(), Error> {
        self.vt.resize(cols, rows, cell_w_px, cell_h_px)
    }

    /// Moves the viewport by `delta` rows; negative scrolls into history.
    pub fn scroll(&mut self, delta: isize) {
        self.vt.scroll_delta(delta);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.vt.scroll_to_bottom();
    }

    /// The engine, for encoders, modes and selection queries.
    pub fn vt(&self) -> &Terminal {
        &self.vt
    }

    pub fn vt_mut(&mut self) -> &mut Terminal {
        &mut self.vt
    }

    /// Snapshots the viewport for this frame. Call before [`Term::walk_rows`].
    pub fn update_frame(&mut self) -> Result<FrameInfo, Error> {
        self.render.update(&mut self.vt)?;
        let (cols, rows) = self.render.size();
        let (default_bg, default_fg) = self.render.default_colors();
        Ok(FrameInfo {
            cols,
            rows,
            cursor: self.render.cursor(),
            default_fg,
            default_bg,
            cursor_color: self.render.cursor_color(),
            scrollbar: self.vt.scrollbar(),
        })
    }

    /// Visits each viewport row of the last [`Term::update_frame`] without copying cells.
    pub fn walk_rows(&mut self, mut f: impl FnMut(usize, &[Cell])) -> Result<(), Error> {
        self.render.walk_rows(|row, _dirty, cells| f(row, cells))
    }

    /// The viewport as text lines. For tests and debugging, not the render path.
    pub fn text_lines(&mut self) -> Result<Vec<String>, Error> {
        self.render.update(&mut self.vt)?;
        self.render.text_lines()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn term(cols: u16, rows: u16) -> (Term, mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = mpsc::channel();
        (Term::new(cols, rows, tx).unwrap(), rx)
    }

    #[test]
    fn device_status_reply_goes_to_the_pty_sender() {
        let (mut t, rx) = term(20, 3);
        t.feed(b"\x1b[6n");
        assert_eq!(rx.try_recv().unwrap(), b"\x1b[1;1R");
    }

    #[test]
    fn title_and_bell_are_events() {
        let (mut t, _rx) = term(20, 3);
        assert_eq!(
            t.feed(b"\x1b]2;hi\x07\x07"),
            vec![TermEvent::Title("hi".into()), TermEvent::Bell]
        );
        assert_eq!(t.feed(b"plain"), vec![]);
    }

    #[test]
    fn eight_bit_c1_osc_is_scanned() {
        let (mut t, _rx) = term(20, 3);
        assert_eq!(
            t.feed(b"\x9d7;file://h/tmp\x9c"),
            vec![TermEvent::Cwd("/tmp".into())]
        );
    }

    #[test]
    fn resize_changes_frame_dims() {
        let (mut t, _rx) = term(20, 3);
        t.resize(10, 4, 8, 16).unwrap();
        let info = t.update_frame().unwrap();
        assert_eq!((info.cols, info.rows), (10, 4));
    }

    #[test]
    fn scroll_moves_viewport_into_history_and_back() {
        let (mut t, _rx) = term(10, 2);
        t.feed(b"1\r\n2\r\n3\r\n4");
        t.scroll(-1);
        assert!(t.update_frame().unwrap().scrollbar.unwrap().scrolled_back());
        assert_eq!(t.text_lines().unwrap()[0].trim_end(), "2");
        t.scroll_to_bottom();
        assert_eq!(t.text_lines().unwrap()[0].trim_end(), "3");
    }

    #[test]
    fn walk_rows_visits_every_viewport_row() {
        let (mut t, _rx) = term(10, 3);
        t.feed(b"ab");
        let info = t.update_frame().unwrap();
        let mut rows = Vec::new();
        t.walk_rows(|row, cells| rows.push((row, cells.len(), cells[1].text.clone())))
            .unwrap();
        assert_eq!(rows.len(), info.rows as usize);
        assert_eq!(rows[0], (0, 10, "b".to_string()));
    }
}
