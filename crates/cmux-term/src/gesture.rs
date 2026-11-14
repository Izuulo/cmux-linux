//! Ghostty's selection gesture rules (vendor/ghostty src/terminal/SelectionGesture.zig, MIT,
//! Copyright (c) 2024 Mitchell Hashimoto, Ghostty contributors): the within-cell threshold for
//! cell drags (`dragSelection`, non-rectangular) and the autoscroll edge (`drag`). Pure; the app
//! owns the timers.

use ghostty_vt::SelectionPoint;

/// Share of a cell's width the pointer must cross for that cell to join a drag selection.
const THRESHOLD: f32 = 0.6;
/// Ghostty's 1 px edge so fullscreen-edge drags still autoscroll.
const EDGE: f32 = 1.0;

fn key(p: SelectionPoint) -> (u32, u16) {
    (p.row, p.column)
}

fn right(p: SelectionPoint, cols: u16) -> SelectionPoint {
    if p.column + 1 < cols {
        SelectionPoint {
            column: p.column + 1,
            ..p
        }
    } else {
        SelectionPoint {
            column: 0,
            row: p.row + 1,
        }
    }
}

fn left(p: SelectionPoint, cols: u16) -> SelectionPoint {
    if p.column > 0 {
        SelectionPoint {
            column: p.column - 1,
            ..p
        }
    } else if p.row > 0 {
        SelectionPoint {
            column: cols - 1,
            row: p.row - 1,
        }
    } else {
        p
    }
}

/// The cells a cell-mode drag selects, ordered and inclusive, or `None` while no cell has been
/// crossed. `click_frac` and `drag_frac` are the pointer positions inside their cells, `0..1`.
pub fn cell_span(
    click: SelectionPoint,
    click_frac: f32,
    drag: SelectionPoint,
    drag_frac: f32,
    cols: u16,
) -> Option<(SelectionPoint, SelectionPoint)> {
    if cols == 0 {
        return None;
    }
    let same = click == drag;
    let backwards = if same {
        drag_frac < click_frac
    } else {
        key(drag) < key(click)
    };
    let include_click = if backwards {
        click_frac >= THRESHOLD
    } else {
        click_frac < THRESHOLD
    };
    let include_drag = if backwards {
        drag_frac < THRESHOLD
    } else {
        drag_frac >= THRESHOLD
    };
    let start = match (include_click, backwards) {
        (true, _) => click,
        (false, true) => left(click, cols),
        (false, false) => right(click, cols),
    };
    let end = match (include_drag, backwards) {
        (true, _) => drag,
        (false, true) => right(drag, cols),
        (false, false) => left(drag, cols),
    };
    if (!include_click && (same || end == click)) || (!include_drag && start == drag) {
        return None;
    }
    Some(if key(start) <= key(end) {
        (start, end)
    } else {
        (end, start)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Autoscroll {
    None,
    Up,
    Down,
}

/// Which way a selection drag at `y` (relative to the grid top) scrolls a grid `height` tall.
pub fn autoscroll(y: f32, height: f32) -> Autoscroll {
    if y <= EDGE {
        Autoscroll::Up
    } else if y > height - EDGE {
        Autoscroll::Down
    } else {
        Autoscroll::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(column: u16, row: u32) -> SelectionPoint {
        SelectionPoint { column, row }
    }

    #[test]
    fn same_cell_selects_only_after_crossing_sixty_percent() {
        assert_eq!(cell_span(p(3, 0), 0.1, p(3, 0), 0.5, 80), None);
        assert_eq!(
            cell_span(p(3, 0), 0.1, p(3, 0), 0.7, 80),
            Some((p(3, 0), p(3, 0)))
        );
    }

    #[test]
    fn rightward_drag_includes_cells_by_threshold() {
        assert_eq!(
            cell_span(p(2, 0), 0.8, p(6, 0), 0.2, 80),
            Some((p(3, 0), p(5, 0)))
        );
        assert_eq!(
            cell_span(p(2, 0), 0.1, p(6, 0), 0.9, 80),
            Some((p(2, 0), p(6, 0)))
        );
        // Adjacent cells, both excluded: nothing.
        assert_eq!(cell_span(p(2, 0), 0.8, p(3, 0), 0.2, 80), None);
    }

    #[test]
    fn leftward_drag_mirrors_the_rule() {
        assert_eq!(
            cell_span(p(6, 0), 0.2, p(2, 0), 0.8, 80),
            Some((p(3, 0), p(5, 0)))
        );
        assert_eq!(
            cell_span(p(6, 0), 0.9, p(2, 0), 0.1, 80),
            Some((p(2, 0), p(6, 0)))
        );
    }

    #[test]
    fn excluded_ends_wrap_across_rows() {
        assert_eq!(
            cell_span(p(79, 0), 0.9, p(5, 1), 0.9, 80),
            Some((p(0, 1), p(5, 1)))
        );
        assert_eq!(
            cell_span(p(2, 0), 0.1, p(0, 1), 0.1, 80),
            Some((p(2, 0), p(79, 0)))
        );
    }

    #[test]
    fn autoscroll_uses_a_one_pixel_edge() {
        assert_eq!(autoscroll(-20.0, 400.0), Autoscroll::Up);
        assert_eq!(autoscroll(1.0, 400.0), Autoscroll::Up);
        assert_eq!(autoscroll(200.0, 400.0), Autoscroll::None);
        assert_eq!(autoscroll(399.5, 400.0), Autoscroll::Down);
    }
}
