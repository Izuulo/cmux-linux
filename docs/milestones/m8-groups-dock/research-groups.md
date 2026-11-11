# Research: upstream cmux workspace groups

Source: upstream cmux @ `88b87c05` (2026-11-11), GPL-3.0-or-later parts only. Paths are relative to the cmux repo root.
Abbreviations: `WGC` = `Packages/macOS/CmuxWorkspaces/Sources/CmuxWorkspaces/Coordinators/WorkspaceGroupCoordinator.swift`,
`INV` = `.../CmuxWorkspaces/Model/WorkspacesModel+GroupInvariants.swift`, `WRC` = `.../Coordinators/WorkspaceReorderCoordinator.swift`,
`HDR` = `Sources/Sidebar/AppKitList/Cells/SidebarGroupHeaderRowView.swift`, `VTS` = `Sources/VerticalTabsSidebar+WorkspaceGroups.swift`,
`SOCK` = `Packages/macOS/CmuxControlSocket/Sources/CmuxControlSocket/Coordinator/WorkspaceGroup/ControlCommandCoordinator+WorkspaceGroup.swift`.
"(derived)" = concluded by tracing code, not confirmed by an upstream test.

## 1. Data model
1. `WorkspaceGroup` (`.../CmuxWorkspaces/Values/WorkspaceGroup.swift`): `id: UUID`, `name`, `isCollapsed`, `isPinned`, `anchor: WorkspaceGroupAnchor` (`.workspace(UUID)` | `.empty(UUID)`), `customColor: String?` (hex), `iconSymbol: String?` (nil -> `folder.fill`), `externalID: String?` (alias `idempotencyKey`), `anchorWorkspaceProvenance` (`generated` | `user` | `unknown`).
2. Membership is NOT on the group: each workspace has `groupId: UUID?`. The anchor workspace also carries `groupId`. Groups live in `workspaceGroups: [WorkspaceGroup]` per window (`Sources/TabManager.swift`).
3. The anchor has no row of its own: the header IS the anchor (`SidebarWorkspaceRenderItem.renderItems`, `Sources/SidebarWorkspaceRenderItem.swift`, skips the tab whose id == `liveAnchorWorkspaceId`).
4. Ordering invariants (`INV.normalizeWorkspaceGroupContiguity`, `normalizeWorkspaceGroupRunsPreservingOrder`): each group is one contiguous run in `tabs`, anchor first, other members in their existing relative order. Top-level rows (ungrouped workspace or whole group) are ordered pinned tier first, then unpinned tier; relative order inside each tier is preserved (stable partition). A group's tier follows `group.isPinned`, not the anchor's own pin (`WorkspacesModel+Ordering.isGlobalPinnedRow`).
5. Inside a group, per-workspace-pinned members sit before unpinned members, after the anchor (`clampedGroupedMemberReorderIndex`).
6. `workspaceGroups` order is kept in sync with anchor order in `tabs` (`INV.syncWorkspaceGroupsOrderToAnchorOrder`).
7. A `groupId` pointing at an unknown group is cleared to nil during normalize. A group whose anchor is not among its members gets its first member as anchor (provenance `user`).
8. Empty group (`anchor == .empty(groupId)`): only a pinned group survives with zero members; it renders as a header-only row and persists until Delete Group.
9. Persisted: id, name, isCollapsed, isPinned, anchor id + anchor member index, anchorIsEmpty, customColor, iconSymbol, externalID, provenance (`TabManager.swift` session snapshot, `SessionWorkspaceGroupSnapshot`).

## 2. Auto-naming
1. `WGC.nextAutoWorkspaceGroupName`: start at `n = workspaceGroups.count + 1`, format `"Group %lld"`, increment `n` until the name is not used by any existing group. It does not fill gaps below `count + 1`.
2. Used whenever the passed name is empty after trimming (all UI entry points pass `""`).

## 3. Create
1. Entry point for all paths: `WGC.createWorkspaceGroup(name:childWorkspaceIds:anchorWorkingDirectory:selectAnchor:collapseSidebarSelection:externalID:)`. New group is always `isCollapsed=false`, `isPinned=false`, no color, no icon.
2. Empty group (`AppDelegate.createEmptyWorkspaceGroup`): a new workspace is created via `TabManager.createGroupAnchorWorkspace` with `placementOverride: .top` = index `pinnedCount` (top of the unpinned tier), title = group name with `titleSource: .auto`, cwd inherited from the active workspace, and it IS selected. Provenance `generated`.
3. From selection (`AppDelegate.handleGroupSelectedWorkspacesShortcut`): selected ids sorted into `tabs` order; needs >= 2 eligible ids or the shortcut is not consumed. First id becomes the anchor (provenance `user`); no workspace is created.
4. Eligibility: a workspace that is the anchor of another group is silently dropped. A non-anchor member of another group IS accepted and moves to the new group.
5. After such a move, the old group is cleaned up only if what remains is an untouched generated anchor on an unpinned group (`WGC.removeGeneratedAnchorIfOrphaned`); otherwise the old group stays.
6. Placement (`WGC.placeNewWorkspaceGroupAtCreationPosition`): the group lands at the original index of the earliest child in `tabs`; order = anchor, then the other children in original `tabs` order; everything else keeps its order. Then tiers are renormalized (an unpinned group made from pinned workspaces drops to the unpinned tier).
7. The anchor is selected, and the sidebar multi-selection collapses to `[anchor]`.
8. "New Group from Workspace" (context menu, `SidebarWorkspaceRowCommands.addGroupSection`): same call with one id. The workspace becomes an anchor-only group, i.e. its row turns into a header named "Group N" in place.
9. Socket/CLI create passes `selectAnchor: false, collapseSidebarSelection: false`; cwd = `--cwd` or the first child's cwd or inherited.

## 4. Collapse / expand
1. Collapsed: all non-anchor member rows are omitted from the render list; the header stays (`SidebarWorkspaceRenderItem.renderItems`).
2. UI toggle (`WGC.toggleWorkspaceGroupCollapsed`): if the selected workspace is a non-anchor member of that group, selection moves to the anchor first; hidden members are also removed from the sidebar multi-selection. Otherwise selection is untouched.
3. Selecting a non-anchor member of a collapsed group auto-expands the group (`INV.expandWorkspaceGroupForSelectionIfNeeded`, run on every selection change and on `assignGroup`). Selecting the anchor does not expand.
4. Socket collapse (`WGC.setWorkspaceGroupCollapsed`) only flips the flag: a selected member can end up selected-but-hidden.
5. Next/previous workspace (`WorkspacesModel+Cycling.cycleDestination`, scope `.window`) walks all of `tabs`, including anchors and hidden members, wrapping. Landing on a hidden member expands its group (item 3).
6. Cmd+1..9 (`TabManager+NumberedWorkspaceSelection.selectWorkspaceByNumber`) counts only ordinary rendered rows: group headers/anchors are skipped and collapsed members are skipped.
7. After closing the selected workspace, if the slot's new occupant is a hidden member, the anchor is selected instead (`WorkspacesModel+CloseSelection.selectionTargetAfterClose`).
8. Unread badge (`VTS.sidebarWorkspaceGroupTableConfiguration`): collapsed = sum of unread over all members including the anchor; expanded = the anchor's unread only. Hidden when 0.
9. No member count is displayed in the header. `memberCount` in the row model only gates the Ungroup menu item.

## 5. Header row
1. Click on the header body (`VTS.focusWorkspaceGroupAnchor`, dispatched from `SidebarWorkspaceTableController.dispatchRowClick`): selects `TabManager.workspaceGroupHeaderTarget`.
2. Target rule: normally the anchor. If provenance is `generated` AND the anchor is untouched, the target is the first non-anchor member in `tabs` order. If additionally there is no other member, the click toggles collapse instead of selecting.
3. "Untouched" (`TabManager.workspaceGroupGeneratedAnchorIsUntouched`): title is unset or auto, no description, exactly one panel which is a terminal with no explicit input received and no initial command/input, and no dock panels.
4. Cmd- or Shift-click adds the anchor to the sidebar multi-selection and still selects the target.
5. Chevron click: `toggleWorkspaceGroupCollapsed` only (does not select). Double-click on a header: nothing (`didDoubleClickTableRow` only handles workspace rows).
6. `+` button: shown only while the pointer hovers the header, and not while a context menu or shortcut hint is showing (`HDR.updatePlusVisibility`). Click -> section 7. Right-click -> section 11c.
7. Left-to-right layout (`HDR.layout`): [pin glyph if pinned] chevron, icon, name, [unread badge], ... `+` at the trailing edge. Chevron is leading, before the icon.
8. Metrics at font scale 1.0 (`Sources/SidebarWorkspaceGroupHeaderMetrics.swift`, `Sources/WindowChromeMetrics.swift`):
   - Row height = max(chevron 14, icon 14, plus 16, name line height) + 10, about 26 (`HDR.preferredHeight`); 5 vertical padding each side.
   - Background inset 6 from each sidebar edge (`rowOuterHorizontalPadding`); content starts at the background's left edge with no extra leading pad; trailing content pad 10.
   - Pin: `pin.fill` 9pt semibold in a 14 frame, then 4 gap.
   - Chevron: `chevron.down` expanded / `chevron.right` collapsed, 9pt semibold, 14x14 frame, then 4 gap.
   - Icon: 11pt semibold, 14x14 frame, then 6 gap. Default symbol `folder.fill`.
   - Name: 11pt semibold, single line, tail truncation.
   - Unread badge: 6 after the name text, 10pt semibold white text, padding 5 horizontal / 1 vertical, fill = notification badge colour or accent.
   - `+`: `plus` 11pt medium, 16x16 frame, right edge at background right minus 10; name stops 4 before it.
9. Colours: chevron, pin, `+` = secondary label. Icon = group colour (custom or cwd config) else secondary label. Name = label at 0.9 opacity, 1.0 when the anchor is the selected workspace.
10. Background: clear; label colour at 0.08 opacity with corner radius 4 when the anchor is selected; multi-selected uses the row multi-select fill with radius 6. Row at 0.6 opacity while being dragged.
11. Member rows are indented 12 (`SidebarWorkspaceGroupingMetrics.memberIndent`), both content and row background.
12. All sizes multiply by the sidebar font scale; the `+` frame never goes below 16.

## 6. Closing, ungroup, delete
1. Anchor closed (`TabManager.closeWorkspace` -> `INV.promoteAnchorOrRemoveGroupsAnchoredBy`): the first remaining member in `tabs` order becomes anchor (provenance `user`). The header keeps showing the group name; the promoted workspace loses its own row and its own title is no longer shown.
2. Anchor closed with no other member: unpinned group is removed; pinned group stays as an empty header-only group.
3. Non-anchor member closed: group unchanged, except when that leaves only an untouched generated anchor on an unpinned group: then the anchor workspace is closed too and the group is removed (`WGC.removeGeneratedAnchorIfOrphaned`).
4. Batch closes close non-anchors before anchors (`TabManager.anchorLastCloseOrder`). Closing an anchor shows no special prompt.
5. Detach to another window (`INV.dissolveGroupsAnchoredBy`) differs: an unpinned group whose anchor leaves is dissolved; the moved workspace arrives ungrouped.
6. Ungroup (`WGC.ungroupWorkspaceGroup`): every member including the anchor gets `groupId = nil`, the group is removed, nothing is closed, and `tabs` order is NOT renormalized: the anchor becomes a normal row where the header was, members stay below it.
7. Ungroup on a pinned empty group is refused (`emptyPinnedCannotUngroup`); the menu item is hidden in that case.
8. Ungroup with `removeGeneratedAnchor` (socket/CLI only, `WorkspaceGroupCoordinator+GeneratedAnchorCleanup.swift`): requires provenance `generated` and zero non-anchor members; closes the anchor workspace and removes the group. A replacement workspace is created first if it was the window's last.
9. Delete Group (`VTS` `onDelete` -> `WorkspaceGroupCoordinator+DeletionConfirmation.deleteWorkspaceGroup(confirmed:)`): closes the anchor and all members (anchor last), removes the group, creates a fresh workspace if the window would be empty.
10. Prompt shown only when the group is pinned OR has >= 1 non-anchor member. `N` = number of non-anchor members (`containedWorkspaceCount`).
11. Dialog (`Sources/SidebarWorkspaceGroupDialogs.swift`), warning style, title `Delete this group?`, buttons `Delete` (default, Return) and `Cancel` (Escape). Body:
    - N = 0: no body text.
    - N = 1: `Delete the group “<name>” and close its workspace?`
    - N = 2: `Delete the group “<name>” and close its 2 workspaces?`
    - N > 2: `Delete the group “<name>” and close its <N> workspaces?`

## 7. New workspace inside a group
1. All paths end in `WGC.createWorkspaceInGroup(groupId:placement:referenceWorkspaceId:select:)`. cwd = the ANCHOR's current directory (not the selected member's); inherits the active cwd only if the anchor has none.
2. Cmd+N (`AppDelegate.workspaceGroupNewWorkspaceTarget`): applies when the selected workspace has a `groupId` (anchor or member). Reference = the selected workspace. New workspace is selected.
3. Header `+` and "New Workspace in Group": no reference workspace. New workspace is selected. Socket/CLI: no reference, `select: false`.
4. Placement resolution: explicit param, else per-cwd config for the anchor's cwd, else global setting `workspaceGroups.newWorkspacePlacement`, default `afterCurrent`.
5. Placement semantics (`WGC.placeWithinGroup`):
   - `afterCurrent`: right after the reference if it is in this group; otherwise right after the anchor. With the anchor as reference this equals `top`.
   - `top`: right after the anchor (first member row).
   - `end`: after the last existing member; if there are none, right after the anchor.
6. Accepted spellings: `afterCurrent`, `after-current`, `after_current`, `top`, `end`, case-insensitive (`WorkspaceGroupNewPlacement.init(rawString:)`).
7. Creating into an empty pinned group makes the new workspace the anchor and keeps the header's slot.

## 8. Moving / reordering
1. Keyboard (`TabManager+AdjacentWorkspaceReordering.moveSelectedWorkspace(by:)` -> `WRC.reorderWorkspace(tabId:toIndex:)`), selection is kept.
2. Non-anchor member: clamped to its own group's member range; it cannot leave the group by keyboard, and cannot cross the pinned/unpinned member split (`clampedGroupedMemberReorderIndex`).
3. Ungrouped workspace or anchor: moved one raw `tabs` index within its pin tier, then `normalizeWorkspaceGroupContiguity` runs. (derived) Consequences: an ungrouped row moving DOWN hops over a whole group; a group (anchor selected) moving UP hops over one ungrouped row; but an ungrouped row moving UP into a multi-row group, and an anchor moving DOWN in a group with members, normalize back to the old order (no visible move). Upstream has no test for these; a port should swap top-level rows instead.
4. Drag, plan from `SidebarWorkspaceReorderDropResolver.plan` (`Packages/macOS/CmuxFoundation/Sources/CmuxFoundation/SidebarDrop/`), commit via `WRC.reorderSidebarWorkspace`.
5. Drop into a group (`groupScopeCandidate`): pointer on a member row (top half always; bottom half when the next row is in the same group) or on the bottom half of a group header (works for collapsed and anchor-only groups) -> `.reorder(..., explicitGroupId)`. The row is inserted at that index and assigned to the group.
6. Ambiguous boundary (bottom half of a group's last member, top half of the first root row after a group, top half of an empty header): pointer in the right half of the sidebar = into the group, left half = root level.
7. Drop out of a group: dropping a member on a root-level slot uses top-level rows with promotion (`reorderTopLevelWorkspaceItem(promotesGroupedWorkspace: true)`): `groupId` cleared, placed among top-level rows.
8. Dragging a header: always a top-level move of the whole group run (anchor + members), clamped to its pin tier; an anchor can never be dropped into another group.
9. Without an explicit group (`WRC.applyDragInferredGroupMembership`): both new neighbours in the same group -> join it; both ungrouped -> leave group; neighbours differ -> keep current membership.
10. Multi-row drag mixing anchors and plain rows moves only the kind matching the dragged row (`WRC.reorderSidebarWorkspaces`).
11. Context menu "Move to Group" -> `WGC.addWorkspaceToGroup` without placement: the workspace joins and normalize pulls it to the END of the group's run (derived). "Remove from Group" -> `WGC.removeWorkspaceFromGroup`: membership cleared then normalized, so it lands at the group's edge (derived); on an anchor it ungroups the whole group.

## 9. Rename, pin
1. Rename is NOT inline and not an alert: "Rename Group..." opens the command palette in rename mode prefilled with the current name (`VTS` `onRename` -> `AppDelegate.requestCommandPaletteRename`, target kind `.workspaceGroup`).
2. Hint texts (`Sources/ContentView.swift`): `Enter a group name. Press Enter to rename, Escape to cancel.` and `Press Enter to apply this group name, or Escape to cancel.`
3. Empty input is rejected with a beep and the palette stays open. `WGC.renameWorkspaceGroup` trims; whitespace-only or unchanged is a no-op. If the anchor is `generated`, the anchor workspace's title is renamed too.
4. Double-clicking a workspace row starts inline rename of the workspace; there is no equivalent for headers.
5. Pin (`WGC.setWorkspaceGroupPinned`): flips `isPinned`, renormalizes. A newly pinned group becomes the LAST row of the pinned tier; a newly unpinned group becomes the FIRST row of the unpinned tier (stable partition, derived). Independent of member workspaces' own pin flags. Pinned header shows the `pin.fill` glyph, tooltip `Pinned group`.

## 10. Keyboard shortcuts (`Sources/KeyboardShortcutSettings.swift`, `Packages/macOS/CmuxSettings/.../ShortcutAction+Defaults.swift`)
| Identifier | Label | Default |
|---|---|---|
| `newWorkspaceGroup` | New Workspace Group | Ctrl+Cmd+G |
| `groupSelectedWorkspaces` | Group Selected Workspaces | Cmd+Shift+G |
| `toggleFocusedWorkspaceGroupCollapsed` | Toggle Focused Workspace's Group Collapse | Ctrl+Cmd+. |
| `nextSidebarTabInGroup` | Next Workspace in Group | unbound |
| `prevSidebarTabInGroup` | Previous Workspace in Group | unbound |
| `moveWorkspaceUp` / `moveWorkspaceDown` | Move Workspace Up / Down | Ctrl+Opt+Cmd+[ / ] |
| `nextSidebarTab` / `prevSidebarTab` | Next / Previous Workspace | Ctrl+Cmd+] / [ |
1. `groupSelectedWorkspaces` and `toggleFocusedWorkspaceGroupCollapsed` are only consumed when they act (>= 2 eligible selected rows; focused workspace is in a group), otherwise the key falls through (`Sources/AppDelegate.swift`).
2. In-group cycling (`.focusedGroupMembers`) cycles non-anchor members only, wrapping; from the anchor, next = first member, previous = last member; falls back to window cycling when the selected workspace is ungrouped.
3. There are no separate expand-all / collapse-all shortcuts.

## 11. Context menus (exact titles, in order)
a. Workspace row, group section (`SidebarWorkspaceRowCommands.addGroupSection`):
   1. `New Empty Workspace Group` (shows the `newWorkspaceGroup` chord)
   2. `New Group from Workspace` (single target) / `New Group from Selection` (multi) (shows the `groupSelectedWorkspaces` chord)
   3. `Move to Group` > submenu of all group names in group order; the group all targets already share is disabled; the item itself is disabled when no groups exist
   4. `Remove from Group` (only when a target is grouped)
   Items 2-4 are omitted when every target is a group anchor. Blank area below the list: `New Empty Workspace Group` only.
b. Group header (`HDR.makeHeaderMenu`):
   1. `New Workspace in Group` / separator
   2. `Rename Group...`
   3. `Pin Group` or `Unpin Group` / separator
   4. `Mark Group as Read`, `Mark Group as Unread`, `Clear Latest Notifications` (act on the anchor) / separator
   5. `Mark All Workspaces in Group as Read`, `Mark All Workspaces in Group as Unread` (non-anchor members) / separator
   6. `Edit Group Config...`, `Open Workspace Groups Docs` / separator
   7. `Ungroup Workspaces` (hidden when pinned with 0 members)
   8. `Delete Group`
c. `+` button (`HDR.makePlusMenu`): `New Workspace in Group`, [separator + per-cwd configured actions], separator, `Edit Group Config...`, `Open Workspace Groups Docs`.

## 12. Socket / CLI
1. Handle: `workspace_group:N` (`ControlHandleKind.workspaceGroup`), N = per-kind ordinal handed out by the handle registry. `group_id` params accept a UUID or that ref.
2. Group payload (`SOCK.workspaceGroupPayload`): `id`, `ref`, `name`, `is_collapsed`, `is_pinned`, `anchor_workspace_id` (group id when empty), `anchor_workspace_ref`, `is_empty`, `custom_color`, `icon_symbol`, `member_workspace_ids`, `member_workspace_refs`, `member_count` (includes the anchor), `anchor_workspace_provenance`, `anchor_workspace_is_generated`, plus `external_id` + `idempotency_key` when set.
3. Methods, params -> result:
   - `workspace.group.list` -> `{window_id, window_ref, groups: [payload]}`
   - `workspace.group.create` `{name, cwd?, child_workspace_ids: [handle], external_id? | idempotency_key?}` -> `{group: payload, created: bool}`
   - `workspace.group.ungroup` `{group_id, remove_generated_anchor?: bool}` -> `{group_id, operation: "dissolved", kept_workspace_count, removed_anchor_workspace_id?}`
   - `workspace.group.delete` `{group_id, close_workspaces?: bool}` -> without `close_workspaces` identical to ungroup; with it `{group_id, operation: "closed_workspaces", closed_workspace_count}`
   - `workspace.group.rename` `{group_id, name}` -> `{group_id, name}`
   - `workspace.group.collapse` / `.expand` `{group_id}` -> `{group_id, is_collapsed}`
   - `workspace.group.pin` / `.unpin` `{group_id}` -> `{group_id, is_pinned}`
   - `workspace.group.add` `{group_id, workspace_id, placement?, reference_workspace_id?}` -> `{group_id, workspace_id}`
   - `workspace.group.remove` `{workspace_id}` -> `{workspace_id}`
   - `workspace.group.set_anchor` `{group_id, workspace_id}` -> `{group_id, anchor_workspace_id}`
   - `workspace.group.new_workspace` `{group_id, placement?}` -> `{group_id, workspace_id, workspace_ref}`
   - `workspace.group.set_color` `{group_id, hex | color}` -> `{group_id, custom_color}` (empty clears; 6-digit hex)
   - `workspace.group.set_icon` `{group_id, symbol | icon}` -> `{group_id, icon_symbol}`
   - `workspace.group.move` `{group_id, to_index | before_group_id | after_group_id}` -> `{group_id}` (index into the group list, clamped to the pin tier)
   - `workspace.group.focus` `{group_id}` -> `{group_id, anchor_workspace_id, anchor_workspace_ref}` (selects the anchor, focuses the window)
   All accept window routing (`window_id`). Only `focus` changes selection.
4. Errors (code: message):
   - `invalid_params`: `Missing or invalid group_id` (also for an unresolvable ref) / `Missing group_id or name` / `Missing group_id or workspace_id` / `Missing or invalid workspace_id` / `Invalid placement` (add) / `placement must be one of: afterCurrent, top, end` (new_workspace) / `Unresolved child workspace handles: a, b` / `child_workspace_ids must be an array of workspace handles` / `Missing or unresolvable target position` (move)
   - `not_found`: `Group not found` (data `{group_id}`) / `Group or workspace not found` / `Workspace not in a group` / `Group not found or workspace not a member` / `Group or anchor not found` / `Child workspace not found in target window: ...`
   - `invalid_state`: `All requested children are ineligible because they are already group anchors; ungroup them first` / `Workspace is the anchor of another group; ungroup it first` / `Generated-anchor cleanup requires an anchor-only group` / `The group anchor was not generated by cmux`
   - `invalid_request`: `A pinned empty group can only be removed with Delete Group`
   - `unavailable`: `TabManager not available`; `not_created`: `Group was not created`
5. CLI (`CLI/cmux.swift` `runWorkspaceGroup`), text output:
   - `list`: one line per group `<handle>  <name>  (<member_count> members)` + ` [pinned]` + ` [collapsed]` (two spaces between fields); `No groups` when empty.
   - `create`: `OK <group handle>`. `new-workspace`: `OK <workspace_ref>`.
   - `ungroup` and `delete` without `--close-workspaces`: `OK group dissolved (kept N workspace[s])`.
   - `delete --close-workspaces`: `OK group deleted (closed N workspace[s])`.
   - Everything else: `OK`. `--json` prints the raw result.
   - Subcommands: list, create, ungroup, delete, rename, collapse, expand, pin, unpin, add, remove, set-anchor, new-workspace, set-color, set-icon, move, focus.
   - CLI-side errors: `workspace-group <sub> requires a group id or --group <id>`, `rename requires --name <name>`, `add requires --group <id> --workspace <id>`, `Unknown workspace-group subcommand: <sub>`.

## Unknown / not found
1. Tests were not read (`WorkspaceCoordinatorTests.swift`, `cmuxTests/WorkspaceGroupTests.swift`), apart from the numbered-selection and reorder-shortcut tests; items marked (derived) are unconfirmed.
2. How the CLI prints a socket error (prefix/format of the stderr line) for e.g. an unknown group.
3. Exact command-palette rename chrome (title/placeholder) beyond the two hint strings.
4. Order of target ids passed by the context menu for "New Group from Selection" (the shortcut path sorts by `tabs` order; the menu path was not traced), so which row becomes anchor there.
5. `SidebarCompactStatusGlyph.groupHeader` roll-up (compact agent-status mode replaces the numeric badge) was not read.
6. Default chord of `selectWorkspaceByNumber` and the rest of `rootScopedPlan` / cross-window drop details.
7. Per-cwd config (`workspaceGroups.byCwd`) resolution code; the doc says the surface is partly reserved.
