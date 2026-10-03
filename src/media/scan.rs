use std::{
    fs::{self, Metadata},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use super::ignore::is_ignored;
use super::types::{is_hidden, media_kind, Entry, FolderItem, MediaItem, MediaKind};

fn folder_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|n| n.to_str())
}

fn skip_dir(path: &Path, extra: &[String]) -> bool {
    is_hidden(path) || folder_name(path).is_some_and(|n| is_ignored(n, extra))
}

fn stats(metadata: &Metadata) -> (u64, u64) {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (modified, metadata.len())
}

/// Visit an unsorted listing without constructing UI entries. Directory symlinks
/// remain browseable, but recursive walks never follow them. File symlinks work.
fn visit_listing(
    root: &Path,
    flat: bool,
    extra_ignore: &[String],
    mut visit: impl FnMut(PathBuf, Metadata, Option<MediaKind>),
) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if is_hidden(&path) {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                let Ok(metadata) = fs::metadata(&path) else {
                    continue;
                };
                if metadata.is_dir() {
                    if !flat && !skip_dir(&path, extra_ignore) {
                        visit(path, metadata, None);
                    }
                } else if metadata.is_file() {
                    if let Some(kind) = media_kind(&path) {
                        visit(path, metadata, Some(kind));
                    }
                }
            } else if kind.is_dir() {
                if skip_dir(&path, extra_ignore) {
                    continue;
                }
                if flat {
                    stack.push(path);
                } else if let Ok(metadata) = entry.metadata() {
                    visit(path, metadata, None);
                }
            } else if kind.is_file() {
                if let Some(kind) = media_kind(&path) {
                    if let Ok(metadata) = entry.metadata() {
                        visit(path, metadata, Some(kind));
                    }
                }
            }
        }
    }
}

/// Current-directory listing: subfolders first, then media in this folder only.
pub fn scan_browse(dir: &Path, extra_ignore: &[String]) -> Vec<Entry> {
    let mut folders = Vec::new();
    let mut media = Vec::new();
    visit_listing(dir, false, extra_ignore, |path, metadata, kind| {
        let (modified, size) = stats(&metadata);
        let name = folder_name(&path).unwrap_or("untitled").to_string();
        if let Some(kind) = kind {
            media.push(MediaItem {
                path,
                name: name.into(),
                kind,
                modified,
                size,
            });
        } else {
            let media_count = count_immediate_media(&path);
            folders.push(FolderItem {
                path,
                name: name.into(),
                media_count,
                modified,
            });
        }
    });
    folders.sort_by_key(|a| a.name.to_lowercase());
    media.sort_by_key(|a| a.name.to_lowercase());
    folders
        .into_iter()
        .map(Entry::Folder)
        .chain(media.into_iter().map(Entry::Media))
        .collect()
}

fn count_immediate_media(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| {
            let path = entry.path();
            if is_hidden(&path) || media_kind(&path).is_none() {
                return false;
            }
            entry
                .file_type()
                .is_ok_and(|kind| kind.is_file() || (kind.is_symlink() && path.is_file()))
        })
        .count()
}

/// Flattened recursive media-only listing (no directory symlink traversal).
pub fn scan_folder_recursive(root: &Path, extra_ignore: &[String]) -> Vec<Entry> {
    let mut media = Vec::new();
    visit_listing(root, true, extra_ignore, |path, metadata, kind| {
        let Some(kind) = kind else { return };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let (modified, size) = stats(&metadata);
        media.push(MediaItem {
            path,
            name: rel.into(),
            kind,
            modified,
            size,
        });
    });
    media.sort_by_key(|a| a.name.to_lowercase());
    media.into_iter().map(Entry::Media).collect()
}

// Combining per-entry hashes makes stamps independent of read_dir order without
// sorting paths or building names, entry vectors, or folder tile data each poll.
#[derive(Default)]
struct ListingStamp {
    count: u64,
    xor: u64,
    sum: u64,
}

impl ListingStamp {
    fn add(&mut self, path: &Path, modified: u64, size: u64) {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (path, modified, size).hash(&mut hasher);
        let hash = hasher.finish();
        self.count += 1;
        self.xor ^= hash;
        self.sum = self.sum.wrapping_add(hash);
    }

    fn finish(self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.count, self.xor, self.sum).hash(&mut hasher);
        hasher.finish()
    }
}

pub fn listing_stamp(dir: &Path, flat: bool, extra_ignore: &[String]) -> u64 {
    let mut stamp = ListingStamp::default();
    visit_listing(dir, flat, extra_ignore, |path, metadata, kind| {
        let (modified, size) = stats(&metadata);
        let size = if kind.is_none() {
            count_immediate_media(&path) as u64
        } else {
            size
        };
        stamp.add(&path, modified, size);
    });
    stamp.finish()
}

pub(crate) fn stamp_entries(entries: &[Entry]) -> u64 {
    let mut stamp = ListingStamp::default();
    for entry in entries {
        stamp.add(entry.path(), entry.modified(), entry.size());
    }
    stamp.finish()
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

    #[test]
    fn unsorted_poll_stamp_matches_listing_and_detects_media_changes() {
        let dir = temp_tree();
        let ignore = crate::media::default_ignore_list();
        for flat in [false, true] {
            let mut entries = if flat {
                scan_folder_recursive(&dir, &ignore)
            } else {
                scan_browse(&dir, &ignore)
            };
            let original = listing_stamp(&dir, flat, &ignore);
            assert_eq!(original, stamp_entries(&entries));
            entries.reverse();
            assert_eq!(original, stamp_entries(&entries));
            fs::write(dir.join("new.jpg"), b"photo").unwrap();
            assert_ne!(original, listing_stamp(&dir, flat, &ignore));
            fs::remove_file(dir.join("new.jpg")).unwrap();
        }
        let before = listing_stamp(&dir, false, &ignore);
        fs::write(dir.join("pics/c.jpg"), []).unwrap();
        assert_ne!(before, listing_stamp(&dir, false, &ignore));
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn recursive_scan_skips_directory_symlink_cycles_and_keeps_media_symlinks() {
        use std::os::unix::fs::symlink;

        let dir = temp_tree();
        symlink(&dir, dir.join("pics/back-to-root")).unwrap();
        symlink(dir.join("pics/a.jpg"), dir.join("linked.jpg")).unwrap();
        symlink(dir.join("missing.jpg"), dir.join("broken.jpg")).unwrap();
        let ignore = crate::media::default_ignore_list();
        let entries = scan_folder_recursive(&dir, &ignore);
        assert_eq!(entries.len(), 3);
        assert!(entries
            .iter()
            .any(|entry| entry.path() == dir.join("linked.jpg")));
        assert_eq!(listing_stamp(&dir, true, &ignore), stamp_entries(&entries));
        let _ = fs::remove_dir_all(dir);
    }
}
