# m8-groups-dock — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-11 — M8 built (plan Tasks 1–8)

Built in one agent session on the CachyOS desktop. Research: `research-groups.md`, `research-dock.md` (cmux @ `88b87c05`). Decisions D1–D15 are in `plan.md`; this entry lists what happened and every deviation.

**What exists**
- `cmux-core::groups`: groups as runs of the workspace list, `normalize`, and every rule in plan D1–D7 with tests. `Workspaces::rows()` is what the sidebar draws.
- Sidebar: header rows (chevron, folder, name, unread badge, hover `+`, pin glyph), members indented 12, a context menu (`window_view/menu.rs`) on rows, headers and the blank area, inline group rename, Delete Group prompt with cmux's texts.
- Socket and CLI: `workspace.group.*` (14 methods) and `cmux workspace-group`, handle `workspace_group:N`.
- Right sidebar: mode bar (Feed, Dock), draggable width. Dock = one `WorkspaceView` (`new_dock`) owned by the window; `~/.config/cmux-linux/dock.json` seeds it.

**Found while testing on screen**
- M4's reorder-on-notification called `move_to(ws, 0)`, which with groups pulled a notified member out of its group. Now `Workspaces::bump_to_top` (cmux `moveTabToTopForNotification`): the member leads its group and the group goes to the top of the unpinned rows; pinned groups don't move.

**Deviations from cmux**
- No empty pinned groups (D2): a group dies with its last workspace.
- Collapsing over the socket also moves a hidden selection to the anchor (D4).
- "Untouched anchor" means: nothing typed into it and a single surface (D5). Orphaned generated anchors are not auto-closed.
- Keyboard move swaps whole top-level rows (D6). Drag joins a group only above a visible member (D7); no drop onto a header, no left/right-half rule.
- Group rename is inline, not a command palette (D9). A rename of the selected anchor (Super+Shift+R) renames the group, since the anchor has no row.
- Menus: no notification, config or docs entries; "Move to Group" is flat. No `groupSelectedWorkspaces` (no multi-selection).
- `workspace.group.list` rows report `is_empty: false` always; no colour, icon or external id fields.
- Mode bar: both tabs always show their label (cmux collapses unselected tabs to icons); no "open as pane" or close buttons; Ctrl+4/Ctrl+5 fixed, not positional, and only inside the Feed's key context. So from a terminal the keyboard way to the Dock is Super+Shift+E, then Ctrl+5.
- `focusRightSidebar` still never hides the sidebar (M7 behaviour; cmux toggles).
- Dock: main-area minimum pane size (not 48), so a side-by-side split needs a wider sidebar. Config controls are typed into an interactive login shell, not run through cmux's start script; `height`, `env`, `title` ignored; a bad `dock.json` is one stderr line, not an error view. Dock shells carry the window id as `CMUX_WORKSPACE_ID` as in cmux, but our socket doesn't resolve it: `cmux notify` and the Feed hook fail inside the Dock, and OSC notifications from Dock terminals are dropped.

**Checked on KDE (socket and `debug.shortcut.simulate`, screenshots read by the agent)**
- Exit 1: Ctrl+Super+G makes "Group 1" with a selected anchor; Super+N adds members inside; Ctrl+Super+. collapses and the selection goes to the header; `workspace-group` create/add/pin/rename/collapse/new-workspace/list and the error cases behave as the tests say; a pinned group draws its pin and sits on top; the header menu draws with all seven entries.
- Exit 2: with a scratch `dock.json` the Dock opened with two stacked panes running their commands; Super+T added a Dock tab; switching workspace left the Dock as it was. The empty Dock and the Feed tab draw.
- Not checked (needs a mouse or typing): menu clicks, header click and `+`, chevron click, drags, the rename field, the Delete prompt, dragging the sidebar edge, clicking "New Terminal".
