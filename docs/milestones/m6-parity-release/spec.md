# M6 parity-release — Spec

**Goal:** it feels like the Mac app, and it's installable.

## Scope
- Side-by-side visual parity pass against the Swift app: spacing, colours, ring animation, sidebar, tab strip.
- macOS build and smoke test.
- Linux packaging: AUR PKGBUILD (CachyOS/Arch) and a `.desktop` file.
- Performance check: 20 panes with busy output stay smooth; memory reasonable.

## Out of scope
Unscheduled "later" features in `docs/design.md`.

## Exit criteria
1. Parity checklist in the journal fully ticked or each gap explained.
2. Installs from the PKGBUILD on CachyOS; runs on macOS.
3. Performance targets met and recorded.
