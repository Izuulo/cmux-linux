# M1 research: gpui 0.2.2, ghostty-vt, PTY, Ghostty config

Everything below was read from source. `gpui/` means `gpui-0.2.2/src/` in the cargo registry. `vt/` means `vendor/ghostty-vt/src/`. Anything not confirmed is marked **UNVERIFIED**.

Zed reference commit: `69e2130295c2649963eb639fc70b4f2ee8ea1624`. Its message is "gpui 0.2.2" (2025-10-22), and it is the `sha1` in gpui-0.2.2's `.cargo_vcs_info.json`.

---

## 1. gpui 0.2.2: rendering a terminal grid

### 1.1 Custom `Element` (gpui/element.rs:51-104)

`Element: 'static + IntoElement`. A custom element must implement both traits (`examples/input.rs:398-410` does `IntoElement for X { type Element = Self; fn into_element(self) -> Self { self } }`).

```rust
type RequestLayoutState: 'static;  type PrepaintState: 'static;
fn id(&self) -> Option<ElementId>;                                   // :65  (None is fine)
fn source_location(&self) -> Option<&'static core::panic::Location<'static>>; // :69 (REQUIRED, return None)
fn request_layout(&mut self, id: Option<&GlobalElementId>, inspector_id: Option<&InspectorElementId>,
                  window: &mut Window, cx: &mut App) -> (LayoutId, Self::RequestLayoutState);   // :73
fn prepaint(&mut self, id: Option<&GlobalElementId>, inspector_id: Option<&InspectorElementId>,
            bounds: Bounds<Pixels>, request_layout: &mut Self::RequestLayoutState,
            window: &mut Window, cx: &mut App) -> Self::PrepaintState;                          // :83
fn paint(&mut self, id: Option<&GlobalElementId>, inspector_id: Option<&InspectorElementId>,
         bounds: Bounds<Pixels>, request_layout: &mut Self::RequestLayoutState,
         prepaint: &mut Self::PrepaintState, window: &mut Window, cx: &mut App);               // :95
```

Fill the available area (examples/input.rs:421-424):

```rust
let mut style = Style::default();
style.size.width = relative(1.).into();
style.size.height = relative(1.).into();           // Zed terminal does the same (terminal_element.rs:~850)
(window.request_layout(style, [], cx), ())
```

### 1.2 Fonts and cell metrics

The window text system derefs to `TextSystem`, so the metric functions are reachable as `window.text_system()` (window.rs:1435) or `cx.text_system()` (app.rs:1446).

```rust
// text_system.rs
pub fn resolve_font(&self, font: &Font) -> FontId                                   // :150  (font_id() is PRIVATE, :109)
pub fn advance(&self, font_id: FontId, font_size: Pixels, ch: char) -> Result<Size<Pixels>> // :197
pub fn em_width(...)->Result<Pixels>  // :211, typographic bounds of 'm' (NOT the advance)
pub fn em_advance(...)->Result<Pixels> // :218, advance of 'm'
pub fn ascent(&self, font_id: FontId, font_size: Pixels) -> Pixels                  // :254
pub fn descent(&self, font_id: FontId, font_size: Pixels) -> Pixels                 // :260
pub fn baseline_offset(&self, font_id, font_size, line_height: Pixels) -> Pixels    // :265
```

- Zed's cell width is `text_system.advance(font_id, font_pixels, 'm').width` (terminal_element.rs:~985).
- `FontMetrics.line_gap` is `pub(crate)` (text_system.rs:846) and has no public accessor. We cannot reproduce Ghostty's `ascent+descent+line_gap` cell height exactly. **UNVERIFIED** against the screenshot-pair exit criterion; plan a configurable line-height ratio or an `adjust-cell-height` knob.
- On Linux `descent` is negated (`platform/linux/text_system.rs:134`, "todo(linux) confirm this is correct"), so its sign is **UNVERIFIED**. Use `descent.abs()`.
- `paint_line` vertically centres the glyphs in the `line_height` box (`text_system/line.rs:207`: `padding_top = (line_height - ascent - descent)/2`). The cell height can therefore be any rounded value.
- `window.line_height()` (window.rs:1868) is `text_style().line_height_in_pixels(rem)`. `window.text_style().font()` returns a `Font` (style.rs:459).
- Fonts: `Font { family: SharedString, features: FontFeatures, fallbacks: Option<FontFallbacks>, weight: FontWeight, style: FontStyle }` (text_system.rs:787). Bold is `FontWeight::BOLD` (:676). Italic is `FontStyle::Italic`.
- Ligatures: `FontFeatures::disable_ligatures()` sets calt=0 (text_system/font_features.rs:12). Zed defaults to that.
- Custom fonts can be added with `cx.text_system().add_fonts(Vec<Cow<'static,[u8]>>)` (text_system.rs:102).

### 1.3 Shaping and painting text

```rust
// text_system.rs:365
pub fn shape_line(&self, text: SharedString, font_size: Pixels, runs: &[TextRun],
                  force_width: Option<Pixels>) -> ShapedLine;   // panics (debug) if text has '\n'
// text_system.rs:733
pub struct TextRun { pub len: usize /*utf8 bytes*/, pub font: Font, pub color: Hsla,
    pub background_color: Option<Hsla>, pub underline: Option<UnderlineStyle>,
    pub strikethrough: Option<StrikethroughStyle> }
// text_system/line.rs:63
impl ShapedLine { pub fn paint(&self, origin: Point<Pixels>, line_height: Pixels,
                               window: &mut Window, cx: &mut App) -> Result<()>;
                  pub fn paint_background(...same args...) -> Result<()>;   // :86, paints TextRun.background_color
                  // plus Deref to LineLayout: .width, x_for_index(), index_for_x(), closest_index_for_x() (line_layout.rs:58-105)
}
```

- **`force_width`** (text_system/line_layout.rs:568-578) snaps glyph N to `N*force_width` whenever it drifts by more than 1px. It counts glyphs, not cells. A wide (2-column) glyph or a ligature therefore shifts everything after it. Zed avoids this by only batching cells that are column-contiguous (`start.column + cell_count == next.column`), which breaks the batch at wide glyphs. We must do the same.
- Pixel-exact cell grid recipe (Zed): one `shape_line(run_text, font_size, &[run_style], Some(cell_width))` per same-style, column-contiguous run, then `.paint(origin + (col*cell_w, row*line_h), line_h, ..)`.
- Layouts are cached per frame by (text, font_size, runs, force_width) (line_layout.rs:552-590), so re-shaping unchanged rows each frame is cheap.

### 1.4 Quads: cell backgrounds, cursor, selection

```rust
// window.rs:2839
pub fn paint_quad(&mut self, quad: PaintQuad)
// window.rs:5078  / :5090
pub fn fill(bounds: impl Into<Bounds<Pixels>>, background: impl Into<Background>) -> PaintQuad
pub fn outline(bounds, border_color: impl Into<Hsla>, border_style: BorderStyle) -> PaintQuad  // 1px border = hollow-block cursor
// color.rs:14  rgb(0xRRGGBB) -> Rgba ; rgba(0xRRGGBBAA) ; impl From<Rgba> for Hsla (:589)
```

- Background: `window.paint_quad(fill(Bounds::new(point(ox + col*cw, oy + row*lh), size(n*cw, lh)), rgba))`. Merge horizontally adjacent same-colour cells first (Zed `BackgroundRegion`).
- Bar and underline cursors are thin `fill` quads. A hollow block is `outline`. A block cursor needs the glyph repainted in the background colour (Zed shapes the cursor char again with the background colour).
- Window opacity: `bg(rgba(..alpha))` on the root already works (see `crates/cmux-app/src/main.rs`). The default cell background is skipped, so only explicit-bg cells are opaque (matches Ghostty's `background-opacity-cells=false`).

### 1.5 Re-render on new data

`Context::notify(&mut self)` (app/context.rs:229) marks the entity's view dirty (`App::notify`, app.rs:2034). `Render::render` is then rerun. A custom `Element` inside it gets fresh `request_layout/prepaint/paint`. Calling `notify` from a spawned task works (see 3).

`Window::request_animation_frame()` (window.rs:1654) re-notifies the current view next frame. Use it or a timer for cursor blink.

### 1.6 Size and bounds changes

There is no resize event inside an element. `prepaint(.., bounds, ..)` receives the final bounds every frame:

```rust
let cols = (bounds.size.width  / cell_w).floor() as u16;
let rows = (bounds.size.height / line_h).floor() as u16;
if (cols, rows) != last { term.resize(cols, rows, cell_w_px, cell_h_px); pty.resize(PtySize{rows,cols,..}); }
```

Zed does exactly this in prepaint (`terminal.set_size(dimensions)`, terminal_element.rs:~1000; `num_columns/num_lines` at terminal.rs:205-211).

Also available:
- `cx.observe_window_bounds(window, |this, window, cx| ..)` (app/context.rs:429)
- `window.bounds()` (window.rs:1688)
- `window.viewport_size()` (:1716)
- `window.scale_factor()` (:1815)
- `window.set_window_title(&str)` (:1779)

Pixel dims for the VT: `cell_w * scale_factor` (device pixels). **UNVERIFIED** whether Ghostty wants logical or device pixels. It is only used for pixel-size reports and kitty graphics.

---

## 2. gpui 0.2.2: input

### 2.1 Key events

```rust
// interactive.rs:22      pub struct KeyDownEvent { pub keystroke: Keystroke, pub is_held: bool }
// interactive.rs:40      pub struct KeyUpEvent   { pub keystroke: Keystroke }
// interactive.rs:55      pub struct ModifiersChangedEvent { pub modifiers: Modifiers, pub capslock: Capslock }
// platform/keystroke.rs:18
pub struct Keystroke { pub modifiers: Modifiers, pub key: String, pub key_char: Option<String> }
// platform/keystroke.rs:447
pub struct Modifiers { pub control: bool, pub alt: bool, pub shift: bool, pub platform: bool, pub function: bool }
```

- `key` is the layout-independent name: "a", "enter", "tab", "escape", "backspace", "delete", "left/right/up/down", "home/end/pageup/pagedown/insert", "space", "f1".., punctuation as the char itself, keypad as the keysym name with "kp_" stripped. Table in `platform/linux/platform.rs:806-900`.
- `key_char` is the text the key would type (None for cmd-s / Escape etc.). On Wayland, compose and dead keys are resolved by gpui and the composed text arrives in `key_char` (platform/linux/wayland/client.rs:~1320-1335).
- `platform` means Super on Linux and Cmd on macOS. `ModifiersChangedEvent` is dispatched separately.

Registering (div API, elements/div.rs; usable on the root `div()` that wraps the custom element):

```rust
div().track_focus(&self.focus_handle)            // FocusHandle from cx.focus_handle() (app.rs:2029)
     .key_context("Terminal")
     .on_key_down(cx.listener(Self::key_down))   // fn(&mut T, &KeyDownEvent, &mut Window, &mut Context<T>)  (div.rs:387)
     .on_key_up(cx.listener(Self::key_up))       // :419
     .child(TerminalElement { .. })
// focus: window.focus(&handle) (window.rs:1386); handle.is_focused(window) (:237); impl Focusable for T { fn focus_handle(&self,_:&App)->FocusHandle }
// stop further handling: cx.stop_propagation() (app.rs:1712)
```

Dispatch order (window.rs:3730-3850): keymap bindings, then actions, then `on_key_down` listeners (capture, then bubble). The Wayland window then runs `handle_input` (platform/linux/wayland/window.rs:719-736). If the key event was not stopped and modifiers are a subset of shift and `key_char` is set, it calls `input_handler.replace_text_in_range(None, key_char)`. So **a handler that encodes and `stop_propagation()`s prevents double input**, and an unhandled printable key falls through to the IME handler.

Tab is not intercepted unless the app binds it. Focus-next only fires via `Window::focus_next()` (window.rs:1413), which we never call. Do NOT add `.tab_stop(true)`.

Use `window.on_key_event::<ModifiersChangedEvent>` (window.rs:3444) only if modifier-only handling is needed.

Zed's approach (terminal_view.rs:1004): `key_down` calls `try_keystroke(&event.keystroke)`, and if it produced bytes calls `cx.stop_propagation()`.

### 2.2 IME

Trait `EntityInputHandler: 'static + Sized` (input.rs:10-73). All methods take `window: &mut Window, cx: &mut Context<Self>` last (except `marked_text_range(&self, ..)`):

```rust
fn text_for_range(&mut self, range: Range<usize>, adjusted: &mut Option<Range<usize>>, ..) -> Option<String>;
fn selected_text_range(&mut self, ignore_disabled_input: bool, ..) -> Option<UTF16Selection>;  // UTF16Selection{range,reversed} platform.rs:982
fn marked_text_range(&self, ..) -> Option<Range<usize>>;
fn unmark_text(&mut self, ..);
fn replace_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, ..);       // commit
fn replace_and_mark_text_in_range(&mut self, range: Option<Range<usize>>, new_text: &str,
                                  new_selected_range: Option<Range<usize>>, ..);         // preedit
fn bounds_for_range(&mut self, range_utf16: Range<usize>, element_bounds: Bounds<Pixels>, ..) -> Option<Bounds<Pixels>>;
fn character_index_for_point(&mut self, point: Point<Pixels>, ..) -> Option<usize>;
```

Wiring (examples/input.rs:481-486, inside `Element::paint`):

```rust
window.handle_input(&focus_handle, ElementInputHandler::new(bounds, self.input.clone()), cx);  // window.rs:3400
```

`handle_input` only registers when `focus_handle.is_focused(window)` and must be called during paint. `ElementInputHandler::new(element_bounds: Bounds<Pixels>, view: Entity<V>)` is at input.rs:86.

The lower-level `trait InputHandler: 'static` (platform.rs:995-1085) has the same methods without `element_bounds`, taking `cx: &mut App`. Zed implements it directly (`TerminalInputHandler`, terminal_element.rs:1370-1486), which is the better fit because it can capture `cursor_bounds`.

- `window.invalidate_character_coordinates()` (window.rs:4127) moves the IME popup.
- The Wayland client enables `zwp_text_input_v3` while an input handler is registered (wayland/client.rs:295-341).

### 2.3 Mouse

All in interactive.rs:

```rust
MouseDownEvent { button: MouseButton, position: Point<Pixels>, modifiers, click_count: usize, first_mouse: bool }  // :93
MouseUpEvent   { button, position, modifiers, click_count }                                                       // :120
MouseMoveEvent { position, pressed_button: Option<MouseButton>, modifiers }                                       // :335
ScrollWheelEvent { position, delta: ScrollDelta, modifiers, touch_phase: TouchPhase }                             // :363
enum ScrollDelta { Pixels(Point<Pixels>), Lines(Point<f32>) }  // :395; .precise(), .pixel_delta(line_height) -> Point<Pixels>
```

Registration:
- On a div (elements/div.rs): `.on_mouse_down(btn, f)` (:121), `.on_mouse_up(btn, f)` (:173), `.on_mouse_up_out(btn, f)` (:243), `.on_mouse_move(f)` (:263), `.on_scroll_wheel(f)` (:312).
- Inside `paint` (for drags that leave the element): `window.on_mouse_event::<MouseMoveEvent>(move |e, phase: DispatchPhase, window, cx| ..)` (window.rs:3421).
- Hit testing: `window.insert_hitbox(bounds, HitboxBehavior::Normal)` (window.rs:3305) then `hitbox.is_hovered(window)`.
- Cursor shape: `window.set_cursor_style(CursorStyle::IBeam, &hitbox)` (:2351).
- Click counts for double and triple click come from `click_count` (Wayland sets it).

Zed's mouse wiring is `register_mouse_listeners` (terminal_element.rs:646-790). Scroll-lines accumulation is `determine_scroll_lines` (terminal.rs:1927-1951): accumulate `pixel_delta(line_height).y` and emit the integer line delta.

### 2.4 Clipboard (Wayland)

```rust
// app.rs:1041 / :1053
cx.write_to_clipboard(ClipboardItem::new_string(text));     // ClipboardItem platform.rs:1504, new_string :1519
let text: Option<String> = cx.read_from_clipboard().and_then(|i| i.text());   // text() :1553
// PRIMARY selection exists, Linux/FreeBSD only (#[cfg(any(target_os="linux", target_os="freebsd"))]):
cx.write_to_primary(item);  cx.read_from_primary()          // app.rs:1036 / :1048
```

The Wayland backend binds `zwp_primary_selection_device_manager_v1` (wayland/client.rs:45-50, 109, 516). So copy-on-select goes to primary and middle-click pastes from it. These APIs are `cfg`-gated, so guard them for the later macOS port.

---

## 3. gpui 0.2.2: async PTY to Entity

Facts:
- `Context::spawn` (app/context.rs:237): `fn spawn<AsyncFn, R>(&self, f: AsyncFn) -> Task<R> where AsyncFn: AsyncFnOnce(WeakEntity<T>, &mut AsyncApp) -> R + 'static`. It runs on the main thread.
- `WeakEntity::update(&self, cx: &mut C, f: impl FnOnce(&mut T, &mut Context<T>) -> R) -> Result<R>` (app/entity_map.rs:691). It errs when the entity is dropped, which is how you exit the loop.
- `AsyncApp::background_executor()` (app/async_context.rs:132); `BackgroundExecutor::spawn(future + Send)` (executor.rs:145); `BackgroundExecutor::timer(Duration) -> Task<()>` (executor.rs:357).
- A `Task` must be held or `.detach()`ed (executor.rs:76).
- gpui re-exports no channel crate (only `smol::Timer`, gpui.rs:94). gpui itself depends on `futures = "0.3"` and `smol = "2.0"` (Cargo.toml:270, 374). Add `futures = "0.3"` to cmux-term/app.
- Wakers from foreign threads are fine, because the executor dispatches through the platform dispatcher.

Idiomatic pattern, as Zed uses it (terminal.rs:~655-720): an OS thread does blocking `read()` on the PTY and sends over a channel. A main-thread task drains it, handles the first event immediately for latency, then batches for 4 ms (max 100 events) before updating.

Minimal version (parse on the reader thread under a `Mutex`; the UI only needs a repaint signal):

```rust
use futures::{StreamExt, channel::mpsc};
use std::sync::{Arc, Mutex};

struct TerminalView { term: Arc<Mutex<cmux_term::Term>>, /* focus, pty writer, ... */ _task: gpui::Task<()> }

fn new(cx: &mut Context<TerminalView>, term: Arc<Mutex<Term>>, mut reader: Box<dyn Read + Send>) -> Task<()> {
    let (tx, mut rx) = mpsc::unbounded::<()>();
    let t = term.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 65536];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,                       // EOF/EIO on Linux when child exits
                Ok(n) => { t.lock().unwrap().feed(&buf[..n]); let _ = tx.unbounded_send(()); }
            }
        }
        drop(tx);                                              // rx loop ends -> show "[process exited]"
    });
    cx.spawn(async move |this, cx| {
        while rx.next().await.is_some() {
            // coalesce a burst: drain whatever is queued, then wait ~4ms like Zed
            while let Ok(Some(())) = rx.try_next() {}
            cx.background_executor().timer(std::time::Duration::from_millis(4)).await;
            if this.update(cx, |_, cx| cx.notify()).is_err() { break; }   // entity dropped
        }
    })
}
```

Notes for the plan:
- The ghostty `on_pty_write` callback fires **inside** `feed()` under the lock. Callbacks "must not touch the Terminal" (vt/terminal.rs:348-350). Make it send the bytes into an `std::sync::mpsc::Sender<Vec<u8>>`, and have a PTY-writer thread (owning `take_writer()`) drain it. Never write to the PTY from the reader thread: if the child is blocked on a full pty buffer and we block on write, we deadlock.
- Alternative, simpler for ownership: send `Vec<u8>` chunks over the channel and call `feed` on the main thread inside `this.update`. This avoids the Mutex but parses on the UI thread. Prefer the Mutex design for heavy output (e.g. `cat` a large file).

---

## 4. Zed terminal reference

**Pinned URL:** https://github.com/zed-industries/zed/blob/69e2130295c2649963eb639fc70b4f2ee8ea1624/crates/terminal_view/src/terminal_element.rs

Related files at the same SHA:
- `crates/terminal_view/src/terminal_view.rs`
- `crates/terminal/src/terminal.rs`
- `crates/terminal/src/mappings/keys.rs`

The `terminal_view` crate is `license = "GPL-3.0-or-later"` with `LICENSE-GPL`, so it is compatible with this project. It is built on `alacritty_terminal`, so adapt the structure, not the cell types.

### How it works

- **Batching cells to runs.** `layout_grid` (terminal_element.rs:322-494) walks cells line by line.
  - `BatchedTextRun { start_point, text, cell_count, style: TextRun, font_size }` (:82-157).
  - A cell appends to the current run iff `can_append` (same font, fg, bg, underline, strikethrough; :108) AND same line AND `start.column + cell_count == cell.column`.
  - Blank cells are not emitted (`is_blank`, :1488).
  - Wide-char spacer cells are skipped.
  - Zero-width extras (combining marks) are appended to the text without counting a cell (`append_zero_width_chars`, :120).
  - Each run is painted with `shape_line(text, font_size, &[style], Some(cell_width)).paint(pos, line_height, ..)` (:134-157).
- **Backgrounds.** The same pass builds `BackgroundRegion`s for non-default bg. Adjacent same-colour regions are merged (`merge_background_regions`, :194-273) into single-line `LayoutRect`s (:159-192), painted before text. INVERSE swaps fg/bg first (:350-360).
- **Cell size.**
  - `cell_width = advance('m')`, `line_height = font_size * ratio` (:~985-1010).
  - `TerminalBounds { cell_width, line_height, bounds }` with `num_columns = floor(width/cell_width)` and `num_lines = floor(height/line_height)` (terminal.rs:205-211).
  - Resize is requested from prepaint (`terminal.set_size`, terminal.rs:1385).
- **Cursor.** `shape_cursor` (:496-528) floors the origin to pixels and ceils the width. The cursor char is re-shaped in the background colour for block cursors. Hollow when unfocused. The cursor layout is computed even when hidden, because the IME needs its rect.
- **Paint order** (:1200-1360): background quad, rects, selection highlights, text runs, IME preedit, cursor.
- **IME.**
  - `TerminalInputHandler` (:1370-1486) implements `InputHandler`.
  - `selected_text_range` returns `None` on the alt screen, else `0..0`.
  - `replace_text_in_range` clears marked text and writes the committed text to the PTY.
  - `replace_and_mark_text_in_range` stores `marked_text` in view state (terminal_view.rs:318).
  - `bounds_for_range` is `cursor_bounds` shifted by `cell_width * range.start`.
  - Marked text is painted at the cursor with an underline, and the cursor is hidden while composing (:1262-1296).
- **Event batching.** `subscribe` (terminal.rs:~655-720). First event immediate, then 4 ms windows, max 100 events.

### Functions worth adapting

1. `BatchedTextRun` plus the batching loop in `layout_grid` (terminal_element.rs:82-157, 322-494). Replace `IndexedCell` with a `&[ghostty_vt::Cell]` row.
2. `BackgroundRegion` / `merge_background_regions` / `LayoutRect::paint` (:159-273). Almost verbatim.
3. `TerminalInputHandler` (:1370-1486) and the IME paint block (:1262-1296).
4. `shape_cursor` / `CursorLayout` (:496-528 and cursor.rs in the same crate; not read).
5. `register_mouse_listeners` (:646-790) and `determine_scroll_lines` (terminal.rs:1927-1951). Selection and mouse-mode routing skeleton.
6. The subscribe/batch loop (terminal.rs:~655-720), for the PTY to UI path.

Do not copy:
- Zed's theme and settings plumbing (`cx.theme()`, `ThemeSettings`).
- `ensure_minimum_contrast`.
- Hyperlink and search highlighting.

---

## 5. Vendored ghostty-vt (`vendor/ghostty-vt/src/`)

Crate license MIT (Cargo.toml; `vendor/SOURCE`). It is from cmux-tui at `21dcc2b`, on top of the manaflow-ai/ghostty fork at `ef4f157`.

### 5.1 Threading

- `unsafe impl Send for Terminal {}` (terminal.rs:2008). The comment says to guard it with a Mutex. `unsafe impl Send for RenderState {}` (render.rs:247). `unsafe impl Send for KeyEncoder` (key.rs, after `pub struct KeyEncoder`). `unsafe impl Send for MouseEncoder` (mouse.rs:84).
- None are `Sync`. They use raw C handles and `&mut self` for all operations.
- **Verified by compiling**: a throwaway test asserted `Send` for `cmux_term::Term`, `Terminal`, `RenderState`, `KeyEncoder` and `MouseEncoders`; it passed (test file removed).
- So `Arc<Mutex<Term>>` works. The `Callbacks` closures are `Box<dyn FnMut + Send>`.

### 5.2 Key encoding (key.rs)

```rust
pub struct Mods(pub u16);   // consts SHIFT, CTRL, ALT, SUPER, CAPS_LOCK, NUM_LOCK; BitOr; .contains()  (key.rs:15-35)
pub struct KeyInput { pub key: sys::GhosttyKey /*physical, GHOSTTY_KEY_* */, pub mods: Mods, pub consumed_mods: Mods,
    pub composing: bool, pub utf8: String /*no C0 controls*/, pub unshifted_codepoint: u32, pub shifted_codepoint: u32,
    pub base_layout_codepoint: u32, pub action: Option<KeyAction /*Press|Release|Repeat*/>, pub macos_option_as_alt: bool } // :125
impl KeyEncoder { pub fn new() -> Result<Self>;
    pub fn sync_from_terminal(&mut self, terminal: &Terminal);                 // pulls DECCKM, keypad, kitty flags, modifyOtherKeys
    pub fn encode(&mut self, input: &KeyInput, out: &mut Vec<u8>) -> Result<()> }  // appends bytes; empty = nothing to send
```

- Kitty keyboard protocol is automatic. `sync_from_terminal` before each `encode` picks up the flags the app set via `CSI > u` (the C option `GHOSTTY_KEY_ENCODER_OPT_KITTY_FLAGS`, vendor/ghostty/include/ghostty/vt/key/encoder.h). Release events need `action: Some(KeyAction::Release)` from `on_key_up`. `append_encoded_key` (key.rs:~250) restores shifted/base-layout alternates on CSI-u output.
- Tests that show use: key.rs:`encodes_ctrl_c`, `encodes_arrow_application_mode` (DECCKM `ESC [ ? 1 h` flips `ESC[A` to `ESC OA` after sync).
- `key_input_from_chord("ctrl+shift+a")` (key.rs:72) is a socket/CLI text parser. It has no super, no `is_held`, and covers only a fixed key-name set. Useful for tests, **not** for gpui events.
- We need our own `gpui::Keystroke -> KeyInput` mapping:
  - `key` name to `sys::GHOSTTY_KEY_*` table (A-Z, DIGIT_*, F1-F25, ARROW_*, HOME/END/PAGE_*, NUMPAD_*, etc.; names in `vendor/ghostty/include/ghostty/vt/key/*.h`).
  - mods: `control` to CTRL, `alt` to ALT, `shift` to SHIFT, `platform` to SUPER.
  - `utf8 = key_char` (drop if it contains C0 controls; empty when Ctrl is held, as `key_input_from_chord` does).
  - `unshifted_codepoint = key.chars().next().to_ascii_lowercase()` for single chars.
  - `consumed_mods = SHIFT` when shift produced the printable `key_char`.
  - `action`: `Repeat` if `is_held`.
- **Paste and focus are not wrapped** in Rust, but the bindgen allowlist is `ghostty_.*` (ghostty-vt-sys/build.rs:102), so `ghostty_vt::sys::ghostty_paste_encode(data, len, bracketed, buf, buf_len, &mut written)` (vt/paste.h:87) and `ghostty_focus_encode` (focus.h:64) are callable. `Terminal::mode(2004, false)` gives bracketed paste and `mode(1004, false)` gives focus events.

### 5.3 Mouse (mouse.rs)

```rust
pub enum MouseAction { Press, Release, Motion }
pub enum MouseButton { Left, Right, Middle, WheelUp, WheelDown, WheelLeft, WheelRight }
pub struct MouseInput { pub action, pub button: Option<MouseButton>, pub mods: Mods,
    pub position: (f32, f32) /*surface px*/, pub screen_size: (u32,u32), pub cell_size: (u32,u32), pub any_button_pressed: bool }  // :30
impl MouseEncoders { pub fn new()->Result<Self>; pub fn sync_from_terminal(&mut self,&Terminal);
    pub fn encode(&mut self, MouseInput, out: &mut impl Extend<u8>)->Result<()>;          // :147 (press/motion/wheel)
    pub fn encode_release(&mut self, MouseInput, out: &mut impl Extend<u8>)->Result<()>;  // :151 (separate encoder so release keeps press's protocol)
    pub fn reset_motion_dedupe(&mut self) }
```

- `Terminal::mouse_tracking() -> bool` (terminal.rs:3016) says whether the app wants mouse events. When false, handle selection and scroll locally. Holding Shift bypasses reporting (Zed `mouse_mode(e.shift)`; the Ghostty convention).
- The encoder returns empty output when the current mode does not want the event, so you can call it unconditionally while `mouse_tracking()`.
- The wire format (SGR, etc.) comes from the terminal state through `sync_from_terminal`. `Terminal::active_mouse_format()` exposes it (terminal.rs:2281).
- Wheel: encode `WheelUp/Down` as Press events. In the alt screen without mouse tracking, Ghostty sends arrow keys. **UNVERIFIED** whether `ghostty-vt` does that; Zed's `alt_scroll` does (terminal.rs:1908-1914). Check mode 1007 via `Terminal::mode(1007, false)`.

### 5.4 Callbacks, resize, scroll

```rust
pub struct Callbacks { pub on_pty_write: Option<Box<dyn FnMut(&[u8]) + Send>>,
                       pub on_title_changed: Option<Box<dyn FnMut() + Send>>, pub on_bell: Option<Box<dyn FnMut() + Send>> }  // terminal.rs:354
```

- They run synchronously inside `vt_write` and must not touch the `Terminal`. Read `title()` afterwards.
- `Terminal::new(cols: u16, rows: u16, max_scrollback: usize, callbacks)` (:2041). Test usage: `ghostty-vt/tests/terminal.rs:60-96`.
- `fn resize(&mut self, cols: u16, rows: u16, cell_width_px: u32, cell_height_px: u32) -> Result<()>` (terminal.rs:2688). Cols and rows are clamped to at least 1.
- Scrolling (terminal.rs:3111-3140):
  - `scroll_delta(&mut self, delta: isize)`: **negative = toward history** (tests/render_v7.rs:142 `scroll_delta(-5)` then `scrolled_back()`).
  - `scroll_to_bottom(&mut self)`.
  - `scrollbar() -> Option<Scrollbar { total, offset, len }>` (:293), `.scrolled_back()`. It is documented as potentially expensive, so call it at most once per frame.
  - `scrollback_rows()`, `history_rows()`, `styled_history_rows(start, count)`.
- Typing should call `scroll_to_bottom()` (Zed does `Scroll::Bottom` on input, terminal.rs:1399).
- `title() -> Option<String>`, `pwd()` (OSC 7), `cursor_position()`.

### 5.5 Selection (app-level; the wrapper has no selection state)

```rust
pub struct SelectionPoint { pub column: u16, pub row: u32 }          // absolute screen coords (scrollback + screen), terminal.rs:313
pub struct SelectionRange { pub start: SelectionPoint, pub end: SelectionPoint }  // :321
Terminal::select_word_screen(&self, SelectionPoint) -> Result<Option<SelectionRange>>                       // :3221 (double-click)
Terminal::select_word_between_screen(&self, start, end) -> Result<Option<SelectionRange>>                   // :3247 (word drag)
Terminal::select_line_screen(&self, SelectionPoint) -> Result<Option<SelectionRange>>                       // :3280 (triple-click)
Terminal::normalize_selection_point_screen(&self, SelectionPoint) -> Option<SelectionPoint>                  // :3193 (wide-char lead cell)
Terminal::selection_text_absolute(&mut self, start: (u16,u64), end: (u16,u64)) -> Option<String>            // :3391 (wrap-aware copy text)
Terminal::selection_text(&mut self, start:(u16,u16), end:(u16,u16)) -> Option<String>                       // :3358 (viewport coords)
Terminal::track_screen_point(x, y) -> Result<TrackedScreenPoint>  // :3141, anchor that follows its row through scrollback/reflow
```

- Convert viewport row to absolute row: `abs = scrollbar().offset + viewport_row`.
- The C API has a terminal-owned selection (`GHOSTTY_TERMINAL_OPT_SELECTION`, terminal.h:901) and `GHOSTTY_RENDER_STATE_ROW_DATA_SELECTION`, which would give per-row highlight ranges. The Rust wrapper does **not** expose them (`grep OPT_SELECTION vt/*.rs` is empty). So M1 stores an app-level `SelectionRange` (use `TrackedScreenPoint` if the selection must survive output) and paints the highlight itself.
- `Terminal::viewport_text()` and `plain_text()` are for tests and debugging.

### 5.6 Render state, "runs" and cost

```rust
RenderState::new()->Result<Self>
.update(&mut self, &mut Terminal) -> Result<()>                       // snapshot viewport (render.rs:287), also reloads palette
.dirty() -> Dirty{Clean,Partial,Full}; .set_clean()                   // :371/:380
.size() -> (cols, rows); .default_colors() -> (bg: Rgb, fg: Rgb)      // :392/:399
.palette_color(u8)->Rgb ; .cursor()->Option<CursorInfo{x,y,shape,blinking}> // :412/:424 (None when hidden OR scrolled back)
.cursor_visual()->Result<(CursorShape,bool)>; .cursor_color()->Option<Rgb>   // OSC 12 override only
.walk_rows(&mut self, f: impl FnMut(usize /*row*/, bool /*dirty*/, &[Cell])) -> Result<()>  // :511
.build_frame(&mut self) -> Result<RenderFrame>                        // :469
```

- `build_frame` does `cells.to_vec()` for every row (:480-485). This is the per-frame clone STATUS.md flags. `RenderFrame::{styled_rows, styled_row, runs, row_runs}` (:205-229) and `rows_to_runs(&[Vec<Cell>])` all work on those cloned `Vec<Vec<Cell>>`.
- `walk_rows` is the cheap path. It reuses one row buffer and its `String` capacities, and hands out `&[Cell]` borrowed per row. It also clears per-row dirty flags. It needs `&mut RenderState`, so call it under the lock.
- **`StyledRun` is a poor fit for a grid painter** (render.rs:120-127, `row_to_runs` :~790).
  - It has `text, fg, bg, attrs, underline, width_hint` but **no start column**.
  - Narrow-run column counts are not exposed, because `columns` is private and only reported via `width_hint` when ambiguous.
  - It does not split at wide glyphs.
  - It does not swap inverse (`attrs & ATTR_INVERSE` is left to the caller).
  - So write our own batching from `&[Cell]` inside `walk_rows` (Zed-style, section 4), which also needs column indices for backgrounds, selection and cursor. Do not clone.
- `Cell` fields (render.rs:87): `text: String` (empty = blank), `width: CellWidth {Narrow, Wide, SpacerTail, SpacerHead}`, `fg/bg: ColorSpec {Default, Palette(u8), Rgb}`, `resolved_fg/resolved_bg: Option<Rgb>`, `bold, faint, italic, underline, underline_style: Option<UnderlineStyle>, strikethrough, inverse, blink, invisible`.
- Colour resolution (header doc, vendor/ghostty/include/ghostty/vt/render.h:650-665):
  - `resolved_fg/bg` are palette-resolved but `None` means "use the default". Palette lookups already include OSC 4 overrides.
  - **Bold-as-bright is not applied.**
  - **Inverse is not applied.**
  - Faint and invisible are not applied.
  - The renderer must do: `fg = resolved_fg.unwrap_or(default_fg)`, `bg = resolved_bg.unwrap_or(default_bg)`, then swap on `inverse`, dim on `faint`, skip on `invisible`, and apply the `bold-color` config.
  - Cells whose resolved bg is `None` should not paint a quad. This also keeps window transparency correct.

Defaults and palette (terminal.rs):

```rust
Terminal::set_default_colors(&mut self, fg: Option<Rgb>, bg: Option<Rgb>, cursor: Option<Rgb>)   // :2322 (None = unchanged)
Terminal::replace_default_colors(.., ..)                                                          // :2353 (None clears)
Terminal::set_default_palette(&mut self, &[Option<Rgb>; 256])                                     // :2381 (None = built-in)
Terminal::set_default_cursor(Option<CursorShape>, Option<bool> /*blink*/)                         // :2400
Terminal::effective_palette()->Result<[Rgb;256]>; effective_colors(); effective_cursor_visual()   // :2315 / :2460 / :2471
pub struct TerminalColorOverrides{foreground,background,cursor,cursor_visual,palette:[Option<Rgb>;256]} // :211, app-set OSC overrides, for process-split renderers; not needed in M1
pub fn parse_color(&str)->Option<Rgb>  // :238, Ghostty config semantics: hex, X11 names, rgb:, rgbi:
pub fn parse_palette_entry(&str)->Option<(u8, Rgb)>  // :246, parses "N=#rrggbb"
```

- Setting defaults is also what makes OSC 10/11 queries answer correctly: tests/terminal.rs:98-110 show that the query reply stays empty until `set_default_colors` is called.
- Call them before the first frame and again on config reload.
- `CursorShape {Bar, Block, Underline, BlockHollow}` (render.rs:22). The unfocused-block case is the renderer's job.

---

## 6. PTY

### Candidates

| | cmux `cmux-pty` | `portable-pty` |
|---|---|---|
| Source | manaflow-ai/cmux `cmux-tui/crates/cmux-pty` @ `21dcc2b` (Cargo.toml + `src/lib.rs` 13.8 KB + `src/macos.rs` 29 KB) | crates.io, wezterm repo |
| Version | 0.1.0, `publish = false` | **0.9.0** (latest; crates.io API, updated 2025-02-11) |
| Licence | `license.workspace = true`, which is `MIT` (cmux-tui `Cargo.toml` `[workspace.package]`). No LICENSE file was fetched or checked. | MIT (Cargo.toml:26, `LICENSE.md`) |
| Deps | `anyhow`, `libc`, `portable-pty = "0.9"` | `anyhow`, `downcast-rs`, `filedescriptor`, `libc`, `log`, `nix` (term, fs), `serial2`, `shell-words` (+ windows-only) |
| API | `open(PtySize) -> PtyPair`, `PtyCommand::{new,args,cwd,cwd_descriptor,env,env_clear}`, `PtyPair::{spawn, try_clone_reader_descriptor}`, `SpawnedPty{master,child}`, re-exports `portable_pty::{Child, MasterPty, PtySize,..}` | `native_pty_system().openpty(PtySize) -> PtyPair{slave,master}`, `CommandBuilder`, `MasterPty::{resize, get_size, try_clone_reader, take_writer, as_raw_fd}`, `SlavePty::spawn_command`, `Child::{try_wait, wait, process_id, kill}` |

- `cmux-pty` is a thin layer over `portable-pty`. On Unix it replaces portable-pty's open and spawn with its own implementation (`src/macos.rs`, named for the Apple workaround but used for all Unix).
  - It adds fd-pinned cwd, avoids `ptsname`, duplicates the master fd for polling readers, and does descriptor cleanup.
  - These are hardening features for a multiplexer that spawns many PTYs.
- `portable-pty` `MasterPty` signatures (src/lib.rs:88-125):
  - `fn resize(&self, size: PtySize) -> Result<(), Error>`
  - `fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>, Error>`
  - `fn take_writer(&self) -> Result<Box<dyn Write + Send>, Error>` (once only; dropping it sends EOF)
  - `pub struct PtySize { pub rows: u16, pub cols: u16, pub pixel_width: u16, pub pixel_height: u16 }` (:63)
- `CommandBuilder::new_default_prog()` (cmdbuilder.rs:254):
  - Runs `$SHELL` if executable, else the passwd shell, else `/bin/sh` (get_shell :544-560).
  - **Run as a login shell** (argv0 `-zsh`, :510-520).
  - cwd defaults to `$HOME` (:503-508).
  - Inherits the whole environment. It does **not** set `TERM`/`COLORTERM`.
  - It calls `setsid` + `TIOCSCTTY` (unix.rs:257-272), so SIGWINCH works.

### Recommendation

**Use `portable-pty = "0.9"` directly for M1.**
- It is the crate cmux-pty wraps, MIT, on crates.io, and the same API works on macOS.
- It avoids vendoring a 43 KB module of unreviewed hardening code.
- We lose nothing M1 needs. Revisit `cmux-pty` when many tabs or splits stress fd handling.

### Snippet (spawn the user's shell, read on a thread, resize)

```rust
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

let size = |cols, rows, cw, ch| PtySize { rows, cols, pixel_width: cw, pixel_height: ch };
let pair = native_pty_system().openpty(size(80, 24, 0, 0))?;
let mut cmd = CommandBuilder::new_default_prog();          // login $SHELL, cwd = $HOME
cmd.env("TERM", "xterm-256color");                         // xterm-ghostty terminfo may be absent: UNVERIFIED
cmd.env("COLORTERM", "truecolor");
let child = pair.slave.spawn_command(cmd)?;               // Box<dyn Child + Send + Sync>
let master = pair.master;                                  // keep: resize + lifetime
drop(pair.slave);                                          // else reader never sees EOF
let reader = master.try_clone_reader()?;                   // -> reader thread (section 3)
let writer = master.take_writer()?;                        // -> writer thread fed by mpsc (keys, on_pty_write replies)
// on window resize, after term.resize(..):
master.resize(size(cols, rows, cell_w as u16, cell_h as u16))?;
```

- The master is `Send` but not shared across threads; keep it in the view and call `resize` from the UI thread.
- `child.kill()` or dropping the master on window close ends the shell.

---

## 7. Ghostty config

Source: `vendor/ghostty/src/config/` (manaflow-ai fork at `ef4f157`, post-1.3 because it uses `config.ghostty`).

### Format

- Line-based; parser is `cli/args.zig:1425-1490` (the LineIterator).
- Each line is trimmed (including `\r`). Blank lines and lines **starting with `#`** are skipped. There are **no inline comments**.
- The line is split on the **first `=`**; key and value are trimmed. A value wrapped in double quotes has them stripped (a Zig-escaped string for the full parser, `args.zig:543-560`).
- Repeatable keys accumulate: `font-family`, `font-feature`, `palette`, `config-file`, `keybind`.
  - Setting an empty value (`font-family = ""`) resets the list (Config.zig:150-160).
  - Palette lines are `palette = N=COLOR`. `N` may be decimal, `0x`, `0o` or `0b`. Config.zig:793 documents it.
- `config-file = path` (Config.zig:2504):
  - Paths are relative to the including file.
  - A `?` prefix makes a missing file non-fatal.
  - Cycles are errors.
  - Includes load **after** the including file's own content, so included values override it.
- Colours: `#RRGGBB`, `RRGGBB`, or X11 names. Use `ghostty_vt::parse_color` and `parse_palette_entry`, which implement exactly these semantics, so no custom colour parser is needed.
- `selection-*`, `cursor-color` and `cursor-text` also accept `cell-foreground` / `cell-background` (Config.zig:706, 850, 901).

### Default file

- `file_load.zig:12-60`: `$XDG_CONFIG_HOME/ghostty/config.ghostty` (preferred, Ghostty >= 1.3), else legacy `$XDG_CONFIG_HOME/ghostty/config`. `XDG_CONFIG_HOME` defaults to `~/.config`.
- macOS additionally prefers the `Application Support` directory (`file_load.zig:60-125`).

### Keys (Config.zig, with line numbers)

| Setting | Key | Type / default |
|---|---|---|
| Font family | `font-family` (:168), also `-bold/-italic/-bold-italic` | repeatable string; first is primary, the rest are fallbacks |
| Font size | `font-size` (:262) | f32; default 12 Linux, 13 macOS |
| Font features | `font-feature` (:244) | repeatable; `-calt` disables ligatures |
| Cell tweaks | `adjust-cell-width` / `adjust-cell-height` (:429/430) | `?MetricModifier` |
| Theme | `theme` (:592) | name, absolute path, or `light:X,dark:Y` pair (parser :9933-9973) |
| Background | `background` (:596) | colour; default `#282C34` |
| Foreground | `foreground` (:600) | colour; default `#FFFFFF` |
| Palette | `palette` (:793) | repeatable `N=COLOR` |
| Background opacity | `background-opacity` (:1002) | f64, clamped 0..1 |
| Opacity for cells with explicit bg | `background-opacity-cells` (:1019) | bool, default false |
| Blur | `background-blur` (:1048) | bool or int; Wayland uses `ext-background-effect-v1` (blur is deferred to M3) |
| Cursor style | `cursor-style` (:874) | `block \| bar \| underline \| block_hollow` |
| Cursor blink | `cursor-style-blink` (:893) | `?bool`; unset means the app/DEC mode 12 decides |
| Cursor colour | `cursor-color` (:850) | `?TerminalColor` |
| Cursor text colour | `cursor-text` (:901) | `?TerminalColor` |
| Cursor opacity | `cursor-opacity` (:857) | f64 |
| Selection colours | `selection-background` / `selection-foreground` (:707/706) | `?TerminalColor`; unset = inverted window fg/bg |
| Bold handling | `bold-color` (:3743) | colour or `bright` |
| Faint | `faint-opacity` (:3750) | f64, default 0.5 |
| Contrast | `minimum-contrast` (:775) | f64, default 1 |
| Scrollback | `scrollback-limit` (:1390) | bytes, default 50 MB |
| Copy on select | `copy-on-select` (:2442) | default `true` on Linux/macOS (selection clipboard) |
| Scroll speed | `mouse-scroll-multiplier` (:~985) | default 3 discrete / 1 precision |
| Includes | `config-file` (:2504) | repeatable path |
| TERM | `term` (:3760) | default `xterm-ghostty` |

### Theme lookup (config/theme.zig:8-66, 113+)

1. An absolute path is opened directly.
2. Otherwise search, in order: `$XDG_CONFIG_HOME/ghostty/themes/<name>` (default `~/.config/ghostty/themes/`), then `<resources dir>/themes/<name>`.
   - The resources dir is `$GHOSTTY_RESOURCES_DIR` if set (release builds), else detected relative to the executable. For a system install this is `/usr/share/ghostty` on Linux, or `Ghostty.app/Contents/Resources/ghostty` on macOS.
   - **On this dev machine `/usr/share/ghostty/themes/` exists with 819 themes.**
3. Names containing path separators are errors. Names are case-sensitive.
4. A theme file is an ordinary Ghostty config file with the same syntax; `theme` and `config-file` inside it are ignored. Anything set in the user's own config **overrides** the theme (Config.zig:575-580). So load order is: theme, then user config, then user includes.
5. Theme file content shape (checked on a stock theme): 16 `palette = N=#rrggbb` lines, then `background`, `foreground`, `cursor-color`, `cursor-text`, `selection-background`, `selection-foreground`.
6. For `light:X,dark:Y`, pick by the desktop appearance (`window.appearance()` / `cx.window_appearance()`, app.rs:1029).

### Default font (no `font-family` set)

Ghostty falls back to embedded JetBrains Mono (variable) plus a Symbols-only Nerd Font (`vendor/ghostty/src/font/embedded.zig:9-15`). `fc-list` on this machine shows **no JetBrains Mono installed**. For matching Ghostty's default look, M1 would have to bundle JetBrains Mono (OFL) and register it with `add_fonts`. **UNVERIFIED** that gpui's cosmic-text backend handles the variable font axes; use the static regular, bold, italic and bold-italic faces (embedded.zig:17-20 lists them).

### This machine's config

A Ghostty config exists at `$XDG_CONFIG_HOME/ghostty/config.ghostty`. It sets only these keys (values not recorded here):
- `theme`
- `background`
- `background-opacity`
- `background-opacity-cells`
- `background-blur`
- `clipboard-read`
- `shell-integration`
- `shell-integration-features`
- `notify-on-command-finish`
- `window-decoration`
- `window-padding-balance`
- `window-show-tab-bar`
- `window-theme`
- `gtk-titlebar-style`
- `gtk-toolbar-style`
- `gtk-wide-tabs`

It sets **no** font, cursor, palette or selection keys, so a screenshot pair would use Ghostty's default font (JetBrains Mono 12), the theme's palette, and `background` + `background-opacity`. M1 needs to support at least `theme`, `background`, `background-opacity` and `background-opacity-cells`. `clipboard-read` and `shell-integration*` are out of scope.

---

## Recommendations for the M1 plan

1. **Architecture.**
   - `Arc<Mutex<Term>>` shared between a PTY reader thread (`feed` under lock) and the gpui view. A PTY writer thread is fed by an `std::sync::mpsc` channel.
   - Both `Callbacks::on_pty_write` and keystrokes go to the writer channel (never write from inside a callback or from the reader thread).
   - Add `futures = "0.3"` for `mpsc::unbounded`. gpui only re-exports `smol::Timer`.
2. **PTY.** Use `portable-pty = "0.9"` (MIT) directly, not `cmux-pty`. `CommandBuilder::new_default_prog()` gives a login `$SHELL` in `$HOME`; set `TERM=xterm-256color` and `COLORTERM=truecolor`; `drop(pair.slave)` after spawn. Keep `MasterPty` in the view for `resize`.
3. **UI to repaint.** The reader thread sends a unit message per chunk. A `cx.spawn` task coalesces for about 4 ms (Zed's number), then `this.update(cx, |_, cx| cx.notify())`. Use `window.request_animation_frame()` or a timer for cursor blink.
4. **Custom `TerminalElement`** implementing `Element`.
   - `request_layout`: fill the parent.
   - `prepaint`: compute `cell_w = advance('m')` and `cell_h` from `ascent + |descent|` times a ratio, then `cols/rows = floor(bounds / cell)`. If changed, call `term.resize(cols, rows, cell_w_px, cell_h_px)` and `pty.resize(..)`. Then `render.update(&mut vt)`, `walk_rows(..)` (no clone), and build `LayoutRect`s and `BatchedTextRun`s (adapt Zed's `BackgroundRegion` and `BatchedTextRun`; break batches at non-contiguous columns, wide cells, and style changes).
   - `paint`: background quad, bg rects, selection quads, text runs via `shape_line(.., Some(cell_w)).paint(..)`, IME preedit, cursor. Call `window.handle_input(..)` here.
5. **Do not use `StyledRun`/`rows_to_runs`/`build_frame` in the hot path.** They clone cells, drop column positions, and do not swap inverse. Use `walk_rows` with our own batching. Per-row `dirty` can key a `ShapedLine` cache later.
6. **Colour pipeline.** Apply Ghostty defaults with `replace_default_colors(fg, bg, cursor)` and `set_default_palette(&[Option<Rgb>;256])` (and `set_default_cursor`) before the first frame, so OSC 10/11 queries answer correctly. In the renderer: `None` means default, swap on `inverse`, dim on `faint` (`faint-opacity`), skip `invisible`, apply `bold-color`, and skip bg quads for default-bg cells (needed for `background-opacity`).
7. **Keyboard.**
   - Root `div().track_focus().key_context("Terminal").on_key_down(..).on_key_up(..)`.
   - Map `Keystroke` to `KeyInput` with our own key-name table (do not use `key_input_from_chord`). Call `KeyEncoder::sync_from_terminal` then `encode` to get bytes, which gives kitty protocol and DECCKM for free.
   - If bytes were produced, `cx.stop_propagation()` and `scroll_to_bottom()`.
   - Implement `InputHandler` directly (Zed style) for IME: commit writes UTF-8 to the PTY, preedit is painted at the cursor, and `bounds_for_range` returns the cursor rect.
8. **Mouse.**
   - If `Terminal::mouse_tracking()` and Shift is not held, build `MouseInput` and call `MouseEncoders::{encode, encode_release}` (call `sync_from_terminal` first). Otherwise run local selection.
   - Selection is app-level `SelectionRange` in absolute rows (`scrollbar().offset + viewport_row`), using `select_word_screen` / `select_line_screen` for double and triple click, and `selection_text_absolute` for copy. The wrapper has no selection state.
   - Wheel: accumulate pixels to lines (Zed `determine_scroll_lines`), then `scroll_delta(-lines)`.
9. **Clipboard.** Ctrl+Shift+C/V use `write_to_clipboard` / `read_from_clipboard`. Selection release calls `write_to_primary` and middle-click calls `read_from_primary` (Ghostty's `copy-on-select` default on Linux). Gate on `cfg(linux)`. Paste goes through `sys::ghostty_paste_encode` with `bracketed = term.mode(2004, false)`. Send focus in/out via `sys::ghostty_focus_encode` when `mode(1004, false)`.
10. **Config.** Write a small `ghostty_config` module (about 100 LOC) with the line format above and `config.ghostty` then legacy `config` lookup. Parse colours with `ghostty_vt::parse_color` / `parse_palette_entry`. Resolve `theme` via `$XDG_CONFIG_HOME/ghostty/themes`, then `$GHOSTTY_RESOURCES_DIR/themes`, then `/usr/share/ghostty/themes`. Handle `light:/dark:` pairs. Layer theme, then user file, then includes. M1 keys: `font-family`, `font-size`, `font-feature`, `theme`, `background`, `foreground`, `palette`, `background-opacity`, `background-opacity-cells`, `cursor-*`, `selection-*`, `bold-color`, `faint-opacity`, `minimum-contrast`, `scrollback-limit`, `config-file`.
11. **Fonts.** Default to Ghostty's JetBrains Mono. Bundle the static JetBrains Mono faces, register them with `cx.text_system().add_fonts`, and disable `calt` via `FontFeatures::disable_ligatures()` unless `font-feature` enables it. The spec's "ligatures if the engine reports them" needs a decision. Zed disables by default, and `force_width` breaks under ligatures.
12. **Carry-to-M1 items from STATUS.md.**
    - Wire `Callbacks` in `Term::new` (currently `Callbacks::default()`).
    - Expose `resize`, `scroll_delta`, `scrollbar` and `selection_*` on `Term`.
    - Replace `Snapshot`'s clones with a `walk_rows`-based API (a closure or a reusable layout buffer).
    - Pin the toolchain.
    - 8-bit C1 OSC scanner gap.

### UNVERIFIED items the plan must handle

- Exact Ghostty cell height: gpui does not expose `line_gap`, and `descent` is negated on Linux with a "todo confirm". This affects the "colours/fonts match Ghostty" screenshot criterion. Plan a calibration step and an `adjust-cell-height` knob.
- Whether `Terminal::resize` wants logical or device pixels for `cell_*_px`.
- Whether libghostty-vt (not the app) converts wheel events to arrow keys in the alt screen (mode 1007). The renderer may need to do it.
- Wide glyphs and ligatures against `force_width`. Our batching must break runs at wide cells, and ligatures conflict with a pixel-exact grid.
- Whether gpui's cosmic-text backend handles variable fonts. Use static faces.
- `xterm-ghostty` terminfo availability. M1 uses `xterm-256color`; kitty keyboard still works because it is negotiated by escape sequence.
- macOS key behaviour (`macos_option_as_alt`, `apple_press_and_hold_enabled`) was not examined.
- `cursor.rs` in Zed's `terminal_view` (CursorLayout) was not read.
- `Term`/`Terminal`/`RenderState`/`KeyEncoder`/`MouseEncoders` `Send` was confirmed by compiling a throwaway test. `Sync` is not provided.
