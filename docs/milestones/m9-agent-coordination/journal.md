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

## 2026-11-11 — Part 2 built: workspace auto-naming (plan N1–N6)

Source: cmux `docs/workspace-auto-naming.md` and `CLI/CMUXCLI+AutoNaming.swift` @ `88b87c05` (GPL part of the tree). Throttle numbers, the excerpt rule, the prompt and the title clean-up are cmux's.

**What exists**
- `cmux-core`: `Workspace.auto_title`, `Workspaces::set_auto_title` and `accepts_auto_title`; `rename` marks the title as the user's.
- `cmux-config::auto_naming`: `[auto_naming] enabled, command` from `config.toml`.
- `cmux-meta::autoname`: `throttle`, the Claude transcript excerpt, `prompt`, `title`, and `pass`, which runs the command through the PR poller's deadline runner (it can now feed stdin).
- App: `workspace.auto_name {surface_id, transcript_path}` (`window_view/autoname_ctl.rs`) answers `{queued}` at once and runs the pass on the background pool. The `Stop` message hook sends it before claiming mail.

**Deviations from cmux**
- The app runs the summarizer; cmux's hook does and applies with `workspace.set_auto_title` (plan N2). No new hook entry: people who ran `cmux hooks setup` for Part 1 need nothing.
- Throttle state is per workspace and in memory; cmux keeps it per agent session on disk.
- A run that returns the current title does not move the line baseline (the pass can't tell it from a failure), so an active, stable session asks again every 3 minutes at most.
- Only Claude Code transcripts; the agent's tab is not named; no settings UI.
- On the 60 s deadline only `sh` is killed; a summarizer it started runs on until it ends by itself.

**Checked (exit 3)**
- Against the running app on KDE with a fake command and a 14-line transcript, the hook run by hand: with `enabled = false` nothing runs and the title stays; after flipping to `true` (no restart) the next `Stop` renames the workspace to "Fix Login Bug", and the command got cmux's prompt with the 2 first user messages and the last 4; a second `Stop` right away does not run the command (cooldown); a workspace created with `--name` is not renamed and the command is not run.
- Not checked: the default `claude -p` line under a real Claude Code session (spends usage; goes with the owner's real-Claude run).

## 2026-11-11 — Part 3 built: automations (plan A1–A7)

Source: cmux `docs/automations.md`, `docs/events.md`; from the Swift only the `where` alias keys (`AutomationRule.swift`) and the `run` variable names (`AutomationEngine.swift`) were read.

**What exists**
- `cmux-ipc::automation`: `parse` (bad rules left out with a warning), `Rules::fire` (pattern, `where`, sliding-window rate limit, templates), `event`.
- `cmux-meta::sh`: the shell runner auto-naming already had, now shared.
- App `window_view/automation_ctl.rs`: `emit`, the `notify` and `run` actions, `automation.list` and `automation.reload`. Emit points: `make_view` (`workspace.created`), `close_now` (`workspace.closed`), `on_notify` (`notification.created`), `feed_push` (`feed.item.received`), `message_send` (`agent.message.queued`), `auto_name_from_socket` (`agent.hook.Stop`).
- CLI `cmux automation list|reload`.

**Deviations from cmux**
- No `rpc` or `webhook` action, no rule chain: a rule's own notification emits nothing (A6); loops through `run` are only rate-limited.
- Six events, no stream (`cmux events`), no `events.jsonl`, no redaction: notification text is in the payload, which only rules see.
- No concurrency cap or firing log; a firing's actions run one after another, firings side by side.
- `workspace.created` carries the title at creation ("Terminal"), before `new-workspace --name` sets it.
- `agent.hook.Stop` exists only because the message hook reports the turn's end; other hook events are not events.

**Checked (exit 4)**
- Against the running app on KDE: a `workspace.created` rule fired at start and on `cmux new-workspace`; its `notify` put an unread badge on the new workspace (screenshot) and its `run` then wrote the rule id, event name and event JSON from the `CMUX_AUTOMATION_*` variables. An `agent.hook.*` rule with `where workspace.title prefix "build"` fired for the `Stop` hook run in "Build API" and not for the one in the other workspace. A `category: notification` rule did not fire on the first rule's notification. A rule with an `rpc` action was left out, with the warning on stderr and in `cmux automation list`; `reload` picked up an edit.
