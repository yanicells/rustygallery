use gpui::{
    div, img, prelude::*, px, relative, rgb, Context, ImageSource, MouseButton, ObjectFit, Window,
};

use crate::media::{is_animated, Entry, MediaKind};
use crate::ui::{btn, seg, segmented, Theme};

use super::exif::read_exif;
use super::viewer::ViewMode;
use super::{
    CopyPath, Gallery, MoveToTrash, RevealInFinder, RotateLeft, RotateRight, ToggleFullscreen,
    ToggleSlideshow, ToggleStar, ViewActual, ViewFill, ViewFit,
};

impl Gallery {
    fn visible_image_indices(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                (self.entry_visible(e) && matches!(e, Entry::Media(_))).then_some(i)
            })
            .collect()
    }

    /// "3 / 120": place among the media shown in the current listing.
    pub(super) fn media_position(&self, current: usize) -> String {
        let imgs = self.visible_image_indices();
        let pos = imgs.iter().position(|&i| i == current).unwrap_or(0);
        format!("{} / {}", pos + 1, imgs.len())
    }

    pub(super) fn filmstrip_indices(&self, current: usize) -> Vec<usize> {
        let imgs = self.visible_image_indices();
        let Some(pos) = imgs.iter().position(|&i| i == current) else {
            return Vec::new();
        };
        let start = pos.saturating_sub(4);
        let end = (start + 9).min(imgs.len());
        let start = end.saturating_sub(9);
        imgs[start..end].to_vec()
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
        if item.kind == MediaKind::Video {
            return self.render_video(index, item, window, cx);
        }
        let t = Theme::current();
        let Some(source) = self.viewer.source.clone() else {
            return div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(t.lightbox))
                .child("Loading preview…")
                .into_any_element();
        };
        if self.viewer.peek {
            return self.render_peek(source, item.modified, cx);
        }

        let zoom = self.viewer.zoom;
        let pan = self.viewer.pan;
        let slideshow = self.slideshow;
        let fullscreen = window.is_fullscreen();
        let mode = self.viewer.mode;
        let animated = is_animated(&item.path);
        let starred = self.is_favorite(&item.path);
        let mut meta = self.media_position(index);
        if (zoom - 1.0).abs() > 0.01 {
            meta.push_str(&format!("  ·  {:.0}%", zoom * 100.0));
        }
        if slideshow {
            meta.push_str("  ·  slideshow");
        }
        let strip = self.filmstrip_indices(index);
        let exif = self.viewer.exif.then(|| read_exif(&item.path));
        let hint = if animated {
            "← → browse  ·  Space pause  ·  I info  ·  Esc back"
        } else {
            "← → browse  ·  I info  ·  [ ] rotate  ·  F11 full screen  ·  Esc back"
        };

        div()
            .id("lightbox")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .flex_col()
            .bg(rgb(t.lightbox))
            .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(self.render_lightbox_header(
                &item.name, &meta, slideshow, fullscreen, mode, starred, cx,
            ))
            .child(
                div()
                    .id("lightbox-mid")
                    .flex_1()
                    .w_full()
                    .flex()
                    .flex_row()
                    .min_h_0()
                    .child(self.render_lightbox_body(
                        source,
                        item.modified,
                        zoom,
                        pan,
                        mode,
                        animated,
                        animated && self.viewer.anim_paused,
                        strip.len() > 1,
                        cx,
                    ))
                    .when_some(exif, |s, info| s.child(render_exif_panel(&info))),
            )
            .child(self.render_filmstrip(index, &strip, cx))
            .child(
                div()
                    .px_4()
                    .py_2()
                    .text_xs()
                    .text_color(rgb(t.text_dim))
                    .child(hint),
            )
            .into_any_element()
    }

    fn render_peek(
        &self,
        source: ImageSource,
        modified: u64,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let t = Theme::current();
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
                img(source)
                    .with_fallback(preview_unavailable)
                    .id(("peek-img", modified))
                    .w_full()
                    .h_full()
                    .object_fit(ObjectFit::Contain),
            )
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_lightbox_header(
        &self,
        name: &gpui::SharedString,
        meta: &str,
        slideshow: bool,
        fullscreen: bool,
        mode: ViewMode,
        starred: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let t = Theme::current();
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .px_4()
            .py_2()
            .gap_x_3()
            .gap_y_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
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
                            .flex_shrink_0()
                            .text_sm()
                            .text_color(rgb(t.text_dim))
                            .whitespace_nowrap()
                            .child(meta.to_string()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(segmented().child(seg(
                        "star-btn",
                        if starred { "★ Starred" } else { "☆ Star" },
                        starred,
                        cx,
                        |this, _, window, cx| this.toggle_star(&ToggleStar, window, cx),
                    )))
                    .child(
                        segmented()
                            .child(seg(
                                "fit-btn",
                                "Fit",
                                mode == ViewMode::Fit,
                                cx,
                                |this, _, window, cx| this.view_fit(&ViewFit, window, cx),
                            ))
                            .child(seg(
                                "fill-btn",
                                "Fill",
                                mode == ViewMode::Fill,
                                cx,
                                |this, _, window, cx| this.view_fill(&ViewFill, window, cx),
                            ))
                            .child(seg(
                                "actual-btn",
                                "100%",
                                mode == ViewMode::Actual,
                                cx,
                                |this, _, window, cx| this.view_actual(&ViewActual, window, cx),
                            )),
                    )
                    .child(
                        segmented()
                            .child(seg("rot-l-btn", "↺", false, cx, |this, _, window, cx| {
                                this.rotate_left(&RotateLeft, window, cx)
                            }))
                            .child(seg("rot-r-btn", "↻", false, cx, |this, _, window, cx| {
                                this.rotate_right(&RotateRight, window, cx)
                            })),
                    )
                    .child(
                        segmented()
                            .child(seg(
                                "reveal-btn",
                                "Reveal",
                                false,
                                cx,
                                |this, _, window, cx| {
                                    this.reveal_in_finder(&RevealInFinder, window, cx);
                                },
                            ))
                            .child(seg(
                                "copy-path-btn",
                                "Copy path",
                                false,
                                cx,
                                |this, _, window, cx| {
                                    this.copy_path(&CopyPath, window, cx);
                                },
                            ))
                            .child(seg(
                                "trash-btn",
                                "Trash",
                                false,
                                cx,
                                |this, _, window, cx| {
                                    this.move_to_trash(&MoveToTrash, window, cx);
                                },
                            )),
                    )
                    .child(
                        segmented()
                            .child(seg(
                                "slide-btn",
                                if slideshow {
                                    "Stop slideshow"
                                } else {
                                    "Slideshow"
                                },
                                slideshow,
                                cx,
                                |this, _, window, cx| {
                                    this.toggle_slideshow(&ToggleSlideshow, window, cx);
                                },
                            ))
                            .child(seg(
                                "full-btn",
                                if fullscreen {
                                    "Exit full screen"
                                } else {
                                    "Full screen"
                                },
                                fullscreen,
                                cx,
                                |this, _, window, cx| {
                                    this.toggle_fullscreen(&ToggleFullscreen, window, cx);
                                },
                            )),
                    )
                    .child(btn(
                        "close-btn",
                        "Close",
                        false,
                        true,
                        cx,
                        |this, _, _, cx| {
                            this.reset_viewer(cx);
                            this.selected = None;
                            this.stop_slideshow();
                            cx.notify();
                        },
                    )),
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_lightbox_body(
        &self,
        source: ImageSource,
        modified: u64,
        zoom: f32,
        pan: gpui::Point<gpui::Pixels>,
        mode: ViewMode,
        animated: bool,
        paused: bool,
        can_step: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let still = paused.then(|| self.viewer.still.clone()).flatten();
        let image = if let Some(frame) = still {
            match mode {
                ViewMode::Fit => img(frame)
                    .id(("full-pause", modified))
                    .size_full()
                    .object_fit(ObjectFit::Contain)
                    .into_any_element(),
                ViewMode::Fill => img(frame)
                    .id(("full-pause-fill", modified))
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .into_any_element(),
                ViewMode::Actual => {
                    let (w, h) = self.viewer.px.unwrap_or((800, 600));
                    img(frame)
                        .id(("full-pause-actual", modified))
                        .w(px(w as f32 * zoom))
                        .h(px(h as f32 * zoom))
                        .into_any_element()
                }
            }
        } else {
            match mode {
                ViewMode::Fit => img(source)
                    .with_fallback(preview_unavailable)
                    .id(("full", modified))
                    .size_full()
                    .object_fit(ObjectFit::Contain)
                    .into_any_element(),
                ViewMode::Fill => img(source)
                    .with_fallback(preview_unavailable)
                    .id(("full-fill", modified))
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .into_any_element(),
                ViewMode::Actual => {
                    let (w, h) = self.viewer.px.unwrap_or((800, 600));
                    img(source)
                        .with_fallback(preview_unavailable)
                        .id(("full-actual", modified))
                        .w(px(w as f32 * zoom))
                        .h(px(h as f32 * zoom))
                        .into_any_element()
                }
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
            .when(can_step, |s| {
                s.child(step_arrow("step-prev", "‹", true, cx))
                    .child(step_arrow("step-next", "›", false, cx))
            })
            .when(animated, |s| {
                let t = Theme::current();
                s.child(
                    div()
                        .absolute()
                        .top_3()
                        .left_3()
                        .px_2()
                        .py_0p5()
                        .rounded_md()
                        .bg(rgb(t.btn_active))
                        .text_xs()
                        .text_color(rgb(t.accent_soft))
                        .child(if paused { "GIF paused" } else { "GIF" }),
                )
            })
    }

    pub(super) fn render_filmstrip(
        &self,
        current: usize,
        strip: &[usize],
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let t = Theme::current();
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
                            div().size_full().bg(rgb(t.tile_media)).into_any_element()
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

/// Round previous/next control floating over the image edge.
fn step_arrow(
    id: &'static str,
    glyph: &'static str,
    prev: bool,
    cx: &Context<Gallery>,
) -> impl IntoElement {
    let t = Theme::current();
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .when(prev, |s| s.left_3())
        .when(!prev, |s| s.right_3())
        .flex()
        .items_center()
        .child(
            div()
                .id(id)
                .size(px(40.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(rgb(t.btn))
                .text_color(rgb(t.btn_text))
                .text_xl()
                .cursor_pointer()
                .hover(|s| s.bg(rgb(t.btn_hover)).text_color(rgb(t.on_accent)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.step_image(if prev { -1 } else { 1 }, cx);
                }))
                .child(glyph),
        )
}

fn render_exif_panel(info: &super::exif::ExifInfo) -> impl IntoElement {
    let t = Theme::current();
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

fn preview_unavailable() -> gpui::AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(Theme::current().text_muted))
        .child("Preview unavailable")
        .into_any_element()
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
