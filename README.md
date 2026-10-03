# rusty gallery

<img src="assets/icon/app.png" width="72" alt="gallery icon: a rust tile with a peeking lens" />

A fast, minimal photo/video gallery built with [GPUI](https://www.gpui.rs).

## Run

```bash
cargo run --release
cargo run --release -- ~/Pictures
./scripts/bundle-macos.sh   # writes dist/gallery.app (menu-bar app, no Dock icon)
```

Opens the last recent folder when no path is passed (falls back to `./media`). The `.app` lives in the menu bar; Show/Hide, Saved, Recents, and a random starred preview are on the status item. `brew install --cask ./scripts/gallery.rb` is the Homebrew shape once a release zip exists.

## Keys

Bindings live in `~/Library/Application Support/rusty-gallery/keys.txt` (created on first launch). Format is documented in [KEYS.md](KEYS.md). The file reloads when it changes.

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

## Platforms

| OS | Status |
| --- | --- |
| macOS | Primary. `.app` bundle + menu-bar tray. |
| Linux | CI builds the binary (Wayland/X11). |
| Windows | CI job exists and is allowed to fail until GPUI-on-Windows is solid. |

About (gallery menu) shows the Cargo version and a link to the repo.
