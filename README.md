# rusty gallery

<img src="assets/icon/app.png" width="72" alt="gallery icon: a rust tile with a centered cream ring" />

A fast, minimal photo/video gallery built with [GPUI](https://www.gpui.rs).

## Requirements on macOS

- macOS 13 or later.
- Rust's stable toolchain and Cargo.
- Python 3, used to generate the app icon and bundle metadata.
- Xcode with its Metal toolchain. Check it with `xcrun -sdk macosx metal --version`. If Metal is missing, run `xcodebuild -downloadComponent MetalToolchain` from an Xcode installation.

## Run locally

Clone the repository, or use your existing checkout:

```bash
git clone https://github.com/yanicells/rustygallery.git
cd rustygallery
```

For development, run the app directly with Cargo:

```bash
cargo run --locked
cargo run --locked -- ~/Pictures
```

For optimized playback and large-library browsing:

```bash
cargo run --release --locked -- ~/Pictures
```

Opens the first available recent or saved folder when no usable path is passed. Otherwise it uses the checkout's `media` directory if present, then Pictures or your home directory. Unavailable libraries and stars remain saved for when their drive reconnects.

The `.app` lives in the menu bar; Show/Hide, Open Folder, Saved, Recents, and a random starred preview are on the status item. Closing the window hides the app; Show restores it. Quit exits.

Hiding or closing the window stops video audio and the slideshow. Showing a selected video leaves it paused; press Play to reopen playback.

## Build the macOS app

From the repository root:

```bash
./scripts/bundle-macos.sh
open dist/gallery.app
```

This creates `dist/gallery.app` and `dist/gallery.app.zip`. To install the app locally:

```bash
ditto dist/gallery.app /Applications/gallery.app
open /Applications/gallery.app
```

The script derives the version from Cargo, builds with the lockfile, validates the plist, ad-hoc signs and verifies the completed app, and creates a zip. This is a local build, without Developer ID signing or notarization. No public release or Homebrew download is published by this script; `scripts/gallery.rb` is a future release template.

To build just the executable, run `cargo build --release --locked --bin gallery`; the result is `target/release/gallery`.

## Development checks

```bash
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

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
- **Video** — poster in-grid; native in-app playback on macOS starts paused, with seek, mute, volume, replay, and fullscreen controls. “System player” opens externally; “Video: system” skips the lightbox. Unsupported files or platforms show a clear failure with that fallback.

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
| Shift ← / → | Seek video backward / forward 5 seconds |
| ↑ / ↓ in video | Volume up / down 5% |
| `M` in video | Mute / unmute |
| ⌘⇧S | Star / unstar |
| ⌘⇧F | Stars filter |
| ⌘⇧T | Cycle theme |
| ⌘⇧V | Video stay / system |
| `1` `2` `3` | Density |
| ⌘Q | Quit |

## Formats

JPEG, PNG, GIF, WebP, TIFF, BMP load natively. **HEIC/HEIF**, **RAW** (embedded JPEG), **AVIF/JXL**, and **video posters** use Quick Look (`qlmanage -t`) then `sips` on macOS. Failed previews show an unavailable state; converters have deadlines so broken files cannot leave loading stuck indefinitely. GIF/WebP animate in the lightbox; Space pauses on the first frame.

In-app video uses macOS AVPlayer and renders native frames directly. Playback support depends on the file's codecs; the poster can be available even when playback is unsupported. Video stays fitted to the window, without photo zoom, rotation, or EXIF controls. Volume and mute carry between videos for the current app session. Slideshows start on photos and stop before a video.

## Platforms

| OS | Status |
| --- | --- |
| macOS | Primary. CI runs tests and validates the local `.app` bundle. |
| Linux | CI builds the binary (Wayland/X11). |
| Windows | CI job exists and is allowed to fail until GPUI-on-Windows is solid. |

About (gallery menu) shows the Cargo version and a link to the repo.
