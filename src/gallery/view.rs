use gpui::{div, prelude::*, px, rgb, Context, SharedString, Window};

use crate::media::Entry;
use crate::ui::{btn, sidebar_row, Theme, SIDEBAR_W};

use super::{density::Density, Gallery, GoUp, ToggleFlat, ToggleSaved, ToggleSlideshow, GAP, PAD};

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (_, tile) = self.layout(window);
        let count = self.entries.len();
        let selected = self.selected;
        let density = self.density;
        let loading = self.loading;
        let slideshow = self.slideshow;
        let flat = self.prefs.flat_mode;
        let saved = self.prefs.is_saved(&self.root);
        let crumb = self.breadcrumb();
        let folder_full: SharedString = self.folder.display().to_string().into();

        let folders = self
            .entries
            .iter()
            .filter(|e| matches!(e, Entry::Folder(_)))
            .count();
        let media = count.saturating_sub(folders);
        let status: SharedString = if loading {
            "Loading…".into()
        } else if flat {
            format!("{media} media").into()
        } else {
            format!("{folders} folders · {media} media").into()
        };

        let recents = self.prefs.recents.clone();
        let saved_list = self.prefs.saved.clone();
        let current_root = self.root.clone();
        let t = Theme::DARK;

        let root = div()
            .id("gallery")
            .key_context("Gallery")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::close_viewer))
            .on_action(cx.listener(Self::next_item))
            .on_action(cx.listener(Self::prev_item))
            .on_action(cx.listener(Self::open_focused))
            .on_action(cx.listener(Self::on_move_left))
            .on_action(cx.listener(Self::on_move_right))
            .on_action(cx.listener(Self::on_move_up))
            .on_action(cx.listener(Self::on_move_down))
            .on_action(cx.listener(Self::open_folder_action))
            .on_action(cx.listener(Self::go_up))
            .on_action(cx.listener(Self::density_small))
            .on_action(cx.listener(Self::density_medium))
            .on_action(cx.listener(Self::density_large))
            .on_action(cx.listener(Self::toggle_slideshow))
            .on_action(cx.listener(Self::toggle_flat))
            .on_action(cx.listener(Self::toggle_saved))
            .on_action(cx.listener(Self::reset_zoom))
            .size_full()
            .flex()
            .flex_row()
            .bg(rgb(t.bg))
            .text_color(rgb(t.text))
            // Sidebar
            .child(
                div()
                    .id("sidebar")
                    .w(px(SIDEBAR_W))
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .px_3()
                    .py_3()
                    .border_r_1()
                    .border_color(rgb(t.border))
                    .bg(rgb(t.surface))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("gallery"),
                            )
                            .child(btn(
                                "open-sidebar",
                                "Open Folder",
                                false,
                                true,
                                cx,
                                |this, _, _, cx| this.pick_folder(cx),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .px_2()
                                    .text_xs()
                                    .text_color(rgb(t.text_faint))
                                    .child("SAVED"),
                            )
                            .when(saved_list.is_empty(), |s| {
                                s.child(
                                    div()
                                        .px_2()
                                        .text_xs()
                                        .text_color(rgb(t.text_hint))
                                        .child("Pin a library with Save"),
                                )
                            })
                            .children(saved_list.into_iter().enumerate().map(|(i, path)| {
                                let label: SharedString = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("folder")
                                    .to_string()
                                    .into();
                                let active = path == current_root;
                                sidebar_row(
                                    ("saved", i),
                                    label,
                                    active,
                                    cx,
                                    move |this, _, _, cx| {
                                        this.open_library(path.clone(), true, cx);
                                    },
                                )
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .px_2()
                                    .text_xs()
                                    .text_color(rgb(t.text_faint))
                                    .child("RECENT"),
                            )
                            .children(recents.into_iter().enumerate().map(|(i, path)| {
                                let label: SharedString = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("folder")
                                    .to_string()
                                    .into();
                                let active = path == current_root;
                                sidebar_row(
                                    ("recent", i),
                                    label,
                                    active,
                                    cx,
                                    move |this, _, _, cx| {
                                        this.open_library(path.clone(), true, cx);
                                    },
                                )
                            })),
                    ),
            )
            // Main
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(rgb(t.border))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_3()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .min_w_0()
                                            .flex_1()
                                            .child(btn(
                                                "back",
                                                "← Back",
                                                false,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.go_up(&GoUp, window, cx);
                                                },
                                            ))
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .min_w_0()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .overflow_hidden()
                                                            .whitespace_nowrap()
                                                            .child(crumb),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(rgb(t.text_dim))
                                                            .overflow_hidden()
                                                            .whitespace_nowrap()
                                                            .child(folder_full),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(btn(
                                                "save",
                                                if saved { "Saved ★" } else { "Save" },
                                                saved,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.toggle_saved(&ToggleSaved, window, cx);
                                                },
                                            ))
                                            .child(btn(
                                                "flat",
                                                if flat { "Flat" } else { "Folders" },
                                                flat,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.toggle_flat(&ToggleFlat, window, cx);
                                                },
                                            ))
                                            .child(
                                                div()
                                                    .flex()
                                                    .gap_1()
                                                    .child(btn(
                                                        "d-s",
                                                        Density::Small.label(),
                                                        density == Density::Small,
                                                        false,
                                                        cx,
                                                        |this, _, _, cx| {
                                                            this.set_density(Density::Small, cx)
                                                        },
                                                    ))
                                                    .child(btn(
                                                        "d-m",
                                                        Density::Medium.label(),
                                                        density == Density::Medium,
                                                        false,
                                                        cx,
                                                        |this, _, _, cx| {
                                                            this.set_density(Density::Medium, cx)
                                                        },
                                                    ))
                                                    .child(btn(
                                                        "d-l",
                                                        Density::Large.label(),
                                                        density == Density::Large,
                                                        false,
                                                        cx,
                                                        |this, _, _, cx| {
                                                            this.set_density(Density::Large, cx)
                                                        },
                                                    )),
                                            )
                                            .child(btn(
                                                "slideshow",
                                                if slideshow { "Stop" } else { "Slideshow" },
                                                slideshow,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.toggle_slideshow(
                                                        &ToggleSlideshow,
                                                        window,
                                                        cx,
                                                    );
                                                },
                                            ))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(t.text_muted))
                                                    .child(status),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("grid")
                            .flex_1()
                            .w_full()
                            .overflow_y_scroll()
                            .p(px(PAD))
                            .when(loading, |s| {
                                s.flex().items_center().justify_center().child(
                                    div().text_color(rgb(t.text_dim)).child("Loading folder…"),
                                )
                            })
                            .when(!loading && count == 0, |s| {
                                s.flex().items_center().justify_center().child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .gap_3()
                                        .child(
                                            div()
                                                .text_color(rgb(t.text_dim))
                                                .child("Nothing here yet."),
                                        )
                                        .child(btn(
                                            "open-empty",
                                            "Open Folder",
                                            false,
                                            true,
                                            cx,
                                            |this, _, _, cx| this.pick_folder(cx),
                                        )),
                                )
                            })
                            .when(!loading && count > 0, |s| {
                                s.child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .flex_row()
                                        .flex_wrap()
                                        .gap(px(GAP))
                                        .children(self.entries.iter().enumerate().map(
                                            |(i, entry)| self.render_tile(i, entry, tile, cx),
                                        )),
                                )
                            }),
                    ),
            );

        root.when_some(selected, |s, index| {
            s.child(self.render_lightbox(index, cx))
        })
    }
}
