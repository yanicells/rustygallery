# gallery

A minimal photo/video gallery built with [GPUI](https://www.gpui.rs) (Zed's UI framework).

## Run

```bash
cargo run --release -- ./media
```

Pass any folder of images/videos. Defaults to `./media` if omitted.

## Controls

| Key | Action |
| --- | --- |
| Click | Open lightbox / play video |
| ← → / Space | Prev / next |
| Esc | Close lightbox |
| Cmd+Q | Quit |

Videos open in the system player (GPUI has no built-in video decoder).
