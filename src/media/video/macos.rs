use std::{
    ffi::CString,
    os::unix::ffi::OsStrExt,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, TryRecvError},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use block2::RcBlock;
use core_foundation::base::TCFType;
use core_video::pixel_buffer::CVPixelBuffer;
use gpui::SurfaceSource;
use objc2::{rc::Retained, AnyThread, MainThreadMarker};
use objc2_av_foundation::{
    AVPlayer, AVPlayerActionAtItemEnd, AVPlayerItem, AVPlayerItemStatus, AVPlayerItemVideoOutput,
    AVPlayerStatus, AVPlayerTimeControlStatus, AVURLAsset,
};
use objc2_core_media::{kCMTimeZero, CMTime};
use objc2_foundation::NSURL;

use super::{
    bounded_seek,
    composition::{self, PreparedComposition},
    frame, PlaybackState, VideoSnapshot,
};

const LOAD_TIMEOUT: Duration = Duration::from_secs(15);
const FRAME_TIMEOUT: Duration = Duration::from_secs(10);

struct NativePlayer {
    asset: Retained<AVURLAsset>,
    player: Retained<AVPlayer>,
    item: Retained<AVPlayerItem>,
    output: Retained<AVPlayerItemVideoOutput>,
    frame_validator: frame::Validator,
}

struct PendingSeek {
    id: u64,
    target: f64,
    completed: bool,
}

#[derive(Clone, Copy)]
struct SeekResult {
    id: u64,
    finished: bool,
}

/// Main-thread-only AVPlayer with one retained frame and no native UI.
pub(crate) struct VideoPlayer {
    native: Option<NativePlayer>,
    snapshot: VideoSnapshot,
    frame: Option<CVPixelBuffer>,
    composition: Option<Receiver<Result<PreparedComposition, String>>>,
    lifecycle: Arc<AtomicU64>,
    seek_result: Arc<Mutex<Option<SeekResult>>>,
    seek_id: u64,
    pending_seek: Option<PendingSeek>,
    queued_seek: Option<f64>,
    wants_playback: bool,
    ready: bool,
    load_deadline: Instant,
    frame_deadline: Option<Instant>,
}

impl VideoPlayer {
    /// Opens asynchronously and initially shows a paused frame without playing audio.
    pub(crate) fn new(path: &Path) -> Result<Self, String> {
        let mtm =
            MainThreadMarker::new().ok_or("Video playback must be created on the main thread.")?;
        let path = std::path::absolute(path).map_err(|e| format!("Could not open video: {e}"))?;
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| "The video path contains an invalid character.")?;
        let lifecycle = Arc::new(AtomicU64::new(0));
        let frame_validator = frame::Validator::new()?;
        // SAFETY: MainThreadMarker guards all player/item creation and their
        // retained types keep this backend !Send. The URL points to a local path;
        // AVFoundation owns media loading, decoding, composition and audio.
        let (native, composition) = unsafe {
            let url = NSURL::fileURLWithFileSystemRepresentation_isDirectory_relativeToURL(
                std::ptr::NonNull::new_unchecked(path.as_ptr().cast_mut()),
                false,
                None,
            );
            let asset = AVURLAsset::URLAssetWithURL_options(&url, None);
            let item = AVPlayerItem::playerItemWithAsset(&asset, mtm);
            let output = AVPlayerItemVideoOutput::initWithPixelBufferAttributes(
                AVPlayerItemVideoOutput::alloc(),
                Some(&frame::attributes()),
            );
            // Suppression affects this video output; AVPlayer still renders audio.
            output.setSuppressesPlayerRendering(true);
            item.addOutput(&output);
            item.setSeekingWaitsForVideoCompositionRendering(true);
            let player = AVPlayer::playerWithPlayerItem(None, mtm);
            player.setActionAtItemEnd(AVPlayerActionAtItemEnd::Pause);
            let composition = composition::prepare(&asset, lifecycle.clone());
            (
                NativePlayer {
                    asset,
                    player,
                    item,
                    output,
                    frame_validator,
                },
                composition,
            )
        };
        Ok(Self {
            native: Some(native),
            snapshot: VideoSnapshot::default(),
            frame: None,
            composition: Some(composition),
            lifecycle,
            seek_result: Arc::new(Mutex::new(None)),
            seek_id: 0,
            pending_seek: None,
            queued_seek: None,
            wants_playback: false,
            ready: false,
            load_deadline: Instant::now() + LOAD_TIMEOUT,
            frame_deadline: None,
        })
    }

    /// True only when the snapshot or displayed frame changes. Idle status polls
    /// are safe; the gallery can run those less often without requesting redraws.
    pub(crate) fn poll(&mut self) -> bool {
        if self.native.is_none() {
            return false;
        }
        let before = self.snapshot.clone();
        let frame_changed = self.poll_native();
        frame_changed || before != self.snapshot
    }

    pub(crate) fn needs_poll(&self) -> bool {
        self.native.is_some()
            && (matches!(
                self.snapshot.state,
                PlaybackState::Loading | PlaybackState::Playing | PlaybackState::Waiting
            ) || self.pending_seek.is_some()
                || self.frame_deadline.is_some())
    }

    pub(crate) fn snapshot(&self) -> &VideoSnapshot {
        &self.snapshot
    }

    /// Clones the current retained buffer; this does no decoding or conversion.
    pub(crate) fn frame(&self) -> Option<SurfaceSource> {
        self.frame.clone().map(SurfaceSource::from)
    }

    pub(crate) fn toggle_playback(&mut self) {
        if self.native.is_none() {
            return;
        }
        if self.wants_playback {
            self.pause();
            return;
        }
        self.wants_playback = true;
        if self.snapshot.state == PlaybackState::Ended {
            self.seek(0.0);
        } else if self.ready && self.pending_seek.is_none() {
            // SAFETY: NativePlayer is confined to this main-thread backend.
            unsafe {
                self.native.as_ref().unwrap().player.play();
            }
        }
        if self.ready && self.frame.is_some() {
            self.snapshot.state = PlaybackState::Waiting;
        }
    }

    pub(crate) fn pause(&mut self) {
        self.wants_playback = false;
        if let Some(native) = &self.native {
            // SAFETY: NativePlayer is confined to this main-thread backend.
            unsafe {
                native.player.pause();
            }
            if self.ready && self.frame.is_some() && self.snapshot.state != PlaybackState::Ended {
                self.snapshot.state = PlaybackState::Paused;
            }
        }
    }

    /// Latest seek wins. Callbacks report completion only; they never resume audio.
    pub(crate) fn seek(&mut self, seconds: f64) {
        let Some(target) = bounded_seek(seconds, self.snapshot.duration) else {
            return;
        };
        if self.native.is_none() {
            return;
        }
        self.snapshot.position = target;
        if !self.ready {
            self.queued_seek = Some(target);
            return;
        }
        self.start_seek(target);
    }

    pub(crate) fn set_volume(&mut self, volume: f32) {
        if !volume.is_finite() {
            return;
        }
        if let Some(native) = &self.native {
            self.snapshot.volume = volume.clamp(0.0, 1.0);
            // SAFETY: Finite AVPlayer volume in its supported [0, 1] range.
            unsafe {
                native.player.setVolume(self.snapshot.volume);
            }
        }
    }

    pub(crate) fn toggle_mute(&mut self) {
        if let Some(native) = &self.native {
            self.snapshot.muted = !self.snapshot.muted;
            // SAFETY: NativePlayer is confined to this main-thread backend.
            unsafe {
                native.player.setMuted(self.snapshot.muted);
            }
        }
    }

    /// Synchronously stops audio and detaches native work. Safe to call repeatedly.
    pub(crate) fn dispose(&mut self) {
        self.lifecycle.fetch_add(1, Ordering::AcqRel);
        if let Some(native) = self.native.take() {
            // SAFETY: Drop and every method run on the owning main thread. No
            // observer/delegate was installed; abandoned callbacks own only their
            // Send-safe mailboxes and are rejected by the lifecycle generation.
            unsafe {
                native.player.pause();
                native.item.cancelPendingSeeks();
                native.player.cancelPendingPrerolls();
                native.asset.cancelLoading();
                native.item.removeOutput(&native.output);
                native.player.replaceCurrentItemWithPlayerItem(None);
                native.item.setVideoComposition(None);
            }
        }
        self.composition = None;
        self.pending_seek = None;
        self.queued_seek = None;
        self.frame_deadline = None;
        self.frame = None;
        self.wants_playback = false;
        self.ready = false;
        if self.snapshot.state != PlaybackState::Failed {
            self.snapshot.state = PlaybackState::Paused;
        }
    }

    fn fail(&mut self, message: String) {
        self.snapshot.state = PlaybackState::Failed;
        self.snapshot.error = Some(message);
        self.dispose();
    }

    fn start_seek(&mut self, target: f64) {
        let native = self.native.as_ref().unwrap();
        self.seek_id += 1;
        let id = self.seek_id;
        let result = self.seek_result.clone();
        let lifecycle = self.lifecycle.clone();
        let expected_generation = lifecycle.load(Ordering::Acquire);
        let completion = move |finished: objc2::runtime::Bool| {
            if lifecycle.load(Ordering::Acquire) != expected_generation {
                return;
            }
            if let Ok(mut result) = result.lock() {
                if result.is_none_or(|older| older.id <= id) {
                    *result = Some(SeekResult {
                        id,
                        finished: finished.as_bool(),
                    });
                }
            }
        };
        fn require_send<T: Send>(_: &T) {}
        require_send(&completion);
        let completion = RcBlock::new(completion);
        // Ask for the final frame just before the duration rather than an absent
        // frame at the exclusive end of the video's timeline.
        let decode_target = self
            .snapshot
            .duration
            .filter(|d| target >= *d)
            .map_or(target, |end| (end - 1.0 / 600.0).max(0.0));
        // SAFETY: Finite bounded time; zero tolerances request the actual seek
        // frame. The callback is Send and never captures a native/GPUI object.
        unsafe {
            native.player.pause();
            native.item.cancelPendingSeeks();
            native
                .player
                .seekToTime_toleranceBefore_toleranceAfter_completionHandler(
                    CMTime::with_seconds(decode_target, 600),
                    kCMTimeZero,
                    kCMTimeZero,
                    &completion,
                );
        }
        self.pending_seek = Some(PendingSeek {
            id,
            target,
            completed: false,
        });
        self.frame_deadline = Some(Instant::now() + FRAME_TIMEOUT);
        self.snapshot.state = if self.frame.is_none() {
            PlaybackState::Loading
        } else if self.wants_playback {
            PlaybackState::Waiting
        } else {
            PlaybackState::Paused
        };
    }

    fn poll_native(&mut self) -> bool {
        if let Some(receiver) = &self.composition {
            match receiver.try_recv() {
                Ok(Ok(prepared)) => {
                    let composition = prepared.build();
                    let native = self.native.as_ref().unwrap();
                    // SAFETY: Attaching a composition and item is main-thread-only.
                    unsafe {
                        native.item.setVideoComposition(Some(&composition));
                        native
                            .player
                            .replaceCurrentItemWithPlayerItem(Some(&native.item));
                    }
                    self.composition = None;
                }
                Ok(Err(error)) => {
                    self.fail(error);
                    return false;
                }
                Err(TryRecvError::Disconnected) => {
                    self.fail("Video preparation was interrupted.".into());
                    return false;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        // SAFETY: All these status reads and output acquisition happen on the
        // owning main thread; callback results are synchronized through mailboxes.
        unsafe {
            let native = self.native.as_ref().unwrap();
            if native.item.status() == AVPlayerItemStatus::Failed
                || native.player.status() == AVPlayerStatus::Failed
            {
                let error = native
                    .item
                    .error()
                    .or_else(|| native.player.error())
                    .map(|e| e.localizedDescription().to_string())
                    .unwrap_or_else(|| "AVFoundation could not play this video.".into());
                self.fail(error);
                return false;
            }
            if !self.ready {
                if self.composition.is_some()
                    || native.item.status() != AVPlayerItemStatus::ReadyToPlay
                {
                    if Instant::now() >= self.load_deadline {
                        self.fail("Timed out loading this video.".into());
                    }
                    return false;
                }
                self.snapshot.duration =
                    finite_seconds(native.item.duration()).filter(|d| *d > 0.0);
                self.ready = true;
                let target = self.queued_seek.take().unwrap_or(0.0);
                let target = bounded_seek(target, self.snapshot.duration).unwrap_or(0.0);
                self.snapshot.position = target;
                self.start_seek(target);
            }
            let completion = self
                .seek_result
                .lock()
                .ok()
                .and_then(|mut result| result.take());
            if let (Some(pending), Some(completion)) = (&mut self.pending_seek, completion) {
                if pending.id == completion.id {
                    if !completion.finished {
                        self.fail("The video could not seek to the requested position.".into());
                        return false;
                    }
                    pending.completed = true;
                }
            }
            let native = self.native.as_ref().unwrap();
            let time = native.player.currentTime();
            let mut changed = false;
            let can_acquire = self.pending_seek.as_ref().is_none_or(|seek| seek.completed);
            // A seek to the same encoded frame need not report "new" output.
            // Once its native completion arrives, request the appropriate frame
            // directly instead of waiting forever for that optimization signal.
            if can_acquire
                && (self.pending_seek.is_some() || native.output.hasNewPixelBufferForItemTime(time))
            {
                if let Some(buffer) = native
                    .output
                    .copyPixelBufferForItemTime_itemTimeForDisplay(time, std::ptr::null_mut())
                {
                    // Both bindings represent CVPixelBufferRef. Retain using the
                    // Get rule, then let objc2's returned +1 reference drop.
                    let frame = CVPixelBuffer::wrap_under_get_rule(
                        Retained::as_ptr(&buffer).cast_mut().cast(),
                    );
                    if let Err(error) = native.frame_validator.validate(&frame) {
                        self.fail(error);
                        return false;
                    }
                    changed = self.frame.as_ref() != Some(&frame);
                    self.frame = Some(frame);
                    self.frame_deadline = None;
                    if let Some(seek) = self.pending_seek.take() {
                        self.snapshot.position = seek.target;
                        if self.snapshot.duration.is_some_and(|end| seek.target >= end) {
                            self.wants_playback = false;
                            self.snapshot.state = PlaybackState::Ended;
                        } else if self.wants_playback {
                            native.player.play();
                        }
                    }
                }
            }
            if self
                .frame_deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                self.fail("Timed out waiting for a displayable video frame.".into());
                return changed;
            }
            if self.pending_seek.is_none() {
                if let Some(position) =
                    finite_seconds(time).filter(|_| self.snapshot.state != PlaybackState::Ended)
                {
                    self.snapshot.position =
                        bounded_seek(position, self.snapshot.duration).unwrap_or(0.0);
                }
                let at_end = self
                    .snapshot
                    .duration
                    .is_some_and(|duration| self.snapshot.position >= duration - 1.0 / 600.0);
                if at_end
                    && self.wants_playback
                    && native.player.timeControlStatus() == AVPlayerTimeControlStatus::Paused
                {
                    self.wants_playback = false;
                    self.snapshot.position = self.snapshot.duration.unwrap();
                    self.snapshot.state = PlaybackState::Ended;
                } else if self.snapshot.state != PlaybackState::Ended {
                    self.snapshot.state = if !self.wants_playback {
                        PlaybackState::Paused
                    } else if native.player.timeControlStatus()
                        == AVPlayerTimeControlStatus::Playing
                    {
                        PlaybackState::Playing
                    } else {
                        PlaybackState::Waiting
                    };
                }
            }
            changed
        }
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.dispose();
    }
}

fn finite_seconds(time: CMTime) -> Option<f64> {
    // SAFETY: CMTime is a value type returned by AVFoundation.
    let seconds = unsafe { time.seconds() };
    (seconds.is_finite() && seconds >= 0.0).then_some(seconds)
}
