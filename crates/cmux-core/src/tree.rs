//! The split tree of one workspace: a strictly binary tree of panes, each pane an ordered
//! list of surfaces (tabs). Semantics follow Bonsplit (MIT, github.com/manaflow-ai/bonsplit
//! @58b73fd) as used by cmux; see docs/milestones/m2-splits-tabs/research-cmux.md §1, §2, §6.

use std::collections::HashMap;

/// A leaf of the split tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneId(pub u64);

/// A tab inside a pane (cmux calls it a surface).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfaceId(pub u64);

/// An inner node of the split tree; identifies a divider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SplitId(pub u64);

/// `Horizontal` = side by side (first left), `Vertical` = stacked (first on top), as in Bonsplit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    /// The orientation of the splits whose divider sits across this direction.
    pub fn orientation(self) -> Orientation {
        match self {
            Direction::Left | Direction::Right => Orientation::Horizontal,
            Direction::Up | Direction::Down => Orientation::Vertical,
        }
    }

    /// Right and Down point from a split's first child to its second.
    pub fn forward(self) -> bool {
        matches!(self, Direction::Right | Direction::Down)
    }
}

/// Bonsplit's `dividerPosition` default and clamp range (`BonsplitConfiguration.swift:121`).
pub const DEFAULT_RATIO: f32 = 0.5;
pub const MIN_RATIO: f32 = 0.1;
pub const MAX_RATIO: f32 = 0.9;

#[derive(Debug, Clone, PartialEq)]
pub struct Pane {
    pub id: PaneId,
    /// Never empty while the pane is in the tree.
    pub tabs: Vec<SurfaceId>,
    /// Index into `tabs`.
    pub selected: usize,
}

impl Pane {
    pub fn selected_surface(&self) -> SurfaceId {
        self.tabs[self.selected]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Split {
    pub id: SplitId,
    pub orientation: Orientation,
    pub first: Box<Node>,
    pub second: Box<Node>,
    /// Fraction of the space (minus the divider) given to `first`, in `MIN_RATIO..=MAX_RATIO`.
    pub ratio: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Pane(Pane),
    Split(Split),
}

/// A rectangle in some coordinate space (unit square for focus, logical pixels for layout).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn max_x(&self) -> f32 {
        self.x + self.w
    }

    pub fn max_y(&self) -> f32 {
        self.y + self.h
    }

    /// (start, length) along the axis an `orientation` split divides.
    pub fn along(&self, orientation: Orientation) -> (f32, f32) {
        match orientation {
            Orientation::Horizontal => (self.x, self.w),
            Orientation::Vertical => (self.y, self.h),
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.max_x() && y >= self.y && y < self.max_y()
    }
}

/// What `close_surface` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Closed {
    /// The tab went away; its pane stays.
    Tab,
    /// It was the pane's last tab, so the pane went away too.
    Pane,
    /// It is the only surface left; nothing changed (the app quits instead, M2).
    LastSurface,
    /// No such surface.
    NotFound,
}

impl Node {
    fn pane(&self, id: PaneId) -> Option<&Pane> {
        match self {
            Node::Pane(p) => (p.id == id).then_some(p),
            Node::Split(s) => s.first.pane(id).or_else(|| s.second.pane(id)),
        }
    }

    fn pane_mut(&mut self, id: PaneId) -> Option<&mut Pane> {
        match self {
            Node::Pane(p) => (p.id == id).then_some(p),
            Node::Split(s) => match s.first.pane_mut(id) {
                Some(p) => Some(p),
                None => s.second.pane_mut(id),
            },
        }
    }

    fn collect_panes<'a>(&'a self, out: &mut Vec<&'a Pane>) {
        match self {
            Node::Pane(p) => out.push(p),
            Node::Split(s) => {
                s.first.collect_panes(out);
                s.second.collect_panes(out);
            }
        }
    }

    fn first_pane(&self) -> PaneId {
        match self {
            Node::Pane(p) => p.id,
            Node::Split(s) => s.first.first_pane(),
        }
    }

    /// Replaces the leaf `id` with `make(leaf)`. Returns false when `id` is not here.
    fn replace_pane(&mut self, id: PaneId, make: &mut dyn FnMut(Node) -> Node) -> bool {
        match self {
            Node::Pane(p) if p.id == id => {
                let leaf = std::mem::replace(self, placeholder());
                *self = make(leaf);
                true
            }
            Node::Pane(_) => false,
            Node::Split(s) => s.first.replace_pane(id, make) || s.second.replace_pane(id, make),
        }
    }

    /// Removes leaf `id`; its sibling takes the parent's slot. Returns the first pane of that
    /// sibling, or `None` when `id` is not below this node (or is this node).
    fn remove_pane(&mut self, id: PaneId) -> Option<PaneId> {
        let Node::Split(s) = self else { return None };
        let survivor = if matches!(&*s.first, Node::Pane(p) if p.id == id) {
            std::mem::replace(&mut *s.second, placeholder())
        } else if matches!(&*s.second, Node::Pane(p) if p.id == id) {
            std::mem::replace(&mut *s.first, placeholder())
        } else {
            return s.first.remove_pane(id).or_else(|| s.second.remove_pane(id));
        };
        let absorbed = survivor.first_pane();
        *self = survivor;
        Some(absorbed)
    }

    pub(crate) fn contains_pane(&self, id: PaneId) -> bool {
        self.pane(id).is_some()
    }

    /// Unit-square bounds of every pane, ignoring dividers (Bonsplit `computePaneBounds`).
    fn normalized(&self, r: Rect, out: &mut Vec<(PaneId, Rect)>) {
        match self {
            Node::Pane(p) => out.push((p.id, r)),
            Node::Split(s) => {
                let (a, b) = match s.orientation {
                    Orientation::Horizontal => {
                        let w = r.w * s.ratio;
                        (
                            Rect { w, ..r },
                            Rect {
                                x: r.x + w,
                                w: r.w - w,
                                ..r
                            },
                        )
                    }
                    Orientation::Vertical => {
                        let h = r.h * s.ratio;
                        (
                            Rect { h, ..r },
                            Rect {
                                y: r.y + h,
                                h: r.h - h,
                                ..r
                            },
                        )
                    }
                };
                s.first.normalized(a, out);
                s.second.normalized(b, out);
            }
        }
    }
}

fn placeholder() -> Node {
    Node::Pane(Pane {
        id: PaneId(0),
        tabs: Vec::new(),
        selected: 0,
    })
}

/// Bonsplit's directional-navigation epsilon (`SplitViewController.swift:823`).
const NAV_EPSILON: f32 = 0.001;

/// One workspace's panes and tabs: the tree, the focused pane, focus recency and zoom.
#[derive(Debug, Clone)]
pub struct Layout {
    pub(crate) root: Node,
    focused: PaneId,
    zoomed: Option<PaneId>,
    /// Monotonic focus sequence per pane; higher = focused more recently. Never focused = absent.
    focus_seq: HashMap<PaneId, u64>,
    seq: u64,
    next_id: u64,
}

impl Default for Layout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout {
    /// One pane holding one surface, focused.
    pub fn new() -> Self {
        let mut layout = Layout {
            root: placeholder(),
            focused: PaneId(0),
            zoomed: None,
            focus_seq: HashMap::new(),
            seq: 0,
            next_id: 1,
        };
        let pane = layout.new_pane();
        let id = pane.id;
        layout.root = Node::Pane(pane);
        layout.focus_pane(id);
        layout
    }

    pub(crate) fn next(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn new_pane(&mut self) -> Pane {
        let id = PaneId(self.next());
        let surface = SurfaceId(self.next());
        Pane {
            id,
            tabs: vec![surface],
            selected: 0,
        }
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    pub fn focused(&self) -> PaneId {
        self.focused
    }

    pub fn zoomed(&self) -> Option<PaneId> {
        self.zoomed
    }

    pub fn pane(&self, id: PaneId) -> Option<&Pane> {
        self.root.pane(id)
    }

    pub fn focused_pane(&self) -> &Pane {
        self.root
            .pane(self.focused)
            .expect("focused pane is in the tree")
    }

    /// The selected surface of the focused pane: the one keyboard input goes to.
    pub fn active_surface(&self) -> SurfaceId {
        self.focused_pane().selected_surface()
    }

    /// Panes in depth-first order (first child before second).
    pub fn panes(&self) -> Vec<&Pane> {
        let mut out = Vec::new();
        self.root.collect_panes(&mut out);
        out
    }

    pub fn pane_count(&self) -> usize {
        self.panes().len()
    }

    /// Every surface, pane by pane in tree order.
    pub fn surfaces(&self) -> Vec<SurfaceId> {
        self.panes()
            .iter()
            .flat_map(|p| p.tabs.iter().copied())
            .collect()
    }

    pub fn pane_of(&self, surface: SurfaceId) -> Option<PaneId> {
        self.panes()
            .iter()
            .find(|p| p.tabs.contains(&surface))
            .map(|p| p.id)
    }

    /// Focuses `id` and bumps its recency. False if `id` is not in the tree.
    pub fn focus_pane(&mut self, id: PaneId) -> bool {
        if !self.root.contains_pane(id) {
            return false;
        }
        self.seq += 1;
        self.focus_seq.insert(id, self.seq);
        self.focused = id;
        true
    }

    /// Splits the focused pane: the existing pane stays `first`, a new one-tab pane becomes
    /// `second` (right or below), gets focus, and the ratio is 0.5. Clears zoom. No size check;
    /// the app uses `Layout::try_split`, which checks the minimum pane size first.
    pub fn split(&mut self, orientation: Orientation) -> SurfaceId {
        self.split_pane(self.focused, orientation, false, true)
    }

    /// Splits `target` (which must exist): a new one-tab pane goes `first` when `new_first`
    /// (left or above), else `second`, and gets focus only when `focus`. Ratio 0.5; clears zoom.
    pub(crate) fn split_pane(
        &mut self,
        target: PaneId,
        orientation: Orientation,
        new_first: bool,
        focus: bool,
    ) -> SurfaceId {
        self.zoomed = None;
        let pane = self.new_pane();
        let (pane_id, surface) = (pane.id, pane.tabs[0]);
        let split_id = SplitId(self.next());
        let mut pane = Some(pane);
        self.root.replace_pane(target, &mut |old| {
            let new = Node::Pane(pane.take().expect("replaced once"));
            let (first, second) = if new_first { (new, old) } else { (old, new) };
            Node::Split(Split {
                id: split_id,
                orientation,
                first: Box::new(first),
                second: Box::new(second),
                ratio: DEFAULT_RATIO,
            })
        });
        if focus {
            self.focus_pane(pane_id);
        }
        surface
    }

    /// A new surface in the focused pane, right after the selected tab, and selected.
    pub fn new_tab(&mut self) -> SurfaceId {
        let surface = SurfaceId(self.next());
        let focused = self.focused;
        let pane = self.root.pane_mut(focused).expect("focused pane");
        pane.selected += 1;
        pane.tabs.insert(pane.selected, surface);
        surface
    }

    /// Selects `surface` and focuses its pane (a click on a tab).
    pub fn select_surface(&mut self, surface: SurfaceId) -> bool {
        let Some(pane_id) = self.pane_of(surface) else {
            return false;
        };
        let pane = self.root.pane_mut(pane_id).expect("pane_of");
        pane.selected = pane
            .tabs
            .iter()
            .position(|s| *s == surface)
            .expect("pane_of");
        self.focus_pane(pane_id);
        true
    }

    /// Next (`forward`) or previous tab in the focused pane, wrapping.
    pub fn cycle_tab(&mut self, forward: bool) {
        let focused = self.focused;
        let pane = self.root.pane_mut(focused).expect("focused pane");
        let n = pane.tabs.len();
        pane.selected = if forward {
            (pane.selected + 1) % n
        } else {
            (pane.selected + n - 1) % n
        };
    }

    /// cmux `selectSurfaceByNumber`: 1-8 pick that tab if it exists, 9 always picks the last.
    pub fn select_tab_number(&mut self, number: usize) -> bool {
        let focused = self.focused;
        let pane = self.root.pane_mut(focused).expect("focused pane");
        let index = match number {
            9 => pane.tabs.len() - 1,
            1..=8 if number <= pane.tabs.len() => number - 1,
            _ => return false,
        };
        pane.selected = index;
        true
    }

    /// Moves the focused pane's selected tab one place left (`-1`) or right (`+1`), no wrap.
    pub fn move_tab(&mut self, delta: isize) -> bool {
        let focused = self.focused;
        let pane = self.root.pane_mut(focused).expect("focused pane");
        let to = pane.selected as isize + delta;
        if to < 0 || to as usize >= pane.tabs.len() {
            return false;
        }
        pane.tabs.swap(pane.selected, to as usize);
        pane.selected = to as usize;
        true
    }

    /// Moves `surface` into `to` before tab `index` (an index into `to`'s current tabs; `len`
    /// appends). Dropping a tab on itself or right after itself is a no-op (Bonsplit
    /// `PaneState.swift:89`). The moved tab is selected and its pane focused; a pane left
    /// empty closes. Moving to another pane clears zoom. Returns false when nothing moved.
    pub fn move_surface(&mut self, surface: SurfaceId, to: PaneId, index: usize) -> bool {
        let Some(from) = self.pane_of(surface) else {
            return false;
        };
        if !self.root.contains_pane(to) {
            return false;
        }
        let src = self.root.pane_mut(from).expect("pane_of");
        let at = src
            .tabs
            .iter()
            .position(|s| *s == surface)
            .expect("pane_of");
        if from == to {
            if index == at || index == at + 1 {
                // Bonsplit still selects the tab and focuses its pane (SplitViewController.swift:735-742).
                src.selected = at;
                self.focus_pane(to);
                return false;
            }
            src.tabs.remove(at);
            let index = if index > at { index - 1 } else { index }.min(src.tabs.len());
            src.tabs.insert(index, surface);
            src.selected = index;
            self.focus_pane(to);
            return true;
        }
        Self::remove_tab_at(src, at);
        let emptied = src.tabs.is_empty();
        let dst = self.root.pane_mut(to).expect("checked");
        let index = index.min(dst.tabs.len());
        dst.tabs.insert(index, surface);
        dst.selected = index;
        self.zoomed = None;
        if emptied {
            self.root.remove_pane(from);
            self.focus_seq.remove(&from);
        }
        self.focus_pane(to);
        true
    }

    /// Removes tab `at`, keeping the selection on the same surface, or (when the selected tab
    /// goes) on the tab that slid into its index, else the previous one (`PaneState.swift:63-80`).
    fn remove_tab_at(pane: &mut Pane, at: usize) {
        pane.tabs.remove(at);
        if at < pane.selected || pane.selected >= pane.tabs.len() {
            pane.selected = pane.selected.saturating_sub(1);
        }
    }

    /// Closes one surface. Closing a pane's last tab removes the pane; its sibling takes the
    /// space. Focus then goes to the most recently focused remaining pane (else the sibling);
    /// closing an unfocused pane keeps focus. Closing the zoomed pane clears zoom.
    pub fn close_surface(&mut self, surface: SurfaceId) -> Closed {
        let Some(pane_id) = self.pane_of(surface) else {
            return Closed::NotFound;
        };
        let only_pane = self.pane_count() == 1;
        let pane = self.root.pane_mut(pane_id).expect("pane_of");
        if pane.tabs.len() > 1 {
            let at = pane
                .tabs
                .iter()
                .position(|s| *s == surface)
                .expect("pane_of");
            Self::remove_tab_at(pane, at);
            return Closed::Tab;
        }
        if only_pane {
            return Closed::LastSurface;
        }
        let absorbed = self.root.remove_pane(pane_id).expect("not the root");
        self.focus_seq.remove(&pane_id);
        if self.zoomed == Some(pane_id) {
            self.zoomed = None;
        }
        if self.focused == pane_id {
            let next = self
                .focus_seq
                .iter()
                .max_by_key(|(_, seq)| **seq)
                .map_or(absorbed, |(id, _)| *id);
            self.focus_pane(next);
        }
        Closed::Pane
    }

    /// cmux `closeOtherTabsInPane`: closes every tab of the focused pane but the selected one.
    /// Returns the closed surfaces.
    pub fn close_other_tabs(&mut self) -> Vec<SurfaceId> {
        let focused = self.focused;
        let pane = self.root.pane_mut(focused).expect("focused pane");
        let keep = pane.selected_surface();
        let closed = pane.tabs.iter().copied().filter(|s| *s != keep).collect();
        pane.tabs = vec![keep];
        pane.selected = 0;
        closed
    }

    /// Zoom the focused pane, or unzoom. No-op with one pane (Bonsplit matches Ghostty here).
    pub fn toggle_zoom(&mut self) -> bool {
        if self.zoomed.is_some() {
            self.zoomed = None;
            return true;
        }
        if self.pane_count() < 2 {
            return false;
        }
        self.zoomed = Some(self.focused);
        true
    }

    /// Next/previous pane in tree order, wrapping (cmux `focusNextPane`/`focusPreviousPane`).
    /// Clears zoom, since the target may be hidden.
    pub fn focus_cycle(&mut self, forward: bool) -> bool {
        let ids: Vec<PaneId> = self.panes().iter().map(|p| p.id).collect();
        if ids.len() < 2 {
            return false;
        }
        let at = ids
            .iter()
            .position(|id| *id == self.focused)
            .expect("focused");
        let n = ids.len();
        let next = if forward {
            ids[(at + 1) % n]
        } else {
            ids[(at + n - 1) % n]
        };
        self.zoomed = None;
        self.focus_pane(next)
    }

    /// The pane directional navigation from the focused pane reaches (Bonsplit `adjacentPane`):
    /// panes whose facing edge touches the focused pane's edge with positive overlap; the most
    /// recently focused wins, then larger overlap, then earlier layout order. `None` at an edge.
    pub fn neighbor(&self, dir: Direction) -> Option<PaneId> {
        let mut rects = Vec::new();
        let unit = Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        };
        self.root.normalized(unit, &mut rects);
        let cur = rects.iter().find(|(id, _)| *id == self.focused)?.1;
        let recency = |id: &PaneId| self.focus_seq.get(id).copied().unwrap_or(0);
        rects
            .iter()
            .enumerate()
            .filter(|(_, (id, b))| *id != self.focused && b.w > NAV_EPSILON && b.h > NAV_EPSILON)
            .filter_map(|(order, (id, b))| {
                let (gap, overlap) = match dir {
                    Direction::Left => (cur.x - b.max_x(), overlap_y(&cur, b)),
                    Direction::Right => (b.x - cur.max_x(), overlap_y(&cur, b)),
                    Direction::Up => (cur.y - b.max_y(), overlap_x(&cur, b)),
                    Direction::Down => (b.y - cur.max_y(), overlap_x(&cur, b)),
                };
                (gap.abs() <= NAV_EPSILON && overlap > NAV_EPSILON).then_some((*id, overlap, order))
            })
            .max_by(|a, b| {
                recency(&a.0)
                    .cmp(&recency(&b.0))
                    .then_with(|| {
                        if (a.1 - b.1).abs() > NAV_EPSILON {
                            a.1.total_cmp(&b.1)
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    })
                    .then_with(|| b.2.cmp(&a.2))
            })
            .map(|(id, _, _)| id)
    }

    /// Moves focus to the neighbour in `dir`. Clears zoom when it moves.
    // Bonsplit leaves zoom alone here (research-cmux.md §2, UNVERIFIED); Ghostty's goto_split
    // unzooms, and a hidden pane must not get focus, so we unzoom.
    pub fn focus_direction(&mut self, dir: Direction) -> bool {
        let Some(id) = self.neighbor(dir) else {
            return false;
        };
        self.zoomed = None;
        self.focus_pane(id)
    }
}

fn overlap_x(a: &Rect, b: &Rect) -> f32 {
    a.max_x().min(b.max_x()) - a.x.max(b.x)
}

fn overlap_y(a: &Rect, b: &Rect) -> f32 {
    a.max_y().min(b.max_y()) - a.y.max(b.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Direction::*;
    use Orientation::*;

    fn tabs(l: &Layout, p: PaneId) -> (Vec<SurfaceId>, usize) {
        let pane = l.pane(p).unwrap();
        (pane.tabs.clone(), pane.selected)
    }

    #[test]
    fn new_layout_has_one_focused_pane_with_one_tab() {
        let l = Layout::new();
        assert_eq!(l.pane_count(), 1);
        assert_eq!(l.focused_pane().tabs.len(), 1);
        assert_eq!(l.active_surface(), l.focused_pane().tabs[0]);
    }

    #[test]
    fn split_puts_the_new_pane_second_focused_at_half() {
        let mut l = Layout::new();
        let old = l.focused();
        let s = l.split(Horizontal);
        let Node::Split(split) = l.root() else {
            panic!("root is a split")
        };
        assert_eq!(split.ratio, 0.5);
        assert_eq!(split.orientation, Horizontal);
        assert!(matches!(&*split.first, Node::Pane(p) if p.id == old));
        assert!(matches!(&*split.second, Node::Pane(p) if p.tabs == vec![s]));
        assert_eq!(l.active_surface(), s);
        assert_ne!(l.focused(), old);
    }

    #[test]
    fn split_clears_zoom() {
        let mut l = Layout::new();
        l.split(Horizontal);
        assert!(l.toggle_zoom());
        l.split(Vertical);
        assert_eq!(l.zoomed(), None);
    }

    #[test]
    fn new_tab_goes_after_the_selected_tab() {
        let mut l = Layout::new();
        let p = l.focused();
        let a = l.active_surface();
        let b = l.new_tab();
        assert!(l.select_surface(a));
        let c = l.new_tab();
        assert_eq!(tabs(&l, p), (vec![a, c, b], 1));
    }

    #[test]
    fn closing_the_selected_tab_selects_the_next_else_the_previous() {
        let mut l = Layout::new();
        let p = l.focused();
        let a = l.active_surface();
        let b = l.new_tab();
        let c = l.new_tab();
        l.select_surface(b);
        assert_eq!(l.close_surface(b), Closed::Tab);
        assert_eq!(tabs(&l, p), (vec![a, c], 1), "c slid into b's index");
        assert_eq!(l.close_surface(c), Closed::Tab);
        assert_eq!(tabs(&l, p), (vec![a], 0), "last removed: previous");
    }

    #[test]
    fn closing_an_unselected_tab_keeps_the_selection() {
        let mut l = Layout::new();
        let p = l.focused();
        let a = l.active_surface();
        let b = l.new_tab();
        let c = l.new_tab();
        assert_eq!(l.close_surface(a), Closed::Tab);
        assert_eq!(tabs(&l, p), (vec![b, c], 1));
    }

    #[test]
    fn tab_cycling_wraps_and_numbers_pick_with_nine_last() {
        let mut l = Layout::new();
        let p = l.focused();
        l.new_tab();
        l.new_tab();
        l.cycle_tab(true);
        assert_eq!(l.pane(p).unwrap().selected, 0);
        l.cycle_tab(false);
        assert_eq!(l.pane(p).unwrap().selected, 2);
        assert!(l.select_tab_number(1));
        assert_eq!(l.pane(p).unwrap().selected, 0);
        assert!(!l.select_tab_number(5));
        assert!(l.select_tab_number(9));
        assert_eq!(l.pane(p).unwrap().selected, 2);
    }

    #[test]
    fn move_tab_does_not_wrap() {
        let mut l = Layout::new();
        let p = l.focused();
        let a = l.active_surface();
        let b = l.new_tab();
        assert!(!l.move_tab(1));
        assert!(l.move_tab(-1));
        assert_eq!(tabs(&l, p), (vec![b, a], 0));
        assert!(!l.move_tab(-1));
    }

    #[test]
    fn closing_the_last_tab_of_a_pane_gives_the_sibling_its_slot() {
        let mut l = Layout::new();
        let left = l.focused();
        let right_tab = l.split(Horizontal);
        let bottom_tab = l.split(Vertical);
        let bottom = l.focused();
        assert_eq!(l.close_surface(bottom_tab), Closed::Pane);
        let Node::Split(s) = l.root() else {
            panic!("still a split")
        };
        assert!(matches!(&*s.first, Node::Pane(p) if p.id == left));
        assert!(matches!(&*s.second, Node::Pane(p) if p.tabs == vec![right_tab]));
        assert!(l.pane(bottom).is_none());
    }

    #[test]
    fn closing_the_focused_pane_focuses_the_most_recent_one() {
        let mut l = Layout::new();
        let a = l.focused();
        l.split(Horizontal);
        let b = l.focused();
        let c_tab = l.split(Horizontal);
        // Focus history: a, b, c, a, c. Closing c must go back to a, not to its sibling b.
        l.focus_pane(a);
        l.select_surface(c_tab);
        assert_eq!(l.close_surface(c_tab), Closed::Pane);
        assert_eq!(l.focused(), a);
        assert!(l.pane(b).is_some());
    }

    #[test]
    fn closing_an_unfocused_pane_keeps_focus_and_closing_the_zoomed_pane_unzooms() {
        let mut l = Layout::new();
        let a = l.focused();
        let a_tab = l.active_surface();
        let b_tab = l.split(Horizontal);
        let b = l.focused();
        l.split(Vertical);
        l.focus_pane(b);
        l.toggle_zoom();
        assert_eq!(l.close_surface(a_tab), Closed::Pane);
        assert_eq!(l.focused(), b);
        assert_eq!(l.zoomed(), Some(b));
        assert!(l.pane(a).is_none());
        assert_eq!(l.close_surface(b_tab), Closed::Pane);
        assert_eq!(l.zoomed(), None);
    }

    #[test]
    fn the_last_surface_is_never_closed() {
        let mut l = Layout::new();
        let s = l.active_surface();
        assert_eq!(l.close_surface(s), Closed::LastSurface);
        assert_eq!(l.surfaces(), vec![s]);
        assert_eq!(l.close_surface(SurfaceId(999)), Closed::NotFound);
    }

    #[test]
    fn zoom_is_a_no_op_with_one_pane_and_toggles_otherwise() {
        let mut l = Layout::new();
        assert!(!l.toggle_zoom());
        l.split(Horizontal);
        assert!(l.toggle_zoom());
        assert_eq!(l.zoomed(), Some(l.focused()));
        assert!(l.toggle_zoom());
        assert_eq!(l.zoomed(), None);
    }

    #[test]
    fn close_other_tabs_keeps_only_the_selected_one() {
        let mut l = Layout::new();
        let a = l.active_surface();
        let b = l.new_tab();
        let c = l.new_tab();
        l.select_surface(b);
        assert_eq!(l.close_other_tabs(), vec![a, c]);
        assert_eq!(l.surfaces(), vec![b]);
    }

    #[test]
    fn move_surface_within_a_pane_uses_insertion_indices() {
        let mut l = Layout::new();
        let p = l.focused();
        let a = l.active_surface();
        let b = l.new_tab();
        let c = l.new_tab();
        assert!(!l.move_surface(a, p, 0), "onto itself");
        assert!(!l.move_surface(a, p, 1), "right after itself");
        assert!(l.move_surface(a, p, 3));
        assert_eq!(tabs(&l, p), (vec![b, c, a], 2));
        assert!(l.move_surface(a, p, 0));
        assert_eq!(tabs(&l, p), (vec![a, b, c], 0));
    }

    #[test]
    fn moving_the_last_tab_out_closes_its_pane_and_focuses_the_target() {
        let mut l = Layout::new();
        let left = l.focused();
        let a = l.active_surface();
        let b = l.split(Horizontal);
        let right = l.focused();
        l.toggle_zoom();
        assert!(l.move_surface(b, left, 0));
        assert_eq!(l.pane_count(), 1);
        assert!(l.pane(right).is_none());
        assert_eq!(tabs(&l, left), (vec![b, a], 0));
        assert_eq!(l.focused(), left);
        assert_eq!(l.zoomed(), None);
        assert!(!l.move_surface(b, left, 1), "right after itself");
    }

    /// a | (b / c): a on the left, b top right, c bottom right.
    fn three() -> (Layout, PaneId, PaneId, PaneId) {
        let mut l = Layout::new();
        let a = l.focused();
        l.split(Horizontal);
        let b = l.focused();
        l.split(Vertical);
        let c = l.focused();
        (l, a, b, c)
    }

    #[test]
    fn directional_focus_is_geometric_and_does_not_wrap() {
        let (mut l, a, b, c) = three();
        assert!(l.focus_direction(Up));
        assert_eq!(l.focused(), b);
        assert!(!l.focus_direction(Up));
        assert!(!l.focus_direction(Right));
        assert!(l.focus_direction(Left));
        assert_eq!(l.focused(), a);
        assert!(!l.focus_direction(Left));
        assert!(!l.focus_direction(Down));
        let _ = c;
    }

    #[test]
    fn directional_focus_prefers_the_most_recently_focused_neighbour() {
        let (mut l, a, b, c) = three();
        // From a, both b and c touch the right edge. c was focused last.
        l.focus_pane(a);
        assert!(l.focus_direction(Right));
        assert_eq!(l.focused(), c);
        l.focus_pane(b);
        l.focus_pane(a);
        assert!(l.focus_direction(Right));
        assert_eq!(l.focused(), b);
    }

    #[test]
    fn never_focused_neighbours_rank_by_overlap_then_order() {
        let mut l = Layout::new();
        let a = l.focused();
        l.split(Horizontal);
        let b = l.focused();
        l.split(Vertical);
        let c = l.focused();
        // b (top) is first in layout order; forget both focus histories each time.
        let mut probe = |ratio: f32| {
            if let Node::Split(root) = &mut l.root
                && let Node::Split(right) = &mut *root.second
            {
                right.ratio = ratio;
            }
            l.focus_seq.retain(|id, _| *id == a);
            l.focus_pane(a);
            l.neighbor(Right)
        };
        assert_eq!(probe(0.3), Some(c), "larger overlap beats layout order");
        assert_eq!(
            probe(0.5),
            Some(b),
            "equal overlap falls back to layout order"
        );
    }

    #[test]
    fn focus_cycles_in_tree_order_and_wraps() {
        let (mut l, a, b, c) = three();
        assert!(l.focus_cycle(true));
        assert_eq!(l.focused(), a);
        assert!(l.focus_cycle(true));
        assert_eq!(l.focused(), b);
        assert!(l.focus_cycle(false));
        assert!(l.focus_cycle(false));
        assert_eq!(l.focused(), c);
    }

    #[test]
    fn focus_moves_clear_zoom() {
        let (mut l, _, _, _) = three();
        l.toggle_zoom();
        assert!(l.focus_direction(Up));
        assert_eq!(l.zoomed(), None);
    }
}
