use gpui::{
    div, img, prelude::*, px, rgb, ClickEvent, Context, MouseButton, MouseDownEvent, ObjectFit,
};

use crate::media::{Entry, MediaKind};
use crate::ui::Theme;

use super::Gallery;

impl Gallery {
    pub(super) fn render_tile(
        &self,
        index: usize,
        entry: &Entry,
        tile: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let focused = self.focused == Some(index);
        let checked = self.checked.contains(&index);
        let cut = self.is_cut(entry.path());
        let name = entry.name().clone();
        let t = Theme::DARK;

        let media = match entry {
            Entry::Folder(folder) => div()
                .size_full()
                .relative()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .bg(rgb(t.tile_folder))
                .text_color(rgb(t.accent_soft))
                .child(div().text_xl().child("📁"))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(t.text_muted))
                        .child("folder"),
                )
                .child(
                    div()
                        .absolute()
                        .top_1()
                        .right_1()
                        .px_1p5()
                        .rounded_md()
                        .bg(rgb(t.btn_active))
                        .text_color(rgb(t.on_accent))
                        .text_xs()
                        .child(format!("{}", folder.media_count)),
                )
                .into_any_element(),
            Entry::Media(item) if item.kind == MediaKind::Video => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(t.tile_media))
                .text_color(rgb(t.btn_text))
                .text_lg()
                .child("▶")
                .into_any_element(),
            Entry::Media(item) => {
                if let Some(thumb) = self.thumbs.get(&item.path).cloned() {
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
                        .bg(rgb(t.tile_media))
                        .text_color(rgb(t.text_hint))
                        .text_xs()
                        .child("…")
                        .into_any_element()
                }
            }
        };

        div()
            .id(("tile", index))
            .w(px(tile))
            .flex()
            .flex_col()
            .gap_1()
            .when(cut, |s| s.opacity(0.45))
            .cursor_pointer()
            .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
                this.click_tile(index, event, cx);
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                    this.open_tile_menu(index, event.position, cx);
                }),
            )
            .child(
                div()
                    .w(px(tile))
                    .h(px(tile))
                    .overflow_hidden()
                    .rounded_md()
                    .bg(rgb(t.tile))
                    .border_2()
                    .when(checked, |s| s.border_color(rgb(t.accent)))
                    .when(!checked && focused, |s| s.border_color(rgb(t.accent_soft)))
                    .when(!checked && !focused, |s| s.border_color(rgb(t.tile)))
                    .child(media),
            )
            .child(
                div()
                    .w(px(tile))
                    .px_1()
                    .text_xs()
                    .text_color(if checked || focused {
                        rgb(t.accent)
                    } else {
                        rgb(t.name_idle)
                    })
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(name),
            )
    }
}
