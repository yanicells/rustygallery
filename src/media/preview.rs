use std::{
    fs,
    hash::{Hash, Hasher},
    io::Cursor,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use super::types::{ext_is, HEIC_EXTS, JXL_EXTS, RAW_EXTS};

pub(crate) fn cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("rusty-gallery-thumbs");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub(crate) fn cache_key(path: &Path) -> Option<u64> {
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

/// JPEG bytes of a downscaled preview. Works for photos, RAW embeds, HEIC, and video posters.
pub fn preview_jpeg(path: &Path, max_edge: u32) -> Option<Vec<u8>> {
    if let Ok(img) = image::open(path) {
        return encode_jpeg(&img, max_edge);
    }
    if let Some(bytes) = embedded_jpeg(path) {
        if let Ok(img) = image::load_from_memory(&bytes) {
            return encode_jpeg(&img, max_edge);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(bytes) = macos_preview(path, max_edge) {
            if let Ok(img) = image::load_from_memory(&bytes) {
                return encode_jpeg(&img, max_edge);
            }
        }
    }
    None
}

/// Path GPUI can paint. Native formats stay as-is; HEIC/RAW/video become a cached JPEG.
pub fn display_source(path: &Path) -> PathBuf {
    if can_paint_directly(path) {
        return path.to_path_buf();
    }
    let Some(key) = cache_key(path) else {
        return path.to_path_buf();
    };
    let dest = cache_dir().join(format!("{key:x}-full.jpg"));
    if dest.exists() && dest.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        return dest;
    }
    if let Some(bytes) = preview_jpeg(path, 2048) {
        let _ = fs::write(&dest, bytes);
        if dest.exists() {
            return dest;
        }
    }
    path.to_path_buf()
}

pub fn is_animated(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(ext.to_ascii_lowercase().as_str(), "gif" | "webp")
}

fn can_paint_directly(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    let ext = ext.to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "avif"
    ) && !ext_is(path, RAW_EXTS)
        && !ext_is(path, HEIC_EXTS)
        && !ext_is(path, JXL_EXTS)
}

fn encode_jpeg(img: &image::DynamicImage, max_edge: u32) -> Option<Vec<u8>> {
    let thumb = img.thumbnail(max_edge, max_edge).to_rgb8();
    let mut bytes = Vec::new();
    {
        let mut cursor = Cursor::new(&mut bytes);
        thumb.write_to(&mut cursor, image::ImageFormat::Jpeg).ok()?;
    }
    (!bytes.is_empty()).then_some(bytes)
}

/// Largest JPEG payload inside a RAW (or similar) container.
pub fn embedded_jpeg(path: &Path) -> Option<Vec<u8>> {
    let data = fs::read(path).ok()?;
    largest_jpeg(&data)
}

fn largest_jpeg(data: &[u8]) -> Option<Vec<u8>> {
    let mut best: Option<&[u8]> = None;
    let mut i = 0;
    while i + 3 < data.len() {
        if data[i] == 0xff && data[i + 1] == 0xd8 && data[i + 2] == 0xff {
            if let Some(end) = find_eoi(&data[i..]) {
                let slice = &data[i..i + end];
                if best.is_none_or(|b| slice.len() > b.len()) {
                    best = Some(slice);
                }
                i += end.max(2);
                continue;
            }
        }
        i += 1;
    }
    best.filter(|b| b.len() > 128).map(|b| b.to_vec())
}

fn find_eoi(data: &[u8]) -> Option<usize> {
    let mut i = 2;
    while i + 1 < data.len() {
        if data[i] == 0xff && data[i + 1] == 0xd9 {
            return Some(i + 2);
        }
        i += 1;
    }
    None
}

#[cfg(target_os = "macos")]
fn macos_preview(path: &Path, max_edge: u32) -> Option<Vec<u8>> {
    let dir = cache_dir().join("ql");
    fs::create_dir_all(&dir).ok()?;
    let status = std::process::Command::new("qlmanage")
        .args(["-t", "-s", &max_edge.to_string(), "-o"])
        .arg(&dir)
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return sips_jpeg(path, max_edge);
    }
    let name = path.file_name()?.to_string_lossy();
    let out = dir.join(format!("{name}.png"));
    let bytes = fs::read(&out).ok().or_else(|| {
        fs::read_dir(&dir).ok()?.find_map(|e| {
            let p = e.ok()?.path();
            p.file_name()?
                .to_str()?
                .starts_with(name.split('.').next()?)
                .then(|| fs::read(p).ok())
                .flatten()
        })
    })?;
    let _ = fs::remove_file(&out);
    Some(bytes)
}

#[cfg(target_os = "macos")]
fn sips_jpeg(path: &Path, max_edge: u32) -> Option<Vec<u8>> {
    let dest = cache_dir().join(format!("sips-{}.jpg", std::process::id()));
    let status = std::process::Command::new("sips")
        .args(["-s", "format", "jpeg", "-Z", &max_edge.to_string()])
        .arg(path)
        .arg("--out")
        .arg(&dest)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    let bytes = fs::read(&dest).ok();
    let _ = fs::remove_file(&dest);
    bytes.filter(|b| !b.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_largest_embedded_jpeg() {
        let mut blob = vec![0, 1, 2, 3];
        blob.extend_from_slice(&[0xff, 0xd8, 0xff, 0x10, 11, 0xff, 0xd9]);
        let mut bigger = vec![0xff, 0xd8, 0xff];
        bigger.extend_from_slice(&[7u8; 200]);
        bigger.extend_from_slice(&[0xff, 0xd9]);
        blob.extend_from_slice(&bigger);
        let found = largest_jpeg(&blob).unwrap();
        assert_eq!(found.len(), bigger.len());
    }

    #[test]
    fn skips_tiny_jpeg_noise() {
        let blob = [0xff, 0xd8, 0xff, 1, 0xff, 0xd9];
        assert!(largest_jpeg(&blob).is_none());
    }
}
