mod media;
mod thumbs;

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use gpui::{
    actions, div, img, point, prelude::*, px, relative, rgb, size, App, Application, Bounds,
    ClickEvent, Context, FocusHandle, Image, KeyBinding, Menu, MenuItem, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ObjectFit, PathPromptOptions, Pixels, Point, ScrollWheelEvent,
    SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use media::{scan_folder_recursive, MediaItem, MediaKind};
use thumbs::load_or_make_thumb;

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
        DensitySmall,
        DensityMedium,
        DensityLarge,
        ToggleSlideshow,
        ResetZoom,
    ]
);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Density {
    Small,
    Medium,
    Large,
}

impl Density {
    fn tile(self) -> f32 {
        match self {
            Self::Small => 112.0,
            Self::Medium => 168.0,
            Self::Large => 240.0,
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

struct Gallery {
    folder: PathBuf,
    items: Vec<MediaItem>,
    thumbs: HashMap<PathBuf, Arc<Image>>,
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
    fn new(folder: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        let mut gallery = Self {
            folder: folder.clone(),
            items: Vec::new(),
            thumbs: HashMap::new(),
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
        gallery.begin_load(folder, cx);
        gallery
    }

    fn begin_load(&mut self, folder: PathBuf, cx: &mut Context<Self>) {
        self.folder = folder.clone();
        self.items.clear();
        self.thumbs.clear();
        self.focused = None;
        self.selected = None;
        self.viewer = ViewerState::default();
        self.stop_slideshow();
        self.loading = true;
        self.load_gen += 1;
        self.thumb_gen += 1;
        let gen = self.load_gen;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let items = cx
                .background_spawn(async move { scan_folder_recursive(&folder) })
                .await;

            this.update(cx, |this, cx| {
                if this.load_gen != gen {
                    return;
                }
                this.items = items;
                this.loading = false;
                this.focused = if this.items.is_empty() { None } else { Some(0) };
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
            .items
            .iter()
            .filter(|i| i.kind == MediaKind::Image)
            .map(|i| i.path.clone())
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

    fn columns(&self, window: &Window) -> usize {
        let width: f32 = window.viewport_size().width.into();
        let tile = self.density.tile() + 16.0;
        let usable = (width.max(1.0) - 32.0).max(tile);
        (usable / tile).floor().max(1.0) as usize
    }

    fn open_item(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(item) = self.items.get(index).cloned() else {
            return;
        };
        self.focused = Some(index);
        match item.kind {
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
            self.open_item(i, cx);
        }
    }

    fn move_focus(&mut self, delta: isize, wrap: bool, cx: &mut Context<Self>) {
        if self.selected.is_some() || self.items.is_empty() {
            return;
        }
        let len = self.items.len() as isize;
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
            self.open_item(i, cx);
        }
    }

    fn prev_item(&mut self, _: &PrevItem, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected.is_some() {
            self.step_image(-1, cx);
        }
    }

    fn step_image(&mut self, step: isize, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        let start = match self.selected {
            Some(i) => i as isize + step,
            None => self.focused.unwrap_or(0) as isize,
        };
        let len = self.items.len() as isize;
        let mut i = start.rem_euclid(len);
        for _ in 0..self.items.len() {
            let idx = i as usize;
            if self.items[idx].kind == MediaKind::Image {
                self.selected = Some(idx);
                self.focused = Some(idx);
                self.viewer = ViewerState::default();
                cx.notify();
                return;
            }
            i = (i + step).rem_euclid(len);
        }
    }

    fn open_folder_action(&mut self, _: &OpenFolder, _window: &mut Window, cx: &mut Context<Self>) {
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
                this.begin_load(folder, cx);
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
            if let Some(i) = self.focused {
                if self.items.get(i).map(|it| it.kind) == Some(MediaKind::Image) {
                    self.open_item(i, cx);
                } else {
                    self.step_image(1, cx);
                }
            } else {
                self.step_image(1, cx);
            }
        }

        if self.selected.is_none() {
            return;
        }

        self.slideshow = true;
        self.slideshow_gen += 1;
        let gen = self.slideshow_gen;
        cx.notify();

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(3))
                    .await;
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

    fn toolbar_btn(
        id: &'static str,
        label: impl Into<SharedString>,
        active: bool,
        cx: &Context<Self>,
        on_click: impl Fn(&mut Self, &ClickEvent, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .id(id)
            .px_2()
            .py_1()
            .rounded_md()
            .text_xs()
            .cursor_pointer()
            .when(active, |s| {
                s.bg(rgb(0x3a3a3a)).text_color(rgb(0xffffff))
            })
            .when(!active, |s| {
                s.bg(rgb(0x1e1e1e))
                    .text_color(rgb(0xb0b0b0))
                    .hover(|s| s.bg(rgb(0x2a2a2a)).text_color(rgb(0xe8e8e8)))
            })
            .child(label.into())
            .on_click(cx.listener(on_click))
    }

    fn render_tile(
        &self,
        index: usize,
        item: &MediaItem,
        tile: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let focused = self.focused == Some(index);
        let kind = item.kind;
        let name = item.name.clone();
        let thumb = self.thumbs.get(&item.path).cloned();

        let media = match kind {
            MediaKind::Image => {
                if let Some(thumb) = thumb {
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
                        .bg(rgb(0x1a1a1a))
                        .text_color(rgb(0x666666))
                        .text_xs()
                        .child("…")
                        .into_any_element()
                }
            }
            MediaKind::Video => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(0x1a1a1a))
                .text_color(rgb(0xc8c8c8))
                .text_sm()
                .child("▶")
                .into_any_element(),
        };

        div()
            .id(("tile", index))
            .w(px(tile))
            .flex()
            .flex_col()
            .gap_1()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.open_item(index, cx);
            }))
            .child(
                div()
                    .w(px(tile))
                    .h(px(tile))
                    .overflow_hidden()
                    .rounded_md()
                    .bg(rgb(0x222222))
                    .border_2()
                    .when(focused, |s| s.border_color(rgb(0xe8e8e8)))
                    .when(!focused, |s| s.border_color(rgb(0x222222)))
                    .child(media),
            )
            .child(
                div()
                    .w(px(tile))
                    .px_1()
                    .text_xs()
                    .text_color(if focused { rgb(0xe8e8e8) } else { rgb(0x8a8a8a) })
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(name),
            )
    }

    fn render_lightbox(&self, index: usize, cx: &Context<Self>) -> impl IntoElement {
        let item = &self.items[index];
        let zoom = self.viewer.zoom;
        let pan = self.viewer.pan;
        let slideshow = self.slideshow;
        let label = format!(
            "{}  ·  {} / {}  ·  {:.0}%{}",
            item.name,
            index + 1,
            self.items.len(),
            zoom * 100.0,
            if slideshow { "  ·  slideshow" } else { "" }
        );

        div()
            .id("lightbox")
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .bg(rgb(0x0a0a0a))
            .child(
                div()
                    .id("lightbox-chrome")
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xd0d0d0))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(label),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(Self::toolbar_btn(
                                "slide-btn",
                                if slideshow { "Stop" } else { "Slideshow" },
                                slideshow,
                                cx,
                                |this, _, window, cx| {
                                    this.toggle_slideshow(&ToggleSlideshow, window, cx);
                                },
                            ))
                            .child(Self::toolbar_btn("close-btn", "Esc", false, cx, |this, _, _, cx| {
                                this.selected = None;
                                this.viewer = ViewerState::default();
                                this.stop_slideshow();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("lightbox-body")
                    .flex_1()
                    .w_full()
                    .relative()
                    .overflow_hidden()
                    .cursor(if zoom > 1.0 {
                        gpui::CursorStyle::PointingHand
                    } else {
                        gpui::CursorStyle::Arrow
                    })
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
                    .text_color(rgb(0x777777))
                    .child("Scroll zoom · drag pan · double-click reset · ← → navigate · S slideshow"),
            )
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let folder_label: SharedString = self.folder.display().to_string().into();
        let count = self.items.len();
        let selected = self.selected;
        let density = self.density;
        let tile = density.tile();
        let loading = self.loading;
        let slideshow = self.slideshow;

        let status: SharedString = if loading {
            "Scanning…".into()
        } else {
            format!(
                "{count} item{}",
                if count == 1 { "" } else { "s" }
            )
            .into()
        };

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
            .on_action(cx.listener(Self::density_small))
            .on_action(cx.listener(Self::density_medium))
            .on_action(cx.listener(Self::density_large))
            .on_action(cx.listener(Self::toggle_slideshow))
            .on_action(cx.listener(Self::reset_zoom))
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x121212))
            .text_color(rgb(0xe8e8e8))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(0x2a2a2a))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .min_w_0()
                            .flex_1()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("gallery"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x888888))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(folder_label),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(Self::toolbar_btn("open", "Open", false, cx, |this, _, _, cx| {
                                this.pick_folder(cx);
                            }))
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .child(Self::toolbar_btn(
                                        "d-s",
                                        Density::Small.label(),
                                        density == Density::Small,
                                        cx,
                                        |this, _, _, cx| this.set_density(Density::Small, cx),
                                    ))
                                    .child(Self::toolbar_btn(
                                        "d-m",
                                        Density::Medium.label(),
                                        density == Density::Medium,
                                        cx,
                                        |this, _, _, cx| this.set_density(Density::Medium, cx),
                                    ))
                                    .child(Self::toolbar_btn(
                                        "d-l",
                                        Density::Large.label(),
                                        density == Density::Large,
                                        cx,
                                        |this, _, _, cx| this.set_density(Density::Large, cx),
                                    )),
                            )
                            .child(Self::toolbar_btn(
                                "slideshow",
                                if slideshow { "Stop" } else { "Slideshow" },
                                slideshow,
                                cx,
                                |this, _, window, cx| {
                                    this.toggle_slideshow(&ToggleSlideshow, window, cx);
                                },
                            ))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0x888888))
                                    .child(status),
                            ),
                    ),
            )
            .child(
                div()
                    .id("grid")
                    .flex_1()
                    .w_full()
                    .overflow_y_scroll()
                    .p_4()
                    .when(loading, |s| {
                        s.flex().items_center().justify_center().child(
                            div()
                                .text_color(rgb(0x777777))
                                .child("Scanning folder…"),
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
                                        .text_color(rgb(0x777777))
                                        .child("No images or videos found."),
                                )
                                .child(Self::toolbar_btn(
                                    "open-empty",
                                    "Open Folder",
                                    false,
                                    cx,
                                    |this, _, _, cx| this.pick_folder(cx),
                                )),
                        )
                    })
                    .when(!loading && count > 0, |s| {
                        s.child(
                            div()
                                .flex()
                                .flex_row()
                                .flex_wrap()
                                .gap_4()
                                .children(
                                    self.items
                                        .iter()
                                        .enumerate()
                                        .map(|(i, item)| self.render_tile(i, item, tile, cx)),
                                ),
                        )
                    }),
            );

        root.when_some(selected, |s, index| s.child(self.render_lightbox(index, cx)))
    }
}

fn resolve_folder() -> PathBuf {
    let arg = std::env::args().nth(1);
    let path = arg
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("media"));
    path.canonicalize().unwrap_or(path)
}

fn main() {
    let folder = resolve_folder();

    Application::new().run(move |cx: &mut App| {
        cx.activate(true);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-o", OpenFolder, Some("Gallery")),
            KeyBinding::new("escape", CloseViewer, Some("Gallery")),
            KeyBinding::new("right", MoveRight, Some("Gallery")),
            KeyBinding::new("left", MoveLeft, Some("Gallery")),
            KeyBinding::new("up", MoveUp, Some("Gallery")),
            KeyBinding::new("down", MoveDown, Some("Gallery")),
            KeyBinding::new("enter", OpenFocused, Some("Gallery")),
            KeyBinding::new("space", NextItem, Some("Gallery")),
            KeyBinding::new("1", DensitySmall, Some("Gallery")),
            KeyBinding::new("2", DensityMedium, Some("Gallery")),
            KeyBinding::new("3", DensityLarge, Some("Gallery")),
            KeyBinding::new("s", ToggleSlideshow, Some("Gallery")),
            KeyBinding::new("0", ResetZoom, Some("Gallery")),
        ]);
        cx.set_menus(vec![Menu {
            name: "gallery".into(),
            items: vec![
                MenuItem::action("Open Folder…", OpenFolder),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        }]);

        let title = format!("gallery — {}", folder.display());
        let bounds = Bounds::centered(None, size(px(1100.), px(760.)), cx);

        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some(title.into()),
                    appears_transparent: false,
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                focus: true,
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Gallery::new(folder.clone(), window, cx)),
        )
        .unwrap();
    });
}
