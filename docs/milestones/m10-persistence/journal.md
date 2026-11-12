# m10-persistence — Journal

Append-only, newest last. One entry per session: date, what was tried, what was learned, decisions and deviations from the Swift app.

## 2026-11-11 — plan

- Read cmux @ `88b87c05`: `SessionPersistence.swift` (8 s autosave, snapshot shape, restore skipped on launch args or `CMUX_DISABLE_SESSION_RESTORE`), the Claude hook session record, `shouldAutoResumeNormallyEndedClaude`, `docs/vault.md`, `docs/local-tmux.md`. `docs/agent-recovery.md` is Subrouter's `cmux recover`, not ported.
- Wrote `plan.md`: three parts in spec order. Deviations from cmux, all in the plan: no scrollback or notification persistence; custom agents in `config.toml` with argv-option ids only; local-tmux without a registry or server incarnation marker; no Vault window.
