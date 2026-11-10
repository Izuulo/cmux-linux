//! What a notification does to the panes: the focused-arrival rule (rn §3) and, from Task 5,
//! the pane ring and flash.

/// A notification "arrives focused" only on the focused surface of the selected workspace of
/// the active window. Such an arrival posts no desktop notification, doesn't flash and doesn't
/// reorder the sidebar (rn §3, §9).
pub fn is_focused_arrival(
    window_active: bool,
    workspace_selected: bool,
    surface_is_active: bool,
) -> bool {
    window_active && workspace_selected && surface_is_active
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_window_is_never_a_focused_arrival() {
        for w in [false, true] {
            for ws in [false, true] {
                for s in [false, true] {
                    assert_eq!(is_focused_arrival(w, ws, s), w && ws && s, "{w} {ws} {s}");
                }
            }
        }
        assert!(!is_focused_arrival(false, true, true));
    }
}
