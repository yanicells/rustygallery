use std::path::{Path, PathBuf};

use gpui::{
    actions, div, img, prelude::*, px, rgb, size, App, Application, Bounds, ClickEvent, Context,
    FocusHandle, KeyBinding, Menu, MenuItem, ObjectFit, SharedString, TitlebarOptions, Window,
    WindowBounds, WindowOptions,
};

actions!(gallery, [Quit, CloseViewer, NextItem, PrevItem]);

const TILE: f32 = 168.0;
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff"];
const VIDEO_EXTS: &[&str] = &["mp4", "mov", "mkv", "webm", "avi", "m4v"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum MediaKind {
    Image,
    Video,
}

#[derive(Clone)]
struct MediaItem {
    path: PathBuf,
    name: SharedString,
    kind: MediaKind,
}

struct Gallery {
    folder: PathBuf,
    items: Vec<MediaItem>,
    selected: Option<usize>,
    focus_handle: FocusHandle,
}

impl Gallery {
    fn new(folder: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        focus_handle.focus(window);
        let items = scan_folder(&folder);
        Self {
            folder,
            items,
            selected: None,
            focus_handle,
        }
    }

    fn open_item(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(item) = self.items.get(index) else {
            return;
        };
        match item.kind {
            MediaKind::Image => {
                self.selected = Some(index);
                cx.notify();
            }
            MediaKind::Video => {
                cx.open_with_system(&item.path);
            }
        }
    }

    fn close_viewer(&mut self, _: &CloseViewer, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected.take().is_some() {
            cx.notify();
        }
    }

    fn next_item(&mut self, _: &NextItem, _: &mut Window, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        let next = match self.selected {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        };
        // Skip videos in the lightbox; jump to next image if possible.
        self.advance_selection(next, 1, cx);
    }

    fn prev_item(&mut self, _: &PrevItem, _: &mut Window, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        let prev = match self.selected {
            Some(0) => self.items.len() - 1,
            Some(i) => i - 1,
            None => self.items.len() - 1,
        };
        self.advance_selection(prev, -1, cx);
    }

    fn advance_selection(&mut self, start: usize, step: isize, cx: &mut Context<Self>) {
        let len = self.items.len() as isize;
        let mut i = start as isize;
        for _ in 0..self.items.len() {
            let idx = ((i % len) + len) % len;
            let idx = idx as usize;
            if self.items[idx].kind == MediaKind::Image {
                self.selected = Some(idx);
                cx.notify();
                return;
            }
            i += step;
        }
    }

    fn render_tile(&self, index: usize, item: &MediaItem, cx: &mut Context<Self>) -> impl IntoElement {
        let kind = item.kind;
        let name = item.name.clone();

        let thumb = match kind {
            MediaKind::Image => img(item.path.clone())
                .id(("thumb", index))
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            MediaKind::Video => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(0x1a1a1a))
                .text_color(rgb(0xc8c8c8))
                .text_sm()
                .child("▶ video")
                .into_any_element(),
        };

        div()
            .id(("tile", index))
            .w(px(TILE))
            .flex()
            .flex_col()
            .gap_1()
            .cursor_pointer()
            .hover(|s| s.opacity(0.85))
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.open_item(index, cx);
            }))
            .child(
                div()
                    .w(px(TILE))
                    .h(px(TILE))
                    .overflow_hidden()
                    .rounded_md()
                    .bg(rgb(0x222222))
                    .child(thumb),
            )
            .child(
                div()
                    .w(px(TILE))
                    .px_1()
                    .text_xs()
                    .text_color(rgb(0xa0a0a0))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(name),
            )
    }

    fn render_lightbox(&self, index: usize, cx: &Context<Self>) -> impl IntoElement {
        let item = &self.items[index];
        let label = format!("{}  ·  {} / {}", item.name, index + 1, self.items.len());

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
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0xd0d0d0))
                            .child(label),
                    )
                    .child(
                        div()
                            .id("close-btn")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x2a2a2a))
                            .text_sm()
                            .text_color(rgb(0xe8e8e8))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0x3a3a3a)))
                            .child("Esc")
                            .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                this.selected = None;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("lightbox-body")
                    .flex_1()
                    .w_full()
                    .p_4()
                    .child(
                        img(item.path.clone())
                            .id(("full", index))
                            .size_full()
                            .object_fit(ObjectFit::Contain),
                    ),
            )
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let folder_label: SharedString = self.folder.display().to_string().into();
        let count = self.items.len();
        let selected = self.selected;

        let root = div()
            .id("gallery")
            .key_context("Gallery")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::close_viewer))
            .on_action(cx.listener(Self::next_item))
            .on_action(cx.listener(Self::prev_item))
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
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(0x2a2a2a))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
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
                                    .child(folder_label),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x888888))
                            .child(format!(
                                "{count} item{}",
                                if count == 1 { "" } else { "s" }
                            )),
                    ),
            )
            .child(
                div()
                    .id("grid")
                    .flex_1()
                    .w_full()
                    .overflow_y_scroll()
                    .p_4()
                    .when(count == 0, |s| {
                        s.flex().items_center().justify_center().child(
                            div()
                                .text_color(rgb(0x777777))
                                .child("No images or videos in this folder."),
                        )
                    })
                    .when(count > 0, |s| {
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
                                        .map(|(i, item)| self.render_tile(i, item, cx)),
                                ),
                        )
                    }),
            );

        root.when_some(selected, |s, index| s.child(self.render_lightbox(index, cx)))
    }
}

fn scan_folder(folder: &Path) -> Vec<MediaItem> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };

    let mut items: Vec<MediaItem> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let ext = path
                .extension()?
                .to_str()?
                .to_ascii_lowercase();
            let kind = if IMAGE_EXTS.iter().any(|e| *e == ext) {
                MediaKind::Image
            } else if VIDEO_EXTS.iter().any(|e| *e == ext) {
                MediaKind::Video
            } else {
                return None;
            };
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("untitled")
                .to_string();
            Some(MediaItem {
                path,
                name: name.into(),
                kind,
            })
        })
        .collect();

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
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
            KeyBinding::new("escape", CloseViewer, Some("Gallery")),
            KeyBinding::new("right", NextItem, Some("Gallery")),
            KeyBinding::new("left", PrevItem, Some("Gallery")),
            KeyBinding::new("space", NextItem, Some("Gallery")),
        ]);
        cx.set_menus(vec![Menu {
            name: "gallery".into(),
            items: vec![MenuItem::action("Quit", Quit)],
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
