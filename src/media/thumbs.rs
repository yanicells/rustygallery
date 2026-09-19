use std::{fs, io::Cursor, path::Path, sync::Arc};

use gpui::{Image, ImageFormat};

use super::preview::{cache_dir, cache_key, preview_jpeg};

const THUMB_MAX: u32 = 320;

/// Decode and downscale a still, RAW embed, HEIC, or video poster.
pub fn load_or_make_thumb(path: &Path) -> Option<Arc<Image>> {
    let key = cache_key(path)?;
    let cache_path = cache_dir().join(format!("{key:x}.jpg"));

    if let Ok(bytes) = fs::read(&cache_path) {
        if !bytes.is_empty() {
            return Some(Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes)));
        }
    }

    let bytes = preview_jpeg(path, THUMB_MAX)?;
    let _ = fs::write(&cache_path, &bytes);
    Some(Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes)))
}

pub fn first_frame_image(path: &Path) -> Option<Arc<Image>> {
    let img = image::open(path).ok()?;
    let rgb = img.to_rgb8();
    let mut bytes = Vec::new();
    {
        let mut cursor = Cursor::new(&mut bytes);
        rgb.write_to(&mut cursor, image::ImageFormat::Jpeg).ok()?;
    }
    (!bytes.is_empty()).then(|| Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_png() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rusty-thumb-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("alpha.png");
        let mut img = RgbaImage::new(16, 16);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([40, 180, 90, 128]);
        }
        img.save(&path).unwrap();
        path
    }

    #[test]
    fn encodes_png_with_alpha_as_jpeg_thumb() {
        let path = temp_png();
        let thumb = load_or_make_thumb(&path);
        assert!(thumb.is_some(), "alpha PNG should still produce a thumb");
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
