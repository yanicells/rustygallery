use gpui::{div, prelude::*, px, rgb, Context};

use crate::ui::{btn, Theme};

use super::{About, Gallery};

impl Gallery {
    pub(super) fn show_about(&mut self, _: &About, _: &mut gpui::Window, cx: &mut Context<Self>) {
        self.about_open = true;
        cx.notify();
    }

    pub(super) fn close_about(&mut self, cx: &mut Context<Self>) {
        if self.about_open {
            self.about_open = false;
            cx.notify();
        }
    }

    pub(super) fn render_about(&self, cx: &Context<Self>) -> impl IntoElement {
        let t = Theme::current();
        let version = env!("CARGO_PKG_VERSION");
        div()
            .id("about")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(t.lightbox))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| this.close_about(cx)),
            )
            .child(
                div()
                    .w(px(320.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(t.border))
                    .bg(rgb(t.surface))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(t.text))
                            .child("gallery"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(t.text_dim))
                            .child(format!("version {version}")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(t.text_muted))
                            .child("A small photo viewer built with GPUI."),
                    )
                    .child(
                        div()
                            .id("about-repo")
                            .mt_2()
                            .text_sm()
                            .text_color(rgb(t.accent_soft))
                            .cursor_pointer()
                            .on_click(cx.listener(|_, _, _, _| {
                                open_repo();
                            }))
                            .child("github.com/yanicells/rustygallery"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .justify_end()
                            .child(btn(
                                "about-close",
                                "Close",
                                false,
                                false,
                                cx,
                                |this, _, _, cx| this.close_about(cx),
                            )),
                    ),
            )
    }
}

fn open_repo() {
    let url = "https://github.com/yanicells/rustygallery";
    let _ = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .status()
    } else {
        std::process::Command::new("xdg-open").arg(url).status()
    };
}
