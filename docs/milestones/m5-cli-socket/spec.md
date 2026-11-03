# M5 cli-socket — Spec

**Goal:** agents and scripts drive cmux-linux the same way they drive cmux.

## Scope
- Unix socket server in `cmux-ipc`; `cmux` CLI binary.
- Same env vars as cmux: `CMUX_SOCKET_PATH`, `CMUX_SOCKET_PASSWORD`, `CMUX_WORKSPACE_ID`, `CMUX_SURFACE_ID`, `CMUX_TAB_ID`; set inside every terminal.
- Commands: `notify` (title/subtitle/body/workspace/surface/desktop/clear), `dismiss-notification`, new workspace, split, send keys, list, focus, `--help`, `--version`, `--json`, `--id-format`.
- Wire format and output follow cmux's `docs/cli-contract.md` for these commands.
- Reference: cmux `CLI/` and `docs/cli-contract.md`.

## Out of scope
All cloud, browser, VM, SSH, tmux-compat and hook-installer commands.

## Exit criteria
1. cmux's documented Claude Code hook setup works unchanged against cmux-linux.
2. Each implemented command's text and JSON output matches cmux's contract (golden tests).
