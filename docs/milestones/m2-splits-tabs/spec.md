# M2 splits-tabs — Spec

**Goal:** split panes and per-pane tabs that behave like cmux.

## Scope
- Split tree in `cmux-core` (H/V, ratios, close/collapse), unit-tested.
- Pane tab strip; new/close/reorder tabs; focus follows click and shortcuts.
- Draggable dividers; focus movement between panes.
- Keybinding system in `cmux-config`: cmux's macOS defaults with ⌘→Super, remappable in `~/.config/cmux-linux/config.toml`.
- Reference: cmux Swift split/tab views and its keyboard shortcut list.

## Out of scope
Workspaces, sidebar, notifications.

## Exit criteria
1. Split, tab, focus and resize shortcuts match cmux's table (Super for ⌘).
2. A remapped binding in config takes effect without code changes.
3. Spacing and divider look match the Mac app (screenshot pair in journal).
