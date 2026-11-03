# M9 agent-coordination — Spec

**Goal:** agents in different workspaces can work together.

## Scope
- `cmux agent message <target> "..."`: delivered through the target agent's hooks, never typed into its terminal (cmux `docs/agent-messages.md`).
- Claude Teams: `cmux claude-teams` runs teammate mode with teammates as native splits, sidebar metadata and notifications.
- Workspace auto-naming, opt-in: summarise the agent conversation into a 2–5 word name. **Local only:** runs a configurable command (default `claude -p`), no Manaflow service.
- Automations: event bus + ordered actions from a versioned JSON file (cmux `docs/automations.md`).

## Exit criteria
1. A message sent to a busy agent arrives at its next turn, not mid-prompt.
2. Claude Teams spawns teammates as splits with notifications.
3. Auto-naming renames a workspace after an agent turn when enabled; off by default.
4. A sample automation runs on its event.
