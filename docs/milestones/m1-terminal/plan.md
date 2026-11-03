# M1 terminal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One fully usable terminal in a gpui window, good enough to run Claude Code all day: the user's shell on a PTY, a grid drawn from libghostty-vt cells with Ghostty's colours and fonts, keyboard (kitty protocol included), IME, mouse, selection, clipboard, scrollback and resize.

**Architecture:** `cmux-config` parses Ghostty's config (pure Rust, no window). `cmux-term` owns `Term` (libghostty-vt + OSC scanner, emitting events) and `Session` (portable-pty plus a reader thread that feeds `Arc<Mutex<Term>>` and a writer thread that drains one channel for keystrokes and the terminal's own DA/DSR replies); it stays UI-free. `cmux-app` is a gpui view that turns gpui input into PTY bytes and a custom `Element` that measures the grid, resizes the PTY in `prepaint`, and paints merged background rects, batched text runs and the cursor, adapted from Zed's GPL-3 terminal element.

**Tech Stack:** Rust 2024 (toolchain pinned to 1.99.0), gpui `=0.2.2`, libghostty-vt (vendored, M0), portable-pty 0.9, futures 0.3, JetBrains Mono 2.304 (bundled), Zig 0.16.0 for the Ghostty build.

## Global Constraints

Carried from M0:
- License GPL-3.0-or-later for our crates. Every added dependency, vendored file or asset goes in `NOTICE` with its licence.
- Never copy from cmux's BUSL-1.1 directories.
- Zig must be exactly 0.16.0 (`scripts/install-zig.sh`); every cargo command needs `export PATH="$PWD/.tools/zig:$PATH"` from the repo root first.
- Use `--locked` for build/test/clippy in CI. libc stays pinned at 0.2.189 in `Cargo.lock` (xattr 0.2.3 needs `libc::ENOATTR`): never run a bare `cargo update`; adding a dependency must not move libc (check with `grep -A1 'name = "libc"' Cargo.lock`).
- gpui pinned with `=0.2.2`.
- No personal info (names, emails, home paths with usernames, machine details, the user's config contents) in tracked files.
- `STATUS.md` max 100 lines; append it to `docs/status-archive.md` before replacing it.

New for M1:
- Work happens on branch `m1-terminal`; it merges to `master` at the end (Task 13).
- Toolchain pinned by `rust-toolchain.toml` (1.99.0 with clippy and rustfmt) from Task 1 on.
- Before each commit: `cargo fmt -p <crates you touched>` and `cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings`. Vendored crates are not formatted or linted (we don't edit them).
- Every commit message ends with a blank line and `Co-Authored-By: <model name> <noreply@anthropic.com>`. The commands below use `git commit -m "<subject>" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"`; an executor running on a different model puts its own name there.
- gpui code uses only APIs confirmed in `docs/milestones/m1-terminal/research.md` or the gpui 0.2.2 source (`~/.cargo/registry/src/*/gpui-0.2.2/src/`). All code in this plan was compiled and its tests run against gpui 0.2.2 and the vendored ghostty-vt before the plan was written; only runtime behaviour on a real desktop is left to the verify steps.
- `cmux-term` and `cmux-config` never depend on gpui. `cmux-config` does not depend on ghostty-vt either (see Decisions).
- Linux-only APIs (`write_to_primary`, `read_from_primary`) stay behind `#[cfg(any(target_os = "linux", target_os = "freebsd"))]` for the later macOS port.
- Screenshots are taken with spectacle and stored outside the repo in `~/Pictures/cmux-m1/`; the journal names them by file name only.

## Decisions made in this plan

| Decision | Why |
|---|---|
| `cmux-config` parses colours itself (`#rrggbb`, `rrggbb`, `#rgb`) instead of calling `ghostty_vt::parse_color` | design.md gives cmux-config no dependencies; pulling in ghostty-vt would link libghostty-vt and need Zig just to test a text parser. All 819 themes Ghostty ships use `#rrggbb`. X11 colour names produce a warning (`shortcut:` comment). |
| 8-bit C1 OSC fixed by scanning `Terminal::vt_write_with_normalized`'s output | libghostty-vt already normalises 0x9d/0x9c to `ESC ]`/`ESC \` and returns the bytes it parsed (vendor/ghostty-vt/src/terminal.rs:2158), so the scanner sees the same stream for free. |
| `Snapshot` replaced by `Term::update_frame() -> FrameInfo` plus `Term::walk_rows` (borrowed `&[Cell]`); `Term::text_lines()` kept as the cheap test helper | no per-frame clone; `FrameInfo` has cols/rows and `Debug` (STATUS carry items). |
| Selection lives in `cmux-term::selection` (UI-free) in absolute rows | testable without a window; reused per pane in M2. Ghostty's terminal-owned selection is not exposed by the wrapper. |
| `cmux-app` gets a `lib.rs`; `main.rs` only boots | pure modules (keys, colours, style) are unit-tested, and `pub` items don't trip dead-code warnings while later tasks wire them up. |
| Text input: every key goes through `KeyEncoder`; the IME handler only receives IME commits | the encoder emits plain text too, and handles kitty flags; stopping propagation after encoding prevents gpui's Wayland fallback from inserting the key again (research.md §2.1). |
| Super+key is never sent to the PTY | KDE uses Super for desktop shortcuts and cmux maps ⌘ to Super. |
| Font size is converted from points at 96 dpi (`pt * 96/72`) | Ghostty does this on Linux (`vendor/ghostty/src/font/face.zig:36`); without it text is 25% too small. |
| Cell size snapped to whole device pixels with Ghostty's rounding | matches `font/Metrics.zig` (`@round(face_width)`, `@round(ascent - descent + line_gap)`); JetBrains Mono's `line_gap` is 0, so gpui's missing `line_gap` does not matter for the default font. |
| Wheel in the alternate screen sends arrow keys when DEC mode 1007 is set | the encoder does not do it; mode 1007 defaults to on in the engine (`vendor/ghostty/src/terminal/modes.zig:271`). |
| Shell exit quits the app | Ghostty closes the surface; M1 has one surface. |
| Config warnings go to stderr | design.md wants a one-line warning; the in-window line comes with M2's config work (`shortcut:` comment). |
| Clippy keeps `--exclude ghostty-vt --exclude ghostty-vt-sys` | dropping it was tried while writing this plan: vendored `kitty.rs:1067,1072` fail `clippy::chunks_exact_to_as_chunks` on 1.99. We don't edit vendored code. |

## Deferred (with reason)

- Ligatures: off by default (`FontFeatures::disable_ligatures()`), and `font-feature` is not parsed. `force_width` snaps glyphs, so a ligature shifts the grid (research.md §1.3), and libghostty-vt does not report ligatures — shaping is the renderer's. Revisit with a ligature-aware shaper.
- `font-family-bold/-italic/-bold-italic`, `bold-color`, `minimum-contrast`, `scrollback-limit`, `mouse-scroll-multiplier`, `copy-on-select` options: defaults are hard-wired (bold/italic from the same family; PRIMARY copy-on-select on; 3 lines per notch; 10 000 lines). M2 brings the config surface.
- An app's OSC 11 background change does not repaint the window fill (cells still follow it). `shortcut:` comment in `style.rs`.
- Bell, cwd (OSC 7) and notifications are emitted as `TermEvent`s but have no UI until M3/M4.
- macOS key behaviour (`macos_option_as_alt`, press-and-hold) and the 72 dpi point size: M6 macOS build.
- `xterm-ghostty` terminfo: `TERM=xterm-256color` for M1 (kitty keyboard is negotiated by escape sequence anyway); ship terminfo in M6.

## File structure

| Path | Responsibility |
|---|---|
| `rust-toolchain.toml` | Pins Rust 1.99.0 with clippy and rustfmt |
| `.github/workflows/ci.yml` | Adds fmt check and `--all-targets` clippy |
| `crates/cmux-config/src/lib.rs` | `pub mod ghostty;` |
| `crates/cmux-config/src/ghostty/mod.rs` | `Config` and its types, line parser, per-key `apply`, colour parsing |
| `crates/cmux-config/src/ghostty/load.rs` | `Env`, `load`: `config.ghostty`/`config` lookup, `config-file` includes, theme lookup and layering |
| `crates/cmux-term/src/lib.rs` | `Term`: events, PTY replies via a channel, resize, scroll, `update_frame`/`walk_rows` |
| `crates/cmux-term/src/pty.rs` | `Session`: spawn on a PTY, reader and writer threads, resize |
| `crates/cmux-term/src/input.rs` | Paste and focus encoding through libghostty-vt's C API |
| `crates/cmux-term/src/selection.rs` | `Selection`: cell/word/line selection in absolute rows, text extraction |
| `crates/cmux-term/tests/{spike,pty}.rs` | Byte-stream regression test; real-PTY tests with `sh` |
| `crates/cmux-app/src/lib.rs` | Module list |
| `crates/cmux-app/src/main.rs` | Load fonts and config, open the window |
| `crates/cmux-app/src/keys.rs` | gpui `Keystroke` → ghostty `KeyInput`; default binding table |
| `crates/cmux-app/src/colors.rs` | Cell colour resolution: defaults, inverse, faint, invisible, selection, cursor |
| `crates/cmux-app/src/style.rs` | `TermStyle` from `Config`; applying defaults to `Term`; cell metrics; bundled font loading |
| `crates/cmux-app/src/terminal_view.rs` | `TerminalView`: session, PTY→repaint loop, keyboard, clipboard, focus, title |
| `crates/cmux-app/src/terminal_element.rs` | `TerminalElement`: measure, resize, batch, paint (Zed-derived) |
| `crates/cmux-app/src/mouse.rs` | Mouse: selection, PRIMARY, reporting, wheel scroll |
| `crates/cmux-app/src/ime.rs` | `EntityInputHandler` for IME commit and preedit |
| `crates/cmux-app/assets/fonts/` | JetBrains Mono Regular/Bold/Italic/BoldItalic, `OFL.txt`, `SHA256SUMS` |

---

### Task 1: Toolchain pin, format and lint gates

Covers STATUS "Carry to M1": pin the toolchain, `clippy --all-targets`, rustfmt, and the vendored clippy exclusion.

**Files:**
- Create: `rust-toolchain.toml`
- Modify: `.github/workflows/ci.yml`, any of our crates that `cargo fmt` reformats

**Interfaces:**
- Produces: toolchain 1.99.0 with `cargo fmt` available; CI runs fmt check and `--all-targets` clippy.

- [ ] **Step 1: Pin the toolchain.** Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.99.0"
components = ["clippy", "rustfmt"]
```

Run: `rustup component add rustfmt --toolchain 1.99.0 && cargo fmt --version`. Expected: prints a `rustfmt 1.x` version.

- [ ] **Step 2: Format our crates.** Run: `cargo fmt -p cmux-app -p cmux-config -p cmux-core -p cmux-ipc -p cmux-meta -p cmux-term && git diff --stat`. Expected: zero or more files under `crates/` changed, nothing under `vendor/`. Then `cargo fmt --check -p cmux-app -p cmux-config -p cmux-core -p cmux-ipc -p cmux-meta -p cmux-term` exits 0.

- [ ] **Step 3: Lint tests too.** Run: `cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings`. Expected: clean. Fix any finding in our crates (tests included).

- [ ] **Step 4: Update CI.** Replace `.github/workflows/ci.yml` with:

```yaml
name: CI
on:
  push:
  pull_request:

permissions:
  contents: read

jobs:
  linux:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive
      - name: STATUS.md line limit
        run: scripts/check-status-lines.sh
      - name: System libraries (gpui, bindgen)
        run: |
          sudo apt-get update
          sudo apt-get install -y --no-install-recommends \
            clang libclang-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
            libvulkan-dev libfontconfig-dev libfreetype-dev libx11-xcb-dev libxcb1-dev \
            libssl-dev libzstd-dev
      # Must match rust-toolchain.toml.
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.99.0"
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - name: Zig 0.16.0
        # Ghostty's build.zig runs a bare `zig env`, so zig must be on PATH (ZIG alone isn't enough).
        run: |
          scripts/install-zig.sh
          echo "$PWD/.tools/zig" >> "$GITHUB_PATH"
      - name: Format (our crates; vendored code is left as upstream wrote it)
        run: cargo fmt --check -p cmux-app -p cmux-config -p cmux-core -p cmux-ipc -p cmux-meta -p cmux-term
      - run: cargo build --workspace --locked
      - run: cargo test --workspace --locked
      # Vendored ghostty-vt fails clippy::chunks_exact_to_as_chunks on 1.99 (kitty.rs); we don't edit vendored code.
      - run: cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
```

- [ ] **Step 5: Run the CI commands locally.** Run: `cargo build --workspace --locked && cargo test --workspace --locked`. Expected: all pass (spike test included).

- [ ] **Step 6: Commit and push.**

```bash
git add rust-toolchain.toml .github/workflows/ci.yml crates
git commit -m "build: pin toolchain 1.99.0, add fmt check and all-targets clippy" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push -u origin m1-terminal
gh run watch --exit-status "$(gh run list --branch m1-terminal --limit 1 --json databaseId --jq '.[0].databaseId')"
```

Expected: CI green. If `dtolnay/rust-toolchain` cannot install 1.99.0, record the error in the journal and use `run: rustup toolchain install 1.99.0 --component clippy,rustfmt` instead.

---

### Task 2: Ghostty config parser

**Files:**
- Modify: `crates/cmux-config/src/lib.rs`
- Create: `crates/cmux-config/src/ghostty/mod.rs`

**Interfaces:**
- Produces (module `cmux_config::ghostty`):
  - `struct Rgb { pub r: u8, pub g: u8, pub b: u8 }` (Copy, Default, PartialEq)
  - `enum DynColor { Rgb(Rgb), CellForeground, CellBackground }`
  - `enum CursorStyle { Block (default), Bar, Underline, BlockHollow }`
  - `enum MetricModifier { Absolute(i32), Percent(f32) }`; `MetricModifier::parse(&str) -> Result<Self, String>`; `MetricModifier::apply(self, px: f32) -> f32` (device px; `Percent` holds a factor, `20%` → 1.2)
  - `struct Config { font_family: Vec<String>, font_size: f32 /*points*/, adjust_cell_height: Option<MetricModifier>, background: Rgb, foreground: Rgb, palette: [Option<Rgb>; 256], background_opacity: f32, background_opacity_cells: bool, cursor_style: CursorStyle, cursor_style_blink: Option<bool>, cursor_color: Option<DynColor>, cursor_text: Option<DynColor>, selection_background: Option<DynColor>, selection_foreground: Option<DynColor>, faint_opacity: f32 }` (all `pub`, `Default` = Ghostty's Linux defaults)
  - `Config::apply(&mut self, key: &str, value: &str) -> Result<(), String>`
  - `fn entries(text: &str) -> impl Iterator<Item = (usize /*1-based line*/, &str /*key*/, &str /*value*/)>`
  - `fn parse_rgb(&str) -> Result<Rgb, String>`

- [ ] **Step 1: Write the failing tests.** Replace `crates/cmux-config/src/lib.rs` with:

```rust
//! cmux-config: Ghostty config (M1) and keybindings (M2).

pub mod ghostty;
```

Create `crates/cmux-config/src/ghostty/mod.rs` containing only the tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> (Config, Vec<String>) {
        let mut cfg = Config::default();
        let mut warnings = Vec::new();
        for (line, key, value) in entries(text) {
            if let Err(e) = cfg.apply(key, value) {
                warnings.push(format!("{line}: {key}: {e}"));
            }
        }
        (cfg, warnings)
    }

    #[test]
    fn entries_skip_comments_trim_and_unquote() {
        let text = "# comment\n\n  font-family = \"JetBrains Mono\"  \r\nfont-size=13\nbare-key\n";
        let got: Vec<_> = entries(text).collect();
        assert_eq!(
            got,
            vec![(3, "font-family", "JetBrains Mono"), (4, "font-size", "13"), (5, "bare-key", "")]
        );
    }

    #[test]
    fn value_keeps_text_after_first_equals() {
        let got: Vec<_> = entries("palette = 1=#ff0000").collect();
        assert_eq!(got, vec![(1, "palette", "1=#ff0000")]);
    }

    #[test]
    fn hex_colours() {
        assert_eq!(parse_rgb("#282c34"), Ok(Rgb { r: 0x28, g: 0x2c, b: 0x34 }));
        assert_eq!(parse_rgb("FFFFFF"), Ok(Rgb { r: 255, g: 255, b: 255 }));
        assert_eq!(parse_rgb("#f80"), Ok(Rgb { r: 0xff, g: 0x88, b: 0x00 }));
        assert!(parse_rgb("red").is_err());
        assert!(parse_rgb("#12345g").is_err());
        assert!(parse_rgb("#ééé").is_err());
    }

    #[test]
    fn spec_keys_apply() {
        let (cfg, warnings) = parse(
            "font-family = Fira Code\nfont-family = Noto Color Emoji\nfont-size = 14.5\n\
             adjust-cell-height = 20%\nbackground = #000000\nforeground = #eeeeee\n\
             palette = 0=#111111\npalette = 0x0f=#222222\nbackground-opacity = 0.85\n\
             background-opacity-cells\ncursor-style = bar\ncursor-style-blink = false\n\
             cursor-color = #ff0000\ncursor-text = cell-background\n\
             selection-background = #333333\nselection-foreground = cell-foreground\n\
             faint-opacity = 0.4\n",
        );
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(cfg.font_family, vec!["Fira Code", "Noto Color Emoji"]);
        assert_eq!(cfg.font_size, 14.5);
        assert_eq!(cfg.adjust_cell_height, Some(MetricModifier::Percent(1.2)));
        assert_eq!(cfg.background, Rgb { r: 0, g: 0, b: 0 });
        assert_eq!(cfg.foreground, Rgb { r: 0xee, g: 0xee, b: 0xee });
        assert_eq!(cfg.palette[0], Some(Rgb { r: 0x11, g: 0x11, b: 0x11 }));
        assert_eq!(cfg.palette[15], Some(Rgb { r: 0x22, g: 0x22, b: 0x22 }));
        assert_eq!(cfg.background_opacity, 0.85);
        assert!(cfg.background_opacity_cells);
        assert_eq!(cfg.cursor_style, CursorStyle::Bar);
        assert_eq!(cfg.cursor_style_blink, Some(false));
        assert_eq!(cfg.cursor_color, Some(DynColor::Rgb(Rgb { r: 255, g: 0, b: 0 })));
        assert_eq!(cfg.cursor_text, Some(DynColor::CellBackground));
        assert_eq!(cfg.selection_background, Some(DynColor::Rgb(Rgb { r: 0x33, g: 0x33, b: 0x33 })));
        assert_eq!(cfg.selection_foreground, Some(DynColor::CellForeground));
        assert_eq!(cfg.faint_opacity, 0.4);
    }

    #[test]
    fn empty_font_family_resets_the_list() {
        let (cfg, _) = parse("font-family = A\nfont-family = \"\"\nfont-family = B\n");
        assert_eq!(cfg.font_family, vec!["B"]);
    }

    #[test]
    fn metric_modifiers() {
        assert_eq!(MetricModifier::parse("2"), Ok(MetricModifier::Absolute(2)));
        assert_eq!(MetricModifier::parse("-20%"), Ok(MetricModifier::Percent(0.8)));
        assert_eq!(MetricModifier::parse("-150%"), Ok(MetricModifier::Percent(0.0)));
        assert!(MetricModifier::parse("2.5").is_err());
        assert_eq!(MetricModifier::Absolute(2).apply(17.0), 19.0);
        assert_eq!(MetricModifier::Percent(1.2).apply(17.0), 20.0);
    }

    #[test]
    fn malformed_values_warn_and_keep_defaults() {
        let (cfg, warnings) = parse(
            "font-size = big\nbackground = red\npalette = 300=#000000\ncursor-style = triangle\n\
             background-opacity = 2\nunknown-key = whatever\n",
        );
        assert_eq!(warnings.len(), 4, "{warnings:?}");
        assert!(warnings[0].starts_with("1: font-size:"), "{warnings:?}");
        let defaults = Config::default();
        assert_eq!(cfg.font_size, defaults.font_size);
        assert_eq!(cfg.background, defaults.background);
        assert_eq!(cfg.palette, defaults.palette);
        assert_eq!(cfg.cursor_style, defaults.cursor_style);
        assert_eq!(cfg.background_opacity, 1.0, "clamped, not rejected");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail.** Run: `cargo test -p cmux-config`. Expected: compile errors (`cannot find type Config`, `cannot find function entries`).

- [ ] **Step 3: Implement.** Insert above the tests module in `crates/cmux-config/src/ghostty/mod.rs`:

```rust
//! The subset of Ghostty's config that cmux-linux uses.
//!
//! Syntax and semantics follow Ghostty's `src/cli/args.zig` (line format) and
//! `src/config/Config.zig` (keys); see docs/milestones/m1-terminal/research.md §7.
//! Unknown keys are ignored: Ghostty has hundreds and most are not ours.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A colour that may instead follow the cell under it (`cell-foreground` / `cell-background`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynColor {
    Rgb(Rgb),
    CellForeground,
    CellBackground,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    #[default]
    Block,
    Bar,
    Underline,
    BlockHollow,
}

/// `adjust-cell-height`: whole device pixels (`2`, `-1`) or a percentage (`20%`).
/// Percentages are stored as a factor, as Ghostty does (font/Metrics.zig `Modifier.parse`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricModifier {
    Absolute(i32),
    Percent(f32),
}

impl MetricModifier {
    pub fn parse(s: &str) -> Result<Self, String> {
        if let Some(p) = s.strip_suffix('%') {
            let p: f32 = p.trim().parse().map_err(|_| format!("not a percentage: {s}"))?;
            return Ok(Self::Percent((1.0 + p / 100.0).max(0.0)));
        }
        s.parse().map(Self::Absolute).map_err(|_| format!("not a whole number or percentage: {s}"))
    }

    /// Applies the modifier to a height in device pixels, rounding like Ghostty.
    pub fn apply(self, px: f32) -> f32 {
        match self {
            Self::Absolute(d) => px + d as f32,
            Self::Percent(f) => (px * f).round(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// First entry is the primary font, the rest are fallbacks. Empty = Ghostty's default.
    pub font_family: Vec<String>,
    /// Points. Ghostty's Linux default is 12.
    pub font_size: f32,
    pub adjust_cell_height: Option<MetricModifier>,
    pub background: Rgb,
    pub foreground: Rgb,
    pub palette: [Option<Rgb>; 256],
    pub background_opacity: f32,
    pub background_opacity_cells: bool,
    pub cursor_style: CursorStyle,
    /// `None` lets the application (DEC mode 12) decide.
    pub cursor_style_blink: Option<bool>,
    pub cursor_color: Option<DynColor>,
    pub cursor_text: Option<DynColor>,
    pub selection_background: Option<DynColor>,
    pub selection_foreground: Option<DynColor>,
    pub faint_opacity: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_family: Vec::new(),
            font_size: 12.0,
            adjust_cell_height: None,
            background: Rgb { r: 0x28, g: 0x2c, b: 0x34 },
            foreground: Rgb { r: 0xff, g: 0xff, b: 0xff },
            palette: [None; 256],
            background_opacity: 1.0,
            background_opacity_cells: false,
            cursor_style: CursorStyle::Block,
            cursor_style_blink: None,
            cursor_color: None,
            cursor_text: None,
            selection_background: None,
            selection_foreground: None,
            faint_opacity: 0.5,
        }
    }
}

impl Config {
    /// Applies one `key = value` entry. `theme` and `config-file` are handled by [`load`].
    pub fn apply(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "font-family" if value.is_empty() => self.font_family.clear(),
            "font-family" => self.font_family.push(value.to_string()),
            "font-size" => {
                self.font_size = number(value).and_then(|v| {
                    if v > 0.0 { Ok(v) } else { Err(format!("must be positive: {value}")) }
                })?
            }
            "adjust-cell-height" if value.is_empty() => self.adjust_cell_height = None,
            "adjust-cell-height" => self.adjust_cell_height = Some(MetricModifier::parse(value)?),
            "background" => self.background = parse_rgb(value)?,
            "foreground" => self.foreground = parse_rgb(value)?,
            "palette" => {
                let (index, color) = parse_palette(value)?;
                self.palette[index as usize] = Some(color);
            }
            "background-opacity" => self.background_opacity = number(value)?.clamp(0.0, 1.0),
            "background-opacity-cells" => self.background_opacity_cells = boolean(value)?,
            "cursor-style" => {
                self.cursor_style = match value {
                    "block" => CursorStyle::Block,
                    "bar" => CursorStyle::Bar,
                    "underline" => CursorStyle::Underline,
                    "block_hollow" => CursorStyle::BlockHollow,
                    _ => return Err(format!("unknown cursor style: {value}")),
                }
            }
            "cursor-style-blink" if value.is_empty() => self.cursor_style_blink = None,
            "cursor-style-blink" => self.cursor_style_blink = Some(boolean(value)?),
            "cursor-color" => self.cursor_color = dyn_color(value)?,
            "cursor-text" => self.cursor_text = dyn_color(value)?,
            "selection-background" => self.selection_background = dyn_color(value)?,
            "selection-foreground" => self.selection_foreground = dyn_color(value)?,
            "faint-opacity" => self.faint_opacity = number(value)?.clamp(0.0, 1.0),
            _ => {}
        }
        Ok(())
    }
}

/// `key = value` entries of one file as (1-based line, key, value). Ghostty has no inline
/// comments; only lines starting with `#` are comments. A bare `key` has an empty value.
pub fn entries(text: &str) -> impl Iterator<Item = (usize, &str, &str)> {
    text.lines().enumerate().filter_map(|(i, line)| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (key, value) = line.split_once('=').unwrap_or((line, ""));
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value);
        Some((i + 1, key.trim(), value))
    })
}

/// `#rrggbb`, `rrggbb`, `#rgb` or `rgb`. Every theme Ghostty ships uses `#rrggbb`.
// shortcut: X11 colour names (`red`) are rejected with a warning; add a name table if users hit it.
pub fn parse_rgb(s: &str) -> Result<Rgb, String> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    let digit = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16);
    let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16);
    let rgb = match hex.len() {
        6 if hex.is_ascii() => (pair(0), pair(2), pair(4)),
        3 if hex.is_ascii() => {
            let d = |i| digit(i).map(|v| v * 17);
            (d(0), d(1), d(2))
        }
        _ => return Err(format!("not a hex colour (#rrggbb): {s}")),
    };
    match rgb {
        (Ok(r), Ok(g), Ok(b)) => Ok(Rgb { r, g, b }),
        _ => Err(format!("not a hex colour (#rrggbb): {s}")),
    }
}

/// `N=COLOR` with N decimal, `0x`, `0o` or `0b`.
fn parse_palette(s: &str) -> Result<(u8, Rgb), String> {
    let (index, color) = s.split_once('=').ok_or_else(|| format!("expected N=COLOR: {s}"))?;
    let index = index.trim();
    let parsed = if let Some(h) = index.strip_prefix("0x") {
        u8::from_str_radix(h, 16)
    } else if let Some(o) = index.strip_prefix("0o") {
        u8::from_str_radix(o, 8)
    } else if let Some(b) = index.strip_prefix("0b") {
        u8::from_str_radix(b, 2)
    } else {
        index.parse()
    };
    let index = parsed.map_err(|_| format!("palette index must be 0-255: {index}"))?;
    Ok((index, parse_rgb(color.trim())?))
}

fn dyn_color(s: &str) -> Result<Option<DynColor>, String> {
    Ok(Some(match s {
        "" => return Ok(None),
        "cell-foreground" => DynColor::CellForeground,
        "cell-background" => DynColor::CellBackground,
        _ => DynColor::Rgb(parse_rgb(s)?),
    }))
}

fn number(s: &str) -> Result<f32, String> {
    s.parse().map_err(|_| format!("not a number: {s}"))
}

/// A bare key (`background-opacity-cells`) means true, as in Ghostty.
fn boolean(s: &str) -> Result<bool, String> {
    match s {
        "" | "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("expected true or false: {s}")),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass.** Run: `cargo test -p cmux-config`. Expected: `test result: ok. 7 passed`.

- [ ] **Step 5: Format, lint, commit.**

```bash
cargo fmt -p cmux-config
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-config
git commit -m "feat(cmux-config): parse the Ghostty config keys M1 uses" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Ghostty config files: fallback, includes, themes

**Files:**
- Create: `crates/cmux-config/src/ghostty/load.rs`
- Modify: `crates/cmux-config/src/ghostty/mod.rs` (two lines at the top)

**Interfaces:**
- Consumes: `Config`, `Config::apply`, `entries` (Task 2).
- Produces (re-exported from `cmux_config::ghostty`):
  - `struct Env { pub config_dir: PathBuf /* $XDG_CONFIG_HOME/ghostty */, pub resources_dirs: Vec<PathBuf> /* each has themes/ */, pub dark: bool }`
  - `Env::from_system(dark: bool) -> Option<Env>` (XDG_CONFIG_HOME or `$HOME/.config`; resources: `$GHOSTTY_RESOURCES_DIR`, `/usr/share/ghostty`, `/usr/local/share/ghostty`)
  - `struct Loaded { pub config: Config, pub warnings: Vec<String> }` (`Default`)
  - `fn load(env: &Env) -> Loaded` — never panics; problems become warnings `"<path>:<line>: <key>: <problem>"`.
- Semantics (Ghostty `file_load.zig`, `theme.zig`, `Config.zig:2504`): `config.ghostty` wins over legacy `config`; `config-file` paths are relative to the including file, load after its own lines, `?` prefix = optional, cycles warn; theme names search `<config_dir>/themes` then each `<resources>/themes`; `light:X,dark:Y` picks by `dark`; order of application: theme, then user file, then includes (later wins).

- [ ] **Step 1: Write the failing tests.** Create `crates/cmux-config/src/ghostty/load.rs` with only the tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ghostty::Rgb;

    /// A fresh `<tmp>/cmux-config-<name>-<pid>` with `ghostty/` and `res/themes/` inside.
    fn fixture(name: &str) -> (PathBuf, Env) {
        let root = std::env::temp_dir().join(format!("cmux-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("ghostty/themes")).unwrap();
        std::fs::create_dir_all(root.join("res/themes")).unwrap();
        let env = Env { config_dir: root.join("ghostty"), resources_dirs: vec![root.join("res")], dark: true };
        (root, env)
    }

    fn write(root: &Path, rel: &str, text: &str) {
        std::fs::write(root.join(rel), text).unwrap();
    }

    const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
    const WHITE: Rgb = Rgb { r: 255, g: 255, b: 255 };
    const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };

    #[test]
    fn no_config_file_gives_defaults_and_no_warnings() {
        let (_root, env) = fixture("none");
        let loaded = load(&env);
        assert_eq!(loaded.config, Config::default());
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    #[test]
    fn config_ghostty_wins_over_legacy_config() {
        let (root, env) = fixture("prefer");
        write(&root, "ghostty/config.ghostty", "background = #000000\n");
        write(&root, "ghostty/config", "background = #ffffff\n");
        assert_eq!(load(&env).config.background, BLACK);
    }

    #[test]
    fn legacy_config_is_the_fallback() {
        let (root, env) = fixture("legacy");
        write(&root, "ghostty/config", "background = #ffffff\n");
        assert_eq!(load(&env).config.background, WHITE);
    }

    #[test]
    fn includes_load_after_the_including_file() {
        let (root, env) = fixture("include");
        write(&root, "ghostty/config.ghostty", "config-file = extra\nbackground = #000000\nconfig-file = ?missing\n");
        write(&root, "ghostty/extra", "background = #ff0000\n");
        let loaded = load(&env);
        assert_eq!(loaded.config.background, RED);
        assert!(loaded.warnings.is_empty(), "optional include is silent: {:?}", loaded.warnings);
    }

    #[test]
    fn missing_required_include_and_cycles_warn() {
        let (root, env) = fixture("cycle");
        write(&root, "ghostty/config.ghostty", "config-file = a\nconfig-file = nope\n");
        write(&root, "ghostty/a", "config-file = config.ghostty\n");
        let warnings = load(&env).warnings;
        assert!(warnings.iter().any(|w| w.contains("cycle")), "{warnings:?}");
        assert!(warnings.iter().any(|w| w.contains("not found: nope")), "{warnings:?}");
    }

    #[test]
    fn theme_loads_first_and_user_values_override_it() {
        let (root, env) = fixture("theme");
        write(&root, "res/themes/Night", "palette = 1=#ff0000\nbackground = #ffffff\nforeground = #000000\n");
        write(&root, "ghostty/config.ghostty", "background = #000000\ntheme = Night\n");
        let cfg = load(&env).config;
        assert_eq!(cfg.palette[1], Some(RED));
        assert_eq!(cfg.foreground, BLACK, "from theme");
        assert_eq!(cfg.background, BLACK, "user overrides theme");
    }

    #[test]
    fn user_themes_dir_wins_over_resources() {
        let (root, env) = fixture("theme-user");
        write(&root, "res/themes/Night", "background = #ffffff\n");
        write(&root, "ghostty/themes/Night", "background = #ff0000\n");
        write(&root, "ghostty/config.ghostty", "theme = Night\n");
        assert_eq!(load(&env).config.background, RED);
    }

    #[test]
    fn light_dark_pair_follows_appearance() {
        let (root, mut env) = fixture("pair");
        write(&root, "res/themes/Day", "background = #ffffff\n");
        write(&root, "res/themes/Night", "background = #000000\n");
        write(&root, "ghostty/config.ghostty", "theme = light:Day,dark:Night\n");
        assert_eq!(load(&env).config.background, BLACK);
        env.dark = false;
        assert_eq!(load(&env).config.background, WHITE);
    }

    #[test]
    fn missing_theme_and_bad_values_warn_with_location() {
        let (root, env) = fixture("bad");
        write(&root, "ghostty/config.ghostty", "theme = Nope\nfont-size = big\n");
        let loaded = load(&env);
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.warnings.len(), 2, "{:?}", loaded.warnings);
        assert!(loaded.warnings[0].contains("config.ghostty:1: theme: not found: Nope"), "{:?}", loaded.warnings);
        assert!(loaded.warnings[1].contains("config.ghostty:2: font-size:"), "{:?}", loaded.warnings);
    }
}
```

and add at the top of `crates/cmux-config/src/ghostty/mod.rs`, after the module doc comment:

```rust
mod load;

pub use load::{Env, Loaded, load};
```

- [ ] **Step 2: Run tests to verify they fail.** Run: `cargo test -p cmux-config`. Expected: compile errors (`cannot find struct Env`, unresolved imports `load::Loaded`).

- [ ] **Step 3: Implement.** Insert above the tests module in `load.rs`:

```rust
//! Finding and layering Ghostty config files: `config.ghostty` (or legacy `config`), its
//! `config-file` includes, and the `theme` (Ghostty's `config/file_load.zig`, `config/theme.zig`).

use std::path::{Path, PathBuf};

use super::{Config, entries};

/// Where to look. `from_system` reads the real environment; tests build their own.
#[derive(Debug, Clone)]
pub struct Env {
    /// `$XDG_CONFIG_HOME/ghostty`.
    pub config_dir: PathBuf,
    /// Ghostty resource dirs, each holding a `themes/` directory.
    pub resources_dirs: Vec<PathBuf>,
    /// Desktop appearance, for `theme = light:X,dark:Y`.
    pub dark: bool,
}

impl Env {
    pub fn from_system(dark: bool) -> Option<Env> {
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        let mut resources_dirs: Vec<PathBuf> =
            std::env::var_os("GHOSTTY_RESOURCES_DIR").map(PathBuf::from).into_iter().collect();
        resources_dirs.push("/usr/share/ghostty".into());
        resources_dirs.push("/usr/local/share/ghostty".into());
        Some(Env { config_dir: config_home.join("ghostty"), resources_dirs, dark })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Loaded {
    pub config: Config,
    /// One line each, `path:line: key: problem`. Never fatal.
    pub warnings: Vec<String>,
}

struct Entry {
    origin: String,
    key: String,
    value: String,
}

/// Loads theme, then the user's file, then its includes (later wins), as Ghostty does.
pub fn load(env: &Env) -> Loaded {
    let mut warnings = Vec::new();
    let main = ["config.ghostty", "config"]
        .iter()
        .map(|name| env.config_dir.join(name))
        .find(|p| p.is_file());
    let mut user = Vec::new();
    if let Some(main) = main {
        let mut visited = Vec::new();
        read_with_includes(&main, &mut visited, &mut user, &mut warnings);
    }

    let mut config = Config::default();
    if let Some(theme) = user.iter().rev().find(|e| e.key == "theme") {
        match resolve_theme(env, &theme.value) {
            Ok(path) => {
                let mut entries = Vec::new();
                read_file(&path, &mut entries, &mut Vec::new(), &mut warnings);
                apply_all(&mut config, &entries, &mut warnings);
            }
            Err(e) => warnings.push(format!("{}: theme: {e}", theme.origin)),
        }
    }
    apply_all(&mut config, &user, &mut warnings);
    Loaded { config, warnings }
}

fn apply_all(config: &mut Config, entries: &[Entry], warnings: &mut Vec<String>) {
    for e in entries.iter().filter(|e| e.key != "theme") {
        if let Err(msg) = config.apply(&e.key, &e.value) {
            warnings.push(format!("{}: {}: {msg}", e.origin, e.key));
        }
    }
}

/// Reads one file's entries; `config-file` values are returned in `includes`, not as entries.
fn read_file(path: &Path, out: &mut Vec<Entry>, includes: &mut Vec<String>, warnings: &mut Vec<String>) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => return warnings.push(format!("{}: {e}", path.display())),
    };
    for (line, key, value) in entries(&text) {
        if key == "config-file" {
            includes.push(value.to_string());
        } else {
            let origin = format!("{}:{line}", path.display());
            out.push(Entry { origin, key: key.to_string(), value: value.to_string() });
        }
    }
}

fn read_with_includes(
    path: &Path,
    visited: &mut Vec<PathBuf>,
    out: &mut Vec<Entry>,
    warnings: &mut Vec<String>,
) {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if visited.contains(&canonical) {
        return warnings.push(format!("{}: config-file cycle, skipped", path.display()));
    }
    visited.push(canonical);
    let mut includes = Vec::new();
    read_file(path, out, &mut includes, warnings);
    let dir = path.parent().unwrap_or(Path::new("."));
    for include in includes {
        let (optional, name) = match include.strip_prefix('?') {
            Some(rest) => (true, rest),
            None => (false, include.as_str()),
        };
        let target = dir.join(name);
        if target.is_file() {
            read_with_includes(&target, visited, out, warnings);
        } else if !optional {
            warnings.push(format!("{}: config-file not found: {name}", path.display()));
        }
    }
}

/// `name`, an absolute path, or `light:A,dark:B`. Names are looked up in the user's
/// `themes/` first, then each resources dir.
fn resolve_theme(env: &Env, value: &str) -> Result<PathBuf, String> {
    let name = pick_theme(value, env.dark)?;
    let path = Path::new(name);
    if path.is_absolute() {
        return if path.is_file() { Ok(path.into()) } else { Err(format!("not found: {name}")) };
    }
    if name.contains('/') || name.contains('\\') {
        return Err(format!("theme names cannot contain path separators: {name}"));
    }
    std::iter::once(env.config_dir.join("themes"))
        .chain(env.resources_dirs.iter().map(|d| d.join("themes")))
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
        .ok_or_else(|| format!("not found: {name}"))
}

fn pick_theme(value: &str, dark: bool) -> Result<&str, String> {
    if !value.contains("light:") && !value.contains("dark:") {
        return Ok(value);
    }
    let want = if dark { "dark:" } else { "light:" };
    value
        .split(',')
        .find_map(|part| part.trim().strip_prefix(want))
        .map(str::trim)
        .ok_or_else(|| format!("missing {want} in {value}"))
}
```

- [ ] **Step 4: Run tests to verify they pass.** Run: `cargo test -p cmux-config`. Expected: `test result: ok. 16 passed`.

- [ ] **Step 5: Check against the real files (read-only).** Run: `cargo test -p cmux-config` passes, then confirm the theme files this machine has are hex-only: `grep -hv '^\s*#' /usr/share/ghostty/themes/* | grep '=' | sed 's/^[^=]*= *//; s/^[0-9]*=//' | grep -vcE '^#?[0-9a-fA-F]{6}$'`. Expected: `0` (if Ghostty is not installed, skip; the CI tests cover the logic).

- [ ] **Step 6: Format, lint, commit.**

```bash
cargo fmt -p cmux-config
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-config
git commit -m "feat(cmux-config): load config.ghostty, includes and themes" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Term events, PTY replies, resize, scroll and borrowed frames

Covers STATUS "Carry to M1": `on_pty_write`, bell and title events, resize/scroll API, no `Snapshot` clones, dims + `Debug`, 8-bit C1 OSC.

**Files:**
- Modify: `crates/cmux-term/src/lib.rs` (rewrite), `crates/cmux-term/tests/spike.rs` (rewrite)

**Interfaces:**
- Consumes: `osc::{OscScanner, OscEvent}` (M0); `ghostty_vt::{Terminal, RenderState, Callbacks, Cell, CursorInfo, Rgb, Scrollbar, Error}`.
- Produces (crate root `cmux_term`):
  - `enum TermEvent { Title(String), Bell, Cwd(String), Notify { title: Option<String>, body: String } }` (`Debug, Clone, PartialEq, Eq`; OSC events first, then title, then bell, per `feed`)
  - `struct FrameInfo { pub cols: u16, pub rows: u16, pub cursor: Option<CursorInfo>, pub default_fg: Rgb, pub default_bg: Rgb, pub cursor_color: Option<Rgb>, pub scrollbar: Option<Scrollbar> }` (`Debug, Clone, Copy, PartialEq, Eq`)
  - `Term::new(cols: u16, rows: u16, pty_tx: std::sync::mpsc::Sender<Vec<u8>>) -> Result<Term, Error>`
  - `Term::feed(&mut self, bytes: &[u8]) -> Vec<TermEvent>`
  - `Term::title(&self) -> Option<String>`
  - `Term::resize(&mut self, cols: u16, rows: u16, cell_w_px: u32, cell_h_px: u32) -> Result<(), Error>`
  - `Term::scroll(&mut self, delta: isize)` (negative = into history), `Term::scroll_to_bottom(&mut self)`
  - `Term::vt(&self) -> &Terminal`, `Term::vt_mut(&mut self) -> &mut Terminal`
  - `Term::update_frame(&mut self) -> Result<FrameInfo, Error>`; `Term::walk_rows(&mut self, f: impl FnMut(usize, &[Cell])) -> Result<(), Error>`
  - `Term::text_lines(&mut self) -> Result<Vec<String>, Error>` (tests/debugging only)
- Removes: `Snapshot`, `Term::snapshot`.

- [ ] **Step 1: Write the failing tests.** Append this tests module to the end of `crates/cmux-term/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn term(cols: u16, rows: u16) -> (Term, mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = mpsc::channel();
        (Term::new(cols, rows, tx).unwrap(), rx)
    }

    #[test]
    fn device_status_reply_goes_to_the_pty_sender() {
        let (mut t, rx) = term(20, 3);
        t.feed(b"\x1b[6n");
        assert_eq!(rx.try_recv().unwrap(), b"\x1b[1;1R");
    }

    #[test]
    fn title_and_bell_are_events() {
        let (mut t, _rx) = term(20, 3);
        assert_eq!(t.feed(b"\x1b]2;hi\x07\x07"), vec![TermEvent::Title("hi".into()), TermEvent::Bell]);
        assert_eq!(t.feed(b"plain"), vec![]);
    }

    #[test]
    fn eight_bit_c1_osc_is_scanned() {
        let (mut t, _rx) = term(20, 3);
        assert_eq!(t.feed(b"\x9d7;file://h/tmp\x9c"), vec![TermEvent::Cwd("/tmp".into())]);
    }

    #[test]
    fn resize_changes_frame_dims() {
        let (mut t, _rx) = term(20, 3);
        t.resize(10, 4, 8, 16).unwrap();
        let info = t.update_frame().unwrap();
        assert_eq!((info.cols, info.rows), (10, 4));
    }

    #[test]
    fn scroll_moves_viewport_into_history_and_back() {
        let (mut t, _rx) = term(10, 2);
        t.feed(b"1\r\n2\r\n3\r\n4");
        t.scroll(-1);
        assert!(t.update_frame().unwrap().scrollbar.unwrap().scrolled_back());
        assert_eq!(t.text_lines().unwrap()[0].trim_end(), "2");
        t.scroll_to_bottom();
        assert_eq!(t.text_lines().unwrap()[0].trim_end(), "3");
    }

    #[test]
    fn walk_rows_visits_every_viewport_row() {
        let (mut t, _rx) = term(10, 3);
        t.feed(b"ab");
        let info = t.update_frame().unwrap();
        let mut rows = Vec::new();
        t.walk_rows(|row, cells| rows.push((row, cells.len(), cells[1].text.clone()))).unwrap();
        assert_eq!(rows.len(), info.rows as usize);
        assert_eq!(rows[0], (0, 10, "b".to_string()));
    }
}
```

Replace `crates/cmux-term/tests/spike.rs` with:

```rust
//! M0 exit test, kept as a regression test: a recorded byte stream in; text, styles,
//! cursor and events out.
use cmux_term::{Term, TermEvent, ghostty_vt::ColorSpec};

const STREAM: &[u8] = b"\x1b]2;spike\x07\
\x1b]7;file://host/tmp/my%20dir\x07\
hello \x1b[1;32mworld\x1b[0m\r\n\
\x1b]777;notify;Claude;Waiting\x07ab";

#[test]
fn bytes_to_cells_styles_cursor_and_events() {
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut term = Term::new(40, 5, tx).unwrap();
    let events = term.feed(STREAM);

    assert_eq!(
        events,
        vec![
            TermEvent::Cwd("/tmp/my dir".into()),
            TermEvent::Notify { title: Some("Claude".into()), body: "Waiting".into() },
            TermEvent::Title("spike".into()),
        ]
    );
    assert_eq!(term.title().as_deref(), Some("spike"));

    let lines = term.text_lines().unwrap();
    assert_eq!(lines[0], "hello world");
    assert_eq!(lines[1], "ab");

    let info = term.update_frame().unwrap();
    assert_eq!((info.cols, info.rows), (40, 5));
    let mut row0 = Vec::new();
    term.walk_rows(|row, cells| {
        if row == 0 {
            row0 = cells.to_vec();
        }
    })
    .unwrap();

    let w = row0.iter().find(|c| c.text == "w").expect("cell for 'w'");
    assert!(w.bold);
    assert!(matches!(w.fg, ColorSpec::Palette(2)), "fg was {:?}", w.fg);
    let h = row0.iter().find(|c| c.text == "h").expect("cell for 'h'");
    assert!(!h.bold);

    let cursor = info.cursor.expect("cursor visible");
    assert_eq!((cursor.x, cursor.y), (2, 1));
}
```

- [ ] **Step 2: Run tests to verify they fail.** Run: `cargo test -p cmux-term`. Expected: compile errors (`Term::new` takes 2 arguments, `cannot find type TermEvent`, no method `update_frame`).

- [ ] **Step 3: Implement.** Replace everything above the tests module in `crates/cmux-term/src/lib.rs` with:

```rust
//! cmux-term: PTY + libghostty-vt wrapper.

pub mod osc;

pub use ghostty_vt;

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;

use ghostty_vt::{Callbacks, Cell, CursorInfo, Error, RenderState, Rgb, Scrollbar, Terminal};
use osc::{OscEvent, OscScanner};

/// Scrollback lines kept per terminal.
// shortcut: fixed; Ghostty's `scrollback-limit` (bytes) is not mapped yet. Add with M10 persistence.
const SCROLLBACK: usize = 10_000;

const TITLE_CHANGED: u8 = 1;
const BELL: u8 = 2;

/// Things the UI reacts to, in the order they were found in one `feed` call
/// (OSC events first, then title and bell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermEvent {
    Title(String),
    Bell,
    Cwd(String),
    Notify { title: Option<String>, body: String },
}

impl From<OscEvent> for TermEvent {
    fn from(e: OscEvent) -> Self {
        match e {
            OscEvent::Cwd(path) => TermEvent::Cwd(path),
            OscEvent::Notify { title, body } => TermEvent::Notify { title, body },
        }
    }
}

/// Per-frame facts the renderer needs before it walks the rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    pub cols: u16,
    pub rows: u16,
    /// `None` when hidden or scrolled out of view.
    pub cursor: Option<CursorInfo>,
    pub default_fg: Rgb,
    pub default_bg: Rgb,
    /// OSC 12 cursor colour, if the application set one.
    pub cursor_color: Option<Rgb>,
    pub scrollbar: Option<Scrollbar>,
}

pub struct Term {
    vt: Terminal,
    render: RenderState,
    osc: OscScanner,
    flags: Arc<AtomicU8>,
}

impl Term {
    /// `pty_tx` receives the terminal's replies (DA, DSR, OSC colour queries). The caller
    /// writes them to the PTY; they are produced inside `feed`, so never write from there.
    pub fn new(cols: u16, rows: u16, pty_tx: Sender<Vec<u8>>) -> Result<Self, Error> {
        let flags = Arc::new(AtomicU8::new(0));
        let (title, bell) = (flags.clone(), flags.clone());
        let callbacks = Callbacks {
            on_pty_write: Some(Box::new(move |bytes: &[u8]| {
                let _ = pty_tx.send(bytes.to_vec());
            })),
            on_title_changed: Some(Box::new(move || {
                title.fetch_or(TITLE_CHANGED, Ordering::Relaxed);
            })),
            on_bell: Some(Box::new(move || {
                bell.fetch_or(BELL, Ordering::Relaxed);
            })),
        };
        Ok(Self {
            vt: Terminal::new(cols, rows, SCROLLBACK, callbacks)?,
            render: RenderState::new()?,
            osc: OscScanner::new(),
            flags,
        })
    }

    /// Feeds PTY output and returns the events it contained.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<TermEvent> {
        // Scan what Ghostty parsed: 8-bit C1 controls (0x9d OSC, 0x9c ST) arrive as ESC ] / ESC \.
        let normalized = self.vt.vt_write_with_normalized(bytes);
        let mut events: Vec<TermEvent> =
            self.osc.feed(&normalized).into_iter().map(TermEvent::from).collect();
        let flags = self.flags.swap(0, Ordering::Relaxed);
        if flags & TITLE_CHANGED != 0
            && let Some(title) = self.vt.title()
        {
            events.push(TermEvent::Title(title));
        }
        if flags & BELL != 0 {
            events.push(TermEvent::Bell);
        }
        events
    }

    pub fn title(&self) -> Option<String> {
        self.vt.title()
    }

    /// Cell pixel sizes feed pixel-size reports (CSI 14/16 t) and kitty graphics.
    pub fn resize(&mut self, cols: u16, rows: u16, cell_w_px: u32, cell_h_px: u32) -> Result<(), Error> {
        self.vt.resize(cols, rows, cell_w_px, cell_h_px)
    }

    /// Moves the viewport by `delta` rows; negative scrolls into history.
    pub fn scroll(&mut self, delta: isize) {
        self.vt.scroll_delta(delta);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.vt.scroll_to_bottom();
    }

    /// The engine, for encoders, modes and selection queries.
    pub fn vt(&self) -> &Terminal {
        &self.vt
    }

    pub fn vt_mut(&mut self) -> &mut Terminal {
        &mut self.vt
    }

    /// Snapshots the viewport for this frame. Call before [`Term::walk_rows`].
    pub fn update_frame(&mut self) -> Result<FrameInfo, Error> {
        self.render.update(&mut self.vt)?;
        let (cols, rows) = self.render.size();
        let (default_bg, default_fg) = self.render.default_colors();
        Ok(FrameInfo {
            cols,
            rows,
            cursor: self.render.cursor(),
            default_fg,
            default_bg,
            cursor_color: self.render.cursor_color(),
            scrollbar: self.vt.scrollbar(),
        })
    }

    /// Visits each viewport row of the last [`Term::update_frame`] without copying cells.
    pub fn walk_rows(&mut self, mut f: impl FnMut(usize, &[Cell])) -> Result<(), Error> {
        self.render.walk_rows(|row, _dirty, cells| f(row, cells))
    }

    /// The viewport as text lines. For tests and debugging, not the render path.
    pub fn text_lines(&mut self) -> Result<Vec<String>, Error> {
        self.render.update(&mut self.vt)?;
        self.render.text_lines()
    }
}
```

- [ ] **Step 4: Run tests to verify they pass.** Run: `cargo test -p cmux-term`. Expected: lib `test result: ok. 13 passed` (7 OSC + 6 new), spike `1 passed`.

- [ ] **Step 5: Format, lint, commit.**

```bash
cargo fmt -p cmux-term
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-term
git commit -m "feat(cmux-term): events, PTY replies, resize, scroll and borrowed frames" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: PTY session

**Files:**
- Create: `crates/cmux-term/src/pty.rs`, `crates/cmux-term/tests/pty.rs`
- Modify: `Cargo.toml` (workspace deps), `crates/cmux-term/Cargo.toml`, `crates/cmux-term/src/lib.rs` (module list), `NOTICE`, `Cargo.lock`

**Interfaces:**
- Consumes: `Term::new`, `Term::feed`, `Term::resize`, `TermEvent` (Task 4).
- Produces (module `cmux_term::pty`):
  - `pub use portable_pty::CommandBuilder;`
  - `enum PtyEvent { Output(Vec<TermEvent>), Exited }`
  - `struct Session { pub term: Arc<Mutex<Term>>, .. }`
  - `Session::shell_command() -> CommandBuilder` (login `$SHELL`, `TERM=xterm-256color`, `COLORTERM=truecolor`, cwd `$HOME`)
  - `Session::spawn(cmd: CommandBuilder, cols: u16, rows: u16, init: impl FnOnce(&mut Term), on_event: impl FnMut(PtyEvent) + Send + 'static) -> std::io::Result<Session>` — `init` runs before any output is read; `on_event` runs on the reader thread
  - `Session::write(&self, bytes: Vec<u8>)` (never blocks)
  - `Session::resize(&self, cols: u16, rows: u16, cell_w_px: u32, cell_h_px: u32) -> std::io::Result<()>` (Term first, then the PTY master)
  - `Drop` kills the child.

- [ ] **Step 1: Add the dependency.** In the root `Cargo.toml` `[workspace.dependencies]`, add after `gpui = "=0.2.2"`:

```toml
portable-pty = "0.9"
```

In `crates/cmux-term/Cargo.toml` `[dependencies]`, add `portable-pty.workspace = true`. In `crates/cmux-term/src/lib.rs` change the module list to:

```rust
pub mod osc;
pub mod pty;
```

Append to `NOTICE` (before the "Further components" paragraph):

```
portable-pty (crates.io 0.9, https://github.com/wezterm/wezterm)
  Copyright (c) 2018 Wez Furlong. MIT.
```

- [ ] **Step 2: Write the failing tests.** Create `crates/cmux-term/tests/pty.rs`:

```rust
//! Real PTYs running `sh`. Unix only, like the rest of M1.
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cmux_term::pty::{CommandBuilder, PtyEvent, Session};

/// Runs `script` under `sh -c`, optionally typing `input`, and returns the screen text after exit.
fn run(script: &str, input: Option<&[u8]>) -> String {
    let (tx, rx) = mpsc::channel();
    let mut cmd = CommandBuilder::new("sh");
    cmd.args(["-c", script]);
    let session = Session::spawn(cmd, 80, 24, |_| {}, move |ev| {
        let _ = tx.send(matches!(ev, PtyEvent::Exited));
    })
    .unwrap();
    if let Some(input) = input {
        session.write(input.to_vec());
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(true) => break,
            Ok(false) => continue,
            Err(_) => panic!("child did not exit in time"),
        }
    }
    let lines = session.term.lock().unwrap().text_lines().unwrap();
    lines.join("\n")
}

#[test]
fn child_sees_the_spawn_size() {
    assert!(run("stty size", None).contains("24 80"));
}

#[test]
fn written_bytes_reach_the_child() {
    assert!(run("read line; echo \"got:$line\"", Some(b"abc\r")).contains("got:abc"));
}

#[test]
fn terminal_replies_are_written_back_to_the_child() {
    // The DSR reply ESC [ 1 ; 1 R comes from Term's on_pty_write via the writer thread.
    let out = run("stty -icanon -echo; printf '\\033[6n'; head -c 6 | od -An -tx1", None);
    assert!(out.contains("1b 5b 31 3b 31 52"), "{out}");
}

#[test]
fn shell_command_sets_term_and_colorterm() {
    let cmd = Session::shell_command();
    assert_eq!(cmd.get_env("TERM").and_then(|v| v.to_str()), Some("xterm-256color"));
    assert_eq!(cmd.get_env("COLORTERM").and_then(|v| v.to_str()), Some("truecolor"));
}
```

and create `crates/cmux-term/src/pty.rs` containing only the module doc comment:

```rust
//! Runs a command on a PTY and pumps bytes between it and a [`Term`].
```

- [ ] **Step 3: Run tests to verify they fail.** Run: `cargo test -p cmux-term --test pty`. Expected: compile errors (`unresolved imports cmux_term::pty::CommandBuilder, PtyEvent, Session`). Then check the lock: `grep -A1 'name = "libc"' Cargo.lock`. Expected: `version = "0.2.189"`. If it moved, run `cargo update -p libc --precise 0.2.189` and re-check.

- [ ] **Step 4: Implement.** Replace `crates/cmux-term/src/pty.rs` with:

```rust
//! Runs a command on a PTY and pumps bytes between it and a [`Term`].
//!
//! Threads: a reader feeds PTY output into the `Term` (under its mutex) and reports events;
//! a writer owns the PTY writer and drains one channel that carries both user input and the
//! terminal's own replies, so nothing ever writes from inside `Term::feed`.

use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

pub use portable_pty::CommandBuilder;
use portable_pty::{Child, MasterPty, PtySize, native_pty_system};

use crate::{Term, TermEvent};

#[derive(Debug)]
pub enum PtyEvent {
    /// A chunk of output was parsed; repaint.
    Output(Vec<TermEvent>),
    /// The child closed the PTY (EOF, or EIO on Linux once it exits).
    Exited,
}

pub struct Session {
    pub term: Arc<Mutex<Term>>,
    writer: mpsc::Sender<Vec<u8>>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
}

impl Session {
    /// The user's login shell (`$SHELL`, else the passwd entry) in `$HOME`.
    pub fn shell_command() -> CommandBuilder {
        let mut cmd = CommandBuilder::new_default_prog();
        // shortcut: xterm-256color, not xterm-ghostty, since that terminfo may be missing; kitty
        // keyboard is negotiated by escape sequence either way. Ship terminfo in M6.
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Some(home) = std::env::var_os("HOME") {
            cmd.cwd(home);
        }
        cmd
    }

    /// Spawns `cmd`. `init` runs on the new `Term` before any output is read (set default
    /// colours there so the first OSC 10/11 query already answers). `on_event` runs on the
    /// reader thread.
    pub fn spawn(
        cmd: CommandBuilder,
        cols: u16,
        rows: u16,
        init: impl FnOnce(&mut Term),
        mut on_event: impl FnMut(PtyEvent) + Send + 'static,
    ) -> io::Result<Session> {
        let size = PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
        let pair = native_pty_system().openpty(size).map_err(io::Error::other)?;
        let child = pair.slave.spawn_command(cmd).map_err(io::Error::other)?;
        // Our copy of the slave must close, or the reader never sees EOF when the child exits.
        drop(pair.slave);
        let master = pair.master;
        let mut reader = master.try_clone_reader().map_err(io::Error::other)?;
        let mut pty_writer = master.take_writer().map_err(io::Error::other)?;

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let mut term = Term::new(cols, rows, tx.clone()).map_err(io::Error::other)?;
        init(&mut term);
        let term = Arc::new(Mutex::new(term));

        thread::Builder::new().name("pty-writer".into()).spawn(move || {
            for bytes in rx {
                if pty_writer.write_all(&bytes).and_then(|()| pty_writer.flush()).is_err() {
                    break;
                }
            }
        })?;

        let shared = term.clone();
        thread::Builder::new().name("pty-reader".into()).spawn(move || {
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let events = shared.lock().unwrap().feed(&buf[..n]);
                        on_event(PtyEvent::Output(events));
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            on_event(PtyEvent::Exited);
        })?;

        Ok(Session { term, writer: tx, master, child })
    }

    /// Queues bytes for the child. Never blocks.
    pub fn write(&self, bytes: Vec<u8>) {
        let _ = self.writer.send(bytes);
    }

    /// Resizes the terminal, then tells the kernel (which sends SIGWINCH).
    pub fn resize(&self, cols: u16, rows: u16, cell_w_px: u32, cell_h_px: u32) -> io::Result<()> {
        self.term.lock().unwrap().resize(cols, rows, cell_w_px, cell_h_px).map_err(io::Error::other)?;
        let px = |cells: u16, cell: u32| (u32::from(cells) * cell).min(u32::from(u16::MAX)) as u16;
        let size = PtySize { rows, cols, pixel_width: px(cols, cell_w_px), pixel_height: px(rows, cell_h_px) };
        self.master.resize(size).map_err(io::Error::other)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
```

- [ ] **Step 5: Run tests to verify they pass.** Run: `cargo test -p cmux-term`. Expected: `pty` `test result: ok. 4 passed`; earlier tests still pass.

- [ ] **Step 6: Format, lint, commit.**

```bash
cargo fmt -p cmux-term
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add Cargo.toml Cargo.lock NOTICE crates/cmux-term
git commit -m "feat(cmux-term): PTY session with reader and writer threads" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Paste and focus encoding, selection model

**Files:**
- Create: `crates/cmux-term/src/input.rs`, `crates/cmux-term/src/selection.rs`
- Modify: `crates/cmux-term/src/lib.rs` (module list)

**Interfaces:**
- Consumes: `Term::new`, `Term::feed`, `Term::vt`, `Term::vt_mut` (Task 4); `ghostty_vt::sys::{ghostty_paste_encode, ghostty_focus_encode, GHOSTTY_FOCUS_GAINED, GHOSTTY_FOCUS_LOST, GHOSTTY_SUCCESS}` (bindgen, `vt/paste.h:87`, `vt/focus.h:64`); `Terminal::{select_word_screen, select_line_screen, normalize_selection_point_screen, selection_text_absolute}`.
- Produces:
  - `cmux_term::input::encode_paste(text: &str, bracketed: bool) -> Vec<u8>`
  - `cmux_term::input::encode_focus(gained: bool) -> Vec<u8>`
  - `cmux_term::selection::SelectMode { Cell, Word, Line }`
  - `cmux_term::selection::Selection` (`Copy`): `new(term: &Term, point: SelectionPoint, mode: SelectMode) -> Selection`; `extend(&mut self, term: &Term, point: SelectionPoint)`; `range(&self) -> SelectionRange` (ordered, inclusive); `is_empty_click(&self) -> bool`; `contains(&self, column: u16, row: u32) -> bool` (absolute row); `text(&self, term: &mut Term) -> Option<String>`. Points are absolute: `row = scrollbar.offset + viewport_row`.

- [ ] **Step 1: Write the failing tests.** In `crates/cmux-term/src/lib.rs` change the module list to:

```rust
pub mod input;
pub mod osc;
pub mod pty;
pub mod selection;
```

Create `crates/cmux-term/src/input.rs` with only its tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracketed_paste_wraps_and_keeps_newlines() {
        assert_eq!(encode_paste("a\nb", true), b"\x1b[200~a\nb\x1b[201~");
    }

    #[test]
    fn plain_paste_turns_newlines_into_cr_and_strips_escape() {
        assert_eq!(encode_paste("a\nb\x1bc", false), b"a\rb c");
    }

    #[test]
    fn empty_paste_is_empty() {
        assert_eq!(encode_paste("", false), b"");
    }

    #[test]
    fn focus_reports() {
        assert_eq!(encode_focus(true), b"\x1b[I");
        assert_eq!(encode_focus(false), b"\x1b[O");
    }
}
```

Create `crates/cmux-term/src/selection.rs` with only its tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn term(text: &[u8]) -> Term {
        let (tx, _rx) = std::sync::mpsc::channel();
        let mut t = Term::new(20, 3, tx).unwrap();
        t.feed(text);
        t
    }

    fn p(column: u16, row: u32) -> SelectionPoint {
        SelectionPoint { column, row }
    }

    #[test]
    fn drag_left_selects_cells_in_order() {
        let mut t = term(b"hello world");
        let mut s = Selection::new(&t, p(4, 0), SelectMode::Cell);
        s.extend(&t, p(0, 0));
        assert_eq!(s.text(&mut t).as_deref(), Some("hello"));
        assert!(s.contains(2, 0));
        assert!(!s.contains(5, 0));
        assert!(!s.contains(2, 1));
    }

    #[test]
    fn double_click_selects_a_word() {
        let mut t = term(b"hello world");
        let s = Selection::new(&t, p(7, 0), SelectMode::Word);
        assert_eq!(s.range(), SelectionRange { start: p(6, 0), end: p(10, 0) });
        assert_eq!(s.text(&mut t).as_deref(), Some("world"));
    }

    #[test]
    fn word_drag_extends_by_whole_words() {
        let mut t = term(b"hello world");
        let mut s = Selection::new(&t, p(1, 0), SelectMode::Word);
        s.extend(&t, p(7, 0));
        assert_eq!(s.text(&mut t).as_deref(), Some("hello world"));
    }

    #[test]
    fn triple_click_selects_a_line() {
        let mut t = term(b"hello world\r\none\r\nfoo bar");
        let s = Selection::new(&t, p(1, 2), SelectMode::Line);
        assert_eq!(s.text(&mut t).as_deref(), Some("foo bar"));
    }

    #[test]
    fn click_without_drag_is_empty() {
        let t = term(b"hello");
        assert!(Selection::new(&t, p(1, 0), SelectMode::Cell).is_empty_click());
        assert!(!Selection::new(&t, p(1, 0), SelectMode::Word).is_empty_click());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail.** Run: `cargo test -p cmux-term --lib`. Expected: compile errors (`cannot find function encode_paste`, `cannot find type Selection`).

- [ ] **Step 3: Implement paste and focus.** Insert above the tests module in `input.rs`:

```rust
//! Paste and focus encoding. The vendored wrapper does not cover these, so this calls
//! libghostty-vt's C API (`vt/paste.h`, `vt/focus.h`) directly.

use ghostty_vt::sys;

/// Bytes to send for pasting `text`. Unsafe controls (ESC, NUL, DEL...) become spaces;
/// without bracketed paste newlines become CR; with it the text is wrapped in
/// `ESC[200~ ... ESC[201~`.
pub fn encode_paste(text: &str, bracketed: bool) -> Vec<u8> {
    let mut data = text.as_bytes().to_vec();
    // Encoding never grows the text beyond the two 6-byte bracket markers.
    let mut out = vec![0u8; data.len() + 12];
    let mut written = 0usize;
    let rc = unsafe {
        sys::ghostty_paste_encode(
            data.as_mut_ptr().cast(),
            data.len(),
            bracketed,
            out.as_mut_ptr().cast(),
            out.len(),
            &mut written,
        )
    };
    if rc != sys::GHOSTTY_SUCCESS {
        return Vec::new();
    }
    out.truncate(written);
    out
}

/// `ESC[I` / `ESC[O`, for applications that enabled focus reporting (DEC mode 1004).
pub fn encode_focus(gained: bool) -> Vec<u8> {
    let event = if gained { sys::GHOSTTY_FOCUS_GAINED } else { sys::GHOSTTY_FOCUS_LOST };
    let mut out = [0u8; 8];
    let mut written = 0usize;
    let rc = unsafe { sys::ghostty_focus_encode(event, out.as_mut_ptr().cast(), out.len(), &mut written) };
    if rc != sys::GHOSTTY_SUCCESS {
        return Vec::new();
    }
    out[..written].to_vec()
}
```

- [ ] **Step 4: Implement selection.** Insert above the tests module in `selection.rs`:

```rust
//! Mouse selection in absolute screen rows (scrollback + screen), so it stays put while the
//! viewport scrolls. The vendored wrapper does not expose Ghostty's terminal-owned selection,
//! so the range lives here and the renderer paints it; word and line rules come from Ghostty.

use ghostty_vt::{SelectionPoint, SelectionRange};

use crate::Term;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectMode {
    /// Single click and drag.
    Cell,
    /// Double click: whole words.
    Word,
    /// Triple click: whole lines.
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    mode: SelectMode,
    /// The word or line (or cell) under the initial click.
    anchor: SelectionRange,
    /// Ordered: `start` is never after `end`. Both ends are inclusive.
    range: SelectionRange,
}

fn key(p: SelectionPoint) -> (u32, u16) {
    (p.row, p.column)
}

fn earlier(a: SelectionPoint, b: SelectionPoint) -> SelectionPoint {
    if key(a) <= key(b) { a } else { b }
}

fn later(a: SelectionPoint, b: SelectionPoint) -> SelectionPoint {
    if key(a) >= key(b) { a } else { b }
}

/// The cell, word or line at `point`, ordered.
fn unit(term: &Term, point: SelectionPoint, mode: SelectMode) -> SelectionRange {
    let vt = term.vt();
    let point = vt.normalize_selection_point_screen(point).unwrap_or(point);
    let found = match mode {
        SelectMode::Cell => None,
        SelectMode::Word => vt.select_word_screen(point).ok().flatten(),
        SelectMode::Line => vt.select_line_screen(point).ok().flatten(),
    };
    found
        .map(|r| SelectionRange { start: earlier(r.start, r.end), end: later(r.start, r.end) })
        .unwrap_or(SelectionRange { start: point, end: point })
}

impl Selection {
    pub fn new(term: &Term, point: SelectionPoint, mode: SelectMode) -> Self {
        let anchor = unit(term, point, mode);
        Self { mode, anchor, range: anchor }
    }

    /// Drag to `point`: the selection spans the anchor and the unit under `point`.
    pub fn extend(&mut self, term: &Term, point: SelectionPoint) {
        let head = unit(term, point, self.mode);
        self.range = SelectionRange {
            start: earlier(self.anchor.start, head.start),
            end: later(self.anchor.end, head.end),
        };
    }

    pub fn range(&self) -> SelectionRange {
        self.range
    }

    /// A plain click that never dragged: not a selection.
    pub fn is_empty_click(&self) -> bool {
        self.mode == SelectMode::Cell && self.range.start == self.range.end
    }

    pub fn contains(&self, column: u16, row: u32) -> bool {
        let p = (row, column);
        key(self.range.start) <= p && p <= key(self.range.end)
    }

    /// The selected text, with soft-wrapped lines joined (Ghostty's formatter).
    pub fn text(&self, term: &mut Term) -> Option<String> {
        let (s, e) = (self.range.start, self.range.end);
        term.vt_mut().selection_text_absolute((s.column, u64::from(s.row)), (e.column, u64::from(e.row)))
    }
}
```

- [ ] **Step 5: Run tests to verify they pass.** Run: `cargo test -p cmux-term`. Expected: lib `test result: ok. 22 passed`.

- [ ] **Step 6: Format, lint, commit.**

```bash
cargo fmt -p cmux-term
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-term
git commit -m "feat(cmux-term): paste/focus encoding and selection model" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Key mapping and default bindings

**Files:**
- Create: `crates/cmux-app/src/lib.rs`, `crates/cmux-app/src/keys.rs`
- Modify: `Cargo.toml` (workspace deps), `crates/cmux-app/Cargo.toml`

**Interfaces:**
- Consumes: `cmux_term::ghostty_vt::{KeyInput, KeyAction, Mods, KeyEncoder, sys}`; gpui `Keystroke { modifiers, key, key_char }`, `Modifiers { control, alt, shift, platform, function }` (platform/keystroke.rs:18, :447).
- Produces (module `cmux_app::keys`):
  - `enum AppAction { Copy, Paste }`
  - `const BINDINGS: &[(&str, AppAction)]` — `super-c`, `super-v`, `ctrl-shift-c`, `ctrl-shift-v`
  - `fn chord(ks: &Keystroke) -> String` (`ctrl-alt-shift-super-<key>` order)
  - `fn binding(ks: &Keystroke) -> Option<AppAction>`
  - `fn mods(m: &Modifiers) -> Mods`
  - `fn physical_key(key: &str) -> (sys::GhosttyKey, Option<char> /*unshifted*/)`
  - `fn key_input(ks: &Keystroke, action: KeyAction) -> KeyInput`

- [ ] **Step 1: Wire the crate.** In the root `Cargo.toml` `[workspace.dependencies]` add:

```toml
cmux-term = { path = "crates/cmux-term" }
```

In `crates/cmux-app/Cargo.toml` replace `[dependencies]` with:

```toml
[dependencies]
cmux-term.workspace = true
gpui.workspace = true
```

Create `crates/cmux-app/src/lib.rs`:

```rust
//! cmux-app: the gpui app. Everything but `main` lives here so it can be unit-tested.

pub mod keys;
```

- [ ] **Step 2: Write the failing tests.** Create `crates/cmux-app/src/keys.rs` with only the tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use cmux_term::ghostty_vt::{Callbacks, KeyEncoder, Terminal};

    fn ks(m: &str, key: &str, key_char: Option<&str>) -> Keystroke {
        let modifiers = Modifiers {
            control: m.contains('c'),
            alt: m.contains('a'),
            shift: m.contains('s'),
            platform: m.contains('p'),
            function: false,
        };
        Keystroke { modifiers, key: key.into(), key_char: key_char.map(Into::into) }
    }

    fn encode(k: &Keystroke) -> Vec<u8> {
        let term = Terminal::new(80, 24, 0, Callbacks::default()).unwrap();
        let mut enc = KeyEncoder::new().unwrap();
        enc.sync_from_terminal(&term);
        let mut out = Vec::new();
        enc.encode(&key_input(k, KeyAction::Press), &mut out).unwrap();
        out
    }

    #[test]
    fn encodes_the_key_table() {
        let cases: &[(&str, &str, Option<&str>, &[u8])] = &[
            ("", "a", Some("a"), b"a"),
            ("s", "a", Some("A"), b"A"),
            ("c", "c", None, b"\x03"),
            ("a", "b", Some("b"), b"\x1bb"),
            ("", "1", Some("1"), b"1"),
            ("s", "!", Some("!"), b"!"),
            ("", "space", Some(" "), b" "),
            ("c", "space", None, b"\x00"),
            ("", "enter", Some("\r"), b"\r"),
            ("", "tab", None, b"\t"),
            ("s", "tab", None, b"\x1b[Z"),
            ("", "backspace", None, b"\x7f"),
            ("", "escape", None, b"\x1b"),
            ("", "up", None, b"\x1b[A"),
            ("c", "left", None, b"\x1b[1;5D"),
            ("", "home", None, b"\x1b[H"),
            ("", "end", None, b"\x1b[F"),
            ("", "pageup", None, b"\x1b[5~"),
            ("", "pagedown", None, b"\x1b[6~"),
            ("", "delete", None, b"\x1b[3~"),
            ("", "f1", None, b"\x1bOP"),
            ("", "f5", None, b"\x1b[15~"),
            ("", "f12", None, b"\x1b[24~"),
            ("", "e", Some("é"), "é".as_bytes()),
        ];
        for (m, key, ch, want) in cases {
            assert_eq!(encode(&ks(m, key, *ch)), *want, "mods {m:?} key {key:?}");
        }
    }

    #[test]
    fn physical_keys() {
        assert_eq!(physical_key("a"), (sys::GHOSTTY_KEY_A, Some('a')));
        assert_eq!(physical_key("z"), (sys::GHOSTTY_KEY_Z, Some('z')));
        assert_eq!(physical_key("9"), (sys::GHOSTTY_KEY_DIGIT_9, Some('9')));
        assert_eq!(physical_key("?"), (sys::GHOSTTY_KEY_SLASH, Some('/')));
        assert_eq!(physical_key("f24"), (sys::GHOSTTY_KEY_F24, None));
        assert_eq!(physical_key("f25"), (sys::GHOSTTY_KEY_UNIDENTIFIED, None));
        assert_eq!(physical_key("f"), (sys::GHOSTTY_KEY_F, Some('f')));
        assert_eq!(physical_key("pagedown"), (sys::GHOSTTY_KEY_PAGE_DOWN, None));
        assert_eq!(physical_key("é"), (sys::GHOSTTY_KEY_UNIDENTIFIED, None));
    }

    #[test]
    fn modifiers_map_to_ghostty_bits() {
        let m = mods(&ks("csap", "a", None).modifiers);
        assert!(m.contains(Mods::CTRL | Mods::SHIFT | Mods::ALT | Mods::SUPER));
        assert_eq!(mods(&Modifiers::default()), Mods::default());
    }

    #[test]
    fn control_text_and_ctrl_never_reach_utf8() {
        assert_eq!(key_input(&ks("", "enter", Some("\r")), KeyAction::Press).utf8, "");
        assert_eq!(key_input(&ks("c", "a", Some("a")), KeyAction::Press).utf8, "");
        let shifted = key_input(&ks("s", "a", Some("A")), KeyAction::Repeat);
        assert_eq!((shifted.utf8.as_str(), shifted.consumed_mods), ("A", Mods::SHIFT));
        assert_eq!(shifted.action, Some(KeyAction::Repeat));
    }

    #[test]
    fn default_bindings() {
        assert_eq!(binding(&ks("p", "c", None)), Some(AppAction::Copy));
        assert_eq!(binding(&ks("p", "v", None)), Some(AppAction::Paste));
        assert_eq!(binding(&ks("cs", "c", Some("C"))), Some(AppAction::Copy));
        assert_eq!(binding(&ks("cs", "v", Some("V"))), Some(AppAction::Paste));
        assert_eq!(binding(&ks("c", "c", None)), None);
        assert_eq!(chord(&ks("csap", "x", None)), "ctrl-alt-shift-super-x");
    }
}
```

The expected bytes were checked against the vendored encoder while writing this plan; if one fails, compare with what Ghostty itself sends (`showkey -a` in Ghostty) before changing the expectation.

- [ ] **Step 3: Run tests to verify they fail.** Run: `cargo test -p cmux-app --lib`. Expected: compile errors (`cannot find function key_input`, `cannot find type AppAction`).

- [ ] **Step 4: Implement.** Insert above the tests module in `keys.rs`:

```rust
//! gpui keystrokes to libghostty-vt key events, and the app's default key bindings.

use cmux_term::ghostty_vt::{KeyAction, KeyInput, Mods, sys};
use gpui::{Keystroke, Modifiers};

/// Things a key can do besides typing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Copy,
    Paste,
}

/// Default bindings, as `ctrl-alt-shift-super-key` chords (⌘ maps to Super).
/// Remapping through config arrives in M2; until then this table is the one place to change.
pub const BINDINGS: &[(&str, AppAction)] = &[
    ("super-c", AppAction::Copy),
    ("super-v", AppAction::Paste),
    ("ctrl-shift-c", AppAction::Copy),
    ("ctrl-shift-v", AppAction::Paste),
];

pub fn chord(ks: &Keystroke) -> String {
    let m = &ks.modifiers;
    let mut s = String::new();
    for (on, name) in [(m.control, "ctrl-"), (m.alt, "alt-"), (m.shift, "shift-"), (m.platform, "super-")] {
        if on {
            s.push_str(name);
        }
    }
    s.push_str(&ks.key);
    s
}

pub fn binding(ks: &Keystroke) -> Option<AppAction> {
    let chord = chord(ks);
    BINDINGS.iter().find(|(b, _)| *b == chord).map(|(_, action)| *action)
}

pub fn mods(m: &Modifiers) -> Mods {
    let mut out = Mods::default();
    for (on, bit) in [(m.shift, Mods::SHIFT), (m.control, Mods::CTRL), (m.alt, Mods::ALT), (m.platform, Mods::SUPER)] {
        if on {
            out = out | bit;
        }
    }
    out
}

/// One printable US-layout key: its physical key and unshifted character.
fn ascii_key(c: char) -> Option<(sys::GhosttyKey, char)> {
    let c = c.to_ascii_lowercase();
    let offset = |base: char| c as u32 - base as u32;
    Some(match c {
        'a'..='z' => (sys::GHOSTTY_KEY_A + offset('a'), c),
        '0'..='9' => (sys::GHOSTTY_KEY_DIGIT_0 + offset('0'), c),
        ' ' => (sys::GHOSTTY_KEY_SPACE, ' '),
        '`' | '~' => (sys::GHOSTTY_KEY_BACKQUOTE, '`'),
        '-' | '_' => (sys::GHOSTTY_KEY_MINUS, '-'),
        '=' | '+' => (sys::GHOSTTY_KEY_EQUAL, '='),
        '[' | '{' => (sys::GHOSTTY_KEY_BRACKET_LEFT, '['),
        ']' | '}' => (sys::GHOSTTY_KEY_BRACKET_RIGHT, ']'),
        '\\' | '|' => (sys::GHOSTTY_KEY_BACKSLASH, '\\'),
        ';' | ':' => (sys::GHOSTTY_KEY_SEMICOLON, ';'),
        '\'' | '"' => (sys::GHOSTTY_KEY_QUOTE, '\''),
        ',' | '<' => (sys::GHOSTTY_KEY_COMMA, ','),
        '.' | '>' => (sys::GHOSTTY_KEY_PERIOD, '.'),
        '/' | '?' => (sys::GHOSTTY_KEY_SLASH, '/'),
        '!' => (sys::GHOSTTY_KEY_DIGIT_1, '1'),
        '@' => (sys::GHOSTTY_KEY_DIGIT_2, '2'),
        '#' => (sys::GHOSTTY_KEY_DIGIT_3, '3'),
        '$' => (sys::GHOSTTY_KEY_DIGIT_4, '4'),
        '%' => (sys::GHOSTTY_KEY_DIGIT_5, '5'),
        '^' => (sys::GHOSTTY_KEY_DIGIT_6, '6'),
        '&' => (sys::GHOSTTY_KEY_DIGIT_7, '7'),
        '*' => (sys::GHOSTTY_KEY_DIGIT_8, '8'),
        '(' => (sys::GHOSTTY_KEY_DIGIT_9, '9'),
        ')' => (sys::GHOSTTY_KEY_DIGIT_0, '0'),
        _ => return None,
    })
}

/// gpui's layout-independent key name (gpui platform/linux/platform.rs `from_xkb`) to
/// Ghostty's physical key and unshifted codepoint.
pub fn physical_key(key: &str) -> (sys::GhosttyKey, Option<char>) {
    let mut chars = key.chars();
    if let (Some(c), None) = (chars.next(), chars.next())
        && let Some((k, base)) = ascii_key(c)
    {
        return (k, Some(base));
    }
    let k = match key {
        "enter" => sys::GHOSTTY_KEY_ENTER,
        "tab" => sys::GHOSTTY_KEY_TAB,
        "backspace" => sys::GHOSTTY_KEY_BACKSPACE,
        "escape" => sys::GHOSTTY_KEY_ESCAPE,
        "space" => return (sys::GHOSTTY_KEY_SPACE, Some(' ')),
        "delete" => sys::GHOSTTY_KEY_DELETE,
        "insert" => sys::GHOSTTY_KEY_INSERT,
        "left" => sys::GHOSTTY_KEY_ARROW_LEFT,
        "right" => sys::GHOSTTY_KEY_ARROW_RIGHT,
        "up" => sys::GHOSTTY_KEY_ARROW_UP,
        "down" => sys::GHOSTTY_KEY_ARROW_DOWN,
        "home" => sys::GHOSTTY_KEY_HOME,
        "end" => sys::GHOSTTY_KEY_END,
        "pageup" => sys::GHOSTTY_KEY_PAGE_UP,
        "pagedown" => sys::GHOSTTY_KEY_PAGE_DOWN,
        f if f.len() > 1 && f.starts_with('f') => match f[1..].parse::<u32>() {
            Ok(n @ 1..=24) => sys::GHOSTTY_KEY_F1 + (n - 1),
            _ => sys::GHOSTTY_KEY_UNIDENTIFIED,
        },
        _ => sys::GHOSTTY_KEY_UNIDENTIFIED,
    };
    (k, None)
}

/// The encoder input for a gpui keystroke. `utf8` is the typed text (`key_char`), dropped
/// under Ctrl or when it holds control characters, as the encoder requires.
pub fn key_input(ks: &Keystroke, action: KeyAction) -> KeyInput {
    let m = &ks.modifiers;
    let (key, unshifted) = physical_key(&ks.key);
    let utf8 = match &ks.key_char {
        Some(text) if !m.control && !text.chars().any(char::is_control) => text.clone(),
        _ => String::new(),
    };
    let consumed_mods = if m.shift && !utf8.is_empty() { Mods::SHIFT } else { Mods::default() };
    KeyInput {
        key,
        mods: mods(m),
        consumed_mods,
        utf8,
        unshifted_codepoint: unshifted.map_or(0, u32::from),
        action: Some(action),
        ..Default::default()
    }
}
```

- [ ] **Step 5: Run tests to verify they pass.** Run: `cargo test -p cmux-app --lib`. Expected: `test result: ok. 5 passed`.

- [ ] **Step 6: Format, lint, commit.**

```bash
cargo fmt -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add Cargo.toml Cargo.lock crates/cmux-app
git commit -m "feat(cmux-app): map gpui keystrokes to ghostty key events" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Colours, style, cell metrics and the bundled font

**Files:**
- Create: `crates/cmux-app/src/colors.rs`, `crates/cmux-app/src/style.rs`, `crates/cmux-app/assets/fonts/{JetBrainsMono-Regular,JetBrainsMono-Bold,JetBrainsMono-Italic,JetBrainsMono-BoldItalic}.ttf`, `crates/cmux-app/assets/fonts/OFL.txt`, `crates/cmux-app/assets/fonts/SHA256SUMS`
- Modify: `Cargo.toml`, `crates/cmux-app/Cargo.toml`, `crates/cmux-app/src/lib.rs`, `NOTICE`

**Interfaces:**
- Consumes: `cmux_config::ghostty::{Config, CursorStyle, DynColor, MetricModifier, Rgb}` (Task 2); `Term::vt_mut`, `Term::update_frame` (Task 4); `Terminal::{replace_default_colors, set_default_palette, set_default_cursor}` (terminal.rs:2353, :2381, :2400); gpui `Font`, `FontFeatures::disable_ligatures`, `FontFallbacks::from_fonts`, `FontWeight`, `FontStyle`, `Hsla`, `Rgba`, `App::text_system().add_fonts(Vec<Cow<'static, [u8]>>)` (text_system.rs:104).
- Produces:
  - `cmux_app::colors`: `to_vt(config::Rgb) -> ghostty_vt::Rgb`; `hsla(Rgb, alpha: f32) -> Hsla`; `struct CellColors { pub fg: Rgb, pub bg: Option<Rgb>, pub fg_alpha: f32 }`; `resolve(cell: &Cell, default_fg: Rgb, default_bg: Rgb, faint_opacity: f32) -> CellColors`; `selected(c: CellColors, default_bg: Rgb, fg: Option<DynColor>, bg: Option<DynColor>) -> CellColors`; `cursor(c: CellColors, default_bg: Rgb, osc: Option<Rgb>, color: Option<DynColor>, text: Option<DynColor>) -> (Rgb /*fill*/, Rgb /*glyph*/)`
  - `cmux_app::style`: `const DEFAULT_FONT_FAMILY: &str = "JetBrains Mono"`; `struct TermStyle { pub cfg: Config, pub font: Font, pub font_px: f32 }`; `TermStyle::new(&Config) -> TermStyle`; `TermStyle::window_background(&self) -> Hsla`; `TermStyle::apply_to(&self, term: &mut Term)`; `cell_size(advance: f32, ascent: f32, descent: f32, scale: f32, adjust: Option<MetricModifier>) -> (f32, f32)` (logical px, device-snapped); `load_bundled_fonts(cx: &App)`

- [ ] **Step 1: Download and verify the font.** JetBrains Mono 2.304 (tag `v2.304` = commit `cd5227bd1f61dff3bbd6c814ceaf7ffd95e947d9`), static TTFs, SIL OFL 1.1:

```bash
mkdir -p crates/cmux-app/assets/fonts && cd crates/cmux-app/assets/fonts
BASE=https://raw.githubusercontent.com/JetBrains/JetBrainsMono/cd5227bd1f61dff3bbd6c814ceaf7ffd95e947d9
for f in Regular Bold Italic BoldItalic; do curl -fsSLO "$BASE/fonts/ttf/JetBrainsMono-$f.ttf"; done
curl -fsSLO "$BASE/OFL.txt"
cat > SHA256SUMS <<'SUMS'
a0bf60ef0f83c5ed4d7a75d45838548b1f6873372dfac88f71804491898d138f  JetBrainsMono-Regular.ttf
5590990c82e097397517f275f430af4546e1c45cff408bde4255dad142479dcb  JetBrainsMono-Bold.ttf
9d0a1f7a708e6af183f1193b7e81d40da294f5c67682c085d8401c60aac8ded4  JetBrainsMono-Italic.ttf
4039d5ce0ed225bf9c8b2c8c6436290ae2f356b7e90d70fa666227238324aa3b  JetBrainsMono-BoldItalic.ttf
30f0c136e3c88e422d0791acd97238870f9054a9729bc34cf2ff0d4ed8cac4ad  OFL.txt
SUMS
sha256sum -c SHA256SUMS
fc-scan --format '%{family} | %{style}\n' *.ttf
cd -
```

Expected: five `OK` lines, then `JetBrains Mono | Regular`, `| Bold`, `| Italic`, `| Bold Italic`. Any `FAILED` line: stop and delete the files.

- [ ] **Step 2: Wire the crate and NOTICE.** Root `Cargo.toml` `[workspace.dependencies]` add `cmux-config = { path = "crates/cmux-config" }`; `crates/cmux-app/Cargo.toml` `[dependencies]` add `cmux-config.workspace = true`. Change `crates/cmux-app/src/lib.rs` to:

```rust
//! cmux-app: the gpui app. Everything but `main` lives here so it can be unit-tested.

pub mod colors;
pub mod keys;
pub mod style;
```

Append to `NOTICE` (before the "Further components" paragraph):

```
JetBrains Mono 2.304 (crates/cmux-app/assets/fonts/, https://github.com/JetBrains/JetBrainsMono)
  Copyright 2020 The JetBrains Mono Project Authors. SIL Open Font License 1.1;
  see crates/cmux-app/assets/fonts/OFL.txt. Bundled unmodified as Ghostty's default font.
```

- [ ] **Step 3: Write the failing tests.** Create `crates/cmux-app/src/colors.rs` with only its tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const FG: Rgb = Rgb { r: 200, g: 200, b: 200 };
    const BG: Rgb = Rgb { r: 10, g: 10, b: 10 };
    const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };
    const BLUE: Rgb = Rgb { r: 0, g: 0, b: 255 };

    fn colors(cell: Cell) -> CellColors {
        resolve(&cell, FG, BG, 0.5)
    }

    #[test]
    fn default_cell_uses_default_fg_and_no_bg() {
        assert_eq!(colors(Cell::default()), CellColors { fg: FG, bg: None, fg_alpha: 1.0 });
    }

    #[test]
    fn explicit_colours_pass_through() {
        let c = colors(Cell { resolved_fg: Some(RED), resolved_bg: Some(BLUE), ..Default::default() });
        assert_eq!((c.fg, c.bg), (RED, Some(BLUE)));
    }

    #[test]
    fn inverse_swaps_including_defaults() {
        let c = colors(Cell { inverse: true, ..Default::default() });
        assert_eq!((c.fg, c.bg), (BG, Some(FG)));
        let c = colors(Cell { inverse: true, resolved_fg: Some(RED), resolved_bg: Some(BLUE), ..Default::default() });
        assert_eq!((c.fg, c.bg), (BLUE, Some(RED)));
    }

    #[test]
    fn faint_and_invisible_set_alpha() {
        assert_eq!(colors(Cell { faint: true, ..Default::default() }).fg_alpha, 0.5);
        assert_eq!(colors(Cell { invisible: true, faint: true, ..Default::default() }).fg_alpha, 0.0);
    }

    #[test]
    fn selection_defaults_to_swapping() {
        let c = selected(colors(Cell::default()), BG, None, None);
        assert_eq!((c.fg, c.bg), (BG, Some(FG)));
    }

    #[test]
    fn selection_colours_from_config() {
        let red = DynColor::Rgb(config::Rgb { r: 255, g: 0, b: 0 });
        let base = colors(Cell { resolved_fg: Some(BLUE), ..Default::default() });
        let c = selected(base, BG, Some(DynColor::CellForeground), Some(red));
        assert_eq!((c.fg, c.bg), (BLUE, Some(RED)));
    }

    #[test]
    fn cursor_colours() {
        let base = colors(Cell::default());
        assert_eq!(cursor(base, BG, None, None, None), (FG, BG));
        let red = DynColor::Rgb(config::Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(cursor(base, BG, None, Some(red), Some(DynColor::CellForeground)), (RED, FG));
        assert_eq!(cursor(base, BG, Some(BLUE), Some(red), None), (BLUE, BG), "OSC 12 wins");
    }

    #[test]
    fn hsla_keeps_alpha() {
        assert_eq!(hsla(RED, 0.25).a, 0.25);
    }
}
```

Create `crates/cmux-app/src/style.rs` with only its tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use cmux_config::ghostty::Rgb;

    fn term() -> (Term, std::sync::mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = std::sync::mpsc::channel();
        (Term::new(20, 3, tx).unwrap(), rx)
    }

    fn reply(term: &mut Term, rx: &std::sync::mpsc::Receiver<Vec<u8>>, query: &[u8]) -> String {
        term.feed(query);
        String::from_utf8(rx.try_recv().unwrap_or_default()).unwrap()
    }

    #[test]
    fn default_font_is_bundled_jetbrains_mono_at_12pt() {
        let style = TermStyle::new(&Config::default());
        assert_eq!(style.font.family.as_ref(), DEFAULT_FONT_FAMILY);
        assert_eq!(style.font.features.is_calt_enabled(), Some(false));
        assert_eq!(style.font_px, 16.0);
    }

    #[test]
    fn user_fonts_come_first_then_the_bundled_fallback() {
        let cfg = Config { font_family: vec!["Fira Code".into(), "Noto Emoji".into()], ..Default::default() };
        let style = TermStyle::new(&cfg);
        assert_eq!(style.font.family.as_ref(), "Fira Code");
        let fallbacks = style.font.fallbacks.unwrap();
        assert_eq!(fallbacks.fallback_list(), ["Noto Emoji", DEFAULT_FONT_FAMILY]);
    }

    #[test]
    fn apply_to_answers_colour_queries_with_the_config() {
        let mut cfg = Config::default();
        cfg.palette[1] = Some(Rgb { r: 255, g: 0, b: 0 });
        let (mut t, rx) = term();
        TermStyle::new(&cfg).apply_to(&mut t);
        assert!(reply(&mut t, &rx, b"\x1b]11;?\x07").contains("rgb:2828/2c2c/3434"));
        assert!(reply(&mut t, &rx, b"\x1b]10;?\x07").contains("rgb:ffff/ffff/ffff"));
        assert!(reply(&mut t, &rx, b"\x1b]4;1;?\x07").contains("rgb:ffff/0000/0000"));
    }

    #[test]
    fn apply_to_sets_the_cursor_shape() {
        let cfg = Config { cursor_style: CursorStyle::Bar, ..Default::default() };
        let (mut t, _rx) = term();
        TermStyle::new(&cfg).apply_to(&mut t);
        assert_eq!(t.update_frame().unwrap().cursor.unwrap().shape, CursorShape::Bar);
    }

    #[test]
    fn cell_size_snaps_to_device_pixels() {
        assert_eq!(cell_size(9.6, 13.0, -3.0, 1.0, None), (10.0, 16.0));
        assert_eq!(cell_size(9.6, 13.0, 3.0, 2.0, None), (9.5, 16.0));
        assert_eq!(cell_size(9.6, 13.0, -3.0, 1.0, Some(MetricModifier::Absolute(2))), (10.0, 18.0));
        assert_eq!(cell_size(9.6, 13.0, -3.0, 1.0, Some(MetricModifier::Percent(1.25))), (10.0, 20.0));
    }
}
```

- [ ] **Step 4: Run tests to verify they fail.** Run: `cargo test -p cmux-app --lib`. Expected: compile errors (`cannot find function resolve`, `cannot find struct TermStyle`).

- [ ] **Step 5: Implement colours.** Insert above the tests module in `colors.rs`:

```rust
//! Cell colour resolution. libghostty-vt resolves palette indices but leaves the defaults,
//! inverse, faint and invisible to the renderer (vendor/ghostty/include/ghostty/vt/render.h).

use cmux_config::ghostty::{self as config, DynColor};
use cmux_term::ghostty_vt::{Cell, Rgb};
use gpui::{Hsla, Rgba};

pub fn to_vt(c: config::Rgb) -> Rgb {
    Rgb { r: c.r, g: c.g, b: c.b }
}

pub fn hsla(c: Rgb, alpha: f32) -> Hsla {
    Rgba { r: f32::from(c.r) / 255.0, g: f32::from(c.g) / 255.0, b: f32::from(c.b) / 255.0, a: alpha }.into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellColors {
    pub fg: Rgb,
    /// `None` = the default background: no quad, so window opacity shows through.
    pub bg: Option<Rgb>,
    pub fg_alpha: f32,
}

pub fn resolve(cell: &Cell, default_fg: Rgb, default_bg: Rgb, faint_opacity: f32) -> CellColors {
    let mut fg = cell.resolved_fg.unwrap_or(default_fg);
    let mut bg = cell.resolved_bg;
    if cell.inverse {
        let old_fg = fg;
        fg = bg.unwrap_or(default_bg);
        bg = Some(old_fg);
    }
    let fg_alpha = if cell.invisible {
        0.0
    } else if cell.faint {
        faint_opacity
    } else {
        1.0
    };
    CellColors { fg, bg, fg_alpha }
}

fn pick(c: DynColor, cell: CellColors, default_bg: Rgb) -> Rgb {
    match c {
        DynColor::Rgb(rgb) => to_vt(rgb),
        DynColor::CellForeground => cell.fg,
        DynColor::CellBackground => cell.bg.unwrap_or(default_bg),
    }
}

/// Ghostty's selection colours: unset swaps the cell's fg and bg.
pub fn selected(c: CellColors, default_bg: Rgb, fg: Option<DynColor>, bg: Option<DynColor>) -> CellColors {
    let cell_bg = c.bg.unwrap_or(default_bg);
    CellColors {
        fg: fg.map_or(cell_bg, |d| pick(d, c, default_bg)),
        bg: Some(bg.map_or(c.fg, |d| pick(d, c, default_bg))),
        fg_alpha: c.fg_alpha,
    }
}

/// (cursor fill, glyph colour under a block cursor). An OSC 12 colour beats the config;
/// unset config uses the cell's foreground for the fill and its background for the glyph.
pub fn cursor(
    c: CellColors,
    default_bg: Rgb,
    osc: Option<Rgb>,
    color: Option<DynColor>,
    text: Option<DynColor>,
) -> (Rgb, Rgb) {
    let fill = osc.or(color.map(|d| pick(d, c, default_bg))).unwrap_or(c.fg);
    let glyph = text.map_or(c.bg.unwrap_or(default_bg), |d| pick(d, c, default_bg));
    (fill, glyph)
}
```

- [ ] **Step 6: Implement style.** Insert above the tests module in `style.rs`:

```rust
//! The Ghostty config turned into what the renderer and the terminal need.

use std::borrow::Cow;

use cmux_config::ghostty::{Config, CursorStyle, DynColor, MetricModifier};
use cmux_term::Term;
use cmux_term::ghostty_vt::CursorShape;
use gpui::{App, Font, FontFallbacks, FontFeatures, FontStyle, FontWeight, Hsla};

use crate::colors::{hsla, to_vt};

/// Ghostty's default font (font/embedded.zig), bundled in assets/fonts.
pub const DEFAULT_FONT_FAMILY: &str = "JetBrains Mono";

/// Ghostty sizes fonts in points at 96 dpi on Linux (font/face.zig `default_dpi`).
// shortcut: Linux only; macOS uses 72 dpi. Switch on target_os in the M6 macOS build.
const POINTS_TO_PX: f32 = 96.0 / 72.0;

pub struct TermStyle {
    pub cfg: Config,
    /// Regular face; bold and italic are derived per cell.
    pub font: Font,
    /// Logical pixels.
    pub font_px: f32,
}

impl TermStyle {
    pub fn new(cfg: &Config) -> Self {
        let mut families = cfg.font_family.clone();
        let primary = if families.is_empty() { DEFAULT_FONT_FAMILY.to_string() } else { families.remove(0) };
        // Like Ghostty, fall back to the bundled font after the user's own fallbacks.
        families.push(DEFAULT_FONT_FAMILY.to_string());
        let font = Font {
            family: primary.into(),
            // Ligatures stay off: force_width snaps glyphs, so a ligature would shift the grid.
            features: FontFeatures::disable_ligatures(),
            fallbacks: Some(FontFallbacks::from_fonts(families)),
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        };
        Self { cfg: cfg.clone(), font, font_px: cfg.font_size * POINTS_TO_PX }
    }

    /// Window fill: the configured background at `background-opacity`.
    // shortcut: ignores an app's OSC 11 background change; repaint from FrameInfo.default_bg if needed.
    pub fn window_background(&self) -> Hsla {
        hsla(to_vt(self.cfg.background), self.cfg.background_opacity)
    }

    /// Sets the terminal's default colours, palette and cursor. Call before the first frame
    /// so OSC 4/10/11 queries already answer with the config's colours.
    pub fn apply_to(&self, term: &mut Term) {
        let cfg = &self.cfg;
        let vt = term.vt_mut();
        let cursor = match cfg.cursor_color {
            Some(DynColor::Rgb(c)) => Some(to_vt(c)),
            _ => None,
        };
        vt.replace_default_colors(Some(to_vt(cfg.foreground)), Some(to_vt(cfg.background)), cursor);
        let palette = cfg.palette.map(|c| c.map(to_vt));
        vt.set_default_palette(&palette);
        let shape = match cfg.cursor_style {
            CursorStyle::Block => CursorShape::Block,
            CursorStyle::Bar => CursorShape::Bar,
            CursorStyle::Underline => CursorShape::Underline,
            CursorStyle::BlockHollow => CursorShape::BlockHollow,
        };
        vt.set_default_cursor(Some(shape), cfg.cursor_style_blink);
    }
}

/// Cell size in logical pixels, snapped to whole device pixels the way Ghostty rounds
/// (font/Metrics.zig: `cell_width = @round(face_width)`, `cell_height = @round(ascent - descent + line_gap)`).
/// gpui hides `line_gap` (0 for JetBrains Mono) and negates `descent` on Linux, hence `abs`.
pub fn cell_size(advance: f32, ascent: f32, descent: f32, scale: f32, adjust: Option<MetricModifier>) -> (f32, f32) {
    let w = (advance * scale).round().max(1.0);
    let mut h = ((ascent + descent.abs()) * scale).round();
    if let Some(m) = adjust {
        h = m.apply(h);
    }
    (w / scale, h.max(1.0) / scale)
}

/// Registers the bundled JetBrains Mono faces with gpui.
pub fn load_bundled_fonts(cx: &App) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Italic.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-BoldItalic.ttf").as_slice()),
    ];
    if let Err(e) = cx.text_system().add_fonts(fonts) {
        eprintln!("cmux-linux: could not load the bundled font: {e}");
    }
}
```

- [ ] **Step 7: Run tests to verify they pass.** Run: `cargo test -p cmux-app --lib`. Expected: `test result: ok. 18 passed`.

- [ ] **Step 8: Format, lint, commit.**

```bash
cargo fmt -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add Cargo.toml Cargo.lock NOTICE crates/cmux-app
git commit -m "feat(cmux-app): colour resolution, Ghostty style and bundled JetBrains Mono" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Terminal view and grid element

The first runnable terminal: shell output on screen, typing, resize, cursor, title, focus reports, paste. Mouse and IME follow in Tasks 10 and 11.

**Files:**
- Create: `crates/cmux-app/src/terminal_view.rs`, `crates/cmux-app/src/terminal_element.rs`
- Modify: `crates/cmux-app/src/main.rs` (rewrite), `crates/cmux-app/src/lib.rs`, `crates/cmux-app/Cargo.toml`, `Cargo.toml`, `NOTICE`

**Interfaces:**
- Consumes: `Session::{shell_command, spawn, write, resize}`, `PtyEvent`, `TermEvent`, `Term::{update_frame, walk_rows, scroll_to_bottom, vt}` (Tasks 4–5); `encode_paste`, `encode_focus`, `Selection::contains`, `Selection::text` (Task 6); `keys::{binding, key_input, AppAction}` (Task 7); `TermStyle`, `cell_size`, `load_bundled_fonts`, `colors::*` (Task 8); `cmux_config::ghostty::{Env, load}` (Task 3).
- gpui APIs used (research.md §1–§3, checked in source): `Element` (element.rs:51), `window.request_layout`, `window.text_system().{resolve_font, advance, ascent, descent, shape_line}`, `ShapedLine::paint`, `window.paint_quad(fill(..) | outline(..))`, `window.scale_factor()`, `window.set_window_title`, `Context::{spawn, notify, listener, entity, focus_handle, on_focus_in, on_blur, stop_propagation, quit, write_to_clipboard, read_from_clipboard}`, `WeakEntity::update`, `AsyncApp::background_executor().timer`, `div().track_focus().key_context().on_key_down().on_key_up()`, `Focusable`.
- Produces:
  - `cmux_app::terminal_view::Geometry { pub origin: Point<Pixels>, pub cell_w: f32, pub cell_h: f32, pub cols: u16, pub rows: u16, pub scale: f32 }` with `cell_at(&self, Point<Pixels>) -> (u16, u16)` and `device(&self, logical: f32) -> u32`
  - `cmux_app::terminal_view::TerminalView` (`Render`, `Focusable`), `TerminalView::new(style: Rc<TermStyle>, window: &mut Window, cx: &mut Context<Self>) -> Self`; crate-visible fields `session, style, focus, geometry, selection, marked_text, cursor_cell, cursor_blinks, blink_on, echo_at` and methods `send_input(&mut self, Vec<u8>)`, `encode_key(&mut self, &Keystroke, KeyAction) -> Vec<u8>`, `copy(&mut self, cx)`, `paste_text(&mut self, &str)`, `set_geometry(&mut self, Geometry)`, `painted(&mut self)` — Tasks 10 and 11 use these.
  - `cmux_app::terminal_element::TerminalElement::new(view: Entity<TerminalView>) -> TerminalElement`

- [ ] **Step 1: Wire the crate and NOTICE.** Root `Cargo.toml` `[workspace.dependencies]` add `futures = "0.3"` (already in `Cargo.lock` via gpui); `crates/cmux-app/Cargo.toml` `[dependencies]` add `futures.workspace = true`. Change `crates/cmux-app/src/lib.rs` to:

```rust
//! cmux-app: the gpui app. Everything but `main` lives here so it can be unit-tested.

pub mod colors;
pub mod keys;
pub mod style;
pub mod terminal_element;
pub mod terminal_view;
```

Append to `NOTICE` (before the "Further components" paragraph):

```
Zed terminal view (https://github.com/zed-industries/zed @ 69e2130295c2649963eb639fc70b4f2ee8ea1624,
  crates/terminal_view and crates/terminal)
  Copyright Zed Industries. GPL-3.0-or-later. crates/cmux-app/src/{terminal_element,
  terminal_view,mouse,ime}.rs adapt its cell batching, background merging, prepaint resize,
  paint order, PTY event batching, mouse routing and IME handler.

futures (crates.io 0.3, https://github.com/rust-lang/futures-rs)
  MIT OR Apache-2.0.
```

- [ ] **Step 2: Write the failing test.** Create `crates/cmux-app/src/terminal_view.rs` with only its tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::point;

    #[test]
    fn cell_at_maps_and_clamps_positions() {
        let g = Geometry { origin: point(px(2.), px(2.)), cell_w: 10.0, cell_h: 20.0, cols: 80, rows: 24, scale: 1.0 };
        assert_eq!(g.cell_at(point(px(2.), px(2.))), (0, 0));
        assert_eq!(g.cell_at(point(px(31.), px(41.))), (2, 1));
        assert_eq!(g.cell_at(point(px(-5.), px(-5.))), (0, 0));
        assert_eq!(g.cell_at(point(px(5000.), px(5000.))), (79, 23));
    }

    #[test]
    fn device_pixels_follow_the_scale() {
        let g = Geometry { origin: point(px(0.), px(0.)), cell_w: 9.5, cell_h: 16.0, cols: 1, rows: 1, scale: 2.0 };
        assert_eq!((g.device(g.cell_w), g.device(g.cell_h)), (19, 32));
    }
}
```

Create `crates/cmux-app/src/terminal_element.rs` containing only the `//!` doc-comment lines at the top of the file in Step 4.

- [ ] **Step 3: Run test to verify it fails.** Run: `cargo test -p cmux-app --lib terminal_view`. Expected: compile errors (`cannot find struct Geometry`).

- [ ] **Step 4: Implement the element.** Replace `crates/cmux-app/src/terminal_element.rs` with:

```rust
//! Paints the terminal grid.
//!
//! Adapted from Zed's terminal element (zed-industries/zed@69e2130,
//! crates/terminal_view/src/terminal_element.rs, GPL-3.0-or-later, Copyright Zed Industries):
//! batching column-contiguous same-style cells into one shaped run (`BatchedTextRun`), merging
//! same-colour backgrounds into rects (`BackgroundRegion`), measuring the grid and resizing the
//! terminal in prepaint, and the paint order. Rewritten for libghostty-vt cells.

use cmux_term::ghostty_vt::{CellWidth, CursorShape, UnderlineStyle as VtUnderline};
use gpui::{
    App, BorderStyle, Bounds, Element, ElementId, Entity, Font, FontStyle, FontWeight, GlobalElementId, Hsla,
    InspectorElementId, IntoElement, LayoutId, Pixels, Point, StrikethroughStyle, Style, TextRun, UnderlineStyle,
    Window, fill, outline, point, px, relative, size,
};

use crate::colors::{self, CellColors};
use crate::style::cell_size;
use crate::terminal_view::{Geometry, TerminalView};

pub struct TerminalElement {
    view: Entity<TerminalView>,
}

impl TerminalElement {
    pub fn new(view: Entity<TerminalView>) -> Self {
        Self { view }
    }
}

impl IntoElement for TerminalElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

/// A run of same-colour background cells on one row.
struct Rect {
    row: u16,
    col: u16,
    cells: u16,
    color: Hsla,
}

/// Text shaped and painted as one line, starting at a cell.
struct Run {
    row: u16,
    col: u16,
    cells: u16,
    text: String,
    style: TextRun,
    /// Narrow single-codepoint cells only: `force_width` snaps glyphs, not cells, so wide
    /// and combining cells must stand alone to keep the grid aligned (research.md §1.3).
    batchable: bool,
}

struct CursorPaint {
    row: u16,
    col: u16,
    cells: u16,
    shape: CursorShape,
    color: Hsla,
    /// The glyph repainted in the cursor-text colour, for a solid block.
    glyph: Option<Run>,
}

pub struct Frame {
    origin: Point<Pixels>,
    cell_w: f32,
    cell_h: f32,
    font_size: Pixels,
    rects: Vec<Rect>,
    runs: Vec<Run>,
    cursor: Option<CursorPaint>,
    marked: Option<(u16, u16, String, Hsla)>,
}

fn same_style(a: &TextRun, b: &TextRun) -> bool {
    a.font == b.font && a.color == b.color && a.underline == b.underline && a.strikethrough == b.strikethrough
}

fn face(base: &Font, bold: bool, italic: bool) -> Font {
    let mut font = base.clone();
    if bold {
        font.weight = FontWeight::BOLD;
    }
    if italic {
        font.style = FontStyle::Italic;
    }
    font
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = Frame;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Frame {
        let view = self.view.read(cx);
        let style = view.style.clone();
        let term = view.session.term.clone();
        let selection = view.selection;
        let focused = view.focus.is_focused(window);
        let blink_on = view.blink_on;
        let marked_text = view.marked_text.clone();
        let cfg = &style.cfg;

        // Measure the grid; resize the terminal and PTY when it changes (Zed does this in prepaint too).
        let font_size = px(style.font_px);
        let scale = window.scale_factor();
        let ts = window.text_system();
        let font_id = ts.resolve_font(&style.font);
        let advance = ts.advance(font_id, font_size, 'm').map_or(style.font_px * 0.6, |s| f32::from(s.width));
        let ascent = f32::from(ts.ascent(font_id, font_size));
        let descent = f32::from(ts.descent(font_id, font_size));
        let (cell_w, cell_h) = cell_size(advance, ascent, descent, scale, cfg.adjust_cell_height);
        let cols = ((f32::from(bounds.size.width) / cell_w) as u16).max(1);
        let rows = ((f32::from(bounds.size.height) / cell_h) as u16).max(1);
        let geometry = Geometry { origin: bounds.origin, cell_w, cell_h, cols, rows, scale };
        self.view.update(cx, |view, _| view.set_geometry(geometry));

        let mut rects: Vec<Rect> = Vec::new();
        let mut runs: Vec<Run> = Vec::new();
        let mut cursor = None;
        let mut marked = None;
        let mut cursor_cell = None;
        let mut cursor_blinks = false;
        let mut term = term.lock().unwrap();
        if let Ok(info) = term.update_frame() {
            let (dfg, dbg) = (info.default_fg, info.default_bg);
            let offset = info.scrollbar.map_or(0, |s| s.offset) as u32;
            let bg_alpha = if cfg.background_opacity_cells { cfg.background_opacity } else { 1.0 };
            let at_cursor = info.cursor.map(|c| (c.x, c.y));
            let mut under_cursor: Option<(String, Font, CellColors, u16)> = None;

            let _ = term.walk_rows(|row, cells| {
                let row = row as u16;
                for (i, cell) in cells.iter().enumerate() {
                    let col = i as u16;
                    if matches!(cell.width, CellWidth::SpacerTail | CellWidth::SpacerHead) {
                        continue;
                    }
                    let span = if cell.width == CellWidth::Wide { 2 } else { 1 };
                    let mut c = colors::resolve(cell, dfg, dbg, cfg.faint_opacity);
                    if selection.is_some_and(|s| s.contains(col, offset + u32::from(row))) {
                        c = colors::selected(c, dbg, cfg.selection_foreground, cfg.selection_background);
                    }
                    let font = face(&style.font, cell.bold, cell.italic);
                    if at_cursor == Some((col, row)) {
                        under_cursor = Some((cell.text.clone(), font.clone(), c, span));
                    }
                    if let Some(bg) = c.bg {
                        let color = colors::hsla(bg, bg_alpha);
                        match rects.last_mut() {
                            Some(r) if r.row == row && r.col + r.cells == col && r.color == color => r.cells += span,
                            _ => rects.push(Rect { row, col, cells: span, color }),
                        }
                    }
                    if cell.text.is_empty() || c.fg_alpha == 0.0 {
                        continue;
                    }
                    let fg = colors::hsla(c.fg, c.fg_alpha);
                    let text_run = TextRun {
                        len: cell.text.len(),
                        font,
                        color: fg,
                        background_color: None,
                        underline: cell.underline.then_some(UnderlineStyle {
                            thickness: px(1.),
                            color: Some(fg),
                            wavy: cell.underline_style == Some(VtUnderline::Curly),
                        }),
                        strikethrough: cell.strikethrough.then_some(StrikethroughStyle { thickness: px(1.), color: Some(fg) }),
                    };
                    let batchable = span == 1 && cell.text.chars().count() == 1;
                    match runs.last_mut() {
                        Some(run)
                            if batchable
                                && run.batchable
                                && run.row == row
                                && run.col + run.cells == col
                                && same_style(&run.style, &text_run) =>
                        {
                            run.text.push_str(&cell.text);
                            run.style.len += cell.text.len();
                            run.cells += 1;
                        }
                        _ => runs.push(Run { row, col, cells: span, text: cell.text.clone(), style: text_run, batchable }),
                    }
                }
            });

            cursor_cell = at_cursor;
            cursor_blinks = info.cursor.is_some_and(|c| c.blinking);
            if let Some(c) = info.cursor {
                let (text, font, cell_colors, span) = under_cursor.unwrap_or_else(|| {
                    (String::new(), style.font.clone(), CellColors { fg: dfg, bg: None, fg_alpha: 1.0 }, 1)
                });
                if !marked_text.is_empty() {
                    // While composing, the preedit replaces the cursor (Zed does the same).
                    marked = Some((c.x, c.y, marked_text, colors::hsla(cell_colors.fg, 1.0)));
                } else if !c.blinking || blink_on || !focused {
                    let (fill_rgb, glyph_rgb) =
                        colors::cursor(cell_colors, dbg, info.cursor_color, cfg.cursor_color, cfg.cursor_text);
                    // Ghostty draws an unfocused cursor as a hollow block.
                    let shape = if focused { c.shape } else { CursorShape::BlockHollow };
                    let glyph = (shape == CursorShape::Block && !text.is_empty()).then(|| {
                        let style = TextRun {
                            len: text.len(),
                            font,
                            color: colors::hsla(glyph_rgb, 1.0),
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        };
                        Run { row: c.y, col: c.x, cells: span, text, style, batchable: false }
                    });
                    cursor = Some(CursorPaint {
                        row: c.y,
                        col: c.x,
                        cells: span,
                        shape,
                        color: colors::hsla(fill_rgb, 1.0),
                        glyph,
                    });
                }
            }
        }
        drop(term);
        self.view.update(cx, |view, _| {
            view.cursor_cell = cursor_cell;
            view.cursor_blinks = cursor_blinks;
        });

        Frame { origin: bounds.origin, cell_w, cell_h, font_size, rects, runs, cursor, marked }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _: &mut (),
        frame: &mut Frame,
        window: &mut Window,
        cx: &mut App,
    ) {
        let frame = &*frame;
        for r in &frame.rects {
            window.paint_quad(fill(frame.cell_bounds(r.col, r.row, r.cells), r.color));
        }
        for run in &frame.runs {
            frame.paint_run(run, window, cx);
        }
        if let Some(c) = &frame.cursor {
            let cell = frame.cell_bounds(c.col, c.row, c.cells);
            let thickness = px(2.);
            match c.shape {
                CursorShape::Block => window.paint_quad(fill(cell, c.color)),
                CursorShape::BlockHollow => window.paint_quad(outline(cell, c.color, BorderStyle::Solid)),
                CursorShape::Bar => window.paint_quad(fill(Bounds::new(cell.origin, size(thickness, cell.size.height)), c.color)),
                CursorShape::Underline => {
                    let origin = point(cell.origin.x, cell.origin.y + cell.size.height - thickness);
                    window.paint_quad(fill(Bounds::new(origin, size(cell.size.width, thickness)), c.color));
                }
            }
            if let Some(glyph) = &c.glyph {
                frame.paint_run(glyph, window, cx);
            }
        }
        if let Some((col, row, text, color)) = &frame.marked {
            let style = TextRun {
                len: text.len(),
                font: self.view.read(cx).style.font.clone(),
                color: *color,
                background_color: None,
                underline: Some(UnderlineStyle { thickness: px(1.), color: Some(*color), wavy: false }),
                strikethrough: None,
            };
            let run = Run { row: *row, col: *col, cells: 1, text: text.clone(), style, batchable: false };
            frame.paint_run(&run, window, cx);
        }
        self.view.update(cx, |view, _| view.painted());
    }
}

impl Frame {
    fn cell_bounds(&self, col: u16, row: u16, cells: u16) -> Bounds<Pixels> {
        let origin = point(
            self.origin.x + px(f32::from(col) * self.cell_w),
            self.origin.y + px(f32::from(row) * self.cell_h),
        );
        Bounds::new(origin, size(px(f32::from(cells) * self.cell_w), px(self.cell_h)))
    }

    fn paint_run(&self, run: &Run, window: &mut Window, cx: &mut App) {
        let origin = self.cell_bounds(run.col, run.row, run.cells).origin;
        let line = window.text_system().shape_line(
            run.text.clone().into(),
            self.font_size,
            std::slice::from_ref(&run.style),
            Some(px(self.cell_w)),
        );
        let _ = line.paint(origin, px(self.cell_h), window, cx);
    }
}
```

- [ ] **Step 5: Implement the view.** Insert above the tests module in `terminal_view.rs`:

```rust
//! The terminal view: owns the PTY session, turns gpui input into PTY bytes and hosts the
//! element that paints the grid. The PTY-to-repaint loop and input routing follow Zed's
//! terminal (zed-industries/zed@69e2130, crates/terminal/src/terminal.rs and
//! crates/terminal_view/src/terminal_view.rs, GPL-3.0-or-later, Copyright Zed Industries).

use std::rc::Rc;
use std::time::{Duration, Instant};

use cmux_term::TermEvent;
use cmux_term::ghostty_vt::{KeyAction, KeyEncoder};
use cmux_term::input::{encode_focus, encode_paste};
use cmux_term::pty::{PtyEvent, Session};
use cmux_term::selection::Selection;
use futures::StreamExt;
use futures::channel::mpsc;
use gpui::{
    App, ClipboardItem, Context, FocusHandle, Focusable, IntoElement, KeyDownEvent, KeyUpEvent, Keystroke,
    ParentElement, Pixels, Point, Render, Styled, Subscription, Task, Window, div, prelude::*, px,
};

use crate::keys::{self, AppAction};
use crate::style::TermStyle;
use crate::terminal_element::TerminalElement;

/// Ghostty's default window-padding-x/y is 2 points.
const PADDING_PX: f32 = 2.0 * 96.0 / 72.0;

/// Grid geometry the element measured in its last prepaint, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub origin: Point<Pixels>,
    pub cell_w: f32,
    pub cell_h: f32,
    pub cols: u16,
    pub rows: u16,
    pub scale: f32,
}

impl Geometry {
    /// The viewport cell under a window position, clamped to the grid.
    pub fn cell_at(&self, pos: Point<Pixels>) -> (u16, u16) {
        let x = f32::from(pos.x - self.origin.x) / self.cell_w;
        let y = f32::from(pos.y - self.origin.y) / self.cell_h;
        let clamp = |v: f32, n: u16| (v.max(0.0) as u16).min(n.saturating_sub(1));
        (clamp(x, self.cols), clamp(y, self.rows))
    }

    /// Logical to device pixels. Cells are already snapped, so cell sizes come out whole.
    pub fn device(&self, logical: f32) -> u32 {
        (logical * self.scale).round() as u32
    }
}

pub struct TerminalView {
    pub(crate) session: Session,
    pub(crate) style: Rc<TermStyle>,
    pub(crate) focus: FocusHandle,
    pub(crate) geometry: Option<Geometry>,
    pub(crate) selection: Option<Selection>,
    /// IME preedit text, painted at the cursor.
    pub(crate) marked_text: String,
    /// Viewport cursor cell from the last prepaint, for IME placement.
    pub(crate) cursor_cell: Option<(u16, u16)>,
    pub(crate) cursor_blinks: bool,
    pub(crate) blink_on: bool,
    key_encoder: KeyEncoder,
    title: Option<String>,
    title_dirty: bool,
    /// `CMUX_TRACE_LATENCY=1`: key press time, waiting for its echo.
    trace_latency: bool,
    key_at: Option<Instant>,
    pub(crate) echo_at: Option<Instant>,
    _tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl TerminalView {
    pub fn new(style: Rc<TermStyle>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (tx, mut rx) = mpsc::unbounded::<PtyEvent>();
        let session = Session::spawn(Session::shell_command(), 80, 24, |term| style.apply_to(term), move |ev| {
            let _ = tx.unbounded_send(ev);
        })
        .expect("cmux-linux: could not start the shell on a PTY");

        let pty_task = cx.spawn(async move |this, cx| {
            while let Some(first) = rx.next().await {
                let mut batch = vec![first];
                // First chunk repaints at once (typing echo); a burst then coalesces into one
                // repaint per ~4 ms, as Zed's terminal subscribe loop does.
                while !batch.is_empty() {
                    let events = std::mem::take(&mut batch);
                    if this.update(cx, |view, cx| view.on_pty_events(events, cx)).is_err() {
                        return;
                    }
                    cx.background_executor().timer(Duration::from_millis(4)).await;
                    while let Ok(ev) = rx.try_recv() {
                        batch.push(ev);
                    }
                }
            }
        });
        let blink_task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(600)).await;
                let alive = this.update(cx, |view, cx| {
                    view.blink_on = !view.blink_on;
                    if view.cursor_blinks {
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        });

        let focus = cx.focus_handle();
        let subscriptions = vec![
            cx.on_focus_in(&focus, window, |view, _, cx| view.focus_changed(true, cx)),
            cx.on_blur(&focus, window, |view, _, cx| view.focus_changed(false, cx)),
        ];

        Self {
            session,
            style,
            focus,
            geometry: None,
            selection: None,
            marked_text: String::new(),
            cursor_cell: None,
            cursor_blinks: false,
            blink_on: true,
            key_encoder: KeyEncoder::new().expect("libghostty-vt key encoder"),
            title: None,
            title_dirty: false,
            trace_latency: std::env::var_os("CMUX_TRACE_LATENCY").is_some(),
            key_at: None,
            echo_at: None,
            _tasks: vec![pty_task, blink_task],
            _subscriptions: subscriptions,
        }
    }

    fn on_pty_events(&mut self, events: Vec<PtyEvent>, cx: &mut Context<Self>) {
        for event in events {
            match event {
                // Ghostty closes the surface when its shell exits; M1 has one surface.
                PtyEvent::Exited => return cx.quit(),
                PtyEvent::Output(events) => {
                    if let Some(at) = self.key_at.take() {
                        self.echo_at = Some(at);
                    }
                    for e in events {
                        match e {
                            TermEvent::Title(t) => {
                                self.title = Some(t);
                                self.title_dirty = true;
                            }
                            // Bell, cwd and notifications feed the sidebar and attention UI (M3/M4).
                            TermEvent::Bell | TermEvent::Cwd(_) | TermEvent::Notify { .. } => {}
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    /// Called by the element after painting: logs key-to-echo-to-paint latency when tracing.
    pub(crate) fn painted(&mut self) {
        if let Some(at) = self.echo_at.take()
            && self.trace_latency
        {
            eprintln!("latency key->paint {:.1} ms", at.elapsed().as_secs_f64() * 1000.0);
        }
    }

    /// Writes user input and snaps the viewport back to the live screen, like Ghostty.
    pub(crate) fn send_input(&mut self, bytes: Vec<u8>) {
        if bytes.is_empty() {
            return;
        }
        self.session.term.lock().unwrap().scroll_to_bottom();
        self.session.write(bytes);
        self.blink_on = true;
        if self.trace_latency {
            self.key_at = Some(Instant::now());
        }
    }

    fn focus_changed(&mut self, focused: bool, cx: &mut Context<Self>) {
        if self.session.term.lock().unwrap().vt().mode(1004, false) {
            self.session.write(encode_focus(focused));
        }
        cx.notify();
    }

    pub(crate) fn set_geometry(&mut self, g: Geometry) {
        let changed = self.geometry.is_none_or(|old| {
            (old.cols, old.rows, old.cell_w, old.cell_h) != (g.cols, g.rows, g.cell_w, g.cell_h)
        });
        self.geometry = Some(g);
        if changed {
            let (w, h) = (g.device(g.cell_w), g.device(g.cell_h));
            if let Err(e) = self.session.resize(g.cols, g.rows, w, h) {
                eprintln!("cmux-linux: resize failed: {e}");
            }
        }
    }

    pub(crate) fn encode_key(&mut self, ks: &Keystroke, action: KeyAction) -> Vec<u8> {
        let input = keys::key_input(ks, action);
        let term = self.session.term.lock().unwrap();
        self.key_encoder.sync_from_terminal(term.vt());
        let mut out = Vec::new();
        let _ = self.key_encoder.encode(&input, &mut out);
        out
    }

    fn key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(action) = keys::binding(&ev.keystroke) {
            match action {
                AppAction::Copy => self.copy(cx),
                AppAction::Paste => self.paste(cx),
            }
            return cx.stop_propagation();
        }
        // Super combos are desktop and app shortcuts, never terminal input.
        if ev.keystroke.modifiers.platform {
            return;
        }
        let action = if ev.is_held { KeyAction::Repeat } else { KeyAction::Press };
        let bytes = self.encode_key(&ev.keystroke, action);
        if !bytes.is_empty() {
            // Stopping here also keeps gpui from re-sending the key as IME text.
            self.selection = None;
            self.send_input(bytes);
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn key_up(&mut self, ev: &KeyUpEvent, _: &mut Window, _: &mut Context<Self>) {
        if ev.keystroke.modifiers.platform {
            return;
        }
        // Empty unless the app asked for kitty's report-event-types flag.
        let bytes = self.encode_key(&ev.keystroke, KeyAction::Release);
        if !bytes.is_empty() {
            self.session.write(bytes);
        }
    }

    pub(crate) fn copy(&mut self, cx: &mut Context<Self>) {
        let Some(selection) = self.selection else { return };
        if let Some(text) = selection.text(&mut self.session.term.lock().unwrap()) {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.paste_text(&text);
        }
    }

    pub(crate) fn paste_text(&mut self, text: &str) {
        let bracketed = self.session.term.lock().unwrap().vt().mode(2004, false);
        self.send_input(encode_paste(text, bracketed));
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.title_dirty {
            self.title_dirty = false;
            if let Some(title) = &self.title {
                window.set_window_title(title);
            }
        }
        div()
            .size_full()
            .p(px(PADDING_PX))
            .bg(self.style.window_background())
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .child(TerminalElement::new(cx.entity()))
    }
}
```

- [ ] **Step 6: Replace `main.rs`.** Replace `crates/cmux-app/src/main.rs` with:

```rust
use std::rc::Rc;

use cmux_app::style::{TermStyle, load_bundled_fonts};
use cmux_app::terminal_view::TerminalView;
use cmux_config::ghostty::{Env, load};
use gpui::{
    App, AppContext, Application, Focusable, Bounds, WindowAppearance, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, px, size,
};

fn main() {
    Application::new().run(|cx: &mut App| {
        load_bundled_fonts(cx);
        let dark = matches!(cx.window_appearance(), WindowAppearance::Dark | WindowAppearance::VibrantDark);
        let loaded = Env::from_system(dark).map(|env| load(&env)).unwrap_or_default();
        // shortcut: warnings go to stderr; an in-window warning line comes with the M2 config work.
        for warning in &loaded.warnings {
            eprintln!("cmux-linux: config: {warning}");
        }
        let style = Rc::new(TermStyle::new(&loaded.config));
        // Translucent only when asked. KWin blur needs ext_background_effect_v1, which gpui 0.2.2
        // lacks (M0 journal); Blurred is kept so it works once gpui supports it.
        let window_background = if loaded.config.background_opacity < 1.0 {
            WindowBackgroundAppearance::Blurred
        } else {
            WindowBackgroundAppearance::Opaque
        };
        let bounds = Bounds::centered(None, size(px(1100.), px(700.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background,
                app_id: Some("cmux-linux".into()),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| TerminalView::new(style, window, cx));
                let focus = view.read(cx).focus_handle(cx);
                window.focus(&focus);
                view
            },
        )
        .expect("open window");
        cx.activate(true);
    });
}
```

- [ ] **Step 7: Run tests and lints.** Run: `cargo test --workspace --locked` then the clippy line from Global Constraints. Expected: cmux-app lib `test result: ok. 20 passed`; clippy clean.

- [ ] **Step 8: Run it.** Run: `cargo run -p cmux-app`. Expected: a window with your login shell's prompt in JetBrains Mono (or your configured font) on your Ghostty background. Check each, and fix before committing:
  - Typing `ls --color`, Enter: coloured output; the cursor follows. `printf '\e[2 q'`, `'\e[4 q'`, `'\e[6 q'`: block, underline, bar cursors. Click another window: the cursor turns into a hollow block.
  - Resize the window: `stty size` reports the new rows/cols; `tput cols` matches the visible width.
  - `printf '\e]2;hello\a'`: the window title changes.
  - Ctrl+Shift+V / Super+V pastes clipboard text; a multi-line paste into `cat` arrives with newlines and is not executed line by line in a bracketed-paste shell (zsh/fish).
  - Wide glyphs: `printf '日本語|\nab🎉|\nabcdef|\n'` — the three `|` line up (runs break at wide cells).
  - `exit`: the app quits.
  - If the first frame panics in gpui (debug assertion about updating an entity during prepaint/paint): Zed does the same calls (`terminal_element.rs:999`, `:1268` at the pinned commit), so read the panic message, move the `view.update` out of `paint` into `prepaint` first, and record it in the journal.

- [ ] **Step 9: Format, lint, commit.**

```bash
cargo fmt -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add Cargo.toml Cargo.lock NOTICE crates/cmux-app
git commit -m "feat(cmux-app): terminal view and grid element on a real PTY" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: Mouse: selection, copy-on-select, reporting, wheel

**Files:**
- Create: `crates/cmux-app/src/mouse.rs`
- Modify: `crates/cmux-app/src/terminal_view.rs` (fields, constructor, imports, listeners), `crates/cmux-app/src/lib.rs`

**Interfaces:**
- Consumes: `Geometry::{cell_at, device}`, `TerminalView::{session, geometry, selection, focus, send_input, paste_text, encode_key}` (Task 9); `Selection::{new, extend, is_empty_click, text}`, `SelectMode` (Task 6); `keys::mods` (Task 7); `ghostty_vt::{MouseEncoders, MouseInput, MouseAction, MouseButton, Screen, SelectionPoint}` (mouse.rs:11-160); `Terminal::{mouse_tracking, active_screen, mode, scrollbar}`; gpui `MouseDownEvent, MouseUpEvent, MouseMoveEvent, ScrollWheelEvent, ScrollDelta::pixel_delta, MouseButton, read_from_primary, write_to_primary` (interactive.rs:93-420, app.rs:1036-1048).
- Produces: `TerminalView::{mouse_down, mouse_up, mouse_move, scroll_wheel}` (crate-visible listener targets); fields `mouse: MouseEncoders`, `dragging: bool`, `scroll_px: f32`.
- Behaviour: when the app tracks the mouse and Shift is not held, events are encoded and sent; otherwise left-drag selects (1/2/3 clicks = cell/word/line), release copies to PRIMARY, middle-click pastes PRIMARY, wheel scrolls history (or sends arrows in the alternate screen with mode 1007).

- [ ] **Step 1: Add the module.** Create `crates/cmux-app/src/mouse.rs`:

```rust
//! Mouse input for the terminal view: local selection and wheel scrolling, or reporting to
//! the application when it enabled mouse tracking (Shift bypasses reporting, as in Ghostty).
//! Routing follows Zed's `register_mouse_listeners` and `determine_scroll_lines`
//! (zed-industries/zed@69e2130, GPL-3.0-or-later).

use cmux_term::ghostty_vt::{
    KeyAction, MouseAction, MouseButton as VtButton, MouseInput, Screen, SelectionPoint,
};
use cmux_term::selection::{SelectMode, Selection};
use gpui::{
    ClipboardItem, Context, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent, Window, px,
};

use crate::keys;
use crate::terminal_view::TerminalView;

/// Lines per wheel notch. Ghostty's `mouse-scroll-multiplier` defaults to 3 for discrete wheels.
const WHEEL_LINES_PER_NOTCH: f32 = 3.0;

fn vt_button(b: MouseButton) -> Option<VtButton> {
    match b {
        MouseButton::Left => Some(VtButton::Left),
        MouseButton::Right => Some(VtButton::Right),
        MouseButton::Middle => Some(VtButton::Middle),
        MouseButton::Navigate(_) => None,
    }
}

impl TerminalView {
    fn reporting(&self, mods: &Modifiers) -> bool {
        !mods.shift && self.session.term.lock().unwrap().vt().mouse_tracking()
    }

    /// Absolute selection point (scrollback offset + viewport row) under a window position.
    fn point_at(&self, pos: Point<Pixels>) -> Option<SelectionPoint> {
        let (col, row) = self.geometry?.cell_at(pos);
        let offset = self.session.term.lock().unwrap().vt().scrollbar().map_or(0, |s| s.offset);
        Some(SelectionPoint { column: col, row: (offset + u64::from(row)) as u32 })
    }

    fn report(&mut self, action: MouseAction, button: Option<VtButton>, pos: Point<Pixels>, mods: &Modifiers) {
        let Some(g) = self.geometry else { return };
        let (cw, ch) = (g.device(g.cell_w), g.device(g.cell_h));
        let input = MouseInput {
            action,
            button,
            mods: keys::mods(mods),
            // Device pixels, like the cell size the terminal was resized with.
            position: (f32::from(pos.x - g.origin.x) * g.scale, f32::from(pos.y - g.origin.y) * g.scale),
            screen_size: (cw * u32::from(g.cols), ch * u32::from(g.rows)),
            cell_size: (cw, ch),
            any_button_pressed: action != MouseAction::Release && button.is_some(),
        };
        let mut out = Vec::new();
        if action == MouseAction::Release {
            // The release encoder keeps the protocol that was active at press time.
            let _ = self.mouse.encode_release(input, &mut out);
        } else {
            self.mouse.sync_from_terminal(self.session.term.lock().unwrap().vt());
            let _ = self.mouse.encode(input, &mut out);
        }
        if !out.is_empty() {
            self.session.write(out);
        }
    }

    pub(crate) fn mouse_down(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus);
        if self.reporting(&ev.modifiers) {
            if let Some(b) = vt_button(ev.button) {
                self.report(MouseAction::Press, Some(b), ev.position, &ev.modifiers);
            }
            return;
        }
        match ev.button {
            MouseButton::Left => {
                let Some(point) = self.point_at(ev.position) else { return };
                let mode = match ev.click_count {
                    0 | 1 => SelectMode::Cell,
                    2 => SelectMode::Word,
                    _ => SelectMode::Line,
                };
                self.selection = Some(Selection::new(&self.session.term.lock().unwrap(), point, mode));
                self.dragging = true;
                cx.notify();
            }
            // Middle click pastes the primary selection (X11/Wayland convention).
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            MouseButton::Middle => {
                if let Some(text) = cx.read_from_primary().and_then(|item| item.text()) {
                    self.paste_text(&text);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn mouse_move(&mut self, ev: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.reporting(&ev.modifiers) {
            let button = ev.pressed_button.and_then(vt_button);
            self.report(MouseAction::Motion, button, ev.position, &ev.modifiers);
            return;
        }
        if self.dragging
            && ev.pressed_button == Some(MouseButton::Left)
            && let Some(point) = self.point_at(ev.position)
            && let Some(selection) = self.selection.as_mut()
        {
            selection.extend(&self.session.term.lock().unwrap(), point);
            cx.notify();
        }
    }

    pub(crate) fn mouse_up(&mut self, ev: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.reporting(&ev.modifiers) {
            if let Some(b) = vt_button(ev.button) {
                self.report(MouseAction::Release, Some(b), ev.position, &ev.modifiers);
            }
            return;
        }
        if ev.button != MouseButton::Left || !self.dragging {
            return;
        }
        self.dragging = false;
        if self.selection.is_some_and(|s| s.is_empty_click()) {
            self.selection = None;
        }
        // Ghostty's copy-on-select default on Linux: the selection goes to PRIMARY.
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if let Some(selection) = self.selection
            && let Some(text) = selection.text(&mut self.session.term.lock().unwrap())
        {
            cx.write_to_primary(ClipboardItem::new_string(text));
        }
        cx.notify();
    }

    pub(crate) fn scroll_wheel(&mut self, ev: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(g) = self.geometry else { return };
        let multiplier = match ev.delta {
            ScrollDelta::Lines(_) => WHEEL_LINES_PER_NOTCH,
            ScrollDelta::Pixels(_) => 1.0,
        };
        self.scroll_px += f32::from(ev.delta.pixel_delta(px(g.cell_h)).y) * multiplier;
        let lines = (self.scroll_px / g.cell_h) as i32;
        if lines == 0 {
            return;
        }
        self.scroll_px -= lines as f32 * g.cell_h;
        // gpui: positive y is wheel-up, toward history.
        if self.reporting(&ev.modifiers) {
            let button = if lines > 0 { VtButton::WheelUp } else { VtButton::WheelDown };
            for _ in 0..lines.unsigned_abs() {
                self.report(MouseAction::Press, Some(button), ev.position, &ev.modifiers);
            }
            return;
        }
        let alternate_scroll = {
            let term = self.session.term.lock().unwrap();
            term.vt().active_screen() == Screen::Alternate && term.vt().mode(1007, false)
        };
        if alternate_scroll {
            // Mode 1007 (on by default, ghostty terminal/modes.zig): full-screen apps such as
            // less get arrow keys for the wheel.
            let key = if lines > 0 { "up" } else { "down" };
            let arrow = Keystroke { modifiers: Modifiers::default(), key: key.into(), key_char: None };
            let bytes: Vec<u8> = (0..lines.unsigned_abs()).flat_map(|_| self.encode_key(&arrow, KeyAction::Press)).collect();
            self.session.write(bytes);
        } else {
            self.session.term.lock().unwrap().scroll(-(lines as isize));
            cx.notify();
        }
    }
}
```

In `crates/cmux-app/src/lib.rs` add `pub mod mouse;` after `pub mod keys;`.

- [ ] **Step 2: Add the view state.** In `crates/cmux-app/src/terminal_view.rs`:

Change the ghostty import to:

```rust
use cmux_term::ghostty_vt::{KeyAction, KeyEncoder, MouseEncoders};
```

Add `MouseButton,` to the `gpui::{..}` import list (after `Keystroke,`).

In `pub struct TerminalView`, after `key_encoder: KeyEncoder,` add:

```rust
    pub(crate) mouse: MouseEncoders,
    pub(crate) dragging: bool,
    /// Wheel pixels not yet turned into whole lines.
    pub(crate) scroll_px: f32,
```

In `TerminalView::new`, after `key_encoder: KeyEncoder::new().expect("libghostty-vt key encoder"),` add:

```rust
            mouse: MouseEncoders::new().expect("libghostty-vt mouse encoders"),
            dragging: false,
            scroll_px: 0.0,
```

In `render`, after `.on_key_up(cx.listener(Self::key_up))` add:

```rust
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::mouse_up))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_scroll_wheel(cx.listener(Self::scroll_wheel))
```

- [ ] **Step 3: Build, test, lint.** Run: `cargo test -p cmux-app` and the clippy line. Expected: 20 lib tests pass; clippy clean. (The selection and encoding logic is unit-tested in Tasks 6 and 7; this task is event glue.)

- [ ] **Step 4: Verify by hand.** Run: `cargo run -p cmux-app`, then:
  - `seq 1 500`, wheel up: history scrolls; wheel down returns; typing snaps to the bottom. **Verify direction:** wheel-up must move into history. If reversed, flip the sign comment and change `if lines > 0 { VtButton::WheelUp }`, `"up"` and `scroll(-(lines as isize))` together, and record it in the journal.
  - **Verify notch size:** one wheel notch scrolls 3 lines, like Ghostty. If gpui already reports 3 lines per notch (you see 9), set `WHEEL_LINES_PER_NOTCH` to `1.0` and record it.
  - Drag across text: highlight follows; release; middle-click pastes it. Double-click a word, triple-click a line: whole word/line highlighted. Super+C or Ctrl+Shift+C then Super+V pastes it. A plain click clears the selection.
  - `less /etc/services`: wheel scrolls the pager (alternate scroll, mode 1007), not the history.
  - `vim` with `:set mouse=a`: click moves the cursor, wheel scrolls the buffer, drag makes a visual selection. Shift+drag selects locally instead.
  - `htop`: clicking a column header sorts by it.

- [ ] **Step 5: Format, lint, commit.**

```bash
cargo fmt -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-app
git commit -m "feat(cmux-app): mouse selection, PRIMARY, reporting and wheel scroll" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: IME

**Files:**
- Create: `crates/cmux-app/src/ime.rs`
- Modify: `crates/cmux-app/src/terminal_element.rs` (`paint`, imports), `crates/cmux-app/src/lib.rs`

**Interfaces:**
- Consumes: `TerminalView::{marked_text, geometry, cursor_cell, send_input, focus}` (Task 9); gpui `EntityInputHandler` (input.rs:10), `ElementInputHandler::new(bounds, view)` (input.rs:86), `window.handle_input(&FocusHandle, impl InputHandler, &App)` (window.rs:3400, paint phase only), `UTF16Selection { range, reversed }` (platform.rs:982).
- Produces: `impl EntityInputHandler for TerminalView`. Commit → PTY bytes; preedit → `marked_text`, painted underlined at the cursor by the element (already in Task 9's `prepaint`/`paint`); candidate window at the cursor cell.

- [ ] **Step 1: Add the handler.** Create `crates/cmux-app/src/ime.rs`:

```rust
//! IME (text-input-v3 on Wayland): committed text goes to the PTY, preedit is held for the
//! element to paint at the cursor. Shape follows Zed's `TerminalInputHandler`
//! (zed-industries/zed@69e2130, crates/terminal_view/src/terminal_element.rs, GPL-3.0-or-later).

use std::ops::Range;

use gpui::{Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window, point, px, size};

use crate::terminal_view::TerminalView;

impl EntityInputHandler for TerminalView {
    fn text_for_range(
        &mut self,
        _range: Range<usize>,
        _adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        // An empty caret, so the IME treats the terminal as an editable field.
        Some(UTF16Selection { range: 0..0, reversed: false })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        (!self.marked_text.is_empty()).then(|| 0..self.marked_text.encode_utf16().count())
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_text.clear();
        cx.notify();
    }

    fn replace_text_in_range(&mut self, _: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        self.marked_text.clear();
        self.send_input(text.as_bytes().to_vec());
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked_text = new_text.to_string();
        cx.notify();
    }

    /// The candidate window sits at the cursor cell (offset by `range.start` cells).
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let g = self.geometry?;
        let (col, row) = self.cursor_cell?;
        let x = (f32::from(col) + range.start as f32) * g.cell_w;
        let origin = point(g.origin.x + px(x), g.origin.y + px(f32::from(row) * g.cell_h));
        Some(Bounds::new(origin, size(px(g.cell_w), px(g.cell_h))))
    }

    fn character_index_for_point(&mut self, _: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        None
    }
}
```

In `crates/cmux-app/src/lib.rs` add `pub mod ime;` after `pub mod colors;`.

- [ ] **Step 2: Register it during paint.** In `crates/cmux-app/src/terminal_element.rs`: add `ElementInputHandler,` to the `gpui::{..}` import list (after `ElementId,`); in `fn paint` rename the `_bounds: Bounds<Pixels>` parameter to `bounds: Bounds<Pixels>`; and make the first lines of the `paint` body:

```rust
        let focus = self.view.read(cx).focus.clone();
        window.handle_input(&focus, ElementInputHandler::new(bounds, self.view.clone()), cx);
        let frame = &*frame;
```

- [ ] **Step 3: Build, test, lint.** Run: `cargo test -p cmux-app` and the clippy line. Expected: 20 lib tests pass; clippy clean.

- [ ] **Step 4: Verify by hand.** With an input method running (fcitx5 or ibus with a Japanese, Chinese or Korean engine; KDE: System Settings → Keyboard → Virtual Keyboard → Fcitx 5), run `cargo run -p cmux-app` and `cat`:
  - Compose a word: the preedit shows underlined at the cursor and the candidate popup sits next to it; committing prints it once (not twice).
  - With the IME off, typing ASCII still prints each key exactly once.
  - A dead-key/compose sequence (e.g. `´` then `e`) prints `é` once.
  - If no IME is installed, record "IME not exercised" in the journal and keep the dead-key check.

- [ ] **Step 5: Format, lint, commit.**

```bash
cargo fmt -p cmux-app
cargo clippy --workspace --all-targets --locked --no-deps --exclude ghostty-vt --exclude ghostty-vt-sys -- -D warnings
git add crates/cmux-app
git commit -m "feat(cmux-app): IME commit and preedit" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 12: Exit-criteria verification

Manual. Build release for the checks: `cargo build --release -p cmux-app`, run `target/release/cmux-app`. Screenshots: `mkdir -p ~/Pictures/cmux-m1` once, then `spectacle -b -n -a -d 1500 -o ~/Pictures/cmux-m1/<name>.png` (active window, 1.5 s delay; focus the target window during the delay). Record every result in the journal (Task 13 Step 1).

**Files:** none tracked (screenshots stay outside the repo). Code changes only if a verify step's fallback applies.

- [ ] **Step 1: Cell size and pixel convention (research UNVERIFIED: cell height, descent sign, logical vs device px).** In Ghostty and in cmux-linux, with the same config, run:

```bash
printf '\e[16t'; IFS=';' read -rsd t _ h w; echo "cell ${w}x${h} px"; stty size
```

Expected: identical `cell WxH` in both (both report device pixels, ours from `Geometry::device`). Then resize both windows to the same `stty size` (e.g. 100 cols × 30 rows: start Ghostty with `ghostty --window-width=100 --window-height=30`, drag cmux-linux until `stty size` says `30 100`).
  - If our height differs from Ghostty's: print gpui's metrics once (temporary `eprintln!("advance {advance} ascent {ascent} descent {descent} scale {scale}")` in `prepaint`), compare with Ghostty's `@round(ascent - descent + line_gap)`, and record the numbers. If the difference is the font's `line_gap` (non-zero for a user-configured font), note it as a known gap with `adjust-cell-height` as the user knob; remove the `eprintln!`.
  - If ours equals Ghostty's times the scale factor (we report device px, Ghostty logical), change `set_geometry` to pass `g.cell_w.round() as u32, g.cell_h.round() as u32` and re-check. At scale 1.0 this check cannot tell the two apart; note the scale you tested.

- [ ] **Step 2: Colours and fonts match Ghostty (exit criterion 2).** In both terminals, at the same cols×rows, run:

```bash
clear; for i in $(seq 0 15); do printf '\e[48;5;%sm %2s \e[0m' $i $i; done; echo
printf '\e[1mbold\e[0m \e[3mitalic\e[0m \e[1;3mbolditalic\e[0m \e[4munder\e[0m \e[2mfaint\e[0m \e[7minverse\e[0m \e[9mstrike\e[0m\n'
for i in $(seq 0 8 255); do printf '\e[48;2;%s;0;%sm ' $i $((255-i)); done; printf '\e[0m\n'
printf 'The quick brown fox -> != <= 0O 1lI |\n'
```

Screenshot each as `ghostty-colors.png` and `cmux-colors.png`. Expected: same palette swatches, background colour and opacity, glyph shapes, weights and line spacing; `->` and `!=` render as separate glyphs in ours (ligatures deferred; Ghostty shows ligatures by default — note it, not a failure).

- [ ] **Step 3: Claude Code (exit criterion 1).** Run `claude` in cmux-linux and use it for a real task for at least 15 minutes. Check: the TUI draws without artifacts; colours and the spinner render; typing a long prompt and Shift+Enter for a newline work; pasting a multi-line snippet goes in as one paste; Esc and Ctrl+C interrupt; the wheel scrolls; the window resize re-lays out the TUI; selecting output and Super+C copies it. Screenshot `cmux-claude.png`.

- [ ] **Step 4: vim, htop, git log (exit criterion 1).**
  - `vim` on a source file with syntax highlighting: insert, arrows, `:set mouse=a` click/scroll/drag, `:q`. Screenshot `cmux-vim.png`.
  - `htop`: colours, F-keys (F6 sort, F10 quit), mouse click on a header, resize while running. Screenshot `cmux-htop.png`.
  - `git log --graph --decorate --color` in this repo: pager scrolls by wheel and keys, `q` quits, colours correct. Screenshot `cmux-gitlog.png`.

- [ ] **Step 5: Typing latency (exit criterion 3).** Run `CMUX_TRACE_LATENCY=1 target/release/cmux-app 2> ~/Pictures/cmux-m1/latency.log`, type ~100 characters at a shell prompt, exit, then:

```bash
grep -o '[0-9.]* ms' ~/Pictures/cmux-m1/latency.log | sort -n | awk '{a[NR]=$1} END {print "n="NR, "p50="a[int(NR/2)+1], "p95="a[int(NR*0.95)], "max="a[NR]}'
```

Expected: p95 under 16.7 ms (one 60 Hz frame) and no visible lag next to Ghostty when typing the same text side by side. If p95 is over, check whether the 4 ms coalescing timer or the full-grid repaint dominates (time `prepaint` with `Instant`) and record the numbers before changing code.

- [ ] **Step 6: Malformed config.** Run against a scratch copy, never the real config:

```bash
SCRATCH="$(mktemp -d)"
cp -r "${XDG_CONFIG_HOME:-$HOME/.config}/ghostty" "$SCRATCH/"
F="$SCRATCH/ghostty/config.ghostty"; [ -f "$F" ] || F="$SCRATCH/ghostty/config"
printf 'font-size = big\ntheme = DoesNotExist\n' >> "$F"
XDG_CONFIG_HOME="$SCRATCH" target/release/cmux-app
rm -rf "$SCRATCH"
```

Expected: two `cmux-linux: config:` warnings on stderr (`font-size: not a number: big`, `theme: not found: DoesNotExist`); the window opens with the default size and no theme colours.

---

### Task 13: Close-out

**Files:**
- Modify: `docs/milestones/m1-terminal/journal.md`, `docs/status-archive.md`, `STATUS.md`, `docs/design.md`

- [ ] **Step 1: Journal entry.** Append to `docs/milestones/m1-terminal/journal.md`, filling every `<...>` from Tasks 1–12:

```markdown
## <YYYY-MM-DD> — M1 complete

- Toolchain pinned to 1.99.0; vendored clippy exclusion kept (kitty.rs chunks_exact_to_as_chunks).
- Config: config.ghostty/config, includes and themes; hex colours only (X11 names warn).
- Term: on_pty_write replies via the writer thread; title/bell/cwd/notify events; C1 OSC via vt_write_with_normalized; FrameInfo + walk_rows replace Snapshot.
- Cell size: ours <WxH> px vs Ghostty <WxH> px at scale <s>; pixel convention <device|logical>; <notes>.
- Wheel: direction <ok|flipped>; <N> lines per notch; alternate scroll in less <ok>.
- IME: <exercised with <engine> | not exercised>; dead keys <ok>.
- Exit 1: Claude Code <result>, vim <result>, htop <result>, git log <result>. Screenshots: cmux-claude.png, cmux-vim.png, cmux-htop.png, cmux-gitlog.png.
- Exit 2: colour/font pair ghostty-colors.png / cmux-colors.png: <match | differences>.
- Exit 3: latency n=<n> p50=<ms> p95=<ms> max=<ms>; side by side with Ghostty: <same | notes>.
- Deviations from Ghostty: ligatures off; Super+key never reaches the PTY; config warnings on stderr; window fill ignores OSC 11. <others found>
```

- [ ] **Step 2: design.md.** In the Architecture table, change the `cmux-term` row's "Depends on" cell to `` `ghostty-vt` (vendored), `portable-pty` ``.

- [ ] **Step 3: Archive and rewrite STATUS.md.** First append the current file to the archive:

```bash
{ printf '\n## %s — M1 complete\n\n' "$(date +%F)"; cat STATUS.md; } >> docs/status-archive.md
```

Then edit `STATUS.md`: Updated date; Current milestone `M2 splits-tabs — not started`; Next action `write docs/milestones/m2-splits-tabs/spec.md`; M1 row `done (PR #<n>)`; under Decisions add one line each: portable-pty for PTYs, bundled JetBrains Mono, ligatures off, Super never reaches the PTY; replace "Carry to M1" with "Carry to M2" listing the deferred items from this plan's "Deferred" section that M2 owns (config surface: keybinding remap, `font-feature`, font-family variants, `bold-color`, `scrollback-limit`, scroll multiplier, in-window config warning) and any open item from Task 12. Run `scripts/check-status-lines.sh`. Expected: ≤100 lines.

- [ ] **Step 4: Commit, PR, merge.**

```bash
git add docs STATUS.md
git commit -m "docs: close M1" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push
gh pr create --base master --head m1-terminal --title "M1 terminal" --body "$(printf 'One usable terminal: PTY, Ghostty config, grid renderer, keyboard, IME, mouse, selection, clipboard, scrollback.\n\nExit criteria and screenshots: docs/milestones/m1-terminal/journal.md.\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)')"
gh pr checks --watch
gh pr merge --merge
```

Expected: CI green, PR merged to `master`. Then put the PR number into STATUS.md's M1 row on master if it was not known before (`git switch master && git pull`, edit, commit `docs: record M1 PR number`, push).

---

## Exit criteria and carry-list check

| Requirement (spec.md / STATUS carry list) | Task |
|---|---|
| PTY spawn of the user's shell; resize on window resize | 5, 9 |
| Grid element: fonts, bold/italic, colours, cursor styles, wide chars | 8, 9 |
| Ligatures if the engine reports them | Deferred (engine does not report them; off by default) |
| Ghostty config: font family/size, theme, palette, background opacity, cursor style (+ background, foreground, cursor/selection colours, adjust-cell-height) | 2, 3, 8 |
| Keyboard incl. kitty protocol | 7, 9 |
| IME | 11 |
| Mouse reporting, selection, copy/paste (Wayland clipboard + PRIMARY) | 6, 9, 10 |
| Scrollback with scroll wheel | 4, 10 |
| Exit 1: Claude Code, vim, htop, git log | 12 |
| Exit 2: colours/fonts match Ghostty, screenshot pair in journal | 12, 13 |
| Exit 3: typing latency | 9 (trace), 12 |
| Carry: on_pty_write replies | 4, 5 |
| Carry: bell and title events | 4 |
| Carry: resize/scroll API | 4 |
| Carry: no `Snapshot` clones / double walk | 4 |
| Carry: frame dims + `Debug` | 4 (`FrameInfo`) |
| Carry: `rust-toolchain.toml` | 1 |
| Carry: `clippy --all-targets` | 1 |
| Carry: 8-bit C1 OSC (0x9d) | 4 |
| Carry: rustfmt not installed | 1 |
| Carry: drop clippy vendored exclusion once clean | 1 (kept: vendored code fails clippy on 1.99) |

## research.md UNVERIFIED items

| Item | Handled in |
|---|---|
| Ghostty cell height vs gpui (no `line_gap`) | Task 8 `cell_size` (Ghostty rounding); Task 12 Step 1 measurement and fallback |
| `descent` sign on Linux | Task 8 `cell_size` uses `abs`; Task 12 Step 1 |
| Logical vs device pixels for `Terminal::resize` | Task 9 passes device px; Task 12 Step 1 compares with Ghostty's CSI 16 t, fallback given |
| Wheel → arrows in the alternate screen (mode 1007) | Resolved from source: the engine does not convert, mode 1007 defaults on (`modes.zig:271`); app does it in Task 10; verified with `less` |
| Wide glyphs and ligatures vs `force_width` | Task 9 batches only narrow single-codepoint cells; ligatures off; Task 9 Step 8 alignment check |
| cosmic-text and variable fonts | Static faces bundled (Task 8) |
| `xterm-ghostty` terminfo | `TERM=xterm-256color` (Task 5); deferred to M6 |
| macOS key behaviour | Deferred to M6 |
| Zed `cursor.rs` not read | Cursor painting written from scratch (Task 9); Task 9 Step 8 checks the four shapes |
| `Send` for `Term`/encoders | Confirmed by compiling (all code here builds with `Arc<Mutex<Term>>` across threads) |
