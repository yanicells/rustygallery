//! Inline playback stays on GPUI's main thread; timers carry only session identity.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use gpui::{Context, Task, Window};

use crate::media::{
    video::{PlaybackState, VideoPlayer},
    Entry, MediaKind,
};

use super::{
    Gallery, VideoMute, VideoSeekBack, VideoSeekForward, VideoTogglePlayback, VideoVolumeDown,
    VideoVolumeUp,
};

mod view;

#[derive(Clone, PartialEq, Eq)]
struct SessionId {
    path: PathBuf,
    generation: u64,
}

impl SessionId {
    fn is_current(&self, current: Option<&Self>, selected: Option<&Path>) -> bool {
        current == Some(self) && selected == Some(self.path.as_path())
    }
}

pub(super) struct VideoViewState {
    generation: u64,
    identity: Option<SessionId>,
    player: Option<VideoPlayer>,
    error: Option<String>,
    tick: Option<Task<()>>,
    volume: f32,
    muted: bool,
    play_requested: bool,
}

impl Default for VideoViewState {
    fn default() -> Self {
        Self {
            generation: 0,
            identity: None,
            player: None,
            error: None,
            tick: None,
            volume: 1.0,
            muted: false,
            play_requested: false,
        }
    }
}

impl Gallery {
    pub(super) fn selected_video_path(&self) -> Option<PathBuf> {
        self.selected.and_then(|i| match self.entries.get(i) {
            Some(Entry::Media(item)) if item.kind == MediaKind::Video => Some(item.path.clone()),
            _ => None,
        })
    }

    pub(super) fn player_shortcuts_blocked(&self) -> bool {
        self.search_open
            || self.name_kind.is_some()
            || self.collision.is_some()
            || self.about_open
            || self.playback_hidden
    }

    fn video_session_current(&self, identity: &SessionId) -> bool {
        identity.is_current(
            self.video.identity.as_ref(),
            self.selected_video_path().as_deref(),
        ) && self.prefs.video_inline
            && !self.playback_hidden
    }

    /// Cancel outstanding ticks before releasing native audio and retained frames.
    pub(super) fn dispose_video(&mut self) {
        self.video.generation += 1;
        self.video.tick.take();
        if let Some(mut player) = self.video.player.take() {
            self.video.volume = player.snapshot().volume;
            self.video.muted = player.snapshot().muted;
            player.pause();
            player.dispose();
        }
        self.video.identity = None;
        self.video.error = None;
        self.video.play_requested = false;
    }

    /// Hiding retains the selection, but reopening native playback requires Play.
    pub(crate) fn hide_playback(&mut self, cx: &mut Context<Self>) {
        self.playback_hidden = true;
        self.stop_slideshow();
        self.dispose_video();
        cx.notify();
    }

    pub(crate) fn show_playback(&mut self, cx: &mut Context<Self>) {
        if self.playback_hidden {
            self.playback_hidden = false;
            cx.notify();
        }
    }

    pub(super) fn prepare_video(&mut self, cx: &mut Context<Self>) {
        self.dispose_video();
        self.stop_slideshow();
        let Some(path) = self.selected_video_path() else {
            return;
        };
        if !self.prefs.video_inline || self.playback_hidden {
            return;
        }
        let identity = SessionId {
            path,
            generation: self.video.generation,
        };
        self.video.identity = Some(identity.clone());
        match VideoPlayer::new(&identity.path) {
            Ok(mut player) => {
                player.set_volume(self.video.volume);
                if player.snapshot().muted != self.video.muted {
                    player.toggle_mute();
                }
                if player.poll() {
                    cx.notify();
                }
                self.video.player = Some(player);
            }
            Err(error) => {
                self.video.error = Some(error);
                return;
            }
        }

        self.video.tick = Some(cx.spawn(async move |this, cx| loop {
            let delay = this
                .update(cx, |this, _| {
                    if !this.video_session_current(&identity) {
                        return None;
                    }
                    this.video.player.as_ref().map(|player| {
                        if player.needs_poll() {
                            Duration::from_millis(33)
                        } else {
                            // Paused native failures still reach the UI without busy redraws.
                            Duration::from_millis(200)
                        }
                    })
                })
                .ok()
                .flatten();
            let Some(delay) = delay else { break };
            cx.background_executor().timer(delay).await;
            let keep_polling = this
                .update(cx, |this, cx| {
                    if !this.video_session_current(&identity) {
                        return false;
                    }
                    if crate::app::is_hidden() {
                        this.hide_playback(cx);
                        return false;
                    }
                    let Some(player) = this.video.player.as_mut() else {
                        return false;
                    };
                    if player.poll() {
                        if player.snapshot().state != PlaybackState::Loading {
                            this.video.play_requested = matches!(
                                player.snapshot().state,
                                PlaybackState::Playing | PlaybackState::Waiting
                            );
                        }
                        cx.notify();
                    }
                    true
                })
                .unwrap_or(false);
            if !keep_polling {
                break;
            }
        }));
    }

    pub(super) fn toggle_video_playback(&mut self, cx: &mut Context<Self>) {
        if self.player_shortcuts_blocked()
            || self.selected_video_path().is_none()
            || !self.prefs.video_inline
        {
            return;
        }
        let retry = self
            .video
            .player
            .as_ref()
            .is_none_or(|player| matches!(player.snapshot().state, PlaybackState::Failed));
        if retry {
            self.prepare_video(cx);
        }
        if let Some(player) = self.video.player.as_mut() {
            player.toggle_playback();
            self.video.play_requested = !self.video.play_requested;
        }
        cx.notify();
    }

    fn seek_video(&mut self, seconds: f64, cx: &mut Context<Self>) {
        if self.player_shortcuts_blocked() || self.selected_video_path().is_none() {
            return;
        }
        if let Some(player) = self.video.player.as_mut() {
            if let Some(target) = seek_target(seconds, player.snapshot().duration) {
                player.seek(target);
                cx.notify();
            }
        }
    }

    fn seek_video_by(&mut self, delta: f64, cx: &mut Context<Self>) {
        if let Some(player) = &self.video.player {
            self.seek_video(player.snapshot().position + delta, cx);
        }
    }

    fn set_video_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        if self.player_shortcuts_blocked()
            || self.selected_video_path().is_none()
            || !volume.is_finite()
        {
            return;
        }
        self.video.volume = volume.clamp(0.0, 1.0);
        if let Some(player) = self.video.player.as_mut() {
            player.set_volume(self.video.volume);
        }
        cx.notify();
    }

    pub(super) fn change_video_volume(&mut self, delta: f32, cx: &mut Context<Self>) {
        let volume = self
            .video
            .player
            .as_ref()
            .map_or(self.video.volume, |player| player.snapshot().volume);
        self.set_video_volume(volume + delta, cx);
    }

    fn toggle_video_mute(&mut self, cx: &mut Context<Self>) {
        if self.player_shortcuts_blocked() || self.selected_video_path().is_none() {
            return;
        }
        if let Some(player) = self.video.player.as_mut() {
            player.toggle_mute();
            self.video.muted = player.snapshot().muted;
        } else {
            self.video.muted = !self.video.muted;
        }
        cx.notify();
    }

    pub(super) fn video_toggle_playback(
        &mut self,
        _: &VideoTogglePlayback,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_video_playback(cx);
    }

    pub(super) fn video_seek_back(
        &mut self,
        _: &VideoSeekBack,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.seek_video_by(-5.0, cx);
    }

    pub(super) fn video_seek_forward(
        &mut self,
        _: &VideoSeekForward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.seek_video_by(5.0, cx);
    }

    pub(super) fn video_mute(&mut self, _: &VideoMute, _: &mut Window, cx: &mut Context<Self>) {
        self.toggle_video_mute(cx);
    }

    pub(super) fn video_volume_up(
        &mut self,
        _: &VideoVolumeUp,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.change_video_volume(0.05, cx);
    }

    pub(super) fn video_volume_down(
        &mut self,
        _: &VideoVolumeDown,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.change_video_volume(-0.05, cx);
    }
}

fn seek_target(seconds: f64, duration: Option<f64>) -> Option<f64> {
    if !seconds.is_finite() {
        return None;
    }
    let end = duration.filter(|value| value.is_finite() && *value >= 0.0);
    Some(seconds.max(0.0).min(end.unwrap_or(f64::MAX)))
}

pub(super) fn opens_inline(kind: MediaKind, video_inline: bool) -> bool {
    kind == MediaKind::Image || video_inline
}

pub(super) fn slideshow_eligible(entry: &Entry) -> bool {
    matches!(entry, Entry::Media(item) if item.kind == MediaKind::Image)
}

#[cfg(test)]
mod tests {
    use super::{opens_inline, seek_target, slideshow_eligible, SessionId};
    use crate::media::{Entry, MediaItem, MediaKind};

    #[test]
    fn video_preference_controls_routing_and_slideshows_accept_only_photos() {
        assert!(opens_inline(MediaKind::Image, false));
        assert!(opens_inline(MediaKind::Image, true));
        assert!(!opens_inline(MediaKind::Video, false));
        assert!(opens_inline(MediaKind::Video, true));
        let mut entry = Entry::Media(MediaItem {
            path: "/library/clip.mov".into(),
            name: "clip.mov".into(),
            kind: MediaKind::Video,
            modified: 0,
            size: 0,
        });
        assert!(!slideshow_eligible(&entry));
        if let Entry::Media(item) = &mut entry {
            item.kind = MediaKind::Image;
        }
        assert!(slideshow_eligible(&entry));
    }

    #[test]
    fn late_ticks_cannot_target_a_reopened_path_or_a_different_selection() {
        let first = SessionId {
            path: "/library/clip.mov".into(),
            generation: 1,
        };
        let reopened = SessionId {
            generation: 2,
            ..first.clone()
        };
        let other = SessionId {
            path: "/library/other.mov".into(),
            ..first.clone()
        };
        assert!(first.is_current(Some(&first), Some(first.path.as_path())));
        assert!(!first.is_current(Some(&reopened), Some(first.path.as_path())));
        assert!(!first.is_current(Some(&other), Some(other.path.as_path())));
        assert!(!first.is_current(Some(&first), Some(other.path.as_path())));
        assert!(!first.is_current(None, Some(first.path.as_path())));
        assert!(!first.is_current(Some(&first), None));
    }

    #[test]
    fn seeking_rejects_invalid_values_and_clamps_to_known_duration() {
        assert_eq!(seek_target(f64::NAN, Some(10.0)), None);
        assert_eq!(seek_target(f64::INFINITY, Some(10.0)), None);
        assert_eq!(seek_target(-5.0, Some(10.0)), Some(0.0));
        assert_eq!(seek_target(15.0, Some(10.0)), Some(10.0));
        assert_eq!(seek_target(5.0, None), Some(5.0));
        assert_eq!(seek_target(5.0, Some(f64::NAN)), Some(5.0));
    }
}
