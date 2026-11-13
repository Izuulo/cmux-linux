# M14 mac-look — Spec

**Goal:** the app looks and feels like cmux on current macOS (26 Tahoe, Liquid Glass), and
new UI gets that look by default.

## Scope
- **`mac` kit** (`crates/cmux-app/src/mac/`): one place for the look, used by every
  surface below and by any future UI.
  - Tokens: light and dark colours, glass fills and edges, corner radii, shadows, text sizes
    and weights, spacing, animation timings and curves. Light or dark follows the terminal
    background, as today.
  - Components: glass panel; popover with a soft rounded arrow pointing at its anchor;
    menu (items, separators, shortcuts, hover highlight); toolbar icon button; switch;
    pop-up button; segmented control; text field; grouped form section with rows; sidebar
    list. Popovers and menus animate in and out (fade plus scale from the arrow or anchor).
  - Existing ad-hoc styling (`settings/controls.rs`, constants in `menu.rs`, `panel.rs`,
    `palette.rs`) moves into the kit rather than living next to it.
- **Popups rebuilt on the kit:** notification popover (arrow at the bell), sidebar context
  menu, command palette.
- **Window title bar** (cmux `ContentView.customTitlebar`): a 28 px strip right of the
  sidebar with the focused pane's folder icon and the selected workspace's title in 13 pt
  bold (`title · host` for SSH workspaces), a hairline below, drag to move, double-click to
  maximise. The window buttons stay top-right.
- **Settings window** restyled like Tahoe System Settings on the kit: it already has a section
  sidebar and grouped cards (M12); rows, icon tiles, controls and corners take the Tahoe look.
- **Browser chrome** matched to cmux's browser panel on Tahoe: back/forward/reload buttons
  and a rounded omnibar on the kit.
- **New-browser-tab button** in the pane tab bar next to new-terminal and the split
  buttons (only in builds with the `browser` feature).
- UI text uses Inter (bundled, OFL), the closest licensable match to Apple's SF Pro.

## Out of scope
- Real background blur behind popovers and menus (gpui can't blur its own content);
  glass is faked with a translucent fill, a light top edge and a soft shadow. Owner
  accepted this on 2026-11-12.
- Popovers that extend outside the window (gpui 0.2.2 has no Wayland popup surfaces).
- Moving the window buttons to the left, the macOS menu bar, the macOS build's native
  chrome.
- Dragging the title bar folder icon out to a file manager (cmux allows it; later, if
  wanted).

## Exit criteria
1. Notification popover, context menu and palette open and close with the animation; the
   notification popover's arrow points at the bell. Shown by screenshots on KDE next to
   the owner's cmux screenshots of the same surfaces.
2. The title bar shows the folder icon and the selected workspace's title, follows
   workspace switches and renames, moves the window by drag.
3. Settings uses the kit's sidebar rows, form sections and controls; every existing
   setting is still there and still applies live.
4. Browser chrome and the new-browser-tab button match cmux's layout; the button opens a
   browser tab.
5. No component in (1)–(4) sets its own colour, radius, shadow or timing: `grep` finds
   those values only in `mac/`.
