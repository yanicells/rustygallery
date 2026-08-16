use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use gpui::{
    actions, div, img, point, prelude::*, px, relative, rgb, ClickEvent, Context, FocusHandle,
    Image, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit, PathPromptOptions,
    Pixels, Point, ScrollWheelEvent, SharedString, Window,
};

use crate::media::{load_or_make_thumb, scan_browse, scan_folder_recursive, Entry, MediaKind};
use crate::prefs::Prefs;
use crate::ui::{btn, sidebar_row, Theme, SIDEBAR_W};

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

const PAD: f32 = 20.0;
const GAP: f32 = 12.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Density {
    Small,
    Medium,
    Large,
}

impl Density {
    fn target(self) -> f32 {
        match self {
            Self::Small => 120.0,
            Self::Medium => 176.0,
            Self::Large => 248.0,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Small => "S",
            Self::Medium => "M",
            Self::Large => "L",
        }
    }
}

struct ViewerState {
    zoom: f32,
    pan: Point<Pixels>,
    dragging: bool,
    drag_last: Point<Pixels>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: point(px(0.), px(0.)),
            dragging: false,
            drag_last: point(px(0.), px(0.)),
        }
    }
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

    fn on_viewer_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected.is_none() {
            return;
        }
        let dy: f32 = match event.delta {
            gpui::ScrollDelta::Pixels(p) => p.y.into(),
            gpui::ScrollDelta::Lines(p) => p.y * 40.0,
        };
        let factor = if dy > 0.0 { 1.1 } else { 1.0 / 1.1 };
        let old = self.viewer.zoom;
        self.viewer.zoom = (old * factor).clamp(1.0, 8.0);
        if self.viewer.zoom <= 1.01 {
            self.viewer.zoom = 1.0;
            self.viewer.pan = point(px(0.), px(0.));
        }
        cx.notify();
    }

    fn on_viewer_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected.is_none() || event.button != MouseButton::Left {
            return;
        }
        if event.click_count >= 2 {
            self.viewer = ViewerState::default();
            cx.notify();
            return;
        }
        if self.viewer.zoom > 1.0 {
            self.viewer.dragging = true;
            self.viewer.drag_last = event.position;
            cx.notify();
        }
    }

    fn on_viewer_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.viewer.dragging {
            return;
        }
        let dx = event.position.x - self.viewer.drag_last.x;
        let dy = event.position.y - self.viewer.drag_last.y;
        self.viewer.pan.x += dx;
        self.viewer.pan.y += dy;
        self.viewer.drag_last = event.position;
        cx.notify();
    }

    fn on_viewer_up(&mut self, _: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.viewer.dragging {
            self.viewer.dragging = false;
            cx.notify();
        }
    }

    fn render_tile(
        &self,
        index: usize,
        entry: &Entry,
        tile: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let focused = self.focused == Some(index);
        let name = entry.name().clone();
        let t = Theme::DARK;

        let media = match entry {
            Entry::Folder(_) => div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .bg(rgb(t.tile_folder))
                .text_color(rgb(t.accent_soft))
                .child(div().text_xl().child("📁"))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(t.text_muted))
                        .child("folder"),
                )
                .into_any_element(),
            Entry::Media(item) if item.kind == MediaKind::Video => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(t.tile_media))
                .text_color(rgb(t.btn_text))
                .text_lg()
                .child("▶")
                .into_any_element(),
            Entry::Media(item) => {
                if let Some(thumb) = self.thumbs.get(&item.path).cloned() {
                    img(thumb)
                        .id(("thumb", index))
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .into_any_element()
                } else {
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(rgb(t.tile_media))
                        .text_color(rgb(t.text_hint))
                        .text_xs()
                        .child("…")
                        .into_any_element()
                }
            }
        };

        div()
            .id(("tile", index))
            .w(px(tile))
            .flex()
            .flex_col()
            .gap_1()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.open_entry(index, cx);
            }))
            .child(
                div()
                    .w(px(tile))
                    .h(px(tile))
                    .overflow_hidden()
                    .rounded_md()
                    .bg(rgb(t.tile))
                    .border_2()
                    .when(focused, |s| s.border_color(rgb(t.accent)))
                    .when(!focused, |s| s.border_color(rgb(t.tile)))
                    .child(media),
            )
            .child(
                div()
                    .w(px(tile))
                    .px_1()
                    .text_xs()
                    .text_color(if focused {
                        rgb(t.accent)
                    } else {
                        rgb(t.name_idle)
                    })
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(name),
            )
    }

    fn render_lightbox(&self, index: usize, cx: &Context<Self>) -> impl IntoElement {
        let Entry::Media(item) = &self.entries[index] else {
            return div().into_any_element();
        };
        let zoom = self.viewer.zoom;
        let pan = self.viewer.pan;
        let slideshow = self.slideshow;
        let t = Theme::DARK;
        let label = format!(
            "{}  ·  {} / {}  ·  {:.0}%{}",
            item.name,
            index + 1,
            self.entries.len(),
            zoom * 100.0,
            if slideshow { "  ·  slideshow" } else { "" }
        );

        div()
            .id("lightbox")
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .bg(rgb(t.lightbox))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(t.accent_soft))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(label),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(btn(
                                "slide-btn",
                                if slideshow { "Stop" } else { "Slideshow" },
                                slideshow,
                                false,
                                cx,
                                |this, _, window, cx| {
                                    this.toggle_slideshow(&ToggleSlideshow, window, cx);
                                },
                            ))
                            .child(btn(
                                "close-btn",
                                "Close",
                                false,
                                false,
                                cx,
                                |this, _, _, cx| {
                                    this.selected = None;
                                    this.viewer = ViewerState::default();
                                    this.stop_slideshow();
                                    cx.notify();
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .id("lightbox-body")
                    .flex_1()
                    .w_full()
                    .relative()
                    .overflow_hidden()
                    .on_scroll_wheel(cx.listener(Self::on_viewer_scroll))
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::on_viewer_down))
                    .on_mouse_move(cx.listener(Self::on_viewer_move))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::on_viewer_up))
                    .child(
                        div()
                            .absolute()
                            .left(pan.x)
                            .top(pan.y)
                            .w(relative(zoom))
                            .h(relative(zoom))
                            .child(
                                img(item.path.clone())
                                    .id(("full", index))
                                    .size_full()
                                    .object_fit(ObjectFit::Contain),
                            ),
                    ),
            )
            .child(
                div()
                    .px_4()
                    .py_2()
                    .text_xs()
                    .text_color(rgb(t.text_dim))
                    .child("Scroll zoom · drag pan · double-click reset · ← → · S slideshow"),
            )
            .into_any_element()
    }

    fn breadcrumb(&self) -> SharedString {
        let rel = self
            .folder
            .strip_prefix(&self.root)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .filter(|s| !s.is_empty());
        match rel {
            Some(rel) => format!(
                "{} / {}",
                self.root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("library"),
                rel
            )
            .into(),
            None => self
                .root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("library")
                .to_string()
                .into(),
        }
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (_, tile) = self.layout(window);
        let count = self.entries.len();
        let selected = self.selected;
        let density = self.density;
        let loading = self.loading;
        let slideshow = self.slideshow;
        let flat = self.prefs.flat_mode;
        let saved = self.prefs.is_saved(&self.root);
        let crumb = self.breadcrumb();
        let folder_full: SharedString = self.folder.display().to_string().into();

        let folders = self
            .entries
            .iter()
            .filter(|e| matches!(e, Entry::Folder(_)))
            .count();
        let media = count.saturating_sub(folders);
        let status: SharedString = if loading {
            "Loading…".into()
        } else if flat {
            format!("{media} media").into()
        } else {
            format!("{folders} folders · {media} media").into()
        };

        let recents = self.prefs.recents.clone();
        let saved_list = self.prefs.saved.clone();
        let current_root = self.root.clone();
        let t = Theme::DARK;

        let root = div()
            .id("gallery")
            .key_context("Gallery")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::close_viewer))
            .on_action(cx.listener(Self::next_item))
            .on_action(cx.listener(Self::prev_item))
            .on_action(cx.listener(Self::open_focused))
            .on_action(cx.listener(Self::on_move_left))
            .on_action(cx.listener(Self::on_move_right))
            .on_action(cx.listener(Self::on_move_up))
            .on_action(cx.listener(Self::on_move_down))
            .on_action(cx.listener(Self::open_folder_action))
            .on_action(cx.listener(Self::go_up))
            .on_action(cx.listener(Self::density_small))
            .on_action(cx.listener(Self::density_medium))
            .on_action(cx.listener(Self::density_large))
            .on_action(cx.listener(Self::toggle_slideshow))
            .on_action(cx.listener(Self::toggle_flat))
            .on_action(cx.listener(Self::toggle_saved))
            .on_action(cx.listener(Self::reset_zoom))
            .size_full()
            .flex()
            .flex_row()
            .bg(rgb(t.bg))
            .text_color(rgb(t.text))
            // Sidebar
            .child(
                div()
                    .id("sidebar")
                    .w(px(SIDEBAR_W))
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .px_3()
                    .py_3()
                    .border_r_1()
                    .border_color(rgb(t.border))
                    .bg(rgb(t.surface))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("gallery"),
                            )
                            .child(btn(
                                "open-sidebar",
                                "Open Folder",
                                false,
                                true,
                                cx,
                                |this, _, _, cx| this.pick_folder(cx),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .px_2()
                                    .text_xs()
                                    .text_color(rgb(t.text_faint))
                                    .child("SAVED"),
                            )
                            .when(saved_list.is_empty(), |s| {
                                s.child(
                                    div()
                                        .px_2()
                                        .text_xs()
                                        .text_color(rgb(t.text_hint))
                                        .child("Pin a library with Save"),
                                )
                            })
                            .children(saved_list.into_iter().enumerate().map(|(i, path)| {
                                let label: SharedString = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("folder")
                                    .to_string()
                                    .into();
                                let active = path == current_root;
                                sidebar_row(
                                    ("saved", i),
                                    label,
                                    active,
                                    cx,
                                    move |this, _, _, cx| {
                                        this.open_library(path.clone(), true, cx);
                                    },
                                )
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .px_2()
                                    .text_xs()
                                    .text_color(rgb(t.text_faint))
                                    .child("RECENT"),
                            )
                            .children(recents.into_iter().enumerate().map(|(i, path)| {
                                let label: SharedString = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("folder")
                                    .to_string()
                                    .into();
                                let active = path == current_root;
                                sidebar_row(
                                    ("recent", i),
                                    label,
                                    active,
                                    cx,
                                    move |this, _, _, cx| {
                                        this.open_library(path.clone(), true, cx);
                                    },
                                )
                            })),
                    ),
            )
            // Main
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(rgb(t.border))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .min_w_0()
                                            .flex_1()
                                            .child(btn(
                                                "back",
                                                "← Back",
                                                false,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.go_up(&GoUp, window, cx);
                                                },
                                            ))
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .min_w_0()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .overflow_hidden()
                                                            .whitespace_nowrap()
                                                            .child(crumb),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(rgb(t.text_dim))
                                                            .overflow_hidden()
                                                            .whitespace_nowrap()
                                                            .child(folder_full),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(btn(
                                                "save",
                                                if saved { "Saved ★" } else { "Save" },
                                                saved,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.toggle_saved(&ToggleSaved, window, cx);
                                                },
                                            ))
                                            .child(btn(
                                                "flat",
                                                if flat { "Flat" } else { "Folders" },
                                                flat,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.toggle_flat(&ToggleFlat, window, cx);
                                                },
                                            ))
                                            .child(
                                                div()
                                                    .flex()
                                                    .gap_1()
                                                    .child(btn(
                                                        "d-s",
                                                        Density::Small.label(),
                                                        density == Density::Small,
                                                        false,
                                                        cx,
                                                        |this, _, _, cx| {
                                                            this.set_density(Density::Small, cx)
                                                        },
                                                    ))
                                                    .child(btn(
                                                        "d-m",
                                                        Density::Medium.label(),
                                                        density == Density::Medium,
                                                        false,
                                                        cx,
                                                        |this, _, _, cx| {
                                                            this.set_density(Density::Medium, cx)
                                                        },
                                                    ))
                                                    .child(btn(
                                                        "d-l",
                                                        Density::Large.label(),
                                                        density == Density::Large,
                                                        false,
                                                        cx,
                                                        |this, _, _, cx| {
                                                            this.set_density(Density::Large, cx)
                                                        },
                                                    )),
                                            )
                                            .child(btn(
                                                "slideshow",
                                                if slideshow { "Stop" } else { "Slideshow" },
                                                slideshow,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.toggle_slideshow(
                                                        &ToggleSlideshow,
                                                        window,
                                                        cx,
                                                    );
                                                },
                                            ))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(t.text_muted))
                                                    .child(status),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("grid")
                            .flex_1()
                            .w_full()
                            .overflow_y_scroll()
                            .p(px(PAD))
                            .when(loading, |s| {
                                s.flex().items_center().justify_center().child(
                                    div().text_color(rgb(t.text_dim)).child("Loading folder…"),
                                )
                            })
                            .when(!loading && count == 0, |s| {
                                s.flex().items_center().justify_center().child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .gap_3()
                                        .child(
                                            div()
                                                .text_color(rgb(t.text_dim))
                                                .child("Nothing here yet."),
                                        )
                                        .child(btn(
                                            "open-empty",
                                            "Open Folder",
                                            false,
                                            true,
                                            cx,
                                            |this, _, _, cx| this.pick_folder(cx),
                                        )),
                                )
                            })
                            .when(!loading && count > 0, |s| {
                                s.child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .flex_row()
                                        .flex_wrap()
                                        .gap(px(GAP))
                                        .children(self.entries.iter().enumerate().map(
                                            |(i, entry)| self.render_tile(i, entry, tile, cx),
                                        )),
                                )
                            }),
                    ),
            );

        root.when_some(selected, |s, index| {
            s.child(self.render_lightbox(index, cx))
        })
    }
}
