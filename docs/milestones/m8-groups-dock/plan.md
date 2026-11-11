# M8 groups-dock — Plan

**Goal:** workspace groups in the sidebar and a Dock of terminals in the right sidebar, as the spec says.

**Research:** `research-groups.md` (rg) and `research-dock.md` (rd), both from cmux @ `88b87c05`.

## Global Constraints

- Branch `m8-groups-dock`, cut from `m6-desktop-checks`. One commit per step that changes files.
- Model rules live in `cmux-core` with tests; the app only draws and forwards.
- No new dependency. New icons are Lucide (already in `NOTICE`).
- UI is checked on KDE with `debug.shortcut.simulate` and socket calls plus screenshots; clicks, drags and typing stay with the owner.

## Decisions made in this plan

| | Decision | Why |
|---|---|---|
| D1 | Groups are part of `Workspaces`: `Workspace.group`, a `Vec<Group>` (name, anchor, collapsed, pinned, generated anchor), and one `normalize` after every change (rg §1.4): each group one run, anchor first, pinned groups above the rest. | One place keeps the invariants; the list stays the order the sidebar draws. |
| D2 | No empty groups. A group whose last workspace closes is removed, pinned or not. | rg §1.8 keeps a pinned empty header; that needs a second anchor kind for a corner case. Deviation. |
| D3 | The sidebar draws `Workspaces::rows()`: a header per group (it stands for the anchor), then its other members indented, hidden while collapsed. Drop gaps and scrolling count these rows. | rg §1.3, §4.1. |
| D4 | Selecting a hidden member expands its group; collapsing a group moves a hidden selection to the anchor, on the socket too. Closing never leaves a hidden selection. Number keys count plain visible rows only. Cycling walks everything. | rg §4. Upstream's socket collapse can leave a hidden selection; ours doesn't. |
| D5 | Header click: the anchor, or the first member when the anchor is generated and untouched (no key typed into it, one surface); with no member it toggles collapse. | rg §5.2. "Untouched" is narrower upstream (titles, descriptions); typed-into is what we can see. |
| D6 | Keyboard move: a member stays inside its group; a plain row or a header swaps with the next top-level row. | rg §8.3 recommends the swap; upstream's own moves sometimes do nothing. |
| D7 | Drag: a header moves its whole group; another row joins a group only when dropped above one of its visible members, otherwise it lands at root level. No left/right-half rule, no drop on a header. | rg §8.5–8.9 simplified; "Move to Group" in the menu covers the rest. |
| D8 | A small context menu (new `window_view/menu.rs`): workspace row, group header, blank area. Items per rg §11 minus notifications, config and docs entries; "Move to Group" is one flat item per group. | The app had no menus; groups need rename, pin, ungroup, delete. |
| D9 | Group rename is inline in the header (the workspace rename input). | Upstream uses the command palette (M11). Deviation. |
| D10 | Keys: `newWorkspaceGroup` ctrl+super+g, `toggleFocusedWorkspaceGroupCollapsed` ctrl+super+. . Not ported: `groupSelectedWorkspaces` (no sidebar multi-selection here), the unbound in-group cycling. | rg §10. |
| D11 | Socket: `workspace.group.` list, create, ungroup, delete, rename, collapse, expand, pin, unpin, add, remove, set_anchor, new_workspace, focus; handle `workspace_group:N`; CLI `cmux workspace-group <sub>` with cmux's output lines. Not ported: colour, icon, move, idempotency keys, generated-anchor cleanup. | rg §12. |
| D12 | Right sidebar gets cmux's mode bar with two tabs, Feed and Dock, and a draggable width (min 295, main area keeps 360). | rd §1.5, §2. |
| D13 | The Dock is one `WorkspaceView` owned by the window, drawn in the right sidebar. So it has the main area's tabs, splits and shortcuts with no new code, and it outlives workspace switches. When its last terminal closes the Dock shows "Empty Dock Pane" with "New Terminal". | rd §3.2, §4. |
| D14 | Dock shells get the window's id as `CMUX_WORKSPACE_ID`. | rd §6.1. |
| D15 | `~/.config/cmux-linux/dock.json` seeds the Dock the first time it is shown: each control is a pane below the previous one, its `command` typed into a login shell started in `cwd`. | rd §7. Typing reuses `workspace.create --command`; no temp script. |

## Deferred (with reason)

- Group colour, icon, per-cwd config, placement setting: need `cmux.json` (M11).
- Orphaned generated anchors are not closed automatically (rg §6.3): surprising without persistence of provenance; revisit with M10.
- Dock on the socket (`--placement dock`, `dock_scope` rows, sending keys to Dock surfaces) and Dock notifications (rd §8, §9): the main-area model is keyed by workspace; a Dock owner needs its own pass.
- Dock: 48 px minimum pane (ours stays the main area's), `height` ratios, `env`, `title`, tab drag between Dock and main area, project `.cmux/dock.json` with its trust gate (upstream doesn't render that Dock either, rd §3.3).
- Mode shortcuts by tab position (rd §1.6): Feed keeps ctrl+4, Dock gets ctrl+5, both only inside the Feed's key context.

## Tasks

1. **Model** (`cmux-core::workspaces`): groups, `normalize`, create (from workspaces, empty), auto-name, ungroup, rename, collapse, pin, add, remove, set anchor, new workspace in group, close with promotion, selection rules, `rows()`, moves, header target. Tests for each rule above.
2. **Keys and actions**: two bindings, two actions.
3. **Sidebar**: rows from `rows()`, header row (rg §5.8 metrics), member indent, unread sums, drag with the new gaps, new workspace inside a group (anchor's cwd), delete prompt (rg §6.11).
4. **Menu**: the context menu and its three uses; inline group rename.
5. **Socket and CLI**: handle kind, snapshot, `workspace.group.*`, `cmux workspace-group`, help, tests against the real server.
6. **Right sidebar**: mode bar, resize.
7. **Dock**: `WorkspaceView` frame and dock flag, the window's Dock, empty pane, focus, `dock.json`.
8. **Desktop check and docs**: screenshots on KDE, README, journal, STATUS.
