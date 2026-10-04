use gpui::{div, prelude::*, rgb, Context};

use crate::ui::{seg, seg_disabled, seg_label, segmented, Theme};

use super::{
    density::Density, CycleSort, Filter, Gallery, GoUp, MoveToTrash, ToggleFlat, ToggleSaved,
    ToggleSearch, ToggleSlideshow, ToggleSortDir,
};

impl Gallery {
    /// Two rows: where you are plus actions, then how the listing is shown.
    /// The second row wraps instead of clipping at narrow window widths.
    pub(super) fn render_toolbar(&self, cx: &Context<Self>) -> impl IntoElement {
        let t = Theme::current();
        let can_go_up = self.can_go_up();
        let can_trash = !self.action_paths().is_empty();
        let saved = self.prefs.is_saved(&self.root);
        let flat = self.prefs.flat_mode;
        let density = self.density;
        let filter = self.filter;
        let sort_desc = self.sort_desc;

        div()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(rgb(t.border))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(segmented().child(if can_go_up {
                        seg("back", "← Back", false, cx, |this, _, window, cx| {
                            this.go_up(&GoUp, window, cx);
                        })
                        .into_any_element()
                    } else {
                        seg_disabled("back", "← Back").into_any_element()
                    }))
                    .child(self.render_crumbs(cx))
                    .child(
                        segmented()
                            .child(seg(
                                "search",
                                "Search",
                                self.search_open,
                                cx,
                                |this, _, window, cx| {
                                    this.toggle_search(&ToggleSearch, window, cx);
                                },
                            ))
                            .child(seg(
                                "save",
                                if saved { "★ Saved" } else { "☆ Save" },
                                saved,
                                cx,
                                |this, _, window, cx| {
                                    this.toggle_saved(&ToggleSaved, window, cx);
                                },
                            ))
                            .child(seg(
                                "slideshow",
                                if self.slideshow {
                                    "Stop slideshow"
                                } else {
                                    "Slideshow"
                                },
                                self.slideshow,
                                cx,
                                |this, _, window, cx| {
                                    this.toggle_slideshow(&ToggleSlideshow, window, cx);
                                },
                            )),
                    )
                    .child(segmented().child(if can_trash {
                        seg("trash", "Trash", false, cx, |this, _, window, cx| {
                            this.move_to_trash(&MoveToTrash, window, cx);
                        })
                        .into_any_element()
                    } else {
                        seg_disabled("trash", "Trash").into_any_element()
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x_3()
                    .gap_y_2()
                    .child(
                        segmented()
                            .child(seg(
                                "f-all",
                                "All",
                                filter == Filter::All,
                                cx,
                                |this, _, _, cx| this.set_filter(Filter::All, cx),
                            ))
                            .child(seg(
                                "f-img",
                                "Images",
                                filter == Filter::Images,
                                cx,
                                |this, _, _, cx| this.set_filter(Filter::Images, cx),
                            ))
                            .child(seg(
                                "f-vid",
                                "Videos",
                                filter == Filter::Videos,
                                cx,
                                |this, _, _, cx| this.set_filter(Filter::Videos, cx),
                            ))
                            .child(seg(
                                "f-fav",
                                "Starred",
                                filter == Filter::Favorites,
                                cx,
                                |this, _, _, cx| this.set_filter(Filter::Favorites, cx),
                            )),
                    )
                    .child(
                        segmented()
                            .child(seg_label("Sort"))
                            .child(seg(
                                "sort",
                                self.sort.label(),
                                false,
                                cx,
                                |this, _, window, cx| {
                                    this.cycle_sort(&CycleSort, window, cx);
                                },
                            ))
                            .child(seg(
                                "sort-dir",
                                if sort_desc { "↓" } else { "↑" },
                                false,
                                cx,
                                |this, _, window, cx| {
                                    this.toggle_sort_dir(&ToggleSortDir, window, cx);
                                },
                            )),
                    )
                    .child(
                        segmented()
                            .child(seg(
                                "lay-folders",
                                "Folders",
                                !flat,
                                cx,
                                move |this, _, window, cx| {
                                    if flat {
                                        this.toggle_flat(&ToggleFlat, window, cx);
                                    }
                                },
                            ))
                            .child(seg(
                                "lay-flat",
                                "All files",
                                flat,
                                cx,
                                move |this, _, window, cx| {
                                    if !flat {
                                        this.toggle_flat(&ToggleFlat, window, cx);
                                    }
                                },
                            )),
                    )
                    .child(
                        segmented()
                            .child(seg_label("Tiles"))
                            .child(seg(
                                "d-s",
                                Density::Small.label(),
                                density == Density::Small,
                                cx,
                                |this, _, _, cx| this.set_density(Density::Small, cx),
                            ))
                            .child(seg(
                                "d-m",
                                Density::Medium.label(),
                                density == Density::Medium,
                                cx,
                                |this, _, _, cx| this.set_density(Density::Medium, cx),
                            ))
                            .child(seg(
                                "d-l",
                                Density::Large.label(),
                                density == Density::Large,
                                cx,
                                |this, _, _, cx| this.set_density(Density::Large, cx),
                            )),
                    ),
            )
    }

    /// Clickable path from the library root to the current folder.
    fn render_crumbs(&self, cx: &Context<Self>) -> impl IntoElement {
        let crumbs = self.breadcrumb_parts();
        div()
            .flex()
            .flex_1()
            .min_w_0()
            .items_center()
            .gap_1()
            .text_sm()
            .overflow_hidden()
            .whitespace_nowrap()
            .children(
                crumbs
                    .into_iter()
                    .enumerate()
                    .flat_map(|(i, (label, path))| {
                        let t = Theme::current();
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
                    }),
            )
    }
}
