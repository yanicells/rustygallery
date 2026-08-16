use gpui::{div, img, prelude::*, relative, rgb, Context, MouseButton, ObjectFit};

use crate::media::Entry;
use crate::ui::{btn, Theme};

use super::{viewer::ViewerState, Gallery, ToggleSlideshow};

impl Gallery {
    pub(super) fn render_lightbox(&self, index: usize, cx: &Context<Self>) -> impl IntoElement {
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
}
