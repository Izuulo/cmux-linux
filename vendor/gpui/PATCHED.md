# gpui 0.2.2 (patched)

Source: crates.io `gpui` 0.2.2 (Zed Industries, Apache-2.0, see `LICENSE-APACHE`).
`examples/` and `tests/` left out; their `[[example]]`/`[[test]]` tables removed from
`Cargo.toml`.

Change: `src/window.rs` `DispatchEventResult` is `pub` instead of `pub(crate)`, so
`Window::dispatch_event` can be called from outside gpui (cmux-linux M15 scripted clicks).
Upgrading gpui: re-copy and re-apply this struct (plus doc comments, which gpui's `missing_docs` lint requires).

Change 2 (debug builds only, M15 click-through): `Window::insert_inspector_hitbox` records
every frame, not only while the inspector is picking, and the new
`Window::debug_element_bounds()` returns each id'd element's `GlobalElementId` path and
visible bounds from the last drawn frame.
