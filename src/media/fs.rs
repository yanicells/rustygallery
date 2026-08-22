use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FsError {
    InvalidName,
    Collision,
    OutsideRoot,
    Io,
}

impl FsError {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::InvalidName => "Use a single name, without / or \\.",
            Self::Collision => "That name is already taken.",
            Self::OutsideRoot => "That stays inside this library.",
            Self::Io => "Could not update the disk.",
        }
    }
}

pub(crate) fn valid_name(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return false;
    }
    !name.contains('/') && !name.contains('\\') && !name.contains('\0')
}

fn under_root(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
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
    if !valid_name(new_name) {
        return Err(FsError::InvalidName);
    }
    if !under_root(from, root) {
        return Err(FsError::OutsideRoot);
    }
    let parent = from.parent().ok_or(FsError::Io)?;
    if !under_root(parent, root) {
        return Err(FsError::OutsideRoot);
    }
    let dest = parent.join(new_name.trim());
    if dest.exists() {
        let from_c = fs::canonicalize(from).ok();
        let dest_c = fs::canonicalize(&dest).ok();
        if from_c.is_none() || from_c != dest_c {
            return Err(FsError::Collision);
        }
    }
    fs::rename(from, &dest).map_err(|_| FsError::Io)?;
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
}
