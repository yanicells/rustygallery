# gallery

A fast, minimal photo/video gallery built with [GPUI](https://www.gpui.rs).

## Run

```bash
cargo run --release
cargo run --release -- ~/Pictures
```

Defaults to `./media` when no folder is passed.

## Features

- Recursive folder scan (async, non-blocking)
- Downscaled thumbnail cache
- Folder picker (`Open` or ⌘O)
- Grid density S / M / L (`1` `2` `3`)
- Keyboard grid focus (arrows) · Enter / Space to open
- Lightbox zoom (scroll) · pan (drag) · double-click / `0` to reset
- Slideshow (`S` or toolbar) — 3s interval
- Videos open in the system player

## Controls

| Input | Action |
| --- | --- |
| Click / Enter / Space | Open |
| ← → ↑ ↓ | Focus grid / navigate lightbox |
| Esc | Close lightbox |
| Scroll | Zoom |
| Drag | Pan (when zoomed) |
| Double-click / `0` | Reset zoom |
| `S` | Toggle slideshow |
| `1` `2` `3` | Density |
| ⌘O | Open folder |
| ⌘Q | Quit |
