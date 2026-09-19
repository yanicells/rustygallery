use std::path::PathBuf;

use gpui::{
    point, prelude::*, px, size, App, Bounds, KeyBinding, Menu, MenuItem, Pixels, SystemMenuType,
    TitlebarOptions, WindowBounds, WindowOptions,
};

use crate::gallery::{
    CloseName, CloseSearch, CloseViewer, ConfirmName, ConfirmSearch, CopyPath, CopySelection,
    CopyTo, CutSelection, CycleSort, CycleTheme, DensityLarge, DensityMedium, DensitySmall,
    Duplicate, FilterAll, FilterFavorites, FilterImages, FilterVideos, Gallery, GoUp, MoveDown,
    MoveLeft, MoveRight, MoveTo, MoveToTrash, MoveUp, NewFolder, NextItem, OpenFocused, OpenFolder,
    PasteSelection, Quit, RenameFocused, ResetZoom, RevealInFinder, RotateLeft, RotateRight,
    ToggleFlat, ToggleFullscreen, ToggleSaved, ToggleSearch, ToggleSlideshow, ToggleSortDir,
    ToggleStar, ToggleVideoPref, Undo, ViewActual, ViewFill, ViewFit,
};
use crate::prefs::Prefs;

#[cfg(target_os = "macos")]
mod tray;

pub fn resolve_folder() -> PathBuf {
    if let Some(arg) = std::env::args().nth(1) {
        let path = PathBuf::from(arg);
        return path.canonicalize().unwrap_or(path);
    }
    let prefs = Prefs::load();
    if let Some(recent) = prefs.recents.first() {
        return recent.clone();
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("media");
    path.canonicalize().unwrap_or(path)
}

pub fn start(folder: PathBuf, cx: &mut App) {
    cx.activate(true);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-o", OpenFolder, Some("Gallery")),
        KeyBinding::new("cmd-up", GoUp, Some("Gallery")),
        KeyBinding::new("backspace", GoUp, Some("Gallery")),
        KeyBinding::new("escape", CloseViewer, Some("Gallery")),
        KeyBinding::new("right", MoveRight, Some("Gallery")),
        KeyBinding::new("left", MoveLeft, Some("Gallery")),
        KeyBinding::new("up", MoveUp, Some("Gallery")),
        KeyBinding::new("down", MoveDown, Some("Gallery")),
        KeyBinding::new("enter", OpenFocused, Some("Gallery")),
        KeyBinding::new("space", NextItem, Some("Gallery")),
        KeyBinding::new("1", DensitySmall, Some("Gallery")),
        KeyBinding::new("2", DensityMedium, Some("Gallery")),
        KeyBinding::new("3", DensityLarge, Some("Gallery")),
        KeyBinding::new("s", ToggleSlideshow, Some("Gallery")),
        KeyBinding::new("f", ToggleFlat, Some("Gallery")),
        KeyBinding::new("0", ResetZoom, Some("Gallery")),
        KeyBinding::new("cmd-k", ToggleSearch, Some("Gallery")),
        KeyBinding::new("a", FilterAll, Some("Gallery")),
        KeyBinding::new("i", FilterImages, Some("Gallery")),
        KeyBinding::new("v", FilterVideos, Some("Gallery")),
        KeyBinding::new("cmd-shift-f", FilterFavorites, Some("Gallery")),
        KeyBinding::new("cmd-shift-s", ToggleStar, Some("Gallery")),
        KeyBinding::new("cmd-shift-t", CycleTheme, Some("Gallery")),
        KeyBinding::new("cmd-shift-v", ToggleVideoPref, Some("Gallery")),
        KeyBinding::new("escape", CloseSearch, Some("Search")),
        KeyBinding::new("enter", ConfirmSearch, Some("Search")),
        KeyBinding::new("cmd-r", RevealInFinder, Some("Gallery")),
        KeyBinding::new("cmd-shift-c", CopyPath, Some("Gallery")),
        KeyBinding::new("cmd-n", NewFolder, Some("Gallery")),
        KeyBinding::new("r", RenameFocused, Some("Gallery")),
        KeyBinding::new("escape", CloseName, Some("NamePrompt")),
        KeyBinding::new("enter", ConfirmName, Some("NamePrompt")),
        KeyBinding::new("cmd-d", Duplicate, Some("Gallery")),
        KeyBinding::new("cmd-x", CutSelection, Some("Gallery")),
        KeyBinding::new("cmd-c", CopySelection, Some("Gallery")),
        KeyBinding::new("cmd-v", PasteSelection, Some("Gallery")),
        KeyBinding::new("cmd-z", Undo, Some("Gallery")),
        KeyBinding::new("cmd-backspace", MoveToTrash, Some("Gallery")),
        KeyBinding::new("delete", MoveToTrash, Some("Gallery")),
        KeyBinding::new("f11", ToggleFullscreen, Some("Gallery")),
        KeyBinding::new("cmd-ctrl-f", ToggleFullscreen, Some("Gallery")),
        KeyBinding::new("[", RotateLeft, Some("Gallery")),
        KeyBinding::new("]", RotateRight, Some("Gallery")),
    ]);
    cx.set_menus(vec![
        Menu {
            name: "gallery".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("Open Folder…", OpenFolder),
                MenuItem::action("New Folder", NewFolder),
                MenuItem::action("Rename", RenameFocused),
                MenuItem::action("Go Up", GoUp),
                MenuItem::separator(),
                MenuItem::action("Save Library", ToggleSaved),
                MenuItem::action("Star / Unstar", ToggleStar),
                MenuItem::separator(),
                MenuItem::action("Reveal in Finder", RevealInFinder),
                MenuItem::action("Copy Path", CopyPath),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", Undo),
                MenuItem::separator(),
                MenuItem::action("Cut", CutSelection),
                MenuItem::action("Copy", CopySelection),
                MenuItem::action("Paste", PasteSelection),
                MenuItem::action("Duplicate", Duplicate),
                MenuItem::separator(),
                MenuItem::action("Move to…", MoveTo),
                MenuItem::action("Copy to…", CopyTo),
                MenuItem::action("Move to Trash", MoveToTrash),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Folders / Flat", ToggleFlat),
                MenuItem::separator(),
                MenuItem::action("Density Small", DensitySmall),
                MenuItem::action("Density Medium", DensityMedium),
                MenuItem::action("Density Large", DensityLarge),
                MenuItem::separator(),
                MenuItem::action("All", FilterAll),
                MenuItem::action("Images", FilterImages),
                MenuItem::action("Videos", FilterVideos),
                MenuItem::action("Stars", FilterFavorites),
                MenuItem::separator(),
                MenuItem::action("Cycle Theme", CycleTheme),
                MenuItem::action("Video: stay / system", ToggleVideoPref),
                MenuItem::separator(),
                MenuItem::action("Cycle Sort", CycleSort),
                MenuItem::action("Sort Direction", ToggleSortDir),
                MenuItem::action("Search…", ToggleSearch),
                MenuItem::separator(),
                MenuItem::action("Fullscreen", ToggleFullscreen),
                MenuItem::action("Fit", ViewFit),
                MenuItem::action("Fill", ViewFill),
                MenuItem::action("Actual Size", ViewActual),
            ],
        },
        Menu {
            name: "Playback".into(),
            items: vec![
                MenuItem::action("Slideshow", ToggleSlideshow),
                MenuItem::action("Reset Zoom", ResetZoom),
                MenuItem::separator(),
                MenuItem::action("Rotate Left", RotateLeft),
                MenuItem::action("Rotate Right", RotateRight),
            ],
        },
    ]);

    let title = format!("gallery — {}", folder.display());
    let bounds = restore_bounds(cx);

    cx.open_window(
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(title.into()),
                appears_transparent: false,
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            focus: true,
            ..Default::default()
        },
        |window, cx| cx.new(|cx| Gallery::new(folder.clone(), window, cx)),
    )
    .unwrap();

    // Re-activate after the window exists so the macOS menu bar
    // switches away from the parent (Terminal / IDE) to this app.
    cx.activate(true);

    #[cfg(target_os = "macos")]
    tray::install();
}

fn restore_bounds(cx: &App) -> Bounds<Pixels> {
    let prefs = Prefs::load();
    if let Some((x, y, w, h)) = prefs.window {
        return Bounds {
            origin: point(px(x), px(y)),
            size: size(px(w), px(h)),
        };
    }
    Bounds::centered(None, size(px(1200.), px(800.)), cx)
}
