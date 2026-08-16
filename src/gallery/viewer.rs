use gpui::{
    point, px, Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    ScrollWheelEvent, Window,
};

use super::Gallery;

pub(crate) struct ViewerState {
    pub(crate) zoom: f32,
    pub(crate) pan: Point<Pixels>,
    pub(crate) dragging: bool,
    pub(crate) drag_last: Point<Pixels>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: point(px(0.), px(0.)),
            dragging: false,
            drag_last: point(px(0.), px(0.)),
        }
    }
}

impl Gallery {
    pub(super) fn on_viewer_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected.is_none() {
            return;
        }
        let dy: f32 = match event.delta {
            gpui::ScrollDelta::Pixels(p) => p.y.into(),
            gpui::ScrollDelta::Lines(p) => p.y * 40.0,
        };
        let factor = if dy > 0.0 { 1.1 } else { 1.0 / 1.1 };
        let old = self.viewer.zoom;
        self.viewer.zoom = (old * factor).clamp(1.0, 8.0);
        if self.viewer.zoom <= 1.01 {
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
        if event.click_count >= 2 {
            self.viewer = ViewerState::default();
            cx.notify();
            return;
        }
        if self.viewer.zoom > 1.0 {
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
