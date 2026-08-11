# Agents

Short rules for working on this repo.

## Workflow

1. **Issue first** — open or pick a GitHub issue before coding. No drive-by features.
2. **PR per batch** — one PR for a related set of issues (not one giant PR, not one commit-per-typo spam either).
3. **Link issues** — reference `#N` in commits/PR body; close with `Fixes #N` when done.
4. **Stay minimal** — smallest change that solves the issue. No speculative abstractions.

## Before big feature work

Do a **structure pass** first (see the refactor issue). Prefer scoped modules over a growing `main.rs`.

## Layout (target)

```
src/
  main.rs          # entry + window/menu wiring only
  app/             # top-level app state / shell
  gallery/         # grid, browse, lightbox
  media/           # scan, entry types, formats
  prefs/           # saved/recents/settings
  ui/              # shared chrome (sidebar, buttons, theme tokens)
```

Group by feature. Keep UI pieces next to the feature that owns them. Shared-only code goes in `ui/`.

## Commits

- Clear why, not noise.
- Do not add Cursor/AI attribution trailers.
