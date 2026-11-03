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
