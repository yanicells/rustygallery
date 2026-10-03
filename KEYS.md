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
- `context` is `Gallery`, `Video` (selected inline video), `Search`, `NamePrompt`, or `*` (every context).
- Keystrokes use GPUI syntax: `cmd-q`, `cmd-shift-s`, `escape`, `[`.

The default map is the in-app shortcuts (Open Folder, Space peek, stars, theme, …). Action names match the Rust action types: `OpenFolder`, `ToggleStar`, `About`, …

See `src/app/keys.rs` for the full default file.

In the default map, Space (`NextItem`) plays or pauses a selected video, and Up/Down (`MoveUp` / `MoveDown`) adjust its volume. Left/Right keep navigating between files. These actions follow your existing bindings.

Older maps receive these extra bindings in memory; the file is never rewritten. An extra is skipped if its keystroke is already bound anywhere in your map:

```
shift-left Video VideoSeekBack
shift-right Video VideoSeekForward
m Video VideoMute
```

You can also bind `VideoTogglePlayback`, `VideoVolumeUp`, and `VideoVolumeDown`. Playback shortcuts are suppressed while Search or a name prompt is open. On-screen controls work even when shortcuts are customized.
