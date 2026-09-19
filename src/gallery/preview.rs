use gpui::{AppContext, Context};

use crate::media::{display_source, first_frame_image, is_animated, Entry};

use super::Gallery;

impl Gallery {
    /// Convert previews off the UI thread; discard results after navigation.
    pub(super) fn prepare_preview(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.selected else { return };
        let Some(Entry::Media(item)) = self.entries.get(index) else {
            return;
        };
        let path = item.path.clone();
        let neighbors = self.neighbor_paths(index);
        self.preview_gen += 1;
        let generation = self.preview_gen;
        self.viewer.source = None;
        self.viewer.still = None;
        self.viewer.px = None;
        self.viewer.neighbors.clear();

        cx.spawn(async move |this, cx| {
            let (source, dimensions, still) = cx
                .background_spawn(async move {
                    let source = display_source(&path);
                    let dimensions = image::image_dimensions(&source).ok();
                    let still = is_animated(&path)
                        .then(|| first_frame_image(&path))
                        .flatten();
                    (source, dimensions, still)
                })
                .await;
            let current = this
                .update(cx, |this, cx| {
                    if this.preview_gen != generation || this.selected != Some(index) {
                        return false;
                    }
                    this.viewer.source = Some(source);
                    this.viewer.px = dimensions;
                    this.viewer.still = still;
                    cx.notify();
                    true
                })
                .unwrap_or(false);
            if !current {
                return;
            }
            let sources = cx
                .background_spawn(async move {
                    neighbors.iter().map(|path| display_source(path)).collect()
                })
                .await;
            this.update(cx, |this, cx| {
                if this.preview_gen == generation && this.selected == Some(index) {
                    this.viewer.neighbors = sources;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}
