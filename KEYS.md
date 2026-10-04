# Keymap

On first launch the app writes:

`~/Library/Application Support/rusty-gallery/keys.txt`

Edit that file and save. Bindings reload within about a second. A broken file is ignored and the last good map stays. If the file is invalid at startup, built-in defaults are used until you fix it.

## Format

One binding per line:

```
<keystroke> <context> <Action>
```

- `#` starts a comment.
- `context` is `Gallery`, `Search`, `NamePrompt`, or `*` (every context).
- Keystrokes use GPUI syntax: `cmd-q`, `cmd-shift-s`, `escape`, `[`.

The default map is the in-app shortcuts (Open Folder, Space peek, stars, theme, …). Action names match the Rust action types: `OpenFolder`, `ToggleStar`, `About`, …

See `src/app/keys.rs` for the full default file.
