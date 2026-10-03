//! macOS status item. Native menu; clicks land on the gallery window.

use std::{
    cell::RefCell,
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use gpui::{App, AppContext, AsyncApp, WindowHandle};
use tray_icon::{
    menu::{
        AboutMetadata, Icon as MenuIcon, IconMenuItem, Menu, MenuEvent, MenuItem, MenuItemKind,
        PredefinedMenuItem, Submenu,
    },
    Icon, TrayIcon, TrayIconBuilder,
};

use crate::gallery::Gallery;
use crate::media::preview_jpeg;
use crate::prefs::Prefs;

const TRAY_PNG: &[u8] = include_bytes!("../../assets/icon/tray_template.png");

struct TrayUi {
    _icon: TrayIcon,
    preview: IconMenuItem,
    show: MenuItem,
    saved: Submenu,
    recents: Submenu,
    preview_path: Option<PathBuf>,
    pending_preview: Option<PathBuf>,
    preview_generation: u64,
    failed_previews: HashSet<PathBuf>,
    prefs: Prefs,
    prefs_stamp: Option<(SystemTime, u64)>,
    saved_paths: Vec<(PathBuf, bool)>,
    recent_paths: Vec<(PathBuf, bool)>,
    live_favorites: Vec<PathBuf>,
    next: MenuItem,
    open: MenuItem,
    reveal: MenuItem,
    hidden: bool,
}

pub(crate) fn install(handle: WindowHandle<Gallery>, cx: &mut App) {
    let Ok(ui) = TrayUi::build() else {
        return;
    };
    let ui = Rc::new(RefCell::new(ui));
    let close_ui = ui.clone();
    if handle
        .update(cx, |_, window, cx| {
            let gallery = cx.weak_entity();
            window.on_window_should_close(cx, move |_, cx| {
                gallery
                    .update(cx, |gallery, cx| gallery.hide_playback(cx))
                    .ok();
                close_ui.borrow_mut().set_hidden(true);
                cx.hide();
                false
            });
        })
        .is_err()
    {
        return;
    }
    // Keep a working Dock/window entry if the status item could not be installed.
    hide_dock();
    cx.spawn(async move |cx| {
        let mut refresh_at = std::time::Instant::now();
        loop {
            let mut ids = Vec::new();
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                ids.push(event.id.0);
            }
            for id in ids {
                if id == "next" {
                    request_preview(&ui, cx);
                } else {
                    let _ = cx.update(|cx| dispatch(&id, handle, cx, &ui));
                }
            }
            if std::time::Instant::now() >= refresh_at {
                let (stamp, prefs) = {
                    let ui = ui.borrow();
                    (ui.prefs_stamp, ui.prefs.clone())
                };
                let refresh = cx
                    .background_spawn(async move { read_refresh(stamp, prefs) })
                    .await;
                let needs_preview = ui.borrow_mut().refresh(refresh);
                if needs_preview {
                    request_preview(&ui, cx);
                }
                refresh_at = std::time::Instant::now() + Duration::from_secs(1);
            }
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
        }
    })
    .detach();
}

fn dispatch(id: &str, handle: WindowHandle<Gallery>, cx: &mut App, ui: &Rc<RefCell<TrayUi>>) {
    match id {
        "show" => {
            let hide = !ui.borrow().hidden;
            if hide {
                let _ = handle.update(cx, |gallery, _, cx| gallery.hide_playback(cx));
                ui.borrow_mut().set_hidden(true);
                cx.hide();
            } else {
                ui.borrow_mut().set_hidden(false);
                cx.activate(true);
                let _ = handle.update(cx, |gallery, window, cx| {
                    gallery.show_playback(cx);
                    window.activate_window();
                });
            }
        }
        "folder" => {
            ui.borrow_mut().set_hidden(false);
            cx.activate(true);
            let _ = handle.update(cx, |gallery, window, cx| {
                gallery.show_playback(cx);
                window.activate_window();
                gallery.pick_folder(cx);
            });
        }
        "open" => {
            let path = ui.borrow().preview_path.clone();
            let Some(path) = path else {
                return;
            };
            ui.borrow_mut().set_hidden(false);
            cx.activate(true);
            let _ = handle.update(cx, |gallery, window, cx| {
                gallery.show_playback(cx);
                window.activate_window();
                gallery.tray_open_path(path, cx);
            });
        }
        "reveal" => {
            let path = ui.borrow().preview_path.clone();
            let Some(path) = path else {
                return;
            };
            let _ = handle.update(cx, |_, _, cx| cx.reveal_path(&path));
        }
        other if other.starts_with("saved:") || other.starts_with("recent:") => {
            let state = ui.borrow();
            let list = if other.starts_with("saved:") {
                &state.saved_paths
            } else {
                &state.recent_paths
            };
            let Ok(idx) = other.rsplit(':').next().unwrap_or("").parse::<usize>() else {
                return;
            };
            let Some((path, available)) = list.get(idx).cloned() else {
                return;
            };
            if !available {
                return;
            }
            drop(state);
            ui.borrow_mut().set_hidden(false);
            cx.activate(true);
            let _ = handle.update(cx, |gallery, window, cx| {
                gallery.show_playback(cx);
                window.activate_window();
                gallery.tray_open_path(path, cx);
            });
        }
        _ => {}
    }
}

impl TrayUi {
    fn build() -> Result<Self, ()> {
        let preview = IconMenuItem::with_id("preview", "No stars yet", false, None, None);
        let next = MenuItem::with_id("next", "Next random", false, None);
        let open = MenuItem::with_id("open", "Open in gallery", false, None);
        let reveal = MenuItem::with_id("reveal", "Reveal in Finder", false, None);
        let show = MenuItem::with_id("show", "Hide", true, None);
        let folder = MenuItem::with_id("folder", "Open Folder…", true, None);
        let saved = Submenu::new("Saved", true);
        let recents = Submenu::new("Recents", true);
        fill_paths(&saved, &[], "saved", "No saved libraries");
        fill_paths(&recents, &[], "recent", "No recents");
        let about = PredefinedMenuItem::about(
            Some("About gallery"),
            Some(AboutMetadata {
                name: Some("gallery".into()),
                version: Some(env!("CARGO_PKG_VERSION").into()),
                credits: Some("Built with GPUI.\nhttps://github.com/yanicells/rustygallery".into()),
                ..Default::default()
            }),
        );
        let quit = PredefinedMenuItem::quit(Some("Quit"));

        let menu = Menu::new();
        menu.append(&preview).map_err(|_| ())?;
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&next).ok();
        menu.append(&open).ok();
        menu.append(&reveal).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&show).ok();
        menu.append(&folder).ok();
        menu.append(&saved).ok();
        menu.append(&recents).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&about).ok();
        menu.append(&quit).ok();

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("gallery")
            .with_icon(status_icon())
            .with_icon_as_template(true)
            .build()
            .map_err(|_| ())?;

        Ok(Self {
            _icon: icon,
            preview,
            show,
            saved,
            recents,
            preview_path: None,
            pending_preview: None,
            preview_generation: 0,
            failed_previews: HashSet::new(),
            prefs: Prefs::default(),
            prefs_stamp: None,
            saved_paths: Vec::new(),
            recent_paths: Vec::new(),
            live_favorites: Vec::new(),
            next,
            open,
            reveal,
            hidden: false,
        })
    }

    fn set_hidden(&mut self, hidden: bool) {
        if self.hidden != hidden {
            self.hidden = hidden;
            self.show.set_text(if hidden { "Show" } else { "Hide" });
        }
    }

    fn refresh(&mut self, refresh: TrayRefresh) -> bool {
        if self.saved_paths != refresh.saved {
            fill_paths(&self.saved, &refresh.saved, "saved", "No saved libraries");
            self.saved_paths = refresh.saved;
        }
        if self.recent_paths != refresh.recents {
            fill_paths(&self.recents, &refresh.recents, "recent", "No recents");
            self.recent_paths = refresh.recents;
        }
        if self.prefs_stamp != refresh.stamp {
            self.prefs_stamp = refresh.stamp;
            self.prefs = refresh.prefs;
        }
        if self.live_favorites != refresh.favorites {
            self.live_favorites = refresh.favorites;
            self.failed_previews
                .retain(|p| self.live_favorites.contains(p));
            self.next.set_enabled(self.has_preview_candidates());
        }
        let current = self.pending_preview.as_ref().or(self.preview_path.as_ref());
        match current {
            Some(path) => !self.live_favorites.contains(path),
            None => self.has_preview_candidates(),
        }
    }

    fn has_preview_candidates(&self) -> bool {
        self.live_favorites
            .iter()
            .any(|p| !self.failed_previews.contains(p))
    }

    fn clear_preview(&mut self, text: &str) {
        self.preview_path = None;
        self.preview.set_text(text);
        self.preview.set_icon(None);
        self.open.set_enabled(false);
        self.reveal.set_enabled(false);
    }
}

// File reads, availability checks, and decoding never run on the UI thread.
struct TrayRefresh {
    stamp: Option<(SystemTime, u64)>,
    prefs: Prefs,
    saved: Vec<(PathBuf, bool)>,
    recents: Vec<(PathBuf, bool)>,
    favorites: Vec<PathBuf>,
}

fn read_refresh(stamp: Option<(SystemTime, u64)>, cached: Prefs) -> TrayRefresh {
    let next = std::fs::metadata(Prefs::path())
        .ok()
        .and_then(|m| Some((m.modified().ok()?, m.len())));
    let prefs = if next != stamp { Prefs::load() } else { cached };
    let paths = |list: &[PathBuf]| {
        list.iter()
            .take(12)
            .map(|p| (p.clone(), p.is_dir()))
            .collect()
    };
    TrayRefresh {
        stamp: next,
        saved: paths(&prefs.saved),
        recents: paths(&prefs.recents),
        favorites: prefs
            .favorites
            .iter()
            .filter(|p| p.is_file())
            .cloned()
            .collect(),
        prefs,
    }
}

fn request_preview(ui: &Rc<RefCell<TrayUi>>, cx: &AsyncApp) {
    let (path, generation) = {
        let mut state = ui.borrow_mut();
        let candidates: Vec<_> = state
            .live_favorites
            .iter()
            .filter(|p| !state.failed_previews.contains(*p))
            .cloned()
            .collect();
        let skip = state
            .pending_preview
            .as_deref()
            .or(state.preview_path.as_deref());
        let path = pick_favorite(&candidates, skip);
        state.preview_generation += 1;
        state.pending_preview = path.clone();
        let text = if path.is_some() {
            "Loading star…"
        } else {
            "No available stars"
        };
        state.clear_preview(text);
        state.next.set_enabled(!candidates.is_empty());
        (path, state.preview_generation)
    };
    let Some(path) = path else {
        return;
    };
    let ui = ui.clone();
    cx.spawn(async move |cx| {
        let source = path.clone();
        let image = cx
            .background_spawn(async move { menu_thumb(&source) })
            .await;
        let retry = {
            let mut state = ui.borrow_mut();
            if state.preview_generation != generation {
                return;
            }
            state.pending_preview = None;
            if !state.live_favorites.contains(&path) {
                true
            } else if let Some((rgba, w, h)) = image {
                if let Ok(icon) = MenuIcon::from_rgba(rgba, w, h) {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("favorite");
                    state.preview.set_text(name);
                    state.preview.set_icon(Some(icon));
                    state.preview_path = Some(path);
                    state.open.set_enabled(true);
                    state.reveal.set_enabled(true);
                    false
                } else {
                    state.failed_previews.insert(path);
                    true
                }
            } else {
                state.failed_previews.insert(path);
                true
            }
        };
        if retry {
            request_preview(&ui, cx);
        }
    })
    .detach();
}

fn fill_paths(menu: &Submenu, paths: &[(PathBuf, bool)], prefix: &str, empty: &str) {
    for item in menu.items() {
        match &item {
            MenuItemKind::MenuItem(i) => {
                let _ = menu.remove(i);
            }
            MenuItemKind::Icon(i) => {
                let _ = menu.remove(i);
            }
            MenuItemKind::Check(i) => {
                let _ = menu.remove(i);
            }
            MenuItemKind::Predefined(i) => {
                let _ = menu.remove(i);
            }
            MenuItemKind::Submenu(i) => {
                let _ = menu.remove(i);
            }
        }
    }
    if paths.is_empty() {
        let _ = menu.append(&MenuItem::new(empty, false, None));
        return;
    }
    for (i, (path, available)) in paths.iter().enumerate() {
        let label = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("folder");
        let item = MenuItem::with_id(format!("{prefix}:{i}"), label, *available, None);
        let _ = menu.append(&item);
    }
}

fn pick_favorite(favorites: &[PathBuf], skip: Option<&Path>) -> Option<PathBuf> {
    let live = favorites;
    if live.is_empty() {
        return None;
    }
    if live.len() == 1 {
        return live.first().cloned();
    }
    let tick = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as usize)
        .unwrap_or(0);
    let mut idx = tick % live.len();
    if skip.is_some_and(|s| live[idx] == s) {
        idx = (idx + 1) % live.len();
    }
    Some(live[idx].clone())
}

fn status_icon() -> Icon {
    let img = image::load_from_memory(TRAY_PNG).expect("tray png");
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), w, h).expect("tray icon")
}

fn menu_thumb(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
    let bytes = preview_jpeg(path, 64)?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Some((img.into_raw(), w, h))
}

fn hide_dock() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
}
