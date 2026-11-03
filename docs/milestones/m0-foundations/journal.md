# m0-foundations — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-02 — Task 3: vendored libghostty-vt

- Vendored ghostty-vt / ghostty-vt-sys from cmux @ 21dcc2b; Ghostty fork @ ef4f157 (matches cmux's pin). 191 vendored tests pass on Linux x86_64.
- Ghostty's build.zig runs a bare `zig env`, so `ZIG` in `.cargo/config.toml` isn't enough: `.tools/zig` must be on PATH. Documented in CLAUDE.md "Dev setup"; CI adds it to `$GITHUB_PATH` (plan Task 7).
