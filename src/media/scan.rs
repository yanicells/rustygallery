use std::hash::{Hash, Hasher};
use std::path::Path;

use super::ignore::is_ignored;
use super::types::{file_stats, is_hidden, media_kind, Entry, FolderItem, MediaItem};

fn folder_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|n| n.to_str())
}

fn skip_dir(path: &Path, extra: &[String]) -> bool {
    is_hidden(path) || folder_name(path).is_some_and(|n| is_ignored(n, extra))
}

/// Current-directory listing: subfolders first, then media in this folder only.
pub fn scan_browse(dir: &Path, extra_ignore: &[String]) -> Vec<Entry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut folders = Vec::new();
    let mut media = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if skip_dir(&path, extra_ignore) {
                continue;
            }
            let name = folder_name(&path).unwrap_or("folder").to_string();
            let media_count = count_immediate_media(&path);
            let (modified, _) = file_stats(&path);
            folders.push(FolderItem {
                path,
                name: name.into(),
                media_count,
                modified,
            });
        } else if path.is_file() {
            if is_hidden(&path) {
                continue;
            }
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

    folders.sort_by_key(|a| a.name.to_lowercase());
    media.sort_by_key(|a| a.name.to_lowercase());

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
            path.is_file() && !is_hidden(&path) && media_kind(&path).is_some()
        })
        .count()
}

/// Flattened recursive media-only listing (no folder tiles).
pub fn scan_folder_recursive(root: &Path, extra_ignore: &[String]) -> Vec<Entry> {
    let mut stack = vec![root.to_path_buf()];
    let mut media = Vec::new();

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if skip_dir(&path, extra_ignore) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if !path.is_file() || is_hidden(&path) {
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

    media.sort_by_key(|a| a.name.to_lowercase());
    media.into_iter().map(Entry::Media).collect()
}

pub fn listing_stamp(dir: &Path, flat: bool, extra_ignore: &[String]) -> u64 {
    let entries = if flat {
        scan_folder_recursive(dir, extra_ignore)
    } else {
        scan_browse(dir, extra_ignore)
    };
    stamp_entries(&entries)
}

pub(crate) fn stamp_entries(entries: &[Entry]) -> u64 {
    let mut items: Vec<_> = entries
        .iter()
        .map(|e| (e.path().to_path_buf(), e.modified(), e.size()))
        .collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    items.len().hash(&mut hasher);
    for (path, modified, size) in items {
        path.hash(&mut hasher);
        modified.hash(&mut hasher);
        size.hash(&mut hasher);
    }
    hasher.finish()
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
        fs::create_dir_all(dir.join("node_modules/nested")).unwrap();
        fs::write(dir.join("pics/a.jpg"), []).unwrap();
        fs::write(dir.join("pics/b.png"), []).unwrap();
        fs::write(dir.join("pics/notes.txt"), []).unwrap();
        fs::write(dir.join("node_modules/nested/skip.jpg"), []).unwrap();
        dir
    }

    #[test]
    fn folder_tiles_count_immediate_media_only() {
        let dir = temp_tree();
        let ignore = crate::media::default_ignore_list();
        let entries = scan_browse(&dir, &ignore);
        let counts: Vec<(String, usize)> = entries
            .into_iter()
            .filter_map(|e| match e {
                Entry::Folder(f) => Some((f.name.to_string(), f.media_count)),
                Entry::Media(_) => None,
            })
            .collect();
        assert!(counts.contains(&("empty".into(), 0)));
        assert!(counts.contains(&("pics".into(), 2)));
        assert!(!counts.iter().any(|(n, _)| n == "node_modules"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn recursive_scan_skips_ignored_trees() {
        let dir = temp_tree();
        let ignore = crate::media::default_ignore_list();
        let entries = scan_folder_recursive(&dir, &ignore);
        assert_eq!(entries.len(), 2);
        let mut only_pics = ignore;
        only_pics.push("pics".into());
        let noisy = scan_folder_recursive(&dir, &only_pics);
        assert!(noisy.is_empty());
        let _ = fs::remove_dir_all(dir);
    }
}
