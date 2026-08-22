use gpui::{
    point, px, size, Bounds, Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Point, ScrollWheelEvent, Window,
};

use super::Gallery;

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
        }
    }
}

impl ViewerState {
    pub(crate) fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan = point(px(0.), px(0.));
        self.dragging = false;
    }
}

pub(crate) fn body_bounds(window: &Window, peek: bool, exif: bool) -> Bounds<Pixels> {
    let viewport = window.viewport_size();
    let top = if peek { 0.0 } else { 52.0 };
    let bottom = if peek { 0.0 } else { 120.0 };
    let right = if !peek && exif { 240.0 } else { 0.0 };
    let width: f32 = viewport.width.into();
    let height: f32 = viewport.height.into();
    Bounds {
        origin: point(px(0.), px(top)),
        size: size(
            px((width - right).max(80.0)),
            px((height - top - bottom).max(80.0)),
        ),
    }
}

impl Gallery {
    pub(super) fn on_viewer_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected.is_none() {
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
        let k = new / old;
        let body = body_bounds(window, self.viewer.peek, self.viewer.exif);
        let local = event.position - body.origin;
        self.viewer.pan.x = local.x - (local.x - self.viewer.pan.x) * k;
        self.viewer.pan.y = local.y - (local.y - self.viewer.pan.y) * k;
        self.viewer.zoom = new;
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
        if self.selected.is_none() || event.button != MouseButton::Left {
            return;
        }
        cx.stop_propagation();
        if self.viewer.peek {
            self.selected = None;
            self.viewer = ViewerState::default();
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
    use super::ViewMode;

    #[test]
    fn actual_is_distinct_from_fit() {
        assert_ne!(ViewMode::Fit, ViewMode::Actual);
        assert_ne!(ViewMode::Fill, ViewMode::Fit);
    }
}
