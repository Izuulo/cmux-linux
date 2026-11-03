# M2 research: cmux split panes and per-pane tabs

Facts extracted from cmux at commit 21dcc2b. Nothing here is code to copy; it is a spec to match.

## 0. Sources and licence

- The split/tab implementation is **Bonsplit**, a separate repo pinned as the submodule `cmux/vendor/bonsplit` (`cmux/.gitmodules`, gitlink `58b73fde506ef6aa5fc371b8990e528ac83f0929`). The submodule was **empty** in the checkout, so I cloned `github.com/manaflow-ai/bonsplit` at that exact commit. Bonsplit is **MIT** (`bonsplit/LICENSE:1`). Cited below as `bonsplit/<path>` (paths under `Sources/Bonsplit/`).
- cmux's own code is GPL-3.0-or-later (`cmux/LICENSE:6-8`). BUSL dirs are only `web/`, `workers/*`, `services/iroh-relay-minter/`, `cmux-tui/relays/cloudflare-do/` (`cmux/LICENSE:10-17`). Everything cited below is outside those.
- App glue: `cmux/Sources/Workspace*.swift`, `cmux/Sources/TabManager*.swift`, `cmux/Sources/AppDelegate*.swift`. Pure split math: `cmux/Packages/macOS/CmuxPanes/`.
- **Terminology trap.** cmux "workspace" = sidebar tab (the Ghostty "tab", `cmux/docs/ghostty-keybindings.md:6-7`; action `newTab` is labelled "New Workspace", `cmux/Sources/KeyboardShortcutSettings.swift:257`). cmux "surface" = a tab inside a pane (a Bonsplit tab). "Pane" = a Bonsplit leaf. M2 is only panes + surfaces.

## 1. Model

**Types** (`bonsplit/Internal/Models/`):
- `indirect enum SplitNode { case pane(PaneState), split(SplitState) }` (`SplitNode.swift:12-14`). Strictly binary tree.
- `SplitState { id: UUID, orientation, first, second, dividerPosition: CGFloat /* 0.0 to 1.0 */ }` (`SplitState.swift:13-17`). Default `dividerPosition = 0.5` (`SplitState.swift:48`).
- `SplitOrientation`: `.horizontal` = side by side (first=LEFT, second=RIGHT); `.vertical` = stacked (first=TOP, second=BOTTOM) (`SplitNode.swift:96-106`, `Public/Types/SplitOrientation.swift`). cmux persists the same: `{orientation, dividerPosition: Double, first, second}` and leaf `{panelIds (tab order), selectedPanelId}` (`cmux/Packages/macOS/CmuxPanes/Sources/CmuxPanes/Values/SessionSplitLayoutSnapshot.swift`, `SessionPaneLayoutSnapshot.swift`).
- `PaneState { id, tabs: [TabItem], selectedTabId }` (`PaneState.swift:6-9`). A pane is a leaf holding an ordered tab list plus a selected tab. The ratio is a fraction of the first child; there are no per-pane absolute sizes (Bonsplit has an "imposed extent" mode for cmux's remote-tmux mirror only, `SplitState.swift:18-33`; ignore for M2).
- Panes are located by `PaneID` (UUID); there is also a tab-to-pane index (`SplitViewController.swift:15-17`).

**Surfaces/tabs vs panes.** A pane always holds >= 1 tab except transiently. Each tab = one panel (terminal, browser, ...). Selected tab is the only one visible; others are kept alive (`contentViewLifecycle: .keepAllAlive`, `cmux/Sources/Workspace.swift:4382`).

**Tab insertion/selection (`PaneState.swift`):**
- New tab goes to index `selectedIndex + 1` (`newTabPosition: .current` set by cmux, `Workspace.swift:4383`; Bonsplit logic `bonsplit/Public/BonsplitController.swift:244-259`). Pinned tabs always stay left of unpinned (`PaneState.swift:36-52`).
- Closing the selected tab selects the tab that slid into the same index (the "next" one), else the previous when the last tab was removed (`PaneState.swift:63-80`, comment at :69-71).
- Next/prev tab wraps (`SplitViewController.swift:884-903`).

**Close rules:**
- Closing the last tab of a pane when other panes exist: the pane is removed and the sibling absorbs the space (`SplitViewController.swift:775-783`, `closePaneRecursively` :671-710: parent split is replaced by the surviving child). Divider positions of unrelated splits are unchanged. No collapse of the parent when it has the other subtree; the sibling subtree simply takes the parent's slot.
- Closing the last pane is refused (`closePane` guard `paneStatesById.count > 1`, :640; config `allowCloseLastPane: false`, `Workspace.swift:4379`). What cmux does instead when the last surface of the workspace closes: by default (`app.keepWorkspaceOpenWhenClosingLastSurface` stored value `true`, which means "close the workspace on last surface"; `cmux/Packages/macOS/CmuxSettings/Sources/CmuxSettings/Keys/AppCatalogSection.swift:93-97`) the **workspace closes** (`Workspace.swift:14745-14753`, `TabManager.swift:3531-3541`). With the opposite setting the workspace stays and gets a fresh replacement terminal (`createReplacementTerminalPanel`, `Workspace.swift:12477`). For M2 (no workspaces yet) use the "keep open + replacement terminal" behaviour and revisit in M3.
- Closing a tab/pane may prompt first (running process, agent session); that is a delegate veto before the close (`Workspace.swift:14559`, `:15191`). Out of M2 scope.
- A pinned tab refuses close with a beep (`Workspace.swift:14735-14741`).

**New split (`SplitViewController.swift:395-478`, cmux glue `TabManager+TerminalSplitAction.swift:24-62`, `Workspace.swift:9405-`):**
- Splits the **focused pane** (the pane holding the source panel). Existing pane = `first`, new pane = `second`; so Split Right puts new on the right, Split Down puts new on the bottom. `SplitDirection.left/.up` set `insertFirst` (new pane first) but no default shortcut uses them (`CmuxPanes/Values/SplitDirection.swift`).
- Initial ratio **0.5** (`normalizedInitialDividerPosition`, `SplitViewController.swift:632-635`). Setting `app.equalizeSplitsOnCreate` (default **false**, `AppCatalogSection.swift:123-127`) instead re-equalizes the same-orientation run containing the new pane (`Workspace+EqualizeSplitsSupport.swift:59-95`).
- New pane is **focused** and a one-tab pane holding a new terminal (`focusedPaneId = newPane.id`, :449-451).
- **cwd:** new split inherits the source terminal's current directory (OSC 7 / `GHOSTTY_ACTION_PWD`) via `inheritedTerminalConfig`; Ghostty keys `split-inherit-working-directory`, `tab-inherit-working-directory`, `window-inherit-working-directory` default **true** each (`cmux/Packages/macOS/CmuxTerminalCore/Sources/CmuxTerminalCore/Config/GhosttyWorkingDirectoryInheritance.swift:13-14`; parser `GhosttyConfig.swift:610-621`). If split-inherit is false, the app default dir is used (`Workspace+RemoteTerminalWorkingDirectory.swift:14-25`). Resolution order: explicit request, source's reported cwd, source's requested startup cwd, workspace `currentDirectory` (`Workspace.swift:9483-9486`). New tab uses the `tab` key the same way (`Workspace.swift:9840`).
- **Minimum size admission (#15371, `Workspace+SplitSpace.swift`):** a split is allowed only if each half keeps `max(minimumPaneWidth(100), 160)` wide or `max(minimumPaneHeight(100), tabBarHeight(28) + 3*17 = 79)` = 100 tall (:11-26). If the halves would be too small, it first tries equalizing the run of same-orientation ancestors ("borrow room"), and if even that fails it is refused with `NSSound.beep()` (`ExternalTreeNode+SplitSpace.swift:4-16`, `TabManager+TerminalSplitAction.swift:51-55`).
- Any split clears zoom first (`SplitViewController.swift:403`, `TabManager+TerminalSplitAction.swift:44`).
- **New Pane (Auto Layout)** (`newPaneAutoLayout`, default Ctrl+Cmd+N): rebuilds the whole tree in Zellij's default layout, all dividers equalized: <=5 panes = left column of 1 + right column of the rest; beyond that columns of 4 with the first column taking the remainder (`SplitViewController.swift:577-630`). Optional for M2.

**Equalize** (`equalizeSplits`, `CmuxPanes/Geometry/ExternalTreeNode+SplitGeometry.swift:11-31, 148-166`): for every split, `position = spans(first) / (spans(first)+spans(second))` where `spans` counts leaves along the split's own axis only (a child split of a different orientation counts as 1). So N panes in a row come out with equal widths. Applied post-order (children before parent). Divider-double-click equalizes only the **run** containing that divider: that split, its same-orientation ancestors (stopping at a cross-orientation split), and same-orientation descendants (`equalizeDividerPlan(forSplitRunContainingSplitId:)`, :86-90, `equalizeDividerPlan(forSplitPath:)` :43-66).

## 2. Focus

- `focusedPaneId` is a single value; every change bumps a per-pane monotonic "focus sequence" (`SplitViewController.swift:19-22, 34-39, 287-308`).
- **Directional move (`adjacentPane`, :797-866)**: **geometric**, not tree order. Compute each pane's normalized bounds (:85-111). Candidates = panes whose facing edge is within epsilon 0.001 of the current pane's edge in the asked direction **and** with positive perpendicular overlap (> 0.001) (:835-866). Tie-break among candidates: most recently focused wins (Zellij rule); never-focused panes rank by larger edge overlap, then layout order (:815-821). No wrap: at an outer edge nothing happens (:788-795).
- **Cycle prev/next pane**: depth-first tree order (first child before second), wrapping (`CmuxPanes/Model/PaneCycleNavigator.swift`, `Geometry/ExternalTreeNode+SpatialOrder.swift:6-14`, `Workspace+SurfaceNavigation.swift:221-232`). `focusPaneByNumber` uses the same order, 9 = last (`:279-290`). Both are **unbound by default** (`KeyboardShortcutSettings.swift:592-595`).
- **Focus after closing a pane**: if the closed pane was focused, focus goes to the **most recently focused remaining pane**; if none was ever focused, the sibling that absorbed the space (`SplitViewController.swift:650-664`). An unrelated close never changes focus (:662).
- **Click:** clicking a pane focuses it (terminal pointer-down calls `focusPanelFromTerminalInput`, `GhosttyNSView+PointerFocusActivation.swift:10-27`). The click that focuses an **unfocused** pane is **not forwarded** to the terminal unless the app has mouse reporting on (`TerminalPointerFocusActivationPolicy`, `CmuxTerminalCore/Sources/CmuxTerminalCore/Focus/TerminalPointerFocusActivationPolicy.swift:8-14`; tests `...Tests/TerminalPointerFocusActivationPolicyTests.swift`). Setting `app.focusPaneOnFirstClick` default **false** (`AppCatalogSection.swift:99-103`); UNVERIFIED exactly what flipping it changes (consumed in `GhosttyTerminalView.swift:6962`, `Panels/*`).
- Clicking a tab selects it and focuses its pane (`BonsplitController.selectTab`, :453-465). Clicking empty tab-bar area of an unfocused pane focuses the pane (`TabBarView.swift:2933-2939`).
- **Focus follows mouse:** yes, via Ghostty's own `focus-follows-mouse` config key (default false), read through `ghostty_config_get` and applied on mouse-move when no button is pressed, app active, window key (`GhosttyTerminalView.swift:2809-2816, 4355-4370, 10028`).
- Focus a pane re-selects its selected tab's panel (`Workspace.swift:15118-15135`).
- Directional focus with a zoomed pane: Bonsplit does not clear zoom on `navigateFocus`; UNVERIFIED how cmux avoids focusing a hidden pane (moving surface between panes does clear zoom, `Workspace+SurfaceNavigation.swift:91-94`).

## 3. Keyboard shortcuts

Defaults live in `cmux/Sources/KeyboardShortcutSettings.swift` (`Action.defaultShortcut`, :430-700; mirror table in `cmux/Packages/macOS/CmuxSettings/Sources/CmuxSettings/Values/ShortcutAction+Defaults.swift`). Routing in `AppDelegate.swift:~16268-16620`, `AppDelegate+EqualizeSplitsShortcut.swift`, `AppDelegate+AdjacentNavigationShortcut.swift`. Cmd=Super on Linux.

| Action name (rawValue) | Default chord | Source line | Notes |
|---|---|---|---|
| `splitRight` | Cmd+D | KSS:596 | new pane on right, focused |
| `splitDown` | Cmd+Shift+D | KSS:597 | new pane below |
| `toggleSplitZoom` | Cmd+Shift+Return | KSS:599 | zoom/unzoom focused pane |
| `equalizeSplits` | Cmd+Ctrl+Shift+= | KSS:606 | all dividers |
| `resizePaneLeft/Right/Up/Down` (`resize-pane-*`) | **Ctrl+Shift+H / L / K / J** (no Cmd) | KSS:607-610 | step = `app.paneResizeStepPixels`, default **20**, clamp 1..200 |
| `focusLeft/Right/Up/Down` | Cmd+Opt+Arrow | KSS:580-587 | geometric |
| `focusPreviousPane` / `focusNextPane` | unbound | KSS:592-593 | tree-order cycle |
| `focusPaneByNumber` | unbound | KSS:595 | |
| `newPaneAutoLayout` | Cmd+Ctrl+N | KSS:598 | Zellij layout |
| `newSurface` (new tab in focused pane) | Cmd+T | KSS:660 | terminal, after selected tab, cwd inherited |
| `closeTab` (closes focused surface) | Cmd+W | KSS:559 | see close rules |
| `closeOtherTabsInPane` | Cmd+Opt+T | KSS:561 | |
| `closeWorkspace` | Cmd+Shift+W | KSS:563 | M3 |
| `nextSurface` / `prevSurface` | Cmd+Shift+] / Cmd+Shift+[ | KSS:640-643 | wraps within focused pane |
| legacy next/prev surface | Ctrl+Tab / Ctrl+Shift+Tab | `AppDelegate+DockShortcutRouting.swift:368-381`, `AppDelegate.swift:16596-16620` | hard-coded, reuse nextSurface/prevSurface routing; not remappable on their own |
| `selectSurfaceByNumber` | Ctrl+1..9 | KSS:658; handler `AppDelegate.swift:16271-16285` | 9 = last tab, N = Nth tab in focused pane |
| `moveSurfaceLeft` / `moveSurfaceRight` | Cmd+Shift+Opt+[ / ] | KSS:644-645 | reorder within pane, no wrap (`Workspace+SurfaceNavigation.swift:252-259`) |
| `moveSurfaceToPreviousPane` / `NextPane` | Cmd+Ctrl+Shift+[ / ] | KSS:646-649 | |
| `moveSurfaceToPaneLeft/Right/Up/Down` | Cmd+Shift+Opt+Arrow | KSS:650-657 | creates a directional split if no neighbour (`allowMissingDestinationSplit`, `Workspace+SurfaceNavigation.swift:53-70`) |
| `renameTab` | Cmd+R | KSS:542 | |
| `reopenClosedBrowserPanel` | Cmd+Shift+T | KSS:575 | browser only |
| `selectWorkspaceByNumber` | Cmd+1..9 | KSS:685 | M3 |
| `nextSidebarTab`/`prevSidebarTab` | Cmd+Ctrl+] / [ | KSS:530-533 | workspaces, M3 |
| `focusHistoryBack/Forward` | Cmd+[ / Cmd+] | KSS:535-538 | workspace focus history, not panes |

Notes:
- There is no dedicated "close pane" shortcut; Cmd+W closes the focused surface and the pane disappears when empty.
- Resize semantics are *move the divider*, not grow/shrink (`ResizeDirection`, `CmuxPanes/Values/ResizeDirection.swift:23-39`; `resizeDividerAdjustment`, `ExternalTreeNode+SplitGeometry.swift:182-207`): `right`/`down` need the pane to be the **first** child of an enclosing same-orientation split and move that divider +step; `left`/`up` need the pane to be the **second** child and move the divider -step. Candidates are tried innermost-first. Delta = `pixels / axisPixels` (combined bounds of the split); result clamped to **0.1..0.9**. If no enclosing divider matches, nothing happens and cmux **beeps** (`AppDelegate+EqualizeSplitsShortcut.swift:~100-106`). So in a 2-pane left|right layout Ctrl+Shift+L from the left pane widens it; Ctrl+Shift+H from the left pane does nothing.
- Ctrl+Shift+Return digit hints: holding Ctrl (or Cmd) shows a digit pill on tab icons for the focused pane (`TabItemView.swift:99-203`, `controlShortcutDigit`); optional.
- ⌘ here is `command: true` in `StoredShortcut`; a Linux default of Super collides with the compositor, so every binding must be remappable (already the project rule).

**Remapping** (`cmux/Sources/KeyboardShortcutSettingsFileStore*.swift`, `cmux/docs/ghostty-keybindings.md`, `cmux/docs/configuration.md`):
- File `~/.config/cmux/cmux.json` (JSONC), fallback `~/.config/cmux/settings.json` (`KeyboardShortcutSettingsFileStore.swift:17-27`). Key `shortcuts.bindings.<actionRawValue>`; value is a string `"cmd+shift+]"` or a 2-element array for a two-stroke chord `["ctrl+b", "c"]`; modifier tokens `cmd`, `shift`, `opt`, `ctrl`, key tokens `left/right/up/down`, `return`, `tab`, `space`, `comma`, `period`... (`KeyboardShortcutSettings.swift:2432-2500`, template `KeyboardShortcutSettingsFileStore+Template.swift:333-344`). Unbound = `.unbound`. Some actions take a `when` clause (`whenClausesByAction`, store :63).
- Also a Settings UI recorder (rejects bare keys, system-reserved, conflicts: `KeyboardShortcutSettings.swift:71-75`).
- **Ghostty keybinds are also read**, from the user's Ghostty config, but only for terminal-focused bindings and they are *supplementary*: cmux unbinds Ghostty's defaults for the keys it owns (`super+d`, `super+shift+d`, `super+w`, `super+alt+w`, `super+shift+w`, `super+ctrl+=`, `super+1..9`, `super+n`, `super+t`, `super+q`, `super+,`, `super+enter`, `super+shift+p`, `super+shift+[`/`]`, `ctrl+tab`, `ctrl+shift+tab`, `super+z`) so a cmux rebind/clear cannot be shadowed (`GhosttyApp+KeybindOverrides.swift:5-43`). A key/leader claimed by a cmux shortcut never reaches Ghostty; Ghostty bindings do not fire in non-terminal panes (`docs/ghostty-keybindings.md:25-35`). Ghostty split actions "continue to use cmux's split commands" and `goto_split` is mirrored (`AppDelegate+DockShortcutRouting.swift:383-` `ghosttyGotoSplitShortcut`). For M2 we get this free if the shortcut layer sits in front of libghostty's keybinds.

## 4. Visuals (exact numbers)

All values are in points on macOS; treat as logical px on Linux. Source of cmux's overrides: `Workspace.bonsplitAppearance` (`cmux/Sources/Workspace.swift:4136-4170`) which only sets `tabBarHeight`, `tabTitleFontSize`, `dividerHitExpansion`, `enableAnimations: false`, `chromeColors`, split-button backdrop; everything else is Bonsplit's `Appearance` default (`bonsplit/Public/BonsplitConfiguration.swift:646-668`).

**Pane tab strip**
- Strip height **28** (`cmux/Sources/WindowChromeMetrics.swift:5-7`, `sharedChromeBarHeight = 28`; same as titlebar). Bonsplit default 30 is overridden.
- Shown **always**, even with a single tab (`tabBarVisibility: .always` default, `BonsplitConfiguration.swift:120`; cmux setting `app.tabBarVisibility` `always` | `multiple-tabs`, `AppCatalogSection.swift:339-343`, `PaneTabBarVisibility.swift:4-8`; minimal presentation mode forces `.always`, `Workspace.swift:4732-4735`).
- Tab: height = bar height (28); min width **48** (`TabBarMetrics.tabMinWidth`; the `Appearance.tabMinWidth` 140 is *ignored*: `TabItemView.swift:223-227`), max width **220** (`Appearance.tabMaxWidth`). Tabs size to their content (`fixedSize`, `TabItemView.swift:380-390`) between those bounds; strip scrolls horizontally on overflow (`TabBarView.swift:1231`). Horizontal padding **6** (`TabBarMetrics.swift:16`), tab spacing **0**, no corner radius (`:15,17`).
- Content order: [icon] [title] ... [zoom/audio accessories] [close/dirty/pin slot]. Icon slot **14** (terminal glyph uses 11.5 = max(10, 14-2.5), `TabItemView.swift:975-978`), icon-title gap **6**, close slot = `min(tabHeight, max(16, ceil(max(8, titleSize-2)+4)))` = **16** at default font (`:1030-1033`), close glyph `xmark` size **9** semibold. Icon, gap, close glyph scale with `titleFontSize/11` (`:961-981`). Terminal tabs use SF symbol `terminal.fill` (`Workspace.swift:4533`).
- Title font: system, **11 pt (default), regular weight** (`TabItemView.swift:652` `.font(.system(size: appearance.tabTitleFontSize))`), single line, tail-truncated. Size comes from Ghostty-style config key `surface-tab-bar-font-size`, default 11, clamped **8..14** (`CmuxGhosttyConfigSettingEditor.swift:21-28`, parse `GhosttyConfig.swift:589-593`).
- **Title source** for terminals: Ghostty `SET_TITLE` action (OSC 0/2, which Ghostty also emits from shell integration/process), pushed through a coalescing ingress (`GhosttyTerminalView.swift:3533-3545`). Empty title falls back to `"Terminal"` (`Panels/TerminalPanel.swift:134-136`); a generic fallback is `"Tab"` (`Workspace.swift:5766`). A user rename (`customTitle`, Cmd+R) overrides it; automation labels prefix it as `"<label> · <title>"` (`Workspace.swift:5764-5776`). cwd is *not* used as the tab title.
- Colors (no custom bg): active tab fill = `controlBackgroundColor`, hover fill = same @ 0.5 alpha, inactive = clear, text = `labelColor` / `secondaryLabelColor` (`TabBarColors.swift:178-241`). cmux **always** passes a custom bg (the terminal background), so the real computation is:
  - bar bg = terminal bg (composited w/ opacity); transparent when the window shares a backdrop (`Workspace.swift:4011-4049`).
  - active tab fill: light bg -> darken 0.065; dark bg -> lighten 0.12 (RGB each channel +/-, alpha kept) (`TabBarColors.swift:178-190`); `.clear` when sharing the backdrop.
  - hover fill (not selected): light -> darken 0.03; dark -> lighten 0.07; then alpha **0.78**; for shared backdrop: black@0.055 / white@0.075 overlay (`:193-205`).
  - text: if bar bg is "light" (WCAG black-contrast > white-contrast): active black@**0.82**, inactive black@**0.62**; else active white@**0.82**, inactive white@**0.68** (`:11-14, 96-109`).
  - separator colour (tab right edge, bar bottom edge): `WindowChromeColorResolver.separatorColor` -> light bg: each RGB channel **-0.30**, alpha **0.26**; dark bg: **+0.16**, alpha **0.36**; Increase-Contrast variants -0.55/0.55 and +0.40/0.65 (`CmuxAppKitSupportUI/.../WindowChromeColorResolver.swift:30-45`). Optional user override `paneBorderColor` (`cmux/docs/configuration.md:76-90`).
- Per-tab chrome: 1 px right-edge separator on every tab (`TabItemView.swift:1126-1137`); 1 px bottom separator across the whole bar **except under the selected tab** (selected tab visually merges with content) (`TabBarSelectionChromeView.swift:59-108`); **1.5 px accent indicator on top edge** of the selected tab, inset 1 px from the tab's right (`TabBarMetrics.swift:18-19`, drawn :145-170). Accent = `NSColor.controlAccentColor` (`TabBarColors.swift:269-271`); UNVERIFIED whether cmux's own `app.accentColor` setting feeds this.
- **Unfocused pane**: the *entire tab strip is desaturated* (saturation 0 vs 1: `TabBarView.swift:923-925`), so the accent indicator turns grey; focused pane's strip is normal.
- Close button: shown only on the selected tab or the hovered tab (`TabItemView.swift:1184`); 16 px circle hit slot; on button hover the glyph goes from inactive to active text colour and a circle with the hover fill appears (`:1186-1203`). Dirty dot (8 px) / unread badge (6 px blue) show instead of the x on non-selected, non-hovered tabs (`:1159-1172`). Setting `app.hideTabCloseButton` hides it; middle-click closes a tab (`Workspace.swift:4407`, `TabItemView.swift:393-401`). `allowCloseTabs: !hidesTabCloseButton` (`Workspace.swift:4378`).
- Right-hand **split-action buttons** (this is the "new tab" button; there is no separate "+"): defaults `[newTerminal, newBrowser, splitRight, splitDown]` (`CmuxConfig.swift:1047-1052`; user-configurable via `cmux.json` surface tab bar buttons, `Workspace.swift:4832-4851`), SF symbols `terminal`, `globe`, `square.split.2x1`, `square.split.1x2`, tooltips "New Terminal / New Browser / Split Right / Split Down" (`BonsplitConfiguration.swift:364-413`). Glyph point size **12** (scaled by title font/11), colour = inactive text, pressed = active text at opacity 0.72 (`TabBarView.swift:1993-2005`, `:1706-1713`). Lane: leading pad **6**, trailing pad **8**, button reserved width **22**, spacing **4**, lane max 25% of bar width (`TabBarView.swift:183-201`). The lane sits over the right edge with a faded backdrop (`splitButtonBackdropEffect` default `fadeWidth 136`, `contentFadeWidth 42`, `trailingOpacity 0.80`, `BonsplitConfiguration.swift:440-451`); `splitButtonsOnHover` default false. For M2 Linux: skip the browser button.
- Double-click on empty strip area opens a new tab (`TabBarView.swift:2926-2931`, wired at `:1132` via `onDoubleClick`). Double-click on a **tab** toggles pane zoom (`TabItemView.swift:424-428`). Right-click tab = context menu (rename, close, close to left/right/others, move to left/right pane or workspace, new terminal to right, pin, zoom, ...; `Public/Types/TabContextAction.swift`).
- Pinned tabs: left-pinned group, pin glyph replaces close (`PaneState.swift:36-80`, `TabItemView.swift:1176-1183`).
- Animation: `enableAnimations: false` in cmux (`Workspace.swift:4162`), so no split slide-in. Bonsplit's metrics if wanted: hover 0.1 s, selection 0.15 s, reorder 0.3 s bounce 0.15, split 0.15 s (`TabBarMetrics.swift:57-68`).

**Dividers**
- Thickness **1** (`Appearance.dividerThickness` default 1, clamped 0..12; `TabBarMetrics.swift:41-55`). `dividerStyle = .thin` (`SplitContainerView.swift:335`). Not overridden by cmux.
- **Hit area:** cmux sets `dividerHitExpansion = 8` -> **8 px each side of the 1 px line (17 px total)** for both cursor and drag (`PortalSplitDividerRegion.swift:57`, passed at `Workspace.swift:4159`; Bonsplit default would be 5, `BonsplitConfiguration.swift:571-574`). Cursor: `resizeLeftRight` over a vertical divider, `resizeUpDown` over a horizontal one (`PortalSplitDividerRegion.swift:35-44`, `TerminalWindowPortal.swift:91`). Custom cursors not used (`BonsplitDividerCursors` unset).
- **Colour:** `dividerHex` if the user set Ghostty `split-divider-color` *and* no `paneBorderColor`; otherwise the chrome separator colour above (light: bg-0.30 @ alpha 0.26; dark: bg+0.16 @ alpha 0.36) (`Workspace.swift:3963-3975, 4005-4012`; `TabBarColors.swift:320-327`). The Ghostty-derived fallback `bg.darken(0.08 light / 0.4 dark)` exists as `resolvedSplitDividerColor` (`GhosttyConfig.swift:205-213`) but is applied to the SwiftUI panel appearance; UNVERIFIED whether it ever shows over the Bonsplit divider. Divider is repainted above terminal views where a surface overlaps it (`TerminalWindowPortalDividerOverlayView.swift`).
- Optional focused-pane outline `activePaneBorderColor` (hex, default none) (`PaneChromeSettings.swift:6-14`, `docs/configuration.md:76-90`). No focus border by default.

**Unfocused-pane dimming** (Ghostty keys honoured, parser `GhosttyConfig.swift:736-743`):
- Overlay drawn on a terminal only when `isSplit && !isFocused` (`Panels/TerminalPanelView.swift:114`; `Canvas/WorkspaceCanvasHostView.swift:95`). `isSplit` = workspace has >1 pane.
- `unfocused-split-opacity` default **0.7** (`GhosttyConfig.swift:65`), overlay alpha = `1 - clamp(opacity, 0.15, 1.0)` = **0.30** by default (`:193-197`). Colour = `unfocused-split-fill` if set (hex) else the terminal **background** (`:200-202`). Layer is an NSView above the surface, `setInactiveOverlay(color:opacity:visible:)` (`GhosttyTerminalView.swift:11958-11966`).
- Tab strip of an unfocused pane desaturates (see above).

**Min pane size:** 100 x 100 for divider drag (`BonsplitConfiguration.swift:653-654`), plus the 160-wide admission rule for creating a split (section 1).

**Drag and drop of tabs**
- Native drag session; tab strip shows a **2 x 20** accent insertion bar (`TabBarMetrics.swift:34-35`, `TabBarView.swift:1504-1508`); insertion index = first tab whose `midX` is right of the pointer, else end (`TabDropIndexResolver.swift:38-62`). Dropping on itself / right after itself is a no-op (`PaneState.swift:89`). Pinned/unpinned boundaries are enforced.
- Drop onto a pane body: 5 zones by pointer position, **edge band = max(80, 25% of width/height)**; checked left, right, top, bottom (left/right win in corners); centre = move tab into that pane (append); an edge = split that pane on that side with the dragged tab (`PaneContainerView.swift:334-351`, `performLocalTabDrop` :476-540). Special case: terminal tab dragged to the edge shared with its immediately adjacent pane counts as centre (`:373-391`).
- Drop preview: rounded rect radius **8**, fill accent@**0.25**, 2 px accent stroke, 4 px inset, covering the half of the pane the split would create (whole pane for centre) (`PaneContainerView.swift:55-110`, spring 0.25 s).
- Dragging the last tab out of a pane closes that now-empty pane (`SplitViewController.swift:757-759`). Cross-pane move needs `allowCrossPaneTabMove: true` (`Workspace.swift:4381`); the moved tab's pane gets focus (`:754`).
- Same-pane reorder via keyboard: `moveSurfaceLeft/Right` (section 3).

## 5. Divider dragging

- Native NSSplitView drag, continuous; **no snapping** (the only "snap" hits in `SplitContainerView.swift` are comments about stale-state snap-back). While dragging, `dividerPosition` is written live and any "imposed" extent is cleared (`SplitContainerView.swift:1570-1585`).
- Clamp: `max(requestedMin, ...)` where requested = 100 (width for horizontal, height for vertical), capped at `available/2` when the container is too small; `available = total - dividerThickness` (`SplitContainerView.swift:938-964, 1629-1643`). Fraction also clamped to `dividerPositionRange = 0.1...0.9` (`BonsplitConfiguration.swift:121`; `BonsplitController.setDividerPosition` :1177-1190).
- **Double-click on a divider** equalizes its row/column run (not all splits) when `app.equalizeSplitsOnDividerDoubleClick` (default **true**, `AppCatalogSection.swift:130-134`; handler `Workspace+EqualizeSplitsSupport.swift:11-25`). Bonsplit detects it as a `mouseDown` with `clickCount == 2` inside the divider's effective rect whose release is queued (`SplitContainerView.swift:134-156`); if the host declines, native AppKit double-click is used.
- Keyboard resize: section 3 (Ctrl+Shift+HJKL, 20 px, clamp 0.1..0.9, beep on no-op). Equalize-all: Cmd+Ctrl+Shift+=.
- Window resize: ratios are fractions, so panes scale proportionally.

## 6. Zoom

- `zoomedPaneId` on the controller; rendering shows only that pane's node instead of the root (`SplitViewContainer.swift:36-49`, `SplitViewController.swift:29-30, 327-353`). Other panes stay alive (hidden). Dividers/tab strips of hidden panes are not shown.
- Toggle (`togglePaneZoom`): no-op with a single pane ("Match Ghostty behavior", :348-349); sets zoom + focuses the pane; toggling the zoomed pane again unzooms. cmux then re-focuses the panel and reconciles terminal portal visibility (`Workspace.swift:12374-12394`).
- Triggers: Cmd+Shift+Return, tab context menu "Toggle Zoom", **double-click on a tab**, and the zoom accessory button on the tab (SF `arrow.up.left.and.arrow.down.right`, shown on the zoomed pane's selected tab, `TabItemView.swift:736-745`; accessibility value "Zoomed").
- Zoom auto-clears on: any split creation (:403; `TabManager+TerminalSplitAction.swift:44`), moving a surface to another pane (`Workspace+SurfaceNavigation.swift:91-94`), closing the zoomed pane (`SplitViewController.swift:666-668`), and closing the *last* tab of the zoomed pane. Closing a tab while others remain keeps zoom (issue 8363; `Workspace.swift:14568-14578`).
- Zoomed pane takes the full content area below nothing else (the tab strip of that pane stays at the top of it).

## 7. Notes for the Linux plan

- All of the model is small and pure: binary tree + fraction + per-pane ordered tab list; port `adjacentPane`, `closePane`, equalize span-count, resize delta, split admission, and drop-zone math as unit-tested pure functions in `cmux-core` (cmux's own tests: `cmux/Packages/macOS/CmuxPanes/Tests/CmuxPanesTests/`).
- Behaviours easy to miss: unfocused-click does not reach the PTY; resize shortcuts move dividers by side (not grow/shrink); unfocused strip is greyscale; selected tab merges with content (no bottom line under it); strip shows with one tab; effective min tab width is 48 not 140; hit band 17 px for a 1 px divider; split beeps instead of failing silently.
- Not needed for M2: Canvas layout mode, remote tmux mirror, Dock, browser tabs, agent icons, fork-conversation actions, tab presence/audio badges, full-width tab mode (`isFullWidthTabMode`).
- Abbreviation: KSS = `cmux/Sources/KeyboardShortcutSettings.swift`.
