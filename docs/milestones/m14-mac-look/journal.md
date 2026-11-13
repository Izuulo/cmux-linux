# M14 mac-look — Journal

## 2026-11-12 — spec, plan, build

- Owner asked for the app to look and feel native to macOS (newest, 26 Tahoe), even on Linux: rounded popovers with a soft arrow and an open animation, menus, a System Settings–like Settings window, cmux's title bar (folder icon + workspace title), the browser chrome closer to cmux, and the missing new-browser button in the pane tab strip. Owner accepted "no real blur" and chose approach 1 (a `mac` module in `cmux-app`, not a separate crate or a widget library) so new UI gets the look by default.
- Reference: cmux @ `0fd2e596`: `ContentView.customTitlebar` (title bar), `WindowChromeMetrics` (28 px bars), `BrowserChromeMetrics` / `BrowserPanelView` (omnibar 12 pt text, pill radius 10), `CmuxSurfaceTabBarBuiltInAction` (button order terminal, browser, split right, split down). Read only; nothing copied.
- Built with one implementer agent per task and a reviewer per task (Tasks 3–4 and 6–7 paired). All 11 tasks done; commits on `m14-mac-look`.

### Deviations from the plan
- `popover` takes `window_w` and keeps the arrow on the anchor while the body slides to stay inside the window (`popover_left`, tested). The plan centred the arrow on the body, which points at the wrong place near the window edge (the bell is near the left edge).
- `Fade::leave_from(prev, now)`: closing mid-enter fades out from the current amount instead of jumping to opaque.
- Glass is **opaque**, not 0.94: a KDE screenshot showed sidebar text readable through the notification popover. Without blur any see-through reads as a bug. (Plan K4 updated.)
- Arrow drawn **after** the body, overlapping its top border by 1 px: the screenshot showed the body's border crossing the arrow's base.
- A leaving popover gets a handler-less cover so its buttons ("Clear All") can't fire during the 120 ms fade; a leaving menu has no handlers at all.
- Settings sidebar: the selected row's icon tile is a white 25 % tint (an accent tile on an accent row vanished).
- Title bar: `WorkspaceView::set_top_inset(s(TITLEBAR_H))` keeps the pane layout height right; `csd::inset` and the tab strip's window-button indent were removed (panes no longer touch the top edge). Reviewer checked `top_inset` is only a height, not a pointer offset.
- Added kit helpers not in the plan: `alpha`, `scale_alpha` (SwiftUI `opacity`), `metric::{OMNIBAR_PAD_X, LOCK_GLYPH, BADGE_RADIUS, SWITCH_W, SWITCH_H}`, `controls::{sidebar_row, toolbar_button}`.
- `// mac-ok` also marks two non-colour values that the check script would otherwise flag: the Settings write debounce and the notification panel's reopen click guard (neither is motion).

### Checked
- `cargo test -p cmux-app`: 129 (133 with `--features browser`); clippy `-D warnings` and fmt clean both ways; `scripts/check-mac-tokens.sh` ok (now in CI).
- KDE, dev build, isolated `XDG_*`, `CMUX_SOCKET_MODE=allowAll`, `debug.shortcut.simulate`: notification popover opens under the bell with the arrow on it, body slid inside the window, Inter renders. The two fixes above came from this screenshot; they were not re-shot because the owner was using the desktop (a second `spectacle -a` captured another app; deleted at once).

### Not checked
- Tahoe calibration (plan Task 11 Step 2): needs the owner's cmux screenshots and a free desktop.
- Menu, palette, Settings, title bar, browser bar on screen; any click, hover or drag; Settings config-write click-through.

### Parked minors (from task reviews)
- Reopening a panel/menu during its leave restarts the fade from 0; the palette's leaving shell doesn't slide.
- Fade plumbing in `WindowView::render` is three prune blocks; extract a helper before a fourth overlay.
- `PaneChrome.tl/tr` are now always 0; `s(TITLEBAR_H)` computed in two places.
- Text field: 3 px focus border is unscaled and the 22 px box may clip descenders while editing.
- Switch/slider knobs use `menu_hover_text` (white); an `on_accent` token would read better.
- Some content text sizes in `settings/` and spacing in `browser.rs` are still literals (not covered by the check).
