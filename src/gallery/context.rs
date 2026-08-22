use gpui::{div, prelude::*, px, rgb, Context, MouseButton, Window};

use crate::media::Entry;
use crate::ui::Theme;

use super::{
    CopyPath, CopySelection, CopyTo, CutSelection, Duplicate, Gallery, MoveTo, MoveToTrash,
    RevealInFinder,
};

impl Gallery {
    pub(super) fn render_context(&self, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let Some(menu) = &self.context else {
            return div().into_any_element();
        };
        let t = Theme::DARK;
        let index = menu.index;
        let folder = matches!(self.entries.get(index), Some(Entry::Folder(_)));
        let can_dup = matches!(self.entries.get(index), Some(Entry::Media(_)));
        let can_new = folder && !self.prefs.flat_mode;
        let item_h = 28.0;
        let rows = 10.0 + if can_new { 1.0 } else { 0.0 } + if can_dup { 1.0 } else { 0.0 };
        let mw = 196.0;
        let mh = 8.0 + rows * item_h;
        let vw: f32 = window.viewport_size().width.into();
        let vh: f32 = window.viewport_size().height.into();
        let x: f32 = menu.pos.x.into();
        let y: f32 = menu.pos.y.into();
        let left = x.clamp(8.0, (vw - mw - 8.0).max(8.0));
        let top = y.clamp(8.0, (vh - mh - 8.0).max(8.0));

        div()
            .id("tile-menu")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.dismiss_context(cx)),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| this.dismiss_context(cx)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(mw))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(t.border))
                    .bg(rgb(t.surface))
                    .py_1()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .child(menu_row("Open", cx, move |this, _, _, cx| {
                        this.context_open(index, cx);
                    }))
                    .child(menu_row("Rename", cx, move |this, _, window, cx| {
                        this.context_rename(index, window, cx);
                    }))
                    .when(can_dup, |s| {
                        s.child(menu_row("Duplicate", cx, move |this, _, window, cx| {
                            this.dismiss_context(cx);
                            this.duplicate_selected(&Duplicate, window, cx);
                        }))
                    })
                    .child(menu_row("Cut", cx, move |this, _, window, cx| {
                        this.dismiss_context(cx);
                        this.cut_selected(&CutSelection, window, cx);
                    }))
                    .child(menu_row("Copy", cx, move |this, _, window, cx| {
                        this.dismiss_context(cx);
                        this.copy_selected(&CopySelection, window, cx);
                    }))
                    .child(menu_row("Move to…", cx, move |this, _, window, cx| {
                        this.dismiss_context(cx);
                        this.move_to(&MoveTo, window, cx);
                    }))
                    .child(menu_row("Copy to…", cx, move |this, _, window, cx| {
                        this.dismiss_context(cx);
                        this.copy_to(&CopyTo, window, cx);
                    }))
                    .when(can_new, |s| {
                        s.child(menu_row(
                            "New Folder inside",
                            cx,
                            move |this, _, window, cx| {
                                this.context_new_folder(index, window, cx);
                            },
                        ))
                    })
                    .child(menu_row("Move to Trash", cx, move |this, _, window, cx| {
                        this.dismiss_context(cx);
                        this.move_to_trash(&MoveToTrash, window, cx);
                    }))
                    .child(menu_row(
                        "Reveal in Finder",
                        cx,
                        move |this, _, window, cx| {
                            this.dismiss_context(cx);
                            this.reveal_in_finder(&RevealInFinder, window, cx);
                        },
                    ))
                    .child(menu_row("Copy Path", cx, move |this, _, window, cx| {
                        this.dismiss_context(cx);
                        this.copy_path(&CopyPath, window, cx);
                    })),
            )
            .into_any_element()
    }
}

fn menu_row<T: 'static>(
    label: &'static str,
    cx: &Context<T>,
    on_click: impl Fn(&mut T, &gpui::ClickEvent, &mut Window, &mut Context<T>) + 'static,
) -> impl IntoElement {
    let t = Theme::DARK;
    div()
        .id(label)
        .px_3()
        .py_1()
        .text_sm()
        .cursor_pointer()
        .text_color(rgb(t.text_muted))
        .hover(|s| s.bg(rgb(t.row_active)).text_color(rgb(t.on_accent)))
        .on_click(cx.listener(on_click))
        .child(label)
}
