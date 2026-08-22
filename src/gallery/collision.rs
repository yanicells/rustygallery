use gpui::{div, prelude::*, px, rgb, Context};

use crate::media::Collision;
use crate::ui::{btn, Theme};

use super::Gallery;

impl Gallery {
    pub(super) fn render_collision(&self, cx: &Context<Self>) -> impl IntoElement {
        let Some(ask) = &self.collision else {
            return div().into_any_element();
        };
        let t = Theme::current();
        let name = self.collision_name();
        let more = !ask.remaining.is_empty();
        let apply = ask.apply_all;

        div()
            .id("collision")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt_16()
            .bg(rgb(t.lightbox))
            .occlude()
            .child(
                div()
                    .w(px(400.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(t.border))
                    .bg(rgb(t.surface))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(t.text))
                            .child(format!("“{name}” already exists.")),
                    )
                    .child(div().text_xs().text_color(rgb(t.text_dim)).child(
                        "Keep Both adds a number. Replace moves the existing file to Trash.",
                    ))
                    .when(more, |s| {
                        s.child(
                            div()
                                .id("apply-all")
                                .cursor_pointer()
                                .text_xs()
                                .text_color(rgb(t.text_muted))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_collision_all(cx);
                                }))
                                .child(if apply {
                                    "☑ Apply to remaining"
                                } else {
                                    "☐ Apply to remaining"
                                }),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(btn(
                                "col-cancel",
                                "Cancel",
                                false,
                                false,
                                cx,
                                |this, _, _, cx| this.cancel_collision(cx),
                            ))
                            .child(btn(
                                "col-replace",
                                "Replace",
                                false,
                                false,
                                cx,
                                |this, _, _, cx| this.resolve_collision(Collision::Replace, cx),
                            ))
                            .child(btn(
                                "col-both",
                                "Keep Both",
                                false,
                                true,
                                cx,
                                |this, _, _, cx| this.resolve_collision(Collision::KeepBoth, cx),
                            )),
                    ),
            )
            .into_any_element()
    }
}
