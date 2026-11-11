<h1 align="center">cmux-linux</h1>

<p align="center">
  A native Rust port of <a href="https://github.com/manaflow-ai/cmux">cmux</a>'s terminal workspace for Linux and macOS:<br/>
  vertical tabs, split panes and notification rings for running many AI coding agents side by side.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="License: GPL-3.0-or-later"/></a>
  <img src="https://img.shields.io/badge/status-pre--alpha-orange" alt="Status: pre-alpha"/>
  <img src="https://img.shields.io/badge/platforms-Linux%20%7C%20macOS-lightgrey" alt="Platforms: Linux, macOS"/>
</p>

> **Status: pre-alpha.** A single terminal (M1), splits and tabs (M2), the workspace sidebar
> (M3) and notifications (M4, on a branch) are built; M2–M4 still await their checks on a KDE
> desktop. See [`STATUS.md`](STATUS.md) for where things stand.

## Why

[cmux](https://github.com/manaflow-ai/cmux) is a Ghostty-based macOS terminal
built for coordinating AI coding agents: a sidebar of workspaces, splits, and
notification rings that light up when an agent needs you. It is a Swift/AppKit
app, so it only runs on macOS.

cmux-linux brings that terminal workspace to Linux (KDE Plasma on Wayland first)
and keeps it working on macOS, aiming to look and behave the same as the
original. It is a **local app**: no cloud services, no browser, no account.

## Planned features

- **Workspaces sidebar:** vertical tabs showing working directory, git branch,
  listening ports, linked PR and the latest notification.
- **Splits and tabs:** horizontal and vertical splits, a tab strip per pane.
- **Notifications:** pane rings, unread badges, a notification panel, jump to
  latest unread, and desktop notifications. Driven by OSC 9/99/777 or the CLI.
- **cmux-compatible CLI:** `cmux notify` and friends over a Unix socket, with the
  same commands and environment variables, so existing agent hooks keep working.
- **Ghostty-compatible:** reads your `~/.config/ghostty/config` for fonts,
  themes and colours.
- **Keyboard first:** cmux's shortcuts with ⌘ mapped to Super; every binding can
  be remapped.

Also here: an agent approval Feed, workspace groups, a Dock and agent-to-agent
messages (below). Later: session restore, a command palette and SSH workspaces.

## How it's built

| Part | Choice |
|---|---|
| UI | [gpui](https://github.com/zed-industries/zed/tree/main/crates/gpui), GPU-rendered, the same look on Linux and macOS |
| Terminal engine | libghostty-vt from [Ghostty](https://github.com/ghostty-org/ghostty) |
| Language | Rust |

Full design: [`docs/design.md`](docs/design.md).

## Roadmap

| Milestone | Goal |
|---|---|
| M0 foundations | Toolchain and risky choices proven |
| M1 terminal | One fully usable terminal |
| M2 splits-tabs | Split panes and tabs |
| M3 workspaces-sidebar | Workspaces and the sidebar |
| M4 notifications | Pane rings, badges, notification panel |
| M5 cli-socket | Scriptable through the `cmux` CLI |
| M6 parity-release | Visual parity, packaging, first release |
| M7 feed | Agent approvals in the right sidebar |
| M8 groups-dock | Workspace groups and a Dock of terminals |
| M9–M11 | Agent coordination, persistence, productivity |

Each milestone has a spec, plan and journal under [`docs/milestones/`](docs/milestones).

## Install

Arch / CachyOS: build and install the package. This clones GitHub master (it builds Ghostty's
Zig library too, so it needs network access):

```sh
cd packaging/arch && makepkg -si
```

To build your local checkout instead, run
`_repo="file://$(git rev-parse --show-toplevel)" makepkg -si` in `packaging/arch`; only
committed changes are built.

The PKGBUILD needs `rustup` (it honours `rust-toolchain.toml`), which conflicts with Arch's `rust`
package; if you have `rust` installed, replace it with `rustup` first.

It installs `cmux-linux` and the `cmux` CLI side by side in `/usr/lib/cmux-linux/`, with
symlinks in `/usr/bin` and a `.desktop` entry.

macOS: build a release binary (a debug build of the app hangs) and run it:

```sh
scripts/install-zig.sh
export PATH="$PWD/.tools/zig:$PATH"
cargo build --release --locked --features gpui/runtime_shaders
target/release/cmux-app
```

## Building

```sh
git clone --recursive https://github.com/Izuulo/cmux-linux
# existing clone: git submodule update --init --recursive
```

System packages:

```sh
# Ubuntu 24.04
sudo apt-get install -y --no-install-recommends \
  clang libclang-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libvulkan-dev libfontconfig-dev libfreetype-dev libx11-xcb-dev libxcb1-dev \
  libssl-dev libzstd-dev
# Arch / CachyOS
sudo pacman -S --needed clang libxkbcommon libxkbcommon-x11 wayland \
  vulkan-icd-loader vulkan-headers fontconfig freetype2 libxcb openssl zstd
```

Then install the pinned Zig 0.16.0 and put it on `PATH` (Ghostty's build calls a bare `zig`):

```sh
scripts/install-zig.sh
export PATH="$PWD/.tools/zig:$PATH"
cargo build --locked
cargo test --locked
cargo run -p cmux-app
```

Don't `cargo update` libc: it is pinned to 0.2.189 in `Cargo.lock` because xattr 0.2.3 needs `libc::ENOATTR`.

## Configuration

cmux-linux reads your Ghostty config (`~/.config/ghostty/config`) for fonts, colours,
opacity and split options such as `unfocused-split-opacity` and `split-divider-color`.

Keyboard shortcuts live in `~/.config/cmux-linux/config.toml`. The defaults are cmux's,
with ⌘ as Super. Override any action under `[keybindings]`:

```toml
[keybindings]
splitRight = "ctrl+shift+d"           # one chord
nextSurface = ["super+shift+]", "ctrl+tab"]  # several chords, any of them works
closeOtherTabsInPane = "none"         # "" or "none" unbinds
selectSurfaceByNumber = "alt+1"       # binds alt+1 … alt+9 (9 = last tab)
```

- Keys are cmux's action names: `copy`, `paste`, `splitRight`, `splitDown`,
  `toggleSplitZoom`, `equalizeSplits`, `resizePaneLeft/Right/Up/Down`,
  `focusLeft/Right/Up/Down`, `focusPreviousPane`, `focusNextPane`, `newSurface`,
  `closeTab`, `closeOtherTabsInPane`, `nextSurface`, `prevSurface`,
  `selectSurfaceByNumber`, `moveSurfaceLeft`, `moveSurfaceRight`, and for notifications
  `showNotifications` (Super+I), `jumpToUnread` (Super+Shift+U), `markAllNotificationsRead`
  and `clearAllNotifications` (both unbound).
- A chord is modifiers and a key joined by `+`. Modifiers: `super` (or `cmd`), `shift`,
  `alt` (or `opt`), `ctrl`. Keys: letters, digits, symbols, `left`/`right`/`up`/`down`,
  `return`, `tab`, `space`, `escape`, `f1`–`f20`. An array lists alternative chords. It is
  not cmux's two-stroke chord.
- Unknown actions and bad chords print a warning on stderr. That action keeps its defaults.
- The file is read at startup.

**KDE Plasma.** KWin handles some Super shortcuts before any app sees them. In a default
Plasma 6 setup these include Super+D (show desktop), Super+W (overview), Super+T (edit
tiles) and Super+Alt+Arrows (switch window). That blocks `splitRight`, `closeTab`,
`newSurface` and `focusLeft/Right/Up/Down`. Either free them in System Settings →
Keyboard → Shortcuts, or remap them here, for example:

```toml
[keybindings]
splitRight = "super+shift+backslash"
newSurface = "ctrl+shift+t"
closeTab = "ctrl+shift+w"
focusLeft = "ctrl+alt+shift+left"
focusRight = "ctrl+alt+shift+right"
focusUp = "ctrl+alt+shift+up"
focusDown = "ctrl+alt+shift+down"
```

## Notifications

A program asks for attention with OSC 9 (`printf '\e]9;Build done\a'`), OSC 777
(`\e]777;notify;Title;Body\a`) or OSC 99 (kitty's protocol). As in cmux, the pane gets a blue
ring, its tab a dot, and its workspace row the message and an unread badge; the row moves to
the top. The bell in the sidebar (or Super+I) lists them, Super+Shift+U jumps to the newest
unread one, and on Linux a desktop notification appears unless you are already looking at
that pane. Clicking it brings the window forward on that pane. Typing in a pane clears its ring.

If Plasma binds Meta+I or Meta+Shift+U on your system, remap them as shown above.

## CLI and socket

`cargo build` also builds `cmux`, cmux's command-line client. Keep it next to the
`cmux-app` binary. Every terminal then finds it first on its `PATH` and gets cmux's
variables: `CMUX_WORKSPACE_ID`, `CMUX_SURFACE_ID` (with their aliases `CMUX_TAB_ID` and
`CMUX_PANEL_ID`) and `CMUX_SOCKET_PATH`. Scripts and agent hooks written for cmux work
unchanged, for example a Claude Code hook in `~/.claude/settings.json`:

```json
{"hooks": {"Stop": [{"hooks": [{"type": "command",
  "command": "command -v cmux &>/dev/null && cmux notify --title 'Claude Code' --body 'Done'"}]}]}}
```

Ported commands: `notify`, `dismiss-notification`, `new-workspace`, `new-split`, `send`,
`send-key`, `list-workspaces`, `current-workspace`, `list-panes`, `list-pane-surfaces`,
`select-workspace`, `focus-pane`, `focus-panel`, `workspace-group`, `agent`, `ping`,
`version`, with
`--json` and
`--id-format refs|uuids|both`. `cmux --help` and `cmux <command> --help` list the flags.

The socket is `~/.local/state/cmux-linux/cmux.sock` (`$XDG_STATE_HOME/cmux-linux/` when
set). By default only processes started inside cmux-linux may use it. Set
`CMUX_SOCKET_MODE` before starting the app to change that: `allowAll`, `off`, or `password`
with the password on the first line of `~/.local/state/cmux-linux/socket-control-password`
(the CLI reads `--password`, then `CMUX_SOCKET_PASSWORD`, then that file).

Two things can keep a hook from reaching the app:
- A process that has left the app's process tree is refused in the default mode: a tmux or
  screen server, anything started with `setsid` or `nohup`. Use `allowAll` or `password` if
  your agents run there.
- Shells start as login shells, and a startup file that sets `PATH` outright (or puts
  another `cmux` first) hides ours. Check with `command -v cmux` in a new pane; if it isn't
  ours, link it into a directory early on your `PATH`, e.g. `ln -s "$PWD/target/release/cmux"
  ~/.local/bin/cmux`.

## Feed

The Feed is a right sidebar that collects what a coding agent is waiting on, so you can answer
from any workspace. Claude Code's permission prompts, plan approvals and questions appear as
cards, newest first; an answer given here goes back to Claude. You can still answer in Claude's
own terminal prompt instead; the card then stays pending until the 115 s wait ends and dims.
Open it with Super+Alt+B, or move focus into it with Super+Shift+E.

Install the hook once:

```sh
cmux hooks setup       # adds a PermissionRequest hook to Claude Code's settings
cmux hooks uninstall   # removes only that entry
```

The settings file is `--settings-file <path>`, else `$CLAUDE_CONFIG_DIR/settings.json`, else
`~/.claude/settings.json`. Both commands are idempotent; setup rewrites the file only when our
entry changes (a write re-indents to 2 spaces, key order is kept). Restart
running Claude Code sessions to pick the hook up.

What the buttons send back to Claude:
- **Permission:** Deny refuses, Allow Once allows this call, Always Allow and All tools allow
  it and apply Claude's own permission suggestions (so later calls aren't asked). The two send
  the same thing to Claude Code, as in cmux; All tools does not allow every tool.
- **Plan:** Manual approves and keeps asking for each edit, Auto approves and switches to
  auto-accept, Send feedback rejects the plan with your text.
- **Question:** pick an option for every question, then Submit All Answers (multi-select
  answers are joined with ", ").

If you don't answer within 115 s the card dims and Claude shows its own prompt as usual.
Closing the workspace ends the wait at once. A card also rings and notifies like any other
notification (see above).

The hook acts only inside cmux-linux terminals (they set `CMUX_LINUX=1`). Anywhere else, such
as a Mac terminal of cmux.app, it prints `{}` and does nothing, so it is safe to leave
installed.

Keys, all remappable under `[keybindings]`: `toggleRightSidebar` (Super+Alt+B),
`focusRightSidebar` (Super+Shift+E), `switchRightSidebarToFeed` (Ctrl+4, only while the
sidebar has focus; in a terminal it still selects tab 4, and a remapped binding also acts only
inside the sidebar). In the sidebar, j/k or Down/Up
choose a card, Enter jumps to its terminal and Escape returns to the terminal.

## Workspace groups

Groups are collapsible named sections in the sidebar. The first workspace of a group is its
**anchor**: the group's header stands for it, so it has no row of its own. Closing the anchor
promotes the next workspace; closing the last one removes the group.

- **Create:** Ctrl+Super+G makes a group with a new workspace. Right-click a workspace for
  New Empty Workspace Group, New Group from Workspace, Move to Group and Remove from Group.
- **Header:** click the name to go to the anchor, the chevron to collapse, the `+` (on hover)
  for a new workspace inside, in the anchor's directory. Right-click for New Workspace in
  Group, Rename Group…, Pin / Unpin Group (pinned groups stay on top), Ungroup Workspaces
  (keeps the workspaces) and Delete Group (closes them, after asking).
- **Inside a group:** Super+N adds the new workspace to the group, after the current one.
  Ctrl+Super+. collapses or expands the current workspace's group. Moving a workspace with
  the keyboard keeps it inside its group; drag it above a member to join a group, anywhere
  else to leave. Dragging a header moves the whole group.
- Selecting a workspace of a collapsed group expands it. Super+1–9 count only the plain
  visible rows, not headers or hidden workspaces.

From scripts:

```sh
cmux workspace-group create --name api --from workspace:1,workspace:2
cmux workspace-group new-workspace workspace_group:1 --placement end
cmux workspace-group list
```

`cmux workspace-group --help` lists the rest (ungroup, delete, rename, collapse, expand, pin,
unpin, add, remove, set-anchor, focus). Groups are not saved across restarts yet.

Not ported from cmux: group colours and icons, per-directory group settings, grouping a
multi-selection (the sidebar has none), renaming through a command palette (the header is
edited in place instead).

## Dock

The right sidebar has two tabs, Feed and Dock. The Dock holds terminals that belong to the
window, not to a workspace, so they stay put while you switch workspaces: a git view, logs, a
test watcher. It uses the same tabs, splits and shortcuts as the main area; while a Dock
terminal has focus, Super+T, Super+D, Super+Shift+D, Super+W and the pane-focus keys act on
the Dock. Closing the sidebar hides the Dock but its terminals keep running.

Open it with Super+Alt+B and click Dock, or Super+Shift+E then Ctrl+5 (`switchRightSidebarToDock`,
like Ctrl+4 only while the Feed has focus). Drag the sidebar's left edge to resize it. An
empty Dock offers New Terminal.

To start the same terminals every time, write `~/.config/cmux-linux/dock.json`:

```json
{"controls": [
  {"id": "git", "command": "lazygit", "cwd": "projects/app"},
  {"id": "logs", "command": "tail -f /var/log/syslog"}
]}
```

The first time the Dock is shown, each control gets a pane below the previous one and its
`command` is typed into a shell started in `cwd` (relative to your home directory). When the
command ends you are left in that shell.

Not ported from cmux: browser controls, `height`, `env` and `title` in `dock.json`, project
`.cmux/dock.json`, `--placement dock` and Dock rows in the CLI, notifications from Dock
terminals (`cmux notify` and the Feed hook don't work inside the Dock yet), dragging tabs
between the Dock and the main area.

## Agent messages

`cmux agent message` leaves a message for the agent in another workspace. It is handed over
through that agent's hooks, never typed into its terminal, so it can't land in a half-typed
prompt or press Enter for anyone.

```sh
cmux agent message builder "The relay fix is on main; rebase when free."
cmux agent message workspace:4 --from reviewer "Review posted on #123."
echo "long text" | cmux agent message surface:12 -
cmux agent message --reply-to <message-id> "Done."
cmux agent inbox                 # latest 50, newest first
cmux agent inbox --state queued
```

The target is a workspace or surface id or ref, or a workspace title (exact, then a unique
prefix). A workspace means the terminal whose agent last asked for messages there, else its
focused terminal. `--from` defaults to the sending workspace's title.

Delivery needs the hooks from `cmux hooks setup` (run it again if you installed them before
this feature; it now adds `UserPromptSubmit`, `PostToolUse` and `Stop` entries for Claude
Code). A busy Claude sees a message after its current tool call, or continues with it instead
of stopping; an idle one gets it attached to the next prompt you send. A message is `queued`,
then `delivered` when the agent has been shown it, then `read` when that turn ends.

Limits: text only (no control characters except newline and tab), 32 KiB, sender name one
line of at most 64 characters. Messages are kept in memory and are lost when the app quits.

Not ported from cmux: waking an idle agent, turning messages off per agent or workspace,
Codex and Kiro delivery.

## Workspace auto-naming

Off by default. When on, the end of each Claude Code turn may rename the workspace to a 2-5
word title summarised from the conversation, so several agent workspaces stop looking alike.

```toml
# ~/.config/cmux-linux/config.toml
[auto_naming]
enabled = true
# Optional. A shell line that reads the prompt on stdin and prints the title.
# The default is a tool-less `claude -p --model haiku` call.
# command = "my-summarizer --short"
```

It needs the hooks from `cmux hooks setup` (the `Stop` hook reports the turn). The setting is
read at every turn end, so no restart is needed. The command runs on your own account, in the
temp directory, with no `CMUX_*` variables and a 60 second limit; if it fails, the name stays.

A name you set yourself is never replaced; clear it to let auto-naming back in. To keep calls
rare, sessions under 12 transcript lines are skipped, and a new name needs 6 new lines and 3
minutes since the last try.

Not ported from cmux: naming the agent's tab, and agents other than Claude Code.

## Automations

Rules in `~/.config/cmux-linux/automations.json` run actions when something happens in the app.

```json
{
  "version": 1,
  "rules": [
    {
      "id": "agent-done-in-build",
      "when": { "event": "agent.hook.Stop" },
      "where": { "workspace.title": { "prefix": "build" } },
      "rate_limit": { "interval_seconds": 30, "maximum": 1 },
      "then": [
        { "action": "notify", "title": "Agent finished", "body": "in {{payload.workspace_title}}" },
        { "action": "run", "command": "paplay /usr/share/sounds/freedesktop/stereo/complete.oga" }
      ]
    }
  ]
}
```

Events: `workspace.created`, `workspace.closed`, `notification.created` (`title`, `subtitle`,
`body`), `feed.item.received` (`source`, `request_id`, `title`), `agent.message.queued` (`id`,
`sender_name`, `body_length`) and `agent.hook.Stop` (`source`, `transcript_path`; needs
`cmux hooks setup`). Every payload also has `workspace_id`, `workspace_title` and, when there
is one, `surface_id`.

- `when.event` and `when.category` (the name's first part) take a name or a list; `*` at
  either end is a wildcard.
- `where` compares payload values, ignoring case: a plain value must be equal, or use an
  object of `equals`, `contains`, `prefix`, `suffix`, `in`, `not`. `workspace.title` (or
  `title`), `agent` and `surface.kind` are cmux's aliases.
- `rate_limit` defaults to one firing a second. `"enabled": false` keeps a rule without
  running it.
- Actions run in order, each after the last has ended. `notify` takes `title`, `subtitle` and
  `body` with `{{event.name}}`, `{{event.category}}`, `{{event.source}}` and
  `{{payload.<key>}}`. `run` takes `command` for `sh -c` (in the temp directory, 60 seconds,
  `timeout_seconds` up to 300) with the event as JSON in `CMUX_AUTOMATION_EVENT`, plus
  `CMUX_AUTOMATION_EVENT_NAME`, `CMUX_AUTOMATION_EVENT_CATEGORY` and `CMUX_AUTOMATION_RULE_ID`.

The file is read at start. `cmux automation reload` reads it again and `cmux automation list`
shows the loaded rules, how often each fired, and any rule left out with the reason.

Not ported from cmux: the `rpc` and `webhook` actions (call `cmux ...` or `curl` from `run`;
a rule using them is left out), `workspace.tag`, `cmux events`, and `automation
show|test|enable|disable|logs`. A notification made by a rule triggers no rule, but a `run`
that calls `cmux notify` can: give such rules a rate limit.

## Contributing

Issues and ideas are welcome. Before working on code, read [`STATUS.md`](STATUS.md)
and the current milestone's spec. Contributions are accepted under
GPL-3.0-or-later.

## License

cmux-linux is free software, licensed under the
[GNU General Public License v3.0 or later](LICENSE).

It is a port of cmux, whose application code is GPL-3.0-or-later. Code taken or
adapted from cmux keeps its original copyright. Third-party components and their
licences are listed in [`NOTICE`](NOTICE).

cmux-linux is an independent community project. It is not affiliated with or
endorsed by Manaflow, Inc. or the cmux project. "cmux" is used only to describe
compatibility.
