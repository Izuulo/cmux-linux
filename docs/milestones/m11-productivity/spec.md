# M11 productivity — Spec

**Goal:** faster everyday use.

## Scope
- Command palette over all app actions and custom commands.
- `cmux.json` project custom commands (cmux docs: custom-commands).
- SSH workspaces: `cmux ssh user@host [--command ...]` opens a workspace whose terminals run on that host; image drag-drop uploads via scp. No cloud, no relays.

## Exit criteria
1. Every shortcut action is reachable from the palette.
2. A project's `cmux.json` command appears and runs.
3. `cmux ssh` opens a working remote workspace with sidebar metadata where available.
