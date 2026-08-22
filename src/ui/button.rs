use gpui::{div, prelude::*, rgb, ClickEvent, Context, SharedString, Window};

use super::Theme;

pub fn btn<T: 'static>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    active: bool,
    prominent: bool,
    cx: &Context<T>,
    on_click: impl Fn(&mut T, &ClickEvent, &mut Window, &mut Context<T>) + 'static,
) -> impl IntoElement {
    let id = id.into();
    let t = Theme::current();
    div()
        .id(id)
        .px_3()
        .py_1p5()
        .rounded_md()
        .text_sm()
        .cursor_pointer()
        .when(prominent, |s| {
            s.bg(rgb(t.prominent))
                .text_color(rgb(t.prominent_text))
                .font_weight(gpui::FontWeight::MEDIUM)
                .hover(|s| s.bg(rgb(t.prominent_hover)))
        })
        .when(!prominent && active, |s| {
            s.bg(rgb(t.btn_active)).text_color(rgb(t.on_accent))
        })
        .when(!prominent && !active, |s| {
            s.bg(rgb(t.btn))
                .text_color(rgb(t.btn_text))
                .hover(|s| s.bg(rgb(t.btn_hover)).text_color(rgb(t.on_accent)))
        })
        .child(label.into())
        .on_click(cx.listener(on_click))
}

pub fn btn_disabled(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
) -> impl IntoElement {
    let t = Theme::current();
    div()
        .id(id.into())
        .px_3()
        .py_1p5()
        .rounded_md()
        .text_sm()
        .bg(rgb(t.btn))
        .text_color(rgb(t.text_hint))
        .child(label.into())
}
