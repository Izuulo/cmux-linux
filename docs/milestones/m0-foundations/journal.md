# m0-foundations — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-02 — Task 3: vendored libghostty-vt

- Vendored ghostty-vt / ghostty-vt-sys from cmux @ 21dcc2b; Ghostty fork @ ef4f157 (matches cmux's pin). 191 vendored tests pass on Linux x86_64.
- Ghostty's build.zig runs a bare `zig env`, so `ZIG` in `.cargo/config.toml` isn't enough: `.tools/zig` must be on PATH. Documented in CLAUDE.md "Dev setup"; CI adds it to `$GITHUB_PATH` (plan Task 7).

## 2026-11-02 — Task 6: gpui window

- gpui 0.2.2 window opens and is translucent on Plasma 6.8.2 Wayland, but **not blurred**: KWin 6.8 advertises only `ext_background_effect_manager_v1`; gpui 0.2.2 binds only `org_kde_kwin_blur_manager`. Zed main already supports the new protocol. **Decision (user): defer blur to M3** (spec updated).
- Build break: `xattr 0.2.3` (via gpui) uses `libc::ENOATTR`, which libc 0.2.191 removed on linux-gnu. Cargo.lock pins libc 0.2.189. Don't `cargo update` libc until xattr/libc fix it; CI builds with `--locked`.
- Window shows a title bar; server- vs client-side decorations not checked yet (M2/M6 parity).

## 2026-11-02 — Task 7: CI

- CI runs on every push and PR (build, test, clippy, STATUS.md line check). Clippy uses `--no-deps --exclude ghostty-vt --exclude ghostty-vt-sys`: vendored `ghostty-vt/src/kitty.rs` trips `clippy::chunks_exact_to_as_chunks` on current stable, and `--exclude` alone doesn't help because the crate still gets linted as a dependency, hence `--no-deps`. Drop both once the vendored crates are clean.

## 2026-11-02 — M0 complete

- Zig 0.16.0 via scripts/install-zig.sh; Arch's 0.17 is too new for Ghostty.
- Vendored ghostty-vt from cmux @ 21dcc2b27b2ed014d723067ed253702e914d97ca; Ghostty fork @ ef4f157. Vendored tests: 191/191 pass on Linux x86_64 (see Task 3).
- libghostty-vt wrapper callbacks cover PTY write, title, bell only; OSC 7/9/777 handled by cmux-term::osc.
- Spike test (`crates/cmux-term/tests/spike.rs`): passed unmodified. Text lines, bold + `Palette(2)` foreground, cursor at (2,1) and OSC 2 title come from libghostty-vt; OSC 7/777 events come from our scanner.
- KWin blur: translucent but not blurred on Plasma 6.8.2 Wayland (protocol mismatch, deferred to M3; see Task 6).
- CI: https://github.com/finneritter/cmux-linux/actions/runs/41566208715 (green on first run). Extra apt packages needed: none beyond the workflow's list. Clippy scoping and libc pin: see Tasks 6 and 7.
- **Engine decision: libghostty-vt**, because the vendored tests (191/191) and the spike test passed on cells, styles, cursor and title, so no fallback to `alacritty_terminal` is needed.
