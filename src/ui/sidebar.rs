use gpui::{div, prelude::*, rgb, ClickEvent, Context, ElementId, SharedString, Window};

use super::Theme;

pub fn sidebar_row<T: 'static>(
    id: impl Into<ElementId>,
    label: SharedString,
    active: bool,
    cx: &Context<T>,
    on_click: impl Fn(&mut T, &ClickEvent, &mut Window, &mut Context<T>) + 'static,
) -> impl IntoElement {
    let t = Theme::current();
    div()
        .id(id)
        .w_full()
        .px_2()
        .py_1p5()
        .rounded_md()
        .text_sm()
        .cursor_pointer()
        .overflow_hidden()
        .whitespace_nowrap()
        .when(active, |s| {
            s.bg(rgb(t.row_active)).text_color(rgb(t.on_accent))
        })
        .when(!active, |s| {
            s.text_color(rgb(t.inactive))
                .hover(|s| s.bg(rgb(t.surface_hover)).text_color(rgb(t.text)))
        })
        .child(label)
        .on_click(cx.listener(on_click))
}
