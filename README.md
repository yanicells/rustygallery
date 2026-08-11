# rusty gallery

A fast, minimal photo/video gallery built with [GPUI](https://www.gpui.rs).

## Run

```bash
cargo run --release
cargo run --release -- ~/Pictures
```

Opens the last recent folder when no path is passed (falls back to `./media`).

## What it does

- **Folder browse** — subfolders appear as tiles; click to enter · Back / ⌘↑ to go up
- **Flat mode** — show every nested media file in one grid (`F` or Folders/Flat toggle)
- **Open Folder** — big button in the sidebar (also ⌘O)
- **Saved + Recent** — pin libraries, jump back without re-picking
- **Thumbnails** — downscaled disk cache
- **Lightbox** — zoom, pan, slideshow
- Videos open in the system player

## Controls

| Input | Action |
| --- | --- |
| Open Folder / ⌘O | Pick a library |
| ← Back / ⌘↑ / Backspace | Parent folder |
| Save / ⌘D | Pin current library |
| Folders / Flat / `F` | Browse vs recursive |
| Click / Enter / Space | Open folder or media |
| ← → ↑ ↓ | Focus grid / navigate lightbox |
| Esc | Close lightbox |
| Scroll / drag | Zoom / pan |
| `S` | Slideshow |
| `1` `2` `3` | Density |
| ⌘Q | Quit |
