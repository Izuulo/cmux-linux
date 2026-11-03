# cmux-linux

Rust port of cmux's terminal workspace for Linux (KDE Wayland) and macOS.
Start every session by reading `STATUS.md`, then the current milestone's docs.

## Docs rules
- `STATUS.md` is the permanent handoff: current milestone, next action,
  decisions, risks, blockers. **Max 100 lines.** Update it at the end of every
  session that changes state.
- Before replacing `STATUS.md`, append its current content to
  `docs/status-archive.md` under `## YYYY-MM-DD — <short label>`. Never edit
  past archive entries.
- Each milestone lives in `docs/milestones/<id>/`:
  - `spec.md`: goal, scope, exit criteria. Change only with the user's OK.
  - `plan.md`: implementation plan, written at milestone start.
  - `journal.md`: append-only session notes: what was tried, learned, decided,
    and any deviation from the Swift app.
- Project-wide design: `docs/design.md`.

## Code rules
- License GPL-3.0-or-later. Never copy code from cmux directories under BUSL-1.1
  (check the directory's LICENSE first).
- When behaviour or visuals are unclear, read cmux's Swift source and match it.
- ⌘ in cmux maps to Super here; every binding must be remappable.
- Adding a dependency or vendored code? Add it and its licence to `NOTICE`.
- No personal info (names, emails, home paths, machine details) in tracked files.

## Dev setup
- `scripts/install-zig.sh` once, then `export PATH="$PWD/.tools/zig:$PATH"` from the repo root before any cargo command (Ghostty's build calls a bare `zig`).
