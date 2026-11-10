# M4 research — cmux notifications

Facts from cmux @ 21dcc2b to match, not code to copy. Cited as `rn §n`. UNVERIFIED = not confirmed from GPL dirs.

## 0. Sources and licence
- Read only `Sources/`, `Packages/macOS/`, `Resources/`, `docs/`. No BUSL dirs. Not readable here (so UNVERIFIED): the Ghostty fork (`ghostty/`, OSC parsing) and Bonsplit (`vendor/bonsplit`, the pane tab strip).
- Shorthands: `S/` = `Sources/`, `P/` = `Packages/macOS/`, `TNS` = `S/TerminalNotificationStore.swift`, `CV` = `S/ContentView.swift`, `TM` = `S/TabManager.swift`, `KSS` = `S/KeyboardShortcutSettings.swift`, `NS` = `P/CmuxSettings/Sources/CmuxSettings/Keys/NotificationsCatalogSection.swift`, `SC` = `.../Keys/SidebarCatalogSection.swift`, `ND` = `P/CmuxNotifications/Sources/CmuxNotifications/`.
- Terminology: cmux "tab"/`Tab` = **workspace** (sidebar row); "panel"/"surface" = a tab inside a pane. In the store, `tabId` = workspace id, `surfaceId` = panel id.
- Out of scope: hooks, agent/Feed, cloud/phone forwarding (`docs/notifications.md`).

## 1. Sources of notifications
- OSC 9 / 99 / 777 are parsed **inside Ghostty**; the app only sees one action `GHOSTTY_ACTION_DESKTOP_NOTIFICATION` carrying **title + body** (no subtitle) (`S/GhosttyTerminalView.swift:3562-3576`). Surface-targeted only; an app-targeted one is dropped (`:3228-3232`). So `9;body` = empty title; `777;notify;title;body` = both; OSC 99 is a cmux Ghostty-fork parser (`docs/ghostty-fork.md:1999-2010`). Which OSC 99 keys/chunks it honours, and what `9;4` (ConEmu progress) / other `9;N` do: UNVERIFIED (fork source unreadable). Ghostty truncates OSC bodies to 255 bytes (comment `S/GhosttyDesktopNotificationRequest.swift:84-86`).
- Subtitle is host-owned: always `""` for OSC (`GhosttyDesktopNotificationRequest.swift:11-14`); only `cmux notify --subtitle` fills it.
- **BEL does not create a notification.** `GHOSTTY_ACTION_RING_BELL` → `ringBell` plays system beep and/or custom audio per Ghostty `bell-features` bits (0 system, 1 audio, 2 visual) (`GhosttyTerminalView.swift:2837-2892`, `P/CmuxTerminal/.../TerminalBellService.swift:30-47`). Visual-bit bell on a pane that is not the active-focus one: marks the panel manually unread (unless already) and flashes it; on the active pane: nothing (`S/Workspace+AttentionFlashRouting.swift:53-78, 153-163`). Default of `bell-features`: UNVERIFIED (Ghostty config).
- Dropped silently as "machine payloads" (`GhosttyDesktopNotificationRequest.swift:36-84`, `HookResolution.swift:17-22`): title starting `scheme://`; C0 controls (except `\n \r \t`) or 0x7f-0x9f in title+body; body that is a JSON object/array.
- Ingress: single FIFO consumer, buffer 256, **oldest dropped** on flood (`S/GhosttyDesktopNotificationIngress.swift:12-25`). No time-based rate limit for OSC. Cooldown only when a caller passes `cooldownKey` (agents): drop if within interval (`TNS:1206-1222`).
- Empty title → workspace title, else "Terminal" (`HookResolution.swift:80-83`).
- CLI `cmux notify` (`--title --subtitle --body --workspace --surface --desktop false --clear`, `docs/notifications.md:35-60`) calls the same `addNotification` (`TNS:1156`); `--desktop false` = record + ring, no banner.

## 2. Store model
- `TerminalNotification` (`S/TerminalNotification.swift:5-31`): `id` UUID, `tabId` (workspace), `surfaceId?` (nil = workspace-level), `panelId?`, `title`, `subtitle`, `body`, `createdAt`, `isRead`, `paneFlash` (default true), plus agent/click/reply/sound fields (ignore). `matches(tab,surface)` = same tab and (surfaceId or panelId equals target); nil target matches only workspace-level (`:86-92`).
- Array `notifications`, **newest first** (insert at 0, `TNS:~1676`); sort key createdAt desc then id (`TNS:2988-2993`). Indexes: total unread, unread per workspace, latest per workspace (any read state), unread surface keys (`TNS:2952-2986`).
- **Replacement/dedupe:** a new uncorrelated notification removes every existing one with the same `(tabId, surfaceId)` (read or not) — so ≤1 notification per surface (`TNS:1640-1656`). Correlated ones replace only the same correlationKey.
- **No max count** found in the store (grep of caps: none). History is also copied to a separate persisted feed store (`NotificationFeedHistoryStore`), out of scope.
- Workspace unread count = unread notifications of that workspace + 1 if any manual/restored/panel-derived workspace unread flag (`TNS:1049-1055`). Manual unread (user "Mark as unread", visual bell) is a separate flag; shows ring/badge the same.
- Muted workspace (`isMuted`): nothing recorded, no ring/sound (`TNS:1197-1203, 1530-1535`).

## 3. Read/unread rules
- **Created unread always** (`isRead = !effects.markUnread`, default markUnread true; `TNS:1546-1577`, `ND/NotificationPolicyEffects.swift:34`) — **even when the pane is focused and the app active**. Focused arrival additionally sets a "focused-read indicator" for that surface so the ring stays visible (`TNS:1655-1661`, `:302`).
- Focus state (`S/TerminalNotificationStore+FocusState.swift:6-47`): `isFocusedSurfaceArrival` = app focused (`NSApp.isActive` and key window is a main window, `S/TerminalNotificationStoreSettings.swift:81-93`) AND notification's workspace is selected in its window AND surface is that workspace's focused surface (workspace-level: surface check passes).
- Suppression at creation affects **delivery only**, not recording (§9): focused arrival → no banner, no pane flash, no workspace reorder, no sound unless `soundWhenFocused` (`TNS:1735-1760`, `S/TerminalNotificationDeliveryDecision.swift:11-62`).
- **Marked read when** (all in `ND/NotificationDismissalModel.swift:80-250`; `ND/NotificationDismissalContext.swift`):
  - Workspace selected while app active → focused panel's notifications read (`activeFocus`), plus the workspace's own workspace-level ones (`TM:421-427`, model `:60-100`). Other panes in it keep theirs (`docs/notifications.md:75`).
  - A panel gains focus while app active (`activeFocus`; explicit focus intent = `directInteraction`) (`TM:758`).
  - Any visible panel rendered (`dismissNotificationOnVisiblePanel`, `TM:4296-4304`).
  - **Typing into the terminal** clears it even if app inactive (`terminalInteraction`; `S/Workspace+AttentionFlashRouting.swift:20-40`). A focused arrival (no focus event) therefore stays until typing, a direct click/explicit focus on the pane, or app re-activation (last two UNVERIFIED for an already-focused pane).
  - App becomes active → the focused panel's unread notifications read, with a notification-arrival flash first if any has `paneFlash` (`S/AppDelegate.swift:2189-2209`).
  - Click/open from panel, popover, banner → `markRead(id)` after the focus succeeded (`AppDelegate+NotificationOpen.swift:350-362`); jump-to-unread same.
  - Explicit: popover row "Mark as Read/Unread"; workspace context "Mark Workspace as Read"; menu "Mark All Read"; `markAllNotificationsRead`.
- Mark-read keeps the record (shown as read); it does **not** delete. Surface mark-read leaves the focused-read indicator; only user dismissal clears it (`TNS:1949-1995`, `S/Workspace.swift:6103-6113`).
- **Closing:** closing a panel clears its notifications (deleted) (`S/Workspace+PanelLifecycle.swift:592`); closing a workspace clears all of its (`TM:2817`).
- `suppressOnlyFocusedSurface` (false) narrows auto-read on workspace visit to the exact focused surface (`ND/NotificationDismissalModel.swift:128-140`).

## 4. Pane ring (persistent) and flash (transient)
- Two distinct things. **Ring** = static stroke, opacity 1, shown while the panel `hasVisibleNotificationIndicator` = has unread notification OR focused-read indicator, OR manual/restored unread panel flag, OR workspace-manual-unread representative panel (`TNS:1098-1101`, `S/Workspace.swift:6196-6205`, `S/WorkspaceContentView.swift:281-289`). Gated by setting `unreadPaneRing` (§11) (`S/Panels/TerminalPanelView.swift:115`). **Shown on the focused pane too** (until typed/focused-dismissed, §3).
- Geometry (`S/Panels/Panel.swift:226-231`): rounded rect inset **2 pt** from pane bounds, corner radius **6**, stroke **2 pt** (`P/CmuxFoundation/.../CGFloat+PaneIndicator.swift:7`), round join/cap. Ring glow: shadow opacity 0.35, radius 3 (`Panel.swift:181-185`; layer `GhosttyTerminalView.swift:11104-11116`). No animation on the ring.
- Colour: attention colour = setting `paneFlashColor` hex, else the cmux accent (`S/Panels/WorkspaceAttentionColor.swift:28-36`); default accent mode `cmux` = **#0088FF light / #0091FF dark** (`P/CmuxFoundation/Tests/.../CmuxAccentColorTests.swift:10-11`). Same colour for the badge, sidebar badge and list dots.
- **Flash** (transient, drawn over the ring): same rect/colour, glow opacity 0.6 radius 6 (`Panel.swift:187-191`). Pattern chosen by `paneFlashDoubleBlink`, **default true = double blink**: opacity keyframes 0→1→0→1→0 at t 0, .25, .5, .75, 1 of **0.9 s**, ease-out up / ease-in down per segment (quadratic) (`Panel.swift:280-286, 326-344`). Single pulse (flag off): 0→1→0 at 0, .3, 1 of 0.6 s (`:274-279`). Each flash = fresh run (token), not a loop.
- Flash triggers: arrival on a non-focused pane; dismissing a notification by focus/click/typing (typing only if `paneFlashOnTyping`); app activation with unread; manual-unread jump; "Flash Focused Panel"; plain pane navigation only if no other pane has an indicator (`ND/NotificationDismissalContext.swift:25-40`, `Panel.swift:200-222`). Arrival on the focused surface: no flash (`DeliveryDecision.swift:33-35`). Re-trigger during a running notification flash is ignored (`S/Panels/TerminalPanel.swift:827-836`). `paneFlash` setting off = no flashes.

## 5. Tab-strip highlight (pane tab)
- cmux sets a boolean `showsNotificationBadge` per Bonsplit tab = same `shouldShowUnreadIndicator` rule as the ring (`S/Workspace.swift:5836-5849`). Visual (dot shape/colour/position) is in Bonsplit: UNVERIFIED. Updated whenever the panel's unread state changes.

## 6. Sidebar row
- **Latest-notification text** (`TNS:650-669`, `CV:16519-16535, 16687-16696`): latest notification of the workspace (**read or unread**, newest), text = `body` if non-empty else `title`, whitespace-trimmed; nil if empty. Rendered as a second block below the title row, **10 pt regular**, secondary colour at 0.8 opacity (inverted/white-ish on the selected row), left-aligned, wraps, `lineLimit` = `sidebar.notificationMessageLineLimit` (default **12**, range 1…50), tail truncation, display bounded to 4096 chars; markdown flattened to plain text. Subtitle field not shown. Hidden when `sidebar.showNotificationMessage` false, or in compact-status-glyph mode.
- **Unread badge** (`CV:16544, 16560-16565`, `S/Sidebar/SidebarWorkspaceUnreadBadge.swift`): filled circle, diameter **16 × fontScale**, text = count (workspace unread count, §2) in **9 pt semibold** white, fill accent (or `sidebar.notificationBadgeColor` hex; selected-row inverted: 25% of primary text, text = primary). **No "99+" cap in the sidebar badge**; shown only when count > 0 and not compact-glyph mode. Position by `sidebar.notificationBadgePosition`, default **leading** (left slot, same slot as the agent spinner, badge hidden under the spinner) else trailing (shares the close-button slot) (`SC:179-184`, `S/Sidebar/SidebarWorkspaceLeadingStatusSlot.swift:16-38`).
- Workspace moves to **top of its unpinned tier** when a notification arrives (`app.reorderOnNotification` default `notifications`; modes off | notifications | agentActivity) unless it targets the focused surface (`S/Cloud/Sidebar/TerminalNotificationPolicyEffects+Sidebar.swift:11-29`, `AppCatalogSection.swift:173-177`, `TM:2060`).

## 7. Notification panel
- Three UIs, same list: **titlebar popover** (NSPopover, semitransient) opened by the titlebar bell or `showNotifications` (`S/AppDelegate.swift:15834-15837`, `S/Update/UpdateTitlebarAccessory.swift:1160-1180, 2338-2345`), a page tab (`S/NotificationsPage.swift`) and a menu-bar extra. Port target = popover.
- Popover size: default **560×760**, min 420×320, max 1000×1200, user-resizable via a 16 pt corner handle; size persisted (`cmux.notifications.popover.width/height`) (`S/Update/NotificationsPopoverMetrics.swift:3-10`, `UpdateTitlebarAccessory.swift:2376-2481`). Anchored to the bell button.
- Header (`UpdateTitlebarAccessory.swift:2489-2560`): "Notifications" 14 semibold; unread capsule (11 semibold white on accent) if unread > 0; right: "Jump to Latest" (icon + shortcut chip; disabled when none unread) and "Clear All" (disabled when empty); padding 14/10.
- Row (`S/Update/NotificationPopoverRow.swift`): min height **56**; left 2.5 pt accent bar when unread (clear when read); line 1 = workspace title 12.5 semibold (fallback notification title), time (`.shortened` time) 10.5 secondary at right; line 2 (only if workspace title shown) = notification title 10.5 medium secondary; line 3 = body 11.5 secondary, 2 lines max. Hover tints row (0.11) and reveals a 20 pt round "x" at right. Hairline dividers. Subtitle not shown. Newest first.
- Actions: **click row = jump to the surface** (focus its window/workspace/surface, mark read, restore scroll position), then **popover closes** (`:2749-2756`). "x" = remove that one (deleted, 0.18 s ease-out). Context menu: Open, Copy, Mark as Read/Unread, Dismiss. Clear All = `clearAll()` (delete everything). There is no header "Mark all read" button; it exists as menu item/unbound shortcut (§10).
- Empty state: bell.slash icon, "No notifications yet", "Desktop notifications will appear here." (`:2593-2597`). If only workspace-level unread flags exist: bell.badge + `stateHintTitle` text (UNVERIFIED wording, `MenuBarExtraController.swift:407`).

## 8. Jump to latest unread (`jumpToUnread`)
- Algorithm (`ND/NotificationNavigationCoordinator.swift:jumpToLatestUnread`, `ND/NotificationNavSnapshot.swift:83-92`): scan the store array in order (**newest first, across all workspaces/windows**) for the first notification that is unread, not excluded, and has **no click action**; try to open it; on failure continue to the next. Opening = bring its window forward, select workspace, focus the surface (`S/AppDelegate+NotificationOpen.swift:173-240`), then mark that notification read (completion), which also fires the normal focus dismissal + dismiss flash; restores scroll row.
- If none: try a window-dock manual-unread target; else open the first workspace with a manual/restored unread flag (window order), focusing `preferredUnreadPanelIdForJump` (latest manual-unread panel, else restored, else representative) and clearing that unread (`Workspace.swift:6088-6112`). If nothing at all: no-op, **no beep, no UI**.
- `markOldestUnreadAndJumpNext`: marks the focused panel's latest notification unread and moves it to the oldest position (`TNS:2031-2062`), then jumps excluding it — cycles unreads. `toggleUnread` flips the focused item's read state (`ND/FocusedNotificationMarker.swift`).

## 9. Desktop notifications
- Posted via `UNUserNotificationCenter` (`TNS:2507-2670`), only when effects.desktop and **not suppressed**. Suppressed (banner/sound/command withheld, still recorded) when the arrival is the exact focused surface of the active app; with `suppressWhenAppFocused` (default false) suppressed for **any** notification while cmux is the active app (`FocusState.swift:21-29`). So default: banner shows when app inactive, **or** the surface is not the focused one (other pane, other workspace) even while app active.
- Content: title (empty → app name), subtitle, body (`TNS:2491-2497, 2557-2560`). Identifier = notification UUID; a replaced notification's older banner is withdrawn by id (`TNS:1672-1700`). userInfo: tabId, surfaceId, notificationId.
- Presentation while app is frontmost: `[.banner, .list]` (+`.sound` if sound set and not kept quiet) (`ND/NotificationDeliveryCoordinator.swift:80-95`). Quiet if surface became focused before presentation (`TNS:1724-1731`).
- Sound: setting `notifications.sound` default `"default"` (system alert) / custom file / none; plays only if effects.sound; focused pane silent unless `soundWhenFocused` (default false). `notifications.command` (shell) runs after a successful schedule; permission denied = sound only.
- **Click (default action)** → activate app, then open notification: focus window/workspace/surface, mark read, restore scroll (`NotificationDeliveryCoordinator`, `AppDelegate+NotificationOpen.swift:350-362`). Dismiss action: nothing. Withdrawn automatically when its notification is marked read/removed (`removeNotificationRequests…`, `TNS:498`).
- Banner lifetime is OS-controlled (`docs/notifications.md:96-107`). First notification triggers the permission request, (`TNS:2754-2880`). Dock badge text = unread count, **"99+" above 99** (`TNS:526-534`); titlebar bell badge = `min(count, 99)` without "+" (`UpdateTitlebarAccessory.swift:995`), badge 11–13 pt circle, accent, white text, offset (+3,−3) from bell top-right (`:67-107`).

## 10. Shortcuts (Cmd → Super on Linux; `KSS`; `enum Action: String`, rawValue = case name, `KSS:82`)
| rawValue | Label | Default | KSS (case / label / default) |
|---|---|---|---|
| `showNotifications` | Show Notifications | Cmd+I | 112 / 272 / 500 |
| `jumpToUnread` | Jump to Latest Unread | Cmd+Shift+U | 113 / 273 / 502 |
| `toggleUnread` | Toggle Unread | Cmd+Opt+U | 115 / 275 / 506 |
| `markOldestUnreadAndJumpNext` | Mark as Oldest Unread and Jump to Next Latest Unread | Ctrl+Cmd+U | 116 / 277 / 508 |
| `markAllNotificationsRead` | Mark All Notifications Read | unbound | 117 / 279 / 510 |
| `clearAllNotifications` | Clear All Notifications | unbound | 118 / 281 / 510 |
| `triggerFlash` | Flash Focused Panel | (see KSS:528) | 126 / 289 / 528 |
- Handlers `S/AppDelegate.swift:15834-15937`; palette entry (`CV:7929`); menu items (`S/cmuxApp.swift:622-640, 1316-1323`).

## 11. Settings (id → default; `NS`, `SC`)
- `notifications.dockBadge` true; `notifications.showInMenuBar` true.
- `notifications.unreadPaneRing` true; `notifications.paneFlash` true; `notifications.paneFlashDoubleBlink` **true**; `notifications.paneFlashOnTyping` true; `notifications.paneFlashThemeColor` false (flash uses theme foreground when no custom colour); `notifications.paneFlashColor` "" (= accent).
- `notifications.sound` "default"; `notifications.soundWhenFocused` false; `notifications.customSoundFilePath` ""; `notifications.soundOverrides` "{}"; `notifications.command` "".
- `notifications.suppressOnlyFocusedSurface` false; `notifications.suppressWhenAppFocused` false.
- Agent/hook settings skipped.
- `sidebar.showNotificationMessage` true; `sidebar.notificationMessageLineLimit` 12; `sidebar.notificationBadgePosition` leading; `sidebar.notificationBadgeColor` "" (`SC:68-78, 179-184, 234-238`); `app.reorderOnNotification` notifications.
- Per-workspace mute (`isMuted`), context menu "Mark Workspace as Read/Unread" (`S/TabItemView+WorkspaceNotificationsMenu.swift`).

## 12. Linux mapping notes (no macOS-free equivalent)
- `UNUserNotificationCenter` categories/actions/identifier-replacement, banner-vs-alert, permission prompt, `NSApp.isActive`/key-window focus test, Dock badge, NSPopover semitransient + titlebar accessory, `NSSound.beep`: need freedesktop notifications (replace-id, default action, optional sound hint), a window-focus test, and own popover. Dock badge and menu-bar extra have no direct Linux analogue.
- Ghostty-side behaviour (OSC 9;4 progress, OSC 99 keys, 255-byte truncation, `bell-features` default) lives in the Ghostty core; if cmux-linux embeds libghostty it comes for free, otherwise it needs its own parser (UNVERIFIED details).
