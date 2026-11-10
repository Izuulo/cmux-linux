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

Later: an agent approval feed, workspace groups, a dock, agent-to-agent
messages, session restore, a command palette and SSH workspaces.

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
| M7–M11 | Feed, groups and dock, agent coordination, persistence, productivity |

Each milestone has a spec, plan and journal under [`docs/milestones/`](docs/milestones).

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
`select-workspace`, `focus-pane`, `focus-panel`, `ping`, `version`, with `--json` and
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
