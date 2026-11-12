# M13 browser — Spec

**Goal:** web pages next to terminals, like cmux's browser panes.

## Scope
- A browser surface that lives in a pane or tab like a terminal: URL bar, back, forward, reload,
  page title as tab title.
- Opening: a new-browser-tab shortcut, clicking a link in a terminal (configurable: browser tab
  or system browser), `cmux browser open <url>` over the socket.
- Engine chosen at milestone start (risk: gpui has no web view; candidates include WebKitGTK
  and CEF). Record the decision and its licence in `NOTICE`.
- Persisted with the layout (M10's `session.json`): URL per browser surface.

## Out of scope
- Cookie import from other browsers, cmux Cloud, Manaflow services, cmux's remote browser
  automation commands beyond `open`.

## Exit criteria
1. A browser tab opens next to a terminal, loads a page, and back/forward/reload work.
2. A link clicked in a terminal and `cmux browser open <url>` both land in a browser tab.
3. After quit and relaunch the browser tab comes back at its URL.
