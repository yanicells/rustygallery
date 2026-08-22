use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use exif::{In, Reader, Tag};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExifInfo {
    pub(crate) rows: Vec<(String, String)>,
}

pub(crate) fn read_exif(path: &Path) -> ExifInfo {
    let mut rows = Vec::new();
    if let Ok((w, h)) = image::image_dimensions(path) {
        rows.push(("Size".into(), format!("{w} × {h}")));
    }
    let Ok(file) = File::open(path) else {
        return ExifInfo { rows };
    };
    let Ok(exif) = Reader::new().read_from_container(&mut BufReader::new(&file)) else {
        return ExifInfo { rows };
    };
    let tags = [
        (Tag::DateTimeOriginal, "Taken"),
        (Tag::DateTime, "Modified"),
        (Tag::Make, "Camera"),
        (Tag::Model, "Model"),
        (Tag::LensModel, "Lens"),
        (Tag::FNumber, "Aperture"),
        (Tag::ExposureTime, "Shutter"),
        (Tag::PhotographicSensitivity, "ISO"),
        (Tag::FocalLength, "Focal length"),
    ];
    for (tag, label) in tags {
        if let Some(field) = exif.get_field(tag, In::PRIMARY) {
            let value = field.display_value().with_unit(&exif).to_string();
            if !value.trim().is_empty() {
                rows.push((label.into(), value));
            }
        }
    }
    ExifInfo { rows }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn empty_file_still_returns_a_panel() {
        let dir = std::env::temp_dir().join(format!(
            "rusty-exif-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("blank.png");
        RgbaImage::new(8, 8).save(&path).unwrap();
        let info = read_exif(&path);
        assert!(info.rows.iter().any(|(k, v)| k == "Size" && v == "8 × 8"));
        let _ = fs::remove_dir_all(dir);
    }
}
