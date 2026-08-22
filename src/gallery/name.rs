use gpui::{div, prelude::*, px, rgb, Context, SharedString};

use crate::ui::{btn, Theme};

use super::{CloseName, ConfirmName, Gallery, NameKind};

impl Gallery {
    pub(super) fn render_name(&self, cx: &Context<Self>) -> impl IntoElement {
        let t = Theme::DARK;
        let title = match &self.name_kind {
            Some(NameKind::Rename(_)) => "Rename",
            _ => "New folder",
        };
        let ok = match &self.name_kind {
            Some(NameKind::Rename(_)) => "Rename",
            _ => "Create",
        };
        let query: SharedString = if self.name_query.is_empty() {
            "Type a name…".into()
        } else {
            self.name_query.clone().into()
        };

        div()
            .id("name-prompt")
            .key_context("NamePrompt")
            .track_focus(&self.name_focus)
            .on_action(cx.listener(Self::close_name))
            .on_action(cx.listener(Self::confirm_name))
            .on_key_down(cx.listener(Self::on_name_key))
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
                    .w(px(360.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(t.border))
                    .bg(rgb(t.surface))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_xs().text_color(rgb(t.text_faint)).child(title))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(if self.name_query.is_empty() {
                                        rgb(t.text_hint)
                                    } else {
                                        rgb(t.text)
                                    })
                                    .child(query),
                            )
                            .child(btn(
                                "name-close",
                                "Esc",
                                false,
                                false,
                                cx,
                                |this, _, window, cx| {
                                    this.close_name(&CloseName, window, cx);
                                },
                            )),
                    )
                    .when_some(self.name_error.as_ref(), |s, err| {
                        s.child(
                            div()
                                .text_xs()
                                .text_color(rgb(t.accent_soft))
                                .child(err.clone()),
                        )
                    })
                    .child(div().flex().justify_end().child(btn(
                        "name-ok",
                        ok,
                        false,
                        true,
                        cx,
                        |this, _, window, cx| {
                            this.confirm_name(&ConfirmName, window, cx);
                        },
                    ))),
            )
    }
}
