# M0 foundations — Spec

**Goal:** prove the toolchain and the two risky choices (gpui on KDE Wayland,
libghostty-vt bindings) before building on them.

## Scope
- Cargo workspace with the six crates from `docs/design.md` (empty stubs are fine except where below).
- Vendor cmux's `cmux-tui/crates/ghostty-vt-sys`, `ghostty-vt` and the `ghostty` submodule pin; record source commit in `vendor/SOURCE`.
- Pin a gpui version; open an empty window with KWin background blur on Wayland.
- Spike: feed a recorded byte stream (colours, cursor moves, OSC 7, OSC 9/777) to libghostty-vt and read back the cell grid and OSC events.
- **Engine decision:** if the spike can't read cells, styles and OSC events, switch to `alacritty_terminal`. Record the decision and evidence in the journal and `docs/design.md`.
- CI (GitHub Actions, Linux): build, test, clippy, and a check that `STATUS.md` is ≤100 lines.

## Out of scope
Any terminal rendering, input, or real UI.

## Exit criteria
1. `cargo run -p cmux-app` opens a blurred empty window on KDE Plasma Wayland.
2. `cargo test -p cmux-term` passes the byte-stream spike test.
3. Engine decision recorded.
4. CI green on the default branch.
