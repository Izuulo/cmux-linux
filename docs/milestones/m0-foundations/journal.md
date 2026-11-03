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
