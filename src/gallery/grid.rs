use std::{ops::Range, path::PathBuf, sync::Arc};

use gpui::{
    div, img, prelude::*, px, rgb, uniform_list, AnyElement, App, Bounds, ClickEvent, Context,
    ExternalPaths, MouseButton, MouseDownEvent, ObjectFit, Pixels, Point, ScrollStrategy,
    UniformList, UniformListDecoration, WeakEntity, Window,
};

use crate::media::{Entry, MediaKind};
use crate::ui::Theme;

use super::drag::{drag_preview, TileDrag};
use super::{Gallery, GAP};

const NAME_HEIGHT: f32 = 20.0;
const NAME_GAP: f32 = 4.0;
const THUMB_MARGIN_ROWS: usize = 2;

// A decoration receives the actual viewport range, unlike the row renderer,
// which GPUI also calls for measuring the first row before layout.
struct GridDemand {
    gallery: WeakEntity<Gallery>,
    indices: Arc<[usize]>,
    columns: usize,
    generation: u64,
}

impl UniformListDecoration for GridDemand {
    fn compute(
        &self,
        rows: Range<usize>,
        _: Bounds<Pixels>,
        _: Point<Pixels>,
        _: Pixels,
        _: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let positions = demand_positions(rows, self.columns, self.indices.len());
        self.gallery
            .update(cx, |this, cx| {
                if this.load_gen == self.generation && !this.loading {
                    this.queue_thumbs(
                        positions
                            .into_iter()
                            .flat_map(|range| self.indices[range].iter().copied()),
                        window,
                        cx,
                    );
                }
            })
            .ok();
        div().into_any_element()
    }
}

fn demand_positions(rows: Range<usize>, columns: usize, count: usize) -> [Range<usize>; 3] {
    let start = rows.start.saturating_mul(columns).min(count);
    let end = rows.end.saturating_mul(columns).min(count);
    let before = rows.start.saturating_sub(THUMB_MARGIN_ROWS) * columns;
    let after = rows
        .end
        .saturating_add(THUMB_MARGIN_ROWS)
        .saturating_mul(columns)
        .min(count);
    [start..end, before.min(start)..start, end..after]
}

impl Gallery {
    pub(super) fn render_grid(
        &mut self,
        indices: Vec<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> UniformList {
        let (columns, tile) = self.layout(window);
        let row_height = tile + NAME_GAP + NAME_HEIGHT + GAP;
        if self.grid_focus != self.focused || self.grid_columns != columns {
            if let Some(position) = self
                .focused
                .and_then(|index| indices.iter().position(|&i| i == index))
            {
                self.grid_scroll
                    .scroll_to_item(position / columns, ScrollStrategy::Top);
            }
            self.grid_focus = self.focused;
            self.grid_columns = columns;
        }
        let row_count = indices.len().div_ceil(columns);
        let indices: Arc<[usize]> = indices.into();
        let rows = indices.clone();
        let selected_drag = self.selected_drag_paths();
        let generation = self.load_gen;
        uniform_list(
            "grid-rows",
            row_count,
            cx.processor(move |this, range: Range<usize>, _, cx| {
                range
                    .map(|row| {
                        let start = row * columns;
                        let end = (start + columns).min(rows.len());
                        div()
                            .id(("grid-row", row))
                            .h(px(row_height))
                            .w_full()
                            .flex()
                            .flex_row()
                            .gap(px(GAP))
                            .pb(px(GAP))
                            .children(rows[start..end].iter().filter_map(|&index| {
                                this.entries.get(index).map(|entry| {
                                    this.render_tile(index, entry, tile, &selected_drag, cx)
                                })
                            }))
                    })
                    .collect()
            }),
        )
        .size_full()
        .track_scroll(self.grid_scroll.clone())
        .with_decoration(GridDemand {
            gallery: cx.entity().downgrade(),
            indices,
            columns,
            generation,
        })
    }

    pub(super) fn render_tile(
        &self,
        index: usize,
        entry: &Entry,
        tile: f32,
        selected_drag: &Arc<[PathBuf]>,
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
        let drag = self.drag_paths(index, selected_drag);
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
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(rgb(t.tile_media))
                        .text_color(rgb(t.text_muted))
                        .text_xs()
                        .child(if self.failed_thumbs.contains(&item.path) {
                            "Preview unavailable"
                        } else {
                            "Loading…"
                        })
                        .into_any_element()
                }
            }
        };

        div()
            .id(("tile", index))
            .w(px(tile))
            .flex_shrink_0()
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
                    .h(px(NAME_HEIGHT))
                    .line_height(px(NAME_HEIGHT))
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

#[cfg(test)]
mod tests {
    use super::demand_positions;

    #[test]
    fn viewport_demand_preserves_filtered_entry_indices_and_prioritizes_visible_rows() {
        let indices = [1, 3, 5, 8, 9, 12, 14, 17, 18, 20, 22];
        let requested: Vec<_> = demand_positions(2..3, 3, indices.len())
            .into_iter()
            .flat_map(|range| indices[range].iter().copied())
            .collect();
        assert_eq!(requested, [14, 17, 18, 1, 3, 5, 8, 9, 12, 20, 22]);
    }

    #[test]
    fn viewport_margin_stops_at_listing_edges() {
        assert_eq!(demand_positions(0..1, 4, 5), [0..4, 0..0, 4..5]);
        assert_eq!(demand_positions(1..2, 4, 5), [4..5, 0..4, 5..5]);
    }
}
