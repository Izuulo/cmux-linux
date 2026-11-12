# M13 browser — Plan

**Goal:** a browser surface in panes and tabs, opened by shortcut, link click and socket, restored with the layout. Starts with a spike that decides whether the engine works at all.

Branch `m13-browser`, cut from `master`. Engine research (2026-11-11, journal): gpui 0.2.2 can't host a native child view on Wayland (a subsurface must come from gpui's own Wayland connection; `surface()` is macOS-only, `blade_renderer.rs:816`), so pages are rendered off screen and drawn as images. Reference for behaviour: cmux @ `88b87c05` `Sources/Panels/BrowserPanel*.swift` (chrome bar, title, link routing), its `browser` CLI subcommands (`open` only).

## Engine

| | Decision | Why |
|---|---|---|
| E1 | CEF in windowless (off-screen) mode through the `cef` crate (tauri-apps/cef-rs, MIT/Apache; CEF BSD-3, Chromium's third-party licences shipped as CEF's `LICENSE`/`CREDITS.html`). Dev builds fetch CEF with the crate's downloader into `.tools/cef` (git-ignored, like `.tools/zig`) through `scripts/install-cef.sh`; the Arch package depends on `cef` and builds with `CEF_PATH=/usr/lib/cef` if its version matches the crate, else bundles the download. | Only option that draws inside a pane on Wayland and also works on macOS. WPE WebKit needs hand-made bindings and is Linux-only; WebKitGTK and wry can only be separate windows; Servo isn't embeddable yet. |
| E2 | Behind a cargo feature `browser`, **off by default** (libcef is a link-time dependency of 380 MB stripped; CI and macOS build without it); `scripts/install-cef.sh` fetches CEF 156.0.26 for `cef =156.4.0` into `.tools/cef`, `build.rs` adds an `$ORIGIN` rpath, the Arch package builds with the feature and ships a stripped `libcef.so` and CEF's resources next to the binary. Without the feature the browser actions show "Built without browser support". `main()` calls `cef::execute_process` before anything else (our binary is its own helper, sandbox on), then `initialize` with windowless rendering and CEF's `multi_threaded_message_loop`, cache in `~/.local/state/cmux-linux/browser`. | Spike (journal): works with and without `DISPLAY`, 9 helpers from our binary, flat RSS over 1100 frames, 58 fps scrolling at 13–22 % of a core. A timer pump also worked; CEF's own thread is simpler. |
| E3 | Frames: `on_paint` (CEF thread) copies the BGRA buffer into an `Arc<RenderImage>` and sends it on a channel; the view swaps it in, `window.drop_image`s the old one, `cx.notify()`. Rendered at device pixels: CEF ignores `device_scale_factor` on Linux (spike), so the view rect is reported in device pixels and page zoom is set to the scale, re-applied after each navigation. | gpui's only Linux path for pixels is `img(ImageSource::Render)`; dropping old frames stops the atlas growing. Full-frame upload: fine for pages, heavy for video (noted risk). |

**Spike: done, GO** (2026-11-12, branch `m13-spike`, `spikes/cef-spike/`; report in the journal). Not covered by it: physical input, IME, `<select>` popups, cursor shapes, a real 2× monitor, Arch's `/usr/lib/cef` 154 against the crate's 156 (so the package bundles the download).

## Part 1: the surface (exit 1)

| | Decision | Why |
|---|---|---|
| B1 | A pane tab can hold a terminal or a browser: the surface kind lives in the app's surface map (`cmux-core` tree stays kind-agnostic; it already treats tabs as ids). `BrowserView` entity: URL bar, back, forward, reload buttons (cmux's chrome bar order, 28 px high), page below. Tab title = page title (else the URL host). | cmux `BrowserPanel`. |
| B2 | Input: mouse move/down/up/wheel → `send_mouse_*`; keys → `send_key_event` (raw down/up + char, Windows key codes from gpui `Keystroke`); IME through gpui's `EntityInputHandler` → `ime_set_composition`/`ime_commit_text`; focus in/out → `set_focus`; `on_cursor_change` → gpui cursor style. App shortcuts keep priority (bindings are contextless and match before the page). | Minimum for a usable page. |
| B3 | URL bar: Enter loads; text without a scheme gets `https://` if it has a dot and no spaces, else a search with `https://duckduckgo.com/?q=` (configurable `[browser] search_url` in `config.toml`). Popups (`on_before_popup`) open as a new browser tab in the same pane. | Smallest sensible omnibox. |
| B4 | `newBrowserTab` action (cmux `openBrowser`, Super+Shift+L), in the palette. | Spec "new-browser-tab shortcut". |

Tasks: 1 surface kind in the app and tab strip; 2 `BrowserView` with frames, resize, chrome bar, title; 3 input and IME; 4 action and palette entry; 5 check on KDE: open a tab, load a page, back/forward/reload, screenshot.

## Part 2: opening links and the socket (exit 2)

| | Decision | Why |
|---|---|---|
| L1 | Ctrl+click on a terminal URL (OSC 8 hyperlinks and detected `https?://` text, as Ghostty) opens it in a new browser tab in the clicked pane, or with `[browser] open_links = "system"` through `xdg-open`. | Spec; Ghostty needs a modifier to open links. |
| L2 | Socket `browser.open {url, surface?}` and `cmux browser open <url> [--surface ...]` open a browser tab next to the target (default: the caller's surface's pane). URL must be http(s) or file. | Spec. Other cmux `browser` automation commands out of scope. |

Tasks: 1 link detection and Ctrl+click routing; 2 socket method and CLI; 3 check by script.

## Part 3: persistence (exit 3)

| | Decision | Why |
|---|---|---|
| R1 | M10's `SavedTerm` gains a kind: browser tabs save `{kind: "browser", url}`; restore opens them at that URL. Old files without a kind read as terminals. | Spec; `session.json` version stays 1 (additive field). |

Tasks: 1 save/restore with a round-trip test; 2 check: open a browser tab, quit, relaunch, same URL.

## Deferred (with reason)

- Downloads, find-in-page, devtools, zoom, cookie import, profiles: beyond the spec.
- Damage-rect uploads and GPU-shared textures: need gpui changes; revisit if video is too slow.
- macOS app-bundle helpers (`bundle-cef-app`): the `macos` CI job builds without the `browser` feature until then.
