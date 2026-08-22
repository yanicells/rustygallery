use gpui::{div, prelude::*, px, rgb, Context, SharedString};

use crate::ui::{btn, Theme};

use super::{CloseSearch, ConfirmSearch, Gallery};

impl Gallery {
    pub(super) fn render_search(&self, cx: &Context<Self>) -> impl IntoElement {
        let t = Theme::DARK;
        let hits = self.search_hits();
        let choice = self.search_choice.min(hits.len().saturating_sub(1));
        let query: SharedString = if self.search_query.is_empty() {
            "Type a name…".into()
        } else {
            self.search_query.clone().into()
        };

        div()
            .id("search")
            .key_context("Search")
            .track_focus(&self.search_focus)
            .on_action(cx.listener(Self::close_search))
            .on_action(cx.listener(Self::confirm_search))
            .on_key_down(cx.listener(Self::on_search_key))
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
                    .w(px(420.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(t.border))
                    .bg(rgb(t.surface))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(if self.search_query.is_empty() {
                                        rgb(t.text_hint)
                                    } else {
                                        rgb(t.text)
                                    })
                                    .child(query),
                            )
                            .child(btn(
                                "search-close",
                                "Esc",
                                false,
                                false,
                                cx,
                                |this, _, window, cx| {
                                    this.close_search(&CloseSearch, window, cx);
                                },
                            )),
                    )
                    .when(hits.is_empty(), |s| {
                        s.child(
                            div()
                                .text_xs()
                                .text_color(rgb(t.text_dim))
                                .child("No matches in this listing"),
                        )
                    })
                    .children(hits.into_iter().take(12).enumerate().map(|(row, index)| {
                        let name = self.entries[index].name().clone();
                        let active = row == choice;
                        div()
                            .id(("search-hit", row))
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .when(active, |s| {
                                s.bg(rgb(t.row_active)).text_color(rgb(t.on_accent))
                            })
                            .when(!active, |s| s.text_color(rgb(t.text_muted)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.search_choice = row;
                                this.confirm_search(&ConfirmSearch, window, cx);
                            }))
                            .child(name)
                    })),
            )
    }
}
