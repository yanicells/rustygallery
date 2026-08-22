use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use gpui::SharedString;

pub(crate) const IMAGE_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff", "heic", "heif", "avif", "jxl",
    "cr2", "cr3", "nef", "arw", "dng", "raf", "orf", "rw2", "raw",
];
pub(crate) const VIDEO_EXTS: &[&str] = &["mp4", "mov", "mkv", "webm", "avi", "m4v"];
pub(crate) const HEIC_EXTS: &[&str] = &["heic", "heif"];
pub(crate) const RAW_EXTS: &[&str] = &[
    "cr2", "cr3", "nef", "arw", "dng", "raf", "orf", "rw2", "raw",
];
pub(crate) const JXL_EXTS: &[&str] = &["jxl"];

pub(crate) fn ext_is(path: &Path, exts: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

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

    pub fn path(&self) -> &Path {
        match self {
            Self::Folder(f) => &f.path,
            Self::Media(m) => &m.path,
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

pub(crate) fn is_media_path(path: &Path) -> bool {
    media_kind(path).is_some()
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
