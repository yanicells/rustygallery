use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Default)]
pub struct Prefs {
    pub recents: Vec<PathBuf>,
    pub saved: Vec<PathBuf>,
    pub flat_mode: bool,
}

impl Prefs {
    fn path() -> PathBuf {
        let base = dirs_next();
        base.join("prefs.txt")
    }

    pub fn load() -> Self {
        let Ok(text) = fs::read_to_string(Self::path()) else {
            return Self::default();
        };
        let mut prefs = Self::default();
        let mut section = "";
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = line;
                continue;
            }
            match section {
                "[recents]" => prefs.recents.push(PathBuf::from(line)),
                "[saved]" => prefs.saved.push(PathBuf::from(line)),
                "[flags]" if line == "flat=1" => prefs.flat_mode = true,
                "[flags]" if line == "flat=0" => prefs.flat_mode = false,
                _ => {}
            }
        }
        prefs.recents.retain(|p| p.is_dir());
        prefs.saved.retain(|p| p.is_dir());
        prefs
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut out = String::new();
        out.push_str("[flags]\n");
        out.push_str(if self.flat_mode {
            "flat=1\n"
        } else {
            "flat=0\n"
        });
        out.push_str("\n[recents]\n");
        for p in self.recents.iter().take(12) {
            out.push_str(&p.to_string_lossy());
            out.push('\n');
        }
        out.push_str("\n[saved]\n");
        for p in &self.saved {
            out.push_str(&p.to_string_lossy());
            out.push('\n');
        }
        let _ = fs::write(path, out);
    }

    pub fn touch_recent(&mut self, folder: &Path) {
        let folder = folder.to_path_buf();
        self.recents.retain(|p| p != &folder);
        self.recents.insert(0, folder);
        self.recents.truncate(12);
        self.save();
    }

    pub fn is_saved(&self, folder: &Path) -> bool {
        self.saved.iter().any(|p| p == folder)
    }

    pub fn toggle_saved(&mut self, folder: &Path) {
        if self.is_saved(folder) {
            self.saved.retain(|p| p != folder);
        } else {
            self.saved.insert(0, folder.to_path_buf());
        }
        self.save();
    }
}

fn dirs_next() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("rusty-gallery");
    }
    std::env::temp_dir().join("rusty-gallery")
}
