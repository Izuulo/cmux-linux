# M10 persistence — Spec

**Goal:** nothing is lost when the app or machine restarts.

## Scope
- Layout restore: windows, workspaces, splits, tabs, cwd.
- Vault: detect running agents, record their session ids, offer resume (cmux `docs/vault.md`, `docs/agent-recovery.md`), incl. custom agent registrations.
- Opt-in `local-tmux` profile: terminals backed by a user-owned tmux server so processes survive app restarts (cmux `docs/local-tmux.md`).

## Exit criteria
1. Quit and relaunch restores the exact layout.
2. An interrupted Claude Code session is listed and resumes in place.
3. With `local-tmux` on, a running process survives an app restart.
