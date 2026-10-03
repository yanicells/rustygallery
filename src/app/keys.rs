use std::{fs, time::SystemTime};

use gpui::{App, KeyBinding};

use crate::gallery::{
    About, CloseName, CloseSearch, CloseViewer, ConfirmName, ConfirmSearch, CopyPath,
    CopySelection, CopyTo, CutSelection, CycleSort, CycleTheme, DensityLarge, DensityMedium,
    DensitySmall, Duplicate, FilterAll, FilterFavorites, FilterImages, FilterVideos, GoUp,
    MoveDown, MoveLeft, MoveRight, MoveTo, MoveToTrash, MoveUp, NewFolder, NextItem, OpenFocused,
    OpenFolder, PasteSelection, Quit, RenameFocused, ResetZoom, RevealInFinder, RotateLeft,
    RotateRight, ToggleFlat, ToggleFullscreen, ToggleSaved, ToggleSearch, ToggleSlideshow,
    ToggleSortDir, ToggleStar, ToggleVideoPref, Undo, ViewActual, ViewFill, ViewFit,
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
"#;

pub fn path() -> std::path::PathBuf {
    Prefs::support_dir().join("keys.txt")
}

pub fn install(cx: &mut App) {
    ensure_file();
    if let Ok(bindings) = load() {
        cx.clear_key_bindings();
        cx.bind_keys(bindings);
    }
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
            cx.bind_keys(bindings);
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
    Some(match action {
        "Quit" => KeyBinding::new(keys, Quit, context),
        "OpenFolder" => KeyBinding::new(keys, OpenFolder, context),
        "GoUp" => KeyBinding::new(keys, GoUp, context),
        "CloseViewer" => KeyBinding::new(keys, CloseViewer, context),
        "MoveRight" => KeyBinding::new(keys, MoveRight, context),
        "MoveLeft" => KeyBinding::new(keys, MoveLeft, context),
        "MoveUp" => KeyBinding::new(keys, MoveUp, context),
        "MoveDown" => KeyBinding::new(keys, MoveDown, context),
        "OpenFocused" => KeyBinding::new(keys, OpenFocused, context),
        "NextItem" => KeyBinding::new(keys, NextItem, context),
        "DensitySmall" => KeyBinding::new(keys, DensitySmall, context),
        "DensityMedium" => KeyBinding::new(keys, DensityMedium, context),
        "DensityLarge" => KeyBinding::new(keys, DensityLarge, context),
        "ToggleSlideshow" => KeyBinding::new(keys, ToggleSlideshow, context),
        "ToggleFlat" => KeyBinding::new(keys, ToggleFlat, context),
        "ResetZoom" => KeyBinding::new(keys, ResetZoom, context),
        "ToggleSearch" => KeyBinding::new(keys, ToggleSearch, context),
        "FilterAll" => KeyBinding::new(keys, FilterAll, context),
        "FilterImages" => KeyBinding::new(keys, FilterImages, context),
        "FilterVideos" => KeyBinding::new(keys, FilterVideos, context),
        "FilterFavorites" => KeyBinding::new(keys, FilterFavorites, context),
        "ToggleStar" => KeyBinding::new(keys, ToggleStar, context),
        "CycleTheme" => KeyBinding::new(keys, CycleTheme, context),
        "ToggleVideoPref" => KeyBinding::new(keys, ToggleVideoPref, context),
        "CloseSearch" => KeyBinding::new(keys, CloseSearch, context),
        "ConfirmSearch" => KeyBinding::new(keys, ConfirmSearch, context),
        "RevealInFinder" => KeyBinding::new(keys, RevealInFinder, context),
        "CopyPath" => KeyBinding::new(keys, CopyPath, context),
        "NewFolder" => KeyBinding::new(keys, NewFolder, context),
        "RenameFocused" => KeyBinding::new(keys, RenameFocused, context),
        "CloseName" => KeyBinding::new(keys, CloseName, context),
        "ConfirmName" => KeyBinding::new(keys, ConfirmName, context),
        "Duplicate" => KeyBinding::new(keys, Duplicate, context),
        "CutSelection" => KeyBinding::new(keys, CutSelection, context),
        "CopySelection" => KeyBinding::new(keys, CopySelection, context),
        "PasteSelection" => KeyBinding::new(keys, PasteSelection, context),
        "Undo" => KeyBinding::new(keys, Undo, context),
        "MoveToTrash" => KeyBinding::new(keys, MoveToTrash, context),
        "ToggleFullscreen" => KeyBinding::new(keys, ToggleFullscreen, context),
        "RotateLeft" => KeyBinding::new(keys, RotateLeft, context),
        "RotateRight" => KeyBinding::new(keys, RotateRight, context),
        "ToggleSaved" => KeyBinding::new(keys, ToggleSaved, context),
        "CycleSort" => KeyBinding::new(keys, CycleSort, context),
        "ToggleSortDir" => KeyBinding::new(keys, ToggleSortDir, context),
        "MoveTo" => KeyBinding::new(keys, MoveTo, context),
        "CopyTo" => KeyBinding::new(keys, CopyTo, context),
        "ViewFit" => KeyBinding::new(keys, ViewFit, context),
        "ViewFill" => KeyBinding::new(keys, ViewFill, context),
        "ViewActual" => KeyBinding::new(keys, ViewActual, context),
        "About" => KeyBinding::new(keys, About, context),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_file_parses() {
        assert!(parse(DEFAULT_KEYS).is_ok());
    }

    #[test]
    fn rejects_unknown_action() {
        assert!(parse("cmd-q * NotAnAction\n").is_err());
    }
}
