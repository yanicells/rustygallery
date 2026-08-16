//! Minimal macOS status-item spike (right side of the menu bar).
//! Not part of GPUI — uses `tray-icon` / Cocoa under the hood.

use tray_icon::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    Icon, TrayIconBuilder,
};

fn tiny_icon() -> Icon {
    // Simple light square — enough for a “is it there?” check.
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for px in rgba.chunks_exact_mut(4) {
        px[0] = 0xe8;
        px[1] = 0xe8;
        px[2] = 0xe8;
        px[3] = 0xff;
    }
    Icon::from_rgba(rgba, size, size).expect("tray icon")
}

/// Install a status item titled "gallery" with a tiny dropdown.
/// Must run on the main thread after the NSApp is up.
pub fn install() {
    let menu = Menu::new();
    let note = MenuItem::new("tray spike works", false, None);
    let quit = PredefinedMenuItem::quit(Some("Quit gallery"));
    menu.append(&note).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    menu.append(&quit).ok();

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("gallery")
        .with_title("gallery")
        .with_icon(tiny_icon())
        .build()
        .expect("create status item");

    // Keep alive for the process (TrayIcon is !Send/!Sync).
    std::mem::forget(tray);
}
