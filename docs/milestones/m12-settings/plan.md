# M12 settings — Plan

**Goal:** the spec's Settings window, in three parts: live config apply first (it is what exits 1 and 3 hinge on), then the file writers, then the window.

Branch `m12-settings`, cut from `master`. Reference: cmux @ `88b87c05`, `Packages/macOS/CmuxSettingsUI/Sources/CmuxSettingsUI` (sui): `Scene/SettingsWindowScene.swift`, `Scene/SettingsSidebarList.swift`, `Chrome/*`, `Sections/ThemesSection.swift`, `Sections/TerminalThemeGallery.swift`, `Sections/TerminalGhosttyOptionsCard.swift`, `Sections/KeyboardShortcutsSection.swift`, `Rows/ShortcutRecorderView.swift`, `Rows/ShortcutListRowView.swift`, `Sections/AutomationSection.swift`; `Sources/App/SettingsWindowFactory.swift`; `CmuxFoundation/.../TerminalThemes/CmuxManagedThemeBlock.swift`.

## Part 1: live apply (exit 1, exit 3 "at once")

| | Decision | Why |
|---|---|---|
| A1 | The existing 1 s `scale_loop` becomes a config loop: each tick it stats `config.toml` and every Ghostty file the last load read (main file, includes, the theme file); on any mtime change it reloads what that file feeds. | One poller, as `ui_scale` already does. Hand edits apply live too. |
| A2 | Ghostty change: build a new `TermStyle` (`style.rs`), hand the new `Rc<TermStyle>` to every `WorkspaceView` (and the Dock), which pass it to each `TerminalView::set_style`: swap the Rc, `apply_to(term)` (colours, palette, cursor), `cx.notify()`. Font family and size take effect on the next layout (cell size is measured from the style each frame, `terminal_element.rs::measure`). Window: `set_background_appearance` and the root fill follow the new opacity; blur is dropped or re-enabled by replacing `WindowView::_blur`. | `TermStyle` is built once in `main.rs` today. Swapping the Rc keeps every reader as it is. |
| A3 | Appearance: the window observes `window_appearance` changes; when the theme is a `light:X,dark:Y` pair and dark/light flips, the style is rebuilt with the new `Env.dark`. | `dark` is set once at start today; cmux's slots follow the appearance. |
| A4 | `config.toml` change: `ui_scale` as now; keybindings: `cx.clear_key_bindings()` then `actions::bind` with the new table (warnings logged); `[[vault.agents]]` reloaded into the vault controller. `auto_naming` is already read per turn. | gpui 0.2.2 has `clear_key_bindings`; all our bindings come from `actions::bind`, so clearing loses nothing. |

Tasks: 1 config loop with mtimes (pure "which files changed" helper with a test); 2 `set_style` through the view tree, background and blur; 3 appearance flip; 4 keybindings and agents reload, with a test that a rebound table resolves the new chord; 5 check on KDE: edit the Ghostty file and `config.toml` by script, screenshot before/after, simulate the new chord.

## Part 2: writers (exit 2, exit 3 "survives a restart")

| | Decision | Why |
|---|---|---|
| W1 | Ghostty writes go to the user's own main file (the one the loader found: `config.ghostty`, else `config`; created if neither exists), never to an include. Per key: the last line setting it in that file is rewritten in place (`key = value`), other lines setting it are left; absent keys are appended at the end under a `# cmux-linux` line written once. Comments, blank lines, `config-file` lines and unknown keys are untouched byte for byte. Written to a temp file and renamed; the first write of a session keeps a copy as `<file>.cmux-bak`. | The spec asks for the user's file (cmux writes its own file loaded after). Last line wins in Ghostty, so rewriting the last one is the value Ghostty will use. |
| W2 | When a key's effective value comes from an include or a later file, the row shows "Set in <file>" and the control writes to the main file anyway, with a note that the include wins if it comes later. | cmux's "Overridden by your config" note. |
| W3 | Keys written: `theme` (`light:X,dark:Y` when both slots differ, else `X`), `background-opacity` (2 decimals), `background-blur` (`true`/`false`), `font-family` (one line; earlier `font-family` lines in the main file are commented out with `# ` so the list is one font), `font-size`, `cursor-style`, `cursor-style-blink`, `unfocused-split-opacity`, `unfocused-split-fill`, `split-divider-color`. Pure, `cmux-config::ghostty::edit` with tests on a fixture that has comments, includes, unknown keys, repeated keys and CRLF. | Spec's list. Ghostty's own `+validate-config`, if `ghostty` is on `PATH`, runs in the check task, not at runtime. |
| W4 | `config.toml` writes use `toml_edit` (0.22, already in Cargo.lock via `toml`; added to `NOTICE`): `ui_scale`, `[keybindings]` entries (a list when more than one chord, `"none"` to unbind, key removed on reset), `[auto_naming] enabled/command`, `[[vault.agents]]`. Comments and other tables are kept. Temp file and rename. | Comment-preserving TOML without hand parsing. |

Tasks: 1 `ghostty::edit` with fixture tests; 2 `config.toml` writer with tests; 3 check: write via the writers on a copy of a real config, `ghostty +validate-config` if installed, diff shows only the changed lines.

## Part 3: the Settings window (all exits)

| | Decision | Why |
|---|---|---|
| S1 | A second gpui window, 980×680, title "Settings", opened by `openSettings` (Super+,, remappable) and a "Settings…" item in the sidebar's blank-area context menu. A second open focuses the existing one. Closing the main window also closes Settings (the app quits as now). | sui `SettingsWindowFactory`; gpui keeps running while any window is open. |
| S2 | Layout: sidebar 210 px with group headings and rows (16 px icon + title, 10 px gap), no search; detail a scrolling column, padding 20, spacing 14; section header 13 semibold secondary; card radius 13, fill control background at 0.76, 1 px separator at 0.5; rows label left / control right, padding 14×9, title 13 medium, subtitle 10 secondary (the Ghostty key name for Ghostty rows); controls 200 px wide. Colours from `chrome`. | sui `Chrome/*`, `SettingsCardRow.swift`. |
| S3 | Sections (cmux names, only ours): General → **App** (`ui_scale` stepper 0.5–2.0 by 0.05), **Themes**; Terminal → **Terminal**; Agents & Automation → **Automation**; Keyboard & Advanced → **Keyboard Shortcuts**. | cmux's taxonomy minus Cloud, Browser, Mobile and the rest. |
| S4 | Themes: the current pair as "Light: X · Dark: Y" with a Light/Dark slot toggle; a search field; a grid of cards (adaptive 128–220 px, gap 10, 54 px high: theme background, name 10 px mono in the theme foreground, cursor block 5×11, 2×8 ANSI swatches 8 px high, gap 2, radius 2), grouped "Dark Themes"/"Light Themes" (by background luminance), the current slot's group first; selected card accent 2 px border and 0.12 fill, radius 8. A click writes `theme` at once. Themes listed from the loader's theme dirs (`<config>/themes`, `/usr/share/ghostty/themes`, …), parsed lazily on first open. | sui `TerminalThemeGallery.swift`. No System/Light/Dark app appearance row (the app follows the desktop). |
| S5 | Terminal: Font (dropdown of installed monospace families, `cx.text_system().all_font_names()` filtered by an `M` vs `i` advance test), Font Size (stepper, 1 pt), Cursor Style (segmented: block, bar, underline, block_hollow), Blinking Cursor (toggle), Background Opacity (slider 0–1 step 0.05, 140 px + 44 px percent label, writes on release), Background Blur (toggle), Unfocused Split Opacity (slider 0.15–1), Unfocused Split Fill and Split Divider Color (hex text field with a swatch). Writes debounced 250 ms. | sui `TerminalGhosttyOptionsCard.swift`; the split rows are ours (cmux has none). |
| S6 | Keyboard Shortcuts: every `DEFAULTS` action, titled from M11's palette table; a 160 px recorder button per row showing the chords; click → "Press shortcut…", the next chord with a modifier is recorded (Escape cancels, a bare key is rejected with a red note), Unbind (×) / Restore (↺) button, "Reset Default Shortcuts" at the top; a chord already bound to another action shows the red conflict banner with Undo. Writes `config.toml` at once. While recording, key bindings are not dispatched (the window captures the keystroke first). | sui `KeyboardShortcutsSection.swift`, `ShortcutRecorderView.swift`. No keymap presets or chords of two strokes. |
| S7 | Automation: Auto-naming enabled (toggle) and command (text field); Agents: one card per `[[vault.agents]]` with text fields id, name, process_name, argv_option, resume_command, a Remove button, and "Add Agent". | Spec lists both; cmux's Automation section is Claude/Codex-specific. |
| S8 | Text fields reuse `rename.rs`'s key state machine; slider, stepper, toggle, segmented and dropdown are small private elements in `settings/controls.rs`. | No gpui widget library in the tree. |

Tasks: 1 window shell, sidebar, sections routing, open action and menu item; 2 controls; 3 App and Terminal sections; 4 Themes gallery; 5 Keyboard Shortcuts with recorder; 6 Automation; 7 check on KDE: open with Super+, by `debug.shortcut.simulate`, change ui_scale, theme, blur and opacity, screenshot the main window after each (exit 1); diff the Ghostty file and run `ghostty +validate-config` if available (exit 2); rebind `newSurface`, press it, restart the app, press it again (exit 3).

## Deferred (with reason)

- Settings search, keymap presets, two-stroke chords, cmux.json section, reset-all page: beyond the spec.
- The other Ghostty keys: out of scope by the spec; they stay as written.
- Live font change for the Dock and restored sessions uses the same path; per-terminal font overrides do not exist.
