# M5 cli-socket Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scripts and agent hooks drive cmux-linux the way they drive cmux. The app serves cmux's control socket protocol, every terminal carries cmux's `CMUX_*` variables and finds our `cmux` CLI first on `PATH`, and the CLI's text and JSON output for the ported commands matches cmux's contract.

**Architecture:**
- `cmux-ipc` (library plus the `cmux` binary) holds everything that needs no window: the wire format, handle minting, JSON printing and `--id-format`, a `Snapshot` of the window model with the read-only methods and target resolution, the socket server (one thread per connection, requests handed to the app over a channel), socket paths, key-name parsing, and the CLI. The CLI is a library function `cli::run`, so golden tests run it in-process against the real server.
- `cmux-core` gets a split at a given surface (direction, optional focus), a notification subtitle, and "remove read".
- `cmux-app` starts the server in `WindowView::new`, sets the env vars at every shell spawn, and answers requests on the foreground executor: it builds a `Snapshot` per request for reads and resolution, then mutates the model the same way the keyboard actions do.

**Tech Stack:**
- Rust 2024 (toolchain per `rust-toolchain.toml`), gpui `=0.2.2`.
- New `cmux-ipc` dependencies, all already in `Cargo.lock` with these features enabled: `serde_json` 1.0.154 (workspace dep; `preserve_order` is on through the tree, so key sorting is our job), `uuid` 1.29.0 (`v4` on), `libc` 0.2.189, `futures` 0.3. No new packages.

**Spec:** `docs/milestones/m5-cli-socket/spec.md`. Research: `research-cmux.md` in the same folder (cited `rc §n`; help texts in rc §9). Criterion 1 ("documented Claude Code hook setup") means Claude Code hooks in `~/.claude/settings.json` that call `cmux notify` (owner's ruling, 2026-11-10). `cmux claude-hook`/`hooks setup` stay out of scope.

## Global Constraints

Carried from M0–M4:
- Licence GPL-3.0-or-later. Every added dependency, vendored file or asset goes in `NOTICE`. Never copy from cmux's BUSL-1.1 directories (rc §0).
- Zig exactly 0.16.0; `export PATH="$PWD/.tools/zig:$PATH"` from the repo root before any cargo command.
- `--locked` on build, test and clippy. Exception: Task 1 Step 1 may change `Cargo.lock`, and then only the dependency lists of `cmux-ipc` and `cmux-app` (no package added or moved). libc stays 0.2.189 (`grep -A1 'name = "libc"' Cargo.lock`). Never a bare `cargo update`.
- gpui pinned `=0.2.2`; toolchain per `rust-toolchain.toml`.
- No personal info in tracked files. `STATUS.md` max 100 lines, archived before replacement.
- Before each commit: `cargo fmt -p <crates touched>` and `cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings`.
- Dependency rules: `cmux-core` depends on nothing; `cmux-ipc` never depends on gpui; Linux-only code sits behind `cfg(target_os = "linux")`.
- macOS host builds use `--features gpui/runtime_shaders`. Linux CI is the gate: push and `gh run watch` before ticking a task with `cfg(linux)` code (Tasks 3, 9).

New for M5:
- Branch `m5-cli-socket` from `master`; merges at Task 12. One commit per step that changes files; messages end with a blank line and the session's attribution lines.
- This plan's code was not compiled. Signatures in **Interfaces** blocks are what later tasks consume; any change goes in the journal.
- Where behaviour is unclear, the cmux file that rc §n cites wins. Record deviations in the journal.
- Screenshots go to `~/Pictures/cmux-m5/`, outside the repo.

## Review Focus

1. **A hook that runs from a process the app didn't start** (a daemonized helper, a tmux server started before the app). cmuxOnly denies it with one `ERROR: Access denied …` line; the CLI prints it and exits 1, never hangs. Tests: Task 3 `descendant_walk`, `non_descendant_is_denied`.
2. **The app's main thread is busy or the window is gone while a hook waits.** The request fails with `timeout` after the deadline, or `internal_error` if the app dropped it; the CLI exits 1 within its 15 s. Tests: Task 3 `unanswered_request_times_out`, `dropped_request_is_internal_error`.
3. **A stale socket after a crash, or a second app instance.** The first replaces the dead file; the second must not steal a live socket and falls back to its own `cmux-<pid>.sock`, and its terminals point at their own app. Tests: Task 3 `stale_socket_is_replaced`, `live_socket_is_not_stolen`; Task 9 fallback (Run it).
4. **Hostile input on the socket**: a multi-megabyte line, invalid UTF-8, a client that never sends. Memory stays bounded, the reply is the cmux error envelope or a close, and other clients are served. Tests: Task 1 `parse_errors_exact`; Task 3 `oversized_line_closes_connection`, `other_clients_still_served`.
5. **A hook firing after its pane or workspace closed** (env ids now unknown). `notify` returns `not_found: Workspace not found` and exits 1; nothing is posted to another pane. Tests: Task 2 `caller_with_gone_ids_is_not_found`; Task 7 `notify_not_found_exits_1`.

## Decisions made in this plan

| Decision | Why |
|---|---|
| **D1 Socket path.** App: `$XDG_STATE_HOME/cmux-linux/cmux.sock`, else `~/.local/state/cmux-linux/cmux.sock`. `CMUX_SOCKET_PATH` is honoured only when `CMUX_ALLOW_SOCKET_OVERRIDE` is `1/true/yes/on`. A live server already on the path → `<same dir>/cmux-<pid>.sock`. Dir `0700`, socket `0600` (`0666` for allowAll). Unlinked when `Server` drops, if the inode is still ours. CLI: `--socket`, then `CMUX_SOCKET_PATH`/`CMUX_SOCKET` (conflict check, rc §1), then the same default. No marker-file discovery. | A different file from real cmux, which may run on the same Mac. Terminals always get the right path through `CMUX_SOCKET_PATH`; the override rule stops an app started inside a terminal from grabbing its parent's socket (rc §1). |
| **D2 Wire.** As rc §2: newline-delimited, v2 if the trimmed line starts with `{`. The server itself answers v1 `ping` (`PONG`), v1 `auth`, v2 `system.ping` and `auth.login`; other v2 methods go to the app. Lines over 4 MiB close the connection. Read timeout 30 s, write 5 s. Request deadline 10 s → `timeout`, `data {"retryable":true,"deadline_ms":<ms>,"stage":"main_actor"}`, message `Timed out waiting for the app` (cmux's text UNVERIFIED). | rc §2. The app needs no v1 commands: the CLI only uses v1 for `ping` and `auth`. |
| **D3 Access.** `CMUX_SOCKET_MODE` at app start: `cmuxOnly` (default; aliases rc §3), `password`, `allowAll`/`openaccess`/`fullopenaccess`/`full`, `off` (no server). cmuxOnly on Linux: same uid and peer pid descends from the app (`SO_PEERCRED`, then `/proc/<pid>/stat` field 4, ≤128 levels). On macOS: same uid only (`getpeereid`; `shortcut:` comment, M6). password: same uid; password = first line of `<state dir>/socket-control-password` (trimmed); none → every `auth` gets `ERROR: Password mode is enabled but no socket password is configured.` CLI password order: `--password`, `CMUX_SOCKET_PASSWORD`, that file. | rc §3. No capability tokens, no Keychain, no Settings UI (M6). |
| **D4 Handles.** UUID v4, uppercase. Refs `<kind>:<n>` per kind from 1, minted on first sight, never reused. Keys `(workspace, 0)`, `(workspace, pane)`, `(workspace, surface)` (Pane and Surface ids restart per workspace); window `(0, 0)`. Notifications get a UUID only (text handle `notification:<UUID>`). Resolution scans the live `Snapshot` (UUID case-insensitive, exact ref, `tab:N` = `surface:N`), so closed objects never resolve. | rc §6. No 1 000 000 000 floor: refs reset per run until persistence (M10). |
| **D5 Ported commands.** `ping`, `version`, `help`/`--help`, `list-workspaces`, `current-workspace`, `list-panes`, `list-pane-surfaces`, `select-workspace`, `focus-pane`, `focus-panel`, `new-workspace`, `new-split`, `send`, `send-key`, `notify`, `dismiss-notification`. v2 methods: `system.ping`, `auth.login`, `window.list`, `workspace.list/current/select/create`, `pane.list/surfaces/focus`, `surface.list/focus/split/send_text/send_key`, `terminal.paste`, `notification.create/create_for_target/create_for_caller/clear/dismiss`. Anything else: CLI exit 2 unknown command; server `method_not_found`. | Spec scope. `identify`, `rpc`, `list-notifications`, `clear-notifications` deferred. |
| **D6 Focus policy.** `workspace.create` and `surface.split` change selection and focus only with `focus:true`; `workspace.select`, `pane.focus`, `surface.focus` always do (no window raise: KWin refuses a background app). New workspaces go after the selected one. `left`/`up` put the new pane first. | rc §8. |
| **D7 Notifications over the socket.** Store gains `subtitle`. Routing per rc §8 (`create`, `create_for_target`, `create_for_caller`). **Deviation:** a workspace-level target (no surface) lands on that workspace's active surface, and no TTY matching (`caller_tty` not sent). `effects.desktop:false` posts no desktop bubble. `--reply` is accepted and ignored. Desktop bubble body = `subtitle\nbody` (either may be empty, trimmed). Sidebar summary and panel body line = body, else subtitle, else title (`Sources/TabItemView+WorkspaceNotificationsMenu.swift:31`). | Our store keys by (workspace, surface); hooks always carry `CMUX_SURFACE_ID`. |
| **D8 Terminal env.** `CMUX_WORKSPACE_ID`, `CMUX_TAB_ID` = workspace UUID; `CMUX_SURFACE_ID`, `CMUX_PANEL_ID` = surface UUID; `CMUX_SOCKET_PATH` = our socket (omitted if no server); `CMUX_SOCKET=""`; `PATH` prefixed with the dir of a `cmux` file next to the app's executable, when one exists. `CMUX_SOCKET_PASSWORD` is read by the CLI but never set in terminals. | rc §5. The prefix makes `command -v cmux` find our CLI, not real cmux's. cmux doesn't set the password either (rc §5). |
| **D9 Output.** JSON: Foundation style, 2-space indent, `"key" : value`, sorted keys, `/` unescaped; empty array/object print `[]`/`{}` (Foundation's form UNVERIFIED). `--id-format` per rc §6 (with the `--json` default-keep rule). Text summaries per rc §8. No deprecation notice for `list-workspaces`/`new-workspace`/`select-workspace`: it points at `cmux workspace …`, which we don't have. Version: `cmux <CARGO_PKG_VERSION>`. | Scripts parse the text; tests compare JSON as values plus one exact pretty-print test. |
| **D10 Help.** rc §9 texts verbatim, except lines marked `‡`: drop flags we don't port (`--reply` stays, it is accepted), `native macOS banner` → `desktop notification`, the `list-notifications`/`clear-notifications`/hooks sentences and `--force` draft/dialog sentences dropped, `the password saved in Settings` → `the socket-control-password file`, default socket line → D1's path, no `cmux <path>` line, no `tab-action`/`rename-tab` mentions. Top-level groups: Start & Resume, Navigate & Arrange, Inspect, Automation, Diagnostics / Advanced (rc §9). | Help must not advertise what isn't there. |
| **D11 Not ported** (accepted where harmless): `surface.input_state` draft guard (`--force` is a no-op), `--description/--env/--env-file/--layout/--group*` (unknown flag), global `--window` calling `window.focus` (one window), remote/relay, `CMUX_QUIET`. | Out of spec or no counterpart yet. |

## Deferred (with reason)
- `identify`, `capabilities`, `rpc`, `list-notifications`, `clear-notifications`, `set-status`: not in the spec; M6 parity pass.
- Settings UI and a config key for socket mode/password; capability tokens; macOS ancestry check: M6.
- Persistent refs across restarts (cmux's 10⁹ floor): M10.
- `claude-hook`, `hooks setup`, Feed, agent session tracking: M7/M9.

## File structure

| File | Responsibility |
|---|---|
| `crates/cmux-ipc/Cargo.toml` | deps; `[[bin]] name = "cmux"` |
| `crates/cmux-ipc/src/wire.rs` | request/response lines, parse errors (T1) |
| `crates/cmux-ipc/src/handles.rs` | UUID + ref minting (T1) |
| `crates/cmux-ipc/src/json.rs` | Foundation pretty print, `IdFormat`, `format_ids`, text handles (T1) |
| `crates/cmux-ipc/src/snapshot.rs` | model snapshot, read methods, target resolution (T2) |
| `crates/cmux-ipc/src/paths.rs`, `server.rs` | socket paths, server, access (T3) |
| `crates/cmux-ipc/src/cli/{mod,args,client,help,commands,notify}.rs`, `src/bin/cmux.rs` | the CLI (T4–T7) |
| `crates/cmux-ipc/src/keys.rs`, `env.rs` | key names, paste/typing bytes (T10); terminal env (T9) |
| `crates/cmux-ipc/tests/support/mod.rs`, `tests/{server,cli_*}.rs` | fake app on the real server, goldens |
| `crates/cmux-core/src/{tree,geometry,notifications}.rs` | split at surface, subtitle, remove read (T8) |
| `crates/cmux-app/src/window_view/control.rs` | request dispatch (T9–T11) |
| `crates/cmux-app/src/{window_view,workspace,terminal_view}.rs`, `window_view/{notify,panel}.rs`, `desktop_notify.rs` | server start, env at spawn, subtitle, desktop flag |

---

### Task 1: Wire, handles, JSON output (cmux-ipc, pure)

**Files:** Create `crates/cmux-ipc/src/{wire,handles,json}.rs`; modify `crates/cmux-ipc/{Cargo.toml,src/lib.rs}`, `Cargo.toml` (workspace dep `uuid = { version = "1", features = ["v4"] }`), `crates/cmux-app/Cargo.toml` (`cmux-ipc.workspace = true`, added now so the lock changes once).

**Interfaces — Produces:**
- `wire`: `pub struct RpcError { pub code: String, pub message: String, pub data: Option<Value> }` with `RpcError::new(code: &str, message: impl Into<String>) -> Self` and `fn with_data(self, Value) -> Self`; `pub type Reply = Result<Value, RpcError>;` `pub enum Line { V1 { cmd: String, args: String }, V2 { id: Value, method: String, params: Map<String, Value> }, Bad(String) }` (`Bad` holds the full error response line); `pub fn parse_line(line: &[u8]) -> Line`; `pub fn response(id: &Value, reply: &Reply) -> String`; `pub fn request_line(id: &str, method: &str, params: &Map<String, Value>) -> String`; `pub fn parse_response(line: &str, id: &str) -> Result<Reply, String>`.
- `handles`: `pub enum Kind { Window, Workspace, Pane, Surface }`; `pub type Key = (u64, u64);` `#[derive(Clone, Debug, PartialEq)] pub struct Handle { pub id: String, pub r#ref: String }`; `pub struct Handles` with `new()`, `handle(&mut self, Kind, Key) -> Handle`, `notification(&mut self, n: u64) -> String`.
- `json`: `pub fn pretty(v: &Value) -> String`; `#[derive(Clone, Copy, PartialEq, Debug)] pub enum IdFormat { Refs, Uuids, Both }` with `parse(&str) -> Option<Self>`; `pub fn format_ids(v: &mut Value, f: IdFormat, keep: &[&str])`; `pub fn text_handle(id: Option<&str>, r: Option<&str>, f: IdFormat) -> String` (both → `ref (id)`); `pub fn row_handle(…same…) -> String` (both → `ref id`).

- [ ] **Step 1: Add the dependencies.** `cmux-ipc`: `serde_json`, `uuid`, `libc`, `futures` (workspace). Run `cargo build -p cmux-ipc` (no `--locked`), then `git diff Cargo.lock` must show only the `cmux-ipc`/`cmux-app` dependency arrays; libc still 0.2.189. Commit.
- [ ] **Step 2: Write failing tests** (`#[cfg(test)]` in each file):
  - `v1_and_v2_are_told_apart`: `parse_line(b"  ping\n")` → `V1{cmd:"ping",args:""}`; `parse_line(b"AUTH pw")` → `V1{cmd:"auth",args:"pw"}`; `parse_line(br#" {"method":"m","id":3}"#)` → `V2{id:3, method:"m", params:{}}`.
  - `parse_errors_exact`: `[0xff,0x7b]`→ `{"id":null,"ok":false,"error":{"code":"invalid_utf8","message":"Invalid UTF-8"}}`; `{bad` → `…"parse_error","message":"Invalid JSON"…`; `[1]` → `…"invalid_request","message":"Expected JSON object"…`; `{"id":1}` → `{"id":1,"ok":false,"error":{"code":"invalid_request","message":"Missing method"}}`.
  - `response_lines_exact`: `response(&json!("a"), &Ok(json!({"pong":true})))` == `{"id":"a","ok":true,"result":{"pong":true}}`; an error with data → `{"id":"a","ok":false,"error":{"code":"not_found","message":"Surface not found","data":{"surface_id":"X"}}}`; without data no `data` key.
  - `request_line_puts_method_first`: `request_line("I","surface.split",{focus:false,direction:"right"})` == `{"method":"surface.split","id":"I","params":{"direction":"right","focus":false}}`.
  - `parse_response_checks_id`: wrong id → `Err("Mismatched v2 response id")`; `garbage` → `Err("Invalid v2 response: garbage")`; an error envelope → `Ok(Err(RpcError{code:"not_found",..}))`.
  - `refs_count_per_kind_from_1`: workspaces A, B → `workspace:1`, `workspace:2`; first pane → `pane:1`; same key twice → equal handles; ids match `^[0-9A-F]{8}-[0-9A-F]{4}-4[0-9A-F]{3}-[89AB][0-9A-F]{3}-[0-9A-F]{12}$`; `notification(5)` stable across calls.
  - `pretty_matches_foundation`: `pretty(&json!({"b":[1,{"a":"x/y"}],"a":null,"e":[]}))` == `"{\n  \"a\" : null,\n  \"b\" : [\n    1,\n    {\n      \"a\" : \"x/y\"\n    }\n  ],\n  \"e\" : []\n}"`.
  - `format_ids_matrix`: on `{"id":"U","ref":"pane:1","surface_ids":["S"],"surface_refs":["surface:1"],"workspace_id":"W","workspace_ref":null}` — Refs → `{"ref","surface_refs","workspace_ref"}` only; Uuids → `{"id","surface_ids","workspace_id"}`; Both → unchanged; Refs with `keep=["pane"]` keeps `id`. Recurses into arrays and nested objects.
  - `text_handles`: refs `workspace:2`, uuids `U`, both `workspace:2 (U)`; row both `workspace:2 U`; missing ref falls back to id.
- [ ] **Step 3: Run** `cargo test -p cmux-ipc --locked` → the tests fail (not defined).
- [ ] **Step 4: Implement.** `format_ids` keep rule: under Refs, an `id` survives when the sibling `ref`'s kind (text before `:`) is in `keep`; an `X_id`/`X_ids` survives when `X` is in `keep`. `pretty` writes strings with `serde_json::to_string` then un-escapes `\/` (serde never escapes `/`, so plain `to_string` per string).
- [ ] **Step 5: Run** the tests → pass. fmt, clippy. Commit.

### Task 2: Snapshot, reads and target resolution (cmux-ipc, pure)

**Files:** Create `crates/cmux-ipc/src/snapshot.rs`.

**Interfaces — Consumes:** T1 `Handle`, `Reply`, `RpcError`. **Produces:**
```rust
pub struct Snapshot { pub window: Handle, pub selected: usize, pub workspaces: Vec<WsSnap> }
pub struct WsSnap { pub key: u64, pub handle: Handle, pub title: String, pub custom_title: Option<String>,
    pub cwd: Option<String>, pub ports: Vec<u16>, pub focused_pane: usize, pub panes: Vec<PaneSnap> }
pub struct PaneSnap { pub key: u64, pub handle: Handle, pub selected: usize, pub surfaces: Vec<SurfSnap> }
pub struct SurfSnap { pub key: u64, pub handle: Handle, pub title: String }
pub struct Target<'a> { pub ws: &'a WsSnap, pub pane: &'a PaneSnap, pub surface: &'a SurfSnap }
impl Snapshot {
    pub fn read(&self, method: &str, params: &Map<String, Value>) -> Option<Reply>; // window.list, workspace.list, workspace.current, pane.list, pane.surfaces, surface.list
    pub fn check_window(&self, params: &Map<String, Value>) -> Result<(), RpcError>; // window_id absent or ours, else not_found "Window not found"
    pub fn workspace(&self, params: &Map<String, Value>) -> Result<&WsSnap, RpcError>; // workspace_id, else selected
    pub fn pane<'a>(&self, ws: &'a WsSnap, params: &Map<String, Value>) -> Result<&'a PaneSnap, RpcError>; // pane_id, else focused
    pub fn surface(&self, params: &Map<String, Value>) -> Result<Target<'_>, RpcError>; // surface_id anywhere, else the workspace's focused
    pub fn caller(&self, params: &Map<String, Value>) -> Result<(&WsSnap, Option<Target<'_>>), RpcError>;
    pub fn target_payload(&self, t: &Target) -> Value; // window_/workspace_/pane_/surface_ id+ref
}
```
Payload shapes are rc §8 restricted to these fields — workspace row: `id, ref, index, title, custom_title, has_custom_title, selected, pinned:false, listening_ports, current_directory`; pane row: `id, ref, index, focused, surface_ids, surface_refs, selected_surface_id, selected_surface_ref, surface_count`; surface row: `id, ref, index, title, type:"terminal", selected` (`index` within the pane for `pane.surfaces`, within the workspace for `surface.list`); `window.list` → `{windows:[{id, ref, index:0, selected:true, workspace_count}]}`.

- [ ] **Step 1: Write failing tests** with a fixture: workspace A (`workspace:1`, title `Terminal`, selected, panes P1 {S1 `zsh` selected, S2 `vim`} and P2 {S3}, focused P2) and workspace B (`workspace:2`, custom title `Build`).
  - `workspace_list_payload`: `read("workspace.list",{})` equals the full expected `json!` (both rows, `window_id`, `window_ref`).
  - `pane_list_and_surfaces_payloads`, `surface_list_indexes_span_panes` (S3 has index 2), `workspace_current_payload`, `window_list_payload`.
  - `handles_resolve`: `workspace_id` given as the lowercase UUID, as `workspace:2`; `surface_id: "tab:3"` resolves S3; `workspace:9` → `not_found`/`Workspace not found`; `surface:9` → `not_found`/`Surface not found` with `data {"surface_id":"surface:9"}`; `pane:9` → `not_found`/`Pane not found`; for `pane.surfaces` the error is `Pane or workspace not found`.
  - `surface_defaults_to_focused`: `surface({})` → A/P2/S3; `surface({workspace_id:B})` → B's focused surface.
  - `caller_rules` (rc §8 create_for_caller): preferred workspace A + preferred surface S1 → (A, S1); preferred workspace B + S1 (not in B) → (B, None); preferred surface S1 only → (A, S1); none → (selected, None).
  - `caller_with_gone_ids_is_not_found`: preferred ids that match nothing → `not_found`/`Workspace not found`.
  - `unknown_window_is_not_found`: `check_window({window_id:"window:2"})` → `not_found`/`Window not found`.
- [ ] **Step 2: Run** → fail. **Step 3: Implement.** **Step 4: Run** → pass. fmt, clippy. Commit.

### Task 3: Socket paths and server (cmux-ipc)

**Files:** Create `crates/cmux-ipc/src/{paths,server}.rs`, `crates/cmux-ipc/tests/server.rs`.

**Interfaces — Consumes:** T1 `wire`. **Produces:**
```rust
// paths.rs — `env` returns a variable's value
pub fn state_dir(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf>;   // $XDG_STATE_HOME/cmux-linux | $HOME/.local/state/cmux-linux
pub fn default_socket(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf>;
pub fn app_socket(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf>;   // D1 override rule
pub fn password_file(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf>;
// server.rs
pub enum Access { CmuxOnly { root: u32 }, Password(Option<String>), AllowAll }
impl Access { pub fn from_env(mode: Option<&str>, password_file: Option<&Path>) -> Option<Access> } // None = off
pub struct Request { pub method: String, pub params: Map<String, Value>, /* reply sender */ }
impl Request { pub fn respond(self, reply: Reply) }
pub struct Server { /* path, inode, listener thread */ }
impl Server {
    pub fn start(path: &Path, access: Access, deadline: Duration)
        -> io::Result<(Server, futures::channel::mpsc::UnboundedReceiver<Request>)>; // ErrorKind::AddrInUse when live
    pub fn path(&self) -> &Path;
}
pub fn is_descendant(pid: u32, root: u32, parent_of: impl Fn(u32) -> Option<u32>) -> bool;
pub const DEADLINE: Duration = Duration::from_secs(10);
```

- [ ] **Step 1: Write failing tests.** Unit: `descendant_walk` (table 50→40→30→1: 50 under 30 true, under 99 false, a loop terminates, 128-level cap); `app_socket_override_rule`; `access_modes` (`None`→CmuxOnly, `"cmux-only"`, `"full"`→AllowAll, `"off"`→None, `"password"` reads the file's first line trimmed). Integration (`tests/server.rs`, socket in a fresh dir under `std::env::temp_dir()`, test thread answering requests with `futures::executor::block_on(rx.next())`):
  - `ping_v1_and_v2`: `ping` → `PONG`; `{"id":"1","method":"system.ping"}` → `{"id":"1","ok":true,"result":{"pong":true}}`; both on one connection.
  - `requests_reach_the_app`: `{"id":7,"method":"workspace.list"}` → the receiver gets `method == "workspace.list"`, responds `Ok(json!({"x":1}))` → line `{"id":7,"ok":true,"result":{"x":1}}`.
  - `unknown_v1_command`: `foo` → `ERROR: Unknown command 'foo'. Use 'help' for available commands.`; empty line → `ERROR: Empty command`.
  - `auth_flow`: `Password(Some("pw"))`: v2 ping before auth → `auth_required`/`Authentication required. Send auth <password> first.`; `auth nope` → `ERROR: Invalid password`; `auth` → `ERROR: Missing password. Usage: auth <password>`; `auth pw` → `OK: Authenticated`; then v2 ping ok. AllowAll: `auth x` → `OK: Authentication not required`. `auth.login` with params.password mirrors this (`{"authenticated":true}` / `auth_failed`/`Invalid password`).
  - `non_descendant_is_denied`: `CmuxOnly { root: u32::MAX }` → first line `ERROR: Access denied - only processes started inside cmux can connect`, then EOF. `CmuxOnly { root: std::process::id() }` → ping works (the test process is its own root).
  - `unanswered_request_times_out`: deadline 100 ms, never respond → `{"id":1,"ok":false,"error":{"code":"timeout","message":"Timed out waiting for the app","data":{"retryable":true,"deadline_ms":100,"stage":"main_actor"}}}`.
  - `dropped_request_is_internal_error`: drop the `Request` → `internal_error`/`The app did not answer`. Dropping the receiver → same.
  - `stale_socket_is_replaced` (bind, drop the listener without unlink, start again → ok); `live_socket_is_not_stolen` (second `start` → `AddrInUse`, first still serves); `non_socket_file_is_refused` (→ `AlreadyExists`); `socket_and_dir_modes` (0600 / 0700; AllowAll → 0666); `drop_unlinks_socket`.
  - `oversized_line_closes_connection`: write 4 MiB + 1 bytes without `\n` → EOF; `other_clients_still_served` during it.
- [ ] **Step 2: Run** → fail. **Step 3: Implement.** Peer check per D3 (`#[cfg(target_os = "linux")]` `SO_PEERCRED` + `/proc`; else `getpeereid`). Read lines with a `BufReader::take`-style cap. One `std::thread` per connection. Each request: `oneshot` reply, wait with the deadline (`futures::executor::block_on` over a `select` with a timer thread, or `std::sync::mpsc::Receiver::recv_timeout` behind the oneshot — implementer's choice).
- [ ] **Step 4: Run** → pass on macOS; push and watch Linux CI. fmt, clippy. Commit.

### Task 4: CLI core — options, client, help, version (cmux-ipc)

**Files:** Create `crates/cmux-ipc/src/cli/{mod,args,client,help}.rs`, `crates/cmux-ipc/src/bin/cmux.rs`, `crates/cmux-ipc/tests/support/mod.rs`, `crates/cmux-ipc/tests/cli_core.rs`.

**Interfaces — Consumes:** T1, T3 `paths`, `Server` (tests). **Produces:**
```rust
pub struct Ctx { pub env: HashMap<String, String>, pub cwd: PathBuf }
pub struct Outcome { pub stdout: String, pub stderr: String, pub code: i32 }
pub fn run(args: &[String], ctx: &Ctx) -> Outcome;            // cli/mod.rs; bin prints and exits with code
pub(crate) struct Global { pub socket: Option<String>, pub password: Option<String>, pub window: Option<String>, pub json: bool, pub id_format: Option<IdFormat> }
pub(crate) struct Client;  // connect(path) with rc §1 checks + 0.35 s retry; auth(pw); call(method, params) -> Result<Value, String /*CLI error text*/>; v1(line) -> Result<String, String>
// tests/support/mod.rs
pub struct Fake { pub path: PathBuf, pub requests: Arc<Mutex<Vec<(String, Value)>>> }
impl Fake { pub fn start(snap: Snapshot, canned: HashMap<&'static str, Reply>) -> Fake; pub fn ctx(&self, env: &[(&str, &str)]) -> Ctx; }
```
`Fake` runs the real `Server` (AllowAll); each request is answered from `canned`, else `snap.read`, else `method_not_found`/`Unknown method`, and recorded.

- [ ] **Step 1: Write failing tests** (`tests/cli_core.rs`; expected stderr ends with `\n`):
  - `bare_cmux_exits_2`: `[]` → code 2, stdout empty, stderr `Error: Missing command. Usage: cmux <path>|<command> [options]. Run 'cmux --help' for the full command list.`
  - `unknown_command_suggests`: `notfy` → code 2, `Error: Unknown command 'notfy'. Did you mean 'notify'? Run 'cmux --help' for the full command list.`; `zzzzzz` → no "Did you mean".
  - `help_without_socket`: `--help`, `-h`, `help` → code 0, stdout starts `cmux - control cmux via Unix socket\n`, contains every D5 command's usage line, no `tab-action`; for each D5 command `<cmd> --help` and `--json <cmd> -h` with `CMUX_SOCKET_PATH=/nonexistent` → code 0, stdout starts `cmux <cmd>\n\nUsage: cmux <cmd>`; `notify --help` contains `desktop notification` and not `macOS`.
  - `version_forms_match`: `--version`, `-v`, `version` → `cmux <CARGO_PKG_VERSION>\n`, code 0.
  - `socket_env_conflict`: `CMUX_SOCKET_PATH=/a`, `CMUX_SOCKET=/b`, `ping` → code 1, `Error: Refusing to choose socket: CMUX_SOCKET_PATH and CMUX_SOCKET differ. Use CMUX_SOCKET_PATH or unset CMUX_SOCKET.`; adding `--socket <fake>` → `PONG`.
  - `missing_socket`: `--socket /tmp/cmux-none.sock ping` → `Error: Socket not found at /tmp/cmux-none.sock`; a regular file → `Error: Path exists at <p> but is not a Unix socket`.
  - `bad_id_format`: `--id-format x ping` → `Error: --id-format must be one of: refs, uuids, both`; `--id-format` last → `Error: --id-format requires a value (refs|uuids|both)`.
  - `password_is_sent_first`: a password-mode fake: `--password pw ping` → `PONG`; `--password no ping` → `Error: ERROR: Invalid password`; password from `CMUX_SOCKET_PASSWORD` and from `<XDG_STATE_HOME>/cmux-linux/socket-control-password`.
  - `server_error_is_code_colon_message`: canned `not_found`/`Workspace not found` for `workspace.list` → `Error: not_found: Workspace not found`, code 1.
- [ ] **Step 2: Run** → fail. **Step 3: Implement** (`ping` sends v1 `ping` and prints the reply; command table with the help texts of rc §9 adjusted per D10; suggestion = the D5 name at Levenshtein ≤ 2, closest first). **Step 4: Run** → pass. fmt, clippy. Commit.

### Task 5: CLI reads and focus commands (cmux-ipc)

**Files:** Create `crates/cmux-ipc/src/cli/commands.rs`, `crates/cmux-ipc/tests/cli_read.rs`.

**Interfaces — Consumes:** T2 fixture (copy it into `support`), T4. **Produces:** `pub(crate) fn resolve(client, kind: Kind, handle: &str, scope: &Map) -> Result<String /*UUID or ref*/, String>` — UUID passes, a ref is passed through (workspace refs are checked against `workspace.list`: `Workspace ref not found: X`), an integer is looked up by `index` via `window.list`/`workspace.list`/`pane.list`/`surface.list`, anything else → `Invalid <kind> handle: X (expected UUID, ref like <kind>:1, or index)`, a missing index → `<Kind> index not found`.

- [ ] **Step 1: Write failing tests** against the fixture (`CMUX_WORKSPACE_ID`/`CMUX_SURFACE_ID` = A/S1 unless stated):
  - `list_workspaces_text`: `* workspace:1  Terminal  [selected]\n  workspace:2  Build\n`; `--id-format both` rows `* workspace:1 <A-UUID>  Terminal  [selected]`.
  - `list_workspaces_json_keeps_workspace_ids`: `--json` → rows have `id` and `ref`, top level has `window_ref` and no `window_id`; `--json --id-format refs` → rows have no `id`.
  - `list_panes_text`: `  pane:1  [2 surfaces]\n* pane:2  [1 surface]  [focused]\n`; `--json` keeps `id`, `surface_ids`.
  - `list_pane_surfaces_text`: `--pane pane:1` → `* surface:1  zsh  [selected]\n  surface:2  vim\n`; no `--pane` → the focused pane P2.
  - `current_workspace_text`: `workspace:1\n`; `--id-format uuids` → `<A-UUID>\n`; `both` → `workspace:1 (<A-UUID>)\n`.
  - `select_workspace_by_index`: canned `workspace.select` → `OK workspace:2\n`; recorded params `workspace_id` = B's UUID (index `1` resolved via `workspace.list`); `select-workspace` alone → `Error: select-workspace requires --workspace`; `--workspace workspace:9` → `Error: Workspace ref not found: workspace:9`; `--workspace nope` → `Error: Invalid workspace handle: nope (expected UUID, ref like workspace:1, or index)`.
  - `focus_pane_positional_and_flag`: `focus-pane pane:1` and `focus-pane --pane pane:1` → `OK pane:1 workspace:1\n`, `pane.focus` with `pane_id:"pane:1"`; none → `Error: focus-pane requires --pane <id|ref>`.
  - `focus_panel`: `--panel surface:2` → `OK surface:2 workspace:1\n`; none → `Error: focus-panel requires --panel`.
  - `ping_prints_pong`.
- [ ] **Step 2: Run** → fail. **Step 3: Implement.** **Step 4: Run** → pass. fmt, clippy. Commit.

### Task 6: CLI create, split, send, send-key (cmux-ipc)

**Files:** Modify `crates/cmux-ipc/src/cli/commands.rs`; create `crates/cmux-ipc/tests/cli_write.rs`.

- [ ] **Step 1: Write failing tests** (canned replies built with `snap.target_payload`; assert stdout, code and the recorded method + params):
  - `new_workspace_params`: `new-workspace --name Build --cwd ~/p --command "npm test"` (HOME=/h) → `workspace.create {title:"Build", cwd:"/h/p", initial_input:"npm test\r", focus:false, workspace_id:A, surface_id:S1}`; stdout `OK workspace:3\n` both with and without `--json`; `--cwd .` → `ctx.cwd`; `--focus true` → `focus:true`; `--focus maybe` → `Error: --focus must be true|false`.
  - `new_workspace_unknown_flag`: `--layout {}` → `Error: new-workspace: unknown flag '--layout'. Known flags: --name <title>, --cwd <path>, --command <text>, --window <id|ref|index>, --focus <true|false>`.
  - `new_split_directions`: `new-split right` → `surface.split {direction:"right", surface_id:S1-UUID, focus:false}` + `OK surface:4 workspace:1\n`; `U` → `up`; none → `Error: new-split requires a direction`; `sideways` → `Error: new-split: direction must be left|right|up|down`; `--panel surface:2` = `--surface surface:2`; with `--workspace workspace:2` and no `--surface` → no `surface_id` sent.
  - `send_text_rules`: `send 'echo hi\n'` → `surface.send_text {text:"echo hi\r", surface_id:S1, workspace_id:A}` + `OK surface:1 workspace:1\n`; `send echo --force` → text `echo --force`; `send --force -- -x` → `-x`; `send` → `Error: send requires text`; `send --paste 'a\nb'` → `terminal.paste` with text `a\nb` (backslash kept).
  - `send_key_rules`: `send-key ctrl+c` → `surface.send_key {key:"ctrl+c", …}`; `send-key a b` → `Error: send-key: unexpected arguments: b`, nothing recorded; none → `Error: send-key requires a key`.
  - `queued_suffix`: canned `queued:true` → `OK surface:1 workspace:1 queued (terminal starting; input will be sent when its PTY is ready)\n`.
- [ ] **Step 2–4:** run → fail; implement; run → pass. fmt, clippy. Commit.

### Task 7: CLI notify, dismiss, and the hook test (cmux-ipc)

**Files:** Create `crates/cmux-ipc/src/cli/notify.rs`, `crates/cmux-ipc/tests/cli_notify.rs`.

- [ ] **Step 1: Write failing tests** (canned replies `{workspace_id, surface_id, id:"N-UUID"}`):
  - `notify_for_caller`: `notify --title Done --body "Task complete"` → `notification.create_for_caller {title:"Done", subtitle:"", body:"Task complete", caller:true, prefer_tty:false, preferred_workspace_id:A, preferred_surface_id:S1}`, no `effects`; stdout `OK notification:N-UUID\n`; `--id-format uuids` → `OK N-UUID\n`; `both` → `OK notification:N-UUID (N-UUID)\n`; reply `id:null` → `OK\n`. Defaults: no `--title` → `Notification`.
  - `notify_desktop_flag`: `--desktop false` and `--desktop=0` → `effects:{"desktop":false}`; `--desktop maybe` → `Error: --desktop must be true|false`.
  - `notify_routing`: `--surface <S2-UUID>` → `notification.create` with `surface_id` and no `workspace_id`/`window_id`; `--workspace workspace:1 --surface surface:2` → `notification.create_for_target {workspace_id:A-UUID, surface_id:S2-UUID, …}`.
  - `notify_bad_args`: `--body` (last) → `Error: notify: unexpected arguments: --body`; `--title --body x` → same for `--title`; `--title=--body` accepted; `--tab 0` → `Error: notify: unexpected arguments: --tab`; nothing recorded.
  - `notify_clear`: `--clear` → `notification.clear` with the caller params, stdout `OK\n`; `--clear --surface surface:1` (no workspace env) → `Error: notify --clear --surface requires workspace or window context; specify --workspace or --window`, nothing recorded.
  - `notify_not_found_exits_1`: canned `not_found`/`Workspace not found` → `Error: not_found: Workspace not found`, code 1.
  - `dismiss_rules`: `--id notification:N-UUID` → `notification.dismiss {id:"N-UUID"}`, `OK\n`; `--all-read` → `{all_read:true}`; neither or both → `Error: dismiss-notification requires exactly one of --id or --all-read`.
  - `hook_commands_work_unchanged` (spec criterion 1): spawn `bash -c` with `PATH=<dir of env!("CARGO_BIN_EXE_cmux")>:$PATH`, `CMUX_SOCKET_PATH=<fake>`, `CMUX_WORKSPACE_ID=A`, `CMUX_SURFACE_ID=S1`, running verbatim: `command -v cmux &>/dev/null && cmux notify --title "Claude Code" --subtitle "Permission" --body "Approval needed"` and `notification_id="$(cmux notify --title "Done" --body "Task complete" --id-format uuids | awk '$1 == "OK" {print $2}')"; cmux dismiss-notification --id "$notification_id"`. Assert exit 0 and the recorded `create_for_caller` (subtitle `Permission`) then `notification.dismiss {id:"N-UUID"}`.
- [ ] **Step 2–4:** run → fail; implement; run → pass. fmt, clippy. Commit.

### Task 8: Core — split at a surface, subtitle, remove read (cmux-core)

**Files:** Modify `crates/cmux-core/src/{tree,geometry,notifications}.rs`; fix callers in `crates/cmux-app` so the workspace builds.

**Interfaces — Produces:**
- `Layout::try_split_surface(&mut self, surface: SurfaceId, dir: Direction, focus: bool, width: f32, height: f32, scale: f32) -> Result<SurfaceId, NoSpace>` — splits the pane holding `surface` (precondition: it exists; the app resolves first), new pane first for `Left`/`Up`, focus moves to it only when `focus`; same admission and equalize rules as `try_split`, which now delegates with the focused surface, `Right`/`Down`, `focus = true`.
- `Notification { …, subtitle: String }`; `Notifications::add(ws, s, title, subtitle, body, focused, now)`; `Notification::summary()` = body, else subtitle, else title; `Notifications::remove_read(&mut self) -> Vec<NotificationId>`.

- [ ] **Step 1: Write failing tests:** `split_left_puts_new_pane_first`, `split_up_puts_new_pane_first`, `split_without_focus_keeps_focus`, `split_surface_in_unfocused_pane`, `split_surface_respects_min_size` (same `NoSpace` case as the existing `try_split` test); `summary_prefers_body_then_subtitle_then_title`; `remove_read_keeps_unread` (returns the removed ids, bumps `generation`). Existing tests keep passing.
- [ ] **Step 2–4:** run → fail; implement; update `on_notify` and other `add` callers with `String::new()` subtitle; run `cargo test -p cmux-core --locked` and build `cmux-app` → pass. fmt, clippy. Commit.

### Task 9: App — server, env at spawn, reads and focus (cmux-app)

**Files:** Create `crates/cmux-ipc/src/env.rs`, `crates/cmux-app/src/window_view/control.rs`; modify `window_view.rs`, `workspace.rs`, `terminal_view.rs`.

**Interfaces — Consumes:** T1–T3, T8. **Produces:**
- `cmux_ipc::env::terminal_env(workspace_id: &str, surface_id: &str, socket: Option<&Path>, cli_dir: Option<&Path>, path: Option<&str>) -> Vec<(String, String)>` (D8).
- `TerminalView::start(style, cwd, env: &[(String, String)])`.
- `WindowView` holds `server: Option<cmux_ipc::server::Server>` and `handles: Rc<RefCell<Handles>>` (shared with each `WorkspaceView`, which mints the surface handle before spawning); `fn snapshot(&mut self, cx: &App) -> Snapshot`; `fn control(&mut self, method: &str, params: &Map<String, Value>, window: &mut Window, cx: &mut Context<Self>) -> Reply`.

- [ ] **Step 1: Write the failing test** `terminal_env_sets_cmux_vars` (exact vector: the four ids, `CMUX_SOCKET_PATH`, `CMUX_SOCKET=""`, `PATH=<cli_dir>:<path>`; no `CMUX_SOCKET_PATH` when `socket` is None; `PATH` untouched when `cli_dir` is None). Run → fail; implement; pass. Commit.
- [ ] **Step 2: Start the server** in `WindowView::new` before the first `make_view`: `paths::app_socket(std::env::var)`, `Access::from_env(CMUX_SOCKET_MODE, password_file)`, on `AddrInUse` retry `<dir>/cmux-<pid>.sock`, any failure → one `cmux-linux: socket: …` stderr line and no server. Spawn a task (`cx.spawn_in(window, …)`) that takes each `Request` and responds with `this.update_in(cx, |this, window, cx| this.control(…))` (a dropped view → the request is dropped → `internal_error`). `cli_dir` = `current_exe()`'s dir if it holds a `cmux` file.
- [ ] **Step 3: Env at spawn** in `WorkspaceView::add_terminal` from the shared handles.
- [ ] **Step 4: `control`**: `check_window`, then `snapshot.read`, then `workspace.select` (`select`), `pane.focus` (`Layout::focus_pane` + `focus_active`, selecting the workspace first if needed), `surface.focus` (`jump_to`-style: select workspace, `select_surface`, focus). Replies per rc §8. Other methods → `method_not_found`/`Unknown method` until Tasks 10–11.
- [ ] **Step 5: Run it** (desktop or macOS host): in an app terminal, `env | grep CMUX_`, `command -v cmux` (our target dir), `cmux list-workspaces`, `cmux list-panes`, `cmux select-workspace --workspace 1`, `cmux focus-panel --panel surface:1`; start a second app → it logs the fallback and its terminals' `cmux ping` reach it. Push; Linux CI green. fmt, clippy. Commit.

### Task 10: App — create, split, send (cmux-app, cmux-ipc keys)

**Files:** Create `crates/cmux-ipc/src/keys.rs`; modify `crates/cmux-app/src/window_view/{control.rs,..}`, `workspace.rs`, `terminal_view.rs`.

**Interfaces — Produces:** `cmux_ipc::keys::KeySpec { pub key: String /* gpui key name */, pub ctrl: bool, pub shift: bool, pub alt: bool, pub platform: bool }`; `pub fn parse_key(name: &str) -> Option<KeySpec>` (rc §8 names); `pub fn typed_bytes(text: &str) -> Vec<u8>` (`\r\n` and `\n` → `\r`); `pub fn sanitize_paste(text: &str) -> String` (C0 except `\t \n \r`, and DEL → space).

- [ ] **Step 1: Write failing tests:** `named_keys` (`ctrl-c`, `SIGINT`, `return`, `esc`, `arrow_up`, `page_down`, `backtab` → shift+tab, `forward_delete`), `chords` (`ctrl+l`, `alt+left`, `cmd+k` → platform, `ctrl-\`), `bare_letter`, `unknown_key_is_none` (`f13`, `ctrl+`), `typed_bytes_turn_newlines_into_enter`, `sanitize_paste_replaces_controls`. Run → fail; implement; pass. Commit.
- [ ] **Step 2: Mutations in `control`:**
  - `workspace.create`: like `new_workspace` (after selected) with `title` → rename, `cwd`, then `initial_input` written to the new terminal; without `focus:true` re-select the previous workspace. Reply `{window_*, workspace_*, surface_*}`.
  - `surface.split`: `try_split_surface` on the target workspace's view (container size of the selected workspace when the target is hidden), `initial_input`, focus per D6; `NoSpace` → `invalid_state`/`Not enough space to split` (cmux code UNVERIFIED, journal it); reply adds `pane_*`, `type:"terminal"`.
  - `surface.send_text` → `typed_bytes`; `surface.send_key` → `parse_key` → `gpui::Keystroke` → `TerminalView::encode_key(…, KeyAction::Press)` (unknown → `invalid_params`/`Unknown key`, `data {"key":…}`; empty → `invalid_params`/`Missing key`); `terminal.paste` → `sanitize_paste` → `encode_paste` with the terminal's bracketed mode, no confirmation. Missing text → `invalid_params`/`Missing text`. Replies `{…, queued:false}`.
- [ ] **Step 3: Run it:** `cmux new-workspace --name T --command "echo hi"` (sidebar row, not selected, shell printed `hi`), `cmux new-split left` (new pane on the left, focus stays), `cmux send 'ls\n'`, `cmux send-key ctrl+c`, `cmux send --paste --surface surface:1 'a b'`. Push; CI green. fmt, clippy. Commit.

### Task 11: App — notifications over the socket (cmux-app)

**Files:** Modify `crates/cmux-app/src/window_view/{control,notify,panel}.rs`, `desktop_notify.rs`.

- [ ] **Step 1: Methods:** `notification.create` / `create_for_target` / `create_for_caller` resolve with `Snapshot::surface`/`caller` (workspace-level → that workspace's focused surface, D7), then call `on_notify(ws, surface, title, subtitle, body, desktop, …)` (`desktop` from `effects.desktop`, default true; `effects` not an object → `invalid_params`/`Missing or invalid effects`). Reply `{workspace_id, surface_id, id}` (+ refs and window for `create_for_target`). `notification.clear`: `{workspace_id, surface_id}` → `remove_surface`; `{workspace_id}` → `remove_workspace`; caller params → resolve then `remove_surface`; none → `clear_all`; reply `{cleared:true, workspace_id, workspace_ref, surface_id, surface_ref}` (nulls for a clear-all). `notification.dismiss {id}` → `remove` (unknown → `not_found`/`Notification not found`), reply = `{id, workspace_id, workspace_ref, surface_id, surface_ref, title, subtitle, body, is_read, dismissed:1}`; `{all_read:true}` → `remove_read`, `{dismissed:n, all_read:true}`. All go through `after_change` so desktop bubbles are withdrawn.
- [ ] **Step 2: Subtitle display (D7):** desktop bubble body = trimmed `subtitle\nbody`; the panel body line uses `summary()`'s order (body, else subtitle). `on_notify` with `desktop == false` skips the bubble but keeps ring, flash, badge and reorder.
- [ ] **Step 3: Run it:** `cmux notify --title Done --body hi` from an unfocused pane (ring, badge, bubble), `--desktop false` (no bubble), `--subtitle Permission` (bubble shows it), the `--id-format uuids | awk` + `dismiss-notification` line from rc §8, `cmux notify --clear`. Push; CI green. fmt, clippy. Commit.

### Task 12: Docs, desktop exit checks, merge

- [ ] **Step 1: Desktop exit checks (KDE).** Exit 1: add to `~/.claude/settings.json` (back it up first)
  ```json
  {"hooks":{"Notification":[{"hooks":[{"type":"command","command":"command -v cmux &>/dev/null && cmux notify --title 'Claude Code' --body 'Waiting for input'"}]}],
            "Stop":[{"hooks":[{"type":"command","command":"command -v cmux &>/dev/null && cmux notify --title 'Claude Code' --body 'Done'"}]}]}}
  ```
  Run `claude` in a cmux-linux pane, switch to another pane, let it stop and ask for permission: the ring, badge, panel entry and KDE bubble appear on the right pane. Screenshot `hook.png`. Also (from the final review): `command -v cmux` in a new pane is ours; a hook from inside tmux gets the documented denial; a second app instance logs its `cmux-<pid>.sock` and its panes reach it; the Run-it steps of Tasks 9–11 (deferred there: gpui hangs on the macOS host). Exit 2: `cargo test -p cmux-ipc --locked` green on Linux CI (the goldens). Record both in the journal.
- [ ] **Step 2: Docs.** README "CLI and socket" section (install the `cmux` binary next to the app binary, env vars, socket path and modes, ported commands); `docs/design.md` component table if it changed; `NOTICE` (cmux help texts are cmux-derived; `uuid` if direct dependencies are listed there); journal entry with deviations (D1, D3, D7, D9, D10, D11 and any UNVERIFIED code chosen); STATUS (archive first). Commit.
- [ ] **Step 3: Whole-branch review**, fix findings, then merge to `master` with the owner's OK.
