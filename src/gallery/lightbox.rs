use std::path::PathBuf;

use gpui::{div, img, prelude::*, px, relative, rgb, Context, MouseButton, ObjectFit, Window};

use crate::media::{Entry, MediaKind};
use crate::ui::{btn, Theme};

use super::exif::read_exif;
use super::viewer::ViewMode;
use super::{
    viewer::ViewerState, CopyPath, Gallery, MoveToTrash, RevealInFinder, RotateLeft, RotateRight,
    ToggleFullscreen, ToggleSlideshow, ViewActual, ViewFill, ViewFit,
};

impl Gallery {
    fn visible_image_indices(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                (self.entry_visible(e)
                    && matches!(e, Entry::Media(m) if m.kind == MediaKind::Image))
                .then_some(i)
            })
            .collect()
    }

    fn filmstrip_indices(&self, current: usize) -> Vec<usize> {
        let imgs = self.visible_image_indices();
        let Some(pos) = imgs.iter().position(|&i| i == current) else {
            return Vec::new();
        };
        let start = pos.saturating_sub(4);
        let end = (start + 9).min(imgs.len());
        let start = end.saturating_sub(9);
        imgs[start..end].to_vec()
    }

    fn neighbor_paths(&self, current: usize) -> Vec<PathBuf> {
        let imgs = self.visible_image_indices();
        let Some(pos) = imgs.iter().position(|&i| i == current) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for step in [pos.checked_sub(1), Some(pos + 1)] {
            if let Some(i) = step {
                if let Some(Entry::Media(item)) = imgs.get(i).and_then(|&idx| self.entries.get(idx))
                {
                    out.push(item.path.clone());
                }
            }
        }
        out
    }

    pub(super) fn render_lightbox(
        &self,
        index: usize,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let Entry::Media(item) = &self.entries[index] else {
            return div().into_any_element();
        };
        let t = Theme::DARK;
        if self.viewer.peek {
            return self.render_peek(item.path.clone(), item.modified, cx);
        }

        let zoom = self.viewer.zoom;
        let pan = self.viewer.pan;
        let slideshow = self.slideshow;
        let fullscreen = window.is_fullscreen();
        let mode = self.viewer.mode;
        let meta = format!(
            "·  {} / {}  ·  {}  ·  {:.0}%{}",
            index + 1,
            self.entries.len(),
            mode_label(mode),
            zoom * 100.0,
            if slideshow { "  ·  slideshow" } else { "" }
        );
        let neighbors = self.neighbor_paths(index);
        let strip = self.filmstrip_indices(index);
        let exif = self.viewer.exif.then(|| read_exif(&item.path));

        div()
            .id("lightbox")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .flex_col()
            .bg(rgb(t.lightbox))
            .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(self.render_lightbox_header(&item.name, &meta, slideshow, fullscreen, mode, cx))
            .child(
                div()
                    .id("lightbox-mid")
                    .flex_1()
                    .w_full()
                    .flex()
                    .flex_row()
                    .min_h_0()
                    .child(self.render_lightbox_body(
                        item.path.clone(),
                        item.modified,
                        zoom,
                        pan,
                        mode,
                        cx,
                    ))
                    .when_some(exif, |s, info| s.child(render_exif_panel(&info))),
            )
            .child(self.render_filmstrip(index, &strip, cx))
            .children(neighbors.into_iter().enumerate().map(|(i, path)| {
                img(path)
                    .id(("prefetch", i))
                    .w(px(0.))
                    .h(px(0.))
                    .overflow_hidden()
            }))
            .child(
                div()
                    .px_4()
                    .py_2()
                    .text_xs()
                    .text_color(rgb(t.text_dim))
                    .child("I info  ·  [ ] rotate  ·  F11 full  ·  Space next  ·  Esc back"),
            )
            .into_any_element()
    }

    fn render_peek(&self, path: PathBuf, modified: u64, cx: &Context<Self>) -> gpui::AnyElement {
        let t = Theme::DARK;
        div()
            .id("peek")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(t.lightbox))
            .on_scroll_wheel(cx.listener(Self::on_viewer_scroll))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_viewer_down))
            .child(
                img(path)
                    .id(("peek-img", modified))
                    .w_full()
                    .h_full()
                    .object_fit(ObjectFit::Contain),
            )
            .into_any_element()
    }

    fn render_lightbox_header(
        &self,
        name: &gpui::SharedString,
        meta: &str,
        slideshow: bool,
        fullscreen: bool,
        mode: ViewMode,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let t = Theme::DARK;
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .id("lightbox-name")
                            .text_sm()
                            .text_color(rgb(t.accent_soft))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.rename_focused(&super::RenameFocused, window, cx);
                            }))
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(t.text_dim))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(meta.to_string()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(btn(
                        "fit-btn",
                        "Fit",
                        mode == ViewMode::Fit,
                        false,
                        cx,
                        |this, _, window, cx| this.view_fit(&ViewFit, window, cx),
                    ))
                    .child(btn(
                        "fill-btn",
                        "Fill",
                        mode == ViewMode::Fill,
                        false,
                        cx,
                        |this, _, window, cx| this.view_fill(&ViewFill, window, cx),
                    ))
                    .child(btn(
                        "actual-btn",
                        "100%",
                        mode == ViewMode::Actual,
                        false,
                        cx,
                        |this, _, window, cx| this.view_actual(&ViewActual, window, cx),
                    ))
                    .child(btn(
                        "full-btn",
                        if fullscreen { "Window" } else { "Full" },
                        fullscreen,
                        false,
                        cx,
                        |this, _, window, cx| {
                            this.toggle_fullscreen(&ToggleFullscreen, window, cx);
                        },
                    ))
                    .child(btn(
                        "rot-l-btn",
                        "↺",
                        false,
                        false,
                        cx,
                        |this, _, window, cx| this.rotate_left(&RotateLeft, window, cx),
                    ))
                    .child(btn(
                        "rot-r-btn",
                        "↻",
                        false,
                        false,
                        cx,
                        |this, _, window, cx| this.rotate_right(&RotateRight, window, cx),
                    ))
                    .child(btn(
                        "reveal-btn",
                        "Reveal",
                        false,
                        false,
                        cx,
                        |this, _, window, cx| {
                            this.reveal_in_finder(&RevealInFinder, window, cx);
                        },
                    ))
                    .child(btn(
                        "copy-path-btn",
                        "Copy Path",
                        false,
                        false,
                        cx,
                        |this, _, window, cx| {
                            this.copy_path(&CopyPath, window, cx);
                        },
                    ))
                    .child(btn(
                        "trash-btn",
                        "Trash",
                        false,
                        false,
                        cx,
                        |this, _, window, cx| {
                            this.move_to_trash(&MoveToTrash, window, cx);
                        },
                    ))
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
            )
    }

    fn render_lightbox_body(
        &self,
        path: PathBuf,
        modified: u64,
        zoom: f32,
        pan: gpui::Point<gpui::Pixels>,
        mode: ViewMode,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let image = match mode {
            ViewMode::Fit => img(path)
                .id(("full", modified))
                .size_full()
                .object_fit(ObjectFit::Contain)
                .into_any_element(),
            ViewMode::Fill => img(path)
                .id(("full-fill", modified))
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            ViewMode::Actual => {
                let (w, h) = self.viewer.px.unwrap_or((800, 600));
                img(path)
                    .id(("full-actual", modified))
                    .w(px(w as f32 * zoom))
                    .h(px(h as f32 * zoom))
                    .into_any_element()
            }
        };
        let frame = match mode {
            ViewMode::Actual => div().absolute().left(pan.x).top(pan.y).child(image),
            _ => div()
                .absolute()
                .left(pan.x)
                .top(pan.y)
                .w(relative(zoom))
                .h(relative(zoom))
                .child(image),
        };
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
            .child(frame)
    }

    fn render_filmstrip(
        &self,
        current: usize,
        strip: &[usize],
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let t = Theme::DARK;
        div()
            .id("filmstrip")
            .h(px(72.))
            .px_3()
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .children(strip.iter().copied().map(|i| {
                let active = i == current;
                let thumb = match &self.entries[i] {
                    Entry::Media(item) => {
                        if let Some(thumb) = self.thumbs.get(&item.path).cloned() {
                            img(thumb)
                                .id(("strip-thumb", i))
                                .size_full()
                                .object_fit(ObjectFit::Cover)
                                .into_any_element()
                        } else {
                            img(item.path.clone())
                                .id(("strip-file", item.modified))
                                .size_full()
                                .object_fit(ObjectFit::Cover)
                                .into_any_element()
                        }
                    }
                    Entry::Folder(_) => div().into_any_element(),
                };
                div()
                    .id(("strip", i))
                    .w(px(56.))
                    .h(px(56.))
                    .rounded_sm()
                    .overflow_hidden()
                    .border_2()
                    .border_color(if active { rgb(t.accent) } else { rgb(t.border) })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.jump_to_image(i, cx);
                    }))
                    .child(thumb)
            }))
    }
}

fn mode_label(mode: ViewMode) -> &'static str {
    match mode {
        ViewMode::Fit => "fit",
        ViewMode::Fill => "fill",
        ViewMode::Actual => "100%",
    }
}

fn render_exif_panel(info: &super::exif::ExifInfo) -> impl IntoElement {
    let t = Theme::DARK;
    div()
        .id("exif")
        .w(px(240.))
        .h_full()
        .px_3()
        .py_3()
        .border_l_1()
        .border_color(rgb(t.border))
        .bg(rgb(t.surface))
        .overflow_y_scroll()
        .child(
            div()
                .text_xs()
                .text_color(rgb(t.text_faint))
                .mb_2()
                .child("INFO"),
        )
        .when(info.rows.len() <= 1, |s| {
            s.child(
                div()
                    .text_xs()
                    .text_color(rgb(t.text_dim))
                    .child("No camera metadata on this file."),
            )
        })
        .children(info.rows.iter().map(|(k, v)| {
            div()
                .mb_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(t.text_faint))
                        .child(k.clone()),
                )
                .child(div().text_sm().text_color(rgb(t.text)).child(v.clone()))
        }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn filmstrip_window_stays_near_nine() {
        let imgs: Vec<usize> = (0..20).collect();
        let pos: usize = 10;
        let start = pos.saturating_sub(4);
        let end = (start + 9).min(imgs.len());
        let start = end.saturating_sub(9);
        assert_eq!(imgs[start..end].len(), 9);
        assert!(imgs[start..end].contains(&10));
    }
}
