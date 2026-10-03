//! macOS status item. Native menu; clicks land on the gallery window.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use gpui::{App, WindowHandle};
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
    hidden: bool,
}

pub fn install(handle: WindowHandle<Gallery>, cx: &mut App) {
    hide_dock();
    let Ok(ui) = TrayUi::build() else {
        return;
    };
    let ui = Rc::new(RefCell::new(ui));
    ui.borrow_mut().refresh();
    cx.spawn(async move |cx| loop {
        let mut ids = Vec::new();
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            ids.push(event.id.0);
        }
        for id in ids {
            let _ = cx.update(|cx| dispatch(&id, handle, cx, &ui));
        }
        ui.borrow_mut().refresh();
        cx.background_executor()
            .timer(Duration::from_millis(300))
            .await;
    })
    .detach();
}

fn dispatch(id: &str, handle: WindowHandle<Gallery>, cx: &mut App, ui: &Rc<RefCell<TrayUi>>) {
    match id {
        "show" => {
            let hide = !ui.borrow().hidden;
            if hide {
                ui.borrow_mut().hidden = true;
                cx.hide();
            } else {
                ui.borrow_mut().hidden = false;
                cx.activate(true);
                let _ = handle.update(cx, |_, window, _| window.activate_window());
            }
        }
        "folder" => {
            ui.borrow_mut().hidden = false;
            cx.activate(true);
            let _ = handle.update(cx, |gallery, window, cx| {
                window.activate_window();
                gallery.pick_folder(cx);
            });
        }
        "next" => ui.borrow_mut().next_preview(),
        "open" => {
            let path = ui.borrow().preview_path.clone();
            let Some(path) = path else {
                return;
            };
            ui.borrow_mut().hidden = false;
            cx.activate(true);
            let _ = handle.update(cx, |gallery, window, cx| {
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
            let prefs = Prefs::load();
            let list = if other.starts_with("saved:") {
                &prefs.saved
            } else {
                &prefs.recents
            };
            let Ok(idx) = other.rsplit(':').next().unwrap_or("").parse::<usize>() else {
                return;
            };
            let Some(path) = list.get(idx).cloned() else {
                return;
            };
            ui.borrow_mut().hidden = false;
            cx.activate(true);
            let _ = handle.update(cx, |gallery, window, cx| {
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
        let next = MenuItem::with_id("next", "Next random", true, None);
        let open = MenuItem::with_id("open", "Open in gallery", true, None);
        let reveal = MenuItem::with_id("reveal", "Reveal in Finder", true, None);
        let show = MenuItem::with_id("show", "Hide", true, None);
        let folder = MenuItem::with_id("folder", "Open Folder…", true, None);
        let saved = Submenu::new("Saved", true);
        let recents = Submenu::new("Recents", true);
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
            hidden: false,
        })
    }

    fn refresh(&mut self) {
        self.show.set_text(if self.hidden { "Show" } else { "Hide" });
        let prefs = Prefs::load();
        fill_paths(&self.saved, &prefs.saved, "saved", "No saved libraries");
        fill_paths(&self.recents, &prefs.recents, "recent", "No recents");
        if self.preview_path.as_ref().is_none_or(|p| !p.is_file()) {
            self.preview_path = pick_favorite(&prefs.favorites, None);
            self.apply_preview();
        }
    }

    fn next_preview(&mut self) {
        let prefs = Prefs::load();
        self.preview_path = pick_favorite(&prefs.favorites, self.preview_path.as_deref());
        self.apply_preview();
    }

    fn apply_preview(&self) {
        let Some(path) = &self.preview_path else {
            self.preview.set_text("No stars yet");
            self.preview.set_icon(None);
            return;
        };
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("favorite");
        self.preview.set_text(name);
        self.preview.set_icon(menu_thumb(path));
    }
}

fn fill_paths(menu: &Submenu, paths: &[PathBuf], prefix: &str, empty: &str) {
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
    for (i, path) in paths.iter().take(12).enumerate() {
        let label = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("folder");
        let item = MenuItem::with_id(format!("{prefix}:{i}"), label, path.exists(), None);
        let _ = menu.append(&item);
    }
}

fn pick_favorite(favorites: &[PathBuf], skip: Option<&Path>) -> Option<PathBuf> {
    let live: Vec<PathBuf> = favorites.iter().filter(|p| p.is_file()).cloned().collect();
    if live.is_empty() {
        return None;
    }
    if live.len() == 1 {
        return live.into_iter().next();
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

fn menu_thumb(path: &Path) -> Option<MenuIcon> {
    let bytes = preview_jpeg(path, 64)?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    MenuIcon::from_rgba(img.into_raw(), w, h).ok()
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
