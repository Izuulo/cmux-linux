# m13-browser — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-12 — engine research and spike
- Research: gpui 0.2.2 has no Linux path for a native child view (`surface()` is macOS-only; a Wayland subsurface must come from gpui's own connection), so CEF off-screen rendering drawn through `img(ImageSource::Render)`. WPE needs hand-made bindings, WebKitGTK/wry only separate windows, Servo not embeddable yet.
- Spike report (crate `spikes/cef-spike/`, own workspace):

### M13 spike report: CEF off-screen rendering in gpui (2026-11-12)

#### Verdict: GO, with one caveat

All five points pass on KDE Plasma Wayland, both with and without `DISPLAY`. The caveat is HiDPI: CEF ignores the `device_scale_factor` from `GetScreenInfo`. A page-zoom workaround passes every check at scale 2 (details in point 3).

- Branch `m13-spike`, commit `6b4635f`, crate `spikes/cef-spike/`. It has its own `[workspace]`, and its lock is seeded from the main `Cargo.lock`.
  - `nodisplay`: `env -u DISPLAY`, default settings.
  - `display`: `DISPLAY=:0` (Xwayland present).
  - `extpump`: no DISPLAY, `SPIKE_PUMP=external`.
  - `scale2`: no DISPLAY, scale forced to 2 through `GetScreenInfo`. Fails, see point 3.
  - `zoom2`: no DISPLAY, scale 2 through the zoom workaround. Passes.
- Run it: `cargo build --release` with `CEF_PATH=<repo>/.tools/cef`, then `SPIKE_SCRIPT=<out dir> target/release/cef-spike`. Optional variables:
  - `SPIKE_PUMP=external`
  - `SPIKE_FORCE_SCALE=2`
  - `SPIKE_DSF=zoom`
  - `SPIKE_SWITCHES=a,b=c` (extra Chromium switches)
  - `SPIKE_NO_SANDBOX=1`
  - `SPIKE_CACHE=<dir>`

#### Versions
- Crate: `cef = "=156.4.0"` (156.4.0+156.0.26) with `default-features = false`. That drops `build-util` (clap, cargo_metadata), which Linux doesn't need. `cef-dll-sys 156.4.0+156.0.26` and `download-cef 3.0.0` come with it.
- CEF: `cef_binary_156.0.26+g8ea0c73+chromium-156.0.8078.88_linux64_minimal`.
- gpui `=0.2.2`, `image 0.25`, `smallvec 1`, `futures 0.3`. All of these were already in the main lock.

#### Points 1–5

**1. Init and helper processes: pass.**
- `execute_process` then `initialize` succeeds. It takes 75–290 ms, logged as `cef initialized in …`.
- Settings: `windowless_rendering_enabled=1`, `multi_threaded_message_loop=1` (CEF owns its UI thread), and the sandbox left on.
- The process list in each `log.txt` shows 9 children, all with `exe=` set to our own `cef-spike` binary: zygotes (renderer and GPU are forked from these) and a utility process.
- The sandbox works without root. Chromium uses the user-namespace sandbox, so the suid `chrome-sandbox` isn't needed.
- No `--ozone-platform` switch was needed. Chromium printed nothing to stderr, with or without `DISPLAY`.
- The external pump also works: `do_message_loop_work` every 5 ms from a gpui timer, same results.
- `cef::shutdown()` after `Application::run` returns exits cleanly with status 0.

**2. Draws in gpui with no growth: pass.**
- Path: `on_paint` (CEF thread) copies BGRA into `image::Frame`, which becomes `Arc<RenderImage>`. No swizzle is needed because gpui's image is BGRA too. It goes over an unbounded channel to a gpui task that keeps only the newest frame and calls `cx.notify()`. `render()` calls `window.drop_image(old)` for each replaced frame, then draws `img(ImageSource::Render(f))` sized to frame px / scale.
- Screenshots: `nodisplay/1-example-window.png` (example.org) and `nodisplay/4-wiki-window.png` (Wikipedia, "Terminal emulator").
- 1100 frames while scrolling:

  | Run | Main-process RSS before → after |
  |---|---|
  | `nodisplay` | 484 → 488 MB |
  | `display` | 510 → 516 MB |
  | `zoom2` (12 MB frames) | 560 → 573 MB, back to 561 MB when idle |

  Logged every ~115 frames, RSS is flat, with no upward trend.

**3. Resize: pass at 1x. HiDPI needs the zoom workaround.**
- Resizing the window to 760×560 (`window.resize`) gives a `was_resized` and CEF paints 760×560. Resizing back gives 1000×750. Frames: `3-resized-frame.png`.
- This screen runs at `window.scale_factor()` = 1.0.
- **Problem.** I returned `device_scale_factor=2` from `GetScreenInfo`, called `notify_screen_info_changed` and `was_resized`, and also tried `--force-device-scale-factor=2`. CEF still painted 1000×750, and the page reported `devicePixelRatio` 1. I confirmed that the callback runs (22 calls, return value 1) and that the switch reaches every child process. CEF's source copies the value straight through, so the cause is still unknown: a CEF 156 Linux bug, or something in how cef-rs maps this struct.
- **Workaround (`SPIKE_DSF=zoom`).** Report the view rect in device pixels, `device_scale_factor` 1, and `set_zoom_level(ln(scale)/ln(1.2))`. Zoom has to be applied again on every main-frame `on_address_change`, because Chromium keeps zoom per host. Mouse coordinates are multiplied by the scale.
- Result of the workaround: paint 2000×1500 for a 1000×750 view, page `devicePixelRatio` 2 with `innerWidth` 1000, and the link click, resize (1520×1120) and typing all pass (`zoom2/log.txt`, `zoom2/4-wiki-window.png`).

**4. Input: pass.**
- Link click: the centre of the first `<a>` on example.org was (492, 737). gpui MouseMove, Down and Up go to `send_mouse_*`, and the page navigated to `https://www.iana.org/help/example-domains`.
- Wheel: 5 `ScrollDelta::Lines(-3)` events go to `send_mouse_wheel_event` at 53 px per line, and `scrollY` went from 0 to 795.
- Typing: I injected a real `<input>` and clicked it, which focused it (`activeElement=spk`). Twelve gpui KeyDown/KeyUp events went to RAWKEYDOWN + CHAR + KEYUP with Windows VK codes, including shift-x then backspace. Value `"hello cef!"`, as expected (`5-typed-frame.png`).
- How the events were sent: `Window::dispatch_event` returns a private type in gpui 0.2.2, so it can't be called from outside gpui. The script calls the same `Spike::mouse_down/…/scroll` and `send_key` methods that the real `on_mouse_down`/`on_key_down` listeners call, with real gpui event structs. Only gpui's hit-test step is skipped.
- I didn't test physical mouse or keyboard input: no ydotool or wtype is installed, and I don't use sudo.

**5. Performance (16 cores, scrolling Wikipedia with a 40 px wheel event every 16 ms).**

| Run | Frames/s (gpui = CEF paints) | CPU while scrolling (whole process tree) | CPU idle |
|---|---|---|---|
| 1x, no DISPLAY | 58 | 13–22 % of one core | 2 % |
| 1x, DISPLAY | 58 | 13 % | 2 % |
| 1x, external pump | 58 | 15 % | 3 % |
| 2x (zoom), 12 MB/frame | 58 | 23 % | 5 % |

No frame was coalesced away: gpui frames equal CEF paints. Full-frame upload is acceptable for pages.

#### Runtime location of the pieces
- `cef-dll-sys`'s build script downloads into `$CEF_PATH/<cef version>/cef_linux_x86_64` (with `CEF_PATH=<repo>/.tools/cef`, which is already git-ignored via `/.tools/`). It then **copies** `libcef.so`, `*.pak`, `icudtl.dat`, `v8_context_snapshot.bin`, `locales/`, `libvk_swiftshader.so`, `libvulkan.so.1`, `vk_swiftshader_icd.json` and `chrome-sandbox` next to the binary in `target/<profile>/`.
- The binary links `libcef.so` as `NEEDED`. Pitfall: cef-dll-sys adds no rpath. The spike's `build.rs` adds `-Wl,-rpath,$ORIGIN`.
- CEF finds the paks, ICU and locales in the directory of `libcef.so` / the executable, so no `resources_dir_path` or `locales_dir_path` setting is needed.
- The helper is our own executable: `browser_subprocess_path` is left unset, and `execute_process` runs at the top of `main`.

#### Build time and size
- CEF download: 333 MB (`.tar.bz2`, the "minimal" distribution), extracted to about 1.5 GB. `.tools/cef` totals 1.9 GB including the archive.
- Cold `cargo build --release` of the spike, including the CEF download and extract: 2 min 02 s. Incremental builds take about 15 s. No cmake or C++ compiler is needed on Linux: the wrapper library is only built on Windows and macOS.
- Binary: 33 MB (25 MB stripped; almost all of it is gpui).
- Runtime files next to it: 1.5 GB as shipped, because **`libcef.so` is 1.48 GB unstripped**. Stripped it is 278 MB.
- Full stripped runtime: about 380 MB. That is libcef 278 MB, locales 50 MB (220 files; could be cut to `en-US.pak`), `resources.pak` 25 MB, swiftshader 15 MB, ICU 10 MB and the rest.
- Arch `extra/cef` is 154.0.34 (118 MiB download, 369 MiB installed). It is not installed here and I used no sudo, so `CEF_PATH=/usr/lib/cef` with the crate pinned to 156 is **untested**.

#### Pitfalls hit, and fixes
1. **xattr / libc.** A fresh lock resolves `libc 0.2.191`, and `xattr 0.2.3` (through gpui) fails to compile because `ENOATTR` is gone. Fix: copy the main `Cargo.lock` into the spike before the first build, which keeps libc at 0.2.189.
2. **No rpath for `libcef.so`.** Fix: `build.rs` emits `-Wl,-rpath,$ORIGIN`.
3. **`Window::dispatch_event` can't be called** from outside gpui (private return type). Fix: send events through the view's own handler methods.
4. **`Keystroke::parse("h")` has `key_char: None`**, so no CHAR event is sent and nothing gets typed. Real Wayland events set `key_char`. Test keystrokes need the `"h->h"` form.
5. **The HiDPI scale from `GetScreenInfo` is ignored.** Workaround: page zoom (point 3). Chromium keeps zoom per host, so it must be applied again after each navigation.
6. **`create_browser_sync` can't be used** with `multi_threaded_message_loop`, because it must run on CEF's UI thread. Fix: async `browser_host_create_browser`, then take the `Browser` in `LifeSpanHandler::on_after_created`.
7. **Screenshots catch the wrong window.** `spectacle -a` captured the owner's own cmux window once, after a run killed by timeout. I deleted that file. Only the screenshots taken while the spike window had focus are kept.
8. `Browser` and `BrowserHost` are not `Send`. The spike uses `unsafe impl Send/Sync` on the shared state, which is sound because CEF objects are thread-safe refcounted and host input methods post to the UI thread. M13 should wrap this more narrowly.

#### Recommendations for `crates/cmux-app`
- **Feature flag `browser`.** It brings in `cef` (with `default-features = false`) and a `build.rs` that adds the rpath. Because `libcef.so` is a hard `NEEDED` dependency, a binary built with the feature won't start if libcef is missing. So the feature should be off in CI and macOS jobs, as the plan says, and packaging must ship libcef. Strip `libcef.so` when packaging (1.48 GB down to 278 MB) and trim `locales/`.
- **`main()` order:**
  1. `cef::api_hash(CEF_API_VERSION_LAST, 0)`
  2. `Args::new()` and the `App`
  3. `execute_process`; if it returns a value ≥ 0, `exit` before anything else (no logging setup, no socket, no gpui)
  4. `initialize`, with the cache under `~/.local/state/cmux-linux/browser`
  5. `Application::run`
  6. after `run` returns, close the browsers, wait for `on_before_close`, then `cef::shutdown()`

  The CLI and socket code paths must not run in helper processes.
- **Message pump.** Use `multi_threaded_message_loop=1`: it needs no work on the gpui main thread and showed the same frame rate. The plan's E2 ("external message pump from a gpui timer") also works, but costs a 5 ms wakeup timer forever. I recommend changing E2 to the multi-threaded loop.
- **Frames.** Keep the spike's channel plus "newest frame wins" plus `drop_image` in `render`. Handle `PET_POPUP` (`<select>` dropdowns) by compositing the popup frame, or drawing it as a second `img` at `on_popup_size`. Not done in the spike.
- **Element offset.** Mouse coordinates must be relative to the page element's origin. The spike's page fills the window; in a pane, subtract the bounds origin from the canvas or prepaint.
- **HiDPI.** Start with the zoom workaround: device-px view rect, DSF 1, `set_zoom_level` again on each address change, mouse multiplied by the scale. Before relying on it, spend an hour checking whether a native-DSF fix exists: a cef-rs or CEF issue, or a different CEF build. Wheel deltas are then in device pixels, so the wheel scrolls half as far at 2x; multiply them by the scale too.
- **Input mapping.** The spike's `vk()` table, `mods()` (shift=1<<1, ctrl=1<<2, alt=1<<3, super=1<<7) and RAWKEYDOWN + CHAR (when `key_char` is set and Ctrl isn't held) + KEYUP are enough for text. Still to add:
  - the full VK table (F-keys, punctuation)
  - `native_key_code`: gpui 0.2.2 doesn't expose the scancode; unverified whether shortcuts inside pages, such as Ctrl+A, need it
  - `on_cursor_change` → gpui cursor
  - right and middle buttons, and mouse-leave
- **IME (not tested).** Implement `EntityInputHandler` on the browser view: `replace_and_mark_text_in_range` → `ime_set_composition`, `replace_text_in_range` → `ime_commit_text`, and `bounds_for_range` from `on_ime_composition_range_changed`. When an IME is active, gpui's Wayland backend sends committed text as a keystroke with `key_char`. Avoid double input by not sending CHAR when the input handler has taken the text.
- **NOTICE.** On integration, add cef-rs (MIT/Apache-2.0) and CEF (BSD-3, plus Chromium's `CREDITS.html`). The spike, which isn't shipped, did not touch NOTICE.

#### Not done or unchecked
- Physical input, IME, popups, cursor shapes, a real 2x monitor, and Arch's `/usr/lib/cef` with the 156 crate.
- Video and WebGL performance.
- Cold build time without the CEF download was not measured separately.

## 2026-11-11 — Part 1: the browser surface
- Built (feature `browser` on cmux-app, off by default): `browser.rs` (`BrowserView`: chrome bar, page image, input, IME), `browser/engine.rs` (CEF process setup, one off-screen browser per view, handlers → `Msg` channel; ported from the spike's `web.rs`, `Browser` wrapped in a narrow `Send` newtype instead of the spike's whole-struct `unsafe impl`), `browser/input.rs` (modifier flags, full VK table incl. F-keys and US punctuation/shifted symbols, key → RAWKEYDOWN/CHAR/KEYUP, CEF cursor → gpui cursor; unit-tested). `cmux-config::browser` (`[browser] search_url`, `omnibox`; tested without the feature).
- Surface kind: `workspace.rs` `SurfaceView { Terminal, Browser }` in the app's surface map; `cmux-core` untouched. Every former `TerminalView` lookup goes through `Surface::terminal()` (None for a browser), `title()`, `focus_handle()`, `any_view()`, so session save, socket snapshot, notifications, Dock, ssh workspaces, `busy`, metadata panels and `set_style` skip or handle browsers. `WorkspaceView::open_browser(pane, url)` is the entry point for Part 2 (link clicks, `cmux browser open`); `is_browser(surface)` is what Part 3's session code can read.
- Socket: surface rows carry `"type": "browser"` (cmux's value) through `SurfSnap::browser`; `surface.send_text`/`send_key` on a browser answer `invalid_params` "Surface is not a terminal".
- Actions (all in DEFAULTS, PALETTE and so Settings → Keyboard Shortcuts, remappable): `openBrowser` Super+Shift+L (cmux "Open Browser"), plus cmux's `focusBrowserAddressBar` Super+L, `browserBack` Super+[, `browserForward` Super+], `browserReload` Super+R. Without the feature, Open Browser shows a gpui prompt "Built without browser support"; the other four are not handled.
- `main()`: `browser::run_helper()` first (`api_hash`, `execute_process`, exit for helpers), `browser::shutdown()` after `Application::run` (close every browser, wait ≤ 2 s for `on_before_close`, `cef::shutdown`). `build.rs` adds the rpath `$ORIGIN:$ORIGIN/..` (the second for test binaries in `deps/`). `.cargo/config.toml` sets `CEF_PATH=.tools/cef`, so no env var is needed; `scripts/install-cef.sh` = one feature build (cef-dll-sys downloads there).
- Lock: adding `cef =156.4.0`, `image`, `smallvec` (optional) only added packages; the 9 removed lines are lock disambiguation (`libloading` → `"libloading 0.8.9"`, same for `base64`, `xattr`). libc stays 0.2.189. deny.toml gains `bzip2-1.0.6` and `CDLA-Permissive-2.0`, both from CEF's build-time downloader.
- Deviations from the plan: (1) CEF `initialize` runs on the first browser tab, not in `main()`: a session without browser tabs starts no Chromium processes (checked: helpers appear only after Super+Shift+L, terminals still exit normally after it). (2) Chrome bar 34 px, not 28: cmux's `BrowserChromeMetrics` (26 px buttons + 2 × 4 px padding, pill radius 10). (3) Four extra cmux browser actions (above), needed to drive back/forward/reload without a pointer and matching cmux's defaults. (4) HiDPI is always the spike's zoom workaround (device-pixel view, DSF 1, `set_zoom_level(ln scale / ln 1.2)` re-applied on each main-frame address change; mouse and wheel × scale).
- Deviations from cmux: new tab is `about:blank` with the URL field focused (cmux has its own new-tab page); no omnibox suggestions, lock badge, devtools, profile/theme menus; reload button is reload/stop as in cmux but has no context menu. Popups (`target=_blank`, `window.open`) become a browser tab in the same pane (plan B3). `<select>` dropdowns draw from `PET_POPUP` frames at `on_popup_size`. Copy/Paste actions in the page call the focused frame's `copy`/`paste` (Chromium's clipboard; unverified under Wayland). Paste in the URL field reads gpui's clipboard.
- Part 1 session behaviour: a browser tab is saved as a plain terminal entry (cwd null), so a restart brings a terminal in its place; Part 3 replaces this (marked `shortcut:` in `WorkspaceView::saved`).
- Check on KDE (dev build `--features browser`, `SHELL=/bin/sh`, own `XDG_CONFIG_HOME`/`XDG_STATE_HOME` under `$XDG_RUNTIME_DIR`, inherited `CMUX_*` cleared, `CMUX_SOCKET_MODE=allowAll`, `debug.shortcut.simulate`, active-window screenshots read by the agent): Super+Shift+L opened tab "about:blank" (globe icon) with the URL field focused and the socket listing it as `type: browser`; typing `example.org` + Enter loaded https://example.org/ (tab and sidebar title "Example Domain", back enabled); Super+L, `iana.org`, Enter → "Internet Assigned Numbers Authority"; Super+[ → "Example Domain"; Super+] → IANA (forward greyed); a local page titling itself with a random number (`localhost:8799`, also checks the `http://localhost` rule) changed title on each Super+R. `exit` in the terminal tab still closed it after CEF had started (pidfd path unaffected). `surface.close` on the browser tab removed it and its renderer processes. Super+Q with a live browser exited 0, no helper left; `session.json` held the browser as `{"cwd": null}`. A build without the feature: no `libcef` in `readelf -d`, Super+Shift+L shows the prompt, terminals unchanged. Not checked: physical mouse/keyboard (no pointer tool: no link click, no button click, no wheel, no `<select>`, no popup), IME, a real 2× screen, cursor shapes, video.
- Learned: CEF 156 (Chrome runtime) spawned KDE's `plasma-browser-integration-host` (from `/etc/chromium/native-messaging-hosts`) as our child: Chromium-wide integrations reach the embedded browser. Harmless here; `--disable-extensions` would stop it if it matters. gpui's Linux prompt still takes no keys, so the "Built without browser support" prompt needs a click to dismiss.

## Fixes after review (2026-11-11)
- Important 1: `BrowserView::release(window)` drops the page frame, popup frame and stale frames from the atlas; `WorkspaceView::remove_surface` calls it on every tab removal (`close_surface` both arms, `close_other_tabs`), `release_all` on workspace close (`WindowView::close_now`, before the view is dropped).
- Browser shortcuts (Super+[, Super+], Super+R, Super+L) `cx.propagate()` when the focused tab is not a browser, so terminals get them in feature builds.
- `was_hidden`: `WorkspaceView::sync_hidden` each render (a browser is shown only if its workspace is active and it is its pane's selected tab; zoom-hidden panes count as hidden) and on `set_active(false)` (now takes `cx`; also covers the Dock when the sidebar hides it). A browser hidden before CEF creates it is hidden in `on_after_created`.
- Frames: one slot per kind (`Engine::frames`, page and popup) plus a wake `Msg::Frame` sent only when the slot was empty, so at most one frame of each waits for the main thread.
- `BrowserEvent::Changed` (→ `WorkspaceEvent::Changed`) only when title or URL changed; cursor/loading/history changes just redraw the view.
- Popups open a tab only with `user_gesture`; others are cancelled.
- Without the feature all five browser actions show "Built without browser support" (`no_browser`).
- Socket: `vault.resume`/`surface.agent_session` on a browser answer `invalid_params` "Surface is not a terminal", like `send_text`/`send_key`.
- Arch PKGBUILD: builds with `--features cmux-app/browser` (CEF fetched into `.tools/cef` during that build), installs `libcef.so` (stripped), paks, ICU, V8 snapshot, swiftshader, `locales/` and `CREDITS.html` next to the binary in `/usr/lib/cmux-linux/` (rpath `$ORIGIN`), links `CREDITS.html` into the licence dir, adds libcef's runtime libraries to `depends` and `binutils` to makedepends. Not run through `makepkg`. The owner's uncommitted `pkgver` hunk was left unstaged.
- Re-check on KDE (same setup as above): five rounds of Super+Shift+L, load a local page, Super+W: RSS 505 → 511 MB flat after the first (CEF start); a page tab hidden (Ctrl+1 to the terminal, Super+R there) and shown again (Ctrl+2) repainted; `send_text` to it gave "Surface is not a terminal"; Super+Q exited 0 with no helper left.
- Skipped from the review: 6 (config read per Enter), 9 (LAN IP / IPv6 omnibox), 10 (gpui-bound tests), 11 (`COMMAND` flag).

## 2026-11-11 — Parts 2 and 3: links, socket, persistence
- Links (L1): `cmux-term::links` ports the scheme branch of Ghostty's link regex (`src/config/url.zig`) by hand, for `https://`, `http://` and `file://` only: URL characters plus `(word)`/`[word]` groups, trailing `.`/`,` dropped; Ghostty's own test lines are the unit tests. `Term::link_at(col, row)` checks the cell's OSC 8 URI first (Ghostty's order), then the row's text. Two methods added to the vendored bindings (`Terminal::viewport_row`, `Terminal::hyperlink_at`, over `ghostty_terminal_grid_ref` and `ghostty_grid_ref_hyperlink_uri`), recorded in `vendor/SOURCE`.
- Ctrl+click (Ghostty's Linux link chord, exactly Ctrl) opens the link on press, even when the app tracks the mouse (Ghostty's `mouseLinkRefreshAllowedState`); the matching release is swallowed. `TerminalEvent::OpenLink` → `WorkspaceView::open_link`: a browser tab in the clicked pane, unless `[browser] open_links = "system"`, the URL isn't http(s)/file, or the build has no browser; then `cx.open_url` (xdg-open).
- Socket/CLI (L2): `browser.open {url, surface_id?}` opens a tab in the target surface's pane (default: the focused surface) and answers cmux's open_split fields plus `"type": "browser"`. URLs other than http(s)/file → `invalid_params`; without the feature → `unavailable` "Built without browser support". `cmux browser open <url> [--surface|--workspace|--window]` (caller's `CMUX_SURFACE_ID` by default) prints `OK surface:N pane:N`. Tests: `cmux-ipc/tests/cli_browser.rs` against the real server with a canned reply.
- Persistence (R1): `SavedTerm.url`; a browser tab is written as `{"kind": "browser", "url": ...}`; an entry without `kind` (all older files) is a terminal. `WorkspaceView::build` takes per-surface URLs and calls the new `add_browser` (split out of `open_browser`) for them. Restored browsers in a non-selected workspace start hidden through Part 1's `set_active(false)` / `sync_hidden`.
- Decision: a build without the feature restores a saved browser tab as an empty terminal (keeps the layout's tab count; skipping would mean rewriting the shape). Its URL is lost at that run's next save.
- Deviations from cmux: (1) `browser open` adds a tab in the target's pane, cmux's adds a split (`browser.open_split`); (2) cmux's `browser <surface> open <url>` / `--surface` navigates that browser, ours uses the surface only to pick the pane; (3) no `--focus`, `--profile`: the new tab is always selected in its pane, like a Ctrl+click; (4) socket method is `browser.open`, as the plan says, not cmux's `browser.open_split`.
- Check on KDE (dev build `--features browser`, same setup as Part 1, temp state under `$XDG_RUNTIME_DIR`): `cmux browser open https://example.org` → `OK surface:2 pane:1`, tab "Example Domain"; `javascript:...` and a missing URL refused. Ctrl+click: no pointer tool, so a temporary, uncommitted socket hook built a `MouseDownEvent`/`MouseUpEvent` with Ctrl at a cell's centre and called `TerminalView::mouse_down`/`mouse_up` (the view's own handlers). On `echo`ed `https://www.iana.org/domains` it opened tab "Domain Name Services" next to the terminal; one column past the URL nothing opened; on an OSC 8 link printed as `LINK` it opened `https://example.com/`. `cmux browser open --workspace workspace:2 https://www.iana.org/` in a second workspace, Super+Q: `session.json` held the three browser tabs as `{kind, url}`; relaunch brought back both workspaces with the same tabs, order, selection and URLs, workspace 2's page painted once selected. A build without the feature on the same file: browser tabs came back as terminals, `cmux browser open` → "Error: unavailable: Built without browser support". `open_links = "system"` and the no-feature Ctrl+click were not clicked (they would open the owner's own browser); unit-tested only.

## 2026-11-12 — final review
- Whole-branch review: ready after three fixes, done: a Ctrl+click that opened a browser tab left `link_press` set, swallowing the terminal's next release (cleared at each press); a browser tab whose CEF failed to start saved an empty URL (the placeholder page keeps it); PKGBUILD lacked `cmake` and `ninja` (cef-dll-sys builds its wrapper with them). Also: CEF's `libvulkan.so.1` is no longer shipped next to the binary (the `$ORIGIN` rpath would have put it ahead of the system loader); CI comment says "lint", not "edit", vendored code (ghostty-vt gained two OSC 8 accessors, recorded in `vendor/SOURCE`).
- Accepted: a build without `browser` restores browser tabs as terminals and its next save drops their URLs (the Arch package builds with the feature). OSC 8 links of any scheme go to `xdg-open` with no hover preview (as Ghostty, minus the hover). CEF is fetched in `build()` and checked by SHA-1 only. Every restored browser tab starts a CEF browser at launch.
- Owner checks: mouse and IME in a page, `<select>` popups, a real 2× monitor, `makepkg -si` with the feature, `open_links = "system"`.
