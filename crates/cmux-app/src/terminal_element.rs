//! Paints the terminal grid.
//!
//! Adapted from Zed's terminal element (zed-industries/zed@69e2130,
//! crates/terminal_view/src/terminal_element.rs, GPL-3.0-or-later, Copyright Zed Industries):
//! batching column-contiguous same-style cells into one shaped run (`BatchedTextRun`), merging
//! same-colour backgrounds into rects (`BackgroundRegion`), measuring the grid and resizing the
//! terminal in prepaint, and the paint order. Rewritten for libghostty-vt cells.

use cmux_term::ghostty_vt::{CellWidth, CursorShape, UnderlineStyle as VtUnderline};
use cmux_term::selection::range_contains;
use gpui::{
    App, BorderStyle, Bounds, Element, ElementId, Entity, Font, FontStyle, FontWeight,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels, Point,
    StrikethroughStyle, Style, TextRun, UnderlineStyle, Window, fill, outline, point, px, relative,
    size,
};

use crate::colors::{self, CellColors};
use crate::style::cell_size;
use crate::terminal_view::{Geometry, TerminalView};

pub struct TerminalElement {
    view: Entity<TerminalView>,
}

impl TerminalElement {
    pub fn new(view: Entity<TerminalView>) -> Self {
        Self { view }
    }
}

impl IntoElement for TerminalElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

/// A run of same-colour background cells on one row.
struct Rect {
    row: u16,
    col: u16,
    cells: u16,
    color: Hsla,
}

/// Text shaped and painted as one line, starting at a cell.
struct Run {
    row: u16,
    col: u16,
    cells: u16,
    text: String,
    style: TextRun,
    /// Narrow single-codepoint cells only: `force_width` snaps glyphs, not cells, so wide
    /// and combining cells must stand alone to keep the grid aligned (research.md §1.3).
    batchable: bool,
}

struct CursorPaint {
    row: u16,
    col: u16,
    cells: u16,
    shape: CursorShape,
    color: Hsla,
    /// The glyph repainted in the cursor-text colour, for a solid block.
    glyph: Option<Run>,
}

pub struct Frame {
    origin: Point<Pixels>,
    cell_w: f32,
    cell_h: f32,
    font_size: Pixels,
    rects: Vec<Rect>,
    runs: Vec<Run>,
    cursor: Option<CursorPaint>,
    marked: Option<(u16, u16, String, Hsla)>,
}

fn same_style(a: &TextRun, b: &TextRun) -> bool {
    a.font == b.font
        && a.color == b.color
        && a.underline == b.underline
        && a.strikethrough == b.strikethrough
}

fn face(base: &Font, bold: bool, italic: bool) -> Font {
    let mut font = base.clone();
    if bold {
        font.weight = FontWeight::BOLD;
    }
    if italic {
        font.style = FontStyle::Italic;
    }
    font
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = Frame;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Frame {
        let view = self.view.read(cx);
        let style = view.style.clone();
        let term = view.session.term.clone();
        let focused = view.focus.is_focused(window);
        let blink_on = view.blink_on;
        let marked_text = view.marked_text.clone();
        let cfg = &style.cfg;

        // Measure the grid; resize the terminal and PTY when it changes (Zed does this in prepaint too).
        let font_size = px(style.font_px);
        let scale = window.scale_factor();
        let ts = window.text_system();
        let font_id = ts.resolve_font(&style.font);
        let advance = ts
            .advance(font_id, font_size, 'm')
            .map_or(style.font_px * 0.6, |s| f32::from(s.width));
        let ascent = f32::from(ts.ascent(font_id, font_size));
        let descent = f32::from(ts.descent(font_id, font_size));
        // shortcut: gpui 0.2.2 keeps FontMetrics.line_gap pub(crate) (text_system.rs:846, no
        // accessor; research.md §1), so the leading is taken as 0, which matches JetBrains Mono
        // and most monospace fonts. Pass the real value once gpui exposes it.
        let line_gap = 0.0;
        let (cell_w, cell_h) = cell_size(
            advance,
            ascent,
            descent,
            line_gap,
            scale,
            cfg.adjust_cell_height,
        );
        let cols = ((f32::from(bounds.size.width) / cell_w) as u16).max(1);
        let rows = ((f32::from(bounds.size.height) / cell_h) as u16).max(1);
        let geometry = Geometry {
            origin: bounds.origin,
            cell_w,
            cell_h,
            cols,
            rows,
            scale,
        };
        self.view.update(cx, |view, _| view.set_geometry(geometry));

        let mut rects: Vec<Rect> = Vec::new();
        let mut runs: Vec<Run> = Vec::new();
        let mut cursor = None;
        let mut marked = None;
        let mut cursor_cell = None;
        let mut cursor_blinks = false;
        let mut term = term.lock().unwrap();
        // Resolved once per frame: each `Selection::contains` would be two FFI lookups.
        let selection = self
            .view
            .read(cx)
            .selection
            .as_ref()
            .map(|s| s.range(&term));
        let selection_lost = matches!(selection, Some(None));
        let selection = selection.flatten();
        if let Ok(info) = term.update_frame() {
            let (dfg, dbg) = (info.default_fg, info.default_bg);
            let offset = info.scrollbar.map_or(0, |s| s.offset) as u32;
            let bg_alpha = if cfg.background_opacity_cells {
                cfg.background_opacity
            } else {
                1.0
            };
            let at_cursor = info.cursor.map(|c| (c.x, c.y));
            let mut under_cursor: Option<(String, Font, CellColors, u16)> = None;

            let _ = term.walk_rows(|row, cells| {
                let row = row as u16;
                for (i, cell) in cells.iter().enumerate() {
                    let col = i as u16;
                    if matches!(cell.width, CellWidth::SpacerTail | CellWidth::SpacerHead) {
                        continue;
                    }
                    let span = if cell.width == CellWidth::Wide { 2 } else { 1 };
                    let pre = colors::resolve(cell, dfg, dbg, cfg.faint_opacity, bg_alpha);
                    let abs_row = offset + u32::from(row);
                    // A range can end on a wide char's lead or start on its spacer tail.
                    let selected = selection.is_some_and(|r| {
                        range_contains(r, col, abs_row)
                            || (span == 2 && range_contains(r, col + 1, abs_row))
                    });
                    let c = if selected {
                        colors::selected(
                            pre,
                            dfg,
                            dbg,
                            cfg.selection_foreground,
                            cfg.selection_background,
                        )
                    } else {
                        pre
                    };
                    let font = face(&style.font, cell.bold, cell.italic);
                    if at_cursor == Some((col, row)) {
                        // The cursor picks its colours from the cell before selection, as Ghostty does.
                        under_cursor = Some((cell.text.clone(), font.clone(), pre, span));
                    }
                    if let Some(bg) = c.bg {
                        let color = colors::hsla(bg, c.bg_alpha);
                        match rects.last_mut() {
                            Some(r)
                                if r.row == row && r.col + r.cells == col && r.color == color =>
                            {
                                r.cells += span
                            }
                            _ => rects.push(Rect {
                                row,
                                col,
                                cells: span,
                                color,
                            }),
                        }
                    }
                    // Invisible (SGR 8) hides the glyph and its decorations (Ghostty generic.zig).
                    if cell.text.is_empty() || c.invisible {
                        continue;
                    }
                    let fg = colors::hsla(c.fg, c.fg_alpha);
                    let text_run = TextRun {
                        len: cell.text.len(),
                        font,
                        color: fg,
                        background_color: None,
                        underline: cell.underline.then_some(UnderlineStyle {
                            thickness: px(1.),
                            color: Some(fg),
                            wavy: cell.underline_style == Some(VtUnderline::Curly),
                        }),
                        strikethrough: cell.strikethrough.then_some(StrikethroughStyle {
                            thickness: px(1.),
                            color: Some(fg),
                        }),
                    };
                    let batchable = span == 1 && cell.text.chars().count() == 1;
                    match runs.last_mut() {
                        Some(run)
                            if batchable
                                && run.batchable
                                && run.row == row
                                && run.col + run.cells == col
                                && same_style(&run.style, &text_run) =>
                        {
                            run.text.push_str(&cell.text);
                            run.style.len += cell.text.len();
                            run.cells += 1;
                        }
                        _ => runs.push(Run {
                            row,
                            col,
                            cells: span,
                            text: cell.text.clone(),
                            style: text_run,
                            batchable,
                        }),
                    }
                }
            });

            cursor_cell = at_cursor;
            cursor_blinks = info.cursor.is_some_and(|c| c.blinking);
            if let Some(c) = info.cursor {
                let (text, font, cell_colors, span) = under_cursor.unwrap_or_else(|| {
                    (
                        String::new(),
                        style.font.clone(),
                        CellColors {
                            fg: dfg,
                            bg: None,
                            bg_alpha: 1.0,
                            fg_alpha: 1.0,
                            invisible: false,
                        },
                        1,
                    )
                });
                if !marked_text.is_empty() {
                    // While composing, the preedit replaces the cursor (Zed does the same).
                    marked = Some((c.x, c.y, marked_text, colors::hsla(cell_colors.fg, 1.0)));
                } else if !c.blinking || blink_on || !focused {
                    let (fill_rgb, glyph_rgb) = colors::cursor(
                        cell_colors,
                        dfg,
                        dbg,
                        info.cursor_color,
                        cfg.cursor_color,
                        cfg.cursor_text,
                    );
                    // Ghostty draws an unfocused cursor as a hollow block.
                    let shape = if focused {
                        c.shape
                    } else {
                        CursorShape::BlockHollow
                    };
                    let glyph = (shape == CursorShape::Block && !text.is_empty()).then(|| {
                        let style = TextRun {
                            len: text.len(),
                            font,
                            color: colors::hsla(glyph_rgb, 1.0),
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        };
                        Run {
                            row: c.y,
                            col: c.x,
                            cells: span,
                            text,
                            style,
                            batchable: false,
                        }
                    });
                    cursor = Some(CursorPaint {
                        row: c.y,
                        col: c.x,
                        cells: span,
                        shape,
                        color: colors::hsla(fill_rgb, 1.0),
                        glyph,
                    });
                }
            }
        }
        drop(term);
        self.view.update(cx, |view, _| {
            if selection_lost {
                // Lost rows: drop it to release its tracked points (selection.rs `range`).
                view.selection = None;
            }
            view.cursor_cell = cursor_cell;
            view.cursor_blinks = cursor_blinks;
        });

        Frame {
            origin: bounds.origin,
            cell_w,
            cell_h,
            font_size,
            rects,
            runs,
            cursor,
            marked,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _: &mut (),
        frame: &mut Frame,
        window: &mut Window,
        cx: &mut App,
    ) {
        let frame = &*frame;
        for r in &frame.rects {
            window.paint_quad(fill(frame.cell_bounds(r.col, r.row, r.cells), r.color));
        }
        for run in &frame.runs {
            frame.paint_run(run, window, cx);
        }
        if let Some(c) = &frame.cursor {
            let cell = frame.cell_bounds(c.col, c.row, c.cells);
            let thickness = px(2.);
            match c.shape {
                CursorShape::Block => window.paint_quad(fill(cell, c.color)),
                CursorShape::BlockHollow => {
                    window.paint_quad(outline(cell, c.color, BorderStyle::Solid))
                }
                CursorShape::Bar => window.paint_quad(fill(
                    Bounds::new(cell.origin, size(thickness, cell.size.height)),
                    c.color,
                )),
                CursorShape::Underline => {
                    let origin = point(cell.origin.x, cell.origin.y + cell.size.height - thickness);
                    window.paint_quad(fill(
                        Bounds::new(origin, size(cell.size.width, thickness)),
                        c.color,
                    ));
                }
            }
            if let Some(glyph) = &c.glyph {
                frame.paint_run(glyph, window, cx);
            }
        }
        if let Some((col, row, text, color)) = &frame.marked {
            let style = TextRun {
                len: text.len(),
                font: self.view.read(cx).style.font.clone(),
                color: *color,
                background_color: None,
                underline: Some(UnderlineStyle {
                    thickness: px(1.),
                    color: Some(*color),
                    wavy: false,
                }),
                strikethrough: None,
            };
            let run = Run {
                row: *row,
                col: *col,
                cells: 1,
                text: text.clone(),
                style,
                batchable: false,
            };
            frame.paint_run(&run, window, cx);
        }
        self.view.update(cx, |view, _| view.painted());
    }
}

impl Frame {
    fn cell_bounds(&self, col: u16, row: u16, cells: u16) -> Bounds<Pixels> {
        let origin = point(
            self.origin.x + px(f32::from(col) * self.cell_w),
            self.origin.y + px(f32::from(row) * self.cell_h),
        );
        Bounds::new(
            origin,
            size(px(f32::from(cells) * self.cell_w), px(self.cell_h)),
        )
    }

    fn paint_run(&self, run: &Run, window: &mut Window, cx: &mut App) {
        let origin = self.cell_bounds(run.col, run.row, run.cells).origin;
        let line = window.text_system().shape_line(
            run.text.clone().into(),
            self.font_size,
            std::slice::from_ref(&run.style),
            Some(px(self.cell_w)),
        );
        let _ = line.paint(origin, px(self.cell_h), window, cx);
    }
}
