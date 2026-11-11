# M9 agent-coordination — Plan

**Goal:** the four features of the spec, one part at a time. Each part is usable alone and ends with its exit criterion checked.

Branch `m9-agent-coordination`, cut from `m8-groups-dock`. Reference: cmux @ `88b87c05`, `docs/agent-messages.md` (am), `docs/automations.md`, `docs/workspace-auto-naming.md`.

## Part 1: agent messages (exit 1)

| | Decision | Why |
|---|---|---|
| D1 | A pure store in `cmux-core::messages`: states `queued → delivered → read`, cmux's limits (32 KiB text body without control characters except newline and tab, one-line sender of at most 64 characters), all queued and delivered messages plus the newest 2,000 read ones kept. Not persisted. | am "Delivery", "Limits". Persistence is M10. |
| D2 | Socket `agent.message.send`, `.list`, `.claim`, `.mark_read` with cmux's param and result names. No `.poll`, `.ack`, `.settings`. | am "Socket API". The three left out serve the idle-wake poller and the off switch (below). |
| D3 | CLI `cmux agent message <target> [--from <name>] [--reply-to <id>] <text|->` and `cmux agent inbox [--surface <target>] [--state <s>] [--limit <n>] [--mark-read]`. | am examples. |
| D4 | Target: a surface or workspace id or ref, else a workspace title (exact, then a unique prefix). A workspace means the surface that last claimed messages there (its agent), else its focused surface. | am §1. We don't detect agents; a surface whose hook has asked for messages is one. |
| D5 | Delivery through Claude Code hooks that `cmux hooks setup` installs next to the Feed hook, all running `cmux hooks message --source claude`: `UserPromptSubmit` and `PostToolUse` attach queued messages as additional context; `Stop` continues the turn with them (`decision: block`) and, when there are none, marks what was delivered as read. | am table: a busy agent "sees it at its next step", a prompt carries it "as context", and the Codex row's "continues the turn". Nothing is ever typed into the terminal. |
| D6 | No idle wake: an idle Claude gets the message with its next prompt. | cmux wakes an idle session with a background `asyncRewake` poller plus leases and acks; that is a second protocol for one case. Deviation. |
| D7 | The message text the agent sees is cmux's block, word for word. | am "The recipient sees". |
| D8 | The hook acts only with `CMUX_LINUX=1` and a surface id, like the Feed hook; any error prints `{}`. | M7 D3: safe to leave installed. |

Deferred: turning messages off (per agent, per workspace, global), `cmux events`, the chat view, Codex and Kiro hooks, remote sessions.

Tasks: 1 store and rendering; 2 wire and app methods; 3 CLI; 4 the hook and `hooks setup`; 5 check with two shells and a real hook run, docs.

## Part 2: workspace auto-naming (exit 3)

Reference: cmux `docs/workspace-auto-naming.md` and `CLI/CMUXCLI+AutoNaming.swift` @ `88b87c05`.

| | Decision | Why |
|---|---|---|
| N1 | Setting `[auto_naming]` in `config.toml`: `enabled` (default false) and `command`, a shell line that gets the prompt on stdin and prints the title. Default: cmux's `claude -p --model haiku` with tools, slash commands, session persistence and MCP servers off. Read again at every turn end. | Spec: opt-in, local, configurable command. cmux gates on the live setting, so no restart. |
| N2 | The app runs the pass, not the hook: the installed `Stop` hook (`hooks message`) also sends `workspace.auto_name {surface_id, transcript_path}` and returns at once. | No second hook to install, and naming can't block the agent. cmux runs it in an async hook and applies with `workspace.set_auto_title`. Deviation. |
| N3 | Titles carry who set them. An auto title never replaces one the user set (sidebar rename, `workspace.rename`, `--title`); clearing the name opens the workspace to naming again. | cmux "Precedence". |
| N4 | cmux's throttle, kept per workspace in memory: transcripts under 12 lines skipped, one pass at a time, 180 s between attempts, 6 new lines since the last name, a shrunk transcript only resets the count. | cmux `AutoNamingConfig`. Theirs is per session on disk; ours dies with the app like the rest (M10). |
| N5 | Context and prompt are cmux's: the first 2 user messages and the last 4 messages of the Claude transcript, 240 characters each; the answer's first line, unquoted, cut to 50 characters at a word. | Same names as cmux for the same talk. |
| N6 | The command runs under `sh -c` in the temp directory with every `CMUX_*` variable removed and a 60 s deadline. Any failure leaves the name alone. | Without `CMUX_*` the summarizer's own `Stop` hook does nothing, so it can't name itself in a loop. |

Not done: naming the agent's tab, Codex and other agents' transcripts, a settings UI.

Tasks: 1 title provenance in `cmux-core`; 2 setting in `cmux-config`; 3 engine in `cmux-meta::autoname` (throttle, transcript, prompt, title, runner); 4 `workspace.auto_name` in the app; 5 the hook sends it; 6 check against the app with a fake command, docs.

## Part 3: automations (exit 4)

Reference: cmux `docs/automations.md`, `docs/events.md`, `Sources/AutomationRule.swift` and `AutomationEngine.swift` @ `88b87c05`.

| | Decision | Why |
|---|---|---|
| A1 | Rules in `~/.config/cmux-linux/automations.json`, cmux's format: `version: 1`, `rules[]` with `id`, `enabled`, `when` (`event`, `category`; exact or a `*` prefix/suffix), `where`, `rate_limit`, `then`. Loaded at start and by `cmux automation reload`. A rule that doesn't parse, or uses an action we don't have, is left out with a warning; the rest load. | cmux "Configuration". A half-run rule is worse than a missing one. |
| A2 | The event bus is one `emit` in the app, called where the thing happens: `workspace.created`, `workspace.closed`, `notification.created`, `feed.item.received`, `agent.message.queued`, `agent.hook.Stop`. cmux's names; the payload carries `workspace_id`, `workspace_title`, `surface_id` and the event's own fields. | The events with one clear source today. No `cmux events` stream, log file, sequence numbers or redaction: nothing leaves the process except through a rule. |
| A3 | `where`: cmux's aliases (`workspace.title`/`title`, `agent`, `surface.kind`) and plain payload keys; a value is equal ignoring case, or an object of `equals`, `contains`, `prefix`, `suffix`, `in`, `not`. | cmux `AutomationRule`. `workspace.tag` never matches: we have no tags. |
| A4 | Rate limit per rule, default one firing a second (`interval_seconds`, `maximum`). | cmux. |
| A5 | Actions, in order, the next after the last ends: `notify` (title, subtitle, body with `{{event.name}}`, `{{event.category}}`, `{{event.source}}`, `{{payload.<key>}}`; target defaults to the event's surface) and `run` (`sh -c`, cmux's `CMUX_AUTOMATION_*` variables, 60 s, `timeout_seconds` up to 300). No `rpc` or `webhook`. | `run` reaches both: `cmux ...` for the socket, `curl` for a webhook. Deviation. |
| A6 | A notification made by a rule emits no event. | cmux tracks a rule chain; this stops the one loop that needs no outside process. A `run` that calls `cmux` can still loop, at the rate limit. |
| A7 | Socket `automation.list` and `automation.reload`; CLI `cmux automation list` and `reload`. | Enough to see what loaded and why not. No `show`, `test`, `enable`, `disable`, `logs`. |

Tasks: 1 rules, matching, rate limit and templates in `cmux-ipc::automation`; 2 `emit` and the two actions in the app; 3 the emit points; 4 socket and CLI; 5 check a sample rule against the app, docs.

## Part 4: Claude Teams (exit 2)
`cmux claude-teams`: teammates as native splits. Largest and least certain part (depends on Claude Code's teammate mode and its tmux shim); planned last.
