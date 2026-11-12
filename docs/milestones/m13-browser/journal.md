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
