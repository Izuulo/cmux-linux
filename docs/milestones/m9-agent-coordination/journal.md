# m9-agent-coordination — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-11 — Part 1 built: agent messages (plan Tasks 1–5)

Source: cmux `docs/agent-messages.md` @ `88b87c05`. The Swift implementation was not read; payload field names beyond the doc's table (`created_at_ms`, `sender_surface_ref`, …) and the CLI's text lines are ours.

**What exists**
- `cmux-core::messages`: `MessageStore` (queued → delivered → read, cmux's limits, newest 2,000 read kept), `render_messages` (cmux's block word for word), the last claiming surface per workspace as its agent.
- Socket: `agent.message.send`, `.list`, `.claim`, `.mark_read` (`window_view/message_ctl.rs`). `Snapshot::message_target` resolves handles, then titles.
- CLI: `cmux agent message`, `cmux agent inbox`.
- Hook: `cmux hooks message --source claude`, installed by `cmux hooks setup` on `UserPromptSubmit`, `PostToolUse` (both: `additionalContext`) and `Stop` (`decision: block` with the message as reason; with no mail it marks the delivered ones read).

**Deviations from cmux**
- No idle wake (plan D6): no `asyncRewake` poller, so no `.poll`, `.ack`, leases or `defer_delivery`. An idle Claude gets its mail with the next prompt.
- No off switch (`agent.message.settings`, `failed` state), no `cmux events`, no chat view, no Codex/Kiro paths, not persisted.
- A workspace target falls back to the focused surface when no hook has claimed there yet; cmux resolves "the surface running its agent".
- `claim` with `mark_delivered_read` marks the earlier delivered messages read before claiming; cmux's exact meaning of that flag was not checked.
- A closed terminal's queued and delivered messages are dropped (cmux keeps messages per install).
- `cmux hooks uninstall` removes the message hooks too; the setup/uninstall output still says "Feed hooks".

**Checked (exit 1, as far as it goes without Claude)**
- Against the running app with two workspaces: a message sent by title and by prefix is `queued`; the recipient's `PostToolUse` hook run prints both waiting messages as additional context and they become `delivered`; a reply by `--reply-to` goes back to the sender's surface; the recipient's `Stop` hook prints `{}` and its mail becomes `read`; the sender's `Stop` hook gets the reply as `decision: block`. An unknown target and a body with an escape character are refused with the store unchanged.
- Not checked: a real Claude Code session with the hooks installed (it edits `~/.claude/settings.json` and spends usage; same owner step as M7 Task 9 Step 2). The three hook output shapes were checked against Claude Code's hooks reference (code.claude.com/docs/en/hooks) the same day: `additionalContext` on `UserPromptSubmit` and `PostToolUse`, `decision: block` with `reason` on `Stop`, `timeout` in seconds, a matcher ignored on events without one. The reference also documents `asyncRewake` (a background hook that wakes an idle session on exit code 2), which is what an idle-wake follow-up would use, and a cap of 8 consecutive Stop continuations; our hook can't loop, since a claim empties the queue.

Parts 2–4 (auto-naming, automations, Claude Teams) are not started.
