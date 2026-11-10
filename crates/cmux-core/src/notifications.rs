//! Terminal notifications (OSC 9/99/777): a newest-first store with at most one notification
//! per (workspace, surface), and cmux's read rules (docs/milestones/m4-notifications, rn §2-3).

use std::collections::HashSet;
use std::time::SystemTime;

use crate::{SurfaceId, WorkspaceId};

/// Never reused within a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NotificationId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub id: NotificationId,
    pub workspace: WorkspaceId,
    pub surface: SurfaceId,
    pub title: String,
    /// Only `cmux notify --subtitle` sets one; OSC notifications have none.
    pub subtitle: String,
    pub body: String,
    pub created: SystemTime,
    pub read: bool,
}

impl Notification {
    /// The sidebar line (rn §6): the trimmed body, else the subtitle, else the title (cmux
    /// `TabItemView+WorkspaceNotificationsMenu.swift:31`); None if all are empty.
    pub fn summary(&self) -> Option<&str> {
        [self.body.trim(), self.subtitle.trim(), self.title.trim()]
            .into_iter()
            .find(|s| !s.is_empty())
    }

    /// The panel's body line: the body, else the subtitle (cmux's workspace menu rule).
    pub fn detail(&self) -> &str {
        match self.body.trim() {
            "" => self.subtitle.trim(),
            body => body,
        }
    }

    /// A desktop bubble's body: the subtitle line, then the body, each trimmed (M5 plan D7).
    pub fn desktop_body(&self) -> String {
        [self.subtitle.trim(), self.body.trim()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn at(&self, ws: WorkspaceId, s: SurfaceId) -> bool {
        self.workspace == ws && self.surface == s
    }
}

/// What `add` did: the new id and the ones it replaced on the same surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Added {
    pub id: NotificationId,
    pub replaced: Vec<NotificationId>,
}

/// Mutators return the ids that became read or were removed, so the caller can withdraw
/// their desktop notifications.
#[derive(Debug, Default)]
pub struct Notifications {
    /// Newest first.
    list: Vec<Notification>,
    /// Surfaces that got a notification while focused: their ring stays after a focus-read,
    /// until typing or removal (rn §3).
    focused_read: HashSet<(WorkspaceId, SurfaceId)>,
    next: u64,
    generation: u64,
}

impl Notifications {
    pub fn new() -> Self {
        Self::default()
    }

    /// Changes whenever anything visible changes, and never otherwise.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Stores a new unread notification, replacing any other on the same surface (rn §2).
    /// `focused`: it arrived on the focused surface of the active window.
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &mut self,
        ws: WorkspaceId,
        s: SurfaceId,
        title: String,
        subtitle: String,
        body: String,
        focused: bool,
        now: SystemTime,
    ) -> Added {
        let replaced = self.take(|n| n.at(ws, s));
        self.next += 1;
        let id = NotificationId(self.next);
        self.list.insert(
            0,
            Notification {
                id,
                workspace: ws,
                surface: s,
                title,
                subtitle,
                body,
                created: now,
                read: false,
            },
        );
        if focused {
            self.focused_read.insert((ws, s));
        } else {
            self.focused_read.remove(&(ws, s));
        }
        self.generation += 1;
        Added { id, replaced }
    }

    pub fn list(&self) -> &[Notification] {
        &self.list
    }

    pub fn get(&self, id: NotificationId) -> Option<&Notification> {
        self.list.iter().find(|n| n.id == id)
    }

    pub fn unread_count(&self, ws: WorkspaceId) -> usize {
        self.list
            .iter()
            .filter(|n| n.workspace == ws && !n.read)
            .count()
    }

    pub fn total_unread(&self) -> usize {
        self.list.iter().filter(|n| !n.read).count()
    }

    /// The workspace's newest notification, read or not (the sidebar line).
    pub fn latest(&self, ws: WorkspaceId) -> Option<&Notification> {
        self.list.iter().find(|n| n.workspace == ws)
    }

    /// The newest unread notification across all workspaces (jump to latest unread).
    pub fn latest_unread(&self) -> Option<&Notification> {
        self.list.iter().find(|n| !n.read)
    }

    /// Whether the surface's pane ring shows: an unread notification there, or the
    /// focused-read indicator.
    pub fn shows_ring(&self, ws: WorkspaceId, s: SurfaceId) -> bool {
        self.focused_read.contains(&(ws, s)) || self.list.iter().any(|n| n.at(ws, s) && !n.read)
    }

    pub fn mark_read(&mut self, id: NotificationId) -> Vec<NotificationId> {
        self.read_where(|n| n.id == id)
    }

    /// Focus reached the surface: read, but the focused-read indicator stays.
    pub fn mark_surface_read(&mut self, ws: WorkspaceId, s: SurfaceId) -> Vec<NotificationId> {
        self.read_where(|n| n.at(ws, s))
    }

    /// The user typed into the surface: read, and the indicator goes. Cheap when nothing
    /// changes; it runs on every keystroke.
    pub fn typed(&mut self, ws: WorkspaceId, s: SurfaceId) -> Vec<NotificationId> {
        let read = self.read_where(|n| n.at(ws, s));
        if self.focused_read.remove(&(ws, s)) && read.is_empty() {
            self.generation += 1;
        }
        read
    }

    pub fn mark_all_read(&mut self) -> Vec<NotificationId> {
        self.read_where(|_| true)
    }

    pub fn remove(&mut self, id: NotificationId) -> Vec<NotificationId> {
        self.take(|n| n.id == id)
    }

    pub fn clear_all(&mut self) -> Vec<NotificationId> {
        self.take(|_| true)
    }

    /// `cmux dismiss-notification --all-read`.
    pub fn remove_read(&mut self) -> Vec<NotificationId> {
        self.take(|n| n.read)
    }

    pub fn remove_surface(&mut self, ws: WorkspaceId, s: SurfaceId) -> Vec<NotificationId> {
        self.take(|n| n.at(ws, s))
    }

    pub fn remove_workspace(&mut self, ws: WorkspaceId) -> Vec<NotificationId> {
        self.take(|n| n.workspace == ws)
    }

    fn read_where(&mut self, f: impl Fn(&Notification) -> bool) -> Vec<NotificationId> {
        let mut ids = Vec::new();
        for n in self.list.iter_mut().filter(|n| !n.read && f(n)) {
            n.read = true;
            ids.push(n.id);
        }
        if !ids.is_empty() {
            self.generation += 1;
        }
        ids
    }

    /// Removes matching notifications and the indicators of the surfaces they were on.
    fn take(&mut self, f: impl Fn(&Notification) -> bool) -> Vec<NotificationId> {
        let mut ids = Vec::new();
        self.list.retain(|n| {
            if f(n) {
                ids.push(n.id);
                self.focused_read.remove(&(n.workspace, n.surface));
                false
            } else {
                true
            }
        });
        if !ids.is_empty() {
            self.generation += 1;
        }
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const W1: WorkspaceId = WorkspaceId(1);
    const W2: WorkspaceId = WorkspaceId(2);
    const S1: SurfaceId = SurfaceId(1);
    const S2: SurfaceId = SurfaceId(2);

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn add(n: &mut Notifications, ws: WorkspaceId, s: SurfaceId, body: &str) -> NotificationId {
        n.add(ws, s, "T".into(), String::new(), body.into(), false, at(0))
            .id
    }

    #[test]
    fn add_is_unread_and_newest_first() {
        let mut n = Notifications::new();
        let a = add(&mut n, W1, S1, "a");
        let b = add(&mut n, W1, S2, "b");
        let ids: Vec<_> = n.list().iter().map(|x| x.id).collect();
        assert_eq!(ids, vec![b, a]);
        assert!(n.list().iter().all(|x| !x.read));
        assert_eq!(n.get(a).unwrap().body, "a");
    }

    #[test]
    fn add_replaces_same_surface() {
        let mut n = Notifications::new();
        let mut prev = add(&mut n, W1, S1, "0");
        for i in 1..1000 {
            let added = n.add(
                W1,
                S1,
                String::new(),
                String::new(),
                i.to_string(),
                false,
                at(i),
            );
            assert_eq!(added.replaced, vec![prev]);
            prev = added.id;
        }
        assert_eq!(n.list().len(), 1);
        assert_eq!(n.list()[0].body, "999");
    }

    #[test]
    fn same_surface_id_in_two_workspaces_is_distinct() {
        let mut n = Notifications::new();
        add(&mut n, W1, S1, "a");
        let added = n.add(W2, S1, "T".into(), String::new(), "b".into(), false, at(0));
        assert!(added.replaced.is_empty());
        assert_eq!(n.list().len(), 2);
    }

    #[test]
    fn focused_arrival_is_unread_with_indicator() {
        let mut n = Notifications::new();
        let id = n
            .add(W1, S1, "T".into(), String::new(), "b".into(), true, at(0))
            .id;
        assert!(!n.get(id).unwrap().read);
        assert!(n.shows_ring(W1, S1));
        assert_eq!(n.mark_surface_read(W1, S1), vec![id]);
        assert!(n.get(id).unwrap().read);
        assert!(
            n.shows_ring(W1, S1),
            "focus keeps the focused-read indicator"
        );
        n.typed(W1, S1);
        assert!(!n.shows_ring(W1, S1), "typing clears it");
    }

    #[test]
    fn unfocused_arrival_ring_ends_on_read() {
        let mut n = Notifications::new();
        let id = add(&mut n, W1, S1, "b");
        assert!(n.shows_ring(W1, S1));
        assert!(!n.shows_ring(W1, S2));
        assert_eq!(n.mark_surface_read(W1, S1), vec![id]);
        assert!(!n.shows_ring(W1, S1));
        assert_eq!(n.mark_surface_read(W1, S1), vec![], "already read");
    }

    #[test]
    fn typed_marks_read_and_without_change_keeps_generation() {
        let mut n = Notifications::new();
        let id = add(&mut n, W1, S1, "b");
        let g = n.generation();
        assert_eq!(n.typed(W1, S2), vec![]);
        assert_eq!(n.generation(), g);
        assert_eq!(n.typed(W1, S1), vec![id]);
        assert!(n.generation() > g);
        let g = n.generation();
        assert_eq!(n.typed(W1, S1), vec![]);
        assert_eq!(n.generation(), g);
    }

    #[test]
    fn latest_includes_read() {
        let mut n = Notifications::new();
        add(&mut n, W1, S1, "old");
        let b = add(&mut n, W1, S2, "new");
        n.mark_read(b);
        assert_eq!(n.latest(W1).unwrap().id, b);
        assert!(n.latest(W2).is_none());
    }

    #[test]
    fn summary_prefers_body_then_title() {
        let mut n = Notifications::new();
        let a = n
            .add(
                W1,
                S1,
                " Title ".into(),
                String::new(),
                "  body \n".into(),
                false,
                at(0),
            )
            .id;
        let b = n
            .add(
                W1,
                S2,
                " Title ".into(),
                String::new(),
                " ".into(),
                false,
                at(0),
            )
            .id;
        let c = n
            .add(W2, S1, "".into(), String::new(), "".into(), false, at(0))
            .id;
        assert_eq!(n.get(a).unwrap().summary(), Some("body"));
        assert_eq!(n.get(b).unwrap().summary(), Some("Title"));
        assert_eq!(n.get(c).unwrap().summary(), None);
    }

    #[test]
    fn latest_unread_spans_workspaces() {
        let mut n = Notifications::new();
        let a = add(&mut n, W1, S1, "a");
        let b = add(&mut n, W2, S1, "b");
        assert_eq!(n.latest_unread().unwrap().id, b);
        n.mark_read(b);
        assert_eq!(n.latest_unread().unwrap().id, a);
        n.mark_read(a);
        assert!(n.latest_unread().is_none());
    }

    #[test]
    fn counts() {
        let mut n = Notifications::new();
        let a = add(&mut n, W1, S1, "a");
        add(&mut n, W1, S2, "b");
        add(&mut n, W2, S1, "c");
        assert_eq!(
            (n.unread_count(W1), n.unread_count(W2), n.total_unread()),
            (2, 1, 3)
        );
        n.mark_read(a);
        assert_eq!((n.unread_count(W1), n.total_unread()), (1, 2));
    }

    #[test]
    fn ids_of_closed_surfaces_are_gone() {
        let mut n = Notifications::new();
        let a = n
            .add(W1, S1, "T".into(), String::new(), "a".into(), true, at(0))
            .id;
        let b = add(&mut n, W2, S1, "b");
        let c = add(&mut n, W2, S2, "c");
        assert_eq!(n.remove_surface(W1, S1), vec![a]);
        assert!(n.get(a).is_none());
        assert_eq!(n.mark_read(a), vec![]);
        assert!(!n.shows_ring(W1, S1), "indicator went with the surface");
        let mut gone = n.remove_workspace(W2);
        gone.sort_by_key(|id| id.0);
        assert_eq!(gone, vec![b, c]);
        assert!(n.list().is_empty());
        assert_eq!(n.remove(b), vec![]);
    }

    #[test]
    fn mark_all_read_and_clear_all_return_affected_ids() {
        let mut n = Notifications::new();
        let a = add(&mut n, W1, S1, "a");
        let b = add(&mut n, W2, S1, "b");
        n.mark_read(a);
        assert_eq!(n.mark_all_read(), vec![b]);
        assert_eq!(n.total_unread(), 0);
        assert_eq!(n.mark_all_read(), vec![]);
        let mut gone = n.clear_all();
        gone.sort_by_key(|id| id.0);
        assert_eq!(gone, vec![a, b]);
        assert!(n.list().is_empty());
        assert_eq!(n.remove(a), vec![]);
    }

    #[test]
    fn remove_drops_one_and_its_indicator() {
        let mut n = Notifications::new();
        let a = n
            .add(W1, S1, "T".into(), String::new(), "a".into(), true, at(0))
            .id;
        assert_eq!(n.remove(a), vec![a]);
        assert!(!n.shows_ring(W1, S1));
    }

    #[test]
    fn summary_prefers_body_then_subtitle_then_title() {
        let mut n = Notifications::new();
        let mut summary = |title: &str, subtitle: &str, body: &str| {
            let id = n
                .add(
                    W1,
                    S1,
                    title.into(),
                    subtitle.into(),
                    body.into(),
                    false,
                    at(0),
                )
                .id;
            n.get(id).unwrap().summary().map(str::to_string)
        };
        assert_eq!(summary("T", "S", "B").as_deref(), Some("B"));
        assert_eq!(summary("T", " S ", "  ").as_deref(), Some("S"));
        assert_eq!(summary("T", "", "").as_deref(), Some("T"));
        assert_eq!(summary(" ", "", "").as_deref(), None);
    }

    #[test]
    fn subtitle_is_stored() {
        let mut n = Notifications::new();
        let id = n
            .add(
                W1,
                S1,
                "T".into(),
                "Permission".into(),
                "b".into(),
                false,
                at(0),
            )
            .id;
        assert_eq!(n.get(id).unwrap().subtitle, "Permission");
    }

    #[test]
    fn remove_read_keeps_unread() {
        let mut n = Notifications::new();
        let a = add(&mut n, W1, S1, "a");
        let b = add(&mut n, W1, S2, "b");
        let c = add(&mut n, W2, S1, "c");
        n.mark_read(a);
        n.mark_read(c);
        let before = n.generation();
        let mut gone = n.remove_read();
        gone.sort();
        assert_eq!(gone, vec![a, c]);
        let left: Vec<_> = n.list().iter().map(|x| x.id).collect();
        assert_eq!(left, vec![b]);
        assert!(n.generation() > before);
        let before = n.generation();
        assert!(n.remove_read().is_empty());
        assert_eq!(n.generation(), before, "nothing visible changed");
    }

    #[test]
    fn desktop_body_joins_subtitle_and_body() {
        let mut n = Notifications::new();
        let mut body = |subtitle: &str, b: &str| {
            let id = n
                .add(W1, S1, "T".into(), subtitle.into(), b.into(), false, at(0))
                .id;
            n.get(id).unwrap().desktop_body()
        };
        assert_eq!(
            body(" Permission ", " Approval needed\n"),
            "Permission\nApproval needed"
        );
        assert_eq!(body("", "b"), "b");
        assert_eq!(body("s", " "), "s");
        assert_eq!(body("", ""), "");
    }

    #[test]
    fn detail_is_body_else_subtitle() {
        let mut n = Notifications::new();
        let mut detail = |subtitle: &str, b: &str| {
            let id = n
                .add(W1, S1, "T".into(), subtitle.into(), b.into(), false, at(0))
                .id;
            n.get(id).unwrap().detail().to_string()
        };
        assert_eq!(detail("s", "b"), "b");
        assert_eq!(detail("s", "  "), "s");
        assert_eq!(detail("", ""), "");
    }
}
