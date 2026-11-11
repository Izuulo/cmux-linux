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
Opt-in in `config.toml`; after an agent turn (the `Stop` hook reports it) run a configurable command (default `claude -p`) on the background executor and rename the workspace if it has no custom title. To plan in detail when Part 1 is merged.

## Part 3: automations (exit 4)
Event bus and ordered actions from a versioned JSON file. To plan after reading `docs/automations.md` against the Swift source.

## Part 4: Claude Teams (exit 2)
`cmux claude-teams`: teammates as native splits. Largest and least certain part (depends on Claude Code's teammate mode and its tmux shim); planned last.
