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
- **Thumbnails** — downscaled disk cache (HEIC / RAW embeds / video posters on macOS via `qlmanage` / `sips`)
- **Lightbox** — zoom, pan, slideshow; HEIC/RAW show a JPEG preview
- **Stars** — favorite files, persist, filter (`Stars` chip or ⌘⇧F)
- **Theme** — Dark / Light / System (toolbar or ⌘⇧T)
- **Video** — poster in-grid; lightbox Play opens the system player (or skip the lightbox when “Video: system”)

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
| Space | Peek · close peek · pause GIF · play video · next photo |
| ⌘⇧S | Star / unstar |
| ⌘⇧F | Stars filter |
| ⌘⇧T | Cycle theme |
| ⌘⇧V | Video stay / system |
| `1` `2` `3` | Density |
| ⌘Q | Quit |

## Formats

JPEG, PNG, GIF, WebP, TIFF, BMP load natively. **HEIC/HEIF**, **RAW** (embedded JPEG), **AVIF/JXL**, and **video posters** use Quick Look (`qlmanage -t`) then `sips` on macOS. If neither can decode a file, the tile stays empty instead of crashing. GIF/WebP animate in the lightbox; Space pauses on the first frame.
