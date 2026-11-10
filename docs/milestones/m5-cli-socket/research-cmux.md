# M5 research — cmux control socket and CLI

Facts from cmux @ 94743c7a (2026-11-10) to match, not code to copy. Cited as `rc §n`. UNVERIFIED = not confirmed from source.

## 0. Sources and licence
- Read only `Sources/`, `CLI/`, `Packages/`, `docs/`, `tests/`, `tests_v2/`, `cmuxCLITests/`, `daemon/` (all GPL-3.0-or-later). Never `web/`, `workers/`, `services/`, `cmux-tui/relays/` (BUSL-1.1).
- Shorthands: `C` = `CLI/cmux.swift`, `CS` = `Packages/macOS/CmuxControlSocket/Sources/CmuxControlSocket/`, `TC` = `Sources/TerminalController.swift`, `SS` = `Packages/macOS/CmuxSettings/Sources/CmuxSettings/SocketControl/`.
- Help texts in §9 are copied verbatim (GPL, attributed in `NOTICE` as cmux-derived).

## 1. Socket path
- State dir is always `~/.local/state/cmux` (no `XDG_STATE_HOME`; `SS/../CmuxStateDirectory.swift:47-52`). Stable default `~/.local/state/cmux/cmux.sock`; dev/nightly variants use `/tmp/cmux-<variant>[-slug].sock` (`SocketPathMarkerFiles.swift:86-125`).
- App honours `CMUX_SOCKET_PATH` only with `CMUX_ALLOW_SOCKET_OVERRIDE` truthy (`1/true/yes/on`) or in debug builds (`SocketControlSettings.swift:164-203, 382-397`). So an app started from inside a cmux terminal does not grab the parent's socket.
- CLI order (`C:6631-6650`): `--socket <p>` / `--socket=p` (missing → `--socket requires a path`); then `CMUX_SOCKET_PATH`, then deprecated `CMUX_SOCKET` (both trimmed; set and different →
  `Refusing to choose socket: CMUX_SOCKET_PATH and CMUX_SOCKET differ. Use CMUX_SOCKET_PATH or unset CMUX_SOCKET.`); then default discovery over variant defaults and marker files (`CLISocketPathResolver.swift:246-313`). Explicit/env paths are never probed or rerouted.
- Socket mode `0600` (`0666` only for allowAll); parent dir created `0700`; backlog 128 (`+Bind.swift:101-113`).
- Stale path (`+Bind.swift:69-97`): non-socket file → refuse (`EEXIST`); another uid's socket → refuse; probe connect: `ENOENT`/`ECONNREFUSED` → unlink and bind; connects → `EADDRINUSE` (stable build then falls back to `cmux-<uid>.sock`).
- Client checks (`C:4061-4111`): `lstat`, symlink followed only if owned; target must be `S_IFSOCK` and owned by euid.

## 2. Wire format
- Newline-delimited; many requests per connection; every response is one line + `\n` (`TC:2497-2667`). Line starting with `{` (after trim) is v2, else v1 (`TC:3001,3105`).
- v1: `<cmd> <args>`, cmd lowercased. Replies `OK`, `OK <text>`, `PONG`, `ERROR: <msg>`. Unknown: `ERROR: Unknown command '<cmd>'. Use 'help' for available commands.` Empty: `ERROR: Empty command`.
- v2 request: `{"method":"…","id":<any>,"params":{…}}` (no `jsonrpc` key). `params` defaults to `{}`. CLI sends `id` as a UUID string, `method` first, other keys sorted (`CS/Wire/ControlRequestLine.swift:26-41`).
- v2 responses (`ControlResponseEncoder.swift:25-54`):
  ```
  {"id":<id|null>,"ok":true,"result":{…}}
  {"id":<id|null>,"ok":false,"error":{"code":"…","message":"…","data":{…}}}
  ```
  `data` omitted when empty. Parse errors (no `id`): `invalid_utf8`/`Invalid UTF-8`, `parse_error`/`Invalid JSON`, `invalid_request`/`Expected JSON object`; `invalid_request`/`Missing method` (with id). Unknown method: `method_not_found`/`Unknown method` (`TC:3687`).
- Error codes used by our methods: `invalid_params`, `not_found`, `internal_error`, `timeout`, `auth_required`, `auth_failed`, `method_not_found`.
- Main-thread deadline 10 s → `{"code":"timeout","message":…,"data":{"retryable":true,"deadline_ms":10000,"stage":"main_actor"}}` (`TerminalController+SocketBoundedLanes.swift:158`). Message text UNVERIFIED.
- `system.ping` → `{"pong":true}`; v1 `ping` → `PONG` (`TC:2044, 3126`).

## 3. Access control
- Modes: `off`, `cmuxOnly` (default), `automation`, `password`, `allowAll`; env override `CMUX_SOCKET_MODE` (aliases `cmux-only`/`cmux_only`, `openaccess`/`fullopenaccess`/`full` → allowAll) (`SS/Values/SocketControlMode.swift:52-72`).
- cmuxOnly: peer pid must descend from the app's pid (walk parents ≤128 levels, `CS/Transport/SocketTransport+Peer.swift:68-133`); else (capability token path, not ported). Denied: one line then close:
  `ERROR: Access denied - only processes started inside cmux can connect` / `ERROR: Unable to verify client process`.
- password: same-uid peer; password sent once per connection as v1 `auth <password>` (or v2 `auth.login {password}`). Replies `OK: Authenticated`, `ERROR: Invalid password`, `ERROR: Missing password. Usage: auth <password>`. Before auth: v1 `ERROR: Authentication required — send auth <password> first`, v2 `auth_required`/`Authentication required. Send auth <password> first.` Non-password mode: `auth` → `OK: Authentication not required` (`TerminalController+SocketClientCapability.swift:830-927`, `TC:3129`).
- CLI password: `--password`, then `CMUX_SOCKET_PASSWORD`, then file `~/.local/state/cmux/socket-control-password` (0600), then Keychain. Sends `auth <pw>` first whenever it has one; an `ERROR:` reply fails unless it contains `Unknown command 'auth'` (`C:2998-3010, 3376-3399`).

## 4. CLI errors and exit codes
- All errors: stderr `Error: <message>`, exit 1 unless stated (`C:~46389`).
- Exit 2: `Missing command. Usage: cmux <path>|<command> [options]. Run 'cmux --help' for the full command list.` and `Unknown command '<x>'. [Did you mean '<y>'?] Run 'cmux --help' for the full command list.` (suggestion at edit distance ≤ 2, `CMUXCLI+CommandSuggestions.swift:4-11`).
- Connect: `Socket not found at <p>`, `Path exists at <p> but is not a Unix socket`, `Socket at <p> is not owned by the current user — refusing to connect`, `Failed to connect to socket at <p> (<strerror>, errno <n>)`. `ECONNREFUSED`/`ENOENT`/`EAGAIN` retried for 0.35 s every 25 ms.
- v2 error shown as `<code>: <message>` → `Error: not_found: Workspace not found`. v1 `ERROR:` shown verbatim → `Error: ERROR: …`.
- Timeouts: 15 s response (`CMUXTERM_CLI_RESPONSE_TIMEOUT_SEC` overrides); `Command timed out`, `Socket closed before reply`, `Mismatched v2 response id`, `Invalid v2 response: <raw>`.
- Argument errors (`C:23632-23694`): `<cmd>: unknown option <x>`, `<cmd>: unexpected argument <x>`, `<cmd>: <opt> requires a value`; notify/send-key/dismiss use `<cmd>: unexpected arguments: <rest>`.
- Handle errors (`C:11027-11290`): `Invalid workspace handle: X (expected UUID, ref like workspace:1, or index)` (same for window/pane/surface), `Workspace index not found`, `Workspace ref not found: X`, `Pane index not found`, `Surface index not found`, `Window not found: X`.

## 5. Terminal environment (`TerminalSurface+RuntimeSurfaceCreation.swift:95-247`)
- `CMUX_WORKSPACE_ID`, `CMUX_TAB_ID` = workspace UUID; `CMUX_SURFACE_ID`, `CMUX_PANEL_ID` = surface UUID; `CMUX_SOCKET_PATH` = live socket; `CMUX_SOCKET` = `""` (blanked). UUIDs are Swift `uuidString`: **uppercase**.
- `PATH` is prefixed with the bundled CLI's dir (`Resources/bin`), so `command -v cmux` finds the app's own CLI.
- `CMUX_SOCKET_PASSWORD` is **not** set in terminals (listed as protected in docs, no code sets it).
- Not ported: `TERM_PROGRAM=ghostty`, `CMUX_PORT*`, `CMUX_SOCKET_CAPABILITY`, shell-integration and agent-hook variables.

## 6. Handles and `--id-format`
- Refs `<kind>:<n>`, per-kind counters, minted on first sight, stable across reorder, never reused (`CS/Wire/ControlHandleRegistry.swift:59-71`). Kinds: `window`, `workspace`, `pane`, `surface`; `tab:N` resolves to `surface:N`. Shipping builds start at 1 000 000 000 for cross-restart uniqueness; tests start at 1.
- Server resolves UUID (any case) or exact ref; **bare indexes are resolved by the CLI** against `window.list` / `workspace.list` / `pane.list` / `surface.list` `index` (0-based position) (`C:11019-11300`).
- Server replies always carry both forms (`id`+`ref`, `X_id`+`X_ref`, `X_ids`+`X_refs`).
- `formatIDs` (`C:10897-10955`), recursive over objects: `refs` (default) drops `id` when `ref` exists, `X_id` when `X_ref` exists, `X_ids` when `X_refs` exists; `uuids` drops the ref side; `both` keeps all. With `--json` and no explicit `--id-format`, workspace ids are kept in `list-workspaces`/`current-workspace`, pane+surface ids in `list-panes`/`list-pane-surfaces` (`C:7028-7029`).
- Text handles: `formatHandle` refs `ref ?? id`, uuids `id ?? ref`, both `ref (id)`; list rows `textHandle` both = `ref id` (space).
- Bad mode: `--id-format must be one of: refs, uuids, both`.
- JSON printing: `JSONSerialization` `[.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]` (`CMUXCLI+JSONOutput.swift:113-122`) → 2-space indent, `"key" : value`. Empty containers' exact pretty form UNVERIFIED.

## 7. Global invocation
- Global options before the command: `--socket`, `--password`, `--window`, `--id-format` (each `--x v` or `--x=v`), `--json`; `-h/--help`, `-v/--version`. After the command, `--json` and `--id-format <v>` are stripped from anywhere before `--` (`CmuxCLIArgumentParser.swift:94-141`).
- `cmux <cmd> --help|-h` anywhere before `--`: prints the command help, exit 0, no socket.
- Version: `cmux <version> (<build>) [<commit>]`, fallbacks `cmux <version>`; exit 0; `cmux version` identical (`C:46007-46022`).
- Text output uses `OK` summaries (`C:24504-24568`): `OK <surface> <workspace>` etc.; `--json` prints the formatted payload.
- `list-workspaces`, `new-workspace`, `select-workspace` print a deprecation notice to stderr pointing at `cmux workspace …` (`C:13433-13438`).

## 8. Commands (method, params, server rules, output)
Targets accept `<id|ref|index>`. Env defaults: `CMUX_WORKSPACE_ID`, `CMUX_SURFACE_ID` (UUIDs).

**notify** — flags `--title` (default `Notification`), `--subtitle`, `--body` (default `""`), `--reply`, `--desktop <true|false>` (`1/true/yes/on`, `0/false/no/off`; else `--desktop must be true|false`), `--clear`, `--workspace`, `--surface`, `--window`. A value flag followed by a `--…` token or nothing → `notify: unexpected arguments: <flag>`; `--title=--body` allowed. `--tab/--panel` are **rejected** (`notify: unexpected arguments: --tab`) though `docs/notifications.md:61` shows them (`CMUXCLI+NotificationFormatting.swift:12-14`).
- Routing (`C:20982-21072`): (1) `--surface` given and a non-UUID surface/workspace handle → resolve both → `notification.create_for_target {workspace_id, surface_id, title, subtitle, body}`; (2) `--surface <UUID>` → `notification.create {title, subtitle, body, surface_id[, workspace_id if --workspace]}`; (3) `--window` without `--surface` → `notification.create {…, workspace_id, window_id}`; (4) else `notification.create_for_caller {title, subtitle, body, caller:true, prefer_tty:false, [preferred_workspace_id], [preferred_surface_id]}` from `--workspace` or the env. `effects:{"desktop":<bool>}` only when `--desktop` given.
- create_for_caller (`Sources/TerminalNotificationCallerResolver.swift:160-233`): existing preferred workspace wins (preferred surface if in it, else TTY match, else workspace-level); else preferred surface; else TTY; selectors given but none resolved → `not_found: Workspace not found`; no selectors → selected workspace, workspace-level.
- Replies `{workspace_id, surface_id|null, id|null}`; create_for_target adds `workspace_ref, surface_ref, window_id, window_ref`. Text `OK notification:<UUID>` (refs), `OK <UUID>` (uuids), `OK notification:<UUID> (<UUID>)` (both), `OK` if no id.
- `--clear` → `notification.clear {workspace_id, surface_id}` | `{workspace_id}` | caller params; no params = clear all. Reply `{cleared:true, workspace_id, workspace_ref, surface_id|null, surface_ref|null}`; prints `OK`. `--clear --surface` without workspace/window → `notify --clear --surface requires workspace or window context; specify --workspace or --window` (no request).
- Docs pin: `cmux notify … --id-format uuids | awk '$1 == "OK" {print $2}'` then `cmux dismiss-notification --id "$id"`.

**dismiss-notification** — exactly one of `--id <uuid|notification:<uuid>>` (prefix stripped case-insensitively) or `--all-read` (else `dismiss-notification requires exactly one of --id or --all-read`). `notification.dismiss {id}` → notification payload + `dismissed:1`, unknown → `not_found: Notification not found`; `{all_read:true}` → `{dismissed:n, all_read:true}`. Text `OK`.

**new-workspace** — `--name`, `--cwd` (`~` expanded, made absolute), `--command <text>` (sent as `initial_input = text + "\r"`, blank dropped), `--focus <true|false>` (default false), `--window`; also `--description`, `--env`, `--env-file`, `--layout`, `--group*` (not ported). Unknown: `new-workspace: unknown flag '--x'. Known flags: …`. Context: `--window`, else caller `workspace_id`/`surface_id`. `workspace.create {…, focus}` → `{window_id, window_ref, workspace_id, workspace_ref, surface_id, surface_ref}`. Placement `afterCurrent`; selected only with `focus:true`; `initial_input` typed into the interactive shell. Output always `OK <workspace_ref>`; `--json` ignored.

**new-split** — positional `left|right|up|down` (also `l|r|u|d`, any case; missing → `new-split requires a direction`, bad → `new-split: direction must be left|right|up|down`); `--workspace`, `--surface`, `--panel` (= surface), `--window`, `--command`, `--focus` (default false). Surface default `CMUX_SURFACE_ID` when no workspace/window given. `surface.split {direction, [workspace_id], [surface_id], [initial_input], focus}`; splits that surface (else focused); left/up put the new pane first; focus only with `focus:true`. Reply `{window_id, window_ref, workspace_id, workspace_ref, pane_id, pane_ref, surface_id, surface_ref, type:"terminal"}`; errors `not_found: Surface not found`, `not_found: Workspace not found`, a no-space error (code UNVERIFIED). Text `OK <surface> <workspace>`.

**send** — `--workspace`, `--surface`, `--window`, leading `--paste`/`--force`, optional `--`; text = remaining tokens joined by one space; `\n`/`\r` → CR, `\t` → Tab (CLI unescapes). `surface.send_text {text, …}` (or `terminal.paste` with `--paste`, text not unescaped; control chars such as Esc, Ctrl-C replaced by spaces). Reply `{workspace_id, workspace_ref, surface_id, surface_ref, queued, window_id, window_ref}`. Text `OK <surface> <workspace>`. `send requires text`. (`surface.input_state` draft guard not ported; `--force` accepted.)

**send-key** — same targeting, leading `--force`, exactly one key (`send-key requires a key`, extra → `send-key: unexpected arguments: <rest>`). `surface.send_key {key, …}`. Keys case-insensitive (`TerminalSurface+Input.swift:845-949`): `ctrl-c|ctrl+c|sigint`, `ctrl-d|ctrl+d|eof`, `ctrl-f`, `ctrl-z|sigtstp`, `ctrl-\|sigquit`, `enter|return`, `tab`, `escape|esc`, `backspace`, `up|down|left|right` (also `arrow_up`, `arrowup`, …), `shift+tab|shift-tab|backtab`, `home`, `end`, `delete|del|forward_delete`, `pageup|page_up`, `pagedown|page_down`; chords split on `+`/`-` with `ctrl|control`, `shift`, `alt|opt|option`, `cmd|command|super` over `enter return tab escape esc backspace delete space up down left right \` or a–z; a bare letter. Else `invalid_params: Unknown key`.

**list-workspaces** — `workspace.list` → `{window_id, window_ref, workspaces:[{id, ref, index, title, custom_title, has_custom_title, description, selected, pinned, listening_ports, remote, current_directory, custom_color, …}]}`. Text `No workspaces` or per row `<"* "|"  "><handle>[  <title>][  [selected]]`, e.g. `* workspace:1  Terminal 1  [selected]`.

**current-workspace** — `workspace.current` → `{window_id, window_ref, workspace_id, workspace_ref, workspace:{row}}`; text = the handle only.

**list-panes** — `pane.list {workspace_id}` → `{workspace_id, workspace_ref, window_id, window_ref, panes:[{id, ref, index, focused, surface_ids, surface_refs, selected_surface_id, selected_surface_ref, surface_count}]}`. Text `No panes` or `<"* "|"  "><handle>  [<n> surface|surfaces][  [focused]]`.

**list-pane-surfaces** — `--pane` default focused pane. `pane.surfaces` → `{workspace_id, workspace_ref, pane_id, pane_ref, window_id, window_ref, surfaces:[{id, ref, index, title, type, selected}]}`; `not_found: Pane or workspace not found`. Text `No surfaces in pane` or `<"* "|"  "><handle>  <title>[  [selected]]`.

**select-workspace** — `--workspace` required (no env default): `workspace.select` → `{window_id, window_ref, workspace_id, workspace_ref}`; text `OK <workspace>`.

**focus-pane** — `--pane <h>` or first positional; `pane.focus` → `{…, pane_id, pane_ref}`; text `OK <pane> <workspace>`; `focus-pane requires --pane <id|ref>`.

**focus-panel** — `--panel <h>` required; `surface.focus` → `{workspace_*, surface_*, window_*}`; text `OK <surface> <workspace>`; `focus-panel requires --panel`. (`focus-surface`/`list-surfaces` do not exist.)

## 9. Help texts (verbatim; `cmux <command>`, blank line, then the text)
Sources: `C:23087-23237, 22201-22451`, `CMUXCLI+Paste.swift:414-438`. Swift indentation removed. Lines the plan changes (D10) are marked `‡`.

```
Usage: cmux notify [flags]

Send a notification to a workspace/surface, or clear that resolved target with --clear.

Flags:
  --title <text>         Notification title (default: "Notification")
  --subtitle <text>      Notification subtitle
  --body <text>          Notification body
  --reply                Allow a free-text inline reply                                     ‡
  --desktop <true|false> Post a native macOS banner (default: true). false keeps the entry in the Notifications panel, sidebar badge and pane ring without a banner   ‡
  --clear                Clear notifications for the resolved caller/target instead of posting
  --workspace <id|ref|index>   Target workspace, except explicit surface UUIDs resolve globally
  --surface <id|ref|index>     Target surface (refs/indexes use workspace/window context)
  --window <id|ref|index>      Window context for workspace/surface refs and indexes
  --json                 Print the response payload as JSON
  --id-format <mode>     refs, uuids, or both for human-readable ids

The response includes the created notification id. Use cmux dismiss-notification --id <uuid|notification:<uuid>>, cmux list-notifications, or cmux clear-notifications to manage notifications.   ‡
--desktop sets the notification's desktop effect before notification hooks run; hooks can still override it. It has no effect with --clear.   ‡

Example:
  cmux notify --title "Build done" --body "All tests passed"
  cmux notify --title "Error" --subtitle "test.swift" --body "Line 42: syntax error"
  cmux notify --surface <uuid> --title "Build done"
  cmux notify --title "Deploy finished" --desktop false
  cmux notify --clear
```
```
Usage: cmux dismiss-notification (--id <uuid> | --all-read)

Remove one notification, or remove every already-read notification.

Flags:
  --id <uuid|notification:<uuid>>  Notification id or notify handle to remove
  --all-read            Remove every already-read notification
  --json                Print JSON
  --id-format <mode>    refs, uuids, or both
```
```
Usage: cmux new-workspace [--name <title>] [--description <text>] [--cwd <path>] [--command <text>] [--env KEY=VALUE]... [--env-file <path>]... [--layout <json>] [--window <id|ref|index>] [--focus <true|false>] [--group <id|ref>] [--group-placement afterCurrent|top|end] [--group-reference <workspace>]   ‡

Create a new workspace in the caller's window.

Flags:
  --name <title>       Set a custom name for the new workspace
  --description <text> Set a custom description for the new workspace          ‡
  --cwd <path>         Set the working directory for the new workspace
  --command <text>     Run this as the new terminal's initial command
  --env KEY=VALUE      Set a workspace environment variable. Repeatable.        ‡
                       Reserved CMUX_* variables cannot be overridden.          ‡
  --env-file <path>    Load KEY=VALUE lines from a file. Repeatable.            ‡
  --layout <json>      Create workspace with a predefined split layout.         ‡
                       Layout surfaces define their own commands.               ‡
  --window <id|ref|index> Target window (default: caller's window)
  --focus <true|false> Focus the new workspace (default: false)
  --group <id|ref>     Add the new workspace to a workspace group               ‡
  --group-placement afterCurrent|top|end Placement within --group (default: top)   ‡
  --group-reference <workspace> Reference workspace for afterCurrent placement  ‡

Example:
  cmux new-workspace
  cmux new-workspace --name "Build Server"
  cmux new-workspace --name "Launch" --description "Ship checklist"             ‡
  cmux new-workspace --cwd ~/projects/myapp
  cmux new-workspace --cwd . --command "npm test"
  cmux new-workspace --name "Dev" --layout '{…}'                                ‡
```
```
Usage: cmux new-split <left|right|up|down> [flags]

Split the current pane in the given direction.

Flags:
  --workspace <id|ref>   Target workspace (default: $CMUX_WORKSPACE_ID)
  --surface <id|ref>     Surface to split from (default: $CMUX_SURFACE_ID)
  --panel <id|ref>       Alias for --surface
  --window <id|ref|index>
                          Window context for workspace/surface refs and indexes
  --command <text>     Run this as the new terminal's initial command
  --focus <true|false>   Focus the new split (default: false)

Example:
  cmux new-split right
  cmux new-split down --workspace workspace:1
```
```
Usage: cmux send [flags] [--] <text>

Send text to a terminal surface as keystrokes. Escape sequences: \n and \r send Enter, \t sends Tab.

For large text (for example over 4 KB, or several lines going to an agent), use --paste or cmux paste. Keystrokes press Enter at every newline, which can submit each line on its own.   ‡

With --paste, the text goes through the same paste path as cmux paste and Cmd+V, unchanged: escape sequences such as \n are not interpreted, and control characters such as Esc and Ctrl-C are replaced with spaces. --paste must come before the text.   ‡

If an agent prompt in the target holds text someone is typing, or a question or permission dialog is open, nothing is sent and the command fails. --force sends anyway.   ‡

Flags:
  --workspace <id|ref|index>   Target workspace (default: $CMUX_WORKSPACE_ID)
  --surface <id|ref|index>     Target surface (default: $CMUX_SURFACE_ID)
  --window <id|ref|index>      Window context for workspace/surface refs and indexes
  --paste                      Paste the text instead of typing it
  --force                      Send even over a draft or into an open dialog   ‡

Example:
  cmux send "echo hello"
  cmux send --surface surface:2 "ls -la\n"
  cmux send --paste --surface surface:2 "$(cat notes.md)"
```
```
Usage: cmux send-key [flags] [--] <key>

Send a key event to a terminal surface.

Flags:
  --workspace <id|ref|index>   Target workspace (default: $CMUX_WORKSPACE_ID)
  --surface <id|ref|index>     Target surface (default: $CMUX_SURFACE_ID)
  --window <id|ref|index>      Window context for workspace/surface refs and indexes
  --force                      Send even into an open dialog   ‡

Example:
  cmux send-key enter
  cmux send-key --surface surface:2 ctrl+c
```
```
Usage: cmux list-workspaces [--window <id|ref|index>]

List workspaces in a window.

Flags:
  --window <id|ref|index>   Target window (default: caller/current window)

Example:
  cmux list-workspaces
```
```
Usage: cmux current-workspace [--window <id|ref|index>]

Print the selected workspace ID for a window.
```
```
Usage: cmux list-panes [--workspace <id|ref|index>] [--window <id|ref|index>]

List panes in a workspace.

Flags:
  --workspace <id|ref|index>   Workspace context (default: $CMUX_WORKSPACE_ID)
  --window <id|ref|index>      Window context for workspace refs and indexes

Example:
  cmux list-panes
  cmux list-panes --workspace workspace:2
```
```
Usage: cmux list-pane-surfaces [--workspace <id|ref|index>] [--pane <id|ref|index>] [--window <id|ref|index>]

List surfaces in a pane.

Flags:
  --workspace <id|ref|index>   Workspace context (default: $CMUX_WORKSPACE_ID)
  --pane <id|ref|index>        Restrict to a specific pane (default: focused pane)
  --window <id|ref|index>      Window context for workspace/pane refs and indexes

Example:
  cmux list-pane-surfaces
  cmux list-pane-surfaces --workspace workspace:2 --pane pane:1
```
```
Usage: cmux select-workspace --workspace <id|ref|index> [--window <id|ref|index>]

Select (switch to) the specified workspace.

Flags:
  --workspace <id|ref|index>   Workspace to select (required)
  --window <id|ref|index>      Window context for workspace refs and indexes

Example:
  cmux select-workspace --workspace workspace:2
  cmux select-workspace --workspace 0
```
```
Usage: cmux focus-pane [--pane <id|ref|index> | <id|ref|index>] [flags]

Focus the specified pane.

Flags:
  --pane <id|ref|index>       Pane to focus (required unless passed positionally)
  --workspace <id|ref|index>  Workspace context (default: $CMUX_WORKSPACE_ID)
  --window <id|ref|index>     Window context for workspace/pane refs and indexes

Example:
  cmux focus-pane --pane pane:2
  cmux focus-pane pane:1
  cmux focus-pane --pane pane:1 --workspace workspace:2
```
```
Usage: cmux focus-panel --panel <id|ref|index> [--workspace <id|ref|index>] [--window <id|ref|index>]

Focus a specific panel (surface).

Flags:
  --panel <id|ref|index>       Panel/surface to focus (required)
  --workspace <id|ref|index>   Workspace context (default: $CMUX_WORKSPACE_ID)
  --window <id|ref|index>      Window context for workspace/panel refs and indexes

Example:
  cmux focus-panel --panel surface:2
  cmux focus-panel --panel surface:5 --workspace workspace:2
```

Top level (`CMUXCLI+TaskHelp.swift:137-185`, verbatim skeleton; the `Agent Help`/`Task Help` blocks are not ported):
```
cmux - control cmux via Unix socket

Usage:
  cmux <path>                Open a directory in a new workspace (launches cmux if needed)   ‡
  cmux [global-options] <command> [options]

Targets:
  Commands that accept a window, workspace, pane, or surface take a UUID, a short ref (window:1/workspace:2/pane:3/surface:4), or an index.
  `tab-action` also accepts `tab:<n>` in addition to `surface:<n>`.   ‡
  Output defaults to refs; pass --id-format uuids or --id-format both to include UUIDs.

Socket Auth:
  --password takes precedence, then CMUX_SOCKET_PASSWORD, then the password saved in Settings.   ‡

Commands:
  <group>:
    <usage line per command, 4-space indent; groups separated by a blank line>

Environment:
  CMUX_WORKSPACE_ID   Auto-set in cmux terminals. Used as default --workspace for
                      ALL commands (send, list-panels, new-split, notify, etc.).
  CMUX_TAB_ID         Optional alias used by `tab-action`/`rename-tab` as default --tab.   ‡
  CMUX_SURFACE_ID     Auto-set in cmux terminals. Used as default --surface.
  CMUX_SOCKET_PATH    Override the Unix socket path. Without this, the CLI defaults
                      to ~/.local/state/cmux/cmux.sock and auto-discovers tagged/debug sockets.   ‡
```
Usage lines (`CMUXCLI+TaskHelp.swift:195-334`), cmux groups: Start & Resume (`new-workspace`), Navigate & Arrange (`new-split`, `focus-pane`, `focus-panel`, `select-workspace`), Inspect (`list-workspaces`, `list-panes`, `list-pane-surfaces`, `current-workspace`), Automation (`send`, `send-key`, `notify`, `dismiss-notification`), Diagnostics / Advanced (`ping`, `version`). Group membership per command is UNVERIFIED beyond these names; the lines themselves:
```
new-workspace [--name <title>] [--description <text>] [--cwd <path>] [--command <text>] [--layout <json>] [--window <id|ref|index>] [--focus <true|false>] [--group <id|ref>] [--group-placement afterCurrent|top|end] [--group-reference <workspace>]
new-split <left|right|up|down> [--workspace <id|ref|index>] [--surface <id|ref|index>] [--panel <id|ref|index>] [--window <id|ref|index>] [--command <text>] [--focus <true|false>]
focus-pane --pane <id|ref|index> [--workspace <id|ref|index>] [--window <id|ref|index>]
focus-panel --panel <id|ref|index> [--workspace <id|ref|index>] [--window <id|ref|index>]
select-workspace --workspace <id|ref|index> [--window <id|ref|index>]
list-workspaces [--window <id|ref|index>]
list-panes [--workspace <id|ref|index>] [--window <id|ref|index>]
list-pane-surfaces [--workspace <id|ref|index>] [--pane <id|ref|index>] [--window <id|ref|index>]
current-workspace [--window <id|ref|index>]
send [--workspace <id|ref|index>] [--surface <id|ref|index>] [--window <id|ref|index>] [--paste] <text>
send-key [--workspace <id|ref|index>] [--surface <id|ref|index>] [--window <id|ref|index>] <key>
notify [--title <text>] [--subtitle <text>] [--body <text>] [--reply] [--desktop <true|false>] [--clear] [--workspace <id|ref|index>] [--surface <id|ref|index>] [--window <id|ref|index>]
dismiss-notification (--id <uuid> | --all-read)
```

## 10. Tests worth mirroring
- `tests/test_cli_contract_help.py` — every `cmux <cmd> --help` exits 0 without a socket and contains `Usage: cmux <cmd>`; bare `cmux` exits 2 with `Missing command` on stderr and empty stdout.
- `tests/test_cli_version_flag.py` — `--version` matches `\d+\.\d+\.\d+`, equals `cmux version`.
- `tests_v2/test_cli_id_format_defaults.py`, `cmuxCLITests/CLIWorkspaceStableIDTests.swift` — the id-format matrix.
- `cmuxCLITests/CLIExplicitSurfaceRoutingTests.swift` — send/send-key params; malformed notify/dismiss args fail with no socket; server error → `Error: invalid_params: Surface is not a terminal`.
- `cmuxTests/CLINotifyProcessIntegrationRegressionTests*.swift` — `--surface <UUID>` sends `notification.create` with no `workspace_id`; `effects` only with `--desktop`; `--clear --surface` without context exits 1 with no request.
- `CS/../Tests/CmuxControlSocketTests/ControlResponseEncoderTests.swift` — exact envelopes.
