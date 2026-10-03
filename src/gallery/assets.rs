use gpui::{App, ImageSource, ImgResourceLoader, Window};

/// Cleanup without a window waits until the current window update has returned.
pub(super) fn release_image(source: ImageSource, cx: &mut App) {
    cx.defer(move |cx| {
        for handle in cx.windows() {
            if handle
                .update(cx, |_, window, cx| release_image_in(&source, window, cx))
                .is_ok()
            {
                return;
            }
        }
        // Closed windows have already released their atlases.
        source.remove_asset(cx);
    });
}

/// Removing a decode-cache entry alone leaves its uploaded textures in the atlas.
pub(super) fn release_image_in(source: &ImageSource, window: &mut Window, cx: &mut App) {
    let decoded = match source {
        ImageSource::Resource(resource) => window
            .get_asset::<ImgResourceLoader>(resource, cx)
            .and_then(Result::ok),
        ImageSource::Image(image) => image.clone().get_render_image(window, cx),
        ImageSource::Render(image) => Some(image.clone()),
        ImageSource::Custom(_) => None,
    };
    if let Some(image) = decoded {
        // An updating window is absent from App's window values.
        cx.drop_image(image, Some(window));
    }
    source.remove_asset(cx);
}
