# M0 foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove the toolchain and the two risky choices (gpui blur on KDE Wayland, libghostty-vt bindings) with a buildable workspace, a passing terminal spike test, an empty blurred window, and green CI.

**Architecture:** One Cargo workspace: six crates under `crates/` (see `docs/design.md`), cmux's `ghostty-vt` and `ghostty-vt-sys` crates vendored unmodified under `vendor/` (except their `license` field), and Ghostty as a git submodule at `vendor/ghostty`. `ghostty-vt-sys` builds `libghostty-vt.a` with Zig 0.16.0, which a repo script installs into `.tools/zig`. `cmux-term` wraps `ghostty_vt::Terminal` and adds our own OSC scanner for OSC 7/9/777, because the wrapper's callbacks only cover PTY writes, title and bell.

**Tech Stack:** Rust 2024 edition (local toolchain 1.99), gpui `=0.2.2`, libghostty-vt (manaflow-ai/ghostty fork @ `ef4f1570d62117ebc51e3b0f07ad4f34032267b6`), Zig 0.16.0, bindgen 0.72 (needs libclang), GitHub Actions.

## Global Constraints

- License GPL-3.0-or-later for our crates. Vendored cmux crates stay `MIT` (their Cargo metadata), Ghostty is MIT. Every vendored component goes in `NOTICE`.
- Never copy from cmux's BUSL-1.1 directories (`web/`, `workers/ci-artifacts/`, `workers/iroh-v2/`, `workers/presence/`, `services/iroh-relay-minter/`, `cmux-tui/relays/cloudflare-do/`).
- Zig must be exactly 0.16.0 (Ghostty's `minimum_zig_version`). Arch ships 0.17, so use `scripts/install-zig.sh`.
- Ghostty's build runs a bare `zig`, so every cargo command needs `export PATH="$PWD/.tools/zig:$PATH"` (repo root) first.
- gpui pinned with `=0.2.2`.
- No personal info (names, emails, home paths, machine details) in tracked files.
- `STATUS.md` max 100 lines; archive it to `docs/status-archive.md` before replacing it.
- Default branch is `master`; M0 work happens on branch `m0-foundations` and merges at the end.

## File structure

| Path | Responsibility |
|---|---|
| `Cargo.toml` | Workspace: members, shared package metadata, shared dependency versions |
| `.cargo/config.toml` | Points the Ghostty build at `vendor/ghostty` and `.tools/zig/zig` |
| `scripts/install-zig.sh` | Installs pinned Zig into `.tools/zig` with a checksum check |
| `scripts/check-status-lines.sh` | Fails if `STATUS.md` (or a given file) exceeds 100 lines |
| `crates/cmux-{core,meta,config,ipc}/` | Empty library stubs (filled in later milestones) |
| `crates/cmux-term/src/osc.rs` | `OscScanner`: stateful OSC 7/9/777 scanner over raw PTY bytes |
| `crates/cmux-term/src/lib.rs` | `Term`: `ghostty_vt::Terminal` + `OscScanner`; `Snapshot` |
| `crates/cmux-term/tests/spike.rs` | M0 exit test: bytes in, cells/styles/cursor/events out |
| `crates/cmux-app/src/main.rs` | Opens an empty translucent, blurred gpui window |
| `vendor/ghostty-vt`, `vendor/ghostty-vt-sys` | Vendored cmux crates |
| `vendor/ghostty` | Git submodule (manaflow-ai/ghostty fork) |
| `vendor/SOURCE` | Exact upstream commits and the one local edit |
| `.github/workflows/ci.yml` | Build, test, clippy, STATUS line check |

---

### Task 1: Workspace skeleton

**Files:**
- Create: `Cargo.toml`, `crates/cmux-core/Cargo.toml`, `crates/cmux-core/src/lib.rs`, same pair for `cmux-meta`, `cmux-config`, `cmux-ipc`, `cmux-term`; `crates/cmux-app/Cargo.toml`, `crates/cmux-app/src/main.rs`
- Modify: `.gitignore`

**Interfaces:**
- Produces: workspace with `[workspace.dependencies]` entries later tasks reference by `.workspace = true`; crate names `cmux-core`, `cmux-term`, `cmux-meta`, `cmux-config`, `cmux-ipc`, `cmux-app`.

- [ ] **Step 1: Write the workspace manifest** `Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
edition = "2024"
rust-version = "1.91"
license = "GPL-3.0-or-later"
publish = false

[workspace.lints.rust]
[workspace.lints.clippy]

[workspace.dependencies]
```

- [ ] **Step 2: Create the five library stubs.** For each of `cmux-core`, `cmux-meta`, `cmux-config`, `cmux-ipc`, `cmux-term`, write `crates/<name>/Cargo.toml`:

```toml
[package]
name = "<name>"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true

[dependencies]
```

and `crates/<name>/src/lib.rs` containing one line, e.g. for cmux-core:

```rust
//! cmux-core: workspace, split tree, pane, tab and notification model (filled in from M2).
```

Use these doc lines: cmux-meta `//! cmux-meta: sidebar metadata collectors (filled in M3).`, cmux-config `//! cmux-config: Ghostty config and keybindings (filled in M1/M2).`, cmux-ipc `//! cmux-ipc: socket server and cmux CLI (filled in M5).`, cmux-term `//! cmux-term: PTY + libghostty-vt wrapper.`

- [ ] **Step 3: Create the app stub.** `crates/cmux-app/Cargo.toml`: same as above with `name = "cmux-app"`. `crates/cmux-app/src/main.rs`:

```rust
fn main() {}
```

- [ ] **Step 4: Ignore local tools.** Append to `.gitignore`:

```
# Pinned toolchains installed by scripts/
/.tools/
```

- [ ] **Step 5: Build.** Run: `cargo build --workspace`. Expected: `Finished` with 6 crates compiled, no warnings.

- [ ] **Step 6: Commit.**

```bash
git add Cargo.toml Cargo.lock crates .gitignore
git commit -m "build: add Cargo workspace with six crate stubs"
```

---

### Task 2: Pinned Zig installer

**Files:**
- Create: `scripts/install-zig.sh`, `.cargo/config.toml`

**Interfaces:**
- Produces: `.tools/zig/zig` (Zig 0.16.0); env `ZIG` set for every cargo build via `.cargo/config.toml` (an already-set `ZIG` env var wins).

- [ ] **Step 1: Write `scripts/install-zig.sh`** (and `chmod +x` it):

```bash
#!/usr/bin/env bash
# Installs the exact Zig version Ghostty needs into .tools/zig (gitignored).
set -euo pipefail

VERSION=0.16.0
# shortcut: Linux x86_64 only, add macOS/aarch64 rows from ziglang.org/download/index.json in M6.
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    ARCH=x86_64-linux
    SHA=70e49664a74374b48b51e6f3fdfbf437f6395d42509050588bd49abe52ba3d00
    ;;
  *)
    echo "unsupported platform $(uname -s)-$(uname -m): add its tarball and sha256 from https://ziglang.org/download/index.json" >&2
    exit 1
    ;;
esac

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/.tools/zig"

if [[ -x "$DEST/zig" && "$("$DEST/zig" version)" == "$VERSION" ]]; then
  echo "zig $VERSION already installed at .tools/zig"
  exit 0
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
curl -fsSL "https://ziglang.org/download/$VERSION/zig-$ARCH-$VERSION.tar.xz" -o "$TMP/zig.tar.xz"
echo "$SHA  $TMP/zig.tar.xz" | sha256sum -c -
tar -xJf "$TMP/zig.tar.xz" -C "$TMP"
mkdir -p "$ROOT/.tools"
rm -rf "$DEST"
mv "$TMP/zig-$ARCH-$VERSION" "$DEST"
"$DEST/zig" version
```

- [ ] **Step 2: Run it.** Run: `scripts/install-zig.sh`. Expected: `zig.tar.xz: OK` then `0.16.0`. Run it again. Expected: `zig 0.16.0 already installed at .tools/zig`.

- [ ] **Step 3: Point cargo at it.** Write `.cargo/config.toml`:

```toml
[env]
# Used by vendor/ghostty-vt-sys/build.rs. An exported ZIG / CMUX_GHOSTTY_SRC overrides these.
ZIG = { value = ".tools/zig/zig", relative = true }
CMUX_GHOSTTY_SRC = { value = "vendor/ghostty", relative = true }
```

- [ ] **Step 4: Verify the workspace still builds.** Run: `cargo build --workspace`. Expected: `Finished`.

- [ ] **Step 5: Commit.**

```bash
git add scripts/install-zig.sh .cargo/config.toml
git commit -m "build: pin Zig 0.16.0 via install script and cargo env"
```

---

### Task 3: Vendor libghostty-vt bindings

**Files:**
- Create: `vendor/ghostty` (submodule), `vendor/ghostty-vt/**`, `vendor/ghostty-vt-sys/**`, `vendor/SOURCE`
- Modify: `Cargo.toml` (members, dependencies), `NOTICE`

**Interfaces:**
- Consumes: `.cargo/config.toml` env from Task 2.
- Produces: workspace dependency `ghostty-vt` exposing `Terminal`, `Callbacks`, `RenderState`, `Cell`, `CursorInfo`, `ColorSpec`, `Error` (see the crate's `src/lib.rs`).

- [ ] **Step 1: Add the Ghostty submodule at the commit cmux pins.**

```bash
git submodule add https://github.com/manaflow-ai/ghostty.git vendor/ghostty
git -C vendor/ghostty checkout ef4f1570d62117ebc51e3b0f07ad4f34032267b6
```

Expected: `HEAD is now at ef4f157...`.

- [ ] **Step 2: Copy the two crates from cmux.** Use the scratchpad for the clone, not the repo:

```bash
SRC="$(mktemp -d)"
git clone --filter=blob:none --sparse https://github.com/manaflow-ai/cmux "$SRC/cmux"
git -C "$SRC/cmux" sparse-checkout set cmux-tui/crates/ghostty-vt cmux-tui/crates/ghostty-vt-sys
CMUX_SHA="$(git -C "$SRC/cmux" rev-parse HEAD)"; echo "$CMUX_SHA"
git -C "$SRC/cmux" ls-tree HEAD ghostty   # must print ef4f1570d6...; if not, check out that cmux commit's pin instead
cp -r "$SRC/cmux/cmux-tui/crates/ghostty-vt" "$SRC/cmux/cmux-tui/crates/ghostty-vt-sys" vendor/
```

If `ls-tree` shows a different Ghostty commit than Step 1, re-run Step 1's checkout with the commit it shows, so bindings and headers match.

- [ ] **Step 3: Keep the upstream licence on the vendored crates.** In both `vendor/ghostty-vt/Cargo.toml` and `vendor/ghostty-vt-sys/Cargo.toml` replace `license.workspace = true` with `license = "MIT"`. This is the only edit to vendored files.

- [ ] **Step 4: Record provenance.** Write `vendor/SOURCE` (substitute the real `$CMUX_SHA`):

```
ghostty-vt, ghostty-vt-sys
  from https://github.com/manaflow-ai/cmux/tree/<CMUX_SHA>/cmux-tui/crates
  licence: MIT (cmux-tui Cargo metadata)
  local edit: Cargo.toml `license.workspace = true` -> `license = "MIT"`

ghostty (git submodule)
  https://github.com/manaflow-ai/ghostty @ ef4f1570d62117ebc51e3b0f07ad4f34032267b6
  licence: MIT (Mitchell Hashimoto, Ghostty contributors)
```

- [ ] **Step 5: Wire them into the workspace.** In `Cargo.toml` set `members = ["crates/*", "vendor/ghostty-vt", "vendor/ghostty-vt-sys"]` and fill `[workspace.dependencies]`:

```toml
[workspace.dependencies]
base64 = "0.22"
png = "0.17"
bindgen = "0.72"
ghostty-vt-sys = { path = "vendor/ghostty-vt-sys" }
ghostty-vt = { path = "vendor/ghostty-vt" }
```

- [ ] **Step 6: Run the vendored tests.** Run: `cargo test -p ghostty-vt`. Expected: Zig builds `libghostty-vt` (first build takes minutes), then all tests pass. If a test file fails because it references files outside the crate, delete that one test file, note it in `vendor/SOURCE` under "local edit", and re-run. Any other failure: stop and record it in the M0 journal (it's evidence for the engine decision in Task 8).

- [ ] **Step 7: Add to NOTICE.** Replace the last paragraph of `NOTICE` ("Further components...") with:

```
libghostty-vt Rust bindings (vendor/ghostty-vt, vendor/ghostty-vt-sys)
  From cmux's cmux-tui crates. MIT. Provenance in vendor/SOURCE.

Ghostty (vendor/ghostty, git submodule of github.com/manaflow-ai/ghostty)
  Copyright (c) 2024 Mitchell Hashimoto, Ghostty contributors. MIT.

Further components (gpui, Rust crates) are listed here as they are added,
with their licences.
```

- [ ] **Step 8: Commit.**

```bash
git add .gitmodules vendor Cargo.toml Cargo.lock NOTICE
git commit -m "vendor: add libghostty-vt bindings from cmux and ghostty submodule"
```

---

### Task 4: OSC scanner for cwd and notifications

**Files:**
- Create: `crates/cmux-term/src/osc.rs`
- Modify: `crates/cmux-term/src/lib.rs`

**Interfaces:**
- Produces: `cmux_term::osc::{OscScanner, OscEvent}`; `OscScanner::new() -> OscScanner`; `OscScanner::feed(&mut self, bytes: &[u8]) -> Vec<OscEvent>`; `enum OscEvent { Cwd(String), Notify { title: Option<String>, body: String } }`.

- [ ] **Step 1: Write the failing tests.** Create `crates/cmux-term/src/osc.rs` with only the tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn scan(bytes: &[u8]) -> Vec<OscEvent> {
        OscScanner::new().feed(bytes)
    }

    #[test]
    fn osc7_cwd_with_bel_and_percent_encoding() {
        assert_eq!(
            scan(b"\x1b]7;file://host/tmp/my%20dir\x07"),
            vec![OscEvent::Cwd("/tmp/my dir".into())]
        );
    }

    #[test]
    fn osc9_notification_with_st() {
        assert_eq!(
            scan(b"\x1b]9;Build done\x1b\\"),
            vec![OscEvent::Notify { title: None, body: "Build done".into() }]
        );
    }

    #[test]
    fn osc9_progress_is_ignored() {
        assert_eq!(scan(b"\x1b]9;4;1;50\x07"), vec![]);
    }

    #[test]
    fn osc777_notify_title_and_body() {
        assert_eq!(
            scan(b"\x1b]777;notify;Claude;Waiting for input\x07"),
            vec![OscEvent::Notify { title: Some("Claude".into()), body: "Waiting for input".into() }]
        );
    }

    #[test]
    fn sequence_split_across_feeds() {
        let mut s = OscScanner::new();
        assert_eq!(s.feed(b"text \x1b]9;hel"), vec![]);
        assert_eq!(s.feed(b"lo\x1b"), vec![]);
        assert_eq!(s.feed(b"\\more"), vec![OscEvent::Notify { title: None, body: "hello".into() }]);
    }

    #[test]
    fn oversized_payload_is_dropped() {
        let mut bytes = b"\x1b]9;".to_vec();
        bytes.extend(std::iter::repeat_n(b'a', MAX_PAYLOAD + 1));
        bytes.push(0x07);
        assert_eq!(scan(&bytes), vec![]);
    }

    #[test]
    fn plain_text_and_other_oscs_yield_nothing() {
        assert_eq!(scan(b"hello \x1b[1mbold\x1b[0m \x1b]2;title\x07"), vec![]);
    }
}
```

Add `pub mod osc;` to `crates/cmux-term/src/lib.rs` (below the doc line).

- [ ] **Step 2: Run tests to verify they fail.** Run: `cargo test -p cmux-term`. Expected: compile errors `cannot find type OscScanner` / `OscEvent` / `MAX_PAYLOAD`.

- [ ] **Step 3: Implement.** Add above the tests module in `osc.rs`:

```rust
//! Scans raw PTY bytes for OSCs libghostty-vt does not report: 7 (cwd), 9 and 777 (notifications).
//! Bytes still go to libghostty-vt unchanged; this only observes them.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscEvent {
    Cwd(String),
    Notify { title: Option<String>, body: String },
}

/// Longer OSC payloads are dropped, so a runaway sequence can't grow memory.
pub(crate) const MAX_PAYLOAD: usize = 4096;

#[derive(Default)]
enum State {
    #[default]
    Ground,
    Esc,
    Osc,
    OscEsc,
}

#[derive(Default)]
pub struct OscScanner {
    state: State,
    buf: Vec<u8>,
    overflow: bool,
}

impl OscScanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<OscEvent> {
        let mut out = Vec::new();
        for &b in bytes {
            self.state = match (std::mem::take(&mut self.state), b) {
                (State::Ground, 0x1b) => State::Esc,
                (State::Ground, _) => State::Ground,
                (State::Esc, b']') => self.start(),
                (State::Esc, 0x1b) => State::Esc,
                (State::Esc, _) => State::Ground,
                (State::Osc, 0x07) => self.finish(&mut out),
                (State::Osc, 0x1b) => State::OscEsc,
                // CAN / SUB abort the sequence.
                (State::Osc, 0x18 | 0x1a) => State::Ground,
                (State::Osc, b) => {
                    self.push(b);
                    State::Osc
                }
                (State::OscEsc, b'\\') => self.finish(&mut out),
                (State::OscEsc, b']') => self.start(),
                (State::OscEsc, _) => State::Ground,
            };
        }
        out
    }

    fn start(&mut self) -> State {
        self.buf.clear();
        self.overflow = false;
        State::Osc
    }

    fn push(&mut self, b: u8) {
        if self.buf.len() == MAX_PAYLOAD {
            self.overflow = true;
        } else {
            self.buf.push(b);
        }
    }

    fn finish(&mut self, out: &mut Vec<OscEvent>) -> State {
        if !self.overflow
            && let Ok(s) = std::str::from_utf8(&self.buf)
            && let Some(ev) = parse(s)
        {
            out.push(ev);
        }
        State::Ground
    }
}

fn parse(payload: &str) -> Option<OscEvent> {
    let (code, rest) = payload.split_once(';')?;
    match code {
        "7" => parse_cwd(rest).map(OscEvent::Cwd),
        // shortcut: only ConEmu's 9;4 progress is filtered; other ConEmu 9;N subcommands read as text. Revisit in M4.
        "9" if !rest.is_empty() && !rest.starts_with("4;") => {
            Some(OscEvent::Notify { title: None, body: rest.to_string() })
        }
        "777" => {
            let mut parts = rest.splitn(3, ';');
            if parts.next()? != "notify" {
                return None;
            }
            let title = parts.next()?.to_string();
            let body = parts.next().unwrap_or("").to_string();
            Some(OscEvent::Notify { title: Some(title), body })
        }
        _ => None,
    }
}

/// `file://host/path` -> `/path`, percent-decoded.
fn parse_cwd(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    percent_decode(&rest[rest.find('/')?..])
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            out.push(u8::from_str_radix(s.get(i + 1..i + 3)?, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}
```

- [ ] **Step 4: Run tests to verify they pass.** Run: `cargo test -p cmux-term`. Expected: `test result: ok. 7 passed`.

- [ ] **Step 5: Commit.**

```bash
git add crates/cmux-term
git commit -m "feat(cmux-term): scan PTY bytes for OSC 7/9/777"
```

---

### Task 5: Terminal wrapper and spike test

**Files:**
- Modify: `crates/cmux-term/Cargo.toml`, `crates/cmux-term/src/lib.rs`
- Test: `crates/cmux-term/tests/spike.rs`

**Interfaces:**
- Consumes: `ghostty_vt::{Terminal, Callbacks, RenderState, Cell, CursorInfo, Error}` (Task 3); `OscScanner`, `OscEvent` (Task 4).
- Produces: `cmux_term::Term` with `Term::new(cols: u16, rows: u16) -> Result<Term, ghostty_vt::Error>`, `feed(&mut self, bytes: &[u8]) -> Vec<OscEvent>`, `title(&self) -> Option<String>`, `snapshot(&mut self) -> Result<Snapshot, ghostty_vt::Error>`; `cmux_term::Snapshot { lines: Vec<String>, rows: Vec<Vec<Cell>>, cursor: Option<CursorInfo> }`; re-export `pub use ghostty_vt;`.

- [ ] **Step 1: Add the dependency.** In `crates/cmux-term/Cargo.toml` under `[dependencies]`:

```toml
ghostty-vt.workspace = true
```

- [ ] **Step 2: Write the failing spike test** `crates/cmux-term/tests/spike.rs`:

```rust
//! M0 exit test: a recorded byte stream in; text, styles, cursor and OSC events out.
use cmux_term::{Term, ghostty_vt::ColorSpec, osc::OscEvent};

const STREAM: &[u8] = b"\x1b]2;spike\x07\
\x1b]7;file://host/tmp/my%20dir\x07\
hello \x1b[1;32mworld\x1b[0m\r\n\
\x1b]777;notify;Claude;Waiting\x07ab";

#[test]
fn bytes_to_cells_styles_cursor_and_events() {
    let mut term = Term::new(40, 5).unwrap();
    let events = term.feed(STREAM);

    assert_eq!(
        events,
        vec![
            OscEvent::Cwd("/tmp/my dir".into()),
            OscEvent::Notify { title: Some("Claude".into()), body: "Waiting".into() },
        ]
    );
    assert_eq!(term.title().as_deref(), Some("spike"));

    let snap = term.snapshot().unwrap();
    assert_eq!(snap.lines[0], "hello world");
    assert_eq!(snap.lines[1], "ab");

    let w = snap.rows[0].iter().find(|c| c.text == "w").expect("cell for 'w'");
    assert!(w.bold);
    assert!(matches!(w.fg, ColorSpec::Palette(2)), "fg was {:?}", w.fg);
    let h = snap.rows[0].iter().find(|c| c.text == "h").expect("cell for 'h'");
    assert!(!h.bold);

    let cursor = snap.cursor.expect("cursor visible");
    assert_eq!((cursor.x, cursor.y), (2, 1));
}
```

- [ ] **Step 3: Run it to verify it fails.** Run: `cargo test -p cmux-term --test spike`. Expected: compile error `unresolved import cmux_term::Term`.

- [ ] **Step 4: Implement `Term`.** Replace `crates/cmux-term/src/lib.rs` with:

```rust
//! cmux-term: PTY + libghostty-vt wrapper.

pub mod osc;

pub use ghostty_vt;

use ghostty_vt::{Callbacks, Cell, CursorInfo, Error, RenderState, Terminal};
use osc::{OscEvent, OscScanner};

/// Scrollback lines kept per terminal. Configurable from M1.
const SCROLLBACK: usize = 10_000;

pub struct Term {
    vt: Terminal,
    render: RenderState,
    osc: OscScanner,
}

/// What the renderer needs from one frame.
pub struct Snapshot {
    pub lines: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
    pub cursor: Option<CursorInfo>,
}

impl Term {
    pub fn new(cols: u16, rows: u16) -> Result<Self, Error> {
        Ok(Self {
            vt: Terminal::new(cols, rows, SCROLLBACK, Callbacks::default())?,
            render: RenderState::new()?,
            osc: OscScanner::new(),
        })
    }

    /// Feeds PTY output. Returns the cwd/notification OSCs it contained.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<OscEvent> {
        let events = self.osc.feed(bytes);
        self.vt.vt_write(bytes);
        events
    }

    pub fn title(&self) -> Option<String> {
        self.vt.title()
    }

    pub fn snapshot(&mut self) -> Result<Snapshot, Error> {
        self.render.update(&mut self.vt)?;
        let lines = self.render.text_lines()?;
        let cursor = self.render.cursor();
        let mut rows = Vec::new();
        self.render.walk_rows(|_, _, cells| rows.push(cells.to_vec()))?;
        Ok(Snapshot { lines, rows, cursor })
    }
}
```

- [ ] **Step 5: Run all cmux-term tests.** Run: `cargo test -p cmux-term`. Expected: 7 unit tests and 1 spike test pass. If the spike fails on cells, styles or cursor (not on our OSC events), copy the exact failure into the M0 journal: that is the evidence for Task 8.

- [ ] **Step 6: Commit.**

```bash
git add crates/cmux-term Cargo.lock
git commit -m "feat(cmux-term): wrap libghostty-vt with OSC events; add M0 spike test"
```

---

### Task 6: Empty blurred gpui window

**Files:**
- Modify: `Cargo.toml` (workspace dependency), `crates/cmux-app/Cargo.toml`, `crates/cmux-app/src/main.rs`, `NOTICE`

**Interfaces:**
- Produces: `cargo run -p cmux-app` opens one window, Wayland app id `cmux-linux`.

- [ ] **Step 1: Add gpui.** In root `Cargo.toml` `[workspace.dependencies]` add `gpui = "=0.2.2"`. In `crates/cmux-app/Cargo.toml` `[dependencies]` add `gpui.workspace = true`.

- [ ] **Step 2: Write the window.** Replace `crates/cmux-app/src/main.rs`:

```rust
use gpui::{
    App, Application, Bounds, Context, Window, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, div, prelude::*, px, rgba, size,
};

struct Root;

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Translucent fill so KWin's blur behind the window is visible.
        div().size_full().bg(rgba(0x1e1e2eb3))
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.), px(700.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background: WindowBackgroundAppearance::Blurred,
                app_id: Some("cmux-linux".into()),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Root),
        )
        .expect("open window");
        cx.activate(true);
    });
}
```

- [ ] **Step 3: Build.** Run: `cargo build -p cmux-app`. Expected: `Finished`. If linking fails on a missing system library, install the Arch package that provides it (`pacman -F <libname>.so`), and list it in the M0 journal for the CI step.

- [ ] **Step 4: Verify blur on KDE Plasma Wayland.** Put a busy window (e.g. a browser) behind where the app opens, then run:

```bash
cargo run -p cmux-app &
sleep 3
spectacle -b -n -a -o "${TMPDIR:-/tmp}/m0-blur.png"
kill %1
```

Open `${TMPDIR:-/tmp}/m0-blur.png`. Expected: dark translucent window with the content behind it blurred (not sharp, not opaque). If it's sharp, check System Settings → Desktop Effects → Blur is enabled, and retry. Record the result (blurred / sharp / opaque) in the journal. Don't commit the screenshot (it may show personal content).

- [ ] **Step 5: Add gpui to NOTICE**, above the "Further components" line:

```
gpui (https://github.com/zed-industries/zed, crates.io gpui 0.2.2)
  Copyright Zed Industries. Apache-2.0.
```

- [ ] **Step 6: Commit.**

```bash
git add Cargo.toml Cargo.lock crates/cmux-app NOTICE
git commit -m "feat(cmux-app): open empty blurred gpui window"
```

---

### Task 7: CI with STATUS line check

**Files:**
- Create: `scripts/check-status-lines.sh`, `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `scripts/install-zig.sh` (Task 2).
- Produces: `scripts/check-status-lines.sh [file]` exits 0 when ≤100 lines, 1 otherwise.

- [ ] **Step 1: Write `scripts/check-status-lines.sh`** (`chmod +x`):

```bash
#!/usr/bin/env bash
# STATUS.md is the permanent handoff and must stay at most 100 lines.
set -euo pipefail
FILE="${1:-$(dirname "$0")/../STATUS.md}"
n=$(wc -l < "$FILE")
if (( n > 100 )); then
  echo "$FILE has $n lines (max 100). Archive detail to docs/status-archive.md." >&2
  exit 1
fi
echo "$FILE: $n lines"
```

- [ ] **Step 2: Test both outcomes.**

```bash
scripts/check-status-lines.sh; echo "exit=$?"
seq 101 > "${TMPDIR:-/tmp}/long.md"; scripts/check-status-lines.sh "${TMPDIR:-/tmp}/long.md"; echo "exit=$?"
```

Expected: first prints the line count and `exit=0`; second prints `has 101 lines (max 100)` and `exit=1`.

- [ ] **Step 3: Write `.github/workflows/ci.yml`:**

```yaml
name: CI
on:
  push:
  pull_request:

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
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - name: Zig 0.16.0
        # Ghostty's build.zig runs a bare `zig env`, so zig must be on PATH (ZIG alone isn't enough).
        run: |
          scripts/install-zig.sh
          echo "$PWD/.tools/zig" >> "$GITHUB_PATH"
      - run: cargo build --workspace --locked
      - run: cargo test --workspace --locked
      - run: cargo clippy --workspace --locked -- -D warnings
```

- [ ] **Step 4: Run the same commands locally first.** Run: `cargo test --workspace --locked && cargo clippy --workspace --locked -- -D warnings`. Expected: tests pass, clippy clean. Fix any clippy findings in our crates (not vendored ones; if vendored code fails clippy, add `--exclude ghostty-vt --exclude ghostty-vt-sys` to the clippy line and note why in the journal).

- [ ] **Step 5: Commit, push, check CI.**

```bash
git add scripts/check-status-lines.sh .github/workflows/ci.yml
git commit -m "ci: build, test, clippy and STATUS.md line check"
git push -u origin HEAD
gh run watch --exit-status "$(gh run list --branch "$(git branch --show-current)" --limit 1 --json databaseId --jq '.[0].databaseId')"
```

Expected: run succeeds. If a system library is missing on Ubuntu, add its `-dev` package to the apt line and push again; record it in the journal.

---

### Task 8: Engine decision and M0 close-out

**Files:**
- Modify: `docs/milestones/m0-foundations/journal.md`, `docs/design.md`, `STATUS.md`, `docs/status-archive.md`

- [ ] **Step 1: Decide.** Keep libghostty-vt if Task 3's vendored tests and Task 5's spike passed on cells, styles and cursor. Switch to `alacritty_terminal` only if libghostty-vt could not build or could not report those. (OSC 7/9/777 come from our scanner either way, so they don't affect the decision.)

- [ ] **Step 2: Journal entry.** Append to `docs/milestones/m0-foundations/journal.md`:

```markdown
## <YYYY-MM-DD> — M0 complete

- Zig 0.16.0 via scripts/install-zig.sh; Arch's 0.17 is too new for Ghostty.
- Vendored ghostty-vt from cmux @ <CMUX_SHA>; Ghostty fork @ ef4f157. Vendored tests: <pass/fail details>.
- libghostty-vt wrapper callbacks cover PTY write, title, bell only; OSC 7/9/777 handled by cmux-term::osc.
- Spike test: <pass/fail details>.
- KWin blur: <blurred/sharp/opaque> on Plasma <version> Wayland.
- CI: <link to green run>. Extra apt packages needed: <list or none>.
- **Engine decision: <libghostty-vt | alacritty_terminal>**, because <one line of evidence>.
```

- [ ] **Step 3: Record the decision in `docs/design.md`.** In the Key decisions table replace the Terminal engine row's Choice cell with `**libghostty-vt** via vendored cmux crates (decided M0, see journal)` (or the alacritty equivalent), and drop the fallback sentence.

- [ ] **Step 4: Update STATUS.md.** First append the current file to the archive:

```bash
{ printf '\n## %s — M0 complete\n\n' "$(date +%F)"; cat STATUS.md; } >> docs/status-archive.md
```

Then edit `STATUS.md`: Updated date; Current milestone `M1 terminal — not started`; Next action `write docs/milestones/m1-terminal/plan.md`; M0 row `done`; replace the libghostty-vt risk and blur risk with the outcomes; add the engine decision under Decisions. Run `scripts/check-status-lines.sh`. Expected: ≤100 lines.

- [ ] **Step 5: Commit and push.**

```bash
git add docs STATUS.md
git commit -m "docs: close M0, record engine decision"
git push
```

---

## Exit criteria check (from spec.md)

| Spec exit criterion | Task |
|---|---|
| 1. `cargo run -p cmux-app` opens a blurred empty window on KDE Plasma Wayland | 6 |
| 2. `cargo test -p cmux-term` passes the byte-stream spike test | 4, 5 |
| 3. Engine decision recorded | 8 |
| 4. CI green on the default branch | 7 |
| Scope: six crates, vendored bindings + `vendor/SOURCE`, pinned gpui, STATUS line check | 1, 3, 6, 7 |
