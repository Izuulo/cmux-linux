# m12-settings — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-11 — plan
- Read cmux @ `88b87c05` Settings (`CmuxSettingsUI`): 980×680 window, 210 px sidebar of grouped sections, cards of label/control rows; theme gallery of 54 px cards; keyboard recorder; Ghostty options card. cmux writes Ghostty settings to its own `config.ghostty` loaded after the user's (theme in a `# cmux themes start/end` block); the spec asks for the user's own file, so we edit that file in place (plan W1). Deviation.
- Today only `ui_scale` applies live (1 s poll); `TermStyle` is built once, blur enabled once, keybindings and vault agents loaded once. Part 1 makes them live. gpui 0.2.2 has `clear_key_bindings`, and every binding comes from `actions::bind`.
- `/usr/share/ghostty/themes` has the bundled themes here; `theme = light:X,dark:Y` already parses (`pick_theme`).
- `toml_edit` 0.22 is already in Cargo.lock (via `toml`).
