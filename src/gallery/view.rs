use gpui::{div, prelude::*, px, rgb, Context, ExternalPaths, MouseButton, SharedString, Window};

use crate::media::Entry;
use crate::ui::{btn, sidebar_row, Theme, SIDEBAR_W};

use super::{CycleTheme, DropHint, Gallery, ToggleVideoPref, PAD};

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Theme::set_current(Theme::resolve(&self.prefs.theme, window.appearance()));
        window.set_window_title(&format!("gallery — {}", self.folder.display()));

        let count = self.entries.len();
        let selected = self.selected;
        let loading = self.loading;
        let first_run = !self.prefs.seen_open;

        let visible = self.visible_indices();
        if loading || visible.is_empty() {
            self.queue_thumbs(std::iter::empty(), window, cx);
        }
        let visible_count = visible.len();
        let folders = visible
            .iter()
            .filter(|&&index| matches!(self.entries[index], Entry::Folder(_)))
            .count();
        let media = visible_count.saturating_sub(folders);
        let status_left = self.status_left(folders, media);
        let status_path = self.status_path();
        let search_open = self.search_open;

        let recents = self.prefs.recents.clone();
        let saved_list = self.prefs.saved.clone();
        let current_root = self.root.clone();
        let theme_label = match self.prefs.theme.as_str() {
            "light" => "Light",
            "system" => "System",
            _ => "Dark",
        };
        let video_label = if self.prefs.video_inline {
            "Video: in app"
        } else {
            "Video: system player"
        };
        let t = Theme::current();

        let root = div()
            .id("gallery")
            .key_context(if self.selected_video_path().is_some() && !self.player_shortcuts_blocked() {
                "Gallery Video"
            } else {
                "Gallery"
            })
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
            .on_action(cx.listener(Self::filter_favorites))
            .on_action(cx.listener(Self::toggle_star))
            .on_action(cx.listener(Self::cycle_theme))
            .on_action(cx.listener(Self::toggle_video_pref))
            .on_action(cx.listener(Self::show_about))
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
            .on_action(cx.listener(Self::toggle_fullscreen))
            .on_action(cx.listener(Self::rotate_left))
            .on_action(cx.listener(Self::rotate_right))
            .on_action(cx.listener(Self::view_fit))
            .on_action(cx.listener(Self::view_fill))
            .on_action(cx.listener(Self::view_actual))
            .on_action(cx.listener(Self::video_toggle_playback))
            .on_action(cx.listener(Self::video_seek_back))
            .on_action(cx.listener(Self::video_seek_forward))
            .on_action(cx.listener(Self::video_mute))
            .on_action(cx.listener(Self::video_volume_up))
            .on_action(cx.listener(Self::video_volume_down))
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
                    .min_h_0()
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
                            .flex_shrink_0()
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
                            .id("sidebar-folders")
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px_2()
                                            .text_xs()
                                            .text_color(rgb(t.text_faint))
                                            .child("Saved"),
                                    )
                                    .when(saved_list.is_empty(), |s| {
                                        s.child(
                                            div()
                                                .px_2()
                                                .text_xs()
                                                .text_color(rgb(t.text_hint))
                                                .child("Use Save to pin a folder"),
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
                                    .flex_shrink_0()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px_2()
                                            .text_xs()
                                            .text_color(rgb(t.text_faint))
                                            .child("Recent"),
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
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_shrink_0()
                            .gap_1()
                            .child(sidebar_row(
                                "theme",
                                format!("Theme: {theme_label}").into(),
                                false,
                                cx,
                                |this, _, window, cx| this.cycle_theme(&CycleTheme, window, cx),
                            ))
                            .child(sidebar_row(
                                "video-pref",
                                video_label.into(),
                                false,
                                cx,
                                |this, _, window, cx| {
                                    this.toggle_video_pref(&ToggleVideoPref, window, cx);
                                },
                            )),
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
                    .child(self.render_toolbar(cx))
                    .child(
                        div()
                            .id("grid")
                            .flex_1()
                            .w_full()
                            .relative()
                            .overflow_hidden()
                            .min_h_0()
                            .p(px(PAD))
                            .when_some(self.drop_hint, |s, hint| {
                                let t = Theme::current();
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
                                                .child("Folders you open appear under Recent. Use Save to pin one."),
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
                                s.child(self.render_grid(visible, window, cx))
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
            s.child(self.render_lightbox(index, window, cx))
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
        .when(self.about_open, |s| s.child(self.render_about(cx)))
    }
}
