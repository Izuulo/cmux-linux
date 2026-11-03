# M1 terminal — Spec

**Goal:** one fully usable terminal in a gpui window, good enough to run Claude Code all day.

## Scope
- PTY spawn of the user's shell; resize on window resize.
- gpui terminal element drawing the cell grid: fonts, bold/italic, colours, cursor styles, wide chars, ligatures if the engine reports them.
- Ghostty config: font family/size, theme, palette, background opacity, cursor style.
- Keyboard (incl. kitty keyboard protocol if the engine supports it), IME, mouse reporting, selection, copy/paste (Wayland clipboard), scrollback with scroll wheel.
- Reference: Zed's `terminal_view` crate (GPL-3) for gpui patterns.

## Out of scope
Splits, tabs, sidebar, notifications.

## Exit criteria
1. Claude Code, vim, htop and `git log` render and behave correctly.
2. Colours/fonts match Ghostty with the same config (screenshot pair in journal).
3. Typing latency feels the same as Ghostty (no measurable lag at 60 Hz).
