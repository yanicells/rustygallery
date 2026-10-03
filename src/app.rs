use std::path::PathBuf;

use gpui::{
    point, prelude::*, px, size, App, Bounds, Menu, MenuItem, Pixels, SystemMenuType,
    TitlebarOptions, WindowBounds, WindowOptions,
};

use crate::gallery::{
    About, CopyPath, CopySelection, CopyTo, CutSelection, CycleSort, CycleTheme, DensityLarge,
    DensityMedium, DensitySmall, Duplicate, FilterAll, FilterFavorites, FilterImages, FilterVideos,
    Gallery, GoUp, MoveTo, MoveToTrash, NewFolder, OpenFolder, PasteSelection, Quit, RenameFocused,
    ResetZoom, RevealInFinder, RotateLeft, RotateRight, ToggleFlat, ToggleFullscreen, ToggleSaved,
    ToggleSearch, ToggleSlideshow, ToggleSortDir, ToggleStar, ToggleVideoPref, Undo, ViewActual,
    ViewFill, ViewFit,
};
use crate::prefs::Prefs;

#[cfg(target_os = "macos")]
mod fullscreen;
mod keys;
#[cfg(target_os = "macos")]
mod tray;

pub(crate) fn is_hidden() -> bool {
    #[cfg(target_os = "macos")]
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        return objc2_app_kit::NSApplication::sharedApplication(mtm).isHidden();
    }
    false
}

pub fn resolve_folder() -> PathBuf {
    if let Some(arg) = std::env::args().nth(1) {
        let path = PathBuf::from(arg);
        if path.is_dir() {
            return path.canonicalize().unwrap_or(path);
        }
    }
    let prefs = Prefs::load();
    if let Some(folder) = prefs
        .recents
        .iter()
        .chain(&prefs.saved)
        .find(|p| p.is_dir())
    {
        return folder.clone();
    }
    let media = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("media");
    if media.is_dir() {
        return media.canonicalize().unwrap_or(media);
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let pictures = home.join("Pictures");
        if pictures.is_dir() {
            return pictures;
        }
        if home.is_dir() {
            return home;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
}

pub fn start(folder: PathBuf, cx: &mut App) {
    cx.activate(true);
    cx.on_action(|_: &Quit, cx| cx.quit());
    keys::install(cx);
    keys::watch(cx);
    cx.set_menus(vec![
        Menu {
            name: "gallery".into(),
            items: vec![
                MenuItem::action("About gallery", About),
                MenuItem::separator(),
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
                MenuItem::action("Video: built-in / system", ToggleVideoPref),
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

    let handle = cx
        .open_window(
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
            |window, cx| {
                #[cfg(target_os = "macos")]
                fullscreen::enable(window);
                cx.new(|cx| Gallery::new(folder.clone(), window, cx))
            },
        )
        .unwrap();

    // Re-activate after the window exists so the macOS menu bar
    // switches away from the parent (Terminal / IDE) to this app.
    cx.activate(true);

    #[cfg(target_os = "macos")]
    tray::install(handle, cx);
    #[cfg(not(target_os = "macos"))]
    let _ = handle;
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
