# M14 mac-look — Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** cmux-linux looks and feels like cmux on macOS 26 Tahoe, through one `mac` UI kit that every surface (and any future one) is built from.

**Architecture:** a new module `crates/cmux-app/src/mac/` holds tokens (colours derived from the terminal background, radii, shadows, text, timings) and components (glass panel, popover with arrow, menu, controls, form sections, toolbar button, enter/leave fade). Existing surfaces are rebuilt on it: sidebar context menu, notification popover, command palette, Settings, browser chrome, pane tab strip. A new title bar strip sits over the content column. Everything is drawn by gpui; nothing outside Rust.

**Tech Stack:** Rust, gpui 0.2.2 (`canvas` + `PathBuilder` for the arrow, `anchored`/`deferred` for overlays, `window.request_animation_frame()` for the fades), Inter 4.1 (OFL) for UI text.

## Global Constraints
- Licence GPL-3.0-or-later; never copy cmux code (reference only, cmux @ `0fd2e596`). Inter goes into `NOTICE` with its OFL text.
- Every new colour, radius, shadow, text size and timing lives in `crates/cmux-app/src/mac/`; components outside `mac/` that this plan touches use only kit tokens (spec exit 5).
- Light/dark follows the terminal background (`chrome::wcag_is_light`), as today.
- Scale every length with `crate::ui::px` (it applies the UI scale), never `gpui::px` for kit metrics.
- Before any cargo command: `export PATH="$PWD/.tools/zig:$PATH"` from the repo root.
- Branch `m14-mac-look` (already cut from `master`). One commit per task; PR at the end (owner merges).

## Decisions

| | Decision | Why |
|---|---|---|
| K1 | Kit = `mac/mod.rs` re-exporting `tokens` (`Theme`, `metric`, `motion`, shadows), `fade` (`Fade`), `glass` (`glass_panel`), `popover` (`popover`, arrow), `menu` (`menu`, `MenuRow`), `controls` (moved from `settings/controls.rs`). | One import path (`crate::mac`) for new UI; owner asked for "easily add new things with the same look". |
| K2 | `Theme` replaces `settings::controls::Colors` (same fields plus `glass`, `glass_edge`, `menu_hover`, `menu_hover_text`). Settings code is renamed `Colors` → `Theme`, no behaviour change. | The existing Colors already derives macOS-like values from the background; extend, don't duplicate. |
| K3 | Tahoe starting values: menu radius 12, menu item radius 7, item height 24, menu inset 5; popover radius 16; arrow 26×11 with 3 px tip radius; window corner 16; title bar 28 (cmux `WindowChromeMetrics.sharedChromeBarHeight`); title 13 bold; omnibar pill radius 10, text 12, field height 22, nav hit 26 (cmux `BrowserChromeMetrics`, `BrowserPanelView.omnibarPillCornerRadius`). Calibrated in Task 11 against the owner's cmux screenshots. | Tahoe numbers are not published; cmux source gives the chrome ones. |
| K4 | Glass without blur: background shifted one step off the terminal bg at alpha 0.94, 1 px edge (`white 0.14` dark / `black 0.10` light), two shadows (`0 10 30 black 0.30`, `0 0 1 black 0.30`). | Owner accepted no real blur (2026-11-12). |
| K5 | Motion: gpui can't scale a `div`, so enter = fade 0→1 plus a 6 px slide from the anchor side over 180 ms (ease-out cubic); leave = 1→0 over 120 ms. A closed overlay is kept as a non-interactive "leaving" copy until its fade ends, so open/closed state (`self.menu`, `self.palette`, `self.panel_open`) keeps its meaning everywhere else. | Closest match to NSPopover/NSMenu without transforms; no logic changes to callers. |
| K6 | Title bar: 28 px strip at the top of the content column (right of the sidebar, left of the right sidebar), `folder.svg` + `title_of(selected)` (`· host` for SSH), hairline under it, `csd::window_drag(…, zoom = true)`. Its right end reserves `csd::controls_width`. The workspace view moves down by 28 px. | cmux `ContentView.customTitlebar`. |
| K7 | New-browser-tab strip button: order terminal, browser, split right, split down (cmux `CmuxSurfaceTabBarBuiltInAction` order); compiled only with `feature = "browser"`. | cmux order; non-browser builds have no browser. |

## File map

| File | Change |
|---|---|
| `crates/cmux-app/src/mac/mod.rs` | create: module root, re-exports |
| `crates/cmux-app/src/mac/tokens.rs` | create: `Theme`, `metric`, `motion`, `popover_shadow()` |
| `crates/cmux-app/src/mac/fade.rs` | create: `Fade` |
| `crates/cmux-app/src/mac/glass.rs` | create: `glass_panel()` |
| `crates/cmux-app/src/mac/popover.rs` | create: `ArrowPts`, `arrow_points()`, `popover()` |
| `crates/cmux-app/src/mac/menu.rs` | create: `MenuRow`, `menu()` |
| `crates/cmux-app/src/mac/controls.rs` | moved from `settings/controls.rs`, restyled |
| `crates/cmux-app/src/lib.rs` | `pub mod mac;` |
| `crates/cmux-app/src/settings.rs`, `settings/*.rs` | `Colors` → `Theme`, imports from `crate::mac` |
| `crates/cmux-app/src/style.rs` | load Inter; `UI_FONT_FAMILY` |
| `crates/cmux-app/assets/fonts/Inter-*.ttf`, `Inter-OFL.txt` | add |
| `crates/cmux-app/src/window_view/menu.rs` | rebuilt on `mac::menu` + `Fade` |
| `crates/cmux-app/src/window_view/panel.rs` | rebuilt on `mac::popover` + `Fade` |
| `crates/cmux-app/src/window_view/palette.rs` | rebuilt on `mac::glass_panel` + `Fade` |
| `crates/cmux-app/src/window_view/titlebar.rs` | create: title bar strip |
| `crates/cmux-app/src/window_view.rs` | fields for leaving overlays, title bar in layout, root font |
| `crates/cmux-app/src/csd.rs` | `RADIUS` → `mac::metric::WINDOW_RADIUS` |
| `crates/cmux-app/src/workspace/tab_strip.rs` | browser button |
| `crates/cmux-app/src/browser.rs` | chrome bar on kit tokens, lock glyph |
| `crates/cmux-app/assets/icons/lock.svg` | add (Lucide, ISC, same licence file) |
| `scripts/check-mac-tokens.sh` | create: exit 5 grep |
| `NOTICE`, `docs/milestones/m14-mac-look/journal.md`, `STATUS.md` | update |

---

### Task 1: Kit skeleton and tokens

**Files:**
- Create: `crates/cmux-app/src/mac/mod.rs`, `crates/cmux-app/src/mac/tokens.rs`
- Modify: `crates/cmux-app/src/lib.rs`, `crates/cmux-app/src/settings/controls.rs` (drop `Colors`), every `settings` file that names `Colors`

**Interfaces:**
- Produces: `crate::mac::Theme` (`Copy`; `Theme::new(bg: Rgba) -> Theme`; fields `bg, sidebar, card, separator, text, secondary, accent, hover, control, warning, error, glass, glass_edge, menu_hover, menu_hover_text: Rgba`); `crate::mac::metric::*` (`f32` consts); `crate::mac::motion::{ENTER, LEAVE: Duration, SLIDE: f32, ease_out(t: f32) -> f32}`; `crate::mac::popover_shadow() -> Vec<BoxShadow>`.

- [ ] **Step 1: Write the failing test** in `crates/cmux-app/src/mac/tokens.rs` (create the file with only the test module and `use super::*;` first):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(r: f32, g: f32, b: f32) -> Rgba {
        Rgba { r, g, b, a: 1.0 }
    }

    #[test]
    fn glass_is_nearly_opaque_and_edge_follows_appearance() {
        let dark = Theme::new(rgba(0.1, 0.1, 0.1));
        let light = Theme::new(rgba(0.95, 0.95, 0.95));
        assert!((dark.glass.a - 0.94).abs() < 1e-6);
        assert_eq!((dark.glass_edge.r, dark.glass_edge.a), (1.0, 0.14));
        assert_eq!((light.glass_edge.r, light.glass_edge.a), (0.0, 0.10));
        assert_eq!(dark.menu_hover, dark.accent);
    }

    #[test]
    fn ease_out_starts_fast_and_ends_at_one() {
        assert_eq!(motion::ease_out(0.0), 0.0);
        assert_eq!(motion::ease_out(1.0), 1.0);
        assert!(motion::ease_out(0.5) > 0.8);
        assert_eq!(motion::ease_out(2.0), 1.0);
    }
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p cmux-app mac::tokens` — Expected: compile error, `Theme` not found (and `mac` not a module until Step 3 adds it).

- [ ] **Step 3: Implement.** `crates/cmux-app/src/mac/mod.rs`:

```rust
//! The macOS (Tahoe) look: tokens and components every surface is built from (M14 plan K1).
//! New UI uses these instead of its own colours, radii, shadows or timings.

pub mod tokens;

pub use tokens::{Theme, metric, motion, popover_shadow};
```

`crates/cmux-app/src/mac/tokens.rs` (above the tests): move the body of `Colors::new` from `settings/controls.rs` here unchanged, renamed `Theme`, then add the new fields:

```rust
use crate::chrome;
use crate::ui::px;
use gpui::{BoxShadow, Rgba, point};

/// Colours, derived from the terminal background like the rest of the chrome.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Rgba,
    pub sidebar: Rgba,
    pub card: Rgba,
    pub separator: Rgba,
    pub text: Rgba,
    pub secondary: Rgba,
    pub accent: Rgba,
    pub hover: Rgba,
    pub control: Rgba,
    pub warning: Rgba,
    pub error: Rgba,
    /// Popover, menu and palette fill (plan K4: no blur, so nearly opaque).
    pub glass: Rgba,
    pub glass_edge: Rgba,
    /// Menu row under the pointer: accent fill, white text (NSMenu).
    pub menu_hover: Rgba,
    pub menu_hover_text: Rgba,
}

impl Theme {
    pub fn new(bg: Rgba) -> Self {
        let p = chrome::sidebar_palette(bg);
        let light = chrome::wcag_is_light(bg);
        let ink = |a: f32| {
            let v = if light { 0.0 } else { 1.0 };
            Rgba { r: v, g: v, b: v, a }
        };
        // macOS controlBackgroundColor at 0.76 (sui `SettingsCard`): a step off the backdrop.
        let card = chrome::shift(bg, if light { 0.06 } else { 0.05 }, 0.76);
        Theme {
            bg,
            sidebar: chrome::shift(bg, if light { -0.03 } else { 0.025 }, 1.0),
            card,
            separator: Rgba { a: p.border.a * 0.5, ..p.border },
            text: p.title,
            secondary: p.detail,
            accent: p.accent,
            hover: ink(0.07),
            control: ink(0.12),
            warning: Rgba { r: 1.0, g: 0.62, b: 0.04, a: 1.0 },
            error: Rgba { r: 1.0, g: 0.27, b: 0.23, a: 1.0 },
            glass: chrome::shift(bg, if light { 0.04 } else { 0.06 }, 0.94),
            glass_edge: if light { ink(0.10) } else { ink(0.14) },
            menu_hover: p.accent,
            menu_hover_text: Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
        }
    }
}

/// Tahoe metrics in logical pixels before UI scale (plan K3). Calibrated in Task 11.
pub mod metric {
    pub const WINDOW_RADIUS: f32 = 16.0;
    pub const TITLEBAR_H: f32 = 28.0;
    pub const TITLE_TEXT: f32 = 13.0;
    pub const BODY_TEXT: f32 = 13.0;
    pub const SMALL_TEXT: f32 = 11.0;
    pub const MENU_RADIUS: f32 = 12.0;
    pub const MENU_INSET: f32 = 5.0;
    pub const MENU_ITEM_H: f32 = 24.0;
    pub const MENU_ITEM_RADIUS: f32 = 7.0;
    pub const MENU_ITEM_PAD_X: f32 = 10.0;
    pub const MENU_W: f32 = 220.0;
    pub const POPOVER_RADIUS: f32 = 16.0;
    pub const ARROW_W: f32 = 26.0;
    pub const ARROW_H: f32 = 11.0;
    pub const ARROW_TIP_R: f32 = 3.0;
    pub const PALETTE_RADIUS: f32 = 16.0;
    pub const CARD_RADIUS: f32 = 13.0;
    pub const CONTROL_RADIUS: f32 = 7.0;
    pub const TOOLBAR_BUTTON: f32 = 26.0;
    pub const TOOLBAR_GLYPH: f32 = 14.0;
    pub const OMNIBAR_H: f32 = 22.0;
    pub const OMNIBAR_RADIUS: f32 = 10.0;
    pub const OMNIBAR_TEXT: f32 = 12.0;
    /// Gap kept between an overlay and the window edge.
    pub const OVERLAY_MARGIN: f32 = 8.0;
}

/// Enter/leave timing for popovers, menus and the palette (plan K5).
pub mod motion {
    use std::time::Duration;

    pub const ENTER: Duration = Duration::from_millis(180);
    pub const LEAVE: Duration = Duration::from_millis(120);
    /// How far an entering overlay slides in from its anchor side.
    pub const SLIDE: f32 = 6.0;

    /// Cubic ease-out, clamped to 0..=1.
    pub fn ease_out(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }
}

/// Soft two-layer shadow under glass (plan K4).
pub fn popover_shadow() -> Vec<BoxShadow> {
    let black = |a: f32| Rgba { r: 0.0, g: 0.0, b: 0.0, a };
    vec![
        BoxShadow {
            color: black(0.30).into(),
            offset: point(px(0.), px(10.)),
            blur_radius: px(30.),
            spread_radius: px(0.),
        },
        BoxShadow {
            color: black(0.30).into(),
            offset: point(px(0.), px(0.)),
            blur_radius: px(1.),
            spread_radius: px(0.),
        },
    ]
}
```

In `crates/cmux-app/src/lib.rs` add `pub mod mac;` next to the other `mod` lines. In `settings/controls.rs` delete the `Colors` struct and `impl Colors`, add `pub use crate::mac::Theme;`, and rename every `Colors` in `settings.rs` and `settings/*.rs`:

```bash
sed -i 's/\bColors\b/Theme/g' crates/cmux-app/src/settings.rs crates/cmux-app/src/settings/*.rs
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p cmux-app` — Expected: all pass, including the two new ones and the existing settings tests.

- [ ] **Step 5: Commit**

```bash
git add crates/cmux-app/src/mac crates/cmux-app/src/lib.rs crates/cmux-app/src/settings.rs crates/cmux-app/src/settings
git commit -m "mac kit: tokens (Theme, metrics, motion, shadow)"
```

### Task 2: Inter for UI text

**Files:**
- Create: `crates/cmux-app/assets/fonts/Inter-Regular.ttf`, `Inter-Medium.ttf`, `Inter-SemiBold.ttf`, `Inter-Bold.ttf`, `Inter-OFL.txt`
- Modify: `crates/cmux-app/src/style.rs:144-154`, `crates/cmux-app/assets/fonts/SHA256SUMS`, `NOTICE`, `crates/cmux-app/src/window_view.rs` (root div), `crates/cmux-app/src/settings.rs` (root div)

**Interfaces:**
- Produces: `crate::style::UI_FONT_FAMILY: &str = "Inter"`.

- [ ] **Step 1: Fetch Inter 4.1** into a fresh scratch directory and copy the four static TTFs and the licence:

```bash
d=$(mktemp -d) && curl -sL -o "$d/inter.zip" https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip \
  && unzip -q "$d/inter.zip" -d "$d/x" \
  && for w in Regular Medium SemiBold Bold; do cp "$d/x/extras/ttf/Inter-$w.ttf" crates/cmux-app/assets/fonts/; done \
  && cp "$d/x/LICENSE.txt" crates/cmux-app/assets/fonts/Inter-OFL.txt
(cd crates/cmux-app/assets/fonts && sha256sum Inter-*.ttf >> SHA256SUMS)
```

Expected: four `Inter-*.ttf` files, 300–420 KB each.

- [ ] **Step 2: Load them.** In `style.rs` add after `DEFAULT_FONT_FAMILY`:

```rust
/// UI text: Inter, the closest licensable face to Apple's SF Pro (M14 spec).
pub const UI_FONT_FAMILY: &str = "Inter";
```

and append to the `fonts` vec in `load_bundled_fonts`:

```rust
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Medium.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-SemiBold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Bold.ttf").as_slice()),
```

- [ ] **Step 3: Use it.** On the outermost `div()` returned by `WindowView::render` (`window_view.rs`, the one with `.on_drag_move(cx.listener(Self::on_sidebar_drag))`) and by `SettingsView::render` (`settings.rs:709`), add `.font_family(crate::style::UI_FONT_FAMILY)`. Terminal cells set their own font, so they are unaffected.

- [ ] **Step 4: NOTICE.** Below the JetBrains Mono entry:

```
Inter 4.1 (crates/cmux-app/assets/fonts/Inter-*.ttf, https://github.com/rsms/inter)
  SIL Open Font License 1.1, see crates/cmux-app/assets/fonts/Inter-OFL.txt. UI text.
```

- [ ] **Step 5: Verify and commit**

Run: `cargo test -p cmux-app && cargo run` — Expected: tests pass; sidebar, tab titles and Settings text render in Inter (compare the lowercase "g" and "t" with before), terminals unchanged.

```bash
git add crates/cmux-app/assets/fonts crates/cmux-app/src/style.rs crates/cmux-app/src/window_view.rs crates/cmux-app/src/settings.rs NOTICE
git commit -m "UI text in Inter"
```

### Task 3: `Fade` (enter/leave timing)

**Files:**
- Create: `crates/cmux-app/src/mac/fade.rs`
- Modify: `crates/cmux-app/src/mac/mod.rs`

**Interfaces:**
- Consumes: `motion::{ENTER, LEAVE, ease_out}`
- Produces: `crate::mac::Fade` (`Clone, Copy, Debug`): `Fade::enter() -> Fade`, `Fade::leave() -> Fade`, `fn amount_at(&self, now: Instant) -> f32` (0 = invisible, 1 = fully shown, eased), `fn done_at(&self, now: Instant) -> bool`, `fn leaving(&self) -> bool`, `fn slide(&self, now: Instant) -> f32` (`(1 - amount) * SLIDE`, in logical px).

- [ ] **Step 1: Failing tests** in `fade.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn enter_rises_to_one_and_finishes() {
        let t0 = Instant::now();
        let f = Fade { start: t0, leaving: false };
        assert_eq!(f.amount_at(t0), 0.0);
        assert!(f.amount_at(t0 + motion::ENTER / 2) > 0.5);
        assert_eq!(f.amount_at(t0 + motion::ENTER), 1.0);
        assert!(!f.done_at(t0 + motion::ENTER / 2));
        assert!(f.done_at(t0 + motion::ENTER));
    }

    #[test]
    fn leave_falls_to_zero_and_slides_back() {
        let t0 = Instant::now();
        let f = Fade { start: t0, leaving: true };
        assert_eq!(f.amount_at(t0), 1.0);
        assert_eq!(f.slide(t0), 0.0);
        assert_eq!(f.amount_at(t0 + motion::LEAVE), 0.0);
        assert_eq!(f.slide(t0 + motion::LEAVE), motion::SLIDE);
        assert!(f.done_at(t0 + motion::LEAVE + Duration::from_millis(1)));
    }
}
```

- [ ] **Step 2: Run** `cargo test -p cmux-app mac::fade` — Expected: FAIL, `Fade` not found.

- [ ] **Step 3: Implement** (above the tests):

```rust
//! Enter/leave timing for overlays (plan K5). The caller keeps a `Fade` next to the overlay,
//! draws it with `amount_at` as opacity and `slide` as offset, and asks for another frame
//! while `!done_at(now)`.

use std::time::Instant;

use super::motion;

#[derive(Debug, Clone, Copy)]
pub struct Fade {
    start: Instant,
    leaving: bool,
}

impl Fade {
    pub fn enter() -> Self {
        Fade { start: Instant::now(), leaving: false }
    }

    pub fn leave() -> Self {
        Fade { start: Instant::now(), leaving: true }
    }

    pub fn leaving(&self) -> bool {
        self.leaving
    }

    fn progress(&self, now: Instant) -> f32 {
        let total = if self.leaving { motion::LEAVE } else { motion::ENTER };
        now.saturating_duration_since(self.start).as_secs_f32() / total.as_secs_f32()
    }

    pub fn amount_at(&self, now: Instant) -> f32 {
        let e = motion::ease_out(self.progress(now));
        if self.leaving { 1.0 - e } else { e }
    }

    pub fn done_at(&self, now: Instant) -> bool {
        self.progress(now) >= 1.0
    }

    pub fn slide(&self, now: Instant) -> f32 {
        (1.0 - self.amount_at(now)) * motion::SLIDE
    }
}
```

The tests build `Fade { start, leaving }` directly; that works because they are in the same module.

Add `pub mod fade;` and `pub use fade::Fade;` to `mac/mod.rs`.

- [ ] **Step 4: Run** `cargo test -p cmux-app mac::fade` — Expected: 2 passed.

- [ ] **Step 5: Commit** `git add crates/cmux-app/src/mac && git commit -m "mac kit: Fade"`

### Task 4: Glass panel and popover with arrow

**Files:**
- Create: `crates/cmux-app/src/mac/glass.rs`, `crates/cmux-app/src/mac/popover.rs`
- Modify: `crates/cmux-app/src/mac/mod.rs`

**Interfaces:**
- Consumes: `Theme`, `metric`, `popover_shadow`, `Fade`
- Produces:
  - `crate::mac::glass_panel(t: &Theme, radius: f32) -> gpui::Div` — rounded, filled, edged, shadowed, `overflow_hidden`; caller adds size and children.
  - `crate::mac::ArrowPts` and `crate::mac::arrow_points(tip_x: f32, tip_y: f32) -> ArrowPts` — pure geometry for an up-pointing arrow (tip on top, base `ARROW_H` below), in logical px.
  - `crate::mac::popover(id: &'static str, t: &Theme, tip: Point<Pixels>, width: f32, height: Option<f32>, fade: Fade, now: Instant, content: AnyElement) -> impl IntoElement` — a `deferred(anchored())` overlay whose arrow tip sits at `tip` (window coordinates, below an anchor) and whose body hangs under it, kept inside the window.

- [ ] **Step 1: Failing test** in `popover.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_is_symmetric_and_fits_its_box() {
        let a = arrow_points(100.0, 50.0);
        let (w, h) = (metric::ARROW_W, metric::ARROW_H);
        assert_eq!(a.base_l, (100.0 - w / 2.0, 50.0 + h));
        assert_eq!(a.base_r, (100.0 + w / 2.0, 50.0 + h));
        assert_eq!(100.0 - a.tip_l.0, a.tip_r.0 - 100.0);
        assert_eq!(100.0 - a.shoulder_l.0, a.shoulder_r.0 - 100.0);
        for p in [a.shoulder_l, a.tip_l, a.tip_r, a.shoulder_r] {
            assert!(p.1 >= 50.0 && p.1 <= 50.0 + h, "{p:?} outside the arrow's box");
        }
        assert!(a.tip_ctrl.1 < a.tip_l.1, "the tip curve bulges upward");
    }
}
```

- [ ] **Step 2: Run** `cargo test -p cmux-app mac::popover` — Expected: FAIL, `arrow_points` not found.

- [ ] **Step 3: Implement `glass.rs`:**

```rust
//! The glass surface under popovers, menus and the palette (plan K4).

use gpui::{Div, Styled, div};

use super::{Theme, popover_shadow};
use crate::ui::px;

pub fn glass_panel(t: &Theme, radius: f32) -> Div {
    div()
        .overflow_hidden()
        .rounded(px(radius))
        .bg(t.glass)
        .border_1()
        .border_color(t.glass_edge)
        .shadow(popover_shadow())
}
```

Then `popover.rs` (above the tests):

```rust
//! A glass popover with a soft arrow pointing at its anchor (NSPopover; plan K3–K5).

use std::time::Instant;

use gpui::{
    AnyElement, InteractiveElement, IntoElement, ParentElement, PathBuilder, Pixels, Point,
    Styled, anchored, canvas, deferred, div, point,
};

use super::{Fade, Theme, glass_panel, metric};
use crate::ui::{px, s};

/// Up-pointing arrow outline: flared shoulders into a rounded tip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrowPts {
    pub base_l: (f32, f32),
    pub shoulder_l: (f32, f32),
    pub tip_l: (f32, f32),
    pub tip_ctrl: (f32, f32),
    pub tip_r: (f32, f32),
    pub shoulder_r: (f32, f32),
    pub base_r: (f32, f32),
}

pub fn arrow_points(tip_x: f32, tip_y: f32) -> ArrowPts {
    let (w, h, r) = (metric::ARROW_W, metric::ARROW_H, metric::ARROW_TIP_R);
    let base_y = tip_y + h;
    ArrowPts {
        base_l: (tip_x - w / 2.0, base_y),
        shoulder_l: (tip_x - w * 0.2, tip_y + h * 0.45),
        tip_l: (tip_x - r, tip_y + r),
        tip_ctrl: (tip_x, tip_y - r * 0.2),
        tip_r: (tip_x + r, tip_y + r),
        shoulder_r: (tip_x + w * 0.2, tip_y + h * 0.45),
        base_r: (tip_x + w / 2.0, base_y),
    }
}

/// Scaled gpui path for `a`; the shoulders curve out of the base like NSPopover's.
fn arrow_path(a: ArrowPts, origin: Point<Pixels>) -> PathBuilder {
    let at = |(x, y): (f32, f32)| point(origin.x + px(x), origin.y + px(y));
    let w = metric::ARROW_W;
    let mut b = PathBuilder::fill();
    b.move_to(at(a.base_l));
    b.curve_to(at(a.shoulder_l), at((a.base_l.0 + w * 0.18, a.base_l.1)));
    b.line_to(at(a.tip_l));
    b.curve_to(at(a.tip_r), at(a.tip_ctrl));
    b.line_to(at(a.shoulder_r));
    b.curve_to(at(a.base_r), at((a.base_r.0 - w * 0.18, a.base_r.1)));
    b.close();
    b
}

#[allow(clippy::too_many_arguments)]
pub fn popover(
    id: &'static str,
    t: &Theme,
    tip: Point<Pixels>,
    width: f32,
    height: Option<f32>,
    fade: Fade,
    now: Instant,
    content: AnyElement,
) -> impl IntoElement {
    let amount = fade.amount_at(now);
    let (fill, edge) = (t.glass, t.glass_edge);
    // The arrow is drawn in its own box above the body; the body's top edge hides its base.
    let arrow = canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let a = arrow_points(metric::ARROW_W / 2.0, 0.0);
            if let Ok(p) = arrow_path(a, bounds.origin).build() {
                window.paint_path(p, edge);
            }
            // Inset 1 px so the edge colour shows as a hairline outline.
            let inner = arrow_points(metric::ARROW_W / 2.0, 1.0);
            if let Ok(p) = arrow_path(inner, bounds.origin).build() {
                window.paint_path(p, fill);
            }
        },
    )
    .w(px(metric::ARROW_W))
    .h(px(metric::ARROW_H + 1.0));
    let body = glass_panel(t, metric::POPOVER_RADIUS)
        .w(px(width))
        .when_some(height, |d, h| d.h(px(h)))
        .flex()
        .flex_col()
        .child(content);
    deferred(
        anchored()
            .position(point(tip.x - gpui::px(s(width) / 2.0), tip.y))
            .snap_to_window_with_margin(px(metric::OVERLAY_MARGIN))
            .child(
                div()
                    .id(id)
                    .occlude()
                    .when(fade.leaving(), |d| d.block_mouse_except_scroll())
                    .opacity(amount)
                    .mt(px(fade.slide(now)))
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(arrow)
                    // Overlap the arrow's base by 1 px so no seam shows.
                    .child(div().mt(px(-1.0)).child(body)),
            ),
    )
}
```

If `block_mouse_except_scroll` does not exist in gpui 0.2.2 (`grep -n "fn block_mouse" ~/.cargo/registry/src/*/gpui-0.2.2/src/elements/div.rs`), drop that line: a leaving overlay's own click handlers are removed by the caller in Tasks 5–7 instead. Add `use gpui::prelude::FluentBuilder;` if `.when`/`.when_some` aren't found.

Add to `mac/mod.rs`: `pub mod glass; pub mod popover;` and `pub use glass::glass_panel; pub use popover::{ArrowPts, arrow_points, popover};`.

- [ ] **Step 4: Run** `cargo test -p cmux-app mac::` — Expected: all mac tests pass; `cargo clippy -p cmux-app` clean.

- [ ] **Step 5: Commit** `git add crates/cmux-app/src/mac && git commit -m "mac kit: glass panel and popover with arrow"`

### Task 5: Menu component and the sidebar context menu

**Files:**
- Create: `crates/cmux-app/src/mac/menu.rs`
- Modify: `crates/cmux-app/src/mac/mod.rs`, `crates/cmux-app/src/window_view/menu.rs`, `crates/cmux-app/src/window_view.rs` (field `menu_leaving`, prune in `render`)

**Interfaces:**
- Consumes: `Theme`, `metric`, `glass_panel`, `Fade`
- Produces: `crate::mac::MenuRow<'a> { Item { label: &'a str, shortcut: Option<&'a str> }, Separator }`; `crate::mac::menu(t: &Theme, rows: Vec<AnyElement>) -> gpui::Div` (glass list, caller adds the overlay and handlers); `crate::mac::menu_item(id: impl Into<ElementId>, t: &Theme, label: &str, shortcut: Option<&str>) -> gpui::Stateful<gpui::Div>`; `crate::mac::menu_separator(t: &Theme) -> AnyElement`.

- [ ] **Step 1: Failing test.** The pure part is the row height sum used to keep a menu on screen; in `mac/menu.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_counts_items_separators_and_insets() {
        let h = menu_height(3, 1);
        assert_eq!(
            h,
            3.0 * metric::MENU_ITEM_H + SEPARATOR_H + 2.0 * metric::MENU_INSET
        );
    }
}
```

- [ ] **Step 2: Run** `cargo test -p cmux-app mac::menu` — Expected: FAIL, `menu_height` not found.

- [ ] **Step 3: Implement** `mac/menu.rs`:

```rust
//! NSMenu look: glass list, accent row under the pointer, shortcut right-aligned.

use gpui::{
    AnyElement, Div, ElementId, InteractiveElement, IntoElement, ParentElement, SharedString,
    Stateful, StatefulInteractiveElement, Styled, div,
};

use super::{Theme, glass_panel, metric};
use crate::ui::px;

/// Separator row: 1 px line with 5 px above and below.
pub const SEPARATOR_H: f32 = 11.0;

pub fn menu_height(items: usize, separators: usize) -> f32 {
    items as f32 * metric::MENU_ITEM_H + separators as f32 * SEPARATOR_H + 2.0 * metric::MENU_INSET
}

pub fn menu(t: &Theme, rows: Vec<AnyElement>) -> Div {
    glass_panel(t, metric::MENU_RADIUS)
        .w(px(metric::MENU_W))
        .py(px(metric::MENU_INSET))
        .flex()
        .flex_col()
        .children(rows)
}

pub fn menu_item(
    id: impl Into<ElementId>,
    t: &Theme,
    label: &str,
    shortcut: Option<&str>,
) -> Stateful<Div> {
    let (hover, hover_text) = (t.menu_hover, t.menu_hover_text);
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .h(px(metric::MENU_ITEM_H))
        .mx(px(metric::MENU_INSET))
        .px(px(metric::MENU_ITEM_PAD_X - metric::MENU_INSET))
        .rounded(px(metric::MENU_ITEM_RADIUS))
        .text_size(px(metric::BODY_TEXT))
        .text_color(t.text)
        .hover(move |s| s.bg(hover).text_color(hover_text))
        .child(
            div()
                .flex_1()
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(SharedString::from(label.to_string())),
        )
        .children(shortcut.map(|k| {
            div()
                .flex_none()
                .pl(px(12.))
                .opacity(0.55)
                .child(SharedString::from(k.to_string()))
        }))
}

pub fn menu_separator(t: &Theme) -> AnyElement {
    div()
        .flex_none()
        .h(px(1.))
        .my(px((SEPARATOR_H - 1.0) / 2.0))
        .mx(px(metric::MENU_ITEM_PAD_X))
        .bg(t.separator)
        .into_any_element()
}
```

Exports in `mac/mod.rs`: `pub mod menu; pub use menu::{menu, menu_item, menu_separator, menu_height};` (drop the `MenuRow` enum from the Interfaces if no caller needs it; none does today).

- [ ] **Step 4: Rebuild the sidebar menu on it.** In `window_view/menu.rs`:
  - `struct Menu` gains `fade: Fade`; `open_menu` sets `fade: Fade::enter()` and calls `window.refresh()` through `cx.notify()` as before.
  - `WindowView` gets a field `menu_leaving: Option<window_view::menu::Menu>` (init `None`).
  - `close_menu`: `if let Some(mut m) = self.menu.take() { m.fade = Fade::leave(); self.menu_leaving = Some(m); cx.notify(); true } else { false }`.
  - `render_menu(&self, window: &Window, cx)` draws `self.menu` (interactive) and `self.menu_leaving` (no click handlers, no `on_mouse_down_out`) through one helper `fn menu_overlay(&self, m: &Menu, live: bool, now: Instant, cx) -> impl IntoElement`. Rows use `mac::menu_item(("menu-item", k), &t, label, None)` and `mac::menu_separator(&t)`; the list is `mac::menu(&t, rows)`; the outer wrapper is `div().opacity(m.fade.amount_at(now)).mt(px(m.fade.slide(now)))` inside the existing `deferred(anchored().position(m.at).snap_to_window_with_margin(px(metric::OVERLAY_MARGIN)))`. `t` is `Theme::new(chrome::rgba(cfg.background, 1.0))`.
  - Delete `WIDTH`, `MARGIN`, `PAD_Y`, `ITEM_PAD_X`, `ITEM_PAD_Y`, `ITEM_PX`, `ITEM_RADIUS` and the `BoxShadow` import.
  - In `WindowView::render`, before building the tree: `let now = Instant::now();` then for each of `menu`, `menu_leaving`: if its fade is not `done_at(now)`, call `window.request_animation_frame()`; if `menu_leaving` is done, set it to `None`. Update the call site to `.children(self.render_menu(window, cx))`.

- [ ] **Step 5: Verify**

Run: `cargo test -p cmux-app && cargo clippy -p cmux-app -- -D warnings` — Expected: pass (the existing `labels` tests in `menu.rs` still pass).
Run: `cargo run`, right-click a workspace row — Expected: rounded 12 px glass menu fades and slides in; hover rows turn accent with white text; clicking outside fades it out; clicking an item runs it.

- [ ] **Step 6: Commit** `git add crates/cmux-app/src && git commit -m "mac kit: menu; sidebar context menu on it"`

### Task 6: Notification popover

**Files:**
- Modify: `crates/cmux-app/src/window_view/panel.rs`, `crates/cmux-app/src/window_view/sidebar.rs:521-527` (`bell_anchor` returns the bell's centre-bottom), `crates/cmux-app/src/window_view.rs` (field `panel_fade: Fade`)

**Interfaces:**
- Consumes: `mac::popover`, `Fade`, `Theme`
- Produces: `WindowView::panel_fade: Option<Fade>` (`Some` while opening, open or leaving; `None` when fully closed).

- [ ] **Step 1: Point the anchor at the bell's centre.** `bell_anchor` currently returns the bell's left edge at the band bottom. Change it to the centre: `x + s(BUTTON) / 2.0`, `y = px(BAND_H)`. Grep its callers (`grep -n bell_anchor -r crates/cmux-app/src`) and update any that relied on the left edge.

- [ ] **Step 2: Fade state.** Add `panel_fade: Option<Fade>` to `WindowView` (init `None`). Every place that sets `self.panel_open = true` also sets `self.panel_fade = Some(Fade::enter())`; every place that sets it `false` sets `self.panel_fade = Some(Fade::leave())` (sites: `panel.rs:55-72`, `window_view.rs:779`, `window_view.rs:796`; grep `panel_open =` to find all). In `WindowView::render`, next to the menu pruning: request a frame while the fade isn't done; set `panel_fade = None` when a leaving fade is done.

- [ ] **Step 3: Render on the kit.** In `render_panel`: return `None` when `self.panel_fade.is_none()`. Keep the header and body building as they are, but take colours from `Theme::new(...)` instead of the local constants where they are surface colours (fill, border, shadow). Replace the `deferred(anchored()…div()…rounded(px(8.))…shadow(…))` block with:

```rust
let fade = self.panel_fade?;
let content = div()
    .flex()
    .flex_col()
    .size_full()
    .when(!fade.leaving(), |d| {
        d.on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_panel(cx)))
    })
    .child(header)
    .child(body)
    .into_any_element();
Some(mac::popover("notif-panel", &t, at, w, Some(h), fade, Instant::now(), content))
```

Delete `MARGIN`, the `BoxShadow` import and the local `alpha` helper if now unused.

- [ ] **Step 4: Verify**

Run: `cargo test -p cmux-app` — Expected: pass.
Run: `cargo run`; send `printf '\e]9;hello\a'` in a terminal; click the bell — Expected: popover fades in under the bell with the arrow tip centred on it; Super+I and clicking outside both fade it out; rows still clear/jump as before.

- [ ] **Step 5: Commit** `git add crates/cmux-app/src && git commit -m "Notification popover on the mac kit, arrow at the bell"`

### Task 7: Command palette

**Files:**
- Modify: `crates/cmux-app/src/window_view/palette.rs`, `crates/cmux-app/src/window_view.rs` (field `palette_leaving: Option<Fade>`)

**Interfaces:**
- Consumes: `glass_panel`, `Fade`, `Theme`, `metric::PALETTE_RADIUS`
- Produces: `Palette::fade: Fade` (enter, set where `Palette` is built at `palette.rs:118`).

- [ ] **Step 1:** Add `fade: Fade` to `Palette`, set `Fade::enter()` at construction. The palette's rows depend on its query, so a leaving copy would need the whole struct: on close (`palette.rs:132`, `self.palette.take()`), store only `self.palette_leaving = Some(Fade::leave())` and render a fading empty glass shell of the last height for 120 ms. Record the last rendered height in a `Cell<f32>` on `WindowView` (`palette_h`) inside `render_palette`.

- [ ] **Step 2:** In `render_palette` (`palette.rs:388-410`), replace the panel `div()…rounded(px(RADIUS))…shadow(…)` with `mac::glass_panel(&t, metric::PALETTE_RADIUS)` and wrap it in `div().opacity(p.fade.amount_at(now)).mt(px(p.fade.slide(now)))`. Row highlight uses `t.menu_hover`/`t.menu_hover_text`; row radius `metric::MENU_ITEM_RADIUS`; text `metric::BODY_TEXT`. Delete the local `RADIUS` and shadow constants.

- [ ] **Step 3:** Render the leaving shell: `mac::glass_panel(&t, metric::PALETTE_RADIUS).w(...same width...).h(px(self.palette_h.get())).opacity(fade.amount_at(now))` at the same `anchored()` position, no handlers. Prune and request frames in `WindowView::render` as in Task 5.

- [ ] **Step 4: Verify** `cargo test -p cmux-app` (palette tests unchanged and passing); `cargo run`, Super+Shift+P — Expected: glass palette fades in, Escape fades it out, typing/arrows/Enter unchanged.

- [ ] **Step 5: Commit** `git add crates/cmux-app/src && git commit -m "Command palette on the mac kit"`

### Task 8: Controls and Settings on the kit

**Files:**
- Move: `crates/cmux-app/src/settings/controls.rs` → `crates/cmux-app/src/mac/controls.rs` (`git mv`)
- Modify: `crates/cmux-app/src/mac/mod.rs`, `crates/cmux-app/src/settings.rs`, `crates/cmux-app/src/settings/*.rs`

**Interfaces:**
- Produces: `crate::mac::controls::{CONTROL_W, Handler, Pick, header, card, note, row, toggle, stepper, segmented, fraction, slider, dropdown_button, dropdown_list, text_field}` — same signatures as today, with `Theme` in place of `Colors`; new `crate::mac::controls::sidebar_row(id: impl Into<ElementId>, t: &Theme, icon: &'static str, label: &str, selected: bool) -> Stateful<Div>`.

- [ ] **Step 1: Move.**

```bash
git mv crates/cmux-app/src/settings/controls.rs crates/cmux-app/src/mac/controls.rs
sed -i 's/settings::controls/mac::controls/g; s/super::controls/crate::mac::controls/g; s/crate::settings::controls/crate::mac::controls/g' crates/cmux-app/src/settings.rs crates/cmux-app/src/settings/*.rs
```

Remove `mod controls;` from `settings.rs`, add `pub mod controls;` to `mac/mod.rs`, drop the `pub use crate::mac::Theme;` added in Task 1 and import `Theme` from `super` instead. Run `cargo test -p cmux-app` — Expected: pass (including `fraction_clamps_to_the_track`).

- [ ] **Step 2: Restyle on tokens** (cmux `SettingsCard`/Tahoe System Settings): in `mac/controls.rs` replace `CARD_RADIUS` with `metric::CARD_RADIUS`; `button_base` radius → `metric::CONTROL_RADIUS`; toggle → Tahoe switch: track 38×22 (`metric` gains `SWITCH_W: f32 = 38.0`, `SWITCH_H: f32 = 22.0`), knob `SWITCH_H - 4`, on = `t.accent`, off = `t.control`; dropdown button and segmented use `t.control` fill with `metric::CONTROL_RADIUS`; text field radius `metric::CONTROL_RADIUS`, focus ring 3 px `Rgba { a: 0.5, ..t.accent }` as `border`. Text sizes → `metric::BODY_TEXT` / `metric::SMALL_TEXT`.

- [ ] **Step 3: Settings window.** Add `sidebar_row` to `mac/controls.rs` (the row look below, so any future sidebar list reuses it) and use it from `settings.rs` `render_sidebar` (`:618`), keeping the existing click and key handling: sidebar fill `t.sidebar`; selected row = `t.menu_hover` with `t.menu_hover_text`, radius `metric::MENU_ITEM_RADIUS`, 28 px rows, 13 px text, the section icon in a 20 px rounded square (`metric::CONTROL_RADIUS`) filled `t.accent` with a white glyph (System Settings' coloured icon tiles). Window corner radius from `metric::WINDOW_RADIUS` wherever settings rounds its window.

- [ ] **Step 4: Verify** `cargo test -p cmux-app && cargo clippy -p cmux-app -- -D warnings`; `cargo run`, open Settings (Super+,) — Expected: every section still renders and every control still writes its config key (toggle one per section, `git diff` your `~/.config/ghostty/config` and `config.toml` copies, then undo).

- [ ] **Step 5: Commit** `git add -A crates/cmux-app/src && git commit -m "Settings controls move into the mac kit, Tahoe styling"`

### Task 9: Title bar

**Files:**
- Create: `crates/cmux-app/src/window_view/titlebar.rs`
- Modify: `crates/cmux-app/src/window_view.rs` (mod line, layout at the `.child(div().flex_1()…children(selected))` call), `crates/cmux-app/src/csd.rs` (`RADIUS` → `crate::mac::metric::WINDOW_RADIUS`)

**Interfaces:**
- Consumes: `WindowView::title_of(id) -> String` (`window_view.rs:~500`), `WorkspaceView::remote()` (`sidebar.rs:729` usage), `cmux_meta::remote::host`, `csd::window_drag`, `csd::controls_width`, `Theme`, `metric::{TITLEBAR_H, TITLE_TEXT}`
- Produces: `pub(super) fn titlebar_text(title: &str, host: Option<&str>) -> String` and `WindowView::render_titlebar(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement`.

- [ ] **Step 1: Failing test** in `titlebar.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_workspaces_show_their_host() {
        assert_eq!(titlebar_text("api", None), "api");
        assert_eq!(titlebar_text("api", Some("box")), "api · box");
        assert_eq!(titlebar_text("", Some("box")), "box");
    }
}
```

- [ ] **Step 2: Run** `cargo test -p cmux-app titlebar` — Expected: FAIL, `titlebar_text` not found (add `mod titlebar;` to `window_view.rs` first).

- [ ] **Step 3: Implement:**

```rust
//! The strip over the content column (cmux `ContentView.customTitlebar`, plan K6): folder icon
//! and the selected workspace's title, drag to move, double-click to maximise.

use gpui::{Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, div, svg};

use super::WindowView;
use crate::chrome;
use crate::mac::{Theme, metric};
use crate::ui::{px, s};

/// `title · host` for SSH workspaces (cmux `resolvedWorkspaceWindowTitle`).
pub(super) fn titlebar_text(title: &str, host: Option<&str>) -> String {
    match host {
        Some(h) if title.is_empty() => h.to_string(),
        Some(h) => format!("{title} · {h}"),
        None => title.to_string(),
    }
}

impl WindowView {
    pub(super) fn render_titlebar(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let t = Theme::new(chrome::rgba(self.style.cfg.background, 1.0));
        let id = self.workspaces.selected();
        let host = self.views.get(&id).and_then(|v| {
            Some(cmux_meta::remote::host(v.read(cx).remote()?.last()?).to_string())
        });
        let text = titlebar_text(&self.title_of(id), host.as_deref());
        let strip = div()
            .id("titlebar")
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap(px(6.))
            .h(px(metric::TITLEBAR_H))
            .pl(px(12.))
            .pr(gpui::px(crate::csd::controls_width(window) + s(8.)))
            .border_b_1()
            .border_color(t.separator)
            .child(svg().path("icons/folder.svg").size(px(14.)).text_color(t.secondary))
            .child(
                div()
                    .text_size(px(metric::TITLE_TEXT))
                    .font_weight(FontWeight::BOLD)
                    .text_color(t.text)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(SharedString::from(text)),
            );
        crate::csd::window_drag(strip, true)
    }
}
```

Adjust `remote()`/`title_of` names to what `grep -n "fn remote\|fn title_of" -r crates/cmux-app/src` shows if they differ.

- [ ] **Step 4: Put it in the layout.** In `WindowView::render`, replace
`.child(div().flex_1().min_w(px(0.)).h_full().children(selected))` with

```rust
.child(
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w(px(0.))
        .h_full()
        .child(self.render_titlebar(window, cx))
        .child(div().flex_1().min_h(px(0.)).children(selected)),
)
```

The top-right window buttons (`csd::controls`) already float over the right end; the strip reserves their width. The workspace's tab strip used to indent for those buttons (`chrome.trail` in `tab_strip.rs`); grep `controls_width` and remove that indent for panes now below the title bar, so tabs use the full width.

- [ ] **Step 5: Window radius.** In `csd.rs` replace `const RADIUS: f32 = 10.0;` uses with `crate::mac::metric::WINDOW_RADIUS`.

- [ ] **Step 6: Verify**

Run: `cargo test -p cmux-app` — Expected: pass.
Run: `cargo run` — Expected: 28 px bar with folder icon and bold title right of the sidebar; switching workspaces (Super+1/2) and renaming update it; dragging it moves the window; double-click maximises; window buttons don't overlap the title.

- [ ] **Step 7: Commit** `git add crates/cmux-app/src && git commit -m "Title bar with folder icon and workspace title"`

### Task 10: Browser button and browser chrome

**Files:**
- Modify: `crates/cmux-app/src/workspace/tab_strip.rs:73-77,123-127,425-435`, `crates/cmux-app/src/browser.rs:328-441`
- Create: `crates/cmux-app/assets/icons/lock.svg`

**Interfaces:**
- Consumes: `WorkspaceView::open_browser(pane: Option<PaneId>, url: &str, window, cx) -> Option<SurfaceId>` (`workspace.rs:665`), `Theme`, `metric::{TOOLBAR_BUTTON, TOOLBAR_GLYPH, OMNIBAR_H, OMNIBAR_RADIUS, OMNIBAR_TEXT}`
- Produces: `crate::mac::controls::toolbar_button(id: impl Into<ElementId>, t: &Theme, icon: &'static str, enabled: bool) -> Stateful<Div>` (square `TOOLBAR_BUTTON`, glyph `TOOLBAR_GLYPH`, radius `CONTROL_RADIUS`, hover `t.hover`, 0.4 opacity when disabled; caller adds `on_click`).

- [ ] **Step 1: Strip button.** Add the variant and button, browser builds only:

```rust
enum StripButton {
    NewTerminal,
    #[cfg(feature = "browser")]
    NewBrowser,
    SplitRight,
    SplitDown,
}
```

Turn the `buttons` array into a `Vec` so the browser entry can be conditional:

```rust
let mut buttons = vec![(StripButton::NewTerminal, "icons/square-terminal.svg")];
#[cfg(feature = "browser")]
buttons.push((StripButton::NewBrowser, "icons/globe.svg"));
buttons.push((StripButton::SplitRight, "icons/columns-2.svg"));
buttons.push((StripButton::SplitDown, "icons/rows-2.svg"));
let buttons = buttons.into_iter().map(|(b, icon)| { /* existing closure body */ });
```

and in `strip_button`:

```rust
#[cfg(feature = "browser")]
StripButton::NewBrowser => {
    self.open_browser(Some(pane), "about:blank", window, cx);
}
```

- [ ] **Step 2: Verify the button** — `cargo build -p cmux-app` (no browser: three buttons, compiles) and `cargo run --features cmux-app/browser` (after `scripts/install-cef.sh`): four buttons in cmux's order; the globe opens a browser tab in that pane with the URL field focused.

- [ ] **Step 3: Chrome bar on tokens** (cmux `BrowserPanelView` 1234–1410, `BrowserChromeMetrics`): add `toolbar_button` to `mac/controls.rs` and build the `nav` closure in `render_bar` from it; the field becomes the omnibar pill: height `metric::OMNIBAR_H`, radius `metric::OMNIBAR_RADIUS`, fill `t.control`, 1 px `t.glass_edge` border (accent when editing), text `metric::OMNIBAR_TEXT`, 8 px horizontal padding; when the page URL starts with `https://` and the field isn't being edited, show `icons/lock.svg` (11 px, `t.secondary`) before the text. Delete `BUTTON`, `GLYPH`, `PILL_RADIUS`, `PILL_PAD_X`, `FIELD_PX` once unused. Fetch the glyph:

```bash
curl -sL -o crates/cmux-app/assets/icons/lock.svg https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/lock.svg
```

(Lucide, ISC: covered by `assets/icons/LICENSE-lucide.txt`.)

- [ ] **Step 4: Verify** `cargo test -p cmux-app --features browser`; `cargo run --features cmux-app/browser`, open `https://example.com` — Expected: rounded pill omnibar with a lock, buttons evenly sized; back/forward/reload and typing a URL still work.

- [ ] **Step 5: Commit** `git add crates/cmux-app && git commit -m "Browser: tab-strip button, omnibar and buttons on the mac kit"`

### Task 11: Token check, calibration, docs

**Files:**
- Create: `scripts/check-mac-tokens.sh`, `docs/milestones/m14-mac-look/journal.md`
- Modify: `crates/cmux-app/src/mac/tokens.rs` (calibrated values), `STATUS.md` (archive first), CI workflow if it runs scripts (`grep -rn "scripts/" .github/workflows`)

- [ ] **Step 1: Exit-5 check script:**

```bash
#!/usr/bin/env bash
# M14 exit 5: surfaces built on the mac kit set no colour, radius, shadow or timing of their own.
set -euo pipefail
cd "$(dirname "$0")/.."
files=(
  crates/cmux-app/src/window_view/menu.rs
  crates/cmux-app/src/window_view/panel.rs
  crates/cmux-app/src/window_view/palette.rs
  crates/cmux-app/src/window_view/titlebar.rs
  crates/cmux-app/src/settings.rs
  crates/cmux-app/src/settings
  crates/cmux-app/src/browser.rs
)
# A line that draws content rather than chrome (a theme preview swatch) may carry `// mac-ok`.
if grep -rnE 'BoxShadow|Rgba \{|\.rounded\(px\([0-9]|Duration::from_millis' "${files[@]}" | grep -v 'mac-ok'; then
  echo "check-mac-tokens: the lines above set their own look; use crate::mac tokens" >&2
  exit 1
fi
echo "check-mac-tokens: ok"
```

`chmod +x scripts/check-mac-tokens.sh && scripts/check-mac-tokens.sh` — Expected: `ok`. Fix any hit by moving the value into `mac/tokens.rs`. If CI runs other `scripts/check-*.sh`, add this one the same way.

- [ ] **Step 2: Calibrate.** Ask the owner for screenshots from their Mac of cmux's notification popover, a context menu, the command palette, Settings and a browser tab (light and dark). Take the same five on KDE (`spectacle -b -n -o <file>` per surface). Adjust `metric` and the `Theme` glass/edge values until radii, arrow size, row heights and contrast match by eye; record each changed value and why in the journal.

- [ ] **Step 3: Journal and status.** Start `journal.md` with the session's notes (decisions K1–K7 taken, anything that deviated, calibration table). Append the current `STATUS.md` to `docs/status-archive.md` under `## 2026-11-12 — before M14`, then update `STATUS.md`: current milestone M14, next action, owner checks (mouse: hover rows, clicking outside, title bar drag/double-click; Settings controls click-through).

- [ ] **Step 4: Full check and commit**

Run: `cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace && scripts/check-mac-tokens.sh` — Expected: all pass.

```bash
git add -A scripts docs STATUS.md crates/cmux-app/src/mac
git commit -m "M14: token check, calibration, journal, status"
```

- [ ] **Step 5: PR.** `git push -u origin m14-mac-look` and `gh pr create --base master --title "M14 mac-look" --body …` (summary, exit criteria table with how each was checked, owner checks left).

---

## Spec coverage

| Spec item | Task |
|---|---|
| Kit tokens | 1, 2 |
| Kit components (glass, popover+arrow, menu, controls, form sections, toolbar button, fade) | 3, 4, 5, 8, 10 |
| Ad-hoc styling moved into the kit | 5, 6, 7, 8, 11 |
| Notification popover / context menu / palette | 6, 5, 7 |
| Title bar | 9 |
| Settings like System Settings | 8 |
| Browser chrome | 10 |
| New-browser-tab button | 10 |
| Inter | 2 |
| Exit 1–5 | 5–7 + 11 (screenshots), 9, 8, 10, 11 (script) |
