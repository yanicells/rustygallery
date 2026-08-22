use std::path::{Path, PathBuf};

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
}

#[derive(Clone)]
pub struct FolderItem {
    pub path: PathBuf,
    pub name: SharedString,
    pub media_count: usize,
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
