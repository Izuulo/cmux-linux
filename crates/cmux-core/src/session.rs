//! What M10 saves of the model (plan L3, L5): a split tree without ids, and the workspace list
//! rebuilt from saved parts. The app maps both to and from `session.json`.

use crate::groups::Group;
use crate::tree::{Layout, MAX_RATIO, MIN_RATIO, Node, Orientation, Pane, PaneId, Split, SplitId};
use crate::workspaces::{Workspace, Workspaces};

/// A split tree with counts instead of ids. Surfaces are numbered pane by pane in tree order,
/// as `Layout::surfaces` lists them.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Pane {
        tabs: usize,
        selected: usize,
    },
    Split {
        orientation: Orientation,
        ratio: f32,
        first: Box<Shape>,
        second: Box<Shape>,
    },
}

impl Shape {
    fn of(node: &Node) -> Shape {
        match node {
            Node::Pane(p) => Shape::Pane {
                tabs: p.tabs.len(),
                selected: p.selected,
            },
            Node::Split(s) => Shape::Split {
                orientation: s.orientation,
                ratio: s.ratio,
                first: Box::new(Shape::of(&s.first)),
                second: Box::new(Shape::of(&s.second)),
            },
        }
    }
}

impl Layout {
    /// The tree's shape and the focused pane's index in tree order.
    pub fn shape(&self) -> (Shape, usize) {
        let focused = self.focused();
        let index = self.panes().iter().position(|p| p.id == focused);
        (Shape::of(self.root()), index.unwrap_or(0))
    }

    /// A layout of `shape` with new ids, pane `focused` (tree order) focused. Out-of-range
    /// counts are repaired: a pane has at least one tab, ratios are clamped.
    pub fn from_shape(shape: &Shape, focused: usize) -> Layout {
        let mut l = Layout::new();
        l.root = l.build(shape);
        let panes: Vec<PaneId> = l.panes().iter().map(|p| p.id).collect();
        l.focus_pane(panes[focused.min(panes.len() - 1)]);
        l
    }

    fn build(&mut self, shape: &Shape) -> Node {
        match shape {
            Shape::Pane { tabs, selected } => {
                let id = PaneId(self.next());
                let tabs: Vec<_> = (0..(*tabs).max(1))
                    .map(|_| crate::tree::SurfaceId(self.next()))
                    .collect();
                Node::Pane(Pane {
                    id,
                    selected: (*selected).min(tabs.len() - 1),
                    tabs,
                })
            }
            Shape::Split {
                orientation,
                ratio,
                first,
                second,
            } => Node::Split(Split {
                id: SplitId(self.next()),
                orientation: *orientation,
                first: Box::new(self.build(first)),
                second: Box::new(self.build(second)),
                ratio: if ratio.is_finite() {
                    ratio.clamp(MIN_RATIO, MAX_RATIO)
                } else {
                    crate::tree::DEFAULT_RATIO
                },
            }),
        }
    }
}

impl Workspaces {
    /// A list from saved parts (ids as given). Empty `list` gives a fresh list; `selected` is
    /// clamped; dangling groups and anchors are repaired by `normalize`.
    pub fn from_parts(list: Vec<Workspace>, groups: Vec<Group>, selected: usize) -> Workspaces {
        if list.is_empty() {
            return Workspaces::new();
        }
        let mut w = Workspaces::new();
        w.selected = selected.min(list.len() - 1);
        w.list = list;
        w.groups = groups;
        w.normalize();
        w.next_id = w.list.iter().map(|x| x.id.0).max().unwrap_or(0) + 1;
        w.next_group = w.groups.iter().map(|g| g.id.0).max().unwrap_or(0) + 1;
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::groups::GroupId;
    use crate::tree::{Direction, Orientation::*};
    use crate::workspaces::WorkspaceId;

    #[test]
    fn shape_round_trips_tree_tabs_ratio_and_focus() {
        let mut l = Layout::new();
        l.split(Horizontal);
        l.new_tab();
        l.new_tab();
        l.select_tab_number(2);
        l.split(Vertical);
        l.focus_direction(Direction::Left);
        let (shape, focused) = l.shape();
        let r = Layout::from_shape(&shape, focused);
        assert_eq!(r.shape(), (shape, focused));
        assert_eq!(r.surfaces().len(), 5);
        assert_eq!(focused, 0);
    }

    #[test]
    fn from_shape_repairs_bad_counts() {
        let shape = Shape::Split {
            orientation: Vertical,
            ratio: 7.0,
            first: Box::new(Shape::Pane {
                tabs: 0,
                selected: 5,
            }),
            second: Box::new(Shape::Pane {
                tabs: 2,
                selected: 9,
            }),
        };
        let l = Layout::from_shape(&shape, 99);
        let (got, focused) = l.shape();
        let Shape::Split {
            ratio,
            first,
            second,
            ..
        } = got
        else {
            panic!("split expected")
        };
        assert_eq!(ratio, MAX_RATIO);
        assert_eq!(
            *first,
            Shape::Pane {
                tabs: 1,
                selected: 0
            }
        );
        assert_eq!(
            *second,
            Shape::Pane {
                tabs: 2,
                selected: 1
            }
        );
        assert_eq!(focused, 1);
    }

    #[test]
    fn from_parts_keeps_order_selection_and_groups() {
        let ws = |id, group| Workspace {
            id: WorkspaceId(id),
            custom_title: Some(format!("w{id}")),
            auto_title: id == 3,
            group,
        };
        let g = Group {
            id: GroupId(1),
            name: "g".into(),
            anchor: WorkspaceId(2),
            collapsed: true,
            pinned: false,
            generated_anchor: false,
        };
        let list = vec![
            ws(1, None),
            ws(2, Some(GroupId(1))),
            ws(3, Some(GroupId(1))),
        ];
        let mut w = Workspaces::from_parts(list.clone(), vec![g.clone()], 0);
        assert_eq!(w.list(), &list[..]);
        assert_eq!(w.groups(), vec![&g]);
        assert_eq!(w.selected(), WorkspaceId(1));
        // New ids don't collide with restored ones.
        assert_eq!(w.add_at_end(), WorkspaceId(4));
        assert!(Workspaces::from_parts(Vec::new(), Vec::new(), 3).len() == 1);
    }
}
