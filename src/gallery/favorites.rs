use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use gpui::{Context, Window};

use super::{Filter, Gallery, ToggleStar};

impl Gallery {
    pub(super) fn is_favorite(&self, path: &Path) -> bool {
        self.prefs.is_favorite(path)
    }

    pub(super) fn toggle_star(&mut self, _: &ToggleStar, _: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self
            .selected
            .or(self.focused)
            .and_then(|i| self.entries.get(i))
            .map(|e| e.path().to_path_buf())
        else {
            return;
        };
        if path.is_dir() {
            return;
        }
        self.star_path(path, cx);
    }

    pub(super) fn star_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.prefs.toggle_favorite(&path);
        if self.filter == Filter::Favorites {
            let visible = self.visible_indices();
            let mut indices = [self.selected, self.focused, self.anchor];
            reconcile_visible_selection(&visible, &mut indices, &mut self.checked);
            self.focused = indices[1];
            self.anchor = indices[2];
            self.grid_focus = None;
            if self.selected != indices[0] {
                if let Some(index) = indices[0] {
                    let peek = self.viewer.peek;
                    self.open_entry(index, cx);
                    if peek && self.selected.is_some() && !self.is_video_at(index) {
                        self.viewer.peek = true;
                    }
                } else {
                    self.reset_viewer(cx);
                    self.selected = None;
                    self.stop_slideshow();
                }
            }
        }
        cx.notify();
    }
}

/// Reconcile open, focused, and range-anchor indices after an item is hidden.
fn reconcile_visible_selection(
    visible: &[usize],
    indices: &mut [Option<usize>; 3],
    checked: &mut BTreeSet<usize>,
) {
    let hidden = |index| !visible.contains(&index);
    let next = |index| {
        visible
            .iter()
            .copied()
            .find(|&i| i > index)
            .or_else(|| visible.first().copied())
    };
    if indices[0].is_some_and(hidden) {
        indices[0] = indices[0].and_then(next);
        indices[1] = indices[0];
    } else if indices[1].is_some_and(hidden) {
        indices[1] = indices[1].and_then(next);
    }
    if indices[2].is_some_and(hidden) {
        indices[2] = indices[1];
    }
    checked.retain(|i| visible.contains(i));
}

#[cfg(test)]
mod tests {
    use super::reconcile_visible_selection;

    #[test]
    fn hiding_open_starred_media_moves_to_the_next_visible_item() {
        let mut indices = [Some(2), Some(2), Some(2)];
        let mut checked = [0, 2, 5].into_iter().collect();
        reconcile_visible_selection(&[0, 5, 8], &mut indices, &mut checked);
        assert_eq!(indices, [Some(5), Some(5), Some(5)]);
        assert_eq!(checked, [0, 5].into_iter().collect());

        indices = [Some(8), Some(8), Some(8)];
        reconcile_visible_selection(&[0, 5], &mut indices, &mut checked);
        assert_eq!(indices, [Some(0), Some(0), Some(0)]);
    }

    #[test]
    fn hiding_the_last_starred_item_clears_the_selection() {
        let mut indices = [Some(2), Some(2), Some(2)];
        let mut checked = [2].into_iter().collect();
        reconcile_visible_selection(&[], &mut indices, &mut checked);
        assert_eq!(indices, [None, None, None]);
        assert!(checked.is_empty());
    }

    #[test]
    fn hiding_focused_media_keeps_the_grid_closed_and_preserves_visible_checks() {
        let mut indices = [None, Some(2), Some(2)];
        let mut checked = [0, 2, 5].into_iter().collect();
        reconcile_visible_selection(&[0, 5], &mut indices, &mut checked);
        assert_eq!(indices, [None, Some(5), Some(5)]);
        assert_eq!(checked, [0, 5].into_iter().collect());
    }

    #[test]
    fn visible_selection_survives_other_favorite_changes() {
        let mut indices = [Some(5), Some(5), Some(0)];
        let mut checked = [0, 5].into_iter().collect();
        reconcile_visible_selection(&[0, 5, 8], &mut indices, &mut checked);
        assert_eq!(indices, [Some(5), Some(5), Some(0)]);
        assert_eq!(checked, [0, 5].into_iter().collect());
    }
}
