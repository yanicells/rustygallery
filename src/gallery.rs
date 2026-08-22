use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use gpui::{
    actions, prelude::*, Context, FocusHandle, Image, PathPromptOptions, Pixels, Point,
    SharedString, Subscription, Window,
};

use crate::media::{
    create_folder, load_or_make_thumb, scan_browse, scan_folder_recursive, stamp_entries, Entry,
    MediaKind,
};
use crate::prefs::Prefs;
use crate::ui::SIDEBAR_W;

mod collision;
mod context;
mod density;
mod drag;
mod grid;
mod lightbox;
mod name;
mod ops;
mod search;
mod sort;
mod toast;
mod view;
mod viewer;
mod watch;

use density::Density;
use ops::{Clip, CollisionAsk, Toast};
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
        RevealInFinder,
        CopyPath,
        NewFolder,
        RenameFocused,
        CloseName,
        ConfirmName,
        MoveToTrash,
        Duplicate,
        CutSelection,
        CopySelection,
        PasteSelection,
        MoveTo,
        CopyTo,
        Undo,
    ]
);

pub(crate) const PAD: f32 = 20.0;
pub(crate) const GAP: f32 = 12.0;

#[derive(Clone)]
enum NameKind {
    NewFolder,
    NewFolderIn(PathBuf),
    Rename(usize),
}

struct TileMenu {
    index: usize,
    pos: Point<Pixels>,
}

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
    checked: BTreeSet<usize>,
    anchor: Option<usize>,
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
    name_focus: FocusHandle,
    name_kind: Option<NameKind>,
    name_query: String,
    name_error: Option<String>,
    context: Option<TileMenu>,
    reload_focus: Option<PathBuf>,
    reload_open: bool,
    clip: Option<Clip>,
    toast: Option<Toast>,
    toast_gen: u64,
    collision: Option<CollisionAsk>,
    watch_stamp: Option<u64>,
    drop_hint: Option<DropHint>,
    _bounds: Option<Subscription>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DropHint {
    OpenLibrary,
    ImportHere,
}

impl Gallery {
    pub fn new(folder: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        let search_focus = cx.focus_handle();
        let name_focus = cx.focus_handle();
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
            checked: BTreeSet::new(),
            anchor: None,
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
            name_focus,
            name_kind: None,
            name_query: String::new(),
            name_error: None,
            context: None,
            reload_focus: None,
            reload_open: false,
            clip: None,
            toast: None,
            toast_gen: 0,
            collision: None,
            watch_stamp: None,
            drop_hint: None,
            _bounds: None,
        };
        gallery._bounds = Some(cx.observe_window_bounds(window, |this, window, _cx| {
            this.persist_window(window);
        }));
        if std::env::args().nth(1).is_some() {
            gallery.prefs.mark_opened();
        }
        gallery.open_library(folder, true, cx);
        gallery.start_watch(cx);
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
        self.reload_focus = None;
        self.reload_open = false;
        if matches!(self.clip, Some(Clip::Cut(_))) {
            self.clip = None;
        }
        self.load_folder(folder, cx);
    }

    fn reload_listing(
        &mut self,
        folder: PathBuf,
        focus: PathBuf,
        open: bool,
        cx: &mut Context<Self>,
    ) {
        self.reload_focus = Some(focus);
        self.reload_open = open;
        self.load_folder(folder, cx);
    }

    fn load_folder(&mut self, folder: PathBuf, cx: &mut Context<Self>) {
        self.folder = folder.clone();
        self.entries.clear();
        self.thumbs.clear();
        self.focused = None;
        self.checked.clear();
        self.anchor = None;
        self.selected = None;
        self.viewer = ViewerState::default();
        self.stop_slideshow();
        self.loading = true;
        self.load_gen += 1;
        self.thumb_gen += 1;
        self.watch_stamp = None;
        let gen = self.load_gen;
        let flat = self.prefs.flat_mode;
        let ignore = self.prefs.ignore.clone();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let entries = cx
                .background_spawn(async move {
                    if flat {
                        scan_folder_recursive(&folder, &ignore)
                    } else {
                        scan_browse(&folder, &ignore)
                    }
                })
                .await;

            this.update(cx, |this, cx| {
                if this.load_gen != gen {
                    return;
                }
                this.entries = entries;
                this.apply_sort();
                this.watch_stamp = Some(stamp_entries(&this.entries));
                this.loading = false;
                let restore = this.reload_focus.take();
                let reopen = std::mem::take(&mut this.reload_open);
                this.checked.clear();
                this.anchor = None;
                this.selected = None;
                this.viewer = ViewerState::default();
                if let Some(path) = restore {
                    this.focused = this.entries.iter().position(|e| e.path() == path);
                    if let Some(i) = this.focused {
                        this.checked.insert(i);
                        this.anchor = Some(i);
                        if reopen
                            && matches!(
                                &this.entries[i],
                                Entry::Media(m) if m.kind == MediaKind::Image
                            )
                        {
                            this.selected = Some(i);
                        }
                    } else if !this.entries.is_empty() {
                        this.focused = Some(0);
                    }
                } else {
                    this.focused = if this.entries.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                }
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
        self.checked.clear();
        self.checked.insert(index);
        self.anchor = Some(index);
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
        if self.context.take().is_some() {
            cx.notify();
            return;
        }
        if self.collision.is_some() {
            self.cancel_collision(cx);
            return;
        }
        if matches!(self.clip, Some(Clip::Cut(_))) {
            self.clip = None;
            cx.notify();
            return;
        }
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
        self.checked.retain(|i| vis.contains(i));
        if let Some(a) = self.anchor {
            if !vis.contains(&a) {
                self.anchor = self.focused;
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

    fn click_tile(&mut self, index: usize, event: &gpui::ClickEvent, cx: &mut Context<Self>) {
        let mods = event.modifiers();
        if mods.secondary() {
            self.toggle_checked(index, cx);
            return;
        }
        if mods.shift {
            self.range_check(index, cx);
            return;
        }
        self.open_entry(index, cx);
    }

    fn toggle_checked(&mut self, index: usize, cx: &mut Context<Self>) {
        if !self.checked.remove(&index) {
            self.checked.insert(index);
        }
        self.focused = Some(index);
        self.anchor = Some(index);
        cx.notify();
    }

    fn range_check(&mut self, index: usize, cx: &mut Context<Self>) {
        let vis = self.visible_indices();
        if vis.is_empty() {
            return;
        }
        let start = self.anchor.or(self.focused).unwrap_or(index);
        self.checked.clear();
        for i in range_select(&vis, start, index) {
            self.checked.insert(i);
        }
        self.focused = Some(index);
        cx.notify();
    }

    fn action_paths(&self) -> Vec<PathBuf> {
        let idxs = if !self.checked.is_empty() {
            self.checked.iter().copied().collect()
        } else if let Some(i) = self.selected.or(self.focused) {
            vec![i]
        } else {
            Vec::new()
        };
        idxs.into_iter()
            .filter_map(|i| self.entries.get(i).map(|e| e.path().to_path_buf()))
            .collect()
    }

    fn reveal_target(&self) -> PathBuf {
        if !self.checked.is_empty() {
            if let Some(f) = self.focused {
                if self.checked.contains(&f) {
                    if let Some(path) = self.entries.get(f).map(|e| e.path().to_path_buf()) {
                        return path;
                    }
                }
            }
            if let Some(path) = self
                .checked
                .iter()
                .find_map(|&i| self.entries.get(i).map(|e| e.path().to_path_buf()))
            {
                return path;
            }
        }
        if let Some(i) = self.selected.or(self.focused) {
            if let Some(path) = self.entries.get(i).map(|e| e.path().to_path_buf()) {
                return path;
            }
        }
        self.folder.clone()
    }

    fn reveal_in_finder(&mut self, _: &RevealInFinder, _: &mut Window, cx: &mut Context<Self>) {
        cx.reveal_path(&self.reveal_target());
    }

    fn copy_path(&mut self, _: &CopyPath, _: &mut Window, cx: &mut Context<Self>) {
        let paths = self.action_paths();
        if paths.is_empty() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                self.folder.display().to_string(),
            ));
            return;
        }
        let text = paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
    }

    fn thumb_progress(&self) -> Option<(usize, usize)> {
        let total = self
            .entries
            .iter()
            .filter(|e| matches!(e, Entry::Media(m) if m.kind == MediaKind::Image))
            .count();
        if total == 0 {
            return None;
        }
        Some((self.thumbs.len().min(total), total))
    }

    fn status_left(&self, folders: usize, media: usize) -> SharedString {
        if self.loading {
            return "Loading…".into();
        }
        let mut parts = Vec::new();
        if self.prefs.flat_mode {
            parts.push(format!("{media} media"));
        } else {
            parts.push(format!("{folders} folders · {media} media"));
        }
        if !self.checked.is_empty() {
            parts.push(format!("{} selected", self.checked.len()));
        }
        if let Some(Clip::Cut(paths)) = &self.clip {
            parts.push(format!("{} cut", paths.len()));
        }
        if let Some((done, total)) = self.thumb_progress() {
            if done < total {
                parts.push(format!("thumbs {done}/{total}"));
            }
        }
        parts.join(" · ").into()
    }

    fn status_path(&self) -> SharedString {
        if let Some(i) = self.focused {
            if let Some(entry) = self.entries.get(i) {
                return entry.path().display().to_string().into();
            }
        }
        self.folder.display().to_string().into()
    }

    fn entry_file_name(entry: &Entry) -> String {
        entry
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    fn open_name(&mut self, kind: NameKind, window: &mut Window, cx: &mut Context<Self>) {
        self.context = None;
        self.search_open = false;
        self.search_query.clear();
        self.search_choice = 0;
        self.name_query = match &kind {
            NameKind::NewFolder | NameKind::NewFolderIn(_) => String::new(),
            NameKind::Rename(index) => self
                .entries
                .get(*index)
                .map(Self::entry_file_name)
                .unwrap_or_default(),
        };
        self.name_error = None;
        self.name_kind = Some(kind);
        self.name_focus.focus(window);
        cx.notify();
    }

    fn close_name(&mut self, _: &CloseName, window: &mut Window, cx: &mut Context<Self>) {
        if self.name_kind.take().is_none() {
            return;
        }
        self.name_query.clear();
        self.name_error = None;
        self.focus_handle.focus(window);
        cx.notify();
    }

    fn new_folder(&mut self, _: &NewFolder, window: &mut Window, cx: &mut Context<Self>) {
        if self.prefs.flat_mode || self.name_kind.is_some() {
            return;
        }
        self.open_name(NameKind::NewFolder, window, cx);
    }

    fn rename_focused(&mut self, _: &RenameFocused, window: &mut Window, cx: &mut Context<Self>) {
        if self.name_kind.is_some() {
            return;
        }
        let Some(index) = self.selected.or(self.focused) else {
            return;
        };
        if self.entries.get(index).is_none() {
            return;
        }
        self.open_name(NameKind::Rename(index), window, cx);
    }

    fn confirm_name(&mut self, _: &ConfirmName, window: &mut Window, cx: &mut Context<Self>) {
        let Some(kind) = self.name_kind.clone() else {
            return;
        };
        let name = self.name_query.clone();
        let result = match kind {
            NameKind::NewFolder => {
                if self.prefs.flat_mode {
                    self.close_name(&CloseName, window, cx);
                    return;
                }
                create_folder(&self.folder, &name, &self.root)
                    .map(|path| (self.folder.clone(), path, false))
            }
            NameKind::NewFolderIn(parent) => {
                create_folder(&parent, &name, &self.root).map(|path| (parent, path, false))
            }
            NameKind::Rename(index) => {
                let Some(from) = self.entries.get(index).map(|e| e.path().to_path_buf()) else {
                    self.close_name(&CloseName, window, cx);
                    return;
                };
                let open = self.selected == Some(index);
                self.close_name(&CloseName, window, cx);
                self.begin_rename_with_collision(from, name, open, cx);
                return;
            }
        };
        match result {
            Ok((folder, focus, open)) => {
                self.close_name(&CloseName, window, cx);
                self.reload_listing(folder, focus, open, cx);
            }
            Err(err) => {
                self.name_error = Some(err.as_str().to_string());
                cx.notify();
            }
        }
    }

    fn on_name_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.name_kind.is_none() {
            return;
        }
        let key = event.keystroke.key.as_str();
        if key == "backspace" {
            self.name_query.pop();
            self.name_error = None;
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if let Some(ch) = event.keystroke.key_char.as_deref() {
            if ch.chars().all(|c| !c.is_control()) && !event.keystroke.modifiers.control {
                self.name_query.push_str(ch);
                self.name_error = None;
                cx.notify();
                cx.stop_propagation();
            }
        }
        let _ = window;
    }

    fn open_tile_menu(&mut self, index: usize, pos: Point<Pixels>, cx: &mut Context<Self>) {
        if self.selected.is_some() || self.name_kind.is_some() || self.search_open {
            return;
        }
        if !self.checked.contains(&index) {
            self.checked.clear();
            self.checked.insert(index);
            self.anchor = Some(index);
        }
        self.focused = Some(index);
        self.context = Some(TileMenu { index, pos });
        cx.notify();
    }

    fn dismiss_context(&mut self, cx: &mut Context<Self>) {
        if self.context.take().is_some() {
            cx.notify();
        }
    }

    fn context_open(&mut self, index: usize, cx: &mut Context<Self>) {
        self.context = None;
        self.open_entry(index, cx);
    }

    fn context_rename(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.context = None;
        self.open_name(NameKind::Rename(index), window, cx);
    }

    fn context_new_folder(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Entry::Folder(folder)) = self.entries.get(index).cloned() else {
            self.dismiss_context(cx);
            return;
        };
        self.context = None;
        self.open_name(NameKind::NewFolderIn(folder.path), window, cx);
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

fn range_select(visible: &[usize], anchor: usize, to: usize) -> Vec<usize> {
    let a = visible.iter().position(|&i| i == anchor).unwrap_or(0);
    let b = visible.iter().position(|&i| i == to).unwrap_or(0);
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    visible[lo..=hi].to_vec()
}

#[cfg(test)]
mod tests {
    use super::range_select;

    #[test]
    fn shift_range_follows_visible_order() {
        let vis = [0, 2, 5, 8];
        assert_eq!(range_select(&vis, 2, 8), vec![2, 5, 8]);
        assert_eq!(range_select(&vis, 8, 0), vec![0, 2, 5, 8]);
        assert_eq!(range_select(&vis, 5, 5), vec![5]);
    }
}
