//! Native video playback. The gallery owns controls and schedules `poll` calls.

#[cfg(target_os = "macos")]
mod composition;
#[cfg(target_os = "macos")]
mod frame;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub(crate) use macos::VideoPlayer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PlaybackState {
    Loading,
    Paused,
    Playing,
    Waiting,
    Ended,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VideoSnapshot {
    pub(crate) state: PlaybackState,
    pub(crate) position: f64,
    pub(crate) duration: Option<f64>,
    pub(crate) volume: f32,
    pub(crate) muted: bool,
    pub(crate) error: Option<String>,
}

impl Default for VideoSnapshot {
    fn default() -> Self {
        Self {
            state: PlaybackState::Loading,
            position: 0.0,
            duration: None,
            volume: 1.0,
            muted: false,
            error: None,
        }
    }
}

fn bounded_seek(seconds: f64, duration: Option<f64>) -> Option<f64> {
    if !seconds.is_finite() {
        return None;
    }
    let end = duration.filter(|d| d.is_finite() && *d >= 0.0);
    Some(seconds.max(0.0).min(end.unwrap_or(f64::MAX)))
}

#[cfg(not(target_os = "macos"))]
pub(crate) struct VideoPlayer {
    snapshot: VideoSnapshot,
}

#[cfg(not(target_os = "macos"))]
impl VideoPlayer {
    pub(crate) fn new(_: &std::path::Path) -> Result<Self, String> {
        Err("In-app video playback is supported on macOS only.".into())
    }

    pub(crate) fn poll(&mut self) -> bool {
        false
    }
    pub(crate) fn needs_poll(&self) -> bool {
        false
    }
    pub(crate) fn snapshot(&self) -> &VideoSnapshot {
        &self.snapshot
    }
    pub(crate) fn frame(&self) -> Option<gpui::SurfaceSource> {
        None
    }
    pub(crate) fn toggle_playback(&mut self) {}
    pub(crate) fn pause(&mut self) {}
    pub(crate) fn seek(&mut self, _: f64) {}
    pub(crate) fn set_volume(&mut self, _: f32) {}
    pub(crate) fn toggle_mute(&mut self) {}
    pub(crate) fn dispose(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::bounded_seek;

    #[test]
    fn seeks_reject_nonfinite_values_and_stay_in_bounds() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(bounded_seek(value, Some(12.0)), None);
        }
        assert_eq!(bounded_seek(-2.0, Some(12.0)), Some(0.0));
        assert_eq!(bounded_seek(50.0, Some(12.0)), Some(12.0));
        assert_eq!(bounded_seek(5.5, Some(12.0)), Some(5.5));
        assert_eq!(bounded_seek(5.5, None), Some(5.5));
    }
}
