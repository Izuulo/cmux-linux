//! The workspace list: an ordered list of workspaces with one selected. Insert, close, cycle,
//! number-select, move and rename rules follow cmux; see docs/milestones/m3-workspaces-sidebar.

use crate::groups::{Group, GroupId, Row};

/// One entry of the workspace list. Ids are never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspaceId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub custom_title: Option<String>,
    /// `custom_title` came from auto-naming, not the user (M9 plan N3).
    pub auto_title: bool,
    /// The group this workspace is in (`groups.rs`); its anchor carries it too.
    pub group: Option<GroupId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceClosed {
    Closed,
    /// It was the only workspace; the list is unchanged.
    Last,
    NotFound,
}

#[derive(Debug)]
pub struct Workspaces {
    pub(crate) list: Vec<Workspace>,
    pub(crate) selected: usize,
    next_id: u64,
    pub(crate) groups: Vec<Group>,
    pub(crate) next_group: u64,
}

impl Default for Workspaces {
    fn default() -> Self {
        Self::new()
    }
}

impl Workspaces {
    pub fn new() -> Self {
        let mut w = Workspaces {
            list: Vec::new(),
            selected: 0,
            next_id: 1,
            groups: Vec::new(),
            next_group: 1,
        };
        w.add_at_end();
        w
    }

    pub fn list(&self) -> &[Workspace] {
        &self.list
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn selected(&self) -> WorkspaceId {
        self.list[self.selected].id
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn index_of(&self, id: WorkspaceId) -> Option<usize> {
        self.list.iter().position(|w| w.id == id)
    }

    pub(crate) fn insert_selected(&mut self, at: usize) -> WorkspaceId {
        let id = WorkspaceId(self.next_id);
        self.next_id += 1;
        self.list.insert(
            at,
            Workspace {
                id,
                custom_title: None,
                auto_title: false,
                group: None,
            },
        );
        self.selected = at;
        id
    }

    /// Inserted right after the selected workspace (after its whole group, if it has one), and
    /// selected.
    pub fn add_after_selected(&mut self) -> WorkspaceId {
        let at = if self.list.is_empty() {
            0
        } else {
            self.selected + 1
        };
        let id = self.insert_selected(at);
        self.normalize();
        id
    }

    /// Appended, and selected.
    pub fn add_at_end(&mut self) -> WorkspaceId {
        self.insert_selected(self.list.len())
    }

    /// Closing the selected one selects the workspace that slides into its slot, else the new
    /// last; a hidden member there hands over to its group's anchor. Closing an anchor promotes
    /// the group's next member.
    pub fn close(&mut self, id: WorkspaceId) -> WorkspaceClosed {
        let Some(i) = self.index_of(id) else {
            return WorkspaceClosed::NotFound;
        };
        if self.list.len() == 1 {
            return WorkspaceClosed::Last;
        }
        self.list.remove(i);
        if i < self.selected {
            self.selected -= 1;
        }
        self.selected = self.selected.min(self.list.len() - 1);
        self.normalize();
        if let Some(anchor) = self.hidden_anchor(self.selected()) {
            self.selected = self.index_of(anchor).expect("anchors are listed");
        }
        WorkspaceClosed::Closed
    }

    /// False if unknown or already selected. Selecting a hidden member expands its group.
    pub fn select(&mut self, id: WorkspaceId) -> bool {
        match self.index_of(id) {
            Some(i) if i != self.selected => {
                self.selected = i;
                self.expand_for_selection();
                true
            }
            _ => false,
        }
    }

    /// `1..=8` select that plain visible row, `9` the last one; group headers and hidden
    /// members are not counted. Anything else (or past the end) is a no-op.
    pub fn select_number(&mut self, n: usize) -> bool {
        let plain: Vec<WorkspaceId> = self
            .rows()
            .into_iter()
            .filter_map(|r| match r {
                Row::Workspace { id, .. } => Some(id),
                Row::Header { .. } => None,
            })
            .collect();
        let pick = match n {
            1..=8 => plain.get(n - 1),
            9 => plain.last(),
            _ => None,
        };
        pick.is_some_and(|id| self.select(*id))
    }

    /// Wraps around; false with a single workspace.
    pub fn cycle(&mut self, forward: bool) -> bool {
        let n = self.list.len();
        if n < 2 {
            return false;
        }
        self.selected = if forward {
            (self.selected + 1) % n
        } else {
            (self.selected + n - 1) % n
        };
        self.expand_for_selection();
        true
    }

    /// The name is trimmed; an empty name clears the custom title.
    pub fn rename(&mut self, id: WorkspaceId, name: &str) -> bool {
        let Some(w) = self.list.iter_mut().find(|w| w.id == id) else {
            return false;
        };
        let name = name.trim();
        w.custom_title = (!name.is_empty()).then(|| name.to_string());
        w.auto_title = false;
        true
    }

    /// May auto-naming title this workspace? Not once the user has named it.
    pub fn accepts_auto_title(&self, id: WorkspaceId) -> bool {
        self.list
            .iter()
            .any(|w| w.id == id && (w.auto_title || w.custom_title.is_none()))
    }

    /// An auto-naming title: refused once the user has named the workspace (M9 plan N3).
    pub fn set_auto_title(&mut self, id: WorkspaceId, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || !self.accepts_auto_title(id) {
            return false;
        }
        let w = self.list.iter_mut().find(|w| w.id == id).expect("accepted");
        w.custom_title = Some(name.to_string());
        w.auto_title = true;
        true
    }

    pub fn custom_title(&self, id: WorkspaceId) -> Option<&str> {
        self.list
            .iter()
            .find(|w| w.id == id)
            .and_then(|w| w.custom_title.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(w: &Workspaces) -> Vec<u64> {
        w.list().iter().map(|x| x.id.0).collect()
    }

    #[test]
    fn new_has_one_selected_workspace() {
        let w = Workspaces::new();
        assert_eq!(w.len(), 1);
        assert_eq!(w.selected(), w.list()[0].id);
    }

    #[test]
    fn add_after_selected_inserts_next_and_selects() {
        let mut w = Workspaces::new(); // [a]
        let b = w.add_after_selected(); // [a b]
        w.select(w.list()[0].id);
        let c = w.add_after_selected(); // [a c b]
        assert_eq!(ids(&w), vec![w.list()[0].id.0, c.0, b.0]);
        assert_eq!(w.selected(), c);
        let d = w.add_at_end();
        assert_eq!(*ids(&w).last().unwrap(), d.0);
        assert_eq!(w.selected(), d);
    }

    #[test]
    fn close_selects_the_slot_then_the_last() {
        let mut w = Workspaces::new();
        let a = w.selected();
        let b = w.add_at_end();
        let c = w.add_at_end();
        w.select(b);
        assert_eq!(w.close(b), WorkspaceClosed::Closed);
        assert_eq!(w.selected(), c, "c slid into b's slot");
        assert_eq!(w.close(c), WorkspaceClosed::Closed);
        assert_eq!(w.selected(), a, "no slot left: new last");
        assert_eq!(w.close(a), WorkspaceClosed::Last);
        assert_eq!(w.len(), 1);
        assert_eq!(w.close(WorkspaceId(999)), WorkspaceClosed::NotFound);
    }

    #[test]
    fn closing_an_unselected_workspace_keeps_the_selection() {
        let mut w = Workspaces::new();
        let a = w.selected();
        let b = w.add_at_end();
        w.select(a);
        w.close(b);
        assert_eq!(w.selected(), a);
        let c = w.add_at_end(); // [a c], c selected
        w.close(a);
        assert_eq!(w.selected(), c);
        assert_eq!(w.selected_index(), 0);
    }

    #[test]
    fn select_number_nine_is_last_and_out_of_range_is_noop() {
        let mut w = Workspaces::new();
        let a = w.selected();
        w.add_at_end();
        let c = w.add_at_end();
        assert!(w.select_number(1));
        assert_eq!(w.selected(), a);
        assert!(w.select_number(9));
        assert_eq!(w.selected(), c);
        assert!(!w.select_number(5));
        assert_eq!(w.selected(), c);
        assert!(!w.select_number(0));
    }

    #[test]
    fn cycle_wraps_and_move_does_not() {
        let mut w = Workspaces::new();
        let a = w.selected();
        let b = w.add_at_end();
        assert!(w.cycle(true));
        assert_eq!(w.selected(), a, "b → wraps to a");
        assert!(w.cycle(false));
        assert_eq!(w.selected(), b);
        assert!(!w.move_selected(1), "b is last");
        assert!(w.move_selected(-1));
        assert_eq!(ids(&w), vec![b.0, a.0]);
        let mut one = Workspaces::new();
        assert!(!one.cycle(true));
    }

    #[test]
    fn move_to_uses_gaps_in_current_order() {
        let mut w = Workspaces::new();
        let a = w.selected();
        let b = w.add_at_end();
        let c = w.add_at_end();
        assert!(!w.move_to(a, 0));
        assert!(!w.move_to(a, 1));
        assert!(w.move_to(a, 2));
        assert_eq!(ids(&w), vec![b.0, a.0, c.0]);
        assert!(w.move_to(c, 0));
        assert_eq!(ids(&w), vec![c.0, b.0, a.0]);
        assert!(w.move_to(c, 3));
        assert_eq!(ids(&w), vec![b.0, a.0, c.0]);
        assert_eq!(w.selected(), c, "moving keeps the selected id");
    }

    #[test]
    fn rename_trims_and_empty_clears() {
        let mut w = Workspaces::new();
        let a = w.selected();
        assert!(w.rename(a, "  api  "));
        assert_eq!(w.custom_title(a), Some("api"));
        assert!(w.rename(a, "   "));
        assert_eq!(w.custom_title(a), None);
        assert!(!w.rename(WorkspaceId(42), "x"));
    }

    #[test]
    fn auto_titles_lose_to_the_user() {
        let mut w = Workspaces::new();
        let a = w.selected();
        assert!(w.set_auto_title(a, " fix login "));
        assert!(w.set_auto_title(a, "fix login flow"), "auto replaces auto");
        assert_eq!(w.custom_title(a), Some("fix login flow"));
        assert!(w.rename(a, "mine"));
        assert!(!w.set_auto_title(a, "theirs"));
        assert_eq!(w.custom_title(a), Some("mine"));
        assert!(w.rename(a, ""), "clearing reopens it");
        assert!(!w.set_auto_title(a, "  "));
        assert!(w.set_auto_title(a, "again"));
        assert!(!w.set_auto_title(WorkspaceId(42), "x"));
    }

    #[test]
    fn ids_are_never_reused() {
        let mut w = Workspaces::new();
        let b = w.add_at_end();
        w.close(b);
        assert_ne!(w.add_at_end(), b);
    }
}
