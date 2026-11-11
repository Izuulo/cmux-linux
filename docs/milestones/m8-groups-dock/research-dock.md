# Upstream Dock research (cmux macOS, GPL-3.0-or-later)

Source: upstream commit `88b87c05` (2026-11-11). Paths are relative to the cmux repo root.
"(inferred)" marks conclusions not stated verbatim in code. Browser/Chief parts skipped.

## 1. Right sidebar modes

1. Enum `RightSidebarMode` (`Sources/RightSidebarMode.swift`), case order = default tab order: `files` "Files" `folder`; `find` "Find" `magnifyingglass`; `sessions` "Vault" `books.vertical`; `feed` "Feed" `dot.radiowaves.left.and.right`; `dock` "Dock" `dock.rectangle`; `machines` "Cloud" `cloud`; `customSidebar` (raw `custom-sidebar`) "Custom" `wand.and.stars`.
2. Availability (`RightSidebarMode+Availability.swift` `isAvailable`): files/find/sessions/dock always; feed, machines, customSidebar are feature-gated. Dock cannot be "opened as pane" (`paneModes`).
3. Order/hidden are user prefs: `RightSidebarTabPreferences` keys `rightSidebar.tabs.order`, `rightSidebar.tabs.hidden`. Mode-bar right-click menu: one toggle per mode + "Customize Tabs…" (`RightSidebarPanelView.tabCustomizationMenu`).
4. Default mode `.files`; persisted in UserDefaults `rightSidebar.mode` (`FileExplorerState.init`). Visibility persisted as `fileExplorer.isVisible` (default false).
5. Mode bar (`RightSidebarPanelView.modeBar`, `ModeBarButton` in `RightSidebarChromeStyle.swift`, metrics in `Sources/WindowChromeMetrics.swift`):
   - Bar height = window titlebar height, 28pt default (`WindowChromeMetrics.sharedChromeBarHeight`); leading padding 4, trailing 6, 4pt gap between controls, 1px bottom border.
   - Tab = pill: height 20 (`RightSidebarChromeMetrics.controlHeight`), horizontal padding 8, corner radius 6, icon 11pt + label 11pt regular weight, icon/label gap 4.
   - Colours: foreground `Color.secondary` at opacity 0.86 idle / 0.96 hover / 1.0 selected; selected fill `primary` 10%, hover fill `primary` 5%, otherwise clear.
   - Selected tab always shows its full label; unselected tabs share leftover width and collapse to icon-only (label hidden under ~15pt) (`RightSidebarModeBarTabWidths`). One selection pill slides between tabs (spring 0.28 / damping 0.9).
   - Trailing: "open as pane" icon (`rectangle.split.2x1`, disabled in Dock, tooltip "Dock can’t open as a pane") and close `xmark`; both 20x20 with 10pt icons.
   - Only Feed gets a badge chip (pending count). Dock has no mode-bar badge.
6. Shortcuts (`Sources/KeyboardShortcutSettings.swift`, `Action` raw value = case name unless noted):
   - `toggleRightSidebar` (raw value `toggleFileExplorer`): Cmd+Opt+B, "Toggle Right Sidebar".
   - `focusRightSidebar`: Cmd+Shift+E, "Toggle Right Sidebar Focus".
   - `switchRightSidebarToFiles|Find|Sessions|Feed|Dock|Machines`: default Ctrl+N where N is the 1-based position among *visible* tabs (`rightSidebarPositionalDefaultShortcut`); hidden or unavailable tab, or position > 9, is unbound. Dock is Ctrl+4 without Feed, Ctrl+5 with it.
   - A mode shortcut shows the sidebar, selects the mode and focuses its first item (`AppDelegate.focusRightSidebarInActiveMainWindow(mode:focusFirstItem:)`).

## 2. Right sidebar width

1. Stored default 220 (`FileExplorerState.init`), but the floor is 295 (`RightSidebarWidthSettings.minimumWidth`), so the effective default is 295 (inferred from clamp).
2. Clamp (`ContentView.clampedRightSidebarWidth`, `ContentView+RightSidebarWidth.swift`): min = max(295 or a lower user floor in 160...295, mode-bar minimum width); max = min(1200 built-in or user max up to 4096, window width - 360). 360 is the minimum main terminal width kept (`minimumTerminalWidthWithRightSidebar`).
3. Resizable by dragging the divider (`ContentView` `.explorerDivider`): width = start - translation, clamped live, no animation.
4. Persisted on drag end to UserDefaults `fileExplorer.width` (app-wide, not per window). A hidden sidebar keeps its width and is re-clamped when shown.

## 3. Dock scopes

1. `DockScope` (`Sources/DockScope.swift`): `workspace` and `global`. `global` now means **per-window** Dock; the store's `workspaceId` is the window id (`AppDelegate+WindowDock.swift`).
2. The right sidebar renders **only the window Dock**: `RightSidebarPanelView.dockPanel` calls `AppDelegate.windowDock(for: tabManager)` with `rootDirectory: nil`.
3. The per-workspace Dock (`Workspace.dockSplit`) is legacy: kept for session restore and socket topology, never rendered ("the sidebar never renders one", `AppDelegate.focusedDockStoreForShortcut`). There is no user-facing scope switch.
4. So the visible Dock survives workspace switches inside a window (terminals keep running) and is torn down with the window (`MainWindowContext.teardownWindowDock` -> `DockSplitStore.retire`).
5. Dock state is in the session snapshot per window (`windowDockSessionSnapshot`); a restored snapshot, even an empty one, wins over config seeding.

## 4. Dock container

1. `DockSplitStore` owns its own `BonsplitController` and is its `BonsplitDelegate`; panels are ordinary `TerminalPanel`s rendered through the main-area panel view (`DockSplitContentView`, `DockSplitPanelContentView`). Same tab bar, splits, drag, context menu.
2. Config (`DockSplitStore.makeConfiguration`, `DockSplitStore+Appearance.swift`): splits on, tab reorder and cross-pane move on, `allowCloseLastPane: false`, `autoCloseEmptyPanes: true`, `contentViewLifecycle: .keepAllAlive`, `newTabPosition: .current`, animations off, tab bar height 28.
3. Minimum pane size 48pt in both axes (`minimumDockPaneSize`) versus Bonsplit's 100pt default.
4. Empty pane (`DockEmptyPaneView` in `DockPanelView.swift`): `dock.rectangle` icon 30pt tertiary, text "Empty Dock Pane" (12pt semibold, secondary), bordered small buttons "New Terminal" (`terminal.fill`), "New Browser", "New Chief" (the last two feature-gated).
5. `DockEmptyView` ("No Dock Controls" / "Add controls to .cmux/dock.json." / "Copy Agent Prompt" / "Docs") exists but nothing references it.
6. No dedicated Dock toolbar: `DockSplitStore.newInFocusedPane` (commented as the toolbar "+" menu) has no callers. Creation comes from the Bonsplit tab bar: new-tab request -> `splitTabBar(_:didRequestNewTab:inPane:)`; split button -> `didSplitPane`, which seeds a terminal in the new pane (inheriting the source panel's cwd).
7. Last terminal closes: its pane auto-closes unless it is the last pane; the last pane stays and shows the empty-pane view. The sidebar is not hidden (inferred: no code hides it).
8. Shell exit (Ctrl-D) closes the tab without confirmation or history (`AppDelegate.closeWindowDockRuntimeSurface`). Close button/shortcut use the normal running-process confirmation (`DockSplitStore+CloseConfirmation.swift`).
9. Tabs can be dragged between main area and Dock (`onExternalTabDrop` -> `AppDelegate.moveSurfaceIntoDock`); the live panel moves, it is not copied.

## 5. Focus rules

1. Dock owns shortcuts when `keyboardFocusCoordinator.activeRightSidebarMode == .dock` (`AppDelegate.focusedDockStoreForShortcut`).
2. Into the Dock: click a pane (`focusPaneFromDockInteraction`), a mode shortcut, or `focusRightSidebar`. Entry focuses the focused pane's selected panel (`DockSplitStore.focusFirstControl`).
3. `focusRightSidebar` is a toggle (`MainWindowFocusController.toggleRightSidebarOrTerminalFocus`): if the sidebar owns focus it **hides the sidebar** and restores main-panel focus; otherwise it shows the sidebar and focuses it.
4. Main and Dock focus rings are mutually exclusive (`rightSidebarOwnsInputFocus`).
5. Dock-scoped actions (`KeyboardShortcutSettings.Action.dockShortcutRoutingDisposition`, exhaustive): newSurface, splitRight, splitDown, newPaneAutoLayout, closeTab, closeOtherTabsInPane, nextSurface, prevSurface, selectSurfaceByNumber, moveSurface*, focusLeft/Right/Up/Down, focusPreviousPane, focusNextPane, toggleSplitZoom, resizePane*, equalizeSplits, renameTab, find/findNext/findPrevious/hideFind, toggleTerminalCopyMode, clearScreenKeepScrollback, triggerFlash, focusHistory*. Everything else targets the main container.
6. Routing: `routeCreateToFocusedDock` (new tab in focused Dock pane), `routeSplitToFocusedDock`, `closeFocusedDockPanelForCommand`, `performFocusedDockShortcut` (`DockShortcutCommand`). A failed Dock command beeps; it does not fall through to the main area.
7. Low-level `newSurface`/`newSplit` do not steal keyboard focus; only explicit Dock affordances do (`newSurfaceFromDockAffordance`). Socket creation defaults to `focus: false`.
8. Closing the sidebar (`AppDelegate.toggleRightSidebarInActiveMainWindow`): focus returns to the selected workspace's terminal. Dock terminals **keep running**: `setActive` -> `setVisibleInUI(false)` only hides and unfocuses views (`applyVisibility`). Same when switching to another mode.

## 6. Dock terminal environment

1. Standard vars (`TerminalSurface+StartupEnvironment.swift`): `CMUX_SURFACE_ID` = `CMUX_PANEL_ID` = surface id; `CMUX_WORKSPACE_ID` = `CMUX_TAB_ID` = owner id, which for the window Dock is the **window id**; plus `CMUX_TERMINAL_LIFECYCLE_ID`, `CMUX_SOCKET_PATH`.
2. Dock-specific, config controls only (`DockSplitStore.makeTerminalPanel`): `CMUX_DOCK_CONTROL_ID`, `CMUX_DOCK_CONTROL_TITLE`, plus the control's `env`. No scope variable.
3. cwd for an ad-hoc terminal (`resolvedTerminalStartupWorkingDirectory`): requested cwd, else the source panel's live cwd if the "inherit working directory" setting is on, else the Dock base directory (home for the window Dock).
4. Ad-hoc terminals run a plain shell; a socket `initial_command` is passed straight through.
5. Config controls use `DockSplitStore.shellStartupScript` (`DockSplitStore+TerminalStartup.swift`): a temp `cmux-dock-control-<uuid>.sh` (mode 0700, deletes itself) that
   - resolves the login shell (`dscl` UserShell, else `$SHELL`, else `/bin/sh`) and exports `SHELL`;
   - runs `"$shell" -lc '<prepend bundled CLI dir to PATH>; cd "$CMUX_DOCK_START_DIRECTORY" 2>/dev/null || true; eval "$CMUX_DOCK_START_COMMAND"'` (fish: `"$shell" -l -c '...; cd ...; eval "$CMUX_DOCK_START_COMMAND"'`);
   - then `printf '\n'` and `exec "$shell" -l`. Command and cwd are embedded base64 and passed as env vars `CMUX_DOCK_START_COMMAND`, `CMUX_DOCK_START_DIRECTORY`, `CMUX_DOCK_BUNDLE_BIN`. Write failure falls back to `/bin/sh`.

## 7. dock.json

1. Schema (`DockConfigFile`, `DockControlDefinition`): `{ "controls": [ ... ] }` (required). Control: `id` (required, trimmed, non-blank), `title` (default = id), `type` (`terminal` default | `browser` | `chief`, case-insensitive), `command` (required for terminal), `url`, `chrome`, `cwd`, `height` (number), `env` (string map, default `{}`).
2. Errors (NSError domain `cmux.dock`): 1 "Dock control ids must be unique."; 2 "Dock control id must not be blank."; 3 "Dock control type must be terminal, browser, or Chief."; 4 "Dock control command must not be blank."; 5 "Dock browser control url must not be blank.". Any other failure: "Could not load the Dock config. Check dock.json and try again.". Shown in `DockErrorView` titled "Dock Config Error".
3. Lookup (`DockSplitStore.resolve(scope:rootDirectory:)`): **no cross-scope fallback**.
   - `global` (the rendered Dock): only `~/.config/cmux/dock.json`, base = home.
   - `workspace` (legacy): `.cmux/dock.json` walking up from the workspace directory, stopping at home or `/`; base = the directory containing `.cmux`. `docs/dock.md` still describes "project first, then global"; the code does not do that.
4. Relative `cwd` joins onto the base; absolute stays (`resolvedWorkingDirectory`).
5. `height` (`DockSplitStore.seed`): controls stack top-to-bottom via vertical splits. Each split's divider = previous height / sum of heights from previous onward, clamped 0.1...0.9. Missing height = 200. Heights are ratios only.
6. Seeding runs on first visible activation of Dock mode (`setActive` -> `ensureLoaded`), off the main thread, and only if no seed/restore/explicit creation happened first (`hasAppliedConfigurationSeed`, `recordExplicitPanelCreation`).
7. Reload: `DockSplitStore.reload()` closes all panels and reseeds. Only callers are `trustAndReload` and a config identity change on re-show (`reloadIfBaseDirectoryChanged`). No menu item, shortcut or socket method was found. `openConfiguration()` (creates `{"controls":[]}` and opens it) also has no caller.
8. Trust gate (`trustRequestIfNeeded`): project sources only, so in practice only the legacy workspace Dock; the global file is never gated.
   - Fingerprint (`CmuxActionTrustDescriptor.fingerprint`, `Sources/CmuxActionTrust.swift`): SHA-256 hex of sorted-keys JSON of `{schemaVersion: 1, actionID: "cmux.dock", kind: "dockControls", command: <sorted-keys JSON of the re-encoded controls>, target: "rightSidebarDock", configPath, projectRoot}`. Terminal controls re-encode without `type`.
   - Store: JSON array of fingerprints in `~/Library/Application Support/cmux/trusted-actions.json`.
   - View (`DockTrustView`): "Trust Project Dock?", "This project wants to start commands from its Dock config.", the config path, button "Trust and Start".

## 8. Notifications and attention

1. Window Dock terminals are full notification owners (`AppDelegate.liveSurfaceOwner`, `notificationSurfaceOwner`): owner id = window id, surface id = panel id.
2. Bell (`DockSplitStore.installAttentionRouting`): marks the surface unread unless the Dock pane is focused and active in the key window, then flashes the pane.
3. Unread Dock panels draw the attention ring (`DockUnreadPanelProjection` -> `hasUnreadNotification`); manual mark-unread is supported (`manualUnreadPanelIds`).
4. Opening a notification reveals the sidebar in Dock mode and focuses the panel (`AppDelegate.openWindowDockUnread`). Typing or clicking in the terminal clears it.
5. Window Dock unread counts toward the app badge (`TerminalNotificationStore.windowDockUnreadTargets`).
6. Legacy workspace Dock terminals get no visual attention (not renderable).

## 9. Socket / CLI

1. `cmux new-pane --placement dock` -> v2 `pane.create`; `cmux new-surface --placement dock` -> `surface.create` (`CLI/cmux.swift`). Param `placement`: `workspace` (default) | `dock`. Bad value: `invalid_params`, "placement must be one of: workspace, dock", data `{placement}`.
2. Other params used: `direction` (CLI default `right`), `type`, `workspace_id`, `window_id`, `surface_id` (split source), `pane_id` (surface.create), `focus` (default false), `initial_command`, `working_directory`, `initial_input`, `startup_environment`/`initial_env`, `initial_divider_position`.
3. Target is always the routed window's Dock (`TerminalController.dockPaneCreate`, `dockSurfaceCreate`). `workspace_id` may be the window id or the alias `D0CCD0CC-0000-4000-8000-000000000001`. An empty Dock places into the root pane, no split.
4. Response (`ControlCommandCoordinator+Pane.swift`, `+Surface.swift`, case `.createdDock`): `window_id`, `window_ref`, `workspace_id` (= window id), `workspace_ref`, `placement: "dock"`, `pane_id`/`pane_ref`/`surface_id`/`surface_ref` all null, `dock_pane_id`, `dock_surface_id`, `type`.
5. Errors: "Dock placement supports terminal and browser surfaces", "Dock placement is unavailable", "Dock could not be revealed", "Conflicting Dock routing selectors"; unknown explicit source surface -> `not_found` "No source surface to split".
6. `pane.list`, `surface.list` and tree rows add `dock_scope`: `"workspace"` | `"global"`; non-Dock rows are unchanged. CLI text appends `  [dock:<scope>]` (`dockScopeTag`). `cmux tree` shows global Dock rows only under the selected workspace (`includeGlobalDock`).
7. `send`, `send-key`, `read-screen`, `capture-pane`, `focus`, `close` accept a bare Dock surface id. `cmux right-sidebar set dock` selects the mode.

## Unknown / not found

- Bonsplit tab-bar "+" / split button titles: `vendor/bonsplit` submodule is not checked out. `docs/dock.md` names New Terminal, New Browser, Split Right, Split Down.
- Whether Feed is on by default (decides Dock = Ctrl+4 or Ctrl+5).
- Whether Ctrl+N mode shortcuts fire while a main-area terminal has focus (only the sidebar focus views were read).
- Default chords for split/close/focus-pane actions were not re-read; `docs/dock.md` gives Cmd+T, Cmd+D, Cmd+Shift+D.
- Whether a Dock notification marks any left-sidebar workspace row (owner id is a window id).
- Session snapshot format of the Dock (`DockSplitStore+SessionSnapshot.swift`) was not read.
