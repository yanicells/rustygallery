//! Native fullscreen capability for the Gallery window.

use gpui::Window;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSView, NSWindowCollectionBehavior};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub(crate) fn enable(window: &Window) {
    let Some(_mtm) = MainThreadMarker::new() else {
        return;
    };
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };

    // SAFETY: GPUI's borrowed handle keeps this NSView alive for this call,
    // and the main-thread check above makes AppKit access valid.
    let view = unsafe { appkit.ns_view.cast::<NSView>().as_ref() };
    let Some(native_window) = view.window() else {
        return;
    };

    let mut behavior = native_window.collectionBehavior();
    behavior.remove(
        NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::FullScreenNone,
    );
    behavior.insert(NSWindowCollectionBehavior::FullScreenPrimary);
    native_window.setCollectionBehavior(behavior);
}
