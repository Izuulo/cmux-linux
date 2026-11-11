# m6-parity-release — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-10 — Plan written

- Wrote `plan.md` (8 tasks) on the macOS host. Reference: cmux `Sources/` sparse clone at `43ce5e2` (2026-11-10).
- Found while planning:
  - The macOS cmuxOnly ancestry check already exists (`server.rs` `peer::parent_of`, libproc); removed from the M6 list.
  - cmux reads ports with libproc, not `lsof` (`Sources/ListeningPortLookup.swift`); libc 0.2.189 lacks `socket_fdinfo` and its constants, so the plan mirrors them (D1).
  - `setsid` and GNU `timeout` don't exist on macOS; the PTY exit test is ported with `trap '' HUP` (Task 3).
  - An install in `/usr/bin` would put `/usr/bin` first on every terminal's `PATH` (M5 D8), so the package installs to `/usr/lib/cmux-linux/` (D5).
- Spec gives no performance numbers; the plan sets them (D8). M5's deferred CLI commands stay out unless the owner adds them to the spec.

## 2026-11-10 — Task 3
- Plan D2 (kqueue exit watch on macOS) dropped: macOS revokes the slave when the session leader exits (python pty probe: master reads EOF at once while a setsid'd `sleep` lives), so no exit watch is needed there. `pidfd` stays Linux-only; the shell-exit test now runs on macOS (perl setsid grandchild) as a guard for that behaviour.

## 2026-11-10 — Task 6 parity checklist

- Reference: cmux source `manaflow-ai/cmux` @ **43ce5e2** (`Sources/`, plus `Packages/macOS/{CmuxFoundation,CmuxAppKitSupportUI,CmuxSettings,CmuxTerminalCore}`, all GPL), Bonsplit (MIT) @ its pinned gitlink **58b73fd** (unchanged since M2). Shorthands: `S/` = cmux `Sources/`, `P/` = cmux `Packages/macOS/`, `B/` = `bonsplit/Sources/Bonsplit/`.
- Method (D9, amended by the controller): the owner's cmux.app was never driven; its window was only screenshotted read-only. It runs a non-default configuration, so its window was not a clean reference; Swift values win everywhere. Ours ran from `target/release` on a scratch socket with `XDG_CONFIG_HOME` pointing at an empty config (Ghostty default theme) and at `theme = Apple System Colors` (cmux's default dark theme, via cmux.app's theme dir) for the dark pair; layout 2 workspaces, 2 × 2 splits, one unread notification on a background pane, 1100 × 700 pt.
- Not captured: a 3-tab strip and the open panel need key/mouse events; posting events to our pid needs the macOS PostEvent permission (prompt), so both are compared from source only. `flash.mov` skipped (flash = source values below). The ring is not on screen in cmux; source only.
- Screenshots (`~/Pictures/cmux-m6/`, not committed): `cmux-window-raw`, `cmux-sidebar`, `cmux-tab-strip`; `ours-window-before`, `ours-window`, `ours-sidebar`, `ours-tab-strip`, `ours-splits`, `ours-ring`, `ours-window-dark-theme`, `ours-sidebar-dark-theme`.
- Pixel check: cmux's sidebar border is #353336 on its #262427 backdrop = our `chrome::separator` (+0.16 @ 0.36) exactly; our selected row samples #0091FF.

| Area | Item | Ours (file: const) | cmux (source) | Result |
|---|---|---|---|---|
| sidebar | width default / min / max | `window_view.rs` `SIDEBAR_DEFAULT/MIN/MAX` 240/240/600, `CONTENT_MIN` 300 | `S/SessionPersistence.swift:26-31` | ✓ |
| sidebar | resize band 6 in / 4 out | `sidebar.rs` `RESIZE_IN/OUT` | `S/Sidebar/SidebarState.swift:72,76` | ✓ |
| sidebar | band 28, first row 30 | `BAND_H`, `LIST_TOP` | `S/WindowChromeMetrics.swift:5-7,90` | ✓ |
| sidebar | row pad outer 6 / inner 10 / vertical 8 | `OUTER_PAD_X`, `ROW_PAD_X`, `ROW_PAD_Y` | `S/WindowChromeMetrics.swift:92-94` | ✓ |
| sidebar | row spacing 2, section spacing 4, radius 6 | `ROW_GAP`, `SECTION_GAP`, `RADIUS` | `S/ContentView.swift:11641`; `S/Sidebar/AppKitList/Cells/SidebarWorkspaceRowCellView.swift:1274,319` | ✓ |
| sidebar | title 12.5 semibold; detail 10 mono; message 10 regular | `TITLE_PX`, `DETAIL_PX` | `…RowCellView.swift:624,693`; `…RowSupportViews.swift:416` | ✓ |
| sidebar | badge↔title and title↔close gap 8 | was `ITEM_GAP+2` (6) / `ITEM_GAP` (4) → `TITLE_ROW_GAP` 8 | `…RowCellView.swift:1277-1278,1301,1338` | fixed |
| sidebar | close slot 16, reserved while closable; glyph 9; tint secondary(0.7) | was glyph only → `CLOSE_SLOT` 16, `CLOSE_ALPHA` 0.7 | `…RowCellView.swift:1336-1340,640-642` | fixed |
| sidebar | row badge: fixed 16 circle, 9 semibold, accent / primary@0.25 when selected | was min-width pill → `.size(BADGE)` | `…RowCellView.swift:786-793,1280`; `…RowSlotViews.swift:51` | fixed |
| sidebar | message colour secondary(0.8) = plain secondary / white@0.8 selected | was detail × 0.8 | `…RowCellView.swift:694`; `…RowSupportViews.swift:51-62` | fixed |
| sidebar | dir lines / PR lines 1 apart inside a block; ports wrap, gap 4 | was 4 between every line, no wrap → `LINE_GAP` 1 | `…RowCellView.swift:1455-1500` | fixed |
| sidebar | branch line "branch · dir", secondary(0.75); PR 10 semibold, icon+4+title+4+word, stale 0.5 | `row_model`, `ITEM_GAP` | `…RowSupportViews.swift:416-427,558-614` | ✓ |
| sidebar | selected fill = solid accent (#0088FF / #0091FF), white text; detail 0.75 | `chrome::sidebar_palette`, `cmux_accent` | `S/Sidebar/SidebarAppearanceSupport.swift:405-417,440`; `P/CmuxSettings/…/WorkspaceColorsCatalogSection.swift:5-24`; `P/CmuxFoundation/…/CmuxAccentColor.swift:49-56` | ✓ |
| sidebar | label alphas: title labelColor 0.85, detail secondaryLabel 0.5/0.55 | `sidebar_palette` | `…RowSupportViews.swift:47-62` (macOS label colours) | ✓ |
| sidebar | trailing border 1 px separator | `border_r_1`, `p.border` | `P/CmuxAppKitSupportUI/…/WindowChromeBorder.swift:23-24,50` | ✓ (pixel-checked) |
| sidebar | band buttons 20, icon 12, radius 6, spacing 6 | `BUTTON`, `BUTTON_ICON`, `RADIUS`, `BUTTON_GAP` | `S/WindowChromeMetrics.swift:23-26`; `S/Update/UpdateTitlebarAccessory.swift:63-74` | ✓ |
| sidebar | band icon tint secondary@0.86, hover 0.96, pressed 1.0 | was detail, press dims 0.72 → `ICON_ALPHA`, `ICON_HOVER_ALPHA` | `S/RightSidebarChromeStyle.swift:16-21`; `UpdateTitlebarAccessory.swift:1418-1420` | fixed (pressed 1.0 not modelled) |
| sidebar | bell badge 12 circle, label max(7, 12−6)=7 semibold, offset (3, −3), cap 99 | was 13 / 9 pt pill → `BELL_BADGE` 12, `BELL_BADGE_PX` 7 | `UpdateTitlebarAccessory.swift:67-68,258-259,987-1005` | fixed |
| sidebar | band position | ours: band inside the sidebar, buttons right-aligned | cmux: 28 pt titlebar across the whole window after the traffic lights, leading pad 4 (`S/WindowChromeMetrics.swift:27`) | gap: layout (M3 D3), Linux has no traffic lights |
| sidebar | list top/bottom scroll fades, Chief Home row | none | `S/WindowChromeMetrics.swift:72-99` | gap: behaviour / out of scope (Chief) |
| tab strip | height 28 | `TAB_BAR_HEIGHT` | `S/WindowChromeMetrics.swift:7`; `S/Workspace.swift:4159` | ✓ |
| tab strip | min 48 / max 220, padding 6, spacing 0 | `TAB_MIN_W`, `TAB_MAX_W`, `TAB_PAD` | `B/Internal/Styling/TabBarMetrics.swift:13-17`; `B/Internal/Views/TabItemView.swift:224-225` | ✓ |
| tab strip | icon 14 (glyph 11.5), gap 6, title 11 regular | `ICON_SLOT`, `ICON_GLYPH`, `CONTENT_GAP`, `TITLE_PX` | `TabBarMetrics.swift:24-25,30`; `TabItemView.swift:652,976` | ✓ |
| tab strip | text alphas active 0.82, inactive 0.62 light / 0.68 dark | `chrome::tab_palette` | `B/Internal/Styling/TabBarColors.swift:7-10,96-109` | ✓ |
| tab strip | active fill / hover fill | was bg ±0.12 / ±0.07 @ 0.78 → clear / black@0.055, white@0.075 | cmux always shares the backdrop (`S/Workspace.swift:3886-3888,4166`) → `TabBarColors.swift:177-179,194-199` | fixed |
| tab strip | active indicator 1.5 top, inset 1 right, controlAccentColor, greyscale when unfocused | `INDICATOR_H`, `chrome::accent`, `desaturate` | `TabBarMetrics.swift:18-19`; `TabBarColors.swift:342-344`; `B/Internal/Views/TabBarView.swift:1089` | ✓ |
| tab strip | separators: right edge + bottom except under selected; colour = window separator | `border_r_1`, `border_b_1`, `separator` | `S/Workspace.swift:3995-4013`; `B/Internal/Views/TabBarSelectionChromeView.swift:81-119` | ✓ |
| tab strip | unread dot 6, systemBlue, not desaturated, only on unselected unhovered tab | was cmux accent → `chrome::accent` | `TabBarMetrics.swift:29`; `TabBarColors.swift:363-366`; `TabItemView.swift:1159-1165` | fixed |
| tab strip | close slot 16, glyph 9 semibold, inactive→active text on hover with hover-fill circle; shown on selected or hovered | `CLOSE_SLOT`, `CLOSE_GLYPH` | `TabBarMetrics.swift:26-27`; `TabItemView.swift:1184-1203` | ✓ (glyph weight: svg stroke) |
| tab strip | split lane: lead 6, trail 8, button 22, spacing 4, glyph 12 | `LANE_*`, `BUTTON_*` | `TabBarView.swift:186-189` | ✓ (browser button skipped, M2) |
| splits | divider 1 px; hit 8 each side | `cmux_core::geometry::DIVIDER`; `workspace.rs` `DIVIDER_HIT` | `TabBarMetrics.swift:41`; `S/PortalSplitDividerRegion.swift:57` | ✓ |
| splits | divider colour: Ghostty `split-divider-color` else separator | `chrome::divider` | `S/Workspace.swift:3967-3975,4010-4013` | ✓ |
| splits | unfocused dim: `unfocused-split-fill` or bg at 1 − clamp(opacity 0.7, 0.15…1) | `chrome::dim_overlay` | `P/CmuxTerminalCore/…/Config/GhosttyConfig.swift:65,193-202`; `S/Panels/TerminalPanelView.swift:114` | ✓ |
| ring | inset 2, radius 6, stroke 2 | `attention.rs` `INSET`, `RADIUS`, `STROKE` | `S/Panels/Panel.swift:227-229`; `P/CmuxFoundation/…/CGFloat+PaneIndicator.swift:7` | ✓ |
| ring | colour = cmux accent; glow 0.35 / 3 (ring), 0.6 / 6 (flash) | `attention_color`; glow as wider faint stroke | `Panel.swift:180-190` | ✓ (glow approximated, M4) |
| ring | flash: double blink default, 0→1→0→1→0 at .25 steps, 0.9 s, ease-out up / ease-in down | `FLASH`, `flash_opacity` | `Panel.swift:274-285,309-339`; `S/TerminalNotificationStoreSettings.swift:28` (`defaultDoubleBlink = true`; the doc comment at `Panel.swift:265` says pulse, the value wins) | ✓ |
| panel | size 560 × 760 (min 420 × 320, max 1000 × 1200) | `WIDTH`, `HEIGHT` | `S/Update/NotificationsPopoverMetrics.swift:4-9` | ✓ size; resize handle = gap (behaviour) |
| panel | header "Notifications" 14 semibold, pad 14/10, capsule 11 semibold px 6 py 1 | `HEADER_PAD_*` | `S/Update/NotificationsPopoverView.swift:188-197,258-259` | fixed (capsule py) |
| panel | header buttons 11 pt, pad 8/4, radius 5, fill secondary@0.12 (0.05 disabled), secondary text when disabled | was 12 pt, 8/3, r6, title@0.08, opacity 0.4 | `NotificationsPopoverView.swift:209,225-252` | fixed |
| panel | "Jump to Latest" icon + shortcut chip; phone-forwarding row | none | `NotificationsPopoverView.swift:201-224,262-280` | gap: icon/chip (new UI); phone row out of scope |
| panel | row min 56; lead 4; unread bar 2.5 inset 6; content pl 10 py 8; line gap 2; title–time gap 6; time pad 34 | was bar full height, px 12, gap 8 → `ROW_LEAD`, `UNREAD_BAR_INSET`, `CONTENT_PAD_*`, `TIME_PAD_R` | `S/Update/NotificationPopoverRow.swift:25,114-180` | fixed |
| panel | fonts 12.5 semibold / 10.5 / 10.5 medium / 11.5 ×2 lines; hover primary@0.11 | as before | `NotificationPopoverRow.swift:43,124-170` | ✓ |
| panel | clear button 20 circle primary@0.1, xmark 9 primary@0.7, trailing 10, centred | was top-right 8, fill on hover only | `NotificationPopoverRow.swift:66-67,185-191` | fixed |
| panel | dividers between rows only, @0.4, leading 18 | was bottom border on every row | `NotificationsPopoverView.swift:355-358` | fixed |
| panel | empty state bell.slash 30 @ secondary 0.7, gap 10, 14 medium, 12 centred, pad 24 | was 28 / gap 6 / 13 semibold / 11.5 | `NotificationsPopoverView.swift:292-297,415-429` | fixed |
| panel | background windowBackgroundColor, NSPopover chrome | terminal bg, 8 radius, shadow | `NotificationsPopoverView.swift:77` | gap: no portable system colour |
| window | sidebar/content backdrop = terminal bg (matchTerminalBackground) | `sidebar_palette(bg)` | `S/Workspace.swift:3886-3888` | ✓ |
| window | separators on default theme (#282c34) and dark theme (#1e1e1e) | `chrome::separator` | `P/CmuxAppKitSupportUI/…/WindowChromeColorResolver.swift:24-42` | ✓ |
| window | default theme with no Ghostty colours: cmux injects "Apple System Colors" (dark, #1e1e1e/#ffffff) / "…Light" by appearance | ours: Ghostty default #282c34 | `P/CmuxTerminalCore/…/Config/GhosttyConfig.swift:25-28,451-458`; `P/CmuxSettings/…/TerminalCatalogSection.swift:110-113`; `S/GhosttyTerminalView.swift:1278-1295` | gap: needs a bundled theme + system appearance detection (new behaviour) |

- Gaps above need behaviour, not values; none is under ~30 lines except the pressed-icon opacity, which gpui's `active` cannot express on a child svg without new state.

**Addendum (review round 1)**

| Area | Item | Ours (file: const) | cmux (source) | Result |
|---|---|---|---|---|
| sidebar | row height: content-driven, 8 above the title, blocks 4 apart, 8 below; no fixed height | `sidebar.rs` `ROW_PAD_Y` 8 (py), `SECTION_GAP` 4, rows `flex_none` | `S/Sidebar/AppKitList/Cells/SidebarWorkspaceRowCellView.swift:1273-1274,1525` (`layoutContent` starts y at 8, adds 8 at the end; the table caches the measured height, `S/Sidebar/AppKitList/SidebarWorkspaceTableRowHeightCalculator.swift`) | ✓ |
| ring | tab highlight: the pane's tab gets Bonsplit's `showsNotificationBadge` under the same unread-indicator rule as the ring; Bonsplit draws it as the 6 pt dot on an unselected, unhovered tab | `workspace/tab_strip.rs` dot when `!selected && attention.contains(surface)` (`attention` = the ring set), `UNREAD_DOT` 6 | `S/Workspace.swift:5838-5850` (`syncUnreadBadgeStateForPanel`); dot: `B/Internal/Views/TabItemView.swift:1159-1165` | ✓ (colour fixed in 3518001) |

- Provenance of the `B/` (Bonsplit) rows and the two `parity(tab-strip)` commits: Bonsplit is not in the cmux sparse clone. I read it from a separate clone of https://github.com/manaflow-ai/bonsplit checked out at `58b73fde506ef6aa5fc371b8990e528ac83f0929`, the `vendor/bonsplit` gitlink of cmux 43ce5e2. Its `LICENSE` is MIT, not BUSL-1.1. Values were read for matching only; no code copied. The commit messages' `TabBarColors.swift:363` and `TabItemView.swift:1159-1165` are paths under `Sources/Bonsplit/Internal/Styling/` and `…/Internal/Views/` in that clone.

## 2026-11-10 — Task 7 performance (macOS)

Host: an Apple Silicon Mac, release build. Window 1100 x 732 pt (default, not enlarged); 4 rows x 5 columns = 20 panes fit (`scripts/perf-panes.sh`, rows split first, then columns; splitting columns first nests later rows under one column and hits the 160 pt minimum). 19 panes ran `while :; do seq 100000; done`; 54 + 50 keys typed into the 20th pane via cmux-cua `type_text` (synthetic key events do produce `latency key->paint` lines; 104 samples).

| Run | p50 | p95 | max | RSS | CPU |
|---|---|---|---|---|---|
| 20 panes, 60 s in | 8.5 ms | 10.5 ms | 17.8 ms | ~80 MB | 42 % (`ps` decayed average; ~270 % in a 5 s probe run) |
| after ctrl+c to 19, +5 s | | | | ~66 MB | 0.1 % (`ps` and `top`) |
| idle, 1 pane | 10.8 ms | 13.5 ms | 14.6 ms | | |

Verdict vs D8: p50 <= 16.7 ms pass, p95 <= 33 ms pass, RSS <= 1 GiB pass, CPU < 5 % after stop pass, idle p95 <= 16.7 ms pass. Caveats: the 60 s `ps` CPU is a decayed average, not an instantaneous reading; socket path was /tmp/cmux-perf.sock because the scratchpad path exceeds macOS SUN_LEN.

### Addendum 2026-11-10 — Task 7 memory and CPU re-measure

- Memory: `footprint <pid>` (phys_footprint) on a fresh 20-pane flood run: 70 MB at 60 s under load and 70 MB after the stop (ps RSS 72.7 MB / 70.8 MB). So the earlier ~80 MB RSS was not hiding compressed pages. Dirty: ~27 MB malloc, ~16 MB app tag 1, ~12 MB IOSurface.
- `SCROLLBACK = 10_000` is **bytes**, not lines. The C header (`vendor/ghostty/include/ghostty/vt/terminal.h:188`) says "lines", but the value goes straight to `Screen.Options.max_scrollback` (`vendor/ghostty/src/terminal/Screen.zig:264`, "maximum size of scrollback in bytes", rounded up to a page). So each terminal keeps about one page of history, far less than 10 000 lines. D8's "10 000 lines x 20" memory premise is wrong: the 70 MB figure is with almost no scrollback, and the RSS target is not yet proven for real history. Constant not changed; follow-up: map `scrollback-limit` (bytes) or size it for ~10 000 lines and re-measure.
- CPU: `scripts/perf-panes.sh` now measures CPU over a 5 s window (Linux: /proc utime+stime ticks / CLK_TCK; macOS: `top -l 2` second sample). Under load 43.6 %, after stop (from +1 s) 0.1 %. The 2nd macOS sample is a delta over the window, so these are real window readings. Linux not run here.
- Idle p50 (10.8 ms) above loaded p50 (8.5 ms) is unexplained; a guess is that the render loop is already awake under load while idle must wake from sleep. Not investigated.

### Addendum 2026-11-10 (2) — Task 7 scrollback fixed to 50 MB

- `SCROLLBACK` in `crates/cmux-term/src/lib.rs` is now 50_000_000 bytes (Ghostty default `scrollback-limit`, `vendor/ghostty/src/config/Config.zig:1390`). Before, with 10_000 bytes, an 80x24 terminal kept only ~860 of 5 000 numbered lines (earliest reachable: 4138). New test `five_thousand_lines_stay_in_history`: RED before, GREEN after. Nothing else in cmux-term/cmux-app uses the constant. Config-key mapping still deferred to M10.
- Re-run (same 20-pane flood, release build): footprint 971 MB at 60 s under load and 971 MB after stop (ps RSS 312 MB then 476 MB, so RSS does under-report here); CPU 36.0 % under load, 0.1 % after stop; 104 keys: p50 8.9 ms, p95 10.7 ms, max 70.3 ms (one outlier).
- Worst case: 20 panes x 50 MB = 1 000 MB of history, plus ~70 MB base, above D8's 1 GiB target at the limit. At 60 s the 19 flooding panes had reached ~971 MB footprint, i.e. about 1 GiB-line already (1 GiB = 1024 MB), and a longer run would exceed it. Verdict: D8 memory target is met at 60 s by a narrow margin (about 5 %) and is not guaranteed beyond; the owner may want a lower default limit or a per-app cap.

## 2026-11-11 — Final review fixes

- Deviation from cmux: `SCROLLBACK` is 10_000_000 bytes (upstream Ghostty default), not cmux's 50 MB. Our history is never compressed (libghostty-vt leaves compression to the caller, `vendor/ghostty/include/ghostty/vt/terminal.h:43-52`), so 20 full panes at 50 MB would hold ~1 GB. Bound now: 20 x 10 MB + ~70 MB base = ~270 MB. Footprint not re-measured after this change.
- Sidebar unread badge: checked `Sources/Sidebar/SidebarWorkspaceUnreadBadge.swift` (cmux @ 43ce5e2): a fixed-side circle with `Text("\(unreadCount)")`, no cap and no pill growth. Ours already matches (`badge_label`, test `badge_label_has_no_cap`), so no change.

## 2026-11-11 — Task 8 at the KDE desktop (agent-run part)

Run by the agent on the CachyOS desktop (Plasma 6.8.2, Wayland), driving the app over the socket (`CMUX_SOCKET_MODE=allowAll`) and reading window screenshots. First time any M2–M7 UI was seen drawn on Linux. No mouse or keyboard was available to the agent, so click, drag and typing checks stay with the owner.

- **New: `debug.shortcut.simulate {combo}`** on the socket (cmux's test-only method, `tests_v2/cmux.py simulate_shortcut`). Parses the chord with gpui (`cmd`/`super`, `opt` → alt) and dispatches it to the window after the current update. Needed because the right sidebar opens only by shortcut. No CLI command; send the raw v2 request.
- **Test fix:** `spawned_shell_has_no_inherited_cmux_vars` timed out on this machine: it ran the user's `$SHELL`, whose startup opened a prompt wizard that ate the typed-ahead `env; exit`. The test now sets `SHELL=/bin/sh`. Full suite green on Linux afterwards.
- **Step 1 (install):** `makepkg -si` not run (needs sudo). `makepkg -f -d` result is in the addendum below. From the cargo build: `command -v cmux` in a pane → our `cmux` next to the app binary. `echo $PATH | cut -d: -f1` showed the user's `~/.local/bin`, not our dir: bash's startup files prepend it after we do (the README case); the right `cmux` still wins here.
- **`cmux notify`:** pane ring, sidebar message, row badge and bell badge drawn. D-Bus `Notify` seen with app name `cmux-linux`, summary/body as sent, action `default`/`Open`, hint `desktop-entry=cmux-linux`. Bubble itself not looked at.
- **Step 2 (perf, release build, 20 panes, 30 s soak):** `scripts/perf-panes.sh` Linux path works. Under load RSS 483 MB, CPU 649 % of one core over 5 s (19 `seq` floods on a 16-thread machine); after stop RSS 484 MB, CPU 0.8 %. D8: RSS ≤ 1 GiB pass, CPU < 5 % after stop pass. Key latency (`CMUX_TRACE_LATENCY`, M1 exit 3) not measured: needs typing.
- **Step 3 (parity spot check):** window, sidebar (selected row fill, detail lines, badge), tab strip with its three buttons, split dividers, unfocused-pane dim and the ring all draw as designed with no Ghostty config (default theme). KWin offers `ext_background_effect_manager_v1` (version 1), so the Plasma ≤ 6.6 fallback is not needed here. With a scratch config (`background-opacity = 0.7`, `background-blur = true`) the app starts, draws translucent and logs nothing: none of the rg §1.4 blur-shim crashes. Whether the backdrop is actually blurred can't be told from a window-only capture.
- **M7 Feed on screen:** permission card (four buttons, command box, description), question card (options, disabled submit), header badge, Claude and age chips, and the empty state all draw. Buttons not clicked.
- **"Opening the right sidebar wipes the terminal" (M7 risk): not a bug.** Each terminal gets exactly one resize (42 → 27 columns here) and keeps its content. The two extra prompt lines appear only when the narrowed pane is exactly as wide as the bash prompt (27 columns): readline redraws a prompt that now wraps. Ghostty avoids this with its shell integration's prompt marks (OSC 133), which we don't ship.
- **Step 4** (M2–M5 desktop checks): the socket-driven parts are covered above (splits, tabs strip, workspaces, sidebar metadata with git branch, notifications). Divider drag, tab drag, row drag, rename typing, IME, and clicking a bubble remain for the owner.

### Addendum 2026-11-11 — package build

`makepkg -f -d` in a copy of `packaging/arch` builds `cmux-linux-git-r237.f314c00-1-x86_64.pkg.tar.zst` (8.8 MB) from the pushed `master`. `-d` because Rust here comes from rustup.rs, not the `rustup` package the PKGBUILD lists. File list: `/usr/lib/cmux-linux/{cmux-linux,cmux}`, the two `/usr/bin` symlinks, the desktop entry, and five licence files, as planned in Task 5. Not installed (sudo), `namcap` not available.
