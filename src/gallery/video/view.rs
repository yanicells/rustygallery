use std::{cell::Cell, rc::Rc};

use gpui::{
    canvas, div, prelude::*, px, relative, rgb, surface, AnyElement, Bounds, Context, MouseButton,
    ObjectFit, Pixels, Window,
};

use crate::media::{
    video::{PlaybackState, VideoSnapshot},
    MediaItem,
};
use crate::ui::{btn, btn_disabled, Theme};

use super::super::{Gallery, RenameFocused, ToggleFullscreen, ToggleStar};

impl Gallery {
    pub(in crate::gallery) fn render_video(
        &self,
        index: usize,
        item: &MediaItem,
        window: &Window,
        cx: &Context<Self>,
    ) -> AnyElement {
        let t = Theme::current();
        let player = self.video.player.as_ref();
        let snapshot = player.map(|player| player.snapshot());
        let error = self
            .video
            .error
            .as_ref()
            .or_else(|| snapshot.and_then(|s| s.error.as_ref()));
        let state = snapshot.map(|s| &s.state);
        let failed = error.is_some() || matches!(state, Some(PlaybackState::Failed));
        let status = if failed {
            "Unavailable"
        } else {
            state_label(state)
        };
        let frame = player.and_then(|player| player.frame());
        let message = if failed {
            "Cannot play this video in the gallery. Try the system player."
        } else if matches!(state, Some(PlaybackState::Loading)) {
            "Loading video…"
        } else if matches!(state, Some(PlaybackState::Waiting)) {
            "Waiting for video…"
        } else if player.is_none() {
            "Paused. Press Play to reopen this video."
        } else {
            "No video image at this position."
        };
        let strip = self.filmstrip_indices(index);

        div()
            .id("video-lightbox")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .flex_col()
            .bg(rgb(t.lightbox))
            .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(self.render_video_header(index, item, status, window, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .relative()
                    .overflow_hidden()
                    .child(if let Some(frame) = frame {
                        surface(frame).size_full().object_fit(ObjectFit::Contain).into_any_element()
                    } else {
                        div().size_full().flex().items_center().justify_center()
                            .text_sm().text_color(rgb(t.text_muted)).child(message).into_any_element()
                    })
                    .when_some(error, |s, error| {
                        s.child(div().absolute().bottom_4().left_4().right_4().p_3()
                            .rounded_md().bg(rgb(t.surface)).text_sm().child(error.clone()))
                    }),
            )
            .child(self.render_video_controls(snapshot, failed, cx))
            .child(self.render_filmstrip(index, &strip, cx))
            .child(div().px_4().py_2().text_xs().text_color(rgb(t.text_dim))
                .child("Space play/pause · ← → previous/next · Shift ← → seek · ↑ ↓ volume · M mute · Esc back"))
            .into_any_element()
    }

    fn render_video_header(
        &self,
        index: usize,
        item: &MediaItem,
        status: &str,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let t = Theme::current();
        let starred = self.is_favorite(&item.path);
        let fullscreen = window.is_fullscreen();
        div()
            .flex()
            .items_center()
            .justify_between()
            .flex_wrap()
            .px_4()
            .py_3()
            .gap_3()
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("video-name")
                            .text_sm()
                            .text_color(rgb(t.accent_soft))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.rename_focused(&RenameFocused, window, cx);
                            }))
                            .child(item.name.clone()),
                    )
                    .child(div().text_xs().text_color(rgb(t.text_dim)).child(format!(
                        "· {} / {} · Video · {status}",
                        index + 1,
                        self.entries.len()
                    ))),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(btn(
                        "video-star",
                        if starred { "★" } else { "☆" },
                        starred,
                        false,
                        cx,
                        |this, _, window, cx| this.toggle_star(&ToggleStar, window, cx),
                    ))
                    .child(btn(
                        "video-external",
                        "System player",
                        false,
                        false,
                        cx,
                        |this, _, window, cx| this.play_in_system(window, cx),
                    ))
                    .child(btn(
                        "video-fullscreen",
                        if fullscreen { "Window" } else { "Full" },
                        fullscreen,
                        false,
                        cx,
                        |this, _, window, cx| this.toggle_fullscreen(&ToggleFullscreen, window, cx),
                    ))
                    .child(btn(
                        "video-close",
                        "Close",
                        false,
                        false,
                        cx,
                        |this, _, _, cx| {
                            this.reset_viewer(cx);
                            this.selected = None;
                            this.stop_slideshow();
                            cx.notify();
                        },
                    )),
            )
    }

    fn render_video_controls(
        &self,
        snapshot: Option<&VideoSnapshot>,
        failed: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let t = Theme::current();
        let state = snapshot.map(|s| &s.state);
        let position = snapshot.map_or(0.0, |s| s.position);
        let duration = snapshot
            .and_then(|s| s.duration)
            .filter(|d| d.is_finite() && *d > 0.0);
        let volume = self.video.volume;
        let muted = self.video.muted;
        let playing = matches!(state, Some(PlaybackState::Playing | PlaybackState::Waiting))
            || (matches!(state, Some(PlaybackState::Loading)) && self.video.play_requested);
        let play_label = if failed {
            "Retry"
        } else if playing {
            "Pause"
        } else if matches!(state, Some(PlaybackState::Ended)) {
            "Replay"
        } else {
            "Play"
        };
        let can_seek = snapshot.is_some() && !failed && duration.is_some();
        div()
            .px_4()
            .py_2()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.render_video_seek(position, duration, can_seek, cx))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .child(btn(
                                "video-prev",
                                "←",
                                false,
                                false,
                                cx,
                                |this, _, _, cx| this.step_image(-1, cx),
                            ))
                            .child(btn(
                                "video-play",
                                play_label,
                                playing,
                                true,
                                cx,
                                |this, _, _, cx| this.toggle_video_playback(cx),
                            ))
                            .child(self.video_seek_button("video-back", "−5s", -5.0, can_seek, cx))
                            .child(self.video_seek_button(
                                "video-forward",
                                "+5s",
                                5.0,
                                can_seek,
                                cx,
                            ))
                            .child(btn(
                                "video-next",
                                "→",
                                false,
                                false,
                                cx,
                                |this, _, _, cx| this.step_image(1, cx),
                            ))
                            .child(div().text_sm().text_color(rgb(t.text_muted)).child(format!(
                                "{} / {}",
                                time_label(Some(position)),
                                time_label(duration)
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(btn(
                                "video-mute",
                                if muted { "Unmute" } else { "Mute" },
                                muted,
                                false,
                                cx,
                                |this, _, _, cx| this.toggle_video_mute(cx),
                            ))
                            .child(btn(
                                "video-volume-down",
                                "−",
                                false,
                                false,
                                cx,
                                |this, _, _, cx| this.change_video_volume(-0.05, cx),
                            ))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(t.text_muted))
                                    .child(format!("{:.0}%", volume * 100.0)),
                            )
                            .child(btn(
                                "video-volume-up",
                                "+",
                                false,
                                false,
                                cx,
                                |this, _, _, cx| this.change_video_volume(0.05, cx),
                            )),
                    ),
            )
    }

    fn video_seek_button(
        &self,
        id: &'static str,
        label: &'static str,
        delta: f64,
        enabled: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        if enabled {
            btn(id, label, false, false, cx, move |this, _, _, cx| {
                this.seek_video_by(delta, cx)
            })
            .into_any_element()
        } else {
            btn_disabled(id, label).into_any_element()
        }
    }

    /// Use the actual painted bounds so a click seeks correctly after window resizing.
    fn render_video_seek(
        &self,
        position: f64,
        duration: Option<f64>,
        enabled: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let t = Theme::current();
        let bounds: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
        let painted_bounds = bounds.clone();
        let identity = self.video.identity.clone();
        div()
            .id("video-seek")
            .h(px(20.0))
            .w_full()
            .relative()
            .when(enabled, |s| s.cursor_pointer())
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(8.0))
                    .h(px(4.0))
                    .rounded_md()
                    .bg(rgb(t.btn_active)),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(8.0))
                    .h(px(4.0))
                    .w(relative(timeline_fraction(position, duration)))
                    .rounded_md()
                    .bg(rgb(t.accent)),
            )
            .child(
                canvas(
                    move |area, _, _| painted_bounds.set(Some(area)),
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    let (Some(area), Some(duration), Some(identity)) =
                        (bounds.get(), duration, &identity)
                    else {
                        return;
                    };
                    if !enabled || !this.video_session_current(identity) {
                        return;
                    }
                    let width: f32 = area.size.width.into();
                    let offset: f32 = (event.position.x - area.origin.x).into();
                    if let Some(fraction) = click_fraction(offset, width) {
                        this.seek_video(duration * fraction, cx);
                    }
                }),
            )
    }
}

fn state_label(state: Option<&PlaybackState>) -> &'static str {
    match state {
        Some(PlaybackState::Loading) => "Loading",
        Some(PlaybackState::Playing) => "Playing",
        Some(PlaybackState::Waiting) => "Waiting",
        Some(PlaybackState::Ended) => "Ended",
        Some(PlaybackState::Failed) => "Unavailable",
        Some(PlaybackState::Paused) | None => "Paused",
    }
}

fn timeline_fraction(position: f64, duration: Option<f64>) -> f32 {
    match duration {
        Some(duration) if duration.is_finite() && duration > 0.0 && position.is_finite() => {
            (position / duration).clamp(0.0, 1.0) as f32
        }
        _ => 0.0,
    }
}

fn click_fraction(offset: f32, width: f32) -> Option<f64> {
    (offset.is_finite() && width.is_finite() && width > 0.0)
        .then(|| f64::from((offset / width).clamp(0.0, 1.0)))
}

fn time_label(seconds: Option<f64>) -> String {
    let Some(seconds) = seconds.filter(|s| s.is_finite() && *s >= 0.0) else {
        return "--:--".into();
    };
    let seconds = seconds as u64;
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::{click_fraction, timeline_fraction};

    #[test]
    fn timeline_uses_actual_width_and_clamps_clicks_and_playhead() {
        assert_eq!(click_fraction(150.0, 300.0), Some(0.5));
        assert_eq!(click_fraction(150.0, 600.0), Some(0.25));
        assert_eq!(click_fraction(-5.0, 300.0), Some(0.0));
        assert_eq!(click_fraction(350.0, 300.0), Some(1.0));
        assert_eq!(click_fraction(0.0, 0.0), None);
        assert_eq!(timeline_fraction(12.0, Some(10.0)), 1.0);
        assert_eq!(timeline_fraction(5.0, None), 0.0);
    }
}
