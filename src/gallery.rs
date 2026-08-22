use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use gpui::{
    actions, prelude::*, Context, FocusHandle, Image, PathPromptOptions, SharedString, Window,
};

use crate::media::{load_or_make_thumb, scan_browse, scan_folder_recursive, Entry, MediaKind};
use crate::prefs::Prefs;
use crate::ui::SIDEBAR_W;

mod density;
mod grid;
mod lightbox;
mod view;
mod viewer;

use density::Density;
use viewer::ViewerState;

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
}

impl Gallery {
    pub fn new(folder: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        let prefs = Prefs::load();
        let mut gallery = Self {
            root: folder.clone(),
            folder: folder.clone(),
            entries: Vec::new(),
            thumbs: HashMap::new(),
            prefs,
            loading: false,
            load_gen: 0,
            thumb_gen: 0,
            density: Density::Medium,
            focused: None,
            selected: None,
            viewer: ViewerState::default(),
            slideshow: false,
            slideshow_gen: 0,
            focus_handle,
        };
        if std::env::args().nth(1).is_some() {
            gallery.prefs.mark_opened();
        }
        gallery.open_library(folder, true, cx);
        gallery
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
        if self.selected.is_some() || self.entries.is_empty() {
            return;
        }
        let len = self.entries.len() as isize;
        let cur = self.focused.unwrap_or(0) as isize;
        let next = if wrap {
            (cur + delta).rem_euclid(len)
        } else {
            (cur + delta).clamp(0, len - 1)
        };
        self.focused = Some(next as usize);
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
            if matches!(&self.entries[idx], Entry::Media(m) if m.kind == MediaKind::Image) {
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
