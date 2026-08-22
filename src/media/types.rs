use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use gpui::SharedString;

pub(crate) const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff"];
pub(crate) const VIDEO_EXTS: &[&str] = &["mp4", "mov", "mkv", "webm", "avi", "m4v"];

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
    pub modified: u64,
    pub size: u64,
}

#[derive(Clone)]
pub struct FolderItem {
    pub path: PathBuf,
    pub name: SharedString,
    pub media_count: usize,
    pub modified: u64,
}

#[derive(Clone)]
pub enum Entry {
    Folder(FolderItem),
    Media(MediaItem),
}

impl Entry {
    pub fn name(&self) -> &SharedString {
        match self {
            Self::Folder(f) => &f.name,
            Self::Media(m) => &m.name,
        }
    }

    pub fn modified(&self) -> u64 {
        match self {
            Self::Folder(f) => f.modified,
            Self::Media(m) => m.modified,
        }
    }

    pub fn size(&self) -> u64 {
        match self {
            Self::Folder(f) => f.media_count as u64,
            Self::Media(m) => m.size,
        }
    }

    pub fn type_key(&self) -> u8 {
        match self {
            Self::Folder(_) => 0,
            Self::Media(m) if m.kind == MediaKind::Image => 1,
            Self::Media(_) => 2,
        }
    }
}

pub(super) fn file_stats(path: &Path) -> (u64, u64) {
    let Ok(meta) = std::fs::metadata(path) else {
        return (0, 0);
    };
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (modified, meta.len())
}

pub(super) fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

pub(super) fn media_kind(path: &Path) -> Option<MediaKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if IMAGE_EXTS.iter().any(|e| *e == ext) {
        Some(MediaKind::Image)
    } else if VIDEO_EXTS.iter().any(|e| *e == ext) {
        Some(MediaKind::Video)
    } else {
        None
    }
}
