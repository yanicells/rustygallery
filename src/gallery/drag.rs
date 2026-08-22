use std::path::PathBuf;

use gpui::{div, prelude::*, rgb, Context, ExternalPaths, Point, Render, Window};

use crate::media::{is_media_path, under_root, Entry};
use crate::ui::Theme;

use super::Gallery;

#[derive(Clone)]
pub(super) struct TileDrag {
    pub(super) paths: Vec<PathBuf>,
}

impl Render for TileDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::DARK;
        let n = self.paths.len();
        let label = if n == 1 {
            self.paths[0]
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("item")
                .to_string()
        } else {
            format!("{n} items")
        };
        div()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(rgb(t.surface))
            .border_1()
            .border_color(rgb(t.accent))
            .text_color(rgb(t.text))
            .text_xs()
            .child(label)
    }
}

impl Gallery {
    pub(super) fn drag_paths(&self, index: usize) -> Vec<PathBuf> {
        if self.checked.contains(&index) {
            return self
                .checked
                .iter()
                .filter_map(|&i| match self.entries.get(i) {
                    Some(Entry::Media(item)) => Some(item.path.clone()),
                    _ => None,
                })
                .collect();
        }
        match self.entries.get(index) {
            Some(Entry::Media(item)) => vec![item.path.clone()],
            _ => Vec::new(),
        }
    }

    pub(super) fn drop_tiles(
        &mut self,
        dest: PathBuf,
        drag: &TileDrag,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.prefs.flat_mode || self.name_kind.is_some() || self.collision.is_some() {
            return;
        }
        let copy = window.modifiers().alt;
        let paths = filter_internal_drop(&drag.paths, &dest);
        self.place_paths(paths, dest, copy, false, cx);
    }

    pub(super) fn drop_external(
        &mut self,
        dest: Option<PathBuf>,
        paths: &ExternalPaths,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.name_kind.is_some() || self.collision.is_some() {
            return;
        }
        let paths = paths.paths();
        let dirs: Vec<PathBuf> = paths.iter().filter(|p| p.is_dir()).cloned().collect();
        let media: Vec<PathBuf> = paths
            .iter()
            .filter(|p| p.is_file() && is_media_path(p))
            .cloned()
            .collect();

        self.drop_hint = None;
        if let Some(dest) = dest {
            if self.prefs.flat_mode {
                return;
            }
            let copy = !window.modifiers().alt;
            self.place_paths(media, dest, copy, true, cx);
            return;
        }

        if media.is_empty() && dirs.len() == 1 {
            self.open_library(dirs[0].clone(), true, cx);
            return;
        }

        if !media.is_empty() && !self.prefs.flat_mode {
            let dest = self.folder.clone();
            if !under_root(&dest, &self.root) {
                return;
            }
            let copy = !window.modifiers().alt;
            self.place_paths(media, dest, copy, true, cx);
        }
    }

    pub(super) fn hint_external(&mut self, paths: &ExternalPaths, cx: &mut Context<Self>) {
        let hint = if paths
            .paths()
            .iter()
            .any(|p| p.is_file() && is_media_path(p))
        {
            Some(super::DropHint::ImportHere)
        } else if paths.paths().iter().any(|p| p.is_dir())
            && paths.paths().iter().all(|p| p.is_dir())
        {
            Some(super::DropHint::OpenLibrary)
        } else {
            None
        };
        if self.drop_hint != hint {
            self.drop_hint = hint;
            cx.notify();
        }
    }

    pub(super) fn clear_drop_hint(&mut self, cx: &mut Context<Self>) {
        if self.drop_hint.take().is_some() {
            cx.notify();
        }
    }
}

fn filter_internal_drop(paths: &[PathBuf], dest: &std::path::Path) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|p| p.parent() != Some(dest) && dest != p.as_path() && !dest.starts_with(p))
        .cloned()
        .collect()
}

pub(super) fn drag_preview(
    drag: &TileDrag,
    _pos: Point<gpui::Pixels>,
    _: &mut Window,
    cx: &mut gpui::App,
) -> gpui::Entity<TileDrag> {
    cx.new(|_| drag.clone())
}

#[cfg(test)]
mod tests {
    use super::filter_internal_drop;
    use std::path::PathBuf;

    #[test]
    fn refuses_drop_into_current_parent_or_self() {
        let dest = PathBuf::from("/lib/album");
        let keep = PathBuf::from("/lib/other/a.jpg");
        let same = PathBuf::from("/lib/album/b.jpg");
        let nested = PathBuf::from("/lib/album");
        let out = filter_internal_drop(&[keep.clone(), same, nested], &dest);
        assert_eq!(out, vec![keep]);
    }
}
