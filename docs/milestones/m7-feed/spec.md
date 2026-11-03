# M7 feed — Spec

**Goal:** answer agent permission and plan prompts from the sidebar without switching panes.

## Scope
- Right sidebar Feed panel (cmux opens it with ⌃4; keep that binding, remappable).
- Item types as cmux: permission requests (Once / Always / All tools / Bypass / Deny), ExitPlanMode choices, questions.
- Delivered through Claude Code hooks over the M5 socket; `cmux hooks setup` equivalent that installs them.
- Reference: cmux `docs/feed.md`, `docs/agent-hooks.md`, Swift Feed code.

## Out of scope
OpenTUI `cmux feed tui` (can run later inside the Dock, M8).

## Exit criteria
1. A Claude Code permission request in a background workspace appears in Feed; answering it unblocks the agent.
2. Plan approval works the same way.
3. Hook install is idempotent and reversible.
