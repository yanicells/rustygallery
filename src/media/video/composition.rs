//! Asynchronous native orientation setup, with a Send-safe result mailbox.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc::{sync_channel, Receiver},
    Arc,
};

use block2::RcBlock;
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_av_foundation::{
    AVAsset, AVMutableVideoComposition, AVVideoColorPrimaries_ITU_R_709_2, AVVideoComposition,
    AVVideoCompositionInstruction, AVVideoTransferFunction_ITU_R_709_2,
    AVVideoYCbCrMatrix_ITU_R_601_4,
};
use objc2_core_foundation::CGSize;
use objc2_core_media::CMTime;
use objc2_foundation::{NSArray, NSCopying, NSError};

pub(super) struct PreparedComposition {
    size: CGSize,
    frame_duration: CMTime,
    timing_track: i32,
    instructions: Vec<Retained<AVVideoCompositionInstruction>>,
}

impl PreparedComposition {
    /// Rebuild on the UI thread; only immutable instructions cross the mailbox.
    pub(super) fn build(self) -> Retained<AVMutableVideoComposition> {
        // SAFETY: The native factory supplied these geometry/timing/instruction
        // values. The mutable composition is used solely on the UI thread.
        unsafe {
            let composition = AVMutableVideoComposition::videoComposition();
            composition.setRenderSize(self.size);
            composition.setFrameDuration(self.frame_duration);
            composition.setSourceTrackIDForFrameTiming(self.timing_track);
            let instructions: Vec<_> = self
                .instructions
                .into_iter()
                .map(ProtocolObject::from_retained)
                .collect();
            composition.setInstructions(&NSArray::from_retained_slice(&instructions));
            // GPUI's Surface shader expects full-range BT.601 YCbCr and SDR.
            composition.setColorPrimaries(AVVideoColorPrimaries_ITU_R_709_2);
            composition.setColorTransferFunction(AVVideoTransferFunction_ITU_R_709_2);
            composition.setColorYCbCrMatrix(AVVideoYCbCrMatrix_ITU_R_601_4);
            composition
        }
    }
}

pub(super) fn prepare(
    asset: &AVAsset,
    generation: Arc<AtomicU64>,
) -> Receiver<Result<PreparedComposition, String>> {
    let (sender, receiver) = sync_channel(1);
    let expected_generation = generation.load(Ordering::Acquire);
    let completion = move |composition: *mut AVVideoComposition, error: *mut NSError| {
        if generation.load(Ordering::Acquire) != expected_generation {
            return;
        }
        // SAFETY: AVFoundation lends both callback arguments for this invocation.
        // We borrow them here and send copied immutable instructions and values;
        // no AVAsset, mutable composition, player, or GPUI object is captured.
        let result = unsafe {
            match composition.as_ref() {
                None => Err(error
                    .as_ref()
                    .map(|e| e.localizedDescription().to_string())
                    .unwrap_or_else(|| "Could not prepare this video's orientation.".into())),
                Some(composition) => extract(composition),
            }
        };
        let _ = sender.try_send(result);
    };
    // Enforce the native callback's sendability despite the untyped ObjC block API.
    fn require_send<T: Send>(_: &T) {}
    require_send(&completion);
    let completion = RcBlock::new(completion);
    // SAFETY: The callback captures only Send values. This API loads asynchronously
    // on macOS 13+, avoiding synchronous asset-property reads on the UI thread.
    unsafe {
        AVVideoComposition::videoCompositionWithPropertiesOfAsset_completionHandler(
            asset,
            &completion,
        );
    }
    receiver
}

unsafe fn extract(composition: &AVVideoComposition) -> Result<PreparedComposition, String> {
    let size = composition.renderSize();
    if !size.width.is_finite()
        || !size.height.is_finite()
        || size.width <= 0.0
        || size.height <= 0.0
    {
        return Err("This file has no displayable video track.".into());
    }
    let mut instructions = Vec::new();
    for instruction in composition.instructions().iter() {
        let object: &objc2::runtime::AnyObject = instruction.as_ref();
        let instruction = object
            .downcast_ref::<AVVideoCompositionInstruction>()
            .ok_or("This video requires an unsupported native composition.")?;
        // The immutable copy type has upstream Send + Sync guarantees.
        instructions.push(instruction.copy());
    }
    if instructions.is_empty() {
        return Err("This file has no displayable video track.".into());
    }
    Ok(PreparedComposition {
        size,
        frame_duration: composition.frameDuration(),
        timing_track: composition.sourceTrackIDForFrameTiming(),
        instructions,
    })
}
