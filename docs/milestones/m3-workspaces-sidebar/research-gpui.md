# M3 research — gpui: blur, sidebar UI, background polling

Read-only research for the M3 plan, cited as `rg §n`. Sources: `gpui-0.2.2/…` = the crates.io tarball; `zed@8a47b35` = Zed main HEAD on 2026-11-03; our code as `crates/cmux-app/src/…`. UNVERIFIED = not confirmed. Nothing compiled (researched on a macOS host).

## 1. Blur (`ext_background_effect_v1`)

### 1.1 Published gpui: none has it
- crates.io `gpui`: 0.1.0 (yanked), 0.2.0, 0.2.1, **0.2.2 (2025-10-22, still latest)**. No newer release.
- Zed main added it in **`14b7b0ef3`, 2026-10-30** — "gpui_linux: Replace kde-blur protocol with ext-background-effect protocol (#53746)". Main-only, git dependency.
- Fork `gpui-ce` (crates.io, Apache-2.0): 0.3.3 (2025-12-27) binds only `org_kde_kwin_blur` (`src/platform/linux/wayland/client.rs:64,123,161`); 0.3.0 (2026-10-20) moved Linux into a separate crate, published before Zed's commit, so almost certainly lacks it (UNVERIFIED).

### 1.2 How Zed main does it (`zed@8a47b35 crates/gpui_linux/src/linux/wayland/`)
- `client.rs:44-45,259-261,311-312`: binds `ExtBackgroundEffectManagerV1` (`globals.bind(&qh, 1..=1, ())`), preferred over kde-blur (`window.rs:2479-2506`).
- `update_window` (`window.rs:2439`), called on configure/resize/set_background_appearance (`:1175,1548,1923,2195-2227`): for `Blurred`, `get_background_effect(&surface)` once, then a `wl_region` covering the **whole window area** (rounded CSD corners), `set_blur_region(Some(&region))`, region destroyed. Whole-window blur, not sidebar-only.

### 1.3 Upgrading `=0.2.2` → Zed main: what breaks
- **Git dependency** on the huge zed repo; main still calls itself `gpui 0.2.2` but the API differs.
- **Platform split:** `gpui` + `gpui_platform` + `gpui_linux` + `gpui_wgpu` (`Cargo.toml:97-109`). `Application::new()` is gone → `gpui_platform::application().with_assets(..)` (`gpui_platform.rs:16`, `gpui/src/app.rs:242`); our `main.rs:15` changes.
- **Renderer:** Linux moved from blade to **wgpu** (`gpui_linux/Cargo.toml:64`) — M1 font/latency results would need re-checking.
- **Zed's `[patch.crates-io]`** (async-task, calloop, notify, khronos-egl, …) does not apply to git dependents; we would copy them (whether gpui needs the forks: UNVERIFIED).
- Our API use mostly survives: `TextRun`, `shape_line`, `WindowOptions`, `open_window`, `with_assets` unchanged; `Element` gained defaulted `a11y_*`; `EntityInputHandler` gained defaulted methods, so `ime.rs` should compile. Main adds `cx.background_spawn` (0.2.2 lacks it). Full break list needs a Linux build (UNVERIFIED).
- **Verdict:** too much churn for one protocol, and it still blurs the whole window. **Shim instead.**

### 1.4 Shim on gpui 0.2.2 — feasible, ~70 lines, `#[cfg(target_os = "linux")]`
- **Handles:** `gpui::Window` implements `HasWindowHandle`/`HasDisplayHandle` (`gpui-0.2.2/src/window.rs:4845,4851`). On Wayland they return the real libwayland `wl_surface` proxy (`platform/linux/wayland/window.rs:776-783`) and `wl_display` (`:786-800`).
- **Shared connection:** gpui builds `wayland-backend` with `client_system` + `dlopen` (`gpui-0.2.2/Cargo.toml:515-521`), so `Backend::from_foreign_display(*mut wl_display)` (wayland-backend 0.3.17 `src/sys/mod.rs:102`) and `ObjectId::from_ptr(&Interface, *mut wl_proxy)` (`:57`) exist; we open our own `wayland_client::Connection::from_backend(..)` (wayland-client 0.31.15 `conn.rs:111`) with a separate event queue.
- **Crates:** `wayland-protocols` has `ext::background_effect::v1` from **0.32.9** (2025-07-28), features `client` + `staging`. 0.32.13 is already in `Cargo.lock` (via ashpd); gpui uses 0.31.2 (lacks it). New direct Linux-only deps, all already locked: `wayland-client = "0.31"` (0.31.15), `wayland-backend = { version = "0.3", features = ["client_system"] }` (0.3.17), `wayland-protocols = { version = "0.32.9", features = ["client", "staging"] }`, `raw-window-handle = "0.6"` (0.6.2).
- **Protocol** (staging XML): `ext_background_effect_manager_v1` — `get_background_effect(new_id, wl_surface)`, event `capabilities(flags)` (bit `blur`), `destroy`. `ext_background_effect_surface_v1` — `set_blur_region(wl_region | null)`, `destroy`. Region is surface-local (logical), **clipped by the compositor to the surface**, **double-buffered** (applies on the next `wl_surface.commit`, which gpui does every presented frame); null removes blur; the `wl_region` may be destroyed right after the call.
- **Sketch** (`crates/cmux-app/src/blur.rs`):

```rust
pub struct SidebarBlur { conn: Connection, queue: EventQueue<State>, compositor: WlCompositor,
                         effect: ExtBackgroundEffectSurfaceV1, last_w: Option<i32> }
// State: Dispatch<WlRegistry, GlobalListContents> no-op; delegate_noop! for WlCompositor, WlRegion,
// `ignore` for the manager + surface (the capabilities event isn't needed).
impl SidebarBlur {
  pub fn new(window: &gpui::Window) -> Option<Self> {     // None on X11/macOS/no protocol
    let RawDisplayHandle::Wayland(d) = window.display_handle().ok()?.as_raw() else { return None };
    let RawWindowHandle::Wayland(w) = window.window_handle().ok()?.as_raw() else { return None };
    let conn = Connection::from_backend(unsafe { Backend::from_foreign_display(d.display.as_ptr().cast()) });
    let (globals, queue) = registry_queue_init::<State>(&conn).ok()?;  // one roundtrip, at startup
    let qh = queue.handle();
    let mgr: ExtBackgroundEffectManagerV1 = globals.bind(&qh, 1..=1, ()).ok()?;
    let compositor: WlCompositor = globals.bind(&qh, 1..=1, ()).ok()?;
    let id = unsafe { ObjectId::from_ptr(WlSurface::interface(), w.surface.as_ptr().cast()) }.ok()?;
    let effect = mgr.get_background_effect(&WlSurface::from_id(&conn, id).ok()?, &qh, ());
    Some(Self { conn, queue, compositor, effect, last_w: None })
  }
  /// Call from render; gpui's present commits it the same frame.
  pub fn set_width(&mut self, w: Option<Pixels>) {
    let w = w.map(|w| f32::from(w).round() as i32);
    if self.last_w == w { return; }
    let r = w.map(|w| { let r = self.compositor.create_region(&self.queue.handle(), ());
                        r.add(0, 0, w, i32::MAX / 2); r });  // height clipped by compositor
    self.effect.set_blur_region(r.as_ref());
    if let Some(r) = r { r.destroy(); }
    let _ = self.conn.flush();
    self.last_w = w;
  }
}
```

- **When to update:** only when sidebar width/visibility changes; window resizes need nothing (compositor clips). `x = 0` assumes server-side decorations (KWin SSD, as M0 saw); with CSD offset by gpui's `inset()` (`window.rs:217`).
- **Sidebar-only vs whole window:** blur behind opaque terminal pixels is invisible, so whole-window looks the same at `background-opacity = 1`; sidebar-only keeps a translucent terminal unblurred (Ghostty `background-blur` defaults false). Optionally widen to the whole window when Ghostty config sets `background-blur`.
- **Window background:** the sidebar needs real alpha, so on Linux use `WindowBackgroundAppearance::Transparent` always (today `main.rs` picks `Opaque` at opacity 1). Not `Blurred`: on Plasma ≤ 6.6 gpui's kde-blur path would blur the whole window (`gpui-0.2.2/.../window.rs:1133-1147`). `Transparent` drops gpui's opaque-region hint (`:1121-1131`), a small compositor cost. The terminal must paint its own full background at `background_opacity`.
- **Lifetime:** destroy the effect before gpui destroys the surface; only matters with multiple windows (M6).
- **Risks (UNVERIFIED — spike first, ~1 h at the desktop):** (1) `ObjectId::from_ptr` on a proxy gpui's own wayland-rs created — should work (the sys backend recognises rust-managed proxies) but not run; (2) a roundtrip on our queue while gpui's calloop source owns socket reads — libwayland's per-queue design allows it; (3) KWin honouring a region taller than the surface — protocol says clipped.

## 2. Sidebar UI in gpui 0.2.2

- **Resizable left panel:** reuse M2's divider pattern (`crates/cmux-app/src/workspace.rs` divider `on_drag`/`on_drag_move`): `div().id("sidebar-resize").cursor_col_resize().on_drag(SidebarDrag, |_, _, _, cx| cx.new(|_| EmptyView))`, root `.on_drag_move::<SidebarDrag>(…)` with `width = (ev.event.position.x - ev.bounds.left()).clamp(MIN, MAX)`. Terminal mouse handlers already skip while `cx.has_active_drag()` (`mouse.rs`). Zed's dock (`zed@8a47b35 crates/workspace/src/dock.rs:1281-1338`): 6 px handle (`RESIZE_HANDLE_SIZE`, `dock.rs:25`), `.absolute().right(-3px).h_full()`, wrapped in `deferred(..)` (`gpui-0.2.2/src/elements/deferred.rs:7`) + `.occlude()` (`div.rs:997`), `cx.stop_propagation()` in `on_mouse_down`, double-click reset via `MouseUpEvent.click_count == 2` (`interactive.rs:131`).
- **Layout:** `div().flex().flex_row()` with `[sidebar .w(px(w)).flex_none(), handle, content .flex_1().min_w(px(0.))]`. `WorkspaceView::container` uses `window.viewport_size()`; subtract the sidebar width there.
- **Row list:** ≤ 50 rows → plain flex, not `uniform_list` (`elements/uniform_list.rs:22` measures one item and assumes equal heights; our rows vary) nor `list()`/`ListState` (overkill). `div().id("ws-list").flex().flex_col().overflow_y_scroll().track_scroll(&scroll)` (`div.rs:1062,1077`), `scroll.scroll_to_item(ix)` (`div.rs:3141`). Row: `div().id(("ws", ix))`, `.hover(..)` (`div.rs:670`), `.when(selected, ..)` (`prelude::FluentBuilder`), `.active(..)` (`:1089`), `.on_click(cx.listener(..))` (`:1117`); hover-revealed close via `.group("row")` + `.group_hover("row", ..)` (`:680`).
- **Drag to reorder:** mirror M2's `tab_strip.rs` (`DraggedTab`, `.on_drag`, `on_drag_move`, `on_drop`, `on_children_prepainted` for row bounds) on y instead of x. Insertion line via `drag_over::<DraggedWs>(…)`. `gpui-0.2.2/examples/drag_drop.rs` (137 lines) is a small reference.
- **Inline rename:** 0.2.2 has no text input. `gpui-0.2.2/examples/input.rs` is 746 lines (Apache-2.0; attribute if copied): `TextInput` + actions + `EntityInputHandler` L33-385, `TextElement` L388-564, demo L566-746. Options: (a) focused div with `.on_key_down` (`div.rs:881`) using `keystroke.key_char`, Backspace, Enter commit, Escape cancel, text + 1 px caret (~60 lines, no IME/selection/mouse); (b) trimmed `input.rs` (~350 lines, IME-correct). `window.handle_input` registers only for the focused handle (`window.rs:3400`), so the terminal's handler is idle while renaming. Commit on blur (`cx.on_blur`), refocus the terminal. Recommendation: (a) now, (b) if CJK rename matters.

## 3. Background work and polling

- `cx.spawn(async move |this: WeakEntity<T>, cx: &mut AsyncApp| ..)` runs on the UI thread (`app/context.rs:237`). `cx.background_executor().spawn(fut)` needs `Send + 'static`, pool sized `available_parallelism` (`executor.rs:145`, `platform/linux/dispatcher.rs:30`). Timer: `cx.background_executor().timer(Duration)` (`executor.rs:357`; no `gpui::Timer` re-export; `background_spawn` is main-only). Dropping a `Task` cancels it; `.detach()` at `:76`. `WeakEntity::update/read_with` return `Result` (`app/entity_map.rs:691,730`); `Err` = entity gone → stop. The blink task (`terminal_view.rs`) already uses this.
- **Poll pattern:**

```rust
self._poll = cx.spawn(async move |this, cx| loop {
    let Ok(inputs) = this.read_with(cx, |s, _| s.poll_inputs()) else { break };   // plain data
    let meta = cx.background_executor().spawn(async move { cmux_meta::collect(inputs) }).await;
    if this.update(cx, |s, cx| { s.apply(meta); cx.notify(); }).is_err() { break }
    cx.background_executor().timer(Duration::from_millis(1500)).await;
});
```

  `std::process::Command` inside the background closure blocks one pool thread — fine at this scale. Run `gh` on its own slower task (30–60 s, cached) so a slow network never delays branch/cwd/ports. Collectors stay pure (inputs → `Meta`) for fixture tests without gpui. `smol::process` not needed.
- **File watching:** gpui has none (Zed's is in its `fs` crate on a patched `notify`). Recommendation: **don't watch** — read `.git/HEAD` per workspace every 1.5 s (microseconds), meets "within ~2 s". On cwd change, find the repo in the background by walking up to `.git`; a `.git` *file* (`gitdir: …`, worktree) is followed. If watching is ever needed: `notify = "8"` on the **`.git` directory, non-recursive** (git renames `HEAD.lock` over `HEAD`, so a file watch dies after the first switch).

---
Not done: nothing compiled; the shim was not run; gpui-ce 0.3.0's Linux crate not checked; cmux sidebar widths are in research-cmux.md. Main risk: `ObjectId::from_ptr` on gpui's surface proxy is unrun — spike it at the desktop first.
