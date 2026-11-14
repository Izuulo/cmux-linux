#!/usr/bin/env python3
"""Scripted click-through of cmux-linux's UI (M15), off the owner's screen.

Runs a debug build (`cargo build -p cmux-app -p cmux-ipc`) inside a headless KWin
(`kwin_wayland --virtual`), with its own state and config dirs, no session D-Bus (no desktop
bubbles) and `CMUX_SOCKET_MODE=allowAll`. Each check clicks real controls through
`debug.mouse.simulate`, finds them with `debug.ui.bounds` (gpui element ids, debug builds
only) and reads the result back over the socket or from disk.

Usage: scripts/click-through.py   (from the repo root; prints one line per check, exit 1 on a fail)
"""

import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
APP = os.path.join(ROOT, "target/debug/cmux-app")
RUNTIME = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
DISPLAY_NAME = f"wayland-cmux-click-{os.getpid()}"

# A short dir: the socket path must fit sun_path.
WORK = tempfile.mkdtemp(prefix="cmux-ct-", dir=RUNTIME)
SOCK = os.path.join(WORK, "state/cmux-linux/cmux.sock")
CFG = os.path.join(WORK, "cfg")


def rpc(method, **params):
    s = socket.socket(socket.AF_UNIX)
    s.settimeout(30)
    s.connect(SOCK)
    s.sendall((json.dumps({"id": 1, "method": method, "params": params}) + "\n").encode())
    buf = b""
    while not buf.endswith(b"\n"):
        chunk = s.recv(65536)
        if not chunk:
            break
        buf += chunk
    s.close()
    reply = json.loads(buf)
    if not reply.get("ok"):
        raise RuntimeError(f"{method}: {reply.get('error')}")
    return reply["result"]


def elements(match=""):
    return rpc("debug.ui.bounds", match=match)["elements"]


def find(suffix):
    """The element whose id path ends with `.suffix` (the last one drawn wins)."""
    hits = [e for e in elements(suffix) if e["id"].endswith("." + suffix) or e["id"] == suffix]
    return hits[-1] if hits else None


def wait_for(pred, timeout=3.0):
    end = time.time() + timeout
    while time.time() < end:
        v = pred()
        if v:
            return v
        time.sleep(0.1)
    return pred()


def click(suffix, **kw):
    e = wait_for(lambda: find(suffix))
    if not e:
        raise RuntimeError(f"no element {suffix!r} on screen")
    rpc("debug.mouse.simulate", x=e["x"] + e["w"] / 2, y=e["y"] + e["h"] / 2, **kw)
    time.sleep(0.4)


def key(combo):
    rpc("debug.shortcut.simulate", combo=combo)
    time.sleep(0.4)


def current():
    return rpc("workspace.current")["workspace"]["ref"]


def panes(ws=None):
    return rpc("pane.list", **({"workspace_id": ws} if ws else {}))["panes"]


def root_size():
    e = [e for e in elements() if e["id"].count(".") == 0][0]
    return e["w"], e["h"]


# ---- checks: each returns a short detail string or raises -------------------------------


def sidebar_row():
    rpc("workspace.create", title="two", focus=True)
    assert current() == "workspace:2"
    click("ws-1")
    assert current() == "workspace:1", current()
    return "row click selects workspace:1"


def sidebar_new():
    n = len(rpc("workspace.list")["workspaces"])
    click("sidebar-new")
    m = wait_for(lambda: len(rpc("workspace.list")["workspaces"]) == n + 1 and n + 1)
    assert m, "no new workspace"
    click("ws-1")
    return f"{n} -> {n + 1} workspaces"


def sidebar_toggle():
    click("sidebar-toggle")
    assert wait_for(lambda: not find("ws-list")), "list still shown"
    key("super+b")
    assert wait_for(lambda: find("ws-list")), "list not back"
    return "hides the list"


def strip_buttons():
    before = len(panes())
    click("strip-1-SplitRight")
    assert wait_for(lambda: len(panes()) == before + 1), "no split"
    p = [p for p in panes() if p["focused"]][0]
    n = p["surface_count"]

    def count():
        return [q for q in panes() if q["ref"] == p["ref"]][0]["surface_count"]

    # Element ids carry the pane's own key, not its socket ref: the new pane is rightmost.
    new_tab = max(elements("-NewTerminal"), key=lambda e: e["x"])
    pane_key = new_tab["id"].rsplit(".", 1)[-1].split("-")[1]
    rpc("debug.mouse.simulate", x=new_tab["x"] + new_tab["w"] / 2,
        y=new_tab["y"] + new_tab["h"] / 2)
    assert wait_for(lambda: count() == n + 1), "no new tab"
    close = [e for e in elements(f"tabs-{pane_key}.") if ".tab-close-" in e["id"]]
    assert close, "no tab close button"
    e = close[-1]
    rpc("debug.mouse.simulate", x=e["x"] + e["w"] / 2, y=e["y"] + e["h"] / 2)
    assert wait_for(lambda: count() == n), "tab not closed"
    return "split right, new tab, close tab"


def bell_popover():
    rpc("notification.create_for_target", workspace_id="workspace:2",
        surface_id=rpc("surface.list", workspace_id="workspace:2")["surfaces"][0]["ref"],
        title="Hi", body="there")
    click("ws-1")
    click("sidebar-bell")
    assert wait_for(lambda: find("notif-panel")), "popover not open"
    rows = [e for e in elements("notif-row-") if e["id"].rsplit(".", 1)[-1].startswith("notif-row-")]
    assert rows, "no notification row"
    e = rows[0]
    rpc("debug.mouse.simulate", x=e["x"] + e["w"] / 2, y=e["y"] + e["h"] / 2)
    assert wait_for(lambda: current() == "workspace:2"), "row click did not jump"
    assert wait_for(lambda: not find("notif-panel")), "popover stayed open"
    click("ws-1")
    return "opens under the bell; a row jumps to its workspace"


def context_menu():
    groups = len(rpc("workspace.group.list")["groups"])
    click("ws-1", button="right")
    assert wait_for(lambda: find("context-menu")), "menu not open"
    click("menu-item-1")  # New Group from Workspace
    assert wait_for(lambda: len(rpc("workspace.group.list")["groups"]) == groups + 1), "no group"
    assert not find("context-menu"), "menu stayed open"
    return "right-click opens; New Group from Workspace makes a group"


def group_header():
    g = rpc("workspace.group.list")["groups"][0]
    was = g["is_collapsed"]
    heads = [e for e in elements("ws-group-chevron")]
    assert heads, "no group chevron"
    e = heads[0]
    rpc("debug.mouse.simulate", x=e["x"] + e["w"] / 2, y=e["y"] + e["h"] / 2)
    assert wait_for(lambda: rpc("workspace.group.list")["groups"][0]["is_collapsed"] != was), "no toggle"
    rpc("debug.mouse.simulate", x=e["x"] + e["w"] / 2, y=e["y"] + e["h"] / 2)
    wait_for(lambda: rpc("workspace.group.list")["groups"][0]["is_collapsed"] == was)
    return "chevron collapses and expands"


def palette():
    before = len(panes())
    key("super+shift+p")
    assert wait_for(lambda: find("palette-row-2")), "palette not open"
    click("palette-row-2")  # Split Right
    assert wait_for(lambda: len(panes()) == before + 1), "row did not run"
    assert not find("command-palette"), "palette stayed open"
    return "row 2 (Split Right) runs and closes"


def right_sidebar_and_dock():
    key("super+shift+e")
    assert wait_for(lambda: find("right-tab-dock")), "right sidebar not open"
    click("right-tab-dock")
    click("dock-new-terminal")
    assert wait_for(lambda: panes("window:1")), "no Dock pane"
    click("right-tab-feed")
    assert wait_for(lambda: find("feed-list") or not find("dock-new-terminal")), "Feed tab"
    return "Dock tab, New Terminal, Feed tab"


def feed_card():
    ws = rpc("workspace.current")["workspace_id"]
    surf = rpc("surface.list")["surfaces"][0]["id"]
    ev = {"hook_event_name": "PermissionRequest", "session_id": "s", "cwd": "/tmp",
          "tool_name": "Bash", "tool_input": {"command": "ls"}, "_source": "claude",
          "workspace_id": ws, "surface_id": surf, "_opencode_request_id": "REQ-CT"}
    out = {}
    t = threading.Thread(
        target=lambda: out.update(r=rpc("feed.push", wait_timeout_seconds=20, event=ev))
    )
    t.start()
    key("super+shift+e")
    if not find("feed-list"):
        click("right-tab-feed")
    btn = wait_for(lambda: [e for e in elements("feed-card") if e["id"].endswith(".Allow Once")])
    assert btn, "no Allow Once button"
    e = btn[-1]
    rpc("debug.mouse.simulate", x=e["x"] + e["w"] / 2, y=e["y"] + e["h"] / 2)
    t.join(10)
    r = out.get("r", {})
    assert r.get("status") == "resolved", r
    return f"Allow Once -> {json.dumps(r.get('decision'))}"


def window_buttons():
    w0 = root_size()
    click("window-zoom")
    assert wait_for(lambda: root_size() != w0), "zoom did not maximize"
    click("window-zoom")
    assert wait_for(lambda: root_size() == w0), "zoom did not restore"
    t = find("titlebar")
    rpc("debug.mouse.simulate", x=t["x"] + 200, y=t["y"] + t["h"] / 2, click_count=2,
        kind="click")
    assert wait_for(lambda: root_size() != w0), "title bar double-click did not maximize"
    return f"zoom button and title bar double-click: {w0} <-> {root_size()}"


def read_cfg(*names):
    for n in names:
        try:
            with open(os.path.join(CFG, n)) as f:
                return f.read()
        except FileNotFoundError:
            pass
    return ""


def ghostty_cfg():
    return read_cfg("ghostty/config.ghostty", "ghostty/config")


def settings_controls():
    key("super+,")
    assert wait_for(lambda: find("settings-detail")), "Settings did not open"
    click("ui-scale-1")
    assert wait_for(lambda: "ui_scale" in read_cfg("cmux-linux/config.toml")), "ui scale not saved"
    click("Terminal")
    click("background-blur")
    assert wait_for(lambda: "background-blur" in ghostty_cfg()), "blur switch not saved"
    click("cursor-style-1")
    assert wait_for(lambda: "cursor-style" in ghostty_cfg()), "cursor style not saved"
    return "UI scale +, blur switch, cursor style write their files"


def settings_field_survives_sigterm(app):
    """Last: ends the app. A half-typed field is saved by the quit (M15 Task 3)."""
    click("Automation")
    click("auto-naming-command")
    for ch in "echo ct":
        rpc("debug.shortcut.simulate", combo="space" if ch == " " else f"{ch}->{ch}")
    time.sleep(0.4)
    app.send_signal(signal.SIGTERM)
    app.wait(10)
    text = read_cfg("cmux-linux/config.toml")
    assert "echo ct" in text, text
    return "typed command saved on SIGTERM"


CHECKS = [sidebar_row, sidebar_new, sidebar_toggle, strip_buttons, bell_popover,
          context_menu, group_header, palette, right_sidebar_and_dock, feed_card,
          window_buttons, settings_controls]


def main():
    if not os.path.exists(APP):
        sys.exit(f"build first: cargo build -p cmux-app -p cmux-ipc ({APP} missing)")
    os.makedirs(CFG)
    shell = os.path.join(WORK, "sh")
    with open(shell, "w") as f:
        f.write('#!/bin/sh\nexec bash --noprofile --norc "$@"\n')
    os.chmod(shell, 0o755)
    kwin = subprocess.Popen(
        ["kwin_wayland", "--virtual", "--socket", DISPLAY_NAME, "--width", "1400",
         "--height", "900"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    env = {k: v for k, v in os.environ.items() if not k.startswith("CMUX")}
    env.update(WAYLAND_DISPLAY=DISPLAY_NAME, DISPLAY="",
               DBUS_SESSION_BUS_ADDRESS="unix:path=/nonexistent",
               XDG_STATE_HOME=os.path.join(WORK, "state"), XDG_CONFIG_HOME=CFG,
               CMUX_SOCKET_MODE="allowAll", SHELL=shell)
    app = None
    failed = 0
    try:
        wait_for(lambda: os.path.exists(os.path.join(RUNTIME, DISPLAY_NAME)), 10)
        app = subprocess.Popen([APP], env=env, stdout=subprocess.DEVNULL,
                               stderr=open(os.path.join(WORK, "app.log"), "w"))
        if not wait_for(lambda: os.path.exists(SOCK) and find("ws-1"), 20):
            sys.exit("app did not come up (see app.log)")
        checks = CHECKS + [lambda: settings_field_survives_sigterm(app)]
        names = [c.__name__ for c in CHECKS] + ["settings_field_survives_sigterm"]
        for name, check in zip(names, checks):
            try:
                print(f"PASS {name}: {check()}", flush=True)
            except Exception as e:  # one failing check must not hide the others
                failed += 1
                print(f"FAIL {name}: {e}", flush=True)
                try:
                    key("escape")
                except Exception:
                    pass
    finally:
        for p in (app, kwin):
            if p:
                p.send_signal(signal.SIGTERM)
                try:
                    p.wait(5)
                except subprocess.TimeoutExpired:
                    p.kill()
        shutil.rmtree(WORK, ignore_errors=True)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
