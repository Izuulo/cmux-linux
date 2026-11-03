//! The workspace list: an ordered list of workspaces with one selected. Insert, close, cycle,
//! number-select, move and rename rules follow cmux; see docs/milestones/m3-workspaces-sidebar.

/// One entry of the workspace list. Ids are never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspaceId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub custom_title: Option<String>,
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
    list: Vec<Workspace>,
    selected: usize,
    next_id: u64,
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

    fn insert_selected(&mut self, at: usize) -> WorkspaceId {
        let id = WorkspaceId(self.next_id);
        self.next_id += 1;
        self.list.insert(
            at,
            Workspace {
                id,
                custom_title: None,
            },
        );
        self.selected = at;
        id
    }

    /// Inserted right after the selected workspace, and selected.
    pub fn add_after_selected(&mut self) -> WorkspaceId {
        let at = if self.list.is_empty() {
            0
        } else {
            self.selected + 1
        };
        self.insert_selected(at)
    }

    /// Appended, and selected.
    pub fn add_at_end(&mut self) -> WorkspaceId {
        self.insert_selected(self.list.len())
    }

    /// Closing the selected one selects the workspace that slides into its slot, else the new last.
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
        WorkspaceClosed::Closed
    }

    /// False if unknown or already selected.
    pub fn select(&mut self, id: WorkspaceId) -> bool {
        match self.index_of(id) {
            Some(i) if i != self.selected => {
                self.selected = i;
                true
            }
            _ => false,
        }
    }

    /// `1..=8` select that position, `9` the last; anything else (or past the end) is a no-op.
    pub fn select_number(&mut self, n: usize) -> bool {
        let i = match n {
            1..=8 => n - 1,
            9 => self.list.len() - 1,
            _ => return false,
        };
        match self.list.get(i) {
            Some(w) => self.select(w.id),
            None => false,
        }
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
        true
    }

    /// Moves the selected workspace by `delta` places, without wrapping.
    pub fn move_selected(&mut self, delta: isize) -> bool {
        let to = self.selected as isize + delta;
        if delta == 0 || to < 0 || to >= self.list.len() as isize {
            return false;
        }
        let w = self.list.remove(self.selected);
        self.list.insert(to as usize, w);
        self.selected = to as usize;
        true
    }

    /// Moves `id` into `gap` (0..=len, counted in the current order). The selection follows ids.
    pub fn move_to(&mut self, id: WorkspaceId, gap: usize) -> bool {
        let Some(i) = self.index_of(id) else {
            return false;
        };
        if gap > self.list.len() || gap == i || gap == i + 1 {
            return false;
        }
        let selected = self.selected();
        let w = self.list.remove(i);
        self.list.insert(if gap > i { gap - 1 } else { gap }, w);
        self.selected = self.index_of(selected).expect("selected id still listed");
        true
    }

    /// The name is trimmed; an empty name clears the custom title.
    pub fn rename(&mut self, id: WorkspaceId, name: &str) -> bool {
        let Some(w) = self.list.iter_mut().find(|w| w.id == id) else {
            return false;
        };
        let name = name.trim();
        w.custom_title = (!name.is_empty()).then(|| name.to_string());
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
    fn ids_are_never_reused() {
        let mut w = Workspaces::new();
        let b = w.add_at_end();
        w.close(b);
        assert_ne!(w.add_at_end(), b);
    }
}
