use std::collections::{BTreeSet, HashMap};

use gpui::{Context, Window};

use crate::media::Entry;

use super::{CycleSort, Gallery, NameKind, ToggleSortDir};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortKey {
    Name,
    Modified,
    Size,
    Type,
}

impl SortKey {
    pub(crate) fn as_pref(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Modified => "modified",
            Self::Size => "size",
            Self::Type => "type",
        }
    }

    pub(crate) fn from_pref(value: &str) -> Self {
        match value {
            "modified" => Self::Modified,
            "size" => Self::Size,
            "type" => Self::Type,
            _ => Self::Name,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Modified => "Date",
            Self::Size => "Size",
            Self::Type => "Type",
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Name => Self::Modified,
            Self::Modified => Self::Size,
            Self::Size => Self::Type,
            Self::Type => Self::Name,
        }
    }
}

pub(crate) fn sort_entries(entries: &mut [Entry], key: SortKey, desc: bool) {
    entries.sort_by(|a, b| {
        let a_folder = matches!(a, Entry::Folder(_));
        let b_folder = matches!(b, Entry::Folder(_));
        if a_folder != b_folder {
            return b_folder.cmp(&a_folder);
        }
        let ord = match key {
            SortKey::Name => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
            SortKey::Modified => a.modified().cmp(&b.modified()),
            SortKey::Size => a.size().cmp(&b.size()),
            SortKey::Type => a
                .type_key()
                .cmp(&b.type_key())
                .then_with(|| a.name().to_lowercase().cmp(&b.name().to_lowercase())),
        };
        if desc {
            ord.reverse()
        } else {
            ord
        }
    });
}

impl Gallery {
    pub(super) fn apply_sort(&mut self) {
        let rename = match self.name_kind {
            Some(NameKind::Rename(i)) => Some(i),
            _ => None,
        };
        let mut indices = [
            self.selected,
            self.focused,
            self.anchor,
            rename,
            self.context.as_ref().map(|menu| menu.index),
        ];
        sort_with_selection(
            &mut self.entries,
            self.sort,
            self.sort_desc,
            &mut indices,
            &mut self.checked,
        );
        self.selected = indices[0];
        self.focused = indices[1];
        self.anchor = indices[2];
        if let Some(i) = indices[3] {
            self.name_kind = Some(NameKind::Rename(i));
        }
        if let (Some(menu), Some(i)) = (&mut self.context, indices[4]) {
            menu.index = i;
        }
        self.grid_focus = None;
    }

    fn persist_sort(&mut self) {
        self.prefs.sort = self.sort.as_pref().to_string();
        self.prefs.sort_desc = self.sort_desc;
        self.prefs.save();
    }

    pub(super) fn cycle_sort(&mut self, _: &CycleSort, _: &mut Window, cx: &mut Context<Self>) {
        self.sort = self.sort.next();
        self.apply_sort();
        self.persist_sort();
        cx.notify();
    }

    pub(super) fn toggle_sort_dir(
        &mut self,
        _: &ToggleSortDir,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sort_desc = !self.sort_desc;
        self.apply_sort();
        self.persist_sort();
        cx.notify();
    }
}

/// Preserve file identity across reordering, including the inline player's selection.
fn sort_with_selection(
    entries: &mut [Entry],
    key: SortKey,
    desc: bool,
    indices: &mut [Option<usize>],
    checked: &mut BTreeSet<usize>,
) {
    let path_at = |i: usize| entries.get(i).map(|entry| entry.path().to_path_buf());
    let paths: Vec<_> = indices.iter().map(|i| i.and_then(path_at)).collect();
    let checked_paths: Vec<_> = checked.iter().filter_map(|&i| path_at(i)).collect();
    sort_entries(entries, key, desc);
    let positions: HashMap<_, _> = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| (entry.path(), i))
        .collect();
    for (index, path) in indices.iter_mut().zip(paths) {
        *index = path.and_then(|path| positions.get(path.as_path()).copied());
    }
    *checked = checked_paths
        .iter()
        .filter_map(|path| positions.get(path.as_path()).copied())
        .collect();
}

#[cfg(test)]
mod tests {
    use super::{sort_with_selection, SortKey};
    use crate::media::{Entry, MediaItem, MediaKind};

    #[test]
    fn sorting_keeps_viewer_focus_checks_and_anchor_on_the_same_files() {
        let media = |name: &str, kind| {
            Entry::Media(MediaItem {
                path: format!("/library/{name}").into(),
                name: name.to_string().into(),
                kind,
                modified: 0,
                size: 0,
            })
        };
        let mut entries = vec![
            media("a.jpg", MediaKind::Image),
            media("b.mov", MediaKind::Video),
            media("c.jpg", MediaKind::Image),
        ];
        let mut indices = [Some(1), Some(2), Some(0)];
        let mut checked = [0, 1].into_iter().collect();
        sort_with_selection(
            &mut entries,
            SortKey::Name,
            true,
            &mut indices,
            &mut checked,
        );
        assert_eq!(entries[indices[0].unwrap()].name().as_ref(), "b.mov");
        assert_eq!(entries[indices[1].unwrap()].name().as_ref(), "c.jpg");
        assert_eq!(entries[indices[2].unwrap()].name().as_ref(), "a.jpg");
        assert_eq!(checked, [1, 2].into_iter().collect());
        sort_with_selection(
            &mut entries,
            SortKey::Type,
            false,
            &mut indices,
            &mut checked,
        );
        assert_eq!(entries[indices[0].unwrap()].name().as_ref(), "b.mov");
        assert_eq!(entries[indices[1].unwrap()].name().as_ref(), "c.jpg");
        assert_eq!(checked, [0, 2].into_iter().collect());
    }
}
