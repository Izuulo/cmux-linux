# M3 research — cmux workspaces and sidebar

Facts from cmux @ 21dcc2b to match, not code to copy. Cited as `rc §n`. UNVERIFIED = not confirmed.

## 0. Sources and licence
- Read only GPL-3.0-or-later code: `cmux/Sources/`, `cmux/Packages/macOS/`, `cmux/Resources/shell-integration/` (no own LICENSE). No BUSL directories (`web/`, `workers/`, `services/`, `coderouter/`, `cmux-tui/relays/cloudflare-do/`).
- Shorthands: `S/` = `cmux/Sources/`, `P/` = `cmux/Packages/macOS/`, `SH/` = `cmux/Resources/shell-integration/cmux-zsh-integration.zsh`, `KSS` = `S/KeyboardShortcutSettings.swift`, `CV` = `S/ContentView.swift`, `TM` = `S/TabManager.swift`.
- Terminology (as M2 rc §0): cmux "tab"/`Tab` = **workspace** (a sidebar row); `TabManager` = the per-window workspace list; "panel"/"surface" = a tab inside a pane.
- Two sidebar list implementations share metrics: SwiftUI `TabItemView` (`CV:16174`) and AppKit cells (`S/Sidebar/AppKitList/Cells/SidebarWorkspaceRowCellView.swift`, flag `sidebar-appkit-list-experiment`, `S/FeatureFlags.swift:68`); top pad 8, section spacing 4, radius 6 (cell `:319, :1273-1274`). Which is default: UNVERIFIED, irrelevant.

## 1. Workspace model
- **Owns** (`S/Workspace.swift:2848-2935`): `title`, `customTitle` + source (user/auto/remote), `customDescription`, `isPinned`, `isMuted`, `groupId`, `customColor`, `currentDirectory` + per-panel `panelDirectories` (`:6281-6358`), `processTitle` (`:3741`), `listeningPorts` (`:3387`), the Bonsplit tree (M2).
- **Title** (`S/Workspace+TitleOwnership.swift`): `customTitle` if set, else the **focused panel's** OSC 0/2 title (`applyFocusedPanelTitle` :100, `applyProcessTitle` :56-73); spinner glyphs stripped first (:130-140). Initial title `"Terminal"` (`S/Workspace.swift:4293, 4353-4355`). `setCustomTitle` (:237-256): empty/whitespace **clears** the custom title.
- **New** (`newTab`, Cmd+N; `TM:1739`): placement `app.newWorkspacePlacement` default **`afterCurrent`** (else `top`, `end`; `AppCatalogSection.swift:55-59`) — selected+1, after the pinned block (`TM:1975-2007`); selected. **cwd** (`WorkspaceCreationWorkingDirectoryPolicy.swift:14-27`): explicit → inherited if `app.workspaceInheritWorkingDirectory` (default **true**) and Ghostty `*-inherit-working-directory` allow (`TM:2041-2046`); inherited = **selected workspace's `currentDirectory`** (focused panel's cwd), else any panel dir (`TM:2014-2028`) → Ghostty `working-directory` → home → `/` (`TM:2035-2039`). Double-click empty sidebar space = new workspace **at the end** (`SidebarWorkspaceTableController.swift:2104-2106`).
- **Close** (`closeWorkspace`, Cmd+Shift+W; `TM:2948-2960`): confirm when any shell runs a command (`promptIdle` vs `commandRunning`, `S/Workspace.swift:3743-3755`); title "Close workspace?", message "This will close the workspace and all of its panels." (`TM:3386-3390`); gated by `app.warnBeforeClosingWorkspace` (true) / `app.alwaysConfirmWorkspaceClose` (false); pinned always ask. **Last workspace → closes the window** (`closeWindowForLastWorkspace`, `TM:3366, 3395-3404`); `closeWorkspace` refuses at `count <= 1` (`TM:2656`). Row close button hidden when only one workspace (`canCloseWorkspace = count > 1`, `CV:11938`). Selection after close: the workspace that slides into the slot, else the new last (`WorkspacesModel+CloseSelection.swift`). Last surface closed → workspace closes (`app.keepWorkspaceOpenWhenClosingLastSurface`).
- **Rename:** inline in the sidebar by **double-clicking a row** (`SidebarWorkspaceTableController.swift:1069-1088`) or context menu (`CV:17010-17020`); field in the title font (12.5), placeholder "Workspace name" (`CV:16631-16658`); Enter commits, Esc cancels, focus loss commits, first wins (`S/Sidebar/SidebarInlineRenameCoordinator.swift`). **Cmd+Shift+R** opens a command-palette rename prompt (`S/AppDelegate.swift:16107-16110, 17763-17770`). Existing name preselected (`app.renameSelectsExistingName` true).
- **Reorder:** drag in the sidebar; insertion line **2 pt** accent, inset **8 pt** each side, centred in the gap (`S/Sidebar/SidebarReorderIndicatorView.swift:7-8`); dragged row at 0.6 opacity (`CV:16934`). Keyboard `moveWorkspaceUp/Down` ±1 (`S/AppDelegate+AdjacentNavigationShortcut.swift:66-75`).
- Out of M3: groups, pinning, descriptions, todo lists, remote workspaces. Numbered selection skips group headers (`S/TabManager+NumberedWorkspaceSelection.swift:5-25`).

## 2. Shortcuts (Cmd → Super; `KSS` `defaultShortcut`, labels `KSS:254-323`)

| Action (rawValue) | Default | KSS | Notes |
|---|---|---|---|
| `newTab` ("New Workspace") | Cmd+N | 460 | §1 placement/cwd |
| `closeWorkspace` | Cmd+Shift+W | 563 | §1 |
| `renameWorkspace` | Cmd+Shift+R | 544 | palette prompt |
| `selectWorkspaceByNumber` | Cmd+1..9 | 685 | 9 = last (`S/App/TerminalDirectoryOpenSupport.swift:926-936`); out of range = no-op but key consumed (`S/AppDelegate.swift:16254-16267`) |
| `nextSidebarTab` ("Next Workspace") | Cmd+Ctrl+] | 530 | **wraps** (`WorkspacesModel+Cycling.swift:68-71`) |
| `prevSidebarTab` | Cmd+Ctrl+[ | 532 | wraps |
| `moveWorkspaceUp` / `Down` | Cmd+Ctrl+Opt+[ / ] | 687-688 | ±1, no wrap (UNVERIFIED) |
| `toggleSidebar` | Cmd+B | 453 | |
| `focusSidebar`, `reopenClosedWorkspace` | unbound | 456, 576 | |
| `goToWorkspace` (palette) | Cmd+P | 486 | palette (M11) |
| `newBrowserWorkspace` | Cmd+Opt+N | 469 | out of scope |

Next/prev routing `S/AppDelegate.swift:16058-16080`. While Cmd is held, rows show `⌘1` hint badges (10 pt), last row `⌘9` (`CV:16924-16931`).

## 3. Sidebar visuals
- **Width** (`S/SessionPersistence.swift:22-30`): default **240**, minimum **240** (configurable 120–260), maximum **600**; resizable; resize hit band 6 pt into the sidebar, 4 pt into the content (`S/Sidebar/SidebarState.swift:72-80`); visible by default.
- **Background:** `sidebarAppearance.matchTerminalBackground` default **true** — the sidebar sits on the **same backdrop as the terminal** (`CV:2042`, `S/Workspace.swift:3880-3882`), so colour/translucency follow Ghostty `background` / `background-opacity` / `background-blur`. With it off: NSVisualEffect `.sidebar`, tint `#000000` @ 0.18 (`SidebarAppearanceCatalogSection.swift:12-67`). Trailing vertical border (`WindowChromeBorder`, `CV:12059-12064`; colour probably the M2 separator, UNVERIFIED).
- **Top chrome:** titlebar band **28 pt**; first row at **30 pt** (`S/WindowChromeMetrics.swift:5-11, 90`). Titlebar controls (classic): icon 12, button 20, spacing 6, radius 6 (`S/Update/UpdateTitlebarAccessory.swift:63-74`): toggle sidebar, bell (M4), back/forward, **"+" new workspace** (tooltip "New workspace", `TitlebarNewWorkspaceSplitButton.swift:53-126`). Footer (account/help/update) not needed.
- **Row metrics** (`CV:16572-16932`, `WindowChromeMetrics.swift:92-94`): outer horizontal padding **6**, inner horizontal **10**, vertical **8**, section spacing **4**, row spacing **2** (`CV:11636`), rounded rect radius **6** (`CV:17061`), content-driven height. Fonts: system font × `sidebar-font-size / 12.5`, default 12.5.
- **Row contents, top to bottom:**
  1. Title line: (M4 badge/spinner), pin/mute icons 9 pt, **title 12.5 semibold**, 1 line, tail-truncated (`CV:16661-16665`); trailing `xmark` 9 pt **only on hover** and only when count > 1 (`CV:16425-16430`).
  2. Description (out of scope). 3. Notification subtitle (M4). 4. Remote/log/progress (skip).
  5. **Branch + directory** (`CV:16736-16823`), vertical layout default (`sidebar.branchVerticalLayout` true): one line **per unique directory** across the workspace's panels: `branch*` · 3 pt `circle.fill` dot · directory, **10 pt monospaced**, secondary @ 0.75; `*` = dirty (`S/SidebarWorkspaceSnapshotFactory.swift:389, 401`); branch icon off by default.
  6. **PR rows** (`CV:16826-16861`), one per unique PR URL: status icon, `PR #123` (underlined, clickable), word `open`/`merged`/`closed`; **10 pt semibold**, **not state-coloured** (`.secondary`; selected: white @ 0.75, `CV:17138-17156`); icons: 13 pt stroked paths open/merged, `xmark.circle` 7 pt closed (`CV:17272-17330`); stale at 0.5 opacity; no draft state; label literal `"PR"`.
  7. **Ports** (`CV:16864-16883`): underlined buttons `:3000`, tooltip "Open localhost:3000" (`S/SidebarPortDisplayText.swift`); spacing 4, 10 pt monospaced @ 0.75, 1 line, **ascending**.
- **Directory shortening** (`S/Sidebar/SidebarPathFormatter.swift`): home → `~`; `sidebar.pathLastSegmentOnly` default false → full `~` path, tail-truncated (the fitting-candidate logic `~/a/b/c` → `…/b/c` → `…/c` is in `SidebarDirectoryText.swift`). A directory equal to the title is not repeated (`droppingTitleEcho`, `SidebarWorkspaceSnapshotFactory.swift:370`).
- **Selection** (`S/Sidebar/SidebarAppearanceSupport.swift:402-560`): default `leftRail` style, `subtleSelection` false → selected row = **solid accent fill**, no border. Accent = cmux blue **#0088FF light / #0091FF dark** (`P/CmuxFoundation/.../Color/CmuxAccentColor.swift:49-56`). Selected foreground white if contrast ≥ 2.75 (:434-444): title 1.0, detail 0.75–0.8, icons 0.6. Unselected: no fill, title `.primary`, details `.secondary` (alpha UNVERIFIED, ~50 % black / ~55 % white). **Hover: no fill**, only reveals close. Multi-select fill accent @ 0.25.

## 4. Metadata collectors
- **Which panels feed a row:** **all panels**, spatial order; branch/dir lines deduped per canonical directory; PRs deduped by URL; ports sorted union (`P/CmuxSidebar/.../Git/SidebarBranchOrdering.swift:3-5, 198-312`; `S/Workspace+PanelLifecycle.swift:386-395`). Title and `currentDirectory` come from the **focused** panel.
- **cwd:** OSC 7 (`S/GhosttyTerminalView.swift:3548-3560`) per panel (`S/Workspace.swift:6281-6358`); shell integration may also `report_pwd`. Process cwd not polled.
- **Git branch:** (a) shell integration reads `.git/HEAD` directly: walks up to `.git` dir or follows a `.git` file `gitdir:` (worktrees/submodules, `SH:932-964`); branch only from `ref: refs/heads/…`; **detached HEAD → no branch** (`SH:996-1004`); 1 s HEAD poll while a command runs (`SH:2040-2081`). (b) app `SidebarGitMetadataService` (`P/CmuxSidebarGit/.../SidebarGitMetadataService.swift:6-33`): probe on dir change with retries 0/0.5/1.5/3/6/10 s, then fs watchers on `HEAD`, `index`, `refs`, `packed-refs`, `reftable`, `config` (`GitMetadataService+WatchPaths.swift:158-179`), coalesced 250 ms; dirty via `git status --porcelain=v1 -z --untracked-files=no --ignore-submodules=dirty --no-renames`, bounded **2 s** / **1 byte** (`GitDirtyStatusReading.swift:38-46`); ≤ 2 concurrent probes; deferred while typing (0.65 s); nil branch clears; `sidebar.watchGitStatus` default true.
- **Pull requests:** cmux calls the **GitHub REST API**, not `gh pr view`: `GET /repos/{owner}/{repo}/pulls?state=all&head={owner}:{branch}&sort=updated&direction=desc&per_page=100` (`PullRequestProbeService+Fetch.swift:294-318`); auth `GH_TOKEN`/`GITHUB_TOKEN`/`gh auth token`, fails closed without (`PullRequestProbeService.swift:17-30`). Fields `number, state, html_url, updated_at, merged_at, head.ref, base.ref`; non-empty `merged_at` = MERGED; only head refs equal to the branch kept. Slug from remotes (`upstream`, then `origin`, then others). Default branch (`main`/`master`) not looked up (`PullRequestProbeService.swift:84-88`).
  - **Selection** (`PullRequestProbeService+Selection.swift:30-115`): open > merged > closed, then latest `updated_at`, then highest number; merged older than **14 days** → no badge.
  - **Cadence** (`PullRequestPollService.swift:19-31`): **10 s** focused panel, **60 s** background, ±10 % jitter; merged/closed re-checked every **15 min**; ≤ 3 panels per batch; repo cache 15 s.
  - **Rate limiting** (`GitHubPullRequestRequestCoordinator.swift`): ETags, ≤ 3 concurrent, honour `X-RateLimit-Reset` / `Retry-After`; 404 or `[]` = "known absent" until cache expiry; repeated failures → stale (0.5 opacity).
  - **Triggers:** branch change, prompt, and after a successful `gh pr {merge|close|reopen|create|checkout|ready|edit|view}` (shell `report_pr_action`, cache bypass; `SH:1222-1264`). HEAD change clears the panel's PR. `sidebar.showPullRequests` default true.
- **Ports** (`S/PortScanner.swift:6-19, 86-95`): shell kicks (`report_tty`, `ports_kick` on preexec, and precmd after ≥ 2 s commands / ≥ 10 s since last); app coalesces 200 ms then scans at 0.5, 1.5, 3, 5, 7.5, 10 s; a port must be missing in 2 consecutive scans to drop. Scan = every process whose **controlling TTY is the panel's TTY**, its TCP `LISTEN` sockets (libproc, no subprocess; `S/ListeningPortLookup.swift:14-19, 65`). No scanning while ports hidden. Linux mapping (ours): `/proc/*/stat` `tty_nr` + socket inodes in `/proc/net/tcp{,6}` state `0A`.

## 5. Settings (defaults)

| Setting | Default |
|---|---|
| `sidebar.hideAllDetails` | false |
| `sidebar.showBranchDirectory` / `sidebarShowGitBranch` | true / true |
| `sidebarShowGitBranchIcon` | false |
| `sidebar.branchVerticalLayout` | true |
| `sidebar.pathLastSegmentOnly` | false |
| `sidebar.showPullRequests` / `makePullRequestsClickable` | true / true |
| `sidebar.watchGitStatus` | true |
| `sidebar.showPorts` | true |
| `workspaceColors.subtleSelection` | false |
| `sidebarAppearance.matchTerminalBackground` | true |
| `app.newWorkspacePlacement` | afterCurrent |
| `app.workspaceInheritWorkingDirectory` | true |
| `app.warnBeforeClosingWorkspace` / `alwaysConfirmWorkspaceClose` | true / false |

Not checked: bash/fish integration parity, AppKit-list flag default, exact unselected label alpha, whether closing the last window quits. Spec-relevant surprises: **PR rows are not state-coloured** (word + icon), and **cmux uses the GitHub REST API** with `gh auth token`, not `gh pr view`.
