//! Paints the terminal grid.
//!
//! Adapted from Zed's terminal element (zed-industries/zed@69e2130,
//! crates/terminal_view/src/terminal_element.rs, GPL-3.0-or-later, Copyright Zed Industries):
//! batching column-contiguous same-style cells into one shaped run (`BatchedTextRun`), merging
//! same-colour backgrounds into rects (`BackgroundRegion`), measuring the grid and resizing the
//! terminal in prepaint, and the paint order. Rewritten for libghostty-vt cells.

use cmux_term::ghostty_vt::{
    CellWidth, CursorShape, SelectionRange, UnderlineStyle as VtUnderline,
};
use cmux_term::selection::range_contains;
use cmux_term::{FrameInfo, Term};
use gpui::{
    App, BorderStyle, Bounds, DispatchPhase, Element, ElementId, ElementInputHandler, Entity, Font,
    FontStyle, FontWeight, GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId,
    MouseButton, MouseMoveEvent, Pixels, Point, StrikethroughStyle, Style, TextRun, UnderlineStyle,
    Window, fill, outline, point, px, relative, size,
};

use crate::colors::{self, CellColors};
use crate::style::{TermStyle, cell_size};
use crate::terminal_view::{Geometry, TerminalView, snap_to_device};

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
    /// Drawn as quads by `underline_marks`, except curly (gpui's `wavy`, in `style`).
    underline: Option<VtUnderline>,
    /// Index into `collect_cells`' faces (bit 0 bold, bit 1 italic), to extend a run cheaply.
    face: usize,
    /// Narrow single-codepoint cells only. gpui's `force_width` moves glyph n to n * width, so
    /// wide and combining cells stand alone and get no `force_width` (a combining mark would
    /// otherwise land in the next column); each is painted at its own cell origin instead.
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
    /// Top of a single underline, from the cell top (where gpui puts its own underline).
    underline_y: f32,
    rects: Vec<Rect>,
    runs: Vec<Run>,
    cursor: Option<CursorPaint>,
    marked: Option<Marked>,
}

fn same_style(a: &TextRun, b: &TextRun) -> bool {
    a.font == b.font
        && a.color == b.color
        && a.underline == b.underline
        && a.strikethrough == b.strikethrough
}

/// gpui's `force_width` snaps glyph n to n * width, which is right for a batch of narrow cells
/// and wrong for a cell holding a base plus combining marks or a wide glyph.
fn run_force_width(batchable: bool, cell_w: f32) -> Option<Pixels> {
    batchable.then(|| px(cell_w))
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

        // Measure the grid; resize the terminal and PTY when it changes (Zed does this in prepaint too).
        let (geometry, underline_y) = measure(window, &style, bounds);
        self.view.update(cx, |view, _| view.set_geometry(geometry));

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
        let mut cells = Cells::default();
        let mut cursor = (None, None);
        let mut cursor_cell = None;
        let mut cursor_blinks = false;
        if let Ok(info) = term.update_frame() {
            cells = collect_cells(&mut term, &info, &style, selection);
            cursor_cell = info.cursor.map(|c| (c.x, c.y));
            cursor_blinks = info.cursor.is_some_and(|c| c.blinking);
            let state = CursorState {
                focused,
                blink_on,
                marked_text,
            };
            cursor = plan_cursor(&info, cells.under_cursor.take(), &style, state);
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
            origin: geometry.origin,
            cell_w: geometry.cell_w,
            cell_h: geometry.cell_h,
            font_size: px(style.font_px),
            underline_y,
            rects: cells.rects,
            runs: cells.runs,
            cursor: cursor.0,
            marked: cursor.1,
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        frame: &mut Frame,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.view.read(cx).focus.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.view.clone()),
            cx,
        );
        // gpui sends on_mouse_move only while hovered; a selection drag that leaves the pane
        // (autoscroll, other panes) still has to reach us.
        let view = self.view.clone();
        window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
            if phase == DispatchPhase::Bubble
                && !bounds.contains(&ev.position)
                && ev.pressed_button == Some(MouseButton::Left)
            {
                view.update(cx, |v, cx| {
                    if v.dragging && !v.tracking() {
                        v.drag_to(ev.position, cx);
                    }
                });
            }
        });
        let frame = &*frame;
        for r in &frame.rects {
            window.paint_quad(fill(frame.cell_bounds(r.col, r.row, r.cells), r.color));
        }
        for run in &frame.runs {
            frame.paint_run(run, window, cx);
        }
        if let Some(c) = &frame.cursor {
            frame.paint_cursor(c, window, cx);
        }
        if let Some(marked) = &frame.marked {
            let font = self.view.read(cx).style.font.clone();
            frame.paint_marked(marked, font, window, cx);
        }
        self.view.update(cx, |view, _| view.painted());
    }
}

/// Grid geometry for `bounds`: cell size snapped to device pixels, whole cols/rows, and the
/// origin on a device pixel. Also the underline offset from the cell top.
fn measure(window: &mut Window, style: &TermStyle, bounds: Bounds<Pixels>) -> (Geometry, f32) {
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
        style.cfg.adjust_cell_height,
    );
    let cols = ((f32::from(bounds.size.width) / cell_w) as u16).max(1);
    let rows = ((f32::from(bounds.size.height) / cell_h) as u16).max(1);
    // gpui's own underline position (text_system/line.rs): baseline + 0.618 descent.
    let underline_y = (cell_h - ascent - descent) / 2.0 + ascent + descent * 0.618;
    let g = Geometry {
        origin: point(
            px(snap_to_device(f32::from(bounds.origin.x), scale)),
            px(snap_to_device(f32::from(bounds.origin.y), scale)),
        ),
        cell_w,
        cell_h,
        cols,
        rows,
        scale,
    };
    (g, underline_y)
}

/// The cell under the cursor, as the cursor needs it: text, face, pre-selection colours, width.
struct UnderCursor {
    text: String,
    font: Font,
    colors: CellColors,
    span: u16,
}

#[derive(Default)]
struct Cells {
    rects: Vec<Rect>,
    runs: Vec<Run>,
    under_cursor: Option<UnderCursor>,
}

/// Extends the last rect when `color` continues it on the same row, else starts a new one.
fn push_rect(rects: &mut Vec<Rect>, row: u16, col: u16, span: u16, color: Hsla) {
    match rects.last_mut() {
        Some(r) if r.row == row && r.col + r.cells == col && r.color == color => r.cells += span,
        _ => rects.push(Rect {
            row,
            col,
            cells: span,
            color,
        }),
    }
}

/// Appends `run` to the last run when both are batchable, adjacent and the same style.
fn push_run(runs: &mut Vec<Run>, run: Run) {
    match runs.last_mut() {
        Some(last)
            if run.batchable
                && last.batchable
                && last.row == run.row
                && last.col + last.cells == run.col
                && last.underline == run.underline
                && same_style(&last.style, &run.style) =>
        {
            last.text.push_str(&run.text);
            last.style.len += run.style.len;
            last.cells += run.cells;
        }
        _ => runs.push(run),
    }
}

/// Walks the viewport once: merged background rects, batched text runs, and the cursor cell.
fn collect_cells(
    term: &mut Term,
    info: &FrameInfo,
    style: &TermStyle,
    selection: Option<SelectionRange>,
) -> Cells {
    let cfg = &style.cfg;
    let (dfg, dbg) = (info.default_fg, info.default_bg);
    let offset = info.scrollbar.map_or(0, |s| s.offset) as u32;
    let bg_alpha = if cfg.background_opacity_cells {
        cfg.background_opacity
    } else {
        1.0
    };
    let at_cursor = info.cursor.map(|c| (c.x, c.y));
    // Once per frame; cells index them by `face_i`.
    let faces = [
        face(&style.font, false, false),
        face(&style.font, true, false),
        face(&style.font, false, true),
        face(&style.font, true, true),
    ];
    let mut out = Cells::default();
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
            let face_i = usize::from(cell.bold) | usize::from(cell.italic) << 1;
            if at_cursor == Some((col, row)) {
                // The cursor picks its colours from the cell before selection, as Ghostty does.
                // SGR 8 text stays hidden under a block cursor.
                let text = if c.invisible {
                    String::new()
                } else {
                    cell.text.clone()
                };
                out.under_cursor = Some(UnderCursor {
                    text,
                    font: faces[face_i].clone(),
                    colors: pre,
                    span,
                });
            }
            if let Some(bg) = c.bg {
                push_rect(&mut out.rects, row, col, span, colors::hsla(bg, c.bg_alpha));
            }
            // Invisible (SGR 8) hides the glyph and its decorations (Ghostty generic.zig).
            if cell.text.is_empty() || c.invisible {
                continue;
            }
            let fg = colors::hsla(c.fg, c.fg_alpha);
            let underline = cell
                .underline
                .then(|| cell.underline_style.unwrap_or(VtUnderline::Single));
            let batchable = span == 1 && cell.text.chars().count() == 1;
            // Extend the last run in place when nothing changes: no TextRun, no Font clone.
            if batchable
                && let Some(last) = out.runs.last_mut()
                && last.batchable
                && last.row == row
                && last.col + last.cells == col
                && last.face == face_i
                && last.style.color == fg
                && last.underline == underline
                && last.style.strikethrough.is_some() == cell.strikethrough
            {
                last.text.push_str(&cell.text);
                last.style.len += cell.text.len();
                last.cells += 1;
                continue;
            }
            let text_run = TextRun {
                len: cell.text.len(),
                font: faces[face_i].clone(),
                color: fg,
                background_color: None,
                underline: (underline == Some(VtUnderline::Curly)).then_some(UnderlineStyle {
                    thickness: px(1.),
                    color: Some(fg),
                    wavy: true,
                }),
                strikethrough: cell.strikethrough.then_some(StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(fg),
                }),
            };
            push_run(
                &mut out.runs,
                Run {
                    row,
                    col,
                    cells: span,
                    text: cell.text.clone(),
                    style: text_run,
                    underline,
                    face: face_i,
                    batchable,
                },
            );
        }
    });
    out
}

/// Ghostty's underline sprites (font/sprite/draw/special.zig) as quads with a corner radius:
/// `y` is the top of a single bar, `t` its thickness. Curly is drawn by gpui (`wavy`).
fn underline_marks(
    style: VtUnderline,
    x: f32,
    w: f32,
    y: f32,
    t: f32,
) -> Vec<(Bounds<Pixels>, f32)> {
    let bar =
        |bx: f32, by: f32, bw: f32| (Bounds::new(point(px(bx), px(by)), size(px(bw), px(t))), 0.0);
    match style {
        VtUnderline::Double => vec![bar(x, y - t, w), bar(x, y + t, w)],
        VtUnderline::Dashed => {
            let dash = (w / 3.0).floor() + 1.0;
            let count = (w / dash).floor() as u32 + 1;
            (0..count)
                .step_by(2)
                .map(|i| i as f32 * dash)
                .filter(|dx| *dx < w)
                .map(|dx| bar(x + dx, y, dash.min(w - dx)))
                .collect()
        }
        VtUnderline::Dotted => {
            // Dots of diameter sqrt(2) * t, as many as fit with gaps of about one diameter.
            let r = std::f32::consts::FRAC_1_SQRT_2 * t;
            let n = (w / (4.0 * r))
                .ceil()
                .min((w / (3.0 * r)).floor())
                .min((w / (2.0 * r + 1.0)).floor())
                .max(1.0);
            let gap = w / n;
            (0..n as u32)
                .map(|i| {
                    let cx = x + gap * (i as f32 + 0.5);
                    let origin = point(px(cx - r), px(y + t / 2.0 - r));
                    (Bounds::new(origin, size(px(2.0 * r), px(2.0 * r))), r)
                })
                .collect()
        }
        VtUnderline::Single | VtUnderline::Curly => vec![bar(x, y, w)],
    }
}

/// What the cursor depends on besides the terminal.
struct CursorState {
    focused: bool,
    blink_on: bool,
    marked_text: String,
}

type Marked = (u16, u16, String, Hsla);

/// The cursor to paint (none while blinked off or composing) and the IME preedit, if any.
fn plan_cursor(
    info: &FrameInfo,
    under: Option<UnderCursor>,
    style: &TermStyle,
    state: CursorState,
) -> (Option<CursorPaint>, Option<Marked>) {
    let Some(c) = info.cursor else {
        return (None, None);
    };
    let cfg = &style.cfg;
    let (dfg, dbg) = (info.default_fg, info.default_bg);
    let under = under.unwrap_or_else(|| UnderCursor {
        text: String::new(),
        font: style.font.clone(),
        colors: CellColors {
            fg: dfg,
            bg: None,
            bg_alpha: 1.0,
            fg_alpha: 1.0,
            invisible: false,
        },
        span: 1,
    });
    if !state.marked_text.is_empty() {
        // While composing, the preedit replaces the cursor (Zed does the same).
        let color = colors::hsla(under.colors.fg, 1.0);
        return (None, Some((c.x, c.y, state.marked_text, color)));
    }
    if c.blinking && !state.blink_on && state.focused {
        return (None, None);
    }
    let (fill_rgb, glyph_rgb) = colors::cursor(
        under.colors,
        dfg,
        dbg,
        info.cursor_color,
        cfg.cursor_color,
        cfg.cursor_text,
    );
    // Ghostty draws an unfocused cursor as a hollow block.
    let shape = if state.focused {
        c.shape
    } else {
        CursorShape::BlockHollow
    };
    let glyph = (shape == CursorShape::Block && !under.text.is_empty()).then(|| Run {
        row: c.y,
        col: c.x,
        cells: under.span,
        style: TextRun {
            len: under.text.len(),
            font: under.font,
            color: colors::hsla(glyph_rgb, 1.0),
            background_color: None,
            underline: None,
            strikethrough: None,
        },
        text: under.text,
        underline: None,
        face: 0,
        batchable: false,
    });
    let paint = CursorPaint {
        row: c.y,
        col: c.x,
        cells: under.span,
        shape,
        color: colors::hsla(fill_rgb, 1.0),
        glyph,
    };
    (Some(paint), None)
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

    fn paint_cursor(&self, c: &CursorPaint, window: &mut Window, cx: &mut App) {
        let cell = self.cell_bounds(c.col, c.row, c.cells);
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
            self.paint_run(glyph, window, cx);
        }
    }

    /// The IME preedit at the cursor, underlined in the cell's foreground.
    fn paint_marked(&self, marked: &Marked, font: Font, window: &mut Window, cx: &mut App) {
        let (col, row, text, color) = marked;
        let style = TextRun {
            len: text.len(),
            font,
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
            underline: None,
            face: 0,
            batchable: false,
        };
        self.paint_run(&run, window, cx);
    }

    fn paint_run(&self, run: &Run, window: &mut Window, cx: &mut App) {
        let origin = self.cell_bounds(run.col, run.row, run.cells).origin;
        let line = window.text_system().shape_line(
            run.text.clone().into(),
            self.font_size,
            std::slice::from_ref(&run.style),
            run_force_width(run.batchable, self.cell_w),
        );
        let _ = line.paint(origin, px(self.cell_h), window, cx);
        if let Some(u) = run.underline.filter(|u| *u != VtUnderline::Curly) {
            let cell = self.cell_bounds(run.col, run.row, run.cells);
            let marks = underline_marks(
                u,
                f32::from(cell.origin.x),
                f32::from(cell.size.width),
                f32::from(cell.origin.y) + self.underline_y,
                1.0,
            );
            for (b, r) in marks {
                window.paint_quad(fill(b, run.style.color).corner_radii(px(r)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_batchable_runs_force_the_cell_width() {
        assert_eq!(run_force_width(true, 9.5), Some(px(9.5)));
        // `e` + U+0301 must keep its accent in the same cell.
        assert_eq!(run_force_width(false, 9.5), None);
    }

    fn run(col: u16, text: &str, batchable: bool) -> Run {
        Run {
            row: 0,
            col,
            cells: 1,
            text: text.into(),
            style: TextRun {
                len: text.len(),
                font: gpui::font("JetBrains Mono"),
                color: gpui::black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            },
            underline: None,
            face: 0,
            batchable,
        }
    }

    fn ys(m: &[(Bounds<Pixels>, f32)]) -> Vec<f32> {
        m.iter().map(|(b, _)| f32::from(b.origin.y)).collect()
    }

    #[test]
    fn single_and_double_bars() {
        let single = underline_marks(VtUnderline::Single, 0., 10., 14., 1.);
        assert_eq!(ys(&single), vec![14.]);
        let double = underline_marks(VtUnderline::Double, 0., 10., 14., 1.);
        assert_eq!(ys(&double), vec![13., 15.]);
    }

    #[test]
    fn dashes_follow_ghostty_widths() {
        // w = 10: dash 4 px, count 3, every other one: at 0 and 8, the last clipped to w.
        let m = underline_marks(VtUnderline::Dashed, 0., 10., 14., 1.);
        let xs: Vec<_> = m
            .iter()
            .map(|(b, _)| (f32::from(b.origin.x), f32::from(b.size.width)))
            .collect();
        assert_eq!(xs, vec![(0., 4.), (8., 2.)]);
    }

    #[test]
    fn dots_are_round_and_at_least_one() {
        let m = underline_marks(VtUnderline::Dotted, 0., 10., 14., 1.);
        assert!(!m.is_empty());
        assert!(
            m.iter()
                .all(|(b, r)| (r * 2. - f32::from(b.size.width)).abs() < 1e-5)
        );
        assert_eq!(
            underline_marks(VtUnderline::Dotted, 0., 1., 14., 1.).len(),
            1
        );
    }

    #[test]
    fn adjacent_same_colour_backgrounds_merge() {
        let mut rects = Vec::new();
        push_rect(&mut rects, 0, 0, 1, gpui::red());
        push_rect(&mut rects, 0, 1, 2, gpui::red());
        push_rect(&mut rects, 0, 3, 1, gpui::blue());
        push_rect(&mut rects, 1, 4, 1, gpui::blue());
        let got: Vec<_> = rects.iter().map(|r| (r.row, r.col, r.cells)).collect();
        assert_eq!(got, vec![(0, 0, 3), (0, 3, 1), (1, 4, 1)]);
    }

    #[test]
    fn batchable_neighbours_join_one_run() {
        let mut runs = Vec::new();
        push_run(&mut runs, run(0, "a", true));
        push_run(&mut runs, run(1, "b", true));
        push_run(&mut runs, run(2, "é", false));
        push_run(&mut runs, run(3, "c", true));
        push_run(&mut runs, run(5, "d", true));
        let got: Vec<_> = runs
            .iter()
            .map(|r| (r.col, r.text.as_str(), r.style.len))
            .collect();
        assert_eq!(
            got,
            vec![(0, "ab", 2), (2, "é", 2), (3, "c", 1), (5, "d", 1)]
        );
    }
}
