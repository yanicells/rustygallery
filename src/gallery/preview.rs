use gpui::{AppContext, Context, ImageSource};

use crate::media::{display_source, first_frame_image, is_animated, Entry};

use super::Gallery;

impl Gallery {
    /// Convert previews off the UI thread; discard results after navigation.
    pub(super) fn prepare_preview(&mut self, cx: &mut Context<Self>) {
        self.viewer.clear_preview_assets(cx);
        let Some(index) = self.selected else { return };
        let Some(Entry::Media(item)) = self.entries.get(index) else {
            return;
        };
        let path = item.path.clone();
        // Paths key GPUI's cached pixels and decode failures. Refresh even after a
        // folder reload has reset the viewer, including rotation and file repair.
        ImageSource::from(path.clone()).remove_asset(cx);
        self.preview_gen += 1;
        let generation = self.preview_gen;

        cx.spawn(async move |this, cx| {
            let (source, dimensions, still) = cx
                .background_spawn(async move {
                    let source = display_source(&path);
                    let dimensions = source.dimensions();
                    let still = is_animated(&path)
                        .then(|| first_frame_image(&path))
                        .flatten();
                    (source, dimensions, still)
                })
                .await;
            this.update(cx, |this, cx| {
                if this.preview_gen == generation && this.selected == Some(index) {
                    let source = ImageSource::from(source);
                    source.remove_asset(cx);
                    this.viewer.source = Some(source);
                    this.viewer.px = dimensions;
                    this.viewer.still = still;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}
