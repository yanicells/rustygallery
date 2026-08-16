use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use gpui::{
    actions, prelude::*, Context, FocusHandle, Image, PathPromptOptions, SharedString,
    Subscription, Window,
};

use crate::media::{load_or_make_thumb, scan_browse, scan_folder_recursive, Entry, MediaKind};
use crate::prefs::Prefs;
use crate::ui::SIDEBAR_W;

mod density;
mod grid;
mod lightbox;
mod search;
mod sort;
mod view;
mod viewer;

use density::Density;
use sort::{sort_entries, SortKey};
use viewer::ViewerState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filter {
    All,
    Images,
    Videos,
}

actions!(
    gallery,
    [
        Quit,
        CloseViewer,
        NextItem,
        PrevItem,
        OpenFocused,
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        OpenFolder,
        GoUp,
        DensitySmall,
        DensityMedium,
        DensityLarge,
        ToggleSlideshow,
        ToggleFlat,
        ToggleSaved,
        ResetZoom,
        CycleSort,
        ToggleSortDir,
        FilterAll,
        FilterImages,
        FilterVideos,
        ToggleSearch,
        CloseSearch,
        ConfirmSearch,
    ]
);

pub(crate) const PAD: f32 = 20.0;
pub(crate) const GAP: f32 = 12.0;

pub struct Gallery {
    root: PathBuf,
    folder: PathBuf,
    entries: Vec<Entry>,
    thumbs: HashMap<PathBuf, Arc<Image>>,
    prefs: Prefs,
    loading: bool,
    load_gen: u64,
    thumb_gen: u64,
    density: Density,
    focused: Option<usize>,
    selected: Option<usize>,
    viewer: ViewerState,
    slideshow: bool,
    slideshow_gen: u64,
    focus_handle: FocusHandle,
    search_focus: FocusHandle,
    filter: Filter,
    sort: SortKey,
    sort_desc: bool,
    search_open: bool,
    search_query: String,
    search_choice: usize,
    _bounds: Option<Subscription>,
}

impl Gallery {
    pub fn new(folder: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        let search_focus = cx.focus_handle();
        let prefs = Prefs::load();
        let density = Density::from_pref(&prefs.density);
        let sort = SortKey::from_pref(&prefs.sort);
        let sort_desc = prefs.sort_desc;
        let mut gallery = Self {
            root: folder.clone(),
            folder: folder.clone(),
            entries: Vec::new(),
            thumbs: HashMap::new(),
            prefs,
            loading: false,
            load_gen: 0,
            thumb_gen: 0,
            density,
            focused: None,
            selected: None,
            viewer: ViewerState::default(),
            slideshow: false,
            slideshow_gen: 0,
            focus_handle,
            search_focus,
            filter: Filter::All,
            sort,
            sort_desc,
            search_open: false,
            search_query: String::new(),
            search_choice: 0,
            _bounds: None,
        };
        gallery._bounds = Some(cx.observe_window_bounds(window, |this, window, _cx| {
            this.persist_window(window);
        }));
        if std::env::args().nth(1).is_some() {
            gallery.prefs.mark_opened();
        }
        gallery.open_library(folder, true, cx);
        gallery
    }

    fn persist_window(&mut self, window: &Window) {
        let bounds = window.window_bounds().get_bounds();
        let x: f32 = bounds.origin.x.into();
        let y: f32 = bounds.origin.y.into();
        let w: f32 = bounds.size.width.into();
        let h: f32 = bounds.size.height.into();
        if self.prefs.set_window(x, y, w, h) {
            self.prefs.save();
        }
    }

    fn open_library(&mut self, folder: PathBuf, set_root: bool, cx: &mut Context<Self>) {
        if set_root {
            self.root = folder.clone();
        }
        self.prefs.touch_recent(&folder);
        self.begin_load(folder, cx);
    }

    fn begin_load(&mut self, folder: PathBuf, cx: &mut Context<Self>) {
        self.folder = folder.clone();
        self.entries.clear();
        self.thumbs.clear();
        self.focused = None;
        self.selected = None;
        self.viewer = ViewerState::default();
        self.stop_slideshow();
        self.loading = true;
        self.load_gen += 1;
        self.thumb_gen += 1;
        let gen = self.load_gen;
        let flat = self.prefs.flat_mode;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let entries = cx
                .background_spawn(async move {
                    if flat {
                        scan_folder_recursive(&folder)
                    } else {
                        scan_browse(&folder)
                    }
                })
                .await;

            this.update(cx, |this, cx| {
                if this.load_gen != gen {
                    return;
                }
                this.entries = entries;
                this.apply_sort();
                this.loading = false;
                this.focused = if this.entries.is_empty() {
                    None
                } else {
                    Some(0)
                };
                this.queue_thumbs(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn queue_thumbs(&mut self, cx: &mut Context<Self>) {
        self.thumb_gen += 1;
        let gen = self.thumb_gen;
        let paths: Vec<PathBuf> = self
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Media(m) if m.kind == MediaKind::Image => Some(m.path.clone()),
                _ => None,
            })
            .collect();

        cx.spawn(async move |this, cx| {
            const BATCH: usize = 8;
            for chunk in paths.chunks(BATCH) {
                let chunk = chunk.to_vec();
                let loaded = cx
                    .background_spawn(async move {
                        chunk
                            .into_iter()
                            .filter_map(|path| {
                                let thumb = load_or_make_thumb(&path)?;
                                Some((path, thumb))
                            })
                            .collect::<Vec<_>>()
                    })
                    .await;

                let cont = this
                    .update(cx, |this, cx| {
                        if this.thumb_gen != gen {
                            return false;
                        }
                        for (path, thumb) in loaded {
                            this.thumbs.insert(path, thumb);
                        }
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !cont {
                    break;
                }
            }
        })
        .detach();
    }

    fn content_width(&self, window: &Window) -> f32 {
        let width: f32 = window.viewport_size().width.into();
        (width - SIDEBAR_W).max(200.0)
    }

    fn layout(&self, window: &Window) -> (usize, f32) {
        let usable = (self.content_width(window) - PAD * 2.0).max(self.density.target());
        let target = self.density.target();
        let cols = ((usable + GAP) / (target + GAP)).floor().max(1.0) as usize;
        let tile = (usable - GAP * (cols.saturating_sub(1) as f32)) / cols as f32;
        (cols, tile.max(72.0))
    }

    fn columns(&self, window: &Window) -> usize {
        self.layout(window).0
    }

    fn can_go_up(&self) -> bool {
        !self.prefs.flat_mode && self.folder != self.root
    }

    fn go_up(&mut self, _: &GoUp, _: &mut Window, cx: &mut Context<Self>) {
        if !self.can_go_up() {
            return;
        }
        if let Some(parent) = self.folder.parent() {
            if parent.starts_with(&self.root) || parent == self.root {
                self.begin_load(parent.to_path_buf(), cx);
            }
        }
    }

    fn open_entry(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(entry) = self.entries.get(index).cloned() else {
            return;
        };
        self.focused = Some(index);
        match entry {
            Entry::Folder(folder) => {
                self.selected = None;
                self.stop_slideshow();
                self.begin_load(folder.path, cx);
            }
            Entry::Media(item) => match item.kind {
                MediaKind::Image => {
                    self.selected = Some(index);
                    self.viewer = ViewerState::default();
                    cx.notify();
                }
                MediaKind::Video => {
                    self.selected = None;
                    self.stop_slideshow();
                    cx.open_with_system(&item.path);
                    cx.notify();
                }
            },
        }
    }

    fn close_viewer(&mut self, _: &CloseViewer, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected.take().is_some() {
            self.viewer = ViewerState::default();
            self.stop_slideshow();
            cx.notify();
        }
    }

    fn open_focused(&mut self, _: &OpenFocused, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_some() {
            return;
        }
        if let Some(i) = self.focused {
            self.open_entry(i, cx);
        }
    }

    fn move_focus(&mut self, delta: isize, wrap: bool, cx: &mut Context<Self>) {
        let vis = self.visible_indices();
        if self.selected.is_some() || vis.is_empty() {
            return;
        }
        let cur = self
            .focused
            .and_then(|f| vis.iter().position(|&i| i == f))
            .unwrap_or(0) as isize;
        let len = vis.len() as isize;
        let next = if wrap {
            (cur + delta).rem_euclid(len)
        } else {
            (cur + delta).clamp(0, len - 1)
        };
        self.focused = Some(vis[next as usize]);
        cx.notify();
    }

    fn on_move_left(&mut self, _: &MoveLeft, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_some() {
            self.step_image(-1, cx);
        } else {
            self.move_focus(-1, true, cx);
        }
    }

    fn on_move_right(&mut self, _: &MoveRight, _window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_some() {
            self.step_image(1, cx);
        } else {
            self.move_focus(1, true, cx);
        }
    }

    fn on_move_up(&mut self, _: &MoveUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_some() {
            return;
        }
        let cols = self.columns(window) as isize;
        self.move_focus(-cols, false, cx);
    }

    fn on_move_down(&mut self, _: &MoveDown, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_some() {
            return;
        }
        let cols = self.columns(window) as isize;
        self.move_focus(cols, false, cx);
    }

    fn next_item(&mut self, _: &NextItem, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_some() {
            self.step_image(1, cx);
        } else if let Some(i) = self.focused {
            self.open_entry(i, cx);
        }
    }

    fn prev_item(&mut self, _: &PrevItem, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_some() {
            self.step_image(-1, cx);
        }
    }

    fn step_image(&mut self, step: isize, cx: &mut Context<Self>) {
        if self.entries.is_empty() {
            return;
        }
        let start = match self.selected {
            Some(i) => i as isize + step,
            None => self.focused.unwrap_or(0) as isize,
        };
        let len = self.entries.len() as isize;
        let mut i = start.rem_euclid(len);
        for _ in 0..self.entries.len() {
            let idx = i as usize;
            if matches!(&self.entries[idx], Entry::Media(m) if m.kind == MediaKind::Image)
                && self.entry_visible(&self.entries[idx])
            {
                self.selected = Some(idx);
                self.focused = Some(idx);
                self.viewer = ViewerState::default();
                cx.notify();
                return;
            }
            i = (i + step).rem_euclid(len);
        }
    }

    fn open_folder_action(&mut self, _: &OpenFolder, _: &mut Window, cx: &mut Context<Self>) {
        self.pick_folder(cx);
    }

    fn pick_folder(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open Folder".into()),
        });

        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let Some(folder) = paths.into_iter().next() else {
                return;
            };
            this.update(cx, |this, cx| {
                this.prefs.mark_opened();
                this.open_library(folder, true, cx);
            })
            .ok();
        })
        .detach();
    }

    fn set_density(&mut self, density: Density, cx: &mut Context<Self>) {
        if self.density != density {
            self.density = density;
            self.prefs.density = density.as_pref().to_string();
            self.prefs.save();
            cx.notify();
        }
    }

    fn density_small(&mut self, _: &DensitySmall, _: &mut Window, cx: &mut Context<Self>) {
        self.set_density(Density::Small, cx);
    }
    fn density_medium(&mut self, _: &DensityMedium, _: &mut Window, cx: &mut Context<Self>) {
        self.set_density(Density::Medium, cx);
    }
    fn density_large(&mut self, _: &DensityLarge, _: &mut Window, cx: &mut Context<Self>) {
        self.set_density(Density::Large, cx);
    }

    fn toggle_flat(&mut self, _: &ToggleFlat, _: &mut Window, cx: &mut Context<Self>) {
        self.prefs.flat_mode = !self.prefs.flat_mode;
        self.prefs.save();
        let folder = self.folder.clone();
        self.begin_load(folder, cx);
    }

    fn toggle_saved(&mut self, _: &ToggleSaved, _: &mut Window, cx: &mut Context<Self>) {
        self.prefs.toggle_saved(&self.root);
        cx.notify();
    }

    fn stop_slideshow(&mut self) {
        self.slideshow = false;
        self.slideshow_gen += 1;
    }

    fn toggle_slideshow(&mut self, _: &ToggleSlideshow, _: &mut Window, cx: &mut Context<Self>) {
        if self.slideshow {
            self.stop_slideshow();
            cx.notify();
            return;
        }
        if self.selected.is_none() {
            self.step_image(1, cx);
        }
        if self.selected.is_none() {
            return;
        }
        self.slideshow = true;
        self.slideshow_gen += 1;
        let gen = self.slideshow_gen;
        cx.notify();

        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(3)).await;
            let cont = this
                .update(cx, |this, cx| {
                    if !this.slideshow || this.slideshow_gen != gen {
                        return false;
                    }
                    this.step_image(1, cx);
                    true
                })
                .unwrap_or(false);
            if !cont {
                break;
            }
        })
        .detach();
    }

    fn reset_zoom(&mut self, _: &ResetZoom, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_some() {
            self.viewer = ViewerState::default();
            cx.notify();
        }
    }

    fn apply_sort(&mut self) {
        sort_entries(&mut self.entries, self.sort, self.sort_desc);
    }

    fn persist_sort(&mut self) {
        self.prefs.sort = self.sort.as_pref().to_string();
        self.prefs.sort_desc = self.sort_desc;
        self.prefs.save();
    }

    pub(crate) fn entry_visible(&self, entry: &Entry) -> bool {
        match self.filter {
            Filter::All => true,
            Filter::Images => matches!(entry, Entry::Media(m) if m.kind == MediaKind::Image),
            Filter::Videos => matches!(entry, Entry::Media(m) if m.kind == MediaKind::Video),
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| self.entry_visible(e).then_some(i))
            .collect()
    }

    fn set_filter(&mut self, filter: Filter, cx: &mut Context<Self>) {
        if self.filter == filter {
            return;
        }
        self.filter = filter;
        let vis = self.visible_indices();
        if let Some(f) = self.focused {
            if !vis.contains(&f) {
                self.focused = vis.first().copied();
            }
        }
        if let Some(s) = self.selected {
            if !vis.contains(&s) {
                self.selected = None;
                self.stop_slideshow();
            }
        }
        cx.notify();
    }

    fn filter_all(&mut self, _: &FilterAll, _: &mut Window, cx: &mut Context<Self>) {
        self.set_filter(Filter::All, cx);
    }
    fn filter_images(&mut self, _: &FilterImages, _: &mut Window, cx: &mut Context<Self>) {
        self.set_filter(Filter::Images, cx);
    }
    fn filter_videos(&mut self, _: &FilterVideos, _: &mut Window, cx: &mut Context<Self>) {
        self.set_filter(Filter::Videos, cx);
    }

    fn cycle_sort(&mut self, _: &CycleSort, _: &mut Window, cx: &mut Context<Self>) {
        self.sort = self.sort.next();
        self.apply_sort();
        self.persist_sort();
        cx.notify();
    }

    fn toggle_sort_dir(&mut self, _: &ToggleSortDir, _: &mut Window, cx: &mut Context<Self>) {
        self.sort_desc = !self.sort_desc;
        self.apply_sort();
        self.persist_sort();
        cx.notify();
    }

    fn search_hits(&self) -> Vec<usize> {
        let q = self.search_query.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                if !self.entry_visible(e) {
                    return None;
                }
                e.name().to_lowercase().contains(&q).then_some(i)
            })
            .collect()
    }

    fn toggle_search(&mut self, _: &ToggleSearch, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            self.close_search(&CloseSearch, window, cx);
            return;
        }
        self.search_open = true;
        self.search_query.clear();
        self.search_choice = 0;
        self.search_focus.focus(window);
        cx.notify();
    }

    fn close_search(&mut self, _: &CloseSearch, window: &mut Window, cx: &mut Context<Self>) {
        if !self.search_open {
            return;
        }
        self.search_open = false;
        self.search_query.clear();
        self.search_choice = 0;
        self.focus_handle.focus(window);
        cx.notify();
    }

    fn confirm_search(&mut self, _: &ConfirmSearch, window: &mut Window, cx: &mut Context<Self>) {
        let hits = self.search_hits();
        let Some(&index) = hits.get(self.search_choice) else {
            self.close_search(&CloseSearch, window, cx);
            return;
        };
        self.close_search(&CloseSearch, window, cx);
        self.open_entry(index, cx);
    }

    fn on_search_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.search_open {
            return;
        }
        let key = event.keystroke.key.as_str();
        if key == "backspace" {
            self.search_query.pop();
            self.search_choice = 0;
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if key == "up" {
            self.search_choice = self.search_choice.saturating_sub(1);
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if key == "down" {
            let last = self.search_hits().len().saturating_sub(1);
            self.search_choice = (self.search_choice + 1).min(last);
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if let Some(ch) = event.keystroke.key_char.as_deref() {
            if ch.chars().all(|c| !c.is_control()) && !event.keystroke.modifiers.control {
                self.search_query.push_str(ch);
                self.search_choice = 0;
                cx.notify();
                cx.stop_propagation();
            }
        }
        let _ = window;
    }

    fn breadcrumb_parts(&self) -> Vec<(SharedString, Option<PathBuf>)> {
        let root_label: SharedString = self
            .root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("library")
            .to_string()
            .into();
        let mut parts = vec![(
            root_label,
            if self.folder == self.root {
                None
            } else {
                Some(self.root.clone())
            },
        )];
        let Ok(rel) = self.folder.strip_prefix(&self.root) else {
            return parts;
        };
        let mut acc = self.root.clone();
        for comp in rel.components() {
            acc.push(comp);
            let label: SharedString = comp.as_os_str().to_string_lossy().into_owned().into();
            let current = acc == self.folder;
            parts.push((label, if current { None } else { Some(acc.clone()) }));
        }
        parts
    }

    fn open_crumb(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if path == self.folder {
            return;
        }
        if path == self.root || path.starts_with(&self.root) {
            self.begin_load(path, cx);
        }
    }
}
