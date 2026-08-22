use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FsError {
    InvalidName,
    Collision,
    OutsideRoot,
    Nested,
    Io,
}

impl FsError {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::InvalidName => "Use a single name, without / or \\.",
            Self::Collision => "That name is already taken.",
            Self::OutsideRoot => "That stays inside this library.",
            Self::Nested => "Can't move a folder into itself.",
            Self::Io => "Could not update the disk.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Collision {
    Fail,
    KeepBoth,
    Replace,
}

pub(crate) fn valid_name(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return false;
    }
    !name.contains('/') && !name.contains('\\') && !name.contains('\0')
}

pub(crate) fn under_root(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

pub(crate) fn unique_numbered(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or(path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("item");
    let ext = path.extension().and_then(|s| s.to_str());
    let name = |n: u32| match ext {
        Some(e) => format!("{stem} {n}.{e}"),
        None => format!("{stem} {n}"),
    };
    for n in 2..10_000 {
        let candidate = parent.join(name(n));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(name(std::process::id()))
}

pub(crate) fn unique_copy_name(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or(path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("item");
    let ext = path.extension().and_then(|s| s.to_str());
    let with = |extra: &str| match ext {
        Some(e) => parent.join(format!("{stem} {extra}.{e}")),
        None => parent.join(format!("{stem} {extra}")),
    };
    let first = with("copy");
    if !first.exists() {
        return first;
    }
    for n in 2..10_000 {
        let candidate = with(&format!("copy {n}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    with("copy")
}

pub(crate) fn count_tree(path: &Path) -> usize {
    if !path.is_dir() {
        return 0;
    }
    let mut n = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            n += 1;
            if entry.path().is_dir() {
                stack.push(entry.path());
            }
        }
    }
    n
}

fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn copy_any(from: &Path, to: &Path) -> Result<(), FsError> {
    if from.is_dir() {
        fs::create_dir(to).map_err(|_| FsError::Io)?;
        let entries = fs::read_dir(from).map_err(|_| FsError::Io)?;
        for entry in entries {
            let entry = entry.map_err(|_| FsError::Io)?;
            copy_any(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(from, to).map(|_| ()).map_err(|_| FsError::Io)
    }
}

fn remove_any(path: &Path) -> Result<(), FsError> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|_| FsError::Io)
    } else {
        fs::remove_file(path).map_err(|_| FsError::Io)
    }
}

fn relocate_move(from: &Path, to: &Path) -> Result<(), FsError> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_any(from, to)?;
            remove_any(from)
        }
    }
}

fn prepare_dest(dest: &Path, collision: Collision, root: &Path) -> Result<PathBuf, FsError> {
    if !dest.exists() {
        return Ok(dest.to_path_buf());
    }
    match collision {
        Collision::Fail => Err(FsError::Collision),
        Collision::KeepBoth => Ok(unique_numbered(dest)),
        Collision::Replace => {
            trash_path(dest, root)?;
            Ok(dest.to_path_buf())
        }
    }
}

fn guarded(path: &Path, root: &Path) -> Result<(), FsError> {
    if !under_root(path, root) || path == root {
        Err(FsError::OutsideRoot)
    } else {
        Ok(())
    }
}

pub(crate) fn create_folder(parent: &Path, name: &str, root: &Path) -> Result<PathBuf, FsError> {
    if !valid_name(name) {
        return Err(FsError::InvalidName);
    }
    if !under_root(parent, root) {
        return Err(FsError::OutsideRoot);
    }
    let dest = parent.join(name.trim());
    if dest.exists() {
        return Err(FsError::Collision);
    }
    fs::create_dir(&dest).map_err(|_| FsError::Io)?;
    Ok(dest)
}

pub(crate) fn rename_path(from: &Path, new_name: &str, root: &Path) -> Result<PathBuf, FsError> {
    rename_with(from, new_name, root, Collision::Fail)
}

pub(crate) fn rename_with(
    from: &Path,
    new_name: &str,
    root: &Path,
    collision: Collision,
) -> Result<PathBuf, FsError> {
    if !valid_name(new_name) {
        return Err(FsError::InvalidName);
    }
    guarded(from, root)?;
    let parent = from.parent().ok_or(FsError::Io)?;
    if !under_root(parent, root) {
        return Err(FsError::OutsideRoot);
    }
    let dest = parent.join(new_name.trim());
    if same_path(from, &dest) {
        return Ok(from.to_path_buf());
    }
    let dest = prepare_dest(&dest, collision, root)?;
    relocate_move(from, &dest)?;
    Ok(dest)
}

pub(crate) fn move_into(
    from: &Path,
    dest_dir: &Path,
    root: &Path,
    collision: Collision,
) -> Result<PathBuf, FsError> {
    guarded(from, root)?;
    if !under_root(dest_dir, root) {
        return Err(FsError::OutsideRoot);
    }
    if dest_dir == from || dest_dir.starts_with(from) {
        return Err(FsError::Nested);
    }
    let dest = dest_dir.join(from.file_name().ok_or(FsError::Io)?);
    if same_path(from, &dest) {
        return Ok(from.to_path_buf());
    }
    let dest = prepare_dest(&dest, collision, root)?;
    relocate_move(from, &dest)?;
    Ok(dest)
}

pub(crate) fn copy_into(
    from: &Path,
    dest_dir: &Path,
    root: &Path,
    collision: Collision,
) -> Result<PathBuf, FsError> {
    guarded(from, root)?;
    place_into(from, dest_dir, root, collision, true)
}

/// Copy or move a path into `dest_dir`. Source may live outside the library (Finder drop).
pub(crate) fn import_into(
    from: &Path,
    dest_dir: &Path,
    root: &Path,
    collision: Collision,
    copy: bool,
) -> Result<PathBuf, FsError> {
    if !from.exists() {
        return Err(FsError::Io);
    }
    place_into(from, dest_dir, root, collision, copy)
}

fn place_into(
    from: &Path,
    dest_dir: &Path,
    root: &Path,
    collision: Collision,
    copy: bool,
) -> Result<PathBuf, FsError> {
    if !under_root(dest_dir, root) {
        return Err(FsError::OutsideRoot);
    }
    if dest_dir == from || dest_dir.starts_with(from) {
        return Err(FsError::Nested);
    }
    let dest = dest_dir.join(from.file_name().ok_or(FsError::Io)?);
    if !copy && same_path(from, &dest) {
        return Ok(from.to_path_buf());
    }
    let dest = if copy && same_path(from, &dest) {
        unique_copy_name(from)
    } else {
        prepare_dest(&dest, collision, root)?
    };
    if copy {
        copy_any(from, &dest)?;
    } else {
        relocate_move(from, &dest)?;
    }
    Ok(dest)
}

pub(crate) fn duplicate(from: &Path, root: &Path) -> Result<PathBuf, FsError> {
    guarded(from, root)?;
    if from.is_dir() {
        return Err(FsError::Io);
    }
    let dest = unique_copy_name(from);
    copy_any(from, &dest)?;
    Ok(dest)
}

pub(crate) fn trash_path(path: &Path, root: &Path) -> Result<PathBuf, FsError> {
    guarded(path, root)?;
    #[cfg(target_os = "macos")]
    {
        trash_macos(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        trash_fallback(path)
    }
}

pub(crate) fn restore_path(from: &Path, to: &Path, root: &Path) -> Result<PathBuf, FsError> {
    if !under_root(from, root) {
        return Err(FsError::OutsideRoot);
    }
    if to.exists() && !same_path(from, to) {
        return Err(FsError::Collision);
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|_| FsError::Io)?;
    }
    relocate_move(from, to)?;
    Ok(to.to_path_buf())
}

#[cfg(target_os = "macos")]
fn trash_macos(path: &Path) -> Result<PathBuf, FsError> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};

    let s = path.to_str().ok_or(FsError::Io)?;
    let ns_path = NSString::from_str(s);
    let url = NSURL::fileURLWithPath(&ns_path);
    let mgr = NSFileManager::defaultManager();
    let mut resulting = None;
    mgr.trashItemAtURL_resultingItemURL_error(&url, Some(&mut resulting))
        .map_err(|_| FsError::Io)?;
    let resulting = resulting.ok_or(FsError::Io)?;
    let ns = resulting.path().ok_or(FsError::Io)?;
    Ok(PathBuf::from(ns.to_string()))
}

#[cfg(not(target_os = "macos"))]
fn trash_fallback(path: &Path) -> Result<PathBuf, FsError> {
    let dir = std::env::temp_dir().join("rusty-gallery-trash");
    fs::create_dir_all(&dir).map_err(|_| FsError::Io)?;
    let dest = unique_numbered(&dir.join(path.file_name().ok_or(FsError::Io)?));
    relocate_move(path, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rusty-fs-{tag}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn rejects_empty_dots_and_separators() {
        assert!(!valid_name(""));
        assert!(!valid_name("   "));
        assert!(!valid_name("."));
        assert!(!valid_name(".."));
        assert!(!valid_name("a/b"));
        assert!(!valid_name("a\\b"));
        assert!(valid_name("holiday"));
        assert!(valid_name("shot 01.jpg"));
    }

    #[test]
    fn create_folder_writes_under_root_and_rejects_collisions() {
        let root = temp_dir("create");
        let made = create_folder(&root, " album ", &root).unwrap();
        assert_eq!(made, root.join("album"));
        assert!(made.is_dir());
        assert_eq!(
            create_folder(&root, "album", &root),
            Err(FsError::Collision)
        );
        let outside = temp_dir("outside");
        assert_eq!(
            create_folder(&outside, "nope", &root),
            Err(FsError::OutsideRoot)
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn rename_path_moves_the_file_and_blocks_taken_names() {
        let root = temp_dir("rename");
        let src = root.join("old.jpg");
        fs::write(&src, []).unwrap();
        fs::write(root.join("taken.jpg"), []).unwrap();
        let dest = rename_path(&src, "new.jpg", &root).unwrap();
        assert_eq!(dest, root.join("new.jpg"));
        assert!(dest.is_file());
        assert!(!src.exists());
        assert_eq!(
            rename_path(&dest, "taken.jpg", &root),
            Err(FsError::Collision)
        );
        assert_eq!(
            rename_path(&dest, "../escape.jpg", &root),
            Err(FsError::InvalidName)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn keep_both_numbers_the_copy() {
        let root = temp_dir("both");
        fs::write(root.join("shot.jpg"), b"a").unwrap();
        let numbered = unique_numbered(&root.join("shot.jpg"));
        assert_eq!(numbered, root.join("shot 2.jpg"));
        fs::write(&numbered, b"b").unwrap();
        assert_eq!(
            unique_numbered(&root.join("shot.jpg")),
            root.join("shot 3.jpg")
        );
        let copy = unique_copy_name(&root.join("shot.jpg"));
        assert_eq!(copy, root.join("shot copy.jpg"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn move_and_copy_respect_collisions() {
        let root = temp_dir("relocate");
        let albums = root.join("albums");
        fs::create_dir(&albums).unwrap();
        let src = root.join("a.jpg");
        fs::write(&src, b"1").unwrap();
        fs::write(albums.join("a.jpg"), b"2").unwrap();
        assert_eq!(
            move_into(&src, &albums, &root, Collision::Fail),
            Err(FsError::Collision)
        );
        let kept = move_into(&src, &albums, &root, Collision::KeepBoth).unwrap();
        assert_eq!(kept, albums.join("a 2.jpg"));
        assert!(!src.exists());
        let src2 = root.join("b.jpg");
        fs::write(&src2, b"3").unwrap();
        let copied = copy_into(&src2, &albums, &root, Collision::Fail).unwrap();
        assert_eq!(copied, albums.join("b.jpg"));
        assert!(src2.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn import_copies_from_outside_the_library() {
        let root = temp_dir("import-root");
        let outside = temp_dir("import-src");
        let src = outside.join("shot.jpg");
        fs::write(&src, b"pic").unwrap();
        let dest = import_into(&src, &root, &root, Collision::Fail, true).unwrap();
        assert_eq!(dest, root.join("shot.jpg"));
        assert_eq!(fs::read(&dest).unwrap(), b"pic");
        assert!(src.exists());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn duplicate_never_overwrites() {
        let root = temp_dir("dup");
        let src = root.join("img.jpg");
        fs::write(&src, b"x").unwrap();
        let a = duplicate(&src, &root).unwrap();
        assert_eq!(a, root.join("img copy.jpg"));
        let b = duplicate(&src, &root).unwrap();
        assert_eq!(b, root.join("img copy 2.jpg"));
        assert_eq!(fs::read(&src).unwrap(), b"x");
        let _ = fs::remove_dir_all(root);
    }
}
