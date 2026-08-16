use std::{
    fs,
    hash::{Hash, Hasher},
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
    time::UNIX_EPOCH,
};

use gpui::{Image, ImageFormat};

const THUMB_MAX: u32 = 320;

fn cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("rusty-gallery-thumbs");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn thumb_key(path: &Path) -> Option<u64> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    modified.hash(&mut hasher);
    meta.len().hash(&mut hasher);
    Some(hasher.finish())
}

/// Decode and downscale an image on a background thread. Returns GPUI image bytes.
pub fn load_or_make_thumb(path: &Path) -> Option<Arc<Image>> {
    let key = thumb_key(path)?;
    let cache_path = cache_dir().join(format!("{key:x}.jpg"));

    if let Ok(bytes) = fs::read(&cache_path) {
        if !bytes.is_empty() {
            return Some(Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes)));
        }
    }

    let img = image::open(path).ok()?;
    let thumb = img.thumbnail(THUMB_MAX, THUMB_MAX);
    let mut bytes = Vec::new();
    {
        let mut cursor = Cursor::new(&mut bytes);
        thumb.write_to(&mut cursor, image::ImageFormat::Jpeg).ok()?;
    }
    let _ = fs::write(&cache_path, &bytes);
    Some(Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes)))
}
