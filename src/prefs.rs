use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Prefs {
    pub recents: Vec<PathBuf>,
    pub saved: Vec<PathBuf>,
    pub flat_mode: bool,
    pub seen_open: bool,
    pub density: String,
    pub sort: String,
    pub sort_desc: bool,
    pub window: Option<(f32, f32, f32, f32)>,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            recents: Vec::new(),
            saved: Vec::new(),
            flat_mode: false,
            seen_open: false,
            density: "medium".into(),
            sort: "name".into(),
            sort_desc: false,
            window: None,
        }
    }
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
        let mut win_x = None;
        let mut win_y = None;
        let mut win_w = None;
        let mut win_h = None;
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
                "[flags]" => match line {
                    "flat=1" => prefs.flat_mode = true,
                    "flat=0" => prefs.flat_mode = false,
                    "seen_open=1" => prefs.seen_open = true,
                    _ => {
                        if let Some(value) = line.strip_prefix("density=") {
                            prefs.density = value.to_string();
                        } else if let Some(value) = line.strip_prefix("sort=") {
                            prefs.sort = value.to_string();
                        } else if line == "sort_desc=1" {
                            prefs.sort_desc = true;
                        }
                    }
                },
                "[window]" => {
                    if let Some(value) = line.strip_prefix("x=") {
                        win_x = value.parse().ok();
                    } else if let Some(value) = line.strip_prefix("y=") {
                        win_y = value.parse().ok();
                    } else if let Some(value) = line.strip_prefix("w=") {
                        win_w = value.parse().ok();
                    } else if let Some(value) = line.strip_prefix("h=") {
                        win_h = value.parse().ok();
                    }
                }
                _ => {}
            }
        }
        prefs.recents.retain(|p| p.is_dir());
        prefs.saved.retain(|p| p.is_dir());
        if !prefs.recents.is_empty() {
            prefs.seen_open = true;
        }
        if let (Some(x), Some(y), Some(w), Some(h)) = (win_x, win_y, win_w, win_h) {
            prefs.window = Self::valid_window(x, y, w, h);
        }
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
        if self.seen_open {
            out.push_str("seen_open=1\n");
        }
        out.push_str("density=");
        out.push_str(&self.density);
        out.push('\n');
        out.push_str("sort=");
        out.push_str(&self.sort);
        out.push('\n');
        if self.sort_desc {
            out.push_str("sort_desc=1\n");
        }
        if let Some((x, y, w, h)) = self.window {
            out.push_str("\n[window]\n");
            out.push_str(&format!("x={x}\ny={y}\nw={w}\nh={h}\n"));
        }
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

    pub fn mark_opened(&mut self) {
        if self.seen_open {
            return;
        }
        self.seen_open = true;
        self.save();
    }

    pub fn valid_window(x: f32, y: f32, w: f32, h: f32) -> Option<(f32, f32, f32, f32)> {
        if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
            return None;
        }
        if w < 400.0 || h < 300.0 {
            return None;
        }
        Some((x, y, w, h))
    }

    pub fn set_window(&mut self, x: f32, y: f32, w: f32, h: f32) -> bool {
        let Some(next) = Self::valid_window(x.round(), y.round(), w.round(), h.round()) else {
            return false;
        };
        if self.window == Some(next) {
            return false;
        }
        self.window = Some(next);
        true
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

#[cfg(test)]
mod tests {
    use super::Prefs;

    #[test]
    fn rejects_tiny_or_invalid_window() {
        assert!(Prefs::valid_window(10.0, 10.0, 100.0, 100.0).is_none());
        assert!(Prefs::valid_window(f32::NAN, 10.0, 800.0, 600.0).is_none());
        assert_eq!(
            Prefs::valid_window(12.0, 40.0, 1200.0, 800.0),
            Some((12.0, 40.0, 1200.0, 800.0))
        );
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
