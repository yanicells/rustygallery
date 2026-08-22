use gpui::{
    div, img, prelude::*, px, rgb, ClickEvent, Context, ExternalPaths, MouseButton, MouseDownEvent,
    ObjectFit,
};

use crate::media::{Entry, MediaKind};
use crate::ui::Theme;

use super::drag::{drag_preview, TileDrag};
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
        let t = Theme::current();
        let folder_dest = match entry {
            Entry::Folder(folder) => Some(folder.path.clone()),
            Entry::Media(_) => None,
        };
        let drag = self.drag_paths(index);
        let can_drag = !drag.is_empty();
        let starred = matches!(entry, Entry::Media(m) if self.is_favorite(&m.path));
        let star_path = matches!(entry, Entry::Media(_)).then(|| entry.path().to_path_buf());

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
            Entry::Media(item) if item.kind == MediaKind::Video => {
                let poster = self.thumbs.get(&item.path).cloned();
                div()
                    .size_full()
                    .relative()
                    .bg(rgb(t.tile_media))
                    .when_some(poster, |s, thumb| {
                        s.child(
                            img(thumb)
                                .id(("vthumb", index))
                                .size_full()
                                .object_fit(ObjectFit::Cover),
                        )
                    })
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(t.btn_text))
                            .text_lg()
                            .child("▶"),
                    )
                    .into_any_element()
            }
            Entry::Media(item) => {
                if let Some(thumb) = self.thumbs.get(&item.path).cloned() {
                    img(thumb)
                        .id(("thumb", index))
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .into_any_element()
                } else {
                    div().size_full().bg(rgb(t.tile_media)).into_any_element()
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
            .when(can_drag, |s| {
                s.on_drag(TileDrag { paths: drag }, drag_preview)
            })
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
                    .relative()
                    .overflow_hidden()
                    .rounded_md()
                    .bg(rgb(t.tile))
                    .border_2()
                    .when(checked, |s| s.border_color(rgb(t.accent)))
                    .when(!checked && focused, |s| s.border_color(rgb(t.accent_soft)))
                    .when(!checked && !focused, |s| s.border_color(rgb(t.tile)))
                    .when_some(folder_dest.clone(), |s, dest| {
                        let dest_tiles = dest.clone();
                        s.drag_over::<TileDrag>(|s, _, _, _| {
                            let t = Theme::current();
                            s.border_color(rgb(t.accent)).border_4()
                        })
                        .drag_over::<ExternalPaths>(|s, _, _, _| {
                            let t = Theme::current();
                            s.border_color(rgb(t.accent)).border_4()
                        })
                        .on_drop(cx.listener(move |this, drag: &TileDrag, window, cx| {
                            this.drop_tiles(dest_tiles.clone(), drag, window, cx);
                        }))
                        .on_drop(cx.listener(
                            move |this, paths: &ExternalPaths, window, cx| {
                                this.drop_external(Some(dest.clone()), paths, window, cx);
                            },
                        ))
                    })
                    .child(media)
                    .when_some(star_path, |s, path| {
                        s.child(
                            div()
                                .id(("star", index))
                                .absolute()
                                .top_1()
                                .left_1()
                                .px_1()
                                .rounded_md()
                                .bg(rgb(t.btn_active))
                                .text_xs()
                                .text_color(if starred {
                                    rgb(t.accent)
                                } else {
                                    rgb(t.text_hint)
                                })
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.star_path(path.clone(), cx);
                                    cx.stop_propagation();
                                }))
                                .child(if starred { "★" } else { "☆" }),
                        )
                    }),
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
