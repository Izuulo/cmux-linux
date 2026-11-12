# M12 settings — Spec

**Goal:** configure the app without editing files.

## Scope
- A Settings window (cmux's Settings is the reference: read its Swift source at milestone start
  and match layout and sections where they apply to the local app).
- cmux-linux options from `config.toml`: `ui_scale` (applied live), keybindings (record a chord,
  unbind, reset), auto-naming, agents.
- A Ghostty config editor with real controls: theme picker with preview (Ghostty's bundled
  themes, light/dark pair as `theme = light:X,dark:Y`), background blur on/off, background
  opacity, font family and size, cursor style, split options.
- Writes back to the user's Ghostty config file without losing comments, includes or keys the
  editor doesn't know; changes apply live where the app can (colours, opacity, blur, fonts).
- Opened from a shortcut (cmux's ⌘, → Super+,) and the sidebar.

## Out of scope
- Editing every Ghostty key: unknown keys stay as they are in the file.

## Exit criteria
1. Changing `ui_scale`, a theme, blur and opacity in Settings updates the running window without a restart.
2. After a save, the Ghostty config file keeps its comments, includes and unknown keys, and Ghostty itself still loads it.
3. A keybinding changed in Settings works at once and survives a restart.
