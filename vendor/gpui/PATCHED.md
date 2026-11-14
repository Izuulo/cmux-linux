# gpui 0.2.2 (patched)

Source: crates.io `gpui` 0.2.2 (Zed Industries, Apache-2.0, see `LICENSE-APACHE`).
`examples/` and `tests/` left out; their `[[example]]`/`[[test]]` tables removed from
`Cargo.toml`.

Change: `src/window.rs` `DispatchEventResult` is `pub` instead of `pub(crate)`, so
`Window::dispatch_event` can be called from outside gpui (cmux-linux M15 scripted clicks).
Upgrading gpui: re-copy and re-apply this struct (plus doc comments, which gpui's `missing_docs` lint requires).
