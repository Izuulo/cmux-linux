# m12-settings — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-11 — plan
- Read cmux @ `88b87c05` Settings (`CmuxSettingsUI`): 980×680 window, 210 px sidebar of grouped sections, cards of label/control rows; theme gallery of 54 px cards; keyboard recorder; Ghostty options card. cmux writes Ghostty settings to its own `config.ghostty` loaded after the user's (theme in a `# cmux themes start/end` block); the spec asks for the user's own file, so we edit that file in place (plan W1). Deviation.
- Today only `ui_scale` applies live (1 s poll); `TermStyle` is built once, blur enabled once, keybindings and vault agents loaded once. Part 1 makes them live. gpui 0.2.2 has `clear_key_bindings`, and every binding comes from `actions::bind`.
- `/usr/share/ghostty/themes` has the bundled themes here; `theme = light:X,dark:Y` already parses (`pick_theme`).
- `toml_edit` 0.22 is already in Cargo.lock (via `toml`).

## 2026-11-11 — Part 2: writers
- `cmux_config::ghostty::edit`: `main_file(env)`, `set(path, key, value)`, `set_all(path, &[(key, value)])`, pure `edit(text, changes)`, `theme_value(light, dark)`. Last line per key rewritten in place (keeps its CRLF/LF), new keys appended under one `# cmux-linux` line in the file's line ending, BOM kept, earlier `font-family` lines get `# `. `config-file`, keys with `=`/`#`/whitespace and values with newlines are refused (no line injection). First write per file per process copies `<file>.cmux-bak`.
- `cmux_config::ghostty::origin(env, key) -> Option<(PathBuf, line)>`: the user file/line whose value wins, for W2's "Set in <file>". The loader now finds its main file through `edit::main_file`.
- `cmux_config::write`: `set_ui_scale` (2 decimals), `set_keybinding(action, &[chord])` (`[]` → `"none"`, one → string, more → list), `reset_keybinding`, `set_auto_naming(enabled, Option<command>)` (None/blank removes `command`), `set_agents(&[Registration])` (`[[vault.agents]]` under an implicit `[vault]`; empty removes them; `name` written only when it differs from `id`). A `config.toml` that doesn't parse is reported (`InvalidData`) and never overwritten.
- Learned: `toml_edit`'s `insert` replaces the key with its comments; `put` writes through `get_mut` and keeps the value's trailing comment. Both files go through `write::replace_file`: temp + rename, permissions copied, a symlinked file (dotfile managers) written at its target.
- Deviation: adding `toml_edit` as a direct dep adds one edge to Cargo.lock (no new package, no version change), so this commit carries a one-line Cargo.lock change; `--locked` passes after it.
- Check (task 3): copied the owner's Ghostty and cmux-linux config to a scratch XDG dir, wrote all W3 keys plus a second `font-size` write and the four config.toml writers. `diff -u` shows only the changed/appended lines; the loader reads them back with no warnings; `ghostty +validate-config` (1.4.0) exits 0 on the result (and 1 on a deliberately bad value, so it does check). Only `cargo test/clippy/fmt -p cmux-config` were run here (vendor/ghostty not initialised in this worktree); the change is additive to cmux-config's API.
