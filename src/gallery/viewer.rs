use gpui::{
    point, px, App, Context, ImageSource, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, ScrollWheelEvent, Window,
};

use super::{assets, Gallery};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewMode {
    Fit,
    Fill,
    Actual,
}

pub(crate) struct ViewerState {
    pub(crate) zoom: f32,
    pub(crate) pan: Point<Pixels>,
    pub(crate) dragging: bool,
    pub(crate) drag_last: Point<Pixels>,
    pub(crate) mode: ViewMode,
    pub(crate) peek: bool,
    pub(crate) exif: bool,
    pub(crate) px: Option<(u32, u32)>,
    pub(crate) anim_paused: bool,
    pub(crate) source: Option<ImageSource>,
    pub(crate) still: Option<std::sync::Arc<gpui::Image>>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: point(px(0.), px(0.)),
            dragging: false,
            drag_last: point(px(0.), px(0.)),
            mode: ViewMode::Fit,
            peek: false,
            exif: false,
            px: None,
            anim_paused: false,
            source: None,
            still: None,
        }
    }
}

impl ViewerState {
    /// Release decoded previews before navigation or replacing the viewer state.
    pub(crate) fn clear_preview_assets(&mut self, cx: &mut App) {
        if let Some(source) = self.source.take() {
            assets::release_image(source, cx);
        }
        if let Some(still) = self.still.take() {
            assets::release_image(ImageSource::from(still), cx);
        }
        self.px = None;
    }

    pub(crate) fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan = point(px(0.), px(0.));
        self.dragging = false;
        self.anim_paused = false;
    }

    /// Keep the image point under the cursor fixed in the rendered body.
    fn zoom_around(&mut self, zoom: f32, position: Point<Pixels>, origin: Point<Pixels>) {
        let k = zoom / self.zoom.max(0.01);
        let local = position - origin;
        self.pan.x = local.x - (local.x - self.pan.x) * k;
        self.pan.y = local.y - (local.y - self.pan.y) * k;
        self.zoom = zoom;
    }
}

impl Gallery {
    pub(super) fn on_viewer_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        body_origin: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.selected.is_none() || self.selected_video_path().is_some() {
            return;
        }
        cx.stop_propagation();
        let dy: f32 = match event.delta {
            gpui::ScrollDelta::Pixels(p) => p.y.into(),
            gpui::ScrollDelta::Lines(p) => p.y * 40.0,
        };
        let factor = if dy > 0.0 { 1.1 } else { 1.0 / 1.1 };
        let old = self.viewer.zoom.max(0.01);
        let min = if self.viewer.mode == ViewMode::Actual {
            0.25
        } else {
            1.0
        };
        let new = (old * factor).clamp(min, 8.0);
        self.viewer.zoom_around(new, event.position, body_origin);
        if self.viewer.mode != ViewMode::Actual && self.viewer.zoom <= 1.01 {
            self.viewer.zoom = 1.0;
            self.viewer.pan = point(px(0.), px(0.));
        }
        cx.notify();
    }

    pub(super) fn on_viewer_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected.is_none()
            || self.selected_video_path().is_some()
            || event.button != MouseButton::Left
        {
            return;
        }
        cx.stop_propagation();
        if self.viewer.peek {
            self.reset_viewer(cx);
            self.selected = None;
            self.stop_slideshow();
            cx.notify();
            return;
        }
        if event.click_count >= 2 {
            self.viewer.reset_view();
            cx.notify();
            return;
        }
        if self.viewer.zoom > 1.0 || self.viewer.mode == ViewMode::Actual {
            self.viewer.dragging = true;
            self.viewer.drag_last = event.position;
            cx.notify();
        }
    }

    pub(super) fn on_viewer_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.viewer.dragging {
            return;
        }
        let dx = event.position.x - self.viewer.drag_last.x;
        let dy = event.position.y - self.viewer.drag_last.y;
        self.viewer.pan.x += dx;
        self.viewer.pan.y += dy;
        self.viewer.drag_last = event.position;
        cx.notify();
    }

    pub(super) fn on_viewer_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.viewer.dragging {
            self.viewer.dragging = false;
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{point, px};

    use super::{ViewMode, ViewerState};

    #[test]
    fn zoom_keeps_the_cursor_anchor_after_header_wrapping() {
        for origin in [
            point(px(0.), px(52.)),
            point(px(0.), px(104.)),
            point(px(24.), px(136.)),
        ] {
            let mut viewer = ViewerState {
                zoom: 2.0,
                pan: point(px(-20.), px(-30.)),
                ..ViewerState::default()
            };
            // Image coordinate (150, 100) is initially under this cursor.
            let cursor = origin + point(px(280.), px(170.));
            viewer.zoom_around(3.0, cursor, origin);
            assert_eq!(origin.x + viewer.pan.x + px(150.) * viewer.zoom, cursor.x);
            assert_eq!(origin.y + viewer.pan.y + px(100.) * viewer.zoom, cursor.y);
        }
    }

    #[test]
    fn peek_zoom_out_keeps_the_viewport_cursor_anchor() {
        let mut viewer = ViewerState {
            zoom: 2.0,
            pan: point(px(-20.), px(-30.)),
            peek: true,
            ..ViewerState::default()
        };
        let cursor = point(px(280.), px(170.));
        viewer.zoom_around(1.0, cursor, point(px(0.), px(0.)));
        assert_eq!(viewer.pan.x + px(150.) * viewer.zoom, cursor.x);
        assert_eq!(viewer.pan.y + px(100.) * viewer.zoom, cursor.y);
    }

    #[test]
    fn actual_is_distinct_from_fit() {
        assert_ne!(ViewMode::Fit, ViewMode::Actual);
        assert_ne!(ViewMode::Fill, ViewMode::Fit);
    }
}
