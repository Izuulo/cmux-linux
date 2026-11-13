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

### Final review fixes
- Title bar: on a translucent window the root paints no fill, so the strip was see-through above the terminals. It now paints the tab strips' bar colour (`background` at `background-opacity`) when opacity < 1, and rounds its top-left corner while the left sidebar is hidden and its top-right while the right sidebar is closed. `PaneChrome.tl/tr` and the tab strip's top rounding are gone; `titlebar::titlebar_h()` replaces the two `s(TITLEBAR_H)` calls.
- Overlays: `mac::overlay` is the deferred/anchored layer with fade and slide that the menu, palette and popover now share; a leaving overlay gets the popover's handler-less cover, so the menu and palette also swallow clicks on their own area during the 120 ms leave (before, those clicks went through). `mac::Fading<T>` (unit-tested) holds one value with its fade: `open`, `close(now)`, `live`/`live_mut`/`is_open`, `leaving`, `shown`, `prune(now)`. Menu, notification panel (`Fading<()>`) and palette use it; `WindowView::render` prunes all three in one line. Reopening during a leave drops the leaving copy, so a second menu no longer draws over a fading first one. `Fade::leave()` was unused and is removed.
- The palette fades out with its real rows (no focus, no handlers) instead of the measured empty shell; `palette_h` is gone. A run custom command is cloned, not taken, so the rows don't change mid-fade.
- NOTICE: Lucide list now names every icon in `assets/icons/` (added chevron-left, globe, rotate-cw, lock).
- `cargo test -p cmux-app`: 133 (137 with `--features browser`); clippy and fmt clean both ways; `check-mac-tokens.sh` ok. Not checked on screen.

### Calibration (dark) — cmux on macOS 27, 2026-11-12
Measured from the owner's Retina screenshots (points = logical px); light mode values are unchanged guesses, marked `// light: not yet measured` in `mac/tokens.rs`.

| Value | Before | After | Source |
|---|---|---|---|
| `POPOVER_RADIUS` | 16 | 20 | notification popover |
| `ARROW_W` / `ARROW_H` / `ARROW_TIP_R` | 26 / 11 / 3 | 30 / 12 / 4 | popover arrow; bell-shaped flanks (concave flare, 45 degree tip tangents) |
| Popover fill | bg +0.06 (`glass`) | rgb(30,30,30), `Theme::popover` | popover |
| `MENU_RADIUS` | 12 | 13 | File menu |
| Menu fill | bg +0.06 (`glass`) | rgb(61,59,62), `Theme::menu` | File menu |
| Popover/menu edge | one 1 px line, white 0.14 (`glass_edge`) | outer 1 px black 0.9 (`edge_outer`) + inner 1 px white 0.10 (`edge_inner`, radius-1 overlay) | popover, menu |
| `PALETTE_RADIUS` | 16 | 9 | palette |
| Palette fill / border | bg +0.06 / white 0.14, shadow | terminal bg / white 0.18, no shadow (`palette_panel`) | palette |
| Palette field / row height | ~30 (padding) / 24 | 30 / 20 (`PALETTE_FIELD_H`, `PALETTE_ROW_H`) | palette |
| Palette selection | accent fill, white text | accent 0.14 over fill, normal text (`palette_select`) | palette |
| `MENU_ITEM_RADIUS` | 7 | 8 | palette highlight, menu/sidebar rows |
| `CARD_RADIUS` | 13 | 12 | Settings |
| Card fill (dark) | bg +0.05 at 0.76 | bg -0.03 opaque | window rgb(41,42,43), card rgb(33,33,33) |
| Card edge (dark) | `separator` | white 0.08 (`card_edge`) | Settings |
| `SWITCH_W` | 38 | 44 (H stays 22) | Settings |
| Sidebar row | 28 tall, 20 px accent tile + 12 px white glyph | 32 tall (`SIDEBAR_ROW_H`), plain 16 px glyph (`SIDEBAR_GLYPH`), no tile | Settings |

Other: `glass`/`glass_edge` are gone; the omnibar border keeps its old look as `Theme::field_border`. `glass_panel(t, radius, fill)` takes its fill. Not checked on screen (controller screenshots next). Concern: the Settings dropdown list scrolls (`overflow_y_scroll`), so its inner edge overlay scrolls with the content on long lists.

### Calibration (light) and browser bar
- Light screenshots on the owner's Mac (system appearance switched for a minute and back): cmux's main chrome stays on the terminal theme; the notification popover and Settings follow the system. Popover fill rgb(235,236,237), inner edge white, outer edge near-black (we use black 0.5: at 1x a 1 px line weighs twice a Retina hairline); Settings cards white on a white window, sidebar rgb(235,236,237). Menu light fill not captured cleanly (assumed = popover).
- Browser bar (dark): omnibar 27 pt tall including its 1 pt border, fill ≈ the terminal background, accent border while focused; back/forward/reload are bare glyphs. `OMNIBAR_H` 22 → 27.
- The Mac was left as found: dark mode back on, the browser tab and Settings closed.
