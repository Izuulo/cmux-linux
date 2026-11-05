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

> **Status: pre-alpha.** Milestone M0 (toolchain and foundations) is done.
> Nothing is usable as a terminal yet. See [`STATUS.md`](STATUS.md) for where things stand.

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
  `selectSurfaceByNumber`, `moveSurfaceLeft`, `moveSurfaceRight`.
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
