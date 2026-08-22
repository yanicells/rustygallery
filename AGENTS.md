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

## Code quality

- Feature folders, not technical layers. New code lands in the feature that owns it; `main.rs` stays thin wiring.
- Prefer `foo.rs` + `foo/*.rs` (not `mod.rs`). One responsibility per file; split when a file mixes concerns or grows past a screen or two.
- Keep the crate surface small (`pub(crate)` by default). No god files, no traits or abstractions for a single use.
- Shared chrome and color tokens live in `ui/`. Feature-specific UI stays with that feature.
- rustfmt + clippy clean on your diff. Match existing style; don't "improve" unrelated code in a feature PR.

## Commits

- Clear why, not noise.
- Do not add Cursor/AI attribution trailers.
