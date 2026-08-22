use gpui::{div, prelude::*, px, rgb, Context, ExternalPaths, MouseButton, SharedString, Window};

use crate::media::Entry;
use crate::ui::{btn, btn_disabled, sidebar_row, Theme, SIDEBAR_W};

use super::{
    density::Density, DropHint, Filter, Gallery, GoUp, ToggleFlat, ToggleSaved, ToggleSlideshow,
    GAP, PAD,
};

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title(&format!("gallery — {}", self.folder.display()));

        let (_, tile) = self.layout(window);
        let count = self.entries.len();
        let selected = self.selected;
        let density = self.density;
        let loading = self.loading;
        let slideshow = self.slideshow;
        let flat = self.prefs.flat_mode;
        let saved = self.prefs.is_saved(&self.root);
        let first_run = !self.prefs.seen_open;
        let crumbs = self.breadcrumb_parts();
        let can_go_up = self.can_go_up();
        let folder_full: SharedString = self.folder.display().to_string().into();

        let visible: Vec<(usize, &Entry)> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| self.entry_visible(e))
            .collect();
        let visible_count = visible.len();
        let folders = visible
            .iter()
            .filter(|(_, e)| matches!(e, Entry::Folder(_)))
            .count();
        let media = visible_count.saturating_sub(folders);
        let status_left = self.status_left(folders, media);
        let status_path = self.status_path();
        let filter = self.filter;
        let sort = self.sort;
        let sort_desc = self.sort_desc;
        let search_open = self.search_open;
        let can_trash = !self.action_paths().is_empty();

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
            .on_action(cx.listener(Self::cycle_sort))
            .on_action(cx.listener(Self::toggle_sort_dir))
            .on_action(cx.listener(Self::filter_all))
            .on_action(cx.listener(Self::filter_images))
            .on_action(cx.listener(Self::filter_videos))
            .on_action(cx.listener(Self::toggle_search))
            .on_action(cx.listener(Self::reveal_in_finder))
            .on_action(cx.listener(Self::copy_path))
            .on_action(cx.listener(Self::new_folder))
            .on_action(cx.listener(Self::rename_focused))
            .on_action(cx.listener(Self::move_to_trash))
            .on_action(cx.listener(Self::duplicate_selected))
            .on_action(cx.listener(Self::cut_selected))
            .on_action(cx.listener(Self::copy_selected))
            .on_action(cx.listener(Self::paste_clipboard))
            .on_action(cx.listener(Self::move_to))
            .on_action(cx.listener(Self::copy_to))
            .on_action(cx.listener(Self::undo_last))
            .size_full()
            .flex()
            .flex_row()
            .bg(rgb(t.bg))
            .text_color(rgb(t.text))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                this.drop_external(None, paths, window, cx);
            }))
            .on_drag_move::<ExternalPaths>(cx.listener(
                |this, event: &gpui::DragMoveEvent<ExternalPaths>, _, cx| {
                    let paths = event.drag(cx).clone();
                    this.hint_external(&paths, cx);
                },
            ))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.clear_drop_hint(cx)),
            )
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
                            ))
                            .when(first_run, |s| {
                                s.child(
                                    div()
                                        .px_1()
                                        .text_xs()
                                        .text_color(rgb(t.text_faint))
                                        .child("or ⌘O"),
                                )
                            }),
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
                                            .child(if can_go_up {
                                                btn(
                                                    "back",
                                                    "← Back",
                                                    false,
                                                    false,
                                                    cx,
                                                    |this, _, window, cx| {
                                                        this.go_up(&GoUp, window, cx);
                                                    },
                                                )
                                                .into_any_element()
                                            } else {
                                                btn_disabled("back", "← Back").into_any_element()
                                            })
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .min_w_0()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap_1()
                                                            .text_sm()
                                                            .overflow_hidden()
                                                            .children(crumbs.into_iter().enumerate().flat_map(|(i, (label, path))| {
                                                                let t = Theme::DARK;
                                                                let mut bits = Vec::new();
                                                                if i > 0 {
                                                                    bits.push(
                                                                        div()
                                                                            .text_color(rgb(t.text_faint))
                                                                            .child("/")
                                                                            .into_any_element(),
                                                                    );
                                                                }
                                                                bits.push(match path {
                                                                    Some(path) => div()
                                                                        .id(("crumb", i))
                                                                        .cursor_pointer()
                                                                        .text_color(rgb(t.text_dim))
                                                                        .hover(|s| s.text_color(rgb(t.text)))
                                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                                            this.open_crumb(path.clone(), cx);
                                                                        }))
                                                                        .child(label)
                                                                        .into_any_element(),
                                                                    None => div()
                                                                        .font_weight(gpui::FontWeight::MEDIUM)
                                                                        .child(label)
                                                                        .into_any_element(),
                                                                });
                                                                bits
                                                            })),
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
                                            .child(
                                                div()
                                                    .flex()
                                                    .gap_1()
                                                    .child(btn(
                                                        "f-all",
                                                        "All",
                                                        filter == Filter::All,
                                                        false,
                                                        cx,
                                                        |this, _, _, cx| {
                                                            this.set_filter(Filter::All, cx)
                                                        },
                                                    ))
                                                    .child(btn(
                                                        "f-img",
                                                        "Images",
                                                        filter == Filter::Images,
                                                        false,
                                                        cx,
                                                        |this, _, _, cx| {
                                                            this.set_filter(Filter::Images, cx)
                                                        },
                                                    ))
                                                    .child(btn(
                                                        "f-vid",
                                                        "Videos",
                                                        filter == Filter::Videos,
                                                        false,
                                                        cx,
                                                        |this, _, _, cx| {
                                                            this.set_filter(Filter::Videos, cx)
                                                        },
                                                    )),
                                            )
                                            .child(btn(
                                                "sort",
                                                format!(
                                                    "{} {}",
                                                    sort.label(),
                                                    if sort_desc { "↓" } else { "↑" }
                                                ),
                                                false,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.cycle_sort(&super::CycleSort, window, cx);
                                                },
                                            ))
                                            .child(btn(
                                                "sort-dir",
                                                if sort_desc { "Desc" } else { "Asc" },
                                                sort_desc,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.toggle_sort_dir(
                                                        &super::ToggleSortDir,
                                                        window,
                                                        cx,
                                                    );
                                                },
                                            ))
                                            .child(btn(
                                                "search",
                                                "Search",
                                                search_open,
                                                false,
                                                cx,
                                                |this, _, window, cx| {
                                                    this.toggle_search(
                                                        &super::ToggleSearch,
                                                        window,
                                                        cx,
                                                    );
                                                },
                                            ))
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
                                            .child(if can_trash {
                                                btn(
                                                    "trash",
                                                    "Trash",
                                                    false,
                                                    false,
                                                    cx,
                                                    |this, _, window, cx| {
                                                        this.move_to_trash(
                                                            &super::MoveToTrash,
                                                            window,
                                                            cx,
                                                        );
                                                    },
                                                )
                                                .into_any_element()
                                            } else {
                                                btn_disabled("trash", "Trash").into_any_element()
                                            }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("grid")
                            .flex_1()
                            .w_full()
                            .relative()
                            .overflow_y_scroll()
                            .p(px(PAD))
                            .when_some(self.drop_hint, |s, hint| {
                                let t = Theme::DARK;
                                let text = match hint {
                                    DropHint::OpenLibrary => "Drop to open this folder",
                                    DropHint::ImportHere => "Drop to add to this folder",
                                };
                                s.child(
                                    div()
                                        .id("drop-hint")
                                        .absolute()
                                        .top_3()
                                        .left_0()
                                        .right_0()
                                        .mx_auto()
                                        .w(px(280.))
                                        .py_2()
                                        .rounded_md()
                                        .bg(rgb(t.surface))
                                        .border_1()
                                        .border_color(rgb(t.accent))
                                        .text_color(rgb(t.accent_soft))
                                        .text_xs()
                                        .flex()
                                        .justify_center()
                                        .child(text),
                                )
                            })
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
                                                .text_lg()
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(rgb(t.text))
                                                .child("Open a folder to start"),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(t.text_dim))
                                                .child("Photos and videos in that folder show up here."),
                                        )
                                        .child(btn(
                                            "open-empty",
                                            "Open Folder",
                                            false,
                                            true,
                                            cx,
                                            |this, _, _, cx| this.pick_folder(cx),
                                        ))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(t.text_faint))
                                                .child("Save a library to pin it. Recents remembers where you were."),
                                        ),
                                )
                            })
                            .when(!loading && count > 0 && visible_count == 0, |s| {
                                s.flex().items_center().justify_center().child(
                                    div()
                                        .text_color(rgb(t.text_dim))
                                        .child("Nothing matches this filter."),
                                )
                            })
                            .when(!loading && visible_count > 0, |s| {
                                s.child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .flex_row()
                                        .flex_wrap()
                                        .gap(px(GAP))
                                        .children(visible.into_iter().map(|(i, entry)| {
                                            self.render_tile(i, entry, tile, cx)
                                        })),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_4()
                            .py_2()
                            .border_t_1()
                            .border_color(rgb(t.border))
                            .bg(rgb(t.surface))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(t.text_muted))
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(status_left),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(t.text_dim))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(status_path),
                            ),
                    ),
            );

        root.when_some(selected, |s, index| {
            s.child(self.render_lightbox(index, cx))
        })
        .when(search_open, |s| s.child(self.render_search(cx)))
        .when(self.name_kind.is_some(), |s| s.child(self.render_name(cx)))
        .when(self.collision.is_some(), |s| {
            s.child(self.render_collision(cx))
        })
        .when(self.context.is_some(), |s| {
            s.child(self.render_context(window, cx))
        })
        .when(self.toast.is_some(), |s| s.child(self.render_toast(cx)))
    }
}
