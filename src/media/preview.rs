use std::{
    fs,
    hash::{Hash, Hasher},
    io::{self, Cursor, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::UNIX_EPOCH,
};

#[cfg(any(target_os = "macos", test))]
use std::{
    process::{Child, Command, ExitStatus},
    thread,
    time::{Duration, Instant, SystemTime},
};

use gpui::{Image, ImageFormat, ImageSource};

use super::types::{ext_is, HEIC_EXTS, JXL_EXTS, RAW_EXTS};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "macos")]
const CONVERSION_TIMEOUT: Duration = Duration::from_secs(10);

/// Send-safe preview result; GPUI image sources are created on the UI thread.
pub(crate) enum PreviewSource {
    Path(PathBuf),
    Jpeg(Arc<Image>),
}

impl PreviewSource {
    pub(crate) fn dimensions(&self) -> Option<(u32, u32)> {
        match self {
            Self::Path(path) => image::image_dimensions(path).ok(),
            Self::Jpeg(image) => image::ImageReader::with_format(
                Cursor::new(image.bytes()),
                image::ImageFormat::Jpeg,
            )
            .into_dimensions()
            .ok(),
        }
    }
}

impl From<PreviewSource> for ImageSource {
    fn from(source: PreviewSource) -> Self {
        match source {
            PreviewSource::Path(path) => path.into(),
            PreviewSource::Jpeg(image) => image.into(),
        }
    }
}

pub(crate) fn cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("rusty-gallery-thumbs");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Publish complete cache bytes atomically, including when requests overlap.
pub(super) fn publish_jpeg(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pending = dest.with_extension(format!("pending-{}-{id}", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)?;
    let result = file.write_all(bytes);
    drop(file);
    let result = result.and_then(|()| fs::rename(&pending, dest));
    let _ = fs::remove_file(&pending);
    result
}

pub(crate) fn cache_key(path: &Path) -> Option<u64> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
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
    if ext_is(path, RAW_EXTS) {
        if let Some(bytes) = embedded_jpeg(path) {
            if let Ok(img) = image::load_from_memory(&bytes) {
                return encode_jpeg(&img, max_edge);
            }
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

/// Native formats retain their path and animation; converted previews can survive cache failures.
pub(crate) fn display_source(path: &Path) -> PreviewSource {
    if can_paint_directly(path) {
        return PreviewSource::Path(path.to_path_buf());
    }
    converted_source(path, &cache_dir())
}

fn converted_source(path: &Path, cache: &Path) -> PreviewSource {
    let dest = cache_key(path).map(|key| cache.join(format!("{key:x}-full.jpg")));
    if let Some(dest) = dest.as_ref() {
        if image::open(dest).is_ok() {
            return PreviewSource::Path(dest.clone());
        }
    }
    if let Some(bytes) = preview_jpeg(path, 2048) {
        if let Some(dest) = dest {
            if publish_jpeg(&dest, &bytes).is_ok() {
                return PreviewSource::Path(dest);
            }
        }
        return PreviewSource::Jpeg(Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes)));
    }
    PreviewSource::Path(path.to_path_buf())
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
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tif" | "tiff"
    ) && !ext_is(path, RAW_EXTS)
        && !ext_is(path, HEIC_EXTS)
        && !ext_is(path, JXL_EXTS)
}

fn encode_jpeg(img: &image::DynamicImage, max_edge: u32) -> Option<Vec<u8>> {
    let thumb = if img.width() > max_edge || img.height() > max_edge {
        img.thumbnail(max_edge, max_edge).to_rgb8()
    } else {
        img.to_rgb8()
    };
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
    let dir = unique_temp_dir("ql")?;
    let status = run_with_timeout(
        Command::new("qlmanage")
            .args(["-t", "-s", &max_edge.to_string(), "-o"])
            .arg(&dir.path)
            .arg(path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
        CONVERSION_TIMEOUT,
    )
    .ok();
    let bytes = if status.is_some_and(|status| status.success()) {
        let name = path.file_name()?.to_string_lossy();
        let out = dir.path.join(format!("{name}.png"));
        fs::read(&out).ok().or_else(|| {
            fs::read_dir(&dir.path).ok()?.find_map(|e| {
                let p = e.ok()?.path();
                p.is_file().then(|| fs::read(p).ok()).flatten()
            })
        })
    } else {
        None
    };
    drop(dir);
    if bytes
        .as_deref()
        .is_some_and(|bytes| image::load_from_memory(bytes).is_ok())
    {
        bytes
    } else {
        sips_jpeg(path, max_edge)
    }
}

#[cfg(target_os = "macos")]
fn sips_jpeg(path: &Path, max_edge: u32) -> Option<Vec<u8>> {
    let dir = unique_temp_dir("sips")?;
    let dest = dir.path.join("preview.jpg");
    let status = run_with_timeout(
        Command::new("sips")
            .args(["-s", "format", "jpeg", "-Z", &max_edge.to_string()])
            .arg(path)
            .arg("--out")
            .arg(&dest)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
        CONVERSION_TIMEOUT,
    )
    .ok();
    if !status.is_some_and(|status| status.success()) {
        return None;
    }
    let bytes = fs::read(&dest).ok();
    bytes.filter(|b| !b.is_empty())
}

#[cfg(any(target_os = "macos", test))]
fn run_with_timeout(command: &mut Command, timeout: Duration) -> io::Result<ExitStatus> {
    let mut child = command.spawn()?;
    wait_with_timeout(&mut child, timeout)
}

#[cfg(any(target_os = "macos", test))]
fn wait_with_timeout(child: &mut Child, timeout: Duration) -> io::Result<ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        let error = match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(
                    deadline
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(20)),
                );
                continue;
            }
            Ok(None) => io::Error::new(io::ErrorKind::TimedOut, "preview conversion timed out"),
            Err(error) => error,
        };
        // Reap even if kill races with the child's exit or the status poll failed.
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
}

#[cfg(any(target_os = "macos", test))]
struct PreviewTempDir {
    path: PathBuf,
}

#[cfg(any(target_os = "macos", test))]
impl Drop for PreviewTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(any(target_os = "macos", test))]
fn unique_temp_dir(prefix: &str) -> Option<PreviewTempDir> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    for _ in 0..8 {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rusty-gallery-{prefix}-{now}-{id}"));
        if fs::create_dir(&dir).is_ok() {
            return Some(PreviewTempDir { path: dir });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_fixture(dir: &Path) -> PathBuf {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            24,
            12,
            image::Rgb([40, 180, 90]),
        ));
        let mut bytes = b"RAW container fixture".to_vec();
        bytes.extend(encode_jpeg(&img, 24).unwrap());
        let path = dir.join("sample.cr3");
        fs::write(&path, bytes).unwrap();
        path
    }

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

    #[test]
    fn routes_avif_through_preview_conversion() {
        assert!(!can_paint_directly(Path::new("photo.avif")));
        assert!(can_paint_directly(Path::new("photo.jpg")));
    }

    #[test]
    fn embedded_scan_is_reserved_for_raw_extensions() {
        assert!(ext_is(Path::new("photo.cr3"), RAW_EXTS));
        assert!(!ext_is(Path::new("clip.mp4"), RAW_EXTS));
    }

    #[test]
    fn preserves_native_sources_including_animation() {
        for ext in ["jpg", "png", "gif", "webp", "bmp", "tif", "tiff"] {
            let path = PathBuf::from(format!("native.{ext}"));
            let source = ImageSource::from(display_source(&path));
            let ImageSource::Resource(gpui::Resource::Path(resource)) = source else {
                panic!("native {ext} must retain its resource loader");
            };
            assert_eq!(resource.as_ref(), path.as_path());
        }
        assert!(is_animated(Path::new("native.gif")));
        assert!(is_animated(Path::new("native.webp")));
    }

    #[test]
    fn regenerates_corrupt_full_preview_cache() {
        let dir = unique_temp_dir("full-cache-test").unwrap();
        let path = raw_fixture(&dir.path);
        let dest = dir
            .path
            .join(format!("{:x}-full.jpg", cache_key(&path).unwrap()));
        fs::write(&dest, b"incomplete cached JPEG").unwrap();

        let source = converted_source(&path, &dir.path);
        let PreviewSource::Path(source) = source else {
            panic!("a writable cache should publish its converted preview");
        };
        assert_eq!(source, dest);
        assert!(image::open(&source).is_ok());
        assert!(matches!(
            converted_source(&path, &dir.path),
            PreviewSource::Path(cached) if cached == dest
        ));
    }

    #[test]
    fn retains_jpeg_bytes_when_cache_publication_fails() {
        let dir = unique_temp_dir("full-cache-failure-test").unwrap();
        let path = raw_fixture(&dir.path);
        let dest = dir
            .path
            .join(format!("{:x}-full.jpg", cache_key(&path).unwrap()));
        // Existing invalid cache entries must not win merely because they exist.
        // A directory blocks publication deterministically, even when run as root.
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("partial"), b"incomplete cached JPEG").unwrap();
        let unavailable_cache = dir.path.join("unavailable-cache");
        fs::write(&unavailable_cache, b"blocks the cache directory").unwrap();

        for cache in [&dir.path, &unavailable_cache] {
            let source = converted_source(&path, cache);
            assert_eq!(source.dimensions(), Some((24, 12)));
            let ImageSource::Image(image) = ImageSource::from(source) else {
                panic!("failed publication must retain an in-memory preview");
            };
            assert!(image::load_from_memory(image.bytes()).is_ok());
        }
        assert!(dest.is_dir());
        assert_eq!(
            fs::read(unavailable_cache).unwrap(),
            b"blocks the cache directory"
        );
    }

    #[test]
    fn cache_publication_handles_overlapping_writers_and_cleans_failures() {
        let dir = unique_temp_dir("cache-publication-test").unwrap();
        let dest = dir.path.join("preview.jpg");
        fs::write(&dest, b"incomplete cached JPEG").unwrap();
        let previews = [image::Rgb([180, 40, 90]), image::Rgb([40, 180, 90])].map(|color| {
            let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(24, 12, color));
            encode_jpeg(&img, 24).unwrap()
        });
        let barrier = std::sync::Barrier::new(3);
        thread::scope(|scope| {
            for bytes in &previews {
                let barrier = &barrier;
                let dest = &dest;
                scope.spawn(move || {
                    barrier.wait();
                    publish_jpeg(dest, bytes).unwrap();
                });
            }
            barrier.wait();
        });
        let published = fs::read(&dest).unwrap();
        assert!(previews.contains(&published));
        assert!(image::open(&dest).is_ok());

        let blocked = dir.path.join("blocked.jpg");
        fs::create_dir(&blocked).unwrap();
        assert!(publish_jpeg(&blocked, &previews[0]).is_err());
        assert_eq!(fs::read_dir(&dir.path).unwrap().count(), 2);
    }

    #[test]
    fn cache_key_distinguishes_changes_within_one_second() {
        let dir = unique_temp_dir("cache-key-test").unwrap();
        let path = dir.path.join("photo.jpg");
        fs::write(&path, b"same length").unwrap();
        let file = fs::File::open(&path).unwrap();
        let second = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        file.set_times(fs::FileTimes::new().set_modified(second + Duration::from_millis(100)))
            .unwrap();
        let before = cache_key(&path).unwrap();
        file.set_times(fs::FileTimes::new().set_modified(second + Duration::from_millis(700)))
            .unwrap();
        assert_ne!(before, cache_key(&path).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn converter_timeout_kills_reaps_and_cleans_partial_output() {
        let dir = unique_temp_dir("deadline-test").unwrap();
        let temp_path = dir.path.clone();
        fs::write(temp_path.join("partial.jpg"), b"unfinished preview").unwrap();
        let mut child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let started = Instant::now();
        let error = wait_with_timeout(&mut child, Duration::from_millis(20)).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(2));
        // A killed but unreaped child still exists as a zombie. Check before
        // invoking another wait/try_wait, which could mask a missing reap.
        assert!(!Command::new("/bin/kill")
            .args(["-0", &child.id().to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success());
        drop(dir);
        assert!(!temp_path.exists());
        let status = run_with_timeout(
            Command::new("/bin/sh").args(["-c", "exit 0"]),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(
            status.success(),
            "a timed-out converter must not block the next job"
        );
    }

    #[cfg(unix)]
    #[test]
    fn converter_reports_exit_and_spawn_failures() {
        let dir = unique_temp_dir("converter-failure-test").unwrap();
        let status = run_with_timeout(
            Command::new("/bin/sh").args(["-c", "exit 7"]),
            Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(status.code(), Some(7));
        let error = run_with_timeout(
            &mut Command::new(dir.path.join("missing-converter")),
            Duration::from_millis(20),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}
