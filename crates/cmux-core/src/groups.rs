//! Workspace groups (M8 plan D1-D7, research-groups.md): named sidebar sections. A group is a
//! run of workspaces in the list, its anchor first; the sidebar draws the anchor as the group's
//! header. `normalize` restores that shape after every change.

use crate::workspaces::{Workspace, WorkspaceId, Workspaces};

/// Ids are never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroupId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: GroupId,
    pub name: String,
    /// The member the header stands for.
    pub anchor: WorkspaceId,
    pub collapsed: bool,
    pub pinned: bool,
    /// The anchor was made for the group (an empty group), not picked by the user.
    pub generated_anchor: bool,
}

/// One sidebar row, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// A group's header; it is also its anchor workspace's row.
    Header { group: GroupId, anchor: WorkspaceId },
    /// `member`: indented under a header.
    Workspace { id: WorkspaceId, member: bool },
}

impl Row {
    /// The workspace selecting this row shows.
    pub fn workspace(self) -> WorkspaceId {
        match self {
            Row::Header { anchor, .. } => anchor,
            Row::Workspace { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupError {
    GroupNotFound,
    WorkspaceNotFound,
    /// Every workspace asked for already anchors a group.
    AllAnchors,
    /// The workspace anchors another group.
    AnchorElsewhere,
    NotInGroup,
    NotAMember,
}

/// Where a new workspace lands inside a group (rg §7.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// After the reference member; without one, right after the anchor.
    #[default]
    AfterCurrent,
    Top,
    End,
}

impl Placement {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_lowercase().as_str() {
            "aftercurrent" | "after-current" | "after_current" => Placement::AfterCurrent,
            "top" => Placement::Top,
            "end" => Placement::End,
            _ => return None,
        })
    }
}

/// What a click on a header's name does (rg §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderClick {
    Select(WorkspaceId),
    ToggleCollapse,
}

impl Workspaces {
    /// Groups in sidebar order.
    pub fn groups(&self) -> Vec<&Group> {
        let mut g: Vec<&Group> = self.groups.iter().collect();
        g.sort_by_key(|g| self.index_of(g.anchor));
        g
    }

    pub fn group(&self, id: GroupId) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    fn group_mut(&mut self, id: GroupId) -> Option<&mut Group> {
        self.groups.iter_mut().find(|g| g.id == id)
    }

    pub fn group_of(&self, ws: WorkspaceId) -> Option<GroupId> {
        self.list.iter().find(|w| w.id == ws)?.group
    }

    pub fn is_anchor(&self, ws: WorkspaceId) -> bool {
        self.groups.iter().any(|g| g.anchor == ws)
    }

    /// Anchor first, then the other members in list order.
    pub fn group_members(&self, id: GroupId) -> Vec<WorkspaceId> {
        self.list
            .iter()
            .filter(|w| w.group == Some(id))
            .map(|w| w.id)
            .collect()
    }

    /// The anchor of the collapsed group hiding `ws`, if it is hidden.
    pub(crate) fn hidden_anchor(&self, ws: WorkspaceId) -> Option<WorkspaceId> {
        let g = self.group(self.group_of(ws)?)?;
        (g.collapsed && g.anchor != ws).then_some(g.anchor)
    }

    pub(crate) fn expand_for_selection(&mut self) {
        let selected = self.selected();
        if let Some(g) = self.group_of(selected).and_then(|g| self.group_mut(g))
            && g.anchor != selected
        {
            g.collapsed = false;
        }
    }

    /// The rows the sidebar draws.
    pub fn rows(&self) -> Vec<Row> {
        self.list
            .iter()
            .filter_map(|w| {
                let Some(g) = w.group.and_then(|g| self.group(g)) else {
                    return Some(Row::Workspace {
                        id: w.id,
                        member: false,
                    });
                };
                if g.anchor == w.id {
                    Some(Row::Header {
                        group: g.id,
                        anchor: w.id,
                    })
                } else {
                    (!g.collapsed).then_some(Row::Workspace {
                        id: w.id,
                        member: true,
                    })
                }
            })
            .collect()
    }

    /// Restores the invariants (rg §1.4): no dangling or empty groups, every anchor a member,
    /// each group one run with its anchor first, pinned groups above everything else. Relative
    /// order is otherwise kept, and the selection stays on the same workspace.
    pub(crate) fn normalize(&mut self) {
        let selected = self.list.get(self.selected).map(|w| w.id);
        for w in &mut self.list {
            if w.group
                .is_some_and(|g| !self.groups.iter().any(|x| x.id == g))
            {
                w.group = None;
            }
        }
        let list = &self.list;
        self.groups.retain_mut(|g| {
            let mut members = list.iter().filter(|w| w.group == Some(g.id));
            let Some(first) = members.next() else {
                return false;
            };
            if first.id != g.anchor && !members.any(|w| w.id == g.anchor) {
                // The anchor left or closed: the next member takes over (rg §6.1).
                g.anchor = first.id;
                g.generated_anchor = false;
            }
            true
        });
        let mut blocks: Vec<(bool, Vec<Workspace>)> = Vec::new();
        let mut done: Vec<GroupId> = Vec::new();
        for w in &self.list {
            match w.group {
                None => blocks.push((false, vec![w.clone()])),
                Some(g) if done.contains(&g) => {}
                Some(g) => {
                    done.push(g);
                    let group = self.group(g).expect("dangling ids were cleared");
                    let mut run: Vec<Workspace> = self
                        .list
                        .iter()
                        .filter(|x| x.group == Some(g))
                        .cloned()
                        .collect();
                    run.sort_by_key(|x| x.id != group.anchor); // stable: anchor first
                    blocks.push((group.pinned, run));
                }
            }
        }
        blocks.sort_by_key(|(pinned, _)| !pinned); // stable partition
        self.list = blocks.into_iter().flat_map(|(_, run)| run).collect();
        self.selected = selected
            .and_then(|id| self.index_of(id))
            .unwrap_or(self.selected.min(self.list.len().saturating_sub(1)));
    }

    /// "Group N": from the group count plus one, skipping names in use (rg §2).
    pub fn next_group_name(&self) -> String {
        let mut n = self.groups.len() + 1;
        loop {
            let name = format!("Group {n}");
            if !self.groups.iter().any(|g| g.name == name) {
                return name;
            }
            n += 1;
        }
    }

    fn push_group(&mut self, name: &str, anchor: WorkspaceId, generated_anchor: bool) -> GroupId {
        let id = GroupId(self.next_group);
        self.next_group += 1;
        let name = match name.trim() {
            "" => self.next_group_name(),
            n => n.to_string(),
        };
        self.groups.push(Group {
            id,
            name,
            anchor,
            collapsed: false,
            pinned: false,
            generated_anchor,
        });
        id
    }

    /// Groups existing workspaces (rg §3): the first in list order becomes the anchor and the
    /// group takes its slot. Workspaces that anchor another group are left out; members of
    /// another group move over. An empty `name` gets the next "Group N". Selection is unchanged.
    pub fn create_group(
        &mut self,
        name: &str,
        members: &[WorkspaceId],
    ) -> Result<GroupId, GroupError> {
        let mut ids: Vec<WorkspaceId> = self
            .list
            .iter()
            .map(|w| w.id)
            .filter(|id| members.contains(id))
            .collect();
        if ids.is_empty() {
            return Err(GroupError::WorkspaceNotFound);
        }
        ids.retain(|id| !self.is_anchor(*id));
        let Some(anchor) = ids.first().copied() else {
            return Err(GroupError::AllAnchors);
        };
        let gid = self.push_group(name, anchor, false);
        for w in &mut self.list {
            if ids.contains(&w.id) {
                w.group = Some(gid);
            }
        }
        self.normalize();
        Ok(gid)
    }

    /// A group with a new anchor workspace on top of the unpinned rows, selected (rg §3.2).
    pub fn create_empty_group(&mut self, name: &str) -> (GroupId, WorkspaceId) {
        let pinned = |w: &Workspace| {
            w.group
                .and_then(|g| self.group(g))
                .is_some_and(|g| g.pinned)
        };
        let at = self.list.iter().filter(|w| pinned(w)).count();
        let ws = self.insert_selected(at);
        let gid = self.push_group(name, ws, true);
        self.list[at].group = Some(gid);
        self.normalize();
        (gid, ws)
    }

    /// Dissolves the group and keeps its workspaces where they are; how many it had.
    pub fn ungroup(&mut self, id: GroupId) -> Option<usize> {
        self.group(id)?;
        let n = self.group_members(id).len();
        self.groups.retain(|g| g.id != id);
        self.normalize();
        Some(n)
    }

    /// The name is trimmed; an empty one is refused.
    pub fn rename_group(&mut self, id: GroupId, name: &str) -> bool {
        let name = name.trim();
        match self.group_mut(id) {
            Some(g) if !name.is_empty() => {
                g.name = name.to_string();
                true
            }
            _ => false,
        }
    }

    /// Collapsing over the selected member selects the anchor (plan D4). False if unknown.
    pub fn set_collapsed(&mut self, id: GroupId, collapsed: bool) -> bool {
        let Some(g) = self.group_mut(id) else {
            return false;
        };
        g.collapsed = collapsed;
        if let Some(anchor) = self.hidden_anchor(self.selected()) {
            self.selected = self.index_of(anchor).expect("anchors are listed");
        }
        true
    }

    /// A newly pinned group goes below the other pinned groups, an unpinned one to the top of
    /// the rest (rg §9.5). False if unknown.
    pub fn set_pinned(&mut self, id: GroupId, pinned: bool) -> bool {
        let Some(g) = self.group_mut(id) else {
            return false;
        };
        g.pinned = pinned;
        self.normalize();
        true
    }

    fn take(&mut self, ws: WorkspaceId) -> Option<Workspace> {
        let selected = self.selected();
        let w = self.list.remove(self.index_of(ws)?);
        // Parked on a valid index; callers re-insert `w` and `reselect`.
        self.selected = self.index_of(selected).unwrap_or(0);
        Some(w)
    }

    fn reselect(&mut self, id: WorkspaceId) {
        self.selected = self.index_of(id).expect("selected id still listed");
    }

    /// The list index right after the group's last member.
    fn run_end(&self, id: GroupId) -> usize {
        self.list
            .iter()
            .rposition(|w| w.group == Some(id))
            .map_or(self.list.len(), |i| i + 1)
    }

    /// `ws` joins the group as its last member (rg §8.11).
    pub fn add_to_group(&mut self, id: GroupId, ws: WorkspaceId) -> Result<(), GroupError> {
        if self.group(id).is_none() {
            return Err(GroupError::GroupNotFound);
        }
        match self.index_of(ws) {
            None => return Err(GroupError::WorkspaceNotFound),
            Some(_) if self.group_of(ws) == Some(id) => return Ok(()),
            Some(_) if self.is_anchor(ws) => return Err(GroupError::AnchorElsewhere),
            Some(_) => {}
        }
        let selected = self.selected();
        let mut w = self.take(ws).expect("checked above");
        w.group = Some(id);
        self.list.insert(self.run_end(id), w);
        self.reselect(selected);
        self.normalize();
        self.expand_for_selection();
        Ok(())
    }

    /// `ws` leaves its group and lands right below it; an anchor dissolves the whole group.
    pub fn remove_from_group(&mut self, ws: WorkspaceId) -> Result<(), GroupError> {
        if self.index_of(ws).is_none() {
            return Err(GroupError::WorkspaceNotFound);
        }
        let Some(g) = self.group_of(ws) else {
            return Err(GroupError::NotInGroup);
        };
        if self.is_anchor(ws) {
            self.ungroup(g);
            return Ok(());
        }
        let selected = self.selected();
        let mut w = self.take(ws).expect("checked above");
        w.group = None;
        self.list.insert(self.run_end(g), w);
        self.reselect(selected);
        self.normalize();
        Ok(())
    }

    pub fn set_anchor(&mut self, id: GroupId, ws: WorkspaceId) -> Result<(), GroupError> {
        if self.group(id).is_none() {
            return Err(GroupError::GroupNotFound);
        }
        if self.group_of(ws) != Some(id) {
            return Err(GroupError::NotAMember);
        }
        let g = self.group_mut(id).expect("checked above");
        if g.anchor != ws {
            g.anchor = ws;
            g.generated_anchor = false;
        }
        self.normalize();
        // The old anchor is a member row now; it may be the selection.
        self.expand_for_selection();
        Ok(())
    }

    /// A new workspace inside the group, selected (rg §7). `reference` matters only for
    /// `AfterCurrent`.
    pub fn add_in_group(
        &mut self,
        id: GroupId,
        placement: Placement,
        reference: Option<WorkspaceId>,
    ) -> Option<WorkspaceId> {
        let anchor = self.index_of(self.group(id)?.anchor)?;
        let after_reference = reference
            .filter(|r| self.group_of(*r) == Some(id))
            .and_then(|r| self.index_of(r));
        let at = match placement {
            Placement::AfterCurrent => after_reference.unwrap_or(anchor) + 1,
            Placement::Top => anchor + 1,
            Placement::End => self.run_end(id),
        };
        let ws = self.insert_selected(at);
        self.list[at].group = Some(id);
        self.normalize();
        self.expand_for_selection();
        Some(ws)
    }

    /// What clicking the header's name does. `anchor_untouched`: nothing was typed into the
    /// anchor and it still has its single first terminal.
    pub fn header_target(&self, id: GroupId, anchor_untouched: bool) -> Option<HeaderClick> {
        let g = self.group(id)?;
        if !(g.generated_anchor && anchor_untouched) {
            return Some(HeaderClick::Select(g.anchor));
        }
        Some(match self.group_members(id).get(1) {
            Some(first) => HeaderClick::Select(*first),
            None => HeaderClick::ToggleCollapse,
        })
    }

    fn order(&self) -> Vec<(WorkspaceId, Option<GroupId>)> {
        self.list.iter().map(|w| (w.id, w.group)).collect()
    }

    /// Moves the selected workspace by `delta` places, without wrapping (plan D6): a member
    /// inside its group; a plain row or a header (with its group) past whole top-level rows,
    /// never across the pinned boundary.
    pub fn move_selected(&mut self, delta: isize) -> bool {
        let before = self.order();
        let id = self.selected();
        let group = self.group_of(id);
        if let Some(g) = group.filter(|_| !self.is_anchor(id)) {
            let first = self
                .index_of(self.group(g).expect("member of it").anchor)
                .unwrap()
                + 1;
            let to = self.selected as isize + delta;
            if to < first as isize || to >= self.run_end(g) as isize {
                return false;
            }
            let w = self.list.remove(self.selected);
            self.list.insert(to as usize, w);
        } else {
            // Top-level blocks: (pinned, workspaces).
            let mut blocks: Vec<(bool, Vec<Workspace>)> = Vec::new();
            for w in &self.list {
                let pinned = w
                    .group
                    .and_then(|g| self.group(g))
                    .is_some_and(|g| g.pinned);
                match blocks.last_mut() {
                    Some((_, run)) if w.group.is_some() && run[0].group == w.group => {
                        run.push(w.clone())
                    }
                    _ => blocks.push((pinned, vec![w.clone()])),
                }
            }
            let from = blocks
                .iter()
                .position(|(_, run)| run[0].id == id)
                .expect("a plain row or an anchor starts a block");
            let to = from as isize + delta;
            if to < 0 || to >= blocks.len() as isize || blocks[to as usize].0 != blocks[from].0 {
                return false;
            }
            let b = blocks.remove(from);
            blocks.insert(to as usize, b);
            self.list = blocks.into_iter().flat_map(|(_, run)| run).collect();
        }
        self.reselect(id);
        self.order() != before
    }

    /// Drops the row of `id` into `gap` (0..=rows, counted in `rows()`), plan D7. A header
    /// takes its group along and stays top-level. Any other row joins a group only when
    /// dropped right above one of its visible members; otherwise it becomes (or stays) a plain
    /// row. The selection follows ids.
    pub fn move_to(&mut self, id: WorkspaceId, gap: usize) -> bool {
        let rows = self.rows();
        if gap > rows.len() || self.index_of(id).is_none() {
            return false;
        }
        let next = rows.get(gap).copied();
        let prev = gap.checked_sub(1).map(|k| rows[k]);
        if [prev, next].iter().flatten().any(|r| r.workspace() == id) {
            return false; // dropped where it already is
        }
        let before = self.order();
        let selected = self.selected();
        let next_group = match next {
            Some(Row::Workspace { id, member: true }) => self.group_of(id),
            _ => None,
        };
        if let Some(g) = self.group_of(id).filter(|_| self.is_anchor(id)) {
            if next_group == Some(g) {
                return false; // into itself
            }
            let (run, rest): (Vec<_>, Vec<_>) =
                self.list.drain(..).partition(|w| w.group == Some(g));
            self.list = rest;
            let at = match (next_group, next) {
                // Above a member of another group: below that group instead.
                (Some(other), _) => self.run_end(other),
                (None, Some(r)) => self.index_of(r.workspace()).expect("row is listed"),
                (None, None) => self.list.len(),
            };
            self.list.splice(at..at, run);
        } else {
            let mut w = self.take(id).expect("checked above");
            w.group = next_group;
            let at = next
                .and_then(|r| self.index_of(r.workspace()))
                .unwrap_or(self.list.len());
            self.list.insert(at, w);
        }
        self.reselect(selected);
        self.normalize();
        self.expand_for_selection();
        self.order() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `n` workspaces `[1..=n]`, the first selected.
    fn list(n: u64) -> Workspaces {
        let mut w = Workspaces::new();
        for _ in 1..n {
            w.add_at_end();
        }
        w.select(WorkspaceId(1));
        w
    }

    fn ws(ids: &[u64]) -> Vec<WorkspaceId> {
        ids.iter().map(|i| WorkspaceId(*i)).collect()
    }

    fn ids(w: &Workspaces) -> Vec<u64> {
        w.list().iter().map(|x| x.id.0).collect()
    }

    /// Rows as text: `G1[2]` a header with anchor 2, `.3` a member, `4` a plain row.
    fn rows(w: &Workspaces) -> String {
        w.rows()
            .iter()
            .map(|r| match r {
                Row::Header { group, anchor } => format!("G{}[{}]", group.0, anchor.0),
                Row::Workspace { id, member: true } => format!(".{}", id.0),
                Row::Workspace { id, member: false } => id.0.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn create_group_takes_the_first_members_slot() {
        let mut w = list(5);
        let g = w.create_group("", &ws(&[4, 2])).unwrap();
        assert_eq!(rows(&w), "1 G1[2] .4 3 5");
        assert_eq!(w.group(g).unwrap().name, "Group 1");
        assert!(!w.group(g).unwrap().generated_anchor);
        assert_eq!(w.selected(), WorkspaceId(1), "selection untouched");
        assert_eq!(w.group_members(g), ws(&[2, 4]));
    }

    #[test]
    fn create_group_skips_anchors_and_moves_members_over() {
        let mut w = list(4);
        let g1 = w.create_group("a", &ws(&[1, 2, 3])).unwrap();
        assert_eq!(
            w.create_group("b", &ws(&[1])),
            Err(GroupError::AllAnchors),
            "1 anchors a"
        );
        assert_eq!(
            w.create_group("b", &ws(&[9])),
            Err(GroupError::WorkspaceNotFound)
        );
        let g2 = w.create_group("b", &ws(&[1, 3, 4])).unwrap();
        assert_eq!(rows(&w), "G1[1] .2 G2[3] .4");
        assert_eq!(w.group_members(g1), ws(&[1, 2]));
        assert_eq!(w.group_members(g2), ws(&[3, 4]));
    }

    #[test]
    fn auto_names_start_at_count_plus_one_and_skip_taken() {
        let mut w = list(4);
        let g1 = w.create_group("", &ws(&[1])).unwrap();
        w.create_group("Group 3", &ws(&[2])).unwrap();
        let g3 = w.create_group("", &ws(&[3])).unwrap();
        assert_eq!(w.group(g3).unwrap().name, "Group 4", "3 is taken");
        w.ungroup(g1);
        let g = w.create_group("  ", &ws(&[1])).unwrap();
        assert_eq!(w.group(g).unwrap().name, "Group 5", "no gap filling");
    }

    #[test]
    fn empty_group_gets_a_selected_anchor_below_the_pinned_groups() {
        let mut w = list(3);
        let g1 = w.create_group("p", &ws(&[3])).unwrap();
        w.set_pinned(g1, true);
        assert_eq!(rows(&w), "G1[3] 1 2");
        let (g2, anchor) = w.create_empty_group("");
        assert_eq!(rows(&w), "G1[3] G2[4] 1 2");
        assert_eq!(w.selected(), anchor);
        assert!(w.group(g2).unwrap().generated_anchor);
        assert_eq!(w.group(g2).unwrap().name, "Group 2");
    }

    #[test]
    fn ungroup_keeps_the_order() {
        let mut w = list(4);
        let g = w.create_group("", &ws(&[2, 3])).unwrap();
        assert_eq!(w.ungroup(g), Some(2));
        assert_eq!(rows(&w), "1 2 3 4");
        assert!(w.groups().is_empty());
        assert_eq!(w.ungroup(g), None);
    }

    #[test]
    fn rename_group_trims_and_refuses_empty() {
        let mut w = list(1);
        let g = w.create_group("a", &ws(&[1])).unwrap();
        assert!(w.rename_group(g, "  api "));
        assert_eq!(w.group(g).unwrap().name, "api");
        assert!(!w.rename_group(g, "  "));
        assert!(!w.rename_group(GroupId(9), "x"));
    }

    #[test]
    fn collapse_hides_members_and_hands_the_selection_to_the_anchor() {
        let mut w = list(4);
        let g = w.create_group("", &ws(&[2, 3])).unwrap();
        w.select(WorkspaceId(3));
        assert!(w.set_collapsed(g, true));
        assert_eq!(rows(&w), "1 G1[2] 4");
        assert_eq!(w.selected(), WorkspaceId(2));
        // Selecting the anchor keeps it collapsed; a hidden member expands it.
        w.select(WorkspaceId(1));
        w.select(WorkspaceId(2));
        assert!(w.group(g).unwrap().collapsed);
        w.select(WorkspaceId(3));
        assert!(!w.group(g).unwrap().collapsed);
    }

    #[test]
    fn cycling_walks_hidden_members_and_expands() {
        let mut w = list(3);
        let g = w.create_group("", &ws(&[1, 2])).unwrap();
        w.set_collapsed(g, true);
        assert!(w.cycle(true));
        assert_eq!(w.selected(), WorkspaceId(2));
        assert!(!w.group(g).unwrap().collapsed);
    }

    #[test]
    fn number_keys_count_plain_visible_rows() {
        let mut w = list(5);
        let g = w.create_group("", &ws(&[2, 3])).unwrap();
        // Rows: 1 G[2] .3 4 5
        assert!(w.select_number(2));
        assert_eq!(w.selected(), WorkspaceId(3), "the header is not counted");
        w.select(WorkspaceId(1));
        w.set_collapsed(g, true);
        assert!(w.select_number(2));
        assert_eq!(
            w.selected(),
            WorkspaceId(4),
            "hidden members are not counted"
        );
        assert!(w.select_number(9));
        assert_eq!(w.selected(), WorkspaceId(5));
    }

    #[test]
    fn closing_the_anchor_promotes_the_next_member() {
        let mut w = list(4);
        let (g, anchor) = w.create_empty_group("x");
        w.add_to_group(g, WorkspaceId(2)).unwrap();
        w.add_to_group(g, WorkspaceId(3)).unwrap();
        w.close(anchor);
        let group = w.group(g).unwrap();
        assert_eq!(group.anchor, WorkspaceId(2));
        assert_eq!(group.name, "x", "the header keeps the group's name");
        assert!(!group.generated_anchor);
        assert_eq!(rows(&w), "G1[2] .3 1 4");
        w.close(WorkspaceId(3));
        w.close(WorkspaceId(2));
        assert!(
            w.groups().is_empty(),
            "the last member took the group along"
        );
        assert_eq!(rows(&w), "1 4");
    }

    #[test]
    fn closing_never_leaves_a_hidden_selection() {
        let mut w = list(3);
        let g = w.create_group("", &ws(&[1, 2])).unwrap();
        w.set_collapsed(g, true);
        w.select(WorkspaceId(3));
        // No slot left: the new last is the hidden member 2, so its anchor is selected.
        w.close(WorkspaceId(3));
        assert_eq!(w.selected(), WorkspaceId(1));
        assert!(w.group(g).unwrap().collapsed);
        // Closing the selected anchor: the promoted member is the header in the same slot.
        w.close(WorkspaceId(1));
        assert_eq!(w.selected(), WorkspaceId(2));
        assert_eq!(rows(&w), "G1[2]");
    }

    #[test]
    fn pinned_groups_sit_on_top_in_pin_order() {
        let mut w = list(4);
        let a = w.create_group("a", &ws(&[2])).unwrap();
        let b = w.create_group("b", &ws(&[4])).unwrap();
        w.set_pinned(b, true);
        assert_eq!(rows(&w), "G2[4] 1 G1[2] 3");
        w.set_pinned(a, true);
        assert_eq!(
            rows(&w),
            "G2[4] G1[2] 1 3",
            "newly pinned goes last of the pinned"
        );
        w.set_pinned(b, false);
        assert_eq!(
            rows(&w),
            "G1[2] G2[4] 1 3",
            "unpinned goes first of the rest"
        );
        assert_eq!(
            w.groups().iter().map(|g| g.id).collect::<Vec<_>>(),
            vec![a, b]
        );
    }

    #[test]
    fn add_and_remove_members() {
        let mut w = list(5);
        let g = w.create_group("", &ws(&[2, 3])).unwrap();
        let h = w.create_group("", &ws(&[5])).unwrap();
        assert_eq!(w.add_to_group(g, WorkspaceId(1)), Ok(()));
        assert_eq!(rows(&w), "G1[2] .3 .1 4 G2[5]", "joins at the end");
        assert_eq!(
            w.add_to_group(g, WorkspaceId(1)),
            Ok(()),
            "already in: no-op"
        );
        assert_eq!(
            w.add_to_group(g, WorkspaceId(5)),
            Err(GroupError::AnchorElsewhere)
        );
        assert_eq!(
            w.add_to_group(GroupId(9), WorkspaceId(4)),
            Err(GroupError::GroupNotFound)
        );
        assert_eq!(
            w.add_to_group(g, WorkspaceId(9)),
            Err(GroupError::WorkspaceNotFound)
        );
        assert_eq!(w.remove_from_group(WorkspaceId(3)), Ok(()));
        assert_eq!(
            rows(&w),
            "G1[2] .1 3 4 G2[5]",
            "lands right below the group"
        );
        assert_eq!(
            w.remove_from_group(WorkspaceId(4)),
            Err(GroupError::NotInGroup)
        );
        assert_eq!(w.remove_from_group(WorkspaceId(5)), Ok(()));
        assert!(
            w.group(h).is_none(),
            "removing an anchor dissolves its group"
        );
    }

    #[test]
    fn set_anchor_moves_the_member_to_the_front() {
        let mut w = list(3);
        let g = w.create_group("", &ws(&[1, 2, 3])).unwrap();
        assert_eq!(w.set_anchor(g, WorkspaceId(3)), Ok(()));
        assert_eq!(rows(&w), "G1[3] .1 .2");
        let other = w.add_at_end();
        assert_eq!(w.set_anchor(g, other), Err(GroupError::NotAMember));
        assert_eq!(
            w.set_anchor(GroupId(9), other),
            Err(GroupError::GroupNotFound)
        );
    }

    #[test]
    fn new_workspace_in_group_placements() {
        let mut w = list(4);
        let g = w.create_group("", &ws(&[1, 2, 3])).unwrap();
        let a = w
            .add_in_group(g, Placement::AfterCurrent, Some(WorkspaceId(2)))
            .unwrap();
        assert_eq!(rows(&w), format!("G1[1] .2 .{} .3 4", a.0));
        assert_eq!(w.selected(), a);
        let b = w.add_in_group(g, Placement::AfterCurrent, None).unwrap();
        assert_eq!(
            w.group_members(g)[1],
            b,
            "no reference: right after the anchor"
        );
        let c = w
            .add_in_group(g, Placement::AfterCurrent, Some(WorkspaceId(4)))
            .unwrap();
        assert_eq!(
            w.group_members(g)[1],
            c,
            "a reference outside the group is ignored"
        );
        let d = w
            .add_in_group(g, Placement::Top, Some(WorkspaceId(3)))
            .unwrap();
        assert_eq!(w.group_members(g)[1], d);
        let e = w.add_in_group(g, Placement::End, None).unwrap();
        assert_eq!(*w.group_members(g).last().unwrap(), e);
        assert_eq!(*ids(&w).last().unwrap(), 4, "4 stays below the group");
        assert_eq!(w.add_in_group(GroupId(9), Placement::Top, None), None);
    }

    #[test]
    fn new_workspace_in_a_collapsed_group_expands_it() {
        let mut w = list(2);
        let g = w.create_group("", &ws(&[1, 2])).unwrap();
        w.set_collapsed(g, true);
        w.add_in_group(g, Placement::End, None).unwrap();
        assert!(!w.group(g).unwrap().collapsed);
    }

    #[test]
    fn add_after_selected_never_splits_a_group() {
        let mut w = list(3);
        w.create_group("", &ws(&[1, 2])).unwrap();
        let n = w.add_after_selected(); // selected 1, the anchor
        assert_eq!(rows(&w), format!("G1[1] .2 {} 3", n.0));
        assert_eq!(w.selected(), n);
    }

    #[test]
    fn placement_spellings() {
        for s in ["afterCurrent", "after-current", "AFTER_CURRENT"] {
            assert_eq!(Placement::parse(s), Some(Placement::AfterCurrent));
        }
        assert_eq!(Placement::parse("Top"), Some(Placement::Top));
        assert_eq!(Placement::parse("end"), Some(Placement::End));
        assert_eq!(Placement::parse("bottom"), None);
    }

    #[test]
    fn header_click_skips_an_untouched_generated_anchor() {
        let mut w = list(2);
        let (g, anchor) = w.create_empty_group("");
        assert_eq!(
            w.header_target(g, true),
            Some(HeaderClick::ToggleCollapse),
            "nothing else to show"
        );
        assert_eq!(w.header_target(g, false), Some(HeaderClick::Select(anchor)));
        w.add_to_group(g, WorkspaceId(2)).unwrap();
        assert_eq!(
            w.header_target(g, true),
            Some(HeaderClick::Select(WorkspaceId(2)))
        );
        let picked = w.create_group("", &ws(&[1])).unwrap();
        assert_eq!(
            w.header_target(picked, true),
            Some(HeaderClick::Select(WorkspaceId(1))),
            "a user-picked anchor is never skipped"
        );
        assert_eq!(w.header_target(GroupId(9), true), None);
    }

    #[test]
    fn keyboard_move_keeps_members_inside_their_group() {
        let mut w = list(5);
        w.create_group("", &ws(&[2, 3, 4])).unwrap();
        w.select(WorkspaceId(3));
        assert!(
            !w.move_selected(-1),
            "the anchor's slot is not a member slot"
        );
        assert!(w.move_selected(1));
        assert_eq!(rows(&w), "1 G1[2] .4 .3 5");
        assert!(!w.move_selected(1), "cannot leave the group");
        assert_eq!(w.selected(), WorkspaceId(3));
    }

    #[test]
    fn keyboard_move_swaps_top_level_rows() {
        let mut w = list(4);
        let g = w.create_group("", &ws(&[2, 3])).unwrap();
        assert!(w.move_selected(1), "1 hops over the whole group");
        assert_eq!(rows(&w), "G1[2] .3 1 4");
        w.select(WorkspaceId(2));
        assert!(w.move_selected(1), "the header takes its group along");
        assert_eq!(rows(&w), "1 G1[2] .3 4");
        assert_eq!(w.selected(), WorkspaceId(2));
        assert!(w.move_selected(1));
        assert!(!w.move_selected(1), "already last");
        w.set_pinned(g, true);
        assert_eq!(rows(&w), "G1[2] .3 1 4");
        assert!(!w.move_selected(1), "pinned rows stay above the rest");
        w.select(WorkspaceId(1));
        assert!(!w.move_selected(-1));
    }

    #[test]
    fn drag_a_row_into_and_out_of_a_group() {
        let mut w = list(5);
        let g = w.create_group("", &ws(&[2, 3])).unwrap();
        // Rows: 1 G[2] .3 4 5
        assert!(w.move_to(WorkspaceId(5), 2), "above member 3: joins");
        assert_eq!(rows(&w), "1 G1[2] .5 .3 4");
        assert!(
            w.move_to(WorkspaceId(5), 4),
            "below the last member: root level"
        );
        assert_eq!(rows(&w), "1 G1[2] .3 5 4");
        assert_eq!(w.group_of(WorkspaceId(5)), None);
        assert!(w.move_to(WorkspaceId(3), 0));
        assert_eq!(rows(&w), "3 1 G1[2] 5 4");
        assert_eq!(w.group_members(g), ws(&[2]));
        assert!(!w.move_to(WorkspaceId(3), 0), "already there");
        assert!(!w.move_to(WorkspaceId(3), 1));
        assert!(!w.move_to(WorkspaceId(3), 9));
    }

    #[test]
    fn drag_below_a_collapsed_header_stays_out_of_the_group() {
        let mut w = list(4);
        let g = w.create_group("", &ws(&[1, 2])).unwrap();
        w.set_collapsed(g, true);
        // Rows: G[1] 3 4
        assert!(w.move_to(WorkspaceId(4), 1));
        assert_eq!(ids(&w), vec![1, 2, 4, 3], "below the hidden member");
        assert_eq!(w.group_of(WorkspaceId(4)), None);
    }

    #[test]
    fn drag_a_header_moves_the_whole_group() {
        let mut w = list(6);
        let g = w.create_group("", &ws(&[1, 2])).unwrap();
        w.create_group("", &ws(&[4, 5])).unwrap();
        // Rows: G1[1] .2 3 G2[4] .5 6
        assert!(!w.move_to(WorkspaceId(1), 1), "above its own member");
        assert!(w.move_to(WorkspaceId(1), 3));
        assert_eq!(rows(&w), "3 G1[1] .2 G2[4] .5 6");
        assert!(
            w.move_to(WorkspaceId(1), 4),
            "above member 5: lands below that group"
        );
        assert_eq!(rows(&w), "3 G2[4] .5 G1[1] .2 6");
        assert!(w.move_to(WorkspaceId(1), 6));
        assert_eq!(rows(&w), "3 G2[4] .5 6 G1[1] .2");
        assert_eq!(w.group_members(g), ws(&[1, 2]));
        assert_eq!(w.selected(), WorkspaceId(1));
    }

    #[test]
    fn dragging_the_selected_member_out_keeps_it_selected() {
        let mut w = list(3);
        w.create_group("", &ws(&[1, 2])).unwrap();
        w.select(WorkspaceId(2));
        assert!(w.move_to(WorkspaceId(2), 3));
        assert_eq!(rows(&w), "G1[1] 3 2");
        assert_eq!(w.selected(), WorkspaceId(2));
    }
}
