# Agents

Short rules for working on this repo.

## Workflow

1. **Issue first** — open or pick a GitHub issue before coding. No drive-by features.
2. **Batch issues** — one PR for a related set of issues (not one giant PR, not one issue per typo).
3. **Stack PRs** — sequential work is stacked (each PR targets the previous branch). Merge bottom-up.
4. **PR shape** — about 70% what/why (purpose and outcome), 30% technical detail.
5. **Link issues** — reference `#N` in commits/PR body; close with `Fixes #N` when done.
6. **Stay minimal** — smallest change that solves the issue. No speculative abstractions.
7. **Commit often** — many small commits with a clear why. Prefer a trail of reviewable steps over one dump.

## Before big feature work

Do a **structure pass** first (see the refactor issue). Prefer scoped modules over a growing `main.rs`.

## Layout

```
src/
  main.rs          # process entry only
  app.rs           # window, menus, keys, folder resolve
  app/             # tray / process chrome
  gallery.rs       # entity, actions, load/nav
  gallery/         # density, viewer, grid, lightbox, shell view
  media.rs         # types / scan / thumbs re-exports
  media/           # entry types, folder walk, thumb cache
  prefs.rs         # saved / recents / flags
  ui.rs            # shared chrome re-exports
  ui/              # theme tokens, button, sidebar row
```

Group by feature. Keep UI pieces next to the feature that owns them. Shared-only code goes in `ui/`. When a file grows, add `foo/*.rs` beside `foo.rs`.

## Code quality

- Feature folders, not technical layers. New code lands in the feature that owns it; `main.rs` stays thin wiring.
- Prefer `foo.rs` + `foo/*.rs` (not `mod.rs`). One responsibility per file; split when a file mixes concerns or grows past a screen or two.
- Keep the crate surface small (`pub(crate)` by default). No god files, no traits or abstractions for a single use.
- Shared chrome and color tokens live in `ui/`. Feature-specific UI stays with that feature.
- rustfmt + clippy clean on your diff. Match existing style; don't "improve" unrelated code in a feature PR.

## Commits

- Clear why, not noise.
- Do not add Cursor/AI attribution trailers.
