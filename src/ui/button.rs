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

/// Shared well that groups related `seg` controls into one compact unit.
pub fn segmented() -> gpui::Div {
    div()
        .flex()
        .items_center()
        .flex_shrink_0()
        .gap_0p5()
        .p_0p5()
        .rounded_md()
        .bg(rgb(Theme::current().btn))
}

/// Small control that sits inside `segmented`; `active` lifts it above its siblings.
pub fn seg<T: 'static>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    active: bool,
    cx: &Context<T>,
    on_click: impl Fn(&mut T, &ClickEvent, &mut Window, &mut Context<T>) + 'static,
) -> impl IntoElement {
    let t = Theme::current();
    div()
        .id(id.into())
        .px_2()
        .py_1()
        .rounded_sm()
        .text_sm()
        .cursor_pointer()
        .when(active, |s| {
            s.bg(rgb(t.btn_active)).text_color(rgb(t.on_accent))
        })
        .when(!active, |s| {
            s.text_color(rgb(t.btn_text))
                .hover(|s| s.bg(rgb(t.btn_hover)).text_color(rgb(t.on_accent)))
        })
        .child(label.into())
        .on_click(cx.listener(on_click))
}

pub fn seg_disabled(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .id(id.into())
        .px_2()
        .py_1()
        .rounded_sm()
        .text_sm()
        .text_color(rgb(Theme::current().text_hint))
        .child(label.into())
}

/// Quiet caption inside a `segmented` group, naming what its controls change.
pub fn seg_label(label: impl Into<SharedString>) -> impl IntoElement {
    div()
        .px_2()
        .text_xs()
        .text_color(rgb(Theme::current().text_faint))
        .child(label.into())
}
