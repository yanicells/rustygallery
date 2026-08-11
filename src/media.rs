use std::path::{Path, PathBuf};

use gpui::SharedString;

pub const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff"];
pub const VIDEO_EXTS: &[&str] = &["mp4", "mov", "mkv", "webm", "avi", "m4v"];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
}

#[derive(Clone)]
pub struct MediaItem {
    pub path: PathBuf,
    pub name: SharedString,
    pub kind: MediaKind,
}

pub fn scan_folder_recursive(root: &Path) -> Vec<MediaItem> {
    let mut stack = vec![root.to_path_buf()];
    let mut items = Vec::new();

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            if file_name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if !path.is_file() {
                continue;
            }

            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            let ext = ext.to_ascii_lowercase();
            let kind = if IMAGE_EXTS.iter().any(|e| *e == ext) {
                MediaKind::Image
            } else if VIDEO_EXTS.iter().any(|e| *e == ext) {
                MediaKind::Video
            } else {
                continue;
            };

            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            items.push(MediaItem {
                path,
                name: rel.into(),
                kind,
            });
        }
    }

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
}
