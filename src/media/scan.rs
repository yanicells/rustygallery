use std::path::Path;

use super::types::{is_hidden, media_kind, Entry, FolderItem, MediaItem};

/// Current-directory listing: subfolders first, then media in this folder only.
pub fn scan_browse(dir: &Path) -> Vec<Entry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut folders = Vec::new();
    let mut media = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if is_hidden(&path) {
            continue;
        }
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("folder")
                .to_string();
            folders.push(FolderItem {
                path,
                name: name.into(),
            });
        } else if path.is_file() {
            if let Some(kind) = media_kind(&path) {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("untitled")
                    .to_string();
                media.push(MediaItem {
                    path,
                    name: name.into(),
                    kind,
                });
            }
        }
    }

    folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    media.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    folders
        .into_iter()
        .map(Entry::Folder)
        .chain(media.into_iter().map(Entry::Media))
        .collect()
}

/// Flattened recursive media-only listing (no folder tiles).
pub fn scan_folder_recursive(root: &Path) -> Vec<Entry> {
    let mut stack = vec![root.to_path_buf()];
    let mut media = Vec::new();

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if is_hidden(&path) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let Some(kind) = media_kind(&path) else {
                continue;
            };
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            media.push(MediaItem {
                path,
                name: rel.into(),
                kind,
            });
        }
    }

    media.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    media.into_iter().map(Entry::Media).collect()
}
