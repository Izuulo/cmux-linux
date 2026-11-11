//! The agent Feed: permission / plan / question prompts from Claude Code (M7). A newest-first,
//! capped store keyed by the hook's request id.

use std::collections::VecDeque;

use crate::{SurfaceId, WorkspaceId};

const CAP: usize = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeedItemId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preview {
    Command {
        command: String,
        description: Option<String>,
    },
    File {
        path: String,
        first_line: Option<String>,
    },
    Raw(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionOption {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionPrompt {
    pub header: String,
    pub question: String,
    pub multi_select: bool,
    pub options: Vec<QuestionOption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedKind {
    Permission {
        tool: String,
        preview: Preview,
    },
    ExitPlan {
        plan: String,
        allowed_prompts: Vec<String>,
        plan_file: Option<String>,
    },
    Question {
        prompts: Vec<QuestionPrompt>,
    },
}

impl FeedKind {
    /// Whether `d` answers this kind of prompt (a question reply never answers a permission).
    pub fn accepts(&self, d: &Decision) -> bool {
        matches!(
            (self, d),
            (FeedKind::Permission { .. }, Decision::Permission(_))
                | (FeedKind::ExitPlan { .. }, Decision::ExitPlan { .. })
                | (FeedKind::Question { .. }, Decision::Question { .. })
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    Once,
    Always,
    All,
    Bypass,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanMode {
    Ultraplan,
    BypassPermissions,
    AutoAccept,
    Manual,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Permission(PermissionMode),
    ExitPlan {
        mode: PlanMode,
        feedback: Option<String>,
    },
    /// One entry per question, in order.
    Question {
        selections: Vec<String>,
    },
}

impl Decision {
    /// The resolved item's one-word label (cmux `FP:13-27` for permissions).
    pub fn summary(&self) -> String {
        match self {
            Decision::Permission(m) => match m {
                PermissionMode::Once => "once",
                PermissionMode::Always => "always",
                PermissionMode::All => "all tools",
                PermissionMode::Bypass => "bypass",
                PermissionMode::Deny => "denied",
            },
            Decision::ExitPlan {
                mode: PlanMode::Deny,
                ..
            } => "denied",
            Decision::ExitPlan {
                feedback: Some(_), ..
            } => "feedback sent",
            Decision::ExitPlan { mode, .. } => match mode {
                PlanMode::Ultraplan => "ultraplan",
                PlanMode::Manual => "manual",
                _ => "auto",
            },
            Decision::Question { .. } => "answered",
        }
        .into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedStatus {
    Pending,
    Resolved(Decision),
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewItem {
    pub request_id: String,
    pub source: String,
    pub workspace: WorkspaceId,
    pub surface: SurfaceId,
    pub cwd: String,
    pub created_ms: u64,
    pub kind: FeedKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedItem {
    pub id: FeedItemId,
    pub request_id: String,
    pub source: String,
    pub workspace: WorkspaceId,
    pub surface: SurfaceId,
    pub cwd: String,
    pub created_ms: u64,
    pub kind: FeedKind,
    pub status: FeedStatus,
}

impl FeedItem {
    /// "<cwd basename> · <tool | Plan | first question header>"; just the tail if cwd is empty.
    pub fn title(&self) -> String {
        let tail = match &self.kind {
            FeedKind::Permission { tool, .. } => tool.as_str(),
            FeedKind::ExitPlan { .. } => "Plan",
            FeedKind::Question { prompts } => prompts.first().map_or("Question", |p| &p.header),
        };
        match self.cwd.trim_end_matches('/').rsplit('/').next() {
            Some(base) if !base.is_empty() => format!("{base} · {tail}"),
            _ => tail.into(),
        }
    }
}

#[derive(Default)]
pub struct FeedStore {
    items: VecDeque<FeedItem>,
    next_id: u64,
}

impl FeedStore {
    /// None if the request id is already known.
    pub fn push(&mut self, n: NewItem) -> Option<FeedItemId> {
        if self.items.iter().any(|i| i.request_id == n.request_id) {
            return None;
        }
        self.next_id += 1;
        let id = FeedItemId(self.next_id);
        self.items.push_front(FeedItem {
            id,
            request_id: n.request_id,
            source: n.source,
            workspace: n.workspace,
            surface: n.surface,
            cwd: n.cwd,
            created_ms: n.created_ms,
            kind: n.kind,
            status: FeedStatus::Pending,
        });
        self.items.truncate(CAP);
        Some(id)
    }

    fn pending(&mut self, request_id: &str) -> Option<&mut FeedItem> {
        self.items
            .iter_mut()
            .find(|i| i.request_id == request_id && i.status == FeedStatus::Pending)
    }

    /// Only a pending item resolves.
    pub fn resolve(&mut self, request_id: &str, d: Decision) -> Option<FeedItemId> {
        let i = self.pending(request_id)?;
        i.status = FeedStatus::Resolved(d);
        Some(i.id)
    }

    /// Only a pending item expires.
    pub fn expire(&mut self, request_id: &str) -> Option<FeedItemId> {
        let i = self.pending(request_id)?;
        i.status = FeedStatus::Expired;
        Some(i.id)
    }

    /// Request ids that were pending in `ws`.
    pub fn expire_workspace(&mut self, ws: WorkspaceId) -> Vec<String> {
        let mut out = vec![];
        for i in &mut self.items {
            if i.workspace == ws && i.status == FeedStatus::Pending {
                i.status = FeedStatus::Expired;
                out.push(i.request_id.clone());
            }
        }
        out
    }

    /// Newest first.
    pub fn items(&self) -> impl Iterator<Item = &FeedItem> {
        self.items.iter()
    }

    pub fn get(&self, id: FeedItemId) -> Option<&FeedItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn pending_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == FeedStatus::Pending)
            .count()
    }
}

/// "<1m", "5m", "2h", "3d".
pub fn age_label(now_ms: u64, created_ms: u64) -> String {
    let m = now_ms.saturating_sub(created_ms) / 60_000;
    match m {
        0 => "<1m".into(),
        1..=59 => format!("{m}m"),
        60..=1439 => format!("{}h", m / 60),
        _ => format!("{}d", m / 1440),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new(req: &str, ws: u64, kind: FeedKind) -> NewItem {
        NewItem {
            request_id: req.into(),
            source: "claude".into(),
            workspace: WorkspaceId(ws),
            surface: SurfaceId(1),
            cwd: "/home/u/app".into(),
            created_ms: 0,
            kind,
        }
    }
    fn perm(req: &str, ws: u64) -> NewItem {
        new(
            req,
            ws,
            FeedKind::Permission {
                tool: "Bash".into(),
                preview: Preview::Raw("x".into()),
            },
        )
    }
    fn ids(s: &FeedStore) -> Vec<String> {
        s.items().map(|i| i.request_id.clone()).collect()
    }

    #[test]
    fn push_dedupes_by_request_id() {
        let mut s = FeedStore::default();
        assert!(s.push(perm("a", 1)).is_some());
        assert!(s.push(perm("a", 1)).is_none());
        assert_eq!(s.items().count(), 1);
    }

    #[test]
    fn items_are_newest_first() {
        let mut s = FeedStore::default();
        s.push(perm("a", 1));
        s.push(perm("b", 1));
        assert_eq!(ids(&s), ["b", "a"]);
    }

    #[test]
    fn resolve_marks_pending_only() {
        let mut s = FeedStore::default();
        let id = s.push(perm("a", 1)).unwrap();
        let d = Decision::Permission(PermissionMode::Once);
        assert_eq!(s.resolve("a", d.clone()), Some(id));
        assert_eq!(s.get(id).unwrap().status, FeedStatus::Resolved(d.clone()));
        assert_eq!(s.resolve("a", d), None);
    }

    #[test]
    fn resolve_after_expire_is_ignored() {
        let mut s = FeedStore::default();
        let id = s.push(perm("a", 1)).unwrap();
        assert_eq!(s.expire("a"), Some(id));
        assert_eq!(
            s.resolve("a", Decision::Permission(PermissionMode::Deny)),
            None
        );
        assert_eq!(s.get(id).unwrap().status, FeedStatus::Expired);
        assert_eq!(s.expire("a"), None);
    }

    #[test]
    fn expire_workspace_returns_pending_requests() {
        let mut s = FeedStore::default();
        s.push(perm("a", 1));
        s.push(perm("b", 1));
        s.push(perm("c", 1));
        s.push(perm("d", 2));
        s.resolve("c", Decision::Permission(PermissionMode::Once));
        let mut got = s.expire_workspace(WorkspaceId(1));
        got.sort();
        assert_eq!(got, ["a", "b"]);
        assert_eq!(s.pending_count(), 1);
    }

    #[test]
    fn pending_count_counts_pending() {
        let mut s = FeedStore::default();
        s.push(perm("a", 1));
        s.push(perm("b", 1));
        assert_eq!(s.pending_count(), 2);
        s.expire("a");
        assert_eq!(s.pending_count(), 1);
    }

    #[test]
    fn cap_drops_oldest() {
        let mut s = FeedStore::default();
        for i in 0..2001 {
            s.push(perm(&i.to_string(), 1));
        }
        assert_eq!(s.items().count(), 2000);
        assert!(s.items().all(|i| i.request_id != "0"));
    }

    #[test]
    fn age_labels() {
        assert_eq!(age_label(0, 0), "<1m");
        assert_eq!(age_label(59_999, 0), "<1m");
        assert_eq!(age_label(5 * 60_000, 0), "5m");
        assert_eq!(age_label(2 * 3_600_000, 0), "2h");
        assert_eq!(age_label(3 * 86_400_000, 0), "3d");
        assert_eq!(age_label(0, 10), "<1m");
    }

    #[test]
    fn titles() {
        let mut s = FeedStore::default();
        let a = s.push(perm("a", 1)).unwrap();
        let p = s
            .push(new(
                "p",
                1,
                FeedKind::ExitPlan {
                    plan: "x".into(),
                    allowed_prompts: vec![],
                    plan_file: None,
                },
            ))
            .unwrap();
        let q = s
            .push(new(
                "q",
                1,
                FeedKind::Question {
                    prompts: vec![QuestionPrompt {
                        header: "Auth".into(),
                        question: "?".into(),
                        multi_select: false,
                        options: vec![],
                    }],
                },
            ))
            .unwrap();
        assert_eq!(s.get(a).unwrap().title(), "app · Bash");
        assert_eq!(s.get(p).unwrap().title(), "app · Plan");
        assert_eq!(s.get(q).unwrap().title(), "app · Auth");
        let mut n = perm("e", 1);
        n.cwd.clear();
        let e = s.push(n).unwrap();
        assert_eq!(s.get(e).unwrap().title(), "Bash");
    }

    #[test]
    fn kinds_accept_only_their_own_decision() {
        let perm = FeedKind::Permission {
            tool: "Bash".into(),
            preview: Preview::Raw(String::new()),
        };
        let plan = FeedKind::ExitPlan {
            plan: String::new(),
            allowed_prompts: vec![],
            plan_file: None,
        };
        let question = FeedKind::Question { prompts: vec![] };
        let p = Decision::Permission(PermissionMode::Once);
        let e = Decision::ExitPlan {
            mode: PlanMode::Manual,
            feedback: None,
        };
        let q = Decision::Question { selections: vec![] };
        assert!(perm.accepts(&p) && !perm.accepts(&e) && !perm.accepts(&q));
        assert!(plan.accepts(&e) && !plan.accepts(&p) && !plan.accepts(&q));
        assert!(question.accepts(&q) && !question.accepts(&p) && !question.accepts(&e));
    }

    #[test]
    fn summaries() {
        assert_eq!(
            Decision::Permission(PermissionMode::All).summary(),
            "all tools"
        );
        assert_eq!(
            Decision::Permission(PermissionMode::Deny).summary(),
            "denied"
        );
        let d = |mode, feedback| Decision::ExitPlan { mode, feedback };
        assert_eq!(d(PlanMode::AutoAccept, None).summary(), "auto");
        assert_eq!(
            d(PlanMode::Manual, Some("x".into())).summary(),
            "feedback sent"
        );
        assert_eq!(
            Decision::Question { selections: vec![] }.summary(),
            "answered"
        );
    }
}
