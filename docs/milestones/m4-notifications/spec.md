# M4 notifications — Spec

**Goal:** an agent waiting for input is impossible to miss, exactly as in cmux.

## Scope
- OSC 9 / 99 / 777 parsed in `cmux-term` → notification store in `cmux-core`.
- Blue pane ring, tab highlight, sidebar latest-notification text and unread badge.
- Notification panel; Super+Shift+U jumps to most recent unread; clearing on focus as cmux does.
- KDE desktop notifications via freedesktop D-Bus; clicking one focuses the pane.
- Reference: cmux `docs/notifications.md` and the Swift notification code.

## Out of scope
`cmux notify` CLI (M5).

## Exit criteria
1. `printf '\e]9;hello\a'` in a background pane rings it, badges its workspace, and raises a KDE notification.
2. Super+Shift+U lands on the right pane.
3. Read/unread rules match cmux.
