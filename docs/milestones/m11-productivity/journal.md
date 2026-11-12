# m11-productivity — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-11 — plan
- Read cmux @ `88b87c05`: palette (`ContentView.swift`, `CmuxCommandPalette` Swift matcher; nucleo FFI not ported), `cmux.json` (`CmuxConfig.swift`, executor, trust prompt), `cmux ssh` (default path is the cmux-tui daemon; the legacy `ssh -tt` + per-workspace startup command path is the model, minus cmuxd-remote and its relay). All GPL; BUSL only in `web/`, `workers/*`, `services/iroh-relay-minter/`, `cmux-tui/relays/cloudflare-do/`.
- Plan written; deviations listed there: palette runs every entry after closing, no recency or highlighting; `cmux.json` read on palette open from `~/.config/cmux-linux/` plus the project walk, `commands` only; ssh terminals run ssh as the PTY program over a ControlMaster, sidebar metadata by OSC 7 and a 5 s `ss`/git probe, no reconnect loop.
