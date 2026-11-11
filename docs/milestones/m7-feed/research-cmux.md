# M7 research — cmux Feed and Claude Code hooks

Facts from cmux @ 43ce5e2e (2026-11-10) to match, not code to copy. Cited as `rf §n`. UNVERIFIED = not confirmed from source.

## 0. Sources and licence
- Read only GPL-3.0-or-later dirs: `Sources/`, `CLI/`, `Packages/macOS/`, `docs/`, `daemon/`, `Resources/bin/` (root `LICENSE`; no own LICENSE). Never `web/`, `workers/`, `services/`, `cmux-tui/relays/` (BUSL-1.1).
- Shorthands: `C` = `CLI/cmux.swift`, `CHS` = `CLI/CMUXCLI+ClaudeHookSettings.swift`, `FC` = `Sources/Feed/FeedCoordinator.swift`, `FP` = `Sources/Feed/FeedPanelView.swift`, `WS` = `Packages/macOS/CMUXAgentLaunch/Sources/CMUXAgentLaunch/Workstream/`, `TC` = `Sources/TerminalController.swift`, `RH` = `daemon/remote/cmd/cmuxd-remote/claude_hook_install.go`, `W` = `Resources/bin/cmux-claude-wrapper`.
- `CCH` = Claude Code hooks reference (`code.claude.com/docs/en/hooks.md`, fetched 2026-11-11), PermissionRequest section lines 2144-2240.

## 1. docs/feed.md and docs/agent-hooks.md
- Feed = right-sidebar surface for agent decisions on `Ctrl-4`. Three actionable kinds: permission (Once / Always / All tools / Bypass / Deny), ExitPlanMode (Ultraplan / Manual / Auto, plus Deny-with-feedback), AskUserQuestion (pick one or several, Submit). Everything else is stored as informational activity (`docs/feed.md:3-9`).
- Flow: agent hook → `cmux hooks feed --source <agent>` → v2 `feed.push` → `FeedCoordinator` parks the hook on a semaphore keyed by `request_id` (≤120 s) → `WorkstreamStore` (ring + JSONL) → `FeedPanelView` + native notification if the window isn't focused (`docs/feed.md:15-46`).
- Replies: `feed.permission.reply` / `feed.question.reply` / `feed.exit_plan.reply` wake the hook, which prints the agent's decision JSON on stdout (`docs/feed.md:48`).
- Audit log `~/.cmuxterm/workstream.jsonl`; memory ring 2000 items (`docs/feed.md:50`; `WS/WorkstreamStore.swift:10`).
- Timeout: "Feed is advisory, not blocking." ≤120 s wait, then `{}` and the agent falls through to its own TUI prompt. Claude's PermissionRequest hook timeout 125 s (`docs/feed.md:139-143`).
- Claude Code is **wrapper-injected**, not installed by `cmux hooks setup` (`docs/feed.md:77`, `docs/agent-hooks.md:5,33`). `cmux hooks setup` covers ~19 other agents (list `docs/agent-hooks.md:14`), skips those without a binary on `PATH`.
- Settings-file install for Claude exists only on remote hosts: `cmux claude-hook install|uninstall` writes `${CLAUDE_CONFIG_DIR:-~/.claude}/settings.json`, "removes only the cmux entries" (`docs/agent-hooks.md:20-25`). Those relay hooks do **not** include the Feed bridge: "Permission prompts stay in Claude on the remote host" (`docs/agent-hooks.md:18`).
- Jump: double-click a row → `workspace.select` + `surface.focus` for the agent's surface; no-op if the session isn't mapped (`docs/feed.md:162-164`).
- Decision table claims Bypass sends session bypass and the wrapper adds `--allow-dangerously-skip-permissions` (`docs/feed.md:117-120`). **Stale**: the wrapper has no such flag (`grep dangerously W` only hits comments) and Claude cards hide Bypass (§4.4).

## 2. Hook install
### 2.1 What cmux installs for Claude (wrapper path)
- `W` (bash, 2459 lines) intercepts `claude` in cmux terminals and adds one `--settings` JSON produced by hidden `cmux hooks claude inject-settings` (`W:834-857`, `C:45753-45759`). Passthrough (no hooks) outside cmux or with `CMUX_CLAUDE_HOOKS_DISABLED=1` (`W:939-969`, `docs/agent-hooks.md`).
- Generated settings (`CHS:9-101`), keys sorted. The parts we need:
  ```
  "PermissionRequest": [{"matcher":"","hooks":[{"type":"command",
     "command":"\"${CMUX_CLAUDE_HOOK_CMUX_BIN:-cmux}\" hooks feed --source claude","timeout":125}]}]
  ```
  plus lifecycle groups `SessionStart`, `Stop`, `StopFailure`, `SessionEnd`, `Notification`, `UserPromptSubmit`, `PreToolUse` (matcher `""`), `PostToolUse` (matcher `PushNotification`), `SubagentStop` — queued/async, used for status and for "decided elsewhere" (§4.3). Top level also sets `"preferredNotifChannel":"notifications_disabled"`.
- Only `PermissionRequest` blocks. Claude fires it for normal tools, `ExitPlanMode` and `AskUserQuestion`; cmux reroutes the last two by `tool_name` (`CLI/FeedEventClassifier.swift:151-169, 262-275`). Under `bypassPermissions` mode Claude fires no PermissionRequest at all (`W:2172-2176`).

### 2.2 Settings-file installer (closest model for our `cmux hooks setup`)
- `cmux claude-hook install|uninstall [--settings-file <p>]` (`RH:27-103`). Default path `$CLAUDE_CONFIG_DIR/settings.json` else `~/.claude/settings.json` (`RH:106-115`). Usage error → exit 2.
- Ownership marker: every installed command contains `claude-hook --user-settings ` (`RH:25`); command form `test -x <bin> && <bin> claude-hook --user-settings <event> || :` so it is a no-op if the CLI is gone (`RH:131-148`). Timeout 5 s (relay hooks are non-blocking).
- Install = remove all marker entries, then merge fresh ones (`RH:64-73`). Removal drops marker hooks, then any group / event / `hooks` object left empty; user hooks (even in the same group) and user-written empty entries stay (`RH:150-203`).
- Refuses when the file isn't one JSON object or `hooks` isn't an object (`RH:65-68, 222-224`). Writes only if the re-encoded JSON changed; uninstall with no file creates nothing (`RH:234-247`).
- Write: resolve symlink, temp file in same dir, keep mode (new file `0600`, dir `0700`), fsync, atomic rename; 2-space indent, keys sorted, numbers kept verbatim (`RH:210-304`). **No backup file.**
- Messages: `cmux: installed status hooks in <p>` + `cmux: restart running agent sessions to load them`; `cmux: status hooks are already installed in <p>`; `cmux: removed status hooks from <p>`; `cmux: no status hooks installed in <p>` (`RH:75-92`).
- Generic `cmux hooks setup` path (other agents, `C:37768-37990`): same remove-then-insert, but re-inserts cmux groups at their old index, shows a preview and `Proceed? [y/N]` unless `--yes`/`-y`, prints `<Agent> hooks already up to date at <p>` on no-op. Ownership via `isCmuxOwnedHookCommand` (`CLI/CMUXCLI+AgentHookDefinitions.swift:657`).
- Setup header: `cmux hooks setup: installing agent hooks` and `  (Antigravity, Claude Code, Grok, Muse, and Pi hooks are injected automatically via cmux wrappers)` (`C:46039-46040`).

## 3. Hook command at runtime (`cmux hooks feed --source claude`, `C:44558-45184`)
- Missing `--source` → `cmux hooks feed requires --source <agent-name>` (`C:44567`). Reads all stdin; empty / non-object → prints `{}` (`C:44612-44618`).
- Claude stdin fields used: `hook_event_name`, `tool_name`, `tool_input`, `session_id`, `cwd`, `transcript_path`, `permission_mode`, `permission_suggestions`; `tool_use_id` if present (`C:44622-44626, 44885-44950`). Per `CCH` §PermissionRequest input, Claude sends **no `tool_use_id`** here, so the request id is a fresh UUID (`C:44944-44947`).
- Agent PID: `CMUX_CLAUDE_PID` from the wrapper, else `getppid()` (`C:42516-42540, 44659`) → `_ppid`, used to expire cards when the agent dies (§4.2).
- Target: claimed `workspace_id` / `surface_id` from `CMUX_WORKSPACE_ID` / `CMUX_SURFACE_ID`, validated live against the app (`surface.list` / `agent.resolve_delivery_target`, then PID→TTY binding) (`C:44842-44860`, `CLI/CMUXCLI+AgentHookProcessBinding.swift:8-56`). **No live target → `{}` immediately** (`C:44861`). So outside cmux the hook is a no-op.
- Socket: normal CLI resolution (`CMUX_SOCKET_PATH` …, M5 rc §1). Connect/auth failure → `{}` (`C:45072-45087`).
- Request (one v2 line, with `id`):
  ```
  {"method":"feed.push","id":"<uuid>","params":{"wait_timeout_seconds":<s>,"event":{
    "session_id":"<workstream id>","hook_event_name":"PermissionRequest|ExitPlanMode|AskUserQuestion",
    "_source":"claude","_ppid":<pid>,"workspace_id":"…","surface_id":"…","cwd":"…",
    "tool_name":"…","tool_input":{…},"_opencode_request_id":"<uuid>","_hook_sent_at_ms":<ms>,
    "context":{…},"_cmux_ordered_hook":true}}}
  ```
  (`C:44862-44950, 45117-45123`). Server decodes keys in `WS/WorkstreamEvent.swift:94-109`. `session_id` is a namespaced workstream id (`Self.feedWorkstreamID`, `CmuxFoundation/FeedWorkstreamIdentifier.swift`).
- Timeouts: hook process 125 s declared (`CHS:85-90`); CLI deadline 118 s, decision wait **115 s** (`CLI/CMUXCLI+AgentHookDefinitions.swift:8-10`); server rejects `wait_timeout_seconds` outside 0…120 with `invalid_params` / `feed.push wait_timeout_seconds must be between 0 and 120` (`TC:6748-6754`).
- Result `{"status":"resolved","decision":{…},"item_id":…}` | `acknowledged` | `timed_out` | `not_found` | `unavailable` (`FC:1933-1955`). Anything but `resolved` (or socket error/timeout) → stdout `{}`, exit 0 → Claude shows its own prompt (`C:45166-45183`).
- Decision dicts (`FC:1957-1970`): `{"kind":"permission","mode":"once|always|all|bypass|deny"}`, `{"kind":"exit_plan","mode":"ultraplan|bypassPermissions|autoAccept|manual|deny","feedback"?:"…"}`, `{"kind":"question","selections":["…"]}`.

### 3.1 Stdout per choice (Claude; `C:45414-45436, 45482-45497, 45530-45563, 45614-45643, 45687-45710`)
All are `{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":D}}`, keys sorted. `D`:
| Choice | D |
|---|---|
| Once | `{"behavior":"allow"}` |
| Always / All tools | `{"behavior":"allow","updatedPermissions":<stdin permission_suggestions verbatim>}` (omitted if absent/empty) |
| Bypass | `{"behavior":"allow"}` (same as Once; not offered for Claude, §4.4) |
| Deny | `{"behavior":"deny","message":"User denied permission via cmux Feed."}` (no free-text deny for permissions) |
| Plan Manual / `bypassPermissions` | `{"behavior":"allow","updatedInput":<tool_input>}` |
| Plan Auto | `{"behavior":"allow","updatedInput":<tool_input>,"updatedPermissions":[{"type":"setMode","mode":"auto","destination":"session"}]}` |
| Plan Ultraplan | `{"behavior":"deny","message":"User chose Ultraplan via cmux Feed. Refine this plan with Ultraplan on Claude Code on the web."}` |
| Plan with feedback (any mode) | `{"behavior":"deny","message":"User rejected the plan via cmux Feed and wants this change: <feedback>"}` |
| Plan deny | `{"behavior":"deny","message":"User rejected the plan via cmux Feed."}` |
| Question | `{"behavior":"allow","updatedInput":<tool_input + "answers":{<question text>: <answer>}>}`; key falls back to `Answer <n>` |
| Skip interview | `{"behavior":"deny","message":"User chose Skip interview and plan immediately via cmux Feed. Do not ask more interview questions. Write the plan now."}` (selections == `["Skip interview and plan immediately"]`) |
- Matches `CCH`: `decision.behavior` allow/deny, `updatedInput` (required for AskUserQuestion/ExitPlanMode), `updatedPermissions` (same entry shape as `permission_suggestions`: `addRules`/`setMode`/… with `destination` `session|localSettings|projectSettings|userSettings`), `message` for deny, optional `interrupt` (cmux never sets it). `answers` maps question text → label, multi-select labels joined with `, ` (`CCH:1977`), which is what Feed composes (`FP:3050-3069`).
- Quirk: Feed skips unanswered questions, but the CLI keys answers by array index → a skipped question shifts later answers' keys (`FP:3060`, `C:45695-45704`). Don't copy.
- Non-Claude agents use PreToolUse-style `permissionDecision` output (`C:45438-45480`); out of our scope.

## 4. App side
### 4.1 Model (`WS/`)
- `WorkstreamItem`: `id` UUID, `workstreamId`, `source`, `kind`, `createdAt`, `updatedAt`, `cwd`, `title`, `status`, `payload`, `context`, `reply`, `ppid` (`WorkstreamItem.swift:47-105`).
- Kinds: actionable `permissionRequest`, `exitPlan`, `question`; telemetry `toolUse`, `toolResult`, `userPrompt`, `assistantMessage`, `sessionStart`, `sessionEnd`, `stop`, `todos` (`WorkstreamKind.swift:6-29`).
- Status: `pending`, `resolved(decision, at)`, `expired(at)`, `telemetry` (`WorkstreamItem.swift:15-30`). No separate "resolved elsewhere": it becomes `expired`.
- Payloads: `permissionRequest(requestId, toolName, toolInputJSON, pattern)`, `exitPlan(requestId, plan /*raw tool_input JSON*/, defaultMode=.manual)`, `question(requestId, [prompt{id, header, prompt, multiSelect, options[{id,label,description}]}])` (`WorkstreamPayload.swift`, `WorkstreamStore.swift:396-430`). Question parse: `questions[]` with `question`/`header`/`multiSelect`/`options[{label,description}]`, ids `q<i>`/`opt<i>` (`WorkstreamQuestionPrompt+Parsing.swift`).
- Plan preview reads `plan`, `allowedPrompts[]` (`prompt|description|text`), `planFilePath|plan_file_path` from tool_input (`WorkstreamContext.swift:104-160`).

### 4.2 Store rules
- Append; drop oldest past 2000 (`WorkstreamStore.swift:288-294`). UI shows newest first; status is **not** a sort key (answered items stay in place) (`FP:395-419`).
- Dedupe: waiter registry by `request_id`; a duplicate push joins the owner's wait instead of adding a card (`FC:316-318`, `FeedWaiterRegistry.swift`).
- `markResolved` / `markExpired` only act on pending items (`WorkstreamStore.swift:205-227`). Expired by: wait timeout (`FC:404-415`), agent PID exit (kqueue watcher on `_ppid`, `FC:149-164`), dead PIDs at launch (`WorkstreamStore.swift:362-380`), superseded (§4.3). Nothing deletes items except the ring and `cmux feed clear`.
- Persistence: every mutation appends the full item to `workstream.jsonl`; latest wins on replay (`WorkstreamStore.swift:268-285`).

### 4.3 Holding and replying
- `feed.push` with wait > 0 registers a waiter (semaphore), ingests on main, surfaces attention, posts a notification if still waiting, then blocks the socket worker until reply/deadline (`FC:251-415`). Without `id`, `feed.push` is one-way telemetry, no reply line (`TC:1501`).
- Reply methods (`TC:6873-6939`): params `{"request_id","mode"}`, `{"request_id","selections":[…]}`, `{"request_id","mode","feedback"?}` → `{"delivered":true}`. Errors `invalid_params`: `feed.permission.reply requires request_id`, `… requires mode ∈ once|always|all|bypass|deny`, `feed.question.reply requires selections: [string]`, `feed.exit_plan.reply requires mode ∈ ultraplan|bypassPermissions|autoAccept|manual|deny`. Unknown/stale ids still return `delivered:true`.
- In-app buttons call `FeedCoordinator.deliverReply` directly, not the socket (`FP:931-956`). `deliverReply` wakes the waiter, marks the item resolved, clears attention + notification (`FC:531-562`).
- Answered in the terminal: Claude keeps the abandoned hook running until its own timeout. cmux retires the waiter when a **later-stamped** hook from the same Claude session arrives: `PreToolUse` (except AskUserQuestion/ExitPlanMode), `PostToolUse`, `PostToolUseFailure`, `UserPromptSubmit`, `Stop`, `SessionEnd`. Hook returns `{}`, card → expired, attention cleared (`FC:566-645`). Stamp `_hook_sent_at_ms` is set only after a per-agent ordering barrier (`C:45090-45107`).

### 4.4 Which buttons per agent (`Sources/Feed/FeedPermissionActionPolicy.swift:7-30`)
- Claude: Deny, Allow Once, Always Allow, All tools. **Bypass hidden** for claude, codex, hermes-agent. Always and All tools send identical output for Claude (§3.1).

## 5. Feed UI
- Lives as mode `feed` of the right sidebar (tabs Files / Find / Vault / Feed / Dock / Cloud; label `Feed`, icon `dot.radiowaves.left.and.right`) (`Sources/RightSidebarMode.swift:4-44`). Separate from the notification popover. Sidebar min width 295 (`CmuxSettings/Policies/RightSidebarWidthSettings.swift:28`).
- Shortcut `switchRightSidebarToFeed`, default `⌃4`, title `Show Sidebar Feed`, remappable (`ShortcutAction+Defaults.swift:107`, `+DisplayName.swift:56`). Always shows the sidebar, switches mode, focuses the first item; it does **not** toggle off (`MainWindowFocusController.swift:487-546`, `AppDelegate.swift:15575-15583`).
- Feed tab badge: pending count, `9+` cap, 10 pt bold monospaced, orange on orange 20 % capsule, pad 5×1; a11y `<label> · <n> pending` (`RightSidebarChromeStyle.swift:503-526`, `RightSidebarPanelView.swift:110-113`).
- Control bar: two pills `Actionable` (icon `exclamationmark.circle`, default) / `All Activity` (`checklist`), label 11 pt, icon 10 pt, spacing 6 (`FP:54-123`, `RightSidebarChromeStyle.swift:83-84`). Actionable = 3 actionable kinds; All Activity adds `todos` and `stop` (`FP:395-413`).
- Empty state: `No pending decisions` / `Permission, plan, and question requests from AI agents will appear here.` (Activity: `No activity yet` / `Agent decisions and todo-list updates will appear here.`), 12 pt / 11 pt secondary (0.7), centred, h-pad 16 (`FP:575-595`).
- Rows: 1 px dividers `primary 0.08`; row padding 12 h × 10 v, VStack spacing 10; resolved/expired rows at opacity 0.6 (`FP:1012-1025, 568-573`).
- Header: kind icon 12 pt medium in 14×14 (permission `lock.shield` orange, plan `list.bullet.rectangle` purple, question `questionmark.circle` blue), title 12 pt medium `primary 0.92` middle-truncated = `<cwd basename> · <last user prompt | question header | title>`; right chips: agent (`Claude`, fg rgb(0.92,0.54,0.29) on 18 %) and age (`<1m`, `5m`, `2h`, `3d`) 10 pt medium, pad 5×2, radius 4 (`FP:1074-1183, 3934-3946`). Below: context block / `You: <prompt>` 11 pt secondary, 2 lines (`FP:1014-1056`). Workspace name is **not** shown.
- Permission body: `exclamationmark.triangle.fill` + tool name 11 pt semibold orange; code box pad 10, radius 6, fill `primary 0.05`, stroke `primary 0.10`; preview: Bash `$ <command>` + description, Write/Edit/MultiEdit `file_path` (+ first content line for Write), Read `file_path`, else raw JSON; mono 11 pt, secondary 10 pt (`FP:1421-1597`).
- Permission buttons (one row, spacing 6, equal width): `Deny` (dark: white on black 0.75), `Allow Once` (light: black on white 0.88), `Always Allow` (accent), `All tools` (accent), `Bypass` (destructive red). After answer: one dimmed green `✓ Submitted · once|always|all tools|bypass|denied` (`FP:1430-1497, 13-27`).
- FeedButton medium: label 10.5 pt semibold, icon 10, pad 12×5, radius 6 (compact 10 pt, 8×4) (`FP:1835-1866`, `WindowChromeMetrics.swift:26,60`).
- Plan body: markdown for Claude (`PlanBodyView`), `Allowed prompts`, `Plan file`; feedback field `Tell Claude what to change…` (12 pt, 2–5 lines, pad 10, radius 6); buttons `Ultraplan` (soft; becomes accent `Send feedback` when text entered → sends manual+feedback), `Manual` (soft), `Auto` (green). No Deny button (`FP:2252-2352`).
- Question: per-question options (multi-select tag `Multi-select`), free text `Type something…`, `Submit All Answers` (accent, disabled until an answer); `Skip + plan immediately` shown for plan-interview questions (`FP:2643-3140`).
- Stop cards (Activity only): `Claude finished — reply to continue`, `Reply to Claude…`, `Send to Claude` types into the surface (`FP:3697-3733`). Not needed for exit criteria.
- Keyboard in Feed: `j`/`↓`/`⌃N` next, `k`/`↑`/`⌃P` previous, Return/Enter = jump to the agent's surface, Esc = focus terminal; sidebar-mode shortcuts still work (`FP:763-822`, `RightSidebarMode+Availability.swift:124-144`). Double-click also jumps (`FP:1045`). No per-button keys.
- Attention while pending: target panel gets lifecycle `needsInput` + sidebar status `Needs input` (`bell.fill`), workspace reordered if "Reorder on Notification" is on; cleared on reply/timeout/supersede (`FC:745-1020`). Desktop notification with inline actions (categories `CMUXFeedPermission`, `CMUXFeedExitPlan`, `CMUXFeedQuestion`) when app inactive (`docs/feed.md:174`, `FC:1284-1440`).

## 6. Exit-criteria details
- Always/All tools never write settings themselves; they echo `permission_suggestions` as `updatedPermissions`, so Claude persists them (typically `addRules`, `destination` `localSettings` → `.claude/settings.local.json`) (`C:45492-45496`, `CCH` permission update entries).
- Bypass: for Claude it is plain allow and hidden; a real bypass would need `setMode bypassPermissions` plus a launch flag (`CCH` note). Plan `Auto` = `setMode auto` for the session.
- Background workspace: routing uses `CMUX_WORKSPACE_ID`/`CMUX_SURFACE_ID` of the terminal Claude runs in, not the focused one; card shows regardless of focus (`FC:345-352`).
- Hook must run inside a cmux terminal (env vars set, M5 rc §5); otherwise `{}`.

## 7. Where cmux-linux differs today
- No right sidebar: M4's notification panel is a 560×760 popover overlay (`crates/cmux-app/src/window_view/panel.rs:1-40`). Feed needs a docked right panel with mode tabs.
- Server answers every app request within `DEADLINE` 10 s and reads with a 30 s timeout (`crates/cmux-ipc/src/server.rs:20-23, 331-335`). `feed.push` must block ≤115 s without stalling other connections.
- No `feed.*` methods, no `hooks` / `claude-hook` CLI, no Claude wrapper. We'll install into `~/.claude/settings.json` instead (cmux's remote-host model, §2.2) but add the `PermissionRequest` Feed hook that cmux only injects via the wrapper.

## 8. Suggested cut for a first port
- Essential: `cmux hooks feed --source claude` for PermissionRequest (stdin → `feed.push` with `id`, wait 115 s, print §3.1 JSON or `{}` on any failure).
- Essential: `feed.push` blocking handler + `feed.permission.reply` / `feed.exit_plan.reply` / `feed.question.reply` with cmux params/results.
- Essential: Feed item store (pending/resolved/expired, newest first, dedupe by request id, expire on timeout).
- Essential: right-sidebar Feed panel with permission card (Deny / Allow Once / Always Allow / All tools) and plan card (Ultraplan / Manual / Auto + feedback), `⌃4` remappable.
- Essential: `cmux hooks setup` / `cmux hooks uninstall` for Claude only: marker-based remove-then-merge into `$CLAUDE_CONFIG_DIR|~/.claude/settings.json`, atomic write, keep mode, no-op if unchanged; `PermissionRequest` matcher `""`, timeout 125, `test -x … || :` guard (§2.2).
- Essential: no-op outside cmux (no `CMUX_SURFACE_ID` / no socket → `{}`).
- Near-essential: question card (AskUserQuestion goes through the same hook; needed or Claude's questions hang 115 s).
- Defer: supersede-by-later-hook (needs PreToolUse/PostToolUse/Stop/UserPromptSubmit hooks + ordering); fall back to the 115 s timeout.
- Defer: PID-exit expiry, JSONL persistence, All Activity view, todos/stop cards, Stop reply box.
- Defer: desktop notifications with inline actions, Feed tab badge, Needs-input status/reorder (M4 attention ring may cover some).
- Defer: Bypass button (hidden for Claude anyway), Ultraplan semantics beyond the deny message, other agents, `cmux feed tui`/`clear`, events stream.
