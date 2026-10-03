use std::{fs, time::SystemTime};

use gpui::{Action, App, DummyKeyboardMapper, KeyBinding, KeyBindingContextPredicate};

use crate::gallery::{
    About, CloseName, CloseSearch, CloseViewer, ConfirmName, ConfirmSearch, CopyPath,
    CopySelection, CopyTo, CutSelection, CycleSort, CycleTheme, DensityLarge, DensityMedium,
    DensitySmall, Duplicate, FilterAll, FilterFavorites, FilterImages, FilterVideos, GoUp,
    MoveDown, MoveLeft, MoveRight, MoveTo, MoveToTrash, MoveUp, NewFolder, NextItem, OpenFocused,
    OpenFolder, PasteSelection, Quit, RenameFocused, ResetZoom, RevealInFinder, RotateLeft,
    RotateRight, ToggleFlat, ToggleFullscreen, ToggleSaved, ToggleSearch, ToggleSlideshow,
    ToggleSortDir, ToggleStar, ToggleVideoPref, Undo, VideoMute, VideoSeekBack, VideoSeekForward,
    VideoTogglePlayback, VideoVolumeDown, VideoVolumeUp, ViewActual, ViewFill, ViewFit,
};
use crate::prefs::Prefs;

pub const DEFAULT_KEYS: &str = r#"# rusty-gallery keys
# line: <keystroke> <Context|*> <Action>
# Restart is not required — the file reloads when it changes.

cmd-q * Quit
cmd-o Gallery OpenFolder
cmd-up Gallery GoUp
backspace Gallery GoUp
escape Gallery CloseViewer
right Gallery MoveRight
left Gallery MoveLeft
up Gallery MoveUp
down Gallery MoveDown
enter Gallery OpenFocused
space Gallery NextItem
1 Gallery DensitySmall
2 Gallery DensityMedium
3 Gallery DensityLarge
s Gallery ToggleSlideshow
f Gallery ToggleFlat
0 Gallery ResetZoom
cmd-k Gallery ToggleSearch
a Gallery FilterAll
i Gallery FilterImages
v Gallery FilterVideos
cmd-shift-f Gallery FilterFavorites
cmd-shift-s Gallery ToggleStar
cmd-shift-t Gallery CycleTheme
cmd-shift-v Gallery ToggleVideoPref
escape Search CloseSearch
enter Search ConfirmSearch
cmd-r Gallery RevealInFinder
cmd-shift-c Gallery CopyPath
cmd-n Gallery NewFolder
r Gallery RenameFocused
escape NamePrompt CloseName
enter NamePrompt ConfirmName
cmd-d Gallery Duplicate
cmd-x Gallery CutSelection
cmd-c Gallery CopySelection
cmd-v Gallery PasteSelection
cmd-z Gallery Undo
cmd-backspace Gallery MoveToTrash
delete Gallery MoveToTrash
f11 Gallery ToggleFullscreen
cmd-ctrl-f Gallery ToggleFullscreen
[ Gallery RotateLeft
] Gallery RotateRight
shift-left Video VideoSeekBack
shift-right Video VideoSeekForward
m Video VideoMute
"#;

pub fn path() -> std::path::PathBuf {
    Prefs::support_dir().join("keys.txt")
}

pub fn install(cx: &mut App) {
    ensure_file();
    let bindings =
        load().unwrap_or_else(|()| parse(DEFAULT_KEYS).expect("built-in keymap must be valid"));
    cx.clear_key_bindings();
    cx.bind_keys(with_video_defaults(bindings));
}

pub fn watch(cx: &App) {
    let mut stamp = mtime();
    cx.spawn(async move |cx| loop {
        cx.background_executor()
            .timer(std::time::Duration::from_secs(1))
            .await;
        let next = mtime();
        if next == stamp {
            continue;
        }
        stamp = next;
        let Ok(bindings) = load() else {
            continue;
        };
        let _ = cx.update(|cx| {
            cx.clear_key_bindings();
            cx.bind_keys(with_video_defaults(bindings));
        });
    })
    .detach();
}

fn ensure_file() {
    let path = path();
    if path.exists() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, DEFAULT_KEYS);
}

fn mtime() -> Option<SystemTime> {
    fs::metadata(path()).and_then(|m| m.modified()).ok()
}

fn load() -> Result<Vec<KeyBinding>, ()> {
    let text = fs::read_to_string(path()).map_err(|_| ())?;
    parse(&text)
}

/// Extend older maps in memory, preserving every key the user has already bound.
fn with_video_defaults(mut bindings: Vec<KeyBinding>) -> Vec<KeyBinding> {
    for (keys, action) in [
        ("shift-left", "VideoSeekBack"),
        ("shift-right", "VideoSeekForward"),
        ("m", "VideoMute"),
    ] {
        let extra = binding(keys, Some("Video"), action).expect("valid video default");
        if !bindings
            .iter()
            .any(|existing| existing.keystrokes() == extra.keystrokes())
        {
            bindings.push(extra);
        }
    }
    bindings
}

fn parse(text: &str) -> Result<Vec<KeyBinding>, ()> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let keys = parts.next().ok_or(())?;
        let context = parts.next().ok_or(())?;
        let action = parts.next().ok_or(())?;
        if parts.next().is_some() {
            return Err(());
        }
        let ctx = if context == "*" {
            None
        } else {
            Some(context.to_string())
        };
        out.push(binding(keys, ctx.as_deref(), action).ok_or(())?);
    }
    if out.is_empty() {
        return Err(());
    }
    Ok(out)
}

fn binding(keys: &str, context: Option<&str>, action: &str) -> Option<KeyBinding> {
    let action: Box<dyn Action> = match action {
        "Quit" => Box::new(Quit),
        "OpenFolder" => Box::new(OpenFolder),
        "GoUp" => Box::new(GoUp),
        "CloseViewer" => Box::new(CloseViewer),
        "MoveRight" => Box::new(MoveRight),
        "MoveLeft" => Box::new(MoveLeft),
        "MoveUp" => Box::new(MoveUp),
        "MoveDown" => Box::new(MoveDown),
        "OpenFocused" => Box::new(OpenFocused),
        "NextItem" => Box::new(NextItem),
        "DensitySmall" => Box::new(DensitySmall),
        "DensityMedium" => Box::new(DensityMedium),
        "DensityLarge" => Box::new(DensityLarge),
        "ToggleSlideshow" => Box::new(ToggleSlideshow),
        "ToggleFlat" => Box::new(ToggleFlat),
        "ResetZoom" => Box::new(ResetZoom),
        "ToggleSearch" => Box::new(ToggleSearch),
        "FilterAll" => Box::new(FilterAll),
        "FilterImages" => Box::new(FilterImages),
        "FilterVideos" => Box::new(FilterVideos),
        "FilterFavorites" => Box::new(FilterFavorites),
        "ToggleStar" => Box::new(ToggleStar),
        "CycleTheme" => Box::new(CycleTheme),
        "ToggleVideoPref" => Box::new(ToggleVideoPref),
        "VideoTogglePlayback" => Box::new(VideoTogglePlayback),
        "VideoSeekBack" => Box::new(VideoSeekBack),
        "VideoSeekForward" => Box::new(VideoSeekForward),
        "VideoMute" => Box::new(VideoMute),
        "VideoVolumeUp" => Box::new(VideoVolumeUp),
        "VideoVolumeDown" => Box::new(VideoVolumeDown),
        "CloseSearch" => Box::new(CloseSearch),
        "ConfirmSearch" => Box::new(ConfirmSearch),
        "RevealInFinder" => Box::new(RevealInFinder),
        "CopyPath" => Box::new(CopyPath),
        "NewFolder" => Box::new(NewFolder),
        "RenameFocused" => Box::new(RenameFocused),
        "CloseName" => Box::new(CloseName),
        "ConfirmName" => Box::new(ConfirmName),
        "Duplicate" => Box::new(Duplicate),
        "CutSelection" => Box::new(CutSelection),
        "CopySelection" => Box::new(CopySelection),
        "PasteSelection" => Box::new(PasteSelection),
        "Undo" => Box::new(Undo),
        "MoveToTrash" => Box::new(MoveToTrash),
        "ToggleFullscreen" => Box::new(ToggleFullscreen),
        "RotateLeft" => Box::new(RotateLeft),
        "RotateRight" => Box::new(RotateRight),
        "ToggleSaved" => Box::new(ToggleSaved),
        "CycleSort" => Box::new(CycleSort),
        "ToggleSortDir" => Box::new(ToggleSortDir),
        "MoveTo" => Box::new(MoveTo),
        "CopyTo" => Box::new(CopyTo),
        "ViewFit" => Box::new(ViewFit),
        "ViewFill" => Box::new(ViewFill),
        "ViewActual" => Box::new(ViewActual),
        "About" => Box::new(About),
        _ => return None,
    };
    let predicate = context
        .map(KeyBindingContextPredicate::parse)
        .transpose()
        .ok()?
        .map(Into::into);
    KeyBinding::load(keys, action, predicate, false, None, &DummyKeyboardMapper).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_file_parses() {
        assert!(parse(DEFAULT_KEYS).is_ok());
    }

    #[test]
    fn rejects_invalid_keystroke_and_context() {
        assert!(parse("command-q Gallery Quit\n").is_err());
        assert!(parse("cmd-q ( Quit\n").is_err());
    }

    #[test]
    fn rejects_empty_or_incomplete_maps() {
        assert!(parse("# only a comment\n").is_err());
        assert!(parse("cmd-q *\n").is_err());
        assert!(parse("cmd-q * Quit extra\n").is_err());
    }

    #[test]
    fn rejects_unknown_action() {
        assert!(parse("cmd-q * NotAnAction\n").is_err());
    }

    #[test]
    fn video_defaults_fill_unused_keys_without_replacing_custom_bindings() {
        let original = binding("m", Some("Gallery"), "ToggleStar").unwrap();
        let key = original.keystrokes().to_vec();
        let action = original.action().name();
        let bindings = with_video_defaults(vec![original]);
        let m: Vec<_> = bindings
            .iter()
            .filter(|binding| binding.keystrokes() == key)
            .collect();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].action().name(), action);
        assert_eq!(bindings.len(), 3);
        assert_eq!(with_video_defaults(bindings).len(), 3);
    }
}
