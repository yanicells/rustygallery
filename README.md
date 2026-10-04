# rusty gallery

<img src="assets/icon/app.png" width="72" alt="gallery icon: a rust tile with a centered cream ring" />

A fast, minimal photo/video gallery built with [GPUI](https://www.gpui.rs).

## Run

```bash
cargo run --release
cargo run --release -- ~/Pictures
./scripts/bundle-macos.sh   # writes dist/gallery.app and dist/gallery.app.zip
```

Opens the first available recent or saved folder when no usable path is passed. Otherwise it uses the checkout's `media` directory if present, then Pictures or your home directory. Unavailable libraries and stars remain saved for when their drive reconnects.

The `.app` lives in the menu bar; Show/Hide, Open Folder, Saved, Recents, and a random starred preview are on the status item. Closing the window hides the app; Show restores it. Quit exits.

## Install locally on macOS

With Rust and Xcode's Metal toolchain installed, run the bundle script above, then:

```bash
ditto dist/gallery.app /Applications/gallery.app
open /Applications/gallery.app
```

The script derives the version from Cargo, builds with the lockfile, validates the plist, ad-hoc signs and verifies the completed app, and creates a zip. This is a local build, without Developer ID signing or notarization. No public release or Homebrew download is published by this script; `scripts/gallery.rb` is a future release template.

## Keys

Bindings live in `~/Library/Application Support/rusty-gallery/keys.txt` (created on first launch). Format is documented in [KEYS.md](KEYS.md). The file reloads when it changes.

## What it does

- **Folder browse** — subfolders appear as tiles; click to enter · Back / ⌘↑ to go up
- **Large libraries** — virtualized rows and bounded thumbnail work prioritize the visible grid
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
| Save / File → Save Library | Pin current library |
| ⌘D | Duplicate selected files |
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

JPEG, PNG, GIF, WebP, TIFF, BMP load natively. **HEIC/HEIF**, **RAW** (embedded JPEG), **AVIF/JXL**, and **video posters** use Quick Look (`qlmanage -t`) then `sips` on macOS. Failed previews show an unavailable state; converters have deadlines so broken files cannot leave loading stuck indefinitely. GIF/WebP animate in the lightbox; Space pauses on the first frame.

## Platforms

| OS | Status |
| --- | --- |
| macOS | Primary. CI runs tests and validates the local `.app` bundle. |
| Linux | CI builds the binary (Wayland/X11). |
| Windows | CI job exists and is allowed to fail until GPUI-on-Windows is solid. |

About (gallery menu) shows the Cargo version and a link to the repo.
