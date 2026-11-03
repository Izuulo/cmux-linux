//! cmux-term: PTY + libghostty-vt wrapper.

pub mod osc;

pub use ghostty_vt;

use ghostty_vt::{Callbacks, Cell, CursorInfo, Error, RenderState, Terminal};
use osc::{OscEvent, OscScanner};

/// Scrollback lines kept per terminal. Configurable from M1.
const SCROLLBACK: usize = 10_000;

pub struct Term {
    vt: Terminal,
    render: RenderState,
    osc: OscScanner,
}

/// What the renderer needs from one frame.
pub struct Snapshot {
    pub lines: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
    pub cursor: Option<CursorInfo>,
}

impl Term {
    pub fn new(cols: u16, rows: u16) -> Result<Self, Error> {
        Ok(Self {
            vt: Terminal::new(cols, rows, SCROLLBACK, Callbacks::default())?,
            render: RenderState::new()?,
            osc: OscScanner::new(),
        })
    }

    /// Feeds PTY output. Returns the cwd/notification OSCs it contained.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<OscEvent> {
        let events = self.osc.feed(bytes);
        self.vt.vt_write(bytes);
        events
    }

    pub fn title(&self) -> Option<String> {
        self.vt.title()
    }

    pub fn snapshot(&mut self) -> Result<Snapshot, Error> {
        self.render.update(&mut self.vt)?;
        let lines = self.render.text_lines()?;
        let cursor = self.render.cursor();
        let mut rows = Vec::new();
        self.render.walk_rows(|_, _, cells| rows.push(cells.to_vec()))?;
        Ok(Snapshot { lines, rows, cursor })
    }
}
