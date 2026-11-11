# M7 feed Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Claude Code permission, plan and question prompts from any workspace show up in a right-sidebar Feed, and answering there unblocks the agent.

**Architecture:**
- A Claude Code `PermissionRequest` hook runs `cmux hooks feed --source claude`. It sends cmux's v2 `feed.push` over the M5 socket and blocks until the app answers, then prints Claude's decision JSON (or `{}` so Claude falls back to its own prompt).
- `cmux-core::feed` is the pure model (items, statuses, decisions). `cmux-ipc::feed` turns JSON into that model and decisions back into JSON; the CLI and the app share it. The server lets `feed.push` wait up to its own timeout instead of the 10 s app deadline.
- The app holds each pending `feed.push` request until a Feed button, a `feed.*.reply` call, or the timeout answers it. The Feed is a docked right sidebar (one mode for now). `cmux hooks setup|uninstall` edits Claude Code's `settings.json`.

**Tech Stack:** Rust 2024, gpui `=0.2.2`, serde_json, uuid (all already dependencies). No new crates.

**Spec:** `docs/milestones/m7-feed/spec.md`. **Research:** `docs/milestones/m7-feed/research-cmux.md` (cited `rf §n`; cmux @ 43ce5e2e).

## Global Constraints

Carried from M0–M6:
- Licence GPL-3.0-or-later. Never copy from cmux's BUSL-1.1 directories. Short strings (UI text, JSON keys, messages) copied from cmux are fine and already covered by `NOTICE`'s cmux line.
- Zig exactly 0.16.0; `export PATH="$PWD/.tools/zig:$PATH"` from the repo root before any cargo command. `--locked` on build, test and clippy; never a bare `cargo update`.
- Before each commit: `cargo fmt -p <crates touched>` and `cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings`.
- `cmux-core` depends on nothing; `cmux-ipc` never depends on gpui. `cmux-ipc` may now depend on `cmux-core` (path dependency, Task 2).
- macOS host: run the app only from `cargo build --release --features gpui/runtime_shaders` (debug builds hang).
- No personal info in tracked files. `STATUS.md` max 100 lines, archived before replacement.

New for M7:
- Branch `m7-feed`, cut from `m6-parity-release` (M6 isn't merged yet; rebase onto `master` after it is). One commit per step that changes files; messages end with the session's attribution lines.
- **Never run `cmux hooks setup` or `uninstall` without `--settings-file <temp path>`** on the dev machine, and never touch `~/.claude/` in tests: tests pass a temp `--settings-file` or a temp `CLAUDE_CONFIG_DIR` in the CLI's `Ctx`. Never drive or signal the owner's running `cmux.app` or installed cmux-linux.
- serde_json has `preserve_order` on in workspace builds (gpui enables it) but not in `-p cmux-ipc` builds. Tests compare parsed `serde_json::Value`s, never JSON strings.
- This plan's code was not compiled. Interface changes go in the journal.

## Review Focus

1. **Claude runs inside the owner's `cmux.app` on the Mac** while our hook is installed in `~/.claude/settings.json`. cmux's own wrapper hook answers; ours must print `{}` at once and never talk to cmux.app's socket. Test: Task 4 `feed_hook_outside_cmux_linux_prints_empty` (no `CMUX_LINUX`).
2. **The app isn't running, the socket is stale, or the app quits while a card waits.** The hook prints `{}` and exits 0, so Claude shows its own prompt; it never errors or hangs past 118 s. Test: Task 4 `feed_hook_without_socket_prints_empty`, `feed_hook_timed_out_prints_empty`.
3. **A user's `settings.json` with their own hooks, other keys, comments-free odd formatting, or a symlink** (dotfile repos). Install and uninstall touch only our entries, keep everything else and the file's mode, follow the symlink, and a second install writes nothing. Test: Task 5 `install_keeps_user_hooks_and_is_idempotent`, `uninstall_restores_original`, `install_follows_symlink_and_keeps_mode`, `refuses_non_object`.
4. **The user answers in the terminal instead, or Claude is interrupted.** The card stays pending until the wait ends (115 s), then shows as expired; a click on it after that does nothing harmful. Test: Task 1 `resolve_after_expire_is_ignored`; Task 6 timer path.
5. **The workspace or its terminal closes while a card waits.** The card expires and the hook gets `{}` right away, not after 115 s. Test: Task 1 `expire_workspace_returns_pending_requests`; Task 6 wiring.

## Decisions made in this plan

| Decision | Why |
|---|---|
| **D1 Hook protocol = cmux's** (rf §3): one `PermissionRequest` hook, `tool_name` picks the kind (`ExitPlanMode` → plan, `AskUserQuestion` → question, else permission); v2 `feed.push` with `id` and `params {wait_timeout_seconds: 115, event: {…}}`; result `{"status":"resolved","decision":{…}}` or `timed_out` / `acknowledged` / `not_found` / `unavailable`; replies `feed.permission.reply`, `feed.exit_plan.reply`, `feed.question.reply` → `{"delivered":true}`; stdout per rf §3.1. Timings: hook timeout 125 s, CLI read timeout 118 s, wait 115 s, server accepts 0…120. | Scripts that speak cmux's socket keep working; Claude's documented schema. |
| **D2 Install = `cmux hooks setup` / `cmux hooks uninstall`** (Claude Code only), into `--settings-file`, else `$CLAUDE_CONFIG_DIR/settings.json`, else `$HOME/.claude/settings.json`. cmux injects Claude's hooks through a `claude` wrapper (rf §2.1); we have no wrapper, so we follow cmux's settings-file installer (rf §2.2): marker-based remove-then-merge, atomic write, keep mode, write only on change, no backup. Entry: `"PermissionRequest": [{"matcher": "", "hooks": [{"type": "command", "command": "test -x '<abs cmux>' && '<abs cmux>' hooks feed --source claude || :", "timeout": 125}]}]`. Marker: a command containing `hooks feed --source claude`. No `Proceed? [y/N]` prompt. | Spec exit 3 (idempotent, reversible). The absolute path keeps cmux.app's `cmux` from answering (Review Focus 1); `test -x … || :` makes a removed binary a silent no-op. |
| **D3 Our terminals get `CMUX_LINUX=1`** (added to `terminal_env`). The hook acts only when it, `CMUX_WORKSPACE_ID` and `CMUX_SURFACE_ID` are set; otherwise `{}`. | Review Focus 1. M6 already strips inherited `CMUX_*`, so a nested app doesn't leak it. |
| **D4 Server:** `feed.push` waits `wait_timeout_seconds + 5 s` for the app instead of `DEADLINE`; out of range → `invalid_params` `feed.push wait_timeout_seconds must be between 0 and 120`, answered by the server without asking the app. | The app answers `timed_out` itself at `wait`; the 5 s is slack. One thread per connection already, so a long wait blocks nobody else. |
| **D5 App holds the `Request`:** the request loop hands `feed.push` to `WindowView::feed_push(req, …)`, which keeps it in a map keyed by request id and arms a timer for `wait`. Every end (button, socket reply, timer, workspace closed) goes through one `feed_finish`. | One place answers each request exactly once. |
| **D6 Attention through M4:** a pending card posts an M4 notification on the agent's surface (ring, unread, desktop bubble per M4 rules) with cmux's Feed texts (`Sources/Feed/FeedCoordinator.swift:1306-1337`): title `Claude permission` / `Claude plan ready` / `Claude question`, body `<Tool> needs approval` (fallback `Decision needed`) / `Review and approve the plan` / `Agent is asking a question`. Removed when the card ends. | cmux marks the panel "Needs input" and posts a notification; M4's ring and bubble cover both without new UI. Deviation: the item also lists in the notification panel. |
| **D7 Right sidebar:** docked on the right, fixed width 295 (cmux's minimum, `RightSidebarWidthSettings.swift:28`; its 220 default clamps to it), one mode (Feed) with a header `Feed` and an orange pending badge (rf §5). Closed at launch. Config actions: `toggleRightSidebar` `super+opt+b`, `focusRightSidebar` `super+shift+e`, `switchRightSidebarToFeed` `ctrl+4` (cmux defaults, `ShortcutAction+Defaults.swift:103-107,200`). `ctrl+4` only acts while focus is in the right sidebar (gpui key context `RightSidebar`), so in a terminal it still selects tab 4, as in cmux (`AppDelegate.swift:15570-15584`, `8049-8059`). The workspace area shrinks by the sidebar width (a `right_inset` next to `left_inset`). | Spec's "keep ⌃4, remappable". |
| **D8 Cards (Claude):** permission `Deny` / `Allow Once` / `Always Allow` / `All tools` (no Bypass, rf §4.4). Plan: one-line feedback field `Tell Claude what to change…`, then `Send feedback` (enabled only with text; manual + feedback), `Manual`, `Auto`. Question: options per question (multi-select toggles), `Submit All Answers` enabled when every question has an answer. After an answer: `✓ Submitted · <summary>`. | cmux's sets, minus Ultraplan (cmux Cloud, not planned in `docs/design.md`) and free-text "Other" answers. Requiring every question answered avoids cmux's shifted-answer quirk (rf §3.1). |
| **D9 Feed keyboard** (rf §5): `j`/`down`/`ctrl-n` next, `k`/`up`/`ctrl-p` previous, `enter` jumps to the agent's surface (`select` + `focus_surface`, as `surface.focus` does), `escape` returns focus to the terminal. Double-click on a card jumps too. Typing into the plan feedback field takes keys first. | cmux's; no per-button keys. |
| **D10 Store:** newest first, cap 2000 (oldest dropped), answered and expired items stay in place dimmed to 0.6. Not persisted. | cmux's ring; persistence is M10. |

## Deferred (with reason)
- Retiring a card when the user answers in the terminal (needs PreToolUse/PostToolUse/Stop/UserPromptSubmit hooks and cmux's ordering barrier, rf §4.3): the 115 s timeout covers it (Review Focus 4).
- Agent-PID expiry, JSONL audit log, `All Activity` view, todo/stop cards, desktop-notification inline actions, Bypass, Ultraplan, free-text answers, markdown rendering of plans (plain text here), sidebar resizing, other agents, `cmux feed tui|clear`.
- Other right-sidebar modes (Files, Find, Sessions, Dock is M8).

## File structure
- Create `crates/cmux-core/src/feed.rs`: `FeedStore`, `FeedItem`, `FeedKind`, `Preview`, `QuestionPrompt`, `Decision`, modes, `age_label`. Export from `lib.rs`.
- Create `crates/cmux-ipc/src/feed.rs`: JSON ↔ model (`parse_push`, `wait_of`, `parse_reply`, `decision_json`, `decision_from_json`, `hook_output`, `push_params`). `crates/cmux-ipc/Cargo.toml`: `cmux-core` path dependency.
- Modify `crates/cmux-ipc/src/server.rs` (D4), `env.rs` (D3), `cli/mod.rs` (`Ctx.stdin`, `Ctx.exe`, `hooks` dispatch), `cli/help.rs`.
- Create `crates/cmux-ipc/src/cli/hooks.rs`: `feed`, `setup`, `uninstall`; settings editing as pure functions on `Value`.
- Create `crates/cmux-ipc/tests/cli_hooks.rs`; extend `tests/server.rs`, `tests/support/mod.rs`.
- Create `crates/cmux-app/src/window_view/feed.rs` (sidebar view, cards, keys) and `window_view/feed_ctl.rs` (D5 waiters, `feed_push`, `feed_finish`, reply methods). Modify `window_view.rs` (fields, request loop, layout, workspace close), `control.rs` (reply methods), `workspace.rs` (`right_inset`), `actions.rs` + `cmux-config/src/keybindings.rs` (three actions).
- Docs: README "Feed" section, journal, STATUS.

---

### Task 1: Feed model (`cmux-core::feed`)

**Files:** Create `crates/cmux-core/src/feed.rs`; modify `crates/cmux-core/src/lib.rs`.

**Interfaces — Produces:**
```rust
pub struct FeedItemId(pub u64);                       // Copy, Eq, Hash, Debug
pub enum Preview { Command { command: String, description: Option<String> },
                   File { path: String, first_line: Option<String> }, Raw(String) }
pub struct QuestionOption { pub label: String, pub description: String }
pub struct QuestionPrompt { pub header: String, pub question: String,
                            pub multi_select: bool, pub options: Vec<QuestionOption> }
pub enum FeedKind { Permission { tool: String, preview: Preview },
                    ExitPlan { plan: String, allowed_prompts: Vec<String>, plan_file: Option<String> },
                    Question { prompts: Vec<QuestionPrompt> } }
pub enum PermissionMode { Once, Always, All, Bypass, Deny }
pub enum PlanMode { Ultraplan, BypassPermissions, AutoAccept, Manual, Deny }
pub enum Decision { Permission(PermissionMode),
                    ExitPlan { mode: PlanMode, feedback: Option<String> },
                    Question { selections: Vec<String> } }   // one entry per question, in order
pub enum FeedStatus { Pending, Resolved(Decision), Expired }
pub struct NewItem { pub request_id: String, pub source: String, pub workspace: WorkspaceId,
                     pub surface: SurfaceId, pub cwd: String, pub created_ms: u64, pub kind: FeedKind }
pub struct FeedItem { pub id: FeedItemId, /* NewItem's fields */, pub status: FeedStatus }
impl FeedItem { pub fn title(&self) -> String }       // "<cwd basename> · <tool | Plan | first question header>"
impl Decision { pub fn summary(&self) -> String }
pub struct FeedStore;  // Default
impl FeedStore {
    pub fn push(&mut self, item: NewItem) -> Option<FeedItemId>;          // None: request_id known
    pub fn resolve(&mut self, request_id: &str, d: Decision) -> Option<FeedItemId>; // only if pending
    pub fn expire(&mut self, request_id: &str) -> Option<FeedItemId>;     // only if pending
    pub fn expire_workspace(&mut self, ws: WorkspaceId) -> Vec<String>;   // request ids expired
    pub fn items(&self) -> impl Iterator<Item = &FeedItem>;               // newest first
    pub fn get(&self, id: FeedItemId) -> Option<&FeedItem>;
    pub fn pending_count(&self) -> usize;
}
pub fn age_label(now_ms: u64, created_ms: u64) -> String;   // "<1m", "5m", "2h", "3d"
```
`summary`: permission `once` / `always` / `all tools` / `bypass` / `denied` (cmux, `FP:13-27`); plan `manual` / `auto` / `feedback sent` / `denied` / `ultraplan`; question `answered`.

- [ ] **Step 1: Write failing tests** in `feed.rs` `#[cfg(test)]`:
  `push_dedupes_by_request_id` (second push with same id → `None`, one item); `items_are_newest_first`; `resolve_marks_pending_only` (resolve → `Resolved`; second resolve → `None`); `resolve_after_expire_is_ignored`; `expire_workspace_returns_pending_requests` (two pending in ws 1, one resolved in ws 1, one pending in ws 2 → returns the two ids, ws 2 untouched); `pending_count_counts_pending`; `cap_drops_oldest` (2001 pushes → 2000 items, first gone); `age_labels` (`0 → "<1m"`, `59_999 → "<1m"`, `5 min → "5m"`, `2 h → "2h"`, `3 d → "3d"`, `created > now → "<1m"`); `titles` (cwd `/home/u/app` + Bash → `app · Bash`; plan → `app · Plan`; question header `Auth` → `app · Auth`; empty cwd → just the tail).
- [ ] **Step 2:** `cargo test -p cmux-core --locked feed` → fails to compile.
- [ ] **Step 3:** Implement. A `VecDeque<FeedItem>` (push front) and a counter for ids is enough.
- [ ] **Step 4:** Tests pass; clippy clean.
- [ ] **Step 5:** Commit `m7: feed model in cmux-core`.

### Task 2: Feed wire format (`cmux-ipc::feed`)

**Files:** Create `crates/cmux-ipc/src/feed.rs`; modify `crates/cmux-ipc/src/lib.rs`, `crates/cmux-ipc/Cargo.toml` (`cmux-core = { path = "../cmux-core" }`).

**Interfaces — Consumes:** Task 1 types. **Produces:**
```rust
pub const WAIT_SECS: u64 = 115; pub const MAX_WAIT_SECS: f64 = 120.0;
pub struct FeedPush { pub request_id: String, pub wait: Duration, pub source: String,
                      pub workspace_id: String, pub surface_id: String, pub cwd: String, pub kind: FeedKind }
pub fn wait_of(params: &Map<String, Value>) -> Result<Duration, RpcError>;
pub fn parse_push(params: &Map<String, Value>) -> Result<FeedPush, RpcError>;
pub fn parse_reply(method: &str, params: &Map<String, Value>) -> Result<(String, Decision), RpcError>;
pub fn decision_json(d: &Decision) -> Value;
pub fn decision_from_json(v: &Value) -> Option<Decision>;
pub fn push_params(stdin: &Value, workspace_id: &str, surface_id: &str, request_id: &str) -> Map<String, Value>;
pub fn hook_output(stdin: &Value, d: &Decision) -> Value;
```
Rules (rf §3, §4.1, §4.3):
- `wait_of`: `wait_timeout_seconds` missing → 0; not a number or outside 0…120 → `invalid_params` `feed.push wait_timeout_seconds must be between 0 and 120`.
- `parse_push` reads `params.event`: request id from `_opencode_request_id` (cmux's key), `_source`, `workspace_id`, `surface_id`, `cwd`, `tool_name`, `tool_input`. Missing event / request id / workspace_id / surface_id → `invalid_params` `feed.push requires event.<key>`. Kind by `tool_name`. Preview: `Bash` → `Command{command, description}`; `Write` → `File{file_path, first line of content}`; `Edit`/`MultiEdit`/`Read` → `File{file_path, None}`; else `Raw(pretty tool_input)`. Plan: `plan`, `allowedPrompts[]` (`prompt|description|text`), `planFilePath|plan_file_path`. Questions: `questions[]` with `question`, `header`, `multiSelect`, `options[{label, description}]`.
- `parse_reply` errors, exact: `feed.permission.reply requires request_id`, `feed.permission.reply requires mode ∈ once|always|all|bypass|deny`, `feed.question.reply requires selections: [string]`, `feed.exit_plan.reply requires mode ∈ ultraplan|bypassPermissions|autoAccept|manual|deny` (same `requires request_id` text with each method's name). `feedback` optional, blank → `None`.
- `decision_json`: `{"kind":"permission","mode":"once"}`, `{"kind":"exit_plan","mode":"manual","feedback":"…"}` (no `feedback` key when `None`), `{"kind":"question","selections":[…]}`.
- `push_params`: `{"wait_timeout_seconds": 115, "event": {"session_id", "hook_event_name": "PermissionRequest"|"ExitPlanMode"|"AskUserQuestion", "_source": "claude", "workspace_id", "surface_id", "cwd", "tool_name", "tool_input", "_opencode_request_id"}}` (`session_id`, `cwd`, `tool_*` copied from stdin; absent → omitted).
- `hook_output`: the rf §3.1 table exactly, wrapped as `{"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": D}}`. Question `answers` key = each question's `question` text from `stdin.tool_input.questions[i]`, falling back to `Answer <i+1>`.

- [ ] **Step 1: Failing tests** in `feed.rs`: `wait_bounds` (missing → 0 s, 115 → 115 s, 120.0 ok, -1 and 121 → the exact error); `parse_bash_permission`, `parse_write_preview`, `parse_plan`, `parse_questions` (from Claude-shaped fixtures); `parse_push_errors`; `reply_errors_are_cmux_texts`; `decision_json_round_trips` (every variant); `push_params_shape`; and one `hook_output_*` test per rf §3.1 row (Once, Always with `permission_suggestions` → `updatedPermissions` verbatim, Always without suggestions → no key, Deny message, Plan Manual `updatedInput` = `tool_input`, Plan Auto `setMode auto session`, Plan Ultraplan message, Plan feedback message `User rejected the plan via cmux Feed and wants this change: <f>`, Plan deny, Question with two questions incl. a multi-select already joined `"A, B"`).
- [ ] **Step 2:** `cargo test -p cmux-ipc --locked --lib feed` fails.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Pass; clippy.
- [ ] **Step 5:** Commit `m7: feed wire format in cmux-ipc`.

### Task 3: Long waits on the socket (D4)

**Files:** Modify `crates/cmux-ipc/src/server.rs`; test `crates/cmux-ipc/tests/server.rs`.

**Interfaces — Consumes:** `feed::wait_of`. **Produces:** `pub const FEED_GRACE: Duration = Duration::from_secs(5);` and `handle` routing `feed.push` to `ask_app(…, wait + FEED_GRACE)`.

- [ ] **Step 1: Failing tests** (server started with `deadline` 200 ms, an app thread that answers every request after 600 ms): `feed_push_outlives_the_app_deadline` (`feed.push` with `wait_timeout_seconds: 1` gets the app's result); `other_methods_keep_the_deadline` (`workspace.list` → `timeout`); `feed_push_wait_out_of_range_is_rejected_by_the_server` (121 → `invalid_params` exact text, app never sees it).
- [ ] **Step 2:** Run → fail. **Step 3:** Implement. **Step 4:** Pass. **Step 5:** Commit `m7: feed.push waits its own timeout on the socket`.

### Task 4: `cmux hooks feed --source claude` (D1, D3)

**Files:** Create `crates/cmux-ipc/src/cli/hooks.rs`; modify `cli/mod.rs`, `cli/client.rs` (only if a per-call timeout is needed), `cli/help.rs`, `env.rs`, `tests/support/mod.rs`; create `tests/cli_hooks.rs`.

**Interfaces — Consumes:** Task 2. **Produces:**
- `Ctx` gains `pub stdin: Option<String>` (`None`: read the process's stdin when a command needs it) and `pub exe: Option<PathBuf>` (`from_process` sets `std::env::current_exe().ok()`). `support::T` gets `run_with_stdin(&self, args, stdin: &str)`.
- `terminal_env` adds `("CMUX_LINUX", "1")`.
- Help: a `hooks` entry in the `Automation` group; usage `hooks <setup|uninstall> [--settings-file <path>] | hooks feed --source claude`; help text lists the three forms, one line each.

Behaviour: no `--source` → `Error: cmux hooks feed requires --source <agent-name>`, exit 1. Source not `claude` → `{}`. Everything else ends in exit 0 with one stdout line: the `hook_output` JSON when the result is `{"status":"resolved"}` with a parseable decision, else `{}`. `{}` without contacting the socket when: stdin isn't a JSON object, `CMUX_LINUX` isn't truthy, or `CMUX_WORKSPACE_ID`/`CMUX_SURFACE_ID` isn't a UUID. Request id: a fresh upper-case UUID. Socket read timeout 118 s (ignores `CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC`). Any socket or server error → `{}`.

- [ ] **Step 1: Failing tests** in `tests/cli_hooks.rs` (harness `T` with canned `feed.push` replies; env with `CMUX_LINUX=1` and the fixture's workspace/surface UUIDs): `feed_hook_allow_once` (result resolved/once → parsed stdout == `{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}`, and `t.last()` is `feed.push` whose params match `push_params` with `wait_timeout_seconds` 115); `feed_hook_plan_auto`; `feed_hook_timed_out_prints_empty`; `feed_hook_outside_cmux_linux_prints_empty` (no `CMUX_LINUX`: `{}`, no request recorded); `feed_hook_without_socket_prints_empty` (`CMUX_SOCKET_PATH` to a missing file: `{}`, exit 0); `feed_hook_bad_stdin_prints_empty`; `feed_hook_requires_source`; plus `env.rs`'s test expects `CMUX_LINUX=1`.
- [ ] **Step 2:** Run → fail. **Step 3:** Implement. **Step 4:** `cargo test -p cmux-ipc --locked` all pass. **Step 5:** Commit `m7: cmux hooks feed`.

### Task 5: `cmux hooks setup` / `uninstall` (D2)

**Files:** Modify `crates/cmux-ipc/src/cli/hooks.rs`; test `tests/cli_hooks.rs`.

**Interfaces — Produces** (pure, in `hooks.rs`, `pub(crate)`):
```rust
fn hook_command(exe: &Path) -> String;                     // D2 form, POSIX single-quoted path ('\'' for ')
fn install(settings: &mut Value, command: &str) -> Result<(), String>;   // remove ours, then append ours
fn uninstall(settings: &mut Value) -> Result<(), String>;
```
`remove` drops each hook whose `command` contains `hooks feed --source claude`, then any group left without hooks, any event left empty, and `hooks` if left empty; nothing else changes. Non-object settings → `<path> is not a JSON object`; `hooks` present but not an object → `hooks in <path> is not an object` (exit 1, file untouched). Empty or whitespace file = `{}`.

Files: path per D2 (`--settings-file` beats `CLAUDE_CONFIG_DIR` beats `HOME`). Follow a symlink to its target. Write only when the value changed: `to_string_pretty` + `\n` to a temp file in the target's directory, keep the old mode (new file `0600`, missing dir created `0700`), `sync_all`, rename over. Uninstall with no file creates nothing. Needs `ctx.exe` (absolute); `None` → `Error: cannot find the cmux binary`.

Messages (stdout, exit 0; rf §2.2 with "Feed hooks"): `cmux: installed Feed hooks in <p>` + `cmux: restart running Claude Code sessions to load them`; `cmux: Feed hooks are already installed in <p>`; `cmux: removed Feed hooks from <p>`; `cmux: no Feed hooks installed in <p>`.

- [ ] **Step 1: Failing tests:** unit tests for `hook_command` (a path with a space and a `'`), `install`/`uninstall` on `Value`s; CLI tests on temp files: `install_creates_file_with_0600` (missing file and dir), `install_keeps_user_hooks_and_is_idempotent` (user `Stop` hook and a user `PermissionRequest` hook in the same event survive; second run prints "already installed" and leaves mtime/bytes unchanged), `install_replaces_an_old_path` (entry with another exe → one entry with the new one), `uninstall_restores_original` (install then uninstall → parses equal to the original), `uninstall_without_file_creates_nothing`, `install_follows_symlink_and_keeps_mode` (symlink stays a symlink; target mode `0640` kept), `refuses_non_object` (array file and `"hooks": []` → exit 1, bytes unchanged), `claude_config_dir_is_used` (env in `Ctx`).
- [ ] **Step 2:** Run → fail. **Step 3:** Implement. **Step 4:** Pass. **Step 5:** Commit `m7: cmux hooks setup and uninstall`.

### Task 6: App holds feed requests (D5, D6)

**Files:** Create `crates/cmux-app/src/window_view/feed_ctl.rs`; modify `window_view.rs` (fields `feed: FeedStore`, `feed_waiters: HashMap<String, Waiter>`; request loop; workspace close), `control.rs` (reply methods).

**Interfaces — Consumes:** Tasks 1–2, `Request` (owned), `on_notify`, the notification removal M4 uses for `notification.dismiss`. **Produces:**
```rust
struct Waiter { req: Request, notification: Option<NotificationId> }
impl WindowView {
    pub(super) fn feed_push(&mut self, req: Request, window: &mut Window, cx: &mut Context<Self>);
    pub(super) fn feed_finish(&mut self, request_id: &str, decision: Option<Decision>, cx: &mut Context<Self>);
    pub(super) fn feed_answer(&mut self, id: FeedItemId, decision: Decision, cx: &mut Context<Self>); // UI buttons
}
```
- Request loop: `feed.push` → `feed_push(req, …)` (ownership moves); everything else unchanged.
- `feed_push`: `check_window`; `parse_push` error → respond it. Target: `snap.surface` with the event's `workspace_id` + `surface_id` (both must match one surface) → else `Ok({"status":"not_found"})`. `wait == 0` → `Ok({"status":"acknowledged"})`, no card. Duplicate request id → `Ok({"status":"unavailable"})`. Otherwise push the item (`created_ms` = now), post the D6 notification, store the `Waiter`, spawn a timer for `wait` that calls `feed_finish(id, None)`, `cx.notify()`.
- `feed_finish`: waiter gone → nothing. `Some(d)` → `resolve`, respond `{"status":"resolved","decision":decision_json(d)}`; `None` → `expire`, respond `{"status":"timed_out"}`. Remove the D6 notification either way.
- Reply methods in `control`: `parse_reply` → `feed_finish(id, Some(d))` → `Ok({"delivered":true})` even for unknown ids.
- Workspace closed: `expire_workspace`, and each waiter gets `{"status":"unavailable"}`. A closed surface with a pending card: same, for that surface's items (`expire_surface` is not in Task 1; filter `items()` by surface and call `expire`).

- [ ] **Step 1:** No gpui tests exist in this crate; the logic lives in Tasks 1–2. Write the code, then verify with a release build and the Task 9 smoke run.
- [ ] **Step 2:** `cargo build --release --features gpui/runtime_shaders --locked`; clippy; full `cargo test --locked` for touched crates.
- [ ] **Step 3:** Commit `m7: app holds feed.push until answered`.

### Task 7: Right sidebar, permission card, bindings (D7, D9)

**Files:** Create `crates/cmux-app/src/window_view/feed.rs`; modify `window_view.rs` (layout, `right_sidebar_open`, focus handle), `workspace.rs` (`right_inset`, width = `size.width - left_inset - right_inset`), `actions.rs`, `crates/cmux-config/src/keybindings.rs` (three actions; `switchRightSidebarToFeed` bound with context `RightSidebar`), `assets` only if an icon is missing (Lucide, already in `NOTICE`).

**Interfaces — Consumes:** Task 6 `feed_answer`, `FeedStore::items`, `age_label`, `Decision::summary`. **Produces:** `WindowView::toggle_right_sidebar`, `focus_right_sidebar` (opens and focuses the first item), gpui actions `ToggleRightSidebar`, `FocusRightSidebar`, `SwitchRightSidebarToFeed`.

Visuals (rf §5): header `Feed` + badge (pending count, `9+` cap, 10 pt bold mono, orange on orange 20 %, pad 5×1); empty state `No pending decisions` / `Permission, plan, and question requests from AI agents will appear here.` (12 / 11 pt, secondary 0.7, centred, h-pad 16); rows pad 12×10, spacing 10, 1 px dividers primary 0.08, non-pending rows opacity 0.6; header row: kind icon 12 pt (permission orange), title 12 pt medium primary 0.92 truncated, chips `Claude` (fg rgb(0.92,0.54,0.29) on 18 %) and `age_label`, 10 pt medium, pad 5×2, radius 4; permission body: warning icon + tool name 11 pt semibold orange, code box pad 10, radius 6, fill primary 0.05, stroke primary 0.10, mono 11 pt (Bash `$ <command>` + description 10 pt secondary; file path + first line); buttons one row, spacing 6, equal width, 10.5 pt semibold, pad 12×5 (compact 8×4 if they don't fit 295), radius 6: `Deny` (white on black 0.75), `Allow Once` (black on white 0.88), `Always Allow` and `All tools` (accent); answered → dimmed green `✓ Submitted · <summary>`. Colours from the existing palette where it has them.

- [ ] **Step 1: Failing tests** in `keybindings.rs`: defaults include the three actions with cmux's chords; `ctrl+4` for `switchRightSidebarToFeed` loads with context `RightSidebar` while `selectSurfaceByNumber` keeps `ctrl-4`; `actions.rs` `every_config_action_has_a_gpui_action` passes for them.
- [ ] **Step 2:** Run → fail. **Step 3:** Implement sidebar, permission card, keys (D9). **Step 4:** Tests, clippy, release build. **Step 5:** Commit `m7: right sidebar Feed with permission cards`.

### Task 8: Plan and question cards (D8)

**Files:** Modify `crates/cmux-app/src/window_view/feed.rs`; reuse `rename.rs`'s key-to-text state for the one-line feedback field (extract a shared `TextField` state only if both need the same thing; otherwise call `RenameState` as is).

**Interfaces — Consumes:** Task 7 card frame; `Decision::ExitPlan`, `Decision::Question`.

Plan card (rf §5): icon purple; body plan text 11 pt (plain, scrollable to 240 px), `Allowed prompts` list, `Plan file` path; feedback field placeholder `Tell Claude what to change…` (12 pt, pad 10, radius 6); buttons `Send feedback` (accent, disabled without text → `ExitPlan{Manual, Some(text)}`), `Manual` (soft → `ExitPlan{Manual, None}`), `Auto` (green → `ExitPlan{AutoAccept, None}`). Question card: icon blue; per question its header, text, options (label + description; multi-select tagged `Multi-select`, toggles; single-select picks one); `Submit All Answers` (accent) enabled when each question has ≥ 1 pick → `Question{selections}` with multi-select labels joined `", "`.

- [ ] **Step 1:** Add a pure helper test in `feed.rs` (no gpui): `selections_join_multi_select` and `submit_needs_every_question` on the card's selection state struct.
- [ ] **Step 2–4:** Fail, implement, pass; release build; clippy.
- [ ] **Step 5:** Commit `m7: plan and question cards`.

### Task 9: End to end, docs

- [ ] **Step 1: Mac smoke run (controller, our app only).** Release build from `target/release` (never the installed copy). In one of its terminals, in a second workspace: `cmux hooks feed --source claude < fixture.json` with fixtures for Bash, ExitPlanMode and AskUserQuestion (saved under the scratchpad, not the repo). Check: card appears while the first workspace is selected, ring + notification on the second; each button prints the expected JSON in that terminal; a `wait` timeout prints `{}` and dims the card; closing the workspace mid-wait ends the hook at once; `ctrl+4` in a terminal still selects tab 4. Screenshots to `~/Pictures/cmux-m7/`.
- [ ] **Step 2: Real Claude Code (owner, or controller with the owner's OK — it edits `~/.claude/settings.json` and spends Claude usage):** `cmux hooks setup` (twice: second says already installed); run `claude` in a background workspace, trigger a Bash permission, answer from Feed (exit 1); ask for a plan in plan mode, approve with `Manual` (exit 2); `cmux hooks uninstall` and diff the settings file against a copy taken before (exit 3).
- [ ] **Step 3:** README "Feed" section (setup, uninstall, what each button sends, the 115 s fallback, Mac + cmux.app note), journal entry, STATUS. Commit `m7: docs`.
- [ ] **Step 4:** PR and merge only with the owner's OK, after M6 merges (rebase onto `master`).
