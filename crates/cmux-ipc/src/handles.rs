//! Socket handles (rc §6, plan D4): an uppercase UUID and a `<kind>:<n>` ref per object,
//! minted on first sight and never reused within a run.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Window,
    Workspace,
    Pane,
    Surface,
    WorkspaceGroup,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Window => "window",
            Kind::Workspace => "workspace",
            Kind::Pane => "pane",
            Kind::Surface => "surface",
            Kind::WorkspaceGroup => "workspace_group",
        }
    }
}

/// `(workspace, id)`: pane and surface ids restart per workspace; a workspace or a workspace
/// group is `(id, 0)`, the window `(0, 0)`.
pub type Key = (u64, u64);

#[derive(Debug, Clone, PartialEq)]
pub struct Handle {
    /// Uppercase UUID, as Swift's `uuidString`.
    pub id: String,
    pub r#ref: String,
}

#[derive(Debug, Default)]
pub struct Handles {
    known: HashMap<(Kind, Key), Handle>,
    counts: HashMap<Kind, u64>,
    notifications: HashMap<u64, String>,
}

impl Handles {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle(&mut self, kind: Kind, key: Key) -> Handle {
        let counts = &mut self.counts;
        self.known
            .entry((kind, key))
            .or_insert_with(|| {
                let n = counts.entry(kind).or_insert(0);
                *n += 1;
                Handle {
                    id: new_uuid(),
                    r#ref: format!("{}:{n}", kind.name()),
                }
            })
            .clone()
    }

    pub fn notification(&mut self, n: u64) -> String {
        self.notifications.entry(n).or_insert_with(new_uuid).clone()
    }
}

fn new_uuid() -> String {
    uuid::Uuid::new_v4()
        .hyphenated()
        .encode_upper(&mut uuid::Uuid::encode_buffer())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_upper_v4(id: &str) -> bool {
        let b = id.as_bytes();
        id.len() == 36
            && [8, 13, 18, 23].iter().all(|&i| b[i] == b'-')
            && b[14] == b'4'
            && matches!(b[19], b'8' | b'9' | b'A' | b'B')
            && id
                .chars()
                .all(|c| c == '-' || c.is_ascii_digit() || ('A'..='F').contains(&c))
    }

    #[test]
    fn refs_count_per_kind_from_1() {
        let mut h = Handles::new();
        let a = h.handle(Kind::Workspace, (1, 0));
        let b = h.handle(Kind::Workspace, (2, 0));
        assert_eq!(a.r#ref, "workspace:1");
        assert_eq!(b.r#ref, "workspace:2");
        assert_eq!(h.handle(Kind::Pane, (1, 1)).r#ref, "pane:1");
        assert_eq!(h.handle(Kind::Surface, (2, 1)).r#ref, "surface:1");
        assert_eq!(h.handle(Kind::Window, (0, 0)).r#ref, "window:1");
        assert_eq!(h.handle(Kind::Workspace, (1, 0)), a);
        assert_ne!(a.id, b.id);
        assert!(is_upper_v4(&a.id), "{}", a.id);
    }

    #[test]
    fn notification_uuid_is_stable() {
        let mut h = Handles::new();
        let n = h.notification(5);
        assert!(is_upper_v4(&n));
        assert_eq!(h.notification(5), n);
        assert_ne!(h.notification(6), n);
    }
}
