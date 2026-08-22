use std::path::Path;

use super::types::{file_stats, is_hidden, media_kind, Entry, FolderItem, MediaItem};

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
            let media_count = count_immediate_media(&path);
            let (modified, _) = file_stats(&path);
            folders.push(FolderItem {
                path,
                name: name.into(),
                media_count,
                modified,
            });
        } else if path.is_file() {
            if let Some(kind) = media_kind(&path) {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("untitled")
                    .to_string();
                let (modified, size) = file_stats(&path);
                media.push(MediaItem {
                    path,
                    name: name.into(),
                    kind,
                    modified,
                    size,
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

fn count_immediate_media(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| {
            let path = entry.path();
            path.is_file() && media_kind(&path).is_some()
        })
        .count()
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
            let (modified, size) = file_stats(&path);
            media.push(MediaItem {
                path,
                name: rel.into(),
                kind,
                modified,
                size,
            });
        }
    }

    media.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    media.into_iter().map(Entry::Media).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_tree() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rusty-scan-count-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(dir.join("empty")).unwrap();
        fs::create_dir_all(dir.join("pics")).unwrap();
        fs::write(dir.join("pics/a.jpg"), []).unwrap();
        fs::write(dir.join("pics/b.png"), []).unwrap();
        fs::write(dir.join("pics/notes.txt"), []).unwrap();
        dir
    }

    #[test]
    fn folder_tiles_count_immediate_media_only() {
        let dir = temp_tree();
        let entries = scan_browse(&dir);
        let counts: Vec<(String, usize)> = entries
            .into_iter()
            .filter_map(|e| match e {
                Entry::Folder(f) => Some((f.name.to_string(), f.media_count)),
                Entry::Media(_) => None,
            })
            .collect();
        assert!(counts.contains(&("empty".into(), 0)));
        assert!(counts.contains(&("pics".into(), 2)));
        let _ = fs::remove_dir_all(dir);
    }
}
