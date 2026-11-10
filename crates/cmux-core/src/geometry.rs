//! Pixel layout of the split tree, divider drag, keyboard resize, equalize and split admission.
//! Numbers and rules follow Bonsplit/cmux (research-cmux.md §1 "Minimum size admission",
//! "Equalize", §3 resize notes, §4 "Dividers", §5); cmux's own versions live in
//! `cmux/Packages/macOS/CmuxPanes/Sources/CmuxPanes/Geometry/` (GPL-3.0-or-later).

use crate::tree::{
    Direction, Layout, MAX_RATIO, MIN_RATIO, Node, Orientation, PaneId, Rect, Split, SplitId,
    SurfaceId,
};

/// Divider thickness in logical pixels (Bonsplit `dividerThickness` default).
pub const DIVIDER: f32 = 1.0;
/// Smallest pane side a divider drag leaves (Bonsplit `minimumPaneWidth/Height`).
pub const MIN_DRAG_SIZE: f32 = 100.0;
/// Smallest pane a new split may create (cmux `Workspace+SplitSpace.swift`): 160 wide, and
/// max(100, tab strip 28 + 3 rows × 17) = 100 tall.
pub const MIN_SPLIT_WIDTH: f32 = 160.0;
pub const MIN_SPLIT_HEIGHT: f32 = 100.0;
/// cmux `app.paneResizeStepPixels` default.
pub const RESIZE_STEP: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DividerRect {
    pub split: SplitId,
    /// The split's orientation: a `Horizontal` split has a vertical divider line.
    pub orientation: Orientation,
    /// The 1 px line.
    pub rect: Rect,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rects {
    pub panes: Vec<(PaneId, Rect)>,
    pub dividers: Vec<DividerRect>,
}

/// A split refused because a pane would end up below the minimum size (cmux beeps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoSpace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Fits,
    FitsAfterEqualizingRun,
    NoSpace,
}

/// A usable device scale: non-finite or non-positive scales fall back to 1.
fn sane(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

fn snap(v: f32, scale: f32) -> f32 {
    let scale = sane(scale);
    (v * scale).round() / scale
}

/// The divider's thickness: whole device pixels (at least one), never more than `len`.
fn divider_len(len: f32, scale: f32) -> f32 {
    let scale = sane(scale);
    ((DIVIDER * scale).round().max(1.0) / scale).min(len.max(0.0))
}

fn clamp_ratio(r: f32) -> f32 {
    r.clamp(MIN_RATIO, MAX_RATIO)
}

/// Splits `r` along `o`: `first` gets `ratio` of the space left after the divider. The
/// divider's absolute position is snapped to a device pixel, so nested splits and fractional
/// scales stay aligned when `r` is. Returns (first, divider, second).
fn divide(r: Rect, o: Orientation, ratio: f32, scale: f32) -> (Rect, Rect, Rect) {
    let (start, len) = r.along(o);
    let line = divider_len(len, scale);
    let avail = (len - line).max(0.0);
    let first = (snap(start + avail * ratio, scale) - start).clamp(0.0, avail);
    let second = avail - first;
    match o {
        Orientation::Horizontal => (
            Rect { w: first, ..r },
            Rect {
                x: r.x + first,
                w: line,
                ..r
            },
            Rect {
                x: r.x + first + line,
                w: second,
                ..r
            },
        ),
        Orientation::Vertical => (
            Rect { h: first, ..r },
            Rect {
                y: r.y + first,
                h: line,
                ..r
            },
            Rect {
                y: r.y + first + line,
                h: second,
                ..r
            },
        ),
    }
}

impl Node {
    fn layout(&self, r: Rect, scale: f32, out: &mut Rects) {
        match self {
            Node::Pane(p) => out.panes.push((p.id, r)),
            Node::Split(s) => {
                let (a, line, b) = divide(r, s.orientation, s.ratio, scale);
                s.first.layout(a, scale, out);
                out.dividers.push(DividerRect {
                    split: s.id,
                    orientation: s.orientation,
                    rect: line,
                });
                s.second.layout(b, scale, out);
            }
        }
    }

    /// The rect a split occupies (both children and its divider).
    fn split_rect(&self, r: Rect, id: SplitId, scale: f32) -> Option<Rect> {
        let Node::Split(s) = self else { return None };
        if s.id == id {
            return Some(r);
        }
        let (a, _, b) = divide(r, s.orientation, s.ratio, scale);
        s.first
            .split_rect(a, id, scale)
            .or_else(|| s.second.split_rect(b, id, scale))
    }

    fn find_split(&self, id: SplitId) -> Option<&Split> {
        let Node::Split(s) = self else { return None };
        if s.id == id {
            return Some(s);
        }
        s.first.find_split(id).or_else(|| s.second.find_split(id))
    }

    fn find_split_mut(&mut self, id: SplitId) -> Option<&mut Split> {
        let Node::Split(s) = self else { return None };
        if s.id == id {
            return Some(s);
        }
        // Checked first: returning the borrow from a failed `if let` trips the borrow checker.
        if s.first.find_split(id).is_some() {
            return s.first.find_split_mut(id);
        }
        s.second.find_split_mut(id)
    }

    /// Splits from here down to pane `id`'s parent, each with "the pane is in its first child".
    fn path_to_pane(&self, id: PaneId) -> Option<Vec<(SplitId, bool)>> {
        match self {
            Node::Pane(p) => (p.id == id).then(Vec::new),
            Node::Split(s) => {
                let (in_first, rest) = match s.first.path_to_pane(id) {
                    Some(rest) => (true, rest),
                    None => (false, s.second.path_to_pane(id)?),
                };
                let mut path = vec![(s.id, in_first)];
                path.extend(rest);
                Some(path)
            }
        }
    }

    /// Splits from here down to split `id`, inclusive.
    fn path_to_split(&self, id: SplitId) -> Option<Vec<SplitId>> {
        let Node::Split(s) = self else { return None };
        if s.id == id {
            return Some(vec![s.id]);
        }
        let rest = s
            .first
            .path_to_split(id)
            .or_else(|| s.second.path_to_split(id))?;
        let mut path = vec![s.id];
        path.extend(rest);
        Some(path)
    }

    /// Leaves along `o`; a child split of the other orientation counts as one.
    fn spans(&self, o: Orientation) -> usize {
        match self {
            Node::Split(s) if s.orientation == o => s.first.spans(o) + s.second.spans(o),
            _ => 1,
        }
    }

    /// Post-order: every split gets `spans(first) / spans(both)` along its own axis.
    fn equalize_all(&mut self) {
        if let Node::Split(s) = self {
            s.first.equalize_all();
            s.second.equalize_all();
            let (a, b) = (s.first.spans(s.orientation), s.second.spans(s.orientation));
            s.ratio = clamp_ratio(a as f32 / (a + b) as f32);
        }
    }

    /// Like `equalize_all`, but only through splits of orientation `o`.
    fn equalize_run(&mut self, o: Orientation) {
        if let Node::Split(s) = self
            && s.orientation == o
        {
            s.first.equalize_run(o);
            s.second.equalize_run(o);
            let (a, b) = (s.first.spans(o), s.second.spans(o));
            s.ratio = clamp_ratio(a as f32 / (a + b) as f32);
        }
    }

    /// Leaves along `o` counting `splitting` as two (the split about to happen).
    fn prospective_spans(&self, o: Orientation, splitting: PaneId) -> usize {
        match self {
            Node::Pane(p) if p.id == splitting => 2,
            Node::Split(s) if s.orientation == o => {
                s.first.prospective_spans(o, splitting) + s.second.prospective_spans(o, splitting)
            }
            _ => 1,
        }
    }

    /// cmux `fitsAfterEqualizingRun`: whether every slot of the run still gets `min` once the
    /// run is equalized with `splitting` split in two.
    fn fits_after_equalizing(
        &self,
        extent: f32,
        o: Orientation,
        splitting: PaneId,
        min: f32,
    ) -> bool {
        match self {
            Node::Pane(p) if p.id == splitting => (extent - DIVIDER) / 2.0 >= min,
            Node::Pane(_) => extent >= min,
            Node::Split(s) if s.orientation != o => self.required_extent(o, min) <= extent,
            Node::Split(s) => {
                let a = s.first.prospective_spans(o, splitting);
                let b = s.second.prospective_spans(o, splitting);
                let avail = extent - DIVIDER;
                if avail < 0.0 {
                    return false;
                }
                let pos = clamp_ratio(a as f32 / (a + b) as f32);
                s.first
                    .fits_after_equalizing(avail * pos, o, splitting, min)
                    && s.second
                        .fits_after_equalizing(avail * (1.0 - pos), o, splitting, min)
            }
        }
    }

    /// The smallest extent along `o` that keeps every pane here at `min`, at current ratios.
    fn required_extent(&self, o: Orientation, min: f32) -> f32 {
        match self {
            Node::Pane(_) => min,
            Node::Split(s) => {
                let a = s.first.required_extent(o, min);
                let b = s.second.required_extent(o, min);
                if s.orientation != o {
                    return a.max(b);
                }
                let pos = clamp_ratio(s.ratio);
                (a / pos).max(b / (1.0 - pos)) + DIVIDER
            }
        }
    }
}

fn full(width: f32, height: f32) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: width,
        h: height,
    }
}

impl Layout {
    /// Pane and divider rects for a `width` × `height` container (logical pixels; divider
    /// positions snapped to `scale`). A zoomed pane fills the container alone.
    pub fn rects(&self, width: f32, height: f32, scale: f32) -> Rects {
        if let Some(z) = self.zoomed() {
            return Rects {
                panes: vec![(z, full(width, height))],
                dividers: Vec::new(),
            };
        }
        self.unzoomed_rects(width, height, scale)
    }

    fn unzoomed_rects(&self, width: f32, height: f32, scale: f32) -> Rects {
        let mut out = Rects::default();
        self.root.layout(full(width, height), scale, &mut out);
        out
    }

    pub fn ratio(&self, split: SplitId) -> Option<f32> {
        self.root.find_split(split).map(|s| s.ratio)
    }

    /// Splits the focused pane if both halves keep the minimum size, borrowing room by
    /// equalizing the same-orientation run when needed (cmux #15371); else `Err(NoSpace)`.
    pub fn try_split(
        &mut self,
        o: Orientation,
        width: f32,
        height: f32,
        scale: f32,
    ) -> Result<SurfaceId, NoSpace> {
        let dir = match o {
            Orientation::Horizontal => Direction::Right,
            Orientation::Vertical => Direction::Down,
        };
        self.try_split_surface(self.active_surface(), dir, true, width, height, scale)
    }

    /// `try_split` for the pane holding `surface` (the socket's `surface.split`): the new pane
    /// goes before it for `Left`/`Up`, after it for `Right`/`Down`, and takes focus only when
    /// `focus`. An unknown surface is `Err(NoSpace)`; callers resolve it first.
    pub fn try_split_surface(
        &mut self,
        surface: SurfaceId,
        dir: Direction,
        focus: bool,
        width: f32,
        height: f32,
        scale: f32,
    ) -> Result<SurfaceId, NoSpace> {
        let Some(pane) = self.pane_of(surface) else {
            return Err(NoSpace);
        };
        let o = dir.orientation();
        let verdict = self.split_verdict(pane, o, width, height, scale);
        if verdict == Verdict::NoSpace {
            return Err(NoSpace);
        }
        let new = self.split_pane(pane, o, !dir.forward(), focus);
        if verdict == Verdict::FitsAfterEqualizingRun
            && let Some(path) = self.root.path_to_pane(pane)
            && let Some((parent, _)) = path.last()
        {
            self.equalize_run(*parent);
        }
        Ok(new)
    }

    /// cmux `splitSpaceVerdict` for splitting `pane` along `o`.
    fn split_verdict(
        &self,
        pane: PaneId,
        o: Orientation,
        width: f32,
        height: f32,
        scale: f32,
    ) -> Verdict {
        let min = match o {
            Orientation::Horizontal => MIN_SPLIT_WIDTH,
            Orientation::Vertical => MIN_SPLIT_HEIGHT,
        };
        let rects = self.unzoomed_rects(width, height, scale);
        let Some((_, rect)) = rects.panes.iter().find(|(id, _)| *id == pane) else {
            return Verdict::Fits;
        };
        let extent = rect.along(o).1;
        if extent <= 0.0 {
            // Not measured yet: allow rather than refuse on unknown geometry.
            return Verdict::Fits;
        }
        if (extent - DIVIDER) / 2.0 >= min {
            return Verdict::Fits;
        }
        let path = self.root.path_to_pane(pane).unwrap_or_default();
        let same = |id: SplitId| self.root.find_split(id).is_some_and(|s| s.orientation == o);
        let Some(&(parent, _)) = path.last() else {
            return Verdict::NoSpace;
        };
        if !same(parent) {
            return Verdict::NoSpace;
        }
        let mut run_root = parent;
        for &(ancestor, _) in path.iter().rev().skip(1) {
            if !same(ancestor) {
                break;
            }
            run_root = ancestor;
        }
        let root_rect = full(width, height);
        let run_extent = self
            .root
            .split_rect(root_rect, run_root, scale)
            .map_or(0.0, |r| r.along(o).1);
        let run = Node::Split(self.root.find_split(run_root).expect("on the path").clone());
        if run.fits_after_equalizing(run_extent, o, pane, min) {
            Verdict::FitsAfterEqualizingRun
        } else {
            Verdict::NoSpace
        }
    }

    /// Divider drag: puts `split`'s divider under `pos` (container coordinates along the
    /// split's axis), keeping each side at least 100 px (or half the space when smaller) and
    /// the ratio in 0.1..0.9. Returns whether the ratio changed.
    pub fn drag_divider(
        &mut self,
        split: SplitId,
        pos: f32,
        width: f32,
        height: f32,
        scale: f32,
    ) -> bool {
        let Some(rect) = self.root.split_rect(full(width, height), split, scale) else {
            return false;
        };
        let s = self
            .root
            .find_split_mut(split)
            .expect("split_rect found it");
        let (start, len) = rect.along(s.orientation);
        let avail = len - divider_len(len, scale);
        // NaN would panic in `clamp` or poison the ratio.
        if avail.is_nan() || avail <= 0.0 || !pos.is_finite() {
            return false;
        }
        let min_side = MIN_DRAG_SIZE.min(avail / 2.0);
        let first = (pos - start).clamp(min_side, avail - min_side);
        let ratio = clamp_ratio(first / avail);
        let changed = ratio != s.ratio;
        s.ratio = ratio;
        changed
    }

    /// cmux keyboard resize (`resizeDividerAdjustment`): moves the divider of the innermost
    /// enclosing split of `dir`'s orientation that has the focused pane on the matching side
    /// (first child for Right/Down, second for Left/Up) by `step` px, clamped to 0.1..0.9.
    /// Returns false when no divider moves (cmux beeps).
    pub fn resize_pane(
        &mut self,
        dir: Direction,
        step: f32,
        width: f32,
        height: f32,
        scale: f32,
    ) -> bool {
        if !step.is_finite() {
            return false;
        }
        let o = dir.orientation();
        let Some(path) = self.root.path_to_pane(self.focused()) else {
            return false;
        };
        let Some(&(split, _)) = path.iter().rev().find(|(id, in_first)| {
            *in_first == dir.forward()
                && self
                    .root
                    .find_split(*id)
                    .is_some_and(|s| s.orientation == o)
        }) else {
            return false;
        };
        let axis = self
            .root
            .split_rect(full(width, height), split, scale)
            .map_or(1.0, |r| r.along(o).1.max(1.0));
        let s = self.root.find_split_mut(split).expect("on the path");
        let sign = if dir.forward() { 1.0 } else { -1.0 };
        let ratio = clamp_ratio(s.ratio + sign * step / axis);
        let changed = ratio != s.ratio;
        s.ratio = ratio;
        changed
    }

    /// cmux `equalizeSplits`: every split, by leaf count along its axis.
    pub fn equalize_all(&mut self) {
        self.root.equalize_all();
    }

    /// Divider double-click: equalizes the run of same-orientation splits containing `split`
    /// (up through same-orientation ancestors, down through same-orientation descendants).
    pub fn equalize_run(&mut self, split: SplitId) -> bool {
        let Some(path) = self.root.path_to_split(split) else {
            return false;
        };
        let o = self
            .root
            .find_split(split)
            .expect("on the path")
            .orientation;
        let mut run_root = split;
        for &ancestor in path.iter().rev().skip(1) {
            if self
                .root
                .find_split(ancestor)
                .is_none_or(|s| s.orientation != o)
            {
                break;
            }
            run_root = ancestor;
        }
        let Some(root) = self.root.find_split_mut(run_root) else {
            return false;
        };
        let mut node = Node::Split(root.clone());
        node.equalize_run(o);
        let Node::Split(done) = node else {
            unreachable!()
        };
        *root = done;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{Direction::*, Orientation::*};

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn root_split(l: &Layout) -> &Split {
        let Node::Split(s) = l.root() else {
            panic!("root is a split")
        };
        s
    }

    #[test]
    fn two_panes_side_by_side_with_a_one_pixel_divider() {
        let mut l = Layout::new();
        let a = l.focused();
        l.split(Horizontal);
        let b = l.focused();
        let rects = l.rects(1001.0, 500.0, 1.0);
        assert_eq!(
            rects.panes,
            vec![
                (a, r(0.0, 0.0, 500.0, 500.0)),
                (b, r(501.0, 0.0, 500.0, 500.0))
            ]
        );
        assert_eq!(rects.dividers.len(), 1);
        assert_eq!(rects.dividers[0].rect, r(500.0, 0.0, 1.0, 500.0));
        assert_eq!(rects.dividers[0].orientation, Horizontal);
    }

    #[test]
    fn divider_positions_snap_to_device_pixels() {
        let mut l = Layout::new();
        l.split(Vertical);
        // 799 px of space at 0.5 is 399.5; at scale 1 that rounds to 400, at scale 2 it stays.
        assert_eq!(l.rects(100.0, 800.0, 1.0).dividers[0].rect.y, 400.0);
        assert_eq!(l.rects(100.0, 800.0, 2.0).dividers[0].rect.y, 399.5);
        let panes = l.rects(100.0, 800.0, 1.0).panes;
        assert_eq!(panes[1].1, r(0.0, 401.0, 100.0, 399.0));
    }

    #[test]
    fn a_zoomed_pane_fills_the_container_alone() {
        let mut l = Layout::new();
        l.split(Horizontal);
        let b = l.focused();
        l.toggle_zoom();
        let rects = l.rects(800.0, 600.0, 1.0);
        assert_eq!(rects.panes, vec![(b, r(0.0, 0.0, 800.0, 600.0))]);
        assert!(rects.dividers.is_empty());
    }

    #[test]
    fn drag_follows_the_pointer_but_keeps_100_px_and_the_ratio_range() {
        let mut l = Layout::new();
        l.split(Horizontal);
        let id = root_split(&l).id;
        assert!(l.drag_divider(id, 300.0, 1001.0, 500.0, 1.0));
        assert_eq!(l.ratio(id), Some(0.3));
        l.drag_divider(id, 20.0, 1001.0, 500.0, 1.0);
        assert_eq!(
            l.ratio(id),
            Some(0.1),
            "100 px of 1000 is also the 0.1 floor"
        );
        l.drag_divider(id, 5000.0, 1001.0, 500.0, 1.0);
        assert_eq!(l.ratio(id), Some(0.9));
        l.drag_divider(id, 20.0, 201.0, 500.0, 1.0);
        assert_eq!(l.ratio(id), Some(0.5), "too small for 100 px each: half");
        assert!(!l.drag_divider(id, 20.0, 201.0, 500.0, 1.0), "no change");
    }

    #[test]
    fn keyboard_resize_moves_the_divider_by_side() {
        let mut l = Layout::new();
        let left = l.focused();
        l.split(Horizontal);
        let id = root_split(&l).id;
        l.focus_pane(left);
        assert!(
            !l.resize_pane(Left, RESIZE_STEP, 1000.0, 500.0, 1.0),
            "left pane, left: nothing"
        );
        assert!(
            !l.resize_pane(Up, RESIZE_STEP, 1000.0, 500.0, 1.0),
            "no vertical split"
        );
        assert!(l.resize_pane(Right, RESIZE_STEP, 1000.0, 500.0, 1.0));
        assert!((l.ratio(id).unwrap() - 0.52).abs() < 1e-6);
        l.focus_cycle(true);
        assert!(l.resize_pane(Left, RESIZE_STEP, 1000.0, 500.0, 1.0));
        assert!((l.ratio(id).unwrap() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn keyboard_resize_uses_the_innermost_matching_split_and_stops_at_the_clamp() {
        // a | (b | c), focus b: Right moves the inner divider, Left the outer one.
        let mut l = Layout::new();
        l.split(Horizontal);
        let b = l.focused();
        l.split(Horizontal);
        l.focus_pane(b);
        let outer = root_split(&l).id;
        let Node::Split(inner) = &*root_split(&l).second else {
            panic!()
        };
        let inner = inner.id;
        assert!(l.resize_pane(Right, 50.0, 1001.0, 500.0, 1.0));
        assert_eq!(l.ratio(outer), Some(0.5));
        assert!(l.ratio(inner).unwrap() > 0.5);
        assert!(l.resize_pane(Left, 50.0, 1001.0, 500.0, 1.0));
        assert!(l.ratio(outer).unwrap() < 0.5);
        for _ in 0..100 {
            l.resize_pane(Left, 50.0, 1001.0, 500.0, 1.0);
        }
        assert_eq!(l.ratio(outer), Some(0.1));
        assert!(!l.resize_pane(Left, 50.0, 1001.0, 500.0, 1.0));
    }

    #[test]
    fn equalize_all_counts_leaves_along_each_axis() {
        // a | (b | (c / d)): three columns, the last one split in two rows.
        let mut l = Layout::new();
        l.split(Horizontal);
        l.split(Horizontal);
        l.split(Vertical);
        l.equalize_all();
        let root = root_split(&l);
        assert!((root.ratio - 1.0 / 3.0).abs() < 1e-6);
        let Node::Split(inner) = &*root.second else {
            panic!()
        };
        assert_eq!(inner.ratio, 0.5);
        let Node::Split(rows) = &*inner.second else {
            panic!()
        };
        assert_eq!(rows.ratio, 0.5);
    }

    #[test]
    fn equalize_run_stops_at_a_cross_orientation_split() {
        // a | (b / (c | d)): the c|d run is only that split.
        let mut l = Layout::new();
        l.split(Horizontal);
        l.split(Vertical);
        l.split(Horizontal);
        let root = root_split(&l).id;
        let Node::Split(col) = &*root_split(&l).second else {
            panic!()
        };
        let Node::Split(cd) = &*col.second else {
            panic!()
        };
        let (col, cd) = (col.id, cd.id);
        l.drag_divider(root, 700.0, 1001.0, 800.0, 1.0);
        l.drag_divider(cd, 900.0, 1001.0, 800.0, 1.0);
        l.drag_divider(col, 300.0, 1001.0, 800.0, 1.0);
        let before = (l.ratio(root), l.ratio(col));
        assert!(l.equalize_run(cd));
        assert_eq!(l.ratio(cd), Some(0.5));
        assert_eq!((l.ratio(root), l.ratio(col)), before);
    }

    #[test]
    fn equalize_run_climbs_same_orientation_ancestors() {
        // a | (b | c) with skewed dividers: double-clicking the inner one equalizes both.
        let mut l = Layout::new();
        l.split(Horizontal);
        l.split(Horizontal);
        let outer = root_split(&l).id;
        let Node::Split(inner) = &*root_split(&l).second else {
            panic!()
        };
        let inner = inner.id;
        l.drag_divider(outer, 800.0, 1001.0, 500.0, 1.0);
        assert!(l.equalize_run(inner));
        assert!((l.ratio(outer).unwrap() - 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(l.ratio(inner), Some(0.5));
    }

    fn first_pane_surfaces(l: &Layout) -> Vec<SurfaceId> {
        let mut out = Vec::new();
        fn walk(n: &Node, out: &mut Vec<SurfaceId>) {
            match n {
                Node::Pane(p) => out.push(p.tabs[0]),
                Node::Split(s) => {
                    walk(&s.first, out);
                    walk(&s.second, out);
                }
            }
        }
        walk(l.root(), &mut out);
        out
    }

    #[test]
    fn split_left_puts_new_pane_first() {
        let mut l = Layout::new();
        let old = l.active_surface();
        let new = l
            .try_split_surface(old, Direction::Left, true, 1000.0, 500.0, 1.0)
            .unwrap();
        assert_eq!(root_split(&l).orientation, Horizontal);
        assert_eq!(first_pane_surfaces(&l), vec![new, old]);
        assert_eq!(l.active_surface(), new);
    }

    #[test]
    fn split_up_puts_new_pane_first() {
        let mut l = Layout::new();
        let old = l.active_surface();
        let new = l
            .try_split_surface(old, Direction::Up, true, 1000.0, 500.0, 1.0)
            .unwrap();
        assert_eq!(root_split(&l).orientation, Vertical);
        assert_eq!(first_pane_surfaces(&l), vec![new, old]);
        let down = l
            .try_split_surface(old, Direction::Down, true, 1000.0, 500.0, 1.0)
            .unwrap();
        assert_eq!(first_pane_surfaces(&l), vec![new, old, down]);
    }

    #[test]
    fn split_without_focus_keeps_focus() {
        let mut l = Layout::new();
        let old = l.active_surface();
        let pane = l.focused();
        let new = l
            .try_split_surface(old, Direction::Right, false, 1000.0, 500.0, 1.0)
            .unwrap();
        assert_eq!(l.focused(), pane);
        assert_eq!(l.active_surface(), old);
        assert_eq!(first_pane_surfaces(&l), vec![old, new]);
    }

    #[test]
    fn split_surface_in_unfocused_pane() {
        let mut l = Layout::new();
        let a = l.active_surface();
        let b = l.split(Horizontal);
        assert_eq!(l.active_surface(), b);
        let c = l
            .try_split_surface(a, Direction::Down, false, 1000.0, 500.0, 1.0)
            .unwrap();
        assert_eq!(first_pane_surfaces(&l), vec![a, c, b]);
        assert_eq!(l.active_surface(), b);
    }

    #[test]
    fn split_surface_respects_min_size() {
        let mut l = Layout::new();
        let a = l.active_surface();
        assert_eq!(
            l.try_split_surface(a, Direction::Left, true, 300.0, 500.0, 1.0),
            Err(NoSpace)
        );
        assert_eq!(l.pane_count(), 1);
        assert_eq!(
            l.try_split_surface(SurfaceId(999), Direction::Left, true, 1000.0, 500.0, 1.0),
            Err(NoSpace),
            "an unknown surface is refused, not a panic"
        );
    }

    #[test]
    fn a_split_that_leaves_less_than_160_px_is_refused() {
        let mut l = Layout::new();
        assert_eq!(l.try_split(Horizontal, 300.0, 500.0, 1.0), Err(NoSpace));
        assert_eq!(l.pane_count(), 1);
        assert!(l.try_split(Horizontal, 400.0, 500.0, 1.0).is_ok());
        assert_eq!(l.try_split(Vertical, 400.0, 150.0, 1.0), Err(NoSpace));
    }

    #[test]
    fn a_tight_split_borrows_room_by_equalizing_its_run() {
        let mut l = Layout::new();
        l.split(Horizontal);
        let outer = root_split(&l).id;
        l.drag_divider(outer, 400.0, 501.0, 500.0, 1.0);
        // b is 100 wide; splitting it fits only once a, b, b' get a third each.
        assert!(l.try_split(Horizontal, 501.0, 500.0, 1.0).is_ok());
        assert!((l.ratio(outer).unwrap() - 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(l.pane_count(), 3);
        // A fourth column would be 124 px: refused.
        assert_eq!(l.try_split(Horizontal, 501.0, 500.0, 1.0), Err(NoSpace));
    }

    #[test]
    fn fractional_scale_keeps_every_edge_on_a_device_pixel() {
        let mut l = Layout::new();
        l.split(Orientation::Horizontal);
        l.split(Orientation::Vertical);
        l.split(Orientation::Horizontal);
        let scale = 1.5;
        // A window is a whole number of device pixels: 451 x 299 at 1.5.
        let (w, h) = (451.0 / scale, 299.0 / scale);
        let r = l.rects(w, h, scale);
        let on_px = |v: f32| ((v * scale) - (v * scale).round()).abs() < 1e-3;
        for rect in r
            .panes
            .iter()
            .map(|(_, r)| *r)
            .chain(r.dividers.iter().map(|d| d.rect))
        {
            for v in [rect.x, rect.y, rect.x + rect.w, rect.y + rect.h] {
                assert!(on_px(v), "{rect:?} is off the device grid");
            }
        }
        let area: f32 = r
            .panes
            .iter()
            .map(|(_, r)| r.w * r.h)
            .chain(r.dividers.iter().map(|d| d.rect.w * d.rect.h))
            .sum();
        assert!((area - w * h).abs() < 1e-2, "rects must tile the container");
    }

    #[test]
    fn non_finite_input_is_ignored() {
        let mut l = Layout::new();
        l.split(Orientation::Horizontal);
        let split = l.rects(800.0, 600.0, 1.0).dividers[0].split;
        assert!(!l.drag_divider(split, f32::NAN, 800.0, 600.0, 1.0));
        assert!(!l.drag_divider(split, 300.0, f32::NAN, 600.0, 1.0));
        assert!(!l.resize_pane(Direction::Left, f32::NAN, 800.0, 600.0, 1.0));
        let r = l.rects(800.0, 600.0, 1.0);
        assert!(r.panes.iter().all(|(_, r)| r.w.is_finite()));
    }
}
