use gpui::{div, prelude::*, px, rgb, Context};

use crate::ui::{btn, Theme};

use super::{Gallery, Undo};

impl Gallery {
    pub(super) fn render_toast(&self, cx: &Context<Self>) -> impl IntoElement {
        let Some(toast) = &self.toast else {
            return div().into_any_element();
        };
        let t = Theme::DARK;
        let can_undo = toast.undo.is_some();
        let text = toast.text.clone();

        div()
            .id("toast")
            .absolute()
            .bottom(px(48.))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .occlude()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(t.surface))
                    .border_1()
                    .border_color(rgb(t.border))
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().text_sm().text_color(rgb(t.text_muted)).child(text))
                    .when(can_undo, |s| {
                        s.child(btn(
                            "toast-undo",
                            "Undo",
                            false,
                            true,
                            cx,
                            |this, _, window, cx| {
                                this.undo_last(&Undo, window, cx);
                            },
                        ))
                    }),
            )
            .into_any_element()
    }
}
