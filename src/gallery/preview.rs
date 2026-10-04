use gpui::{AppContext, Context, ImageSource};

use crate::media::{display_source, first_frame_image, is_animated, Entry, MediaKind};

use super::{assets, Gallery};

impl Gallery {
    /// Convert previews off the UI thread; discard results after navigation.
    pub(super) fn prepare_preview(&mut self, cx: &mut Context<Self>) {
        self.dispose_video();
        self.preview_gen += 1;
        self.viewer.clear_preview_assets(cx);
        let Some(index) = self.selected else { return };
        let Some(Entry::Media(item)) = self.entries.get(index) else {
            return;
        };
        let path = item.path.clone();
        if item.kind == MediaKind::Video {
            self.viewer.peek = false;
            self.viewer.exif = false;
            self.prepare_video(cx);
            return;
        }
        // Paths key GPUI's cached pixels and decode failures. Refresh even after a
        // folder reload has reset the viewer, including rotation and file repair.
        assets::release_image(ImageSource::from(path.clone()), cx);
        let generation = self.preview_gen;

        cx.spawn(async move |this, cx| {
            let source_path = path.clone();
            let (source, dimensions, still) = cx
                .background_spawn(async move {
                    let source = display_source(&source_path);
                    let dimensions = source.dimensions();
                    let still = is_animated(&source_path)
                        .then(|| first_frame_image(&source_path))
                        .flatten();
                    (source, dimensions, still)
                })
                .await;
            this.update(cx, |this, cx| {
                let current = this.selected.and_then(|i| this.entries.get(i));
                if this.preview_gen == generation
                    && matches!(current, Some(Entry::Media(m)) if m.path == path && m.kind == MediaKind::Image)
                {
                    let source = ImageSource::from(source);
                    assets::release_image(source.clone(), cx);
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
