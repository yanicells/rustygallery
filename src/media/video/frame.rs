//! The exact pixel-buffer contract required by GPUI's Metal Surface renderer.

use core_foundation::{
    base::TCFType,
    string::{CFString, CFStringRef},
};
use core_video::{
    buffer::TCVBuffer,
    image_buffer::{
        kCVImageBufferColorPrimariesKey, kCVImageBufferColorPrimaries_ITU_R_709_2,
        kCVImageBufferTransferFunctionKey, kCVImageBufferTransferFunction_ITU_R_709_2,
        kCVImageBufferYCbCrMatrixKey, kCVImageBufferYCbCrMatrix_ITU_R_601_4,
    },
    metal_texture::CVMetalTextureGetTexture,
    metal_texture_cache::CVMetalTextureCache,
    pixel_buffer::{
        kCVPixelBufferIOSurfacePropertiesKey, kCVPixelBufferMetalCompatibilityKey,
        kCVPixelBufferPixelFormatTypeKey, kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
        CVPixelBuffer,
    },
    pixel_buffer_io_surface::CVPixelBufferGetIOSurface,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSDictionary, NSNumber, NSString};

pub(super) struct Validator {
    cache: CVMetalTextureCache,
}

impl Validator {
    pub(super) fn new() -> Result<Self, String> {
        // Match GPUI's preference for an internal, high-power Metal device.
        let device = metal::Device::all()
            .into_iter()
            .min_by_key(|device| (device.is_removable(), device.is_low_power()))
            .ok_or("No Metal device is available for in-app video.")?;
        let cache = CVMetalTextureCache::new(None, device, None)
            .map_err(|code| format!("Could not prepare video surfaces (CoreVideo {code})."))?;
        Ok(Self { cache })
    }

    pub(super) fn validate(&self, frame: &CVPixelBuffer) -> Result<(), String> {
        validate(frame)?;
        // Preflight the two exact texture views GPUI creates with unwrap(). No
        // pixels are copied, converted, or uploaded. Failed mapping becomes an
        // in-window error before an incompatible buffer can reach the renderer.
        let result = (|| {
            for (plane, format) in [
                (0, metal::MTLPixelFormat::R8Unorm),
                (1, metal::MTLPixelFormat::RG8Unorm),
            ] {
                let texture = self
                    .cache
                    .create_texture_from_image(
                        frame.as_concrete_TypeRef(),
                        None,
                        format,
                        frame.get_width_of_plane(plane),
                        frame.get_height_of_plane(plane),
                        plane,
                    )
                    .map_err(|code| {
                        format!("The video surface cannot be displayed (CoreVideo {code}).")
                    })?;
                // SAFETY: This is a borrowed Get reference, valid for the texture
                // wrapper's lifetime. Inspect it without taking ownership.
                if unsafe { CVMetalTextureGetTexture(texture.as_concrete_TypeRef()).is_null() } {
                    return Err("The video surface could not be mapped to a Metal texture.".into());
                }
            }
            Ok(())
        })();
        // Release unused preflight views rather than accumulating decoded frames.
        self.cache.flush(0);
        result
    }
}

pub(super) fn attributes() -> Retained<NSDictionary<NSString, AnyObject>> {
    // SAFETY: CoreVideo exports immortal CFString constants. Values follow the
    // documented pixelBufferAttributes types, including an empty IOSurface dict.
    unsafe {
        let keys = [
            kCVPixelBufferPixelFormatTypeKey,
            kCVPixelBufferMetalCompatibilityKey,
            kCVPixelBufferIOSurfacePropertiesKey,
            kCVImageBufferColorPrimariesKey,
            kCVImageBufferTransferFunctionKey,
            kCVImageBufferYCbCrMatrixKey,
        ]
        .map(|key| ns_string(key));
        let format = NSNumber::new_u32(kCVPixelFormatType_420YpCbCr8BiPlanarFullRange);
        let metal = NSNumber::new_bool(true);
        let surface = NSDictionary::<NSString, AnyObject>::new();
        let primaries = ns_string(kCVImageBufferColorPrimaries_ITU_R_709_2);
        let transfer = ns_string(kCVImageBufferTransferFunction_ITU_R_709_2);
        let matrix = ns_string(kCVImageBufferYCbCrMatrix_ITU_R_601_4);
        NSDictionary::from_slices(
            &keys.iter().map(|k| &**k).collect::<Vec<_>>(),
            &[
                format.as_ref(),
                metal.as_ref(),
                surface.as_ref(),
                primaries.as_ref(),
                transfer.as_ref(),
                matrix.as_ref(),
            ],
        )
    }
}

unsafe fn ns_string(value: CFStringRef) -> Retained<NSString> {
    NSString::from_str(&CFString::wrap_under_get_rule(value).to_string())
}

fn validate(frame: &CVPixelBuffer) -> Result<(), String> {
    validate_layout(
        frame.get_pixel_format(),
        (frame.get_width(), frame.get_height()),
        (0..frame.get_plane_count())
            .map(|plane| {
                (
                    frame.get_width_of_plane(plane),
                    frame.get_height_of_plane(plane),
                )
            })
            .collect::<Vec<_>>()
            .as_slice(),
    )?;
    // SAFETY: This Get call returns a borrowed IOSurface. Do not release it.
    if unsafe { CVPixelBufferGetIOSurface(frame.as_concrete_TypeRef()).is_null() } {
        return Err("The video decoder did not produce a Metal-compatible surface.".into());
    }
    let buffer = frame.as_buffer();
    // Reject mismatched or absent color tags rather than showing incorrectly
    // converted HDR/wide-gamut frames through GPUI's fixed SDR shader.
    unsafe {
        for (key, expected) in [
            (
                kCVImageBufferYCbCrMatrixKey,
                kCVImageBufferYCbCrMatrix_ITU_R_601_4,
            ),
            (
                kCVImageBufferColorPrimariesKey,
                kCVImageBufferColorPrimaries_ITU_R_709_2,
            ),
            (
                kCVImageBufferTransferFunctionKey,
                kCVImageBufferTransferFunction_ITU_R_709_2,
            ),
        ] {
            let key = CFString::wrap_under_get_rule(key);
            let expected = CFString::wrap_under_get_rule(expected);
            let matches = buffer
                .get_attachment(&key, None)
                .and_then(|value| value.downcast::<CFString>())
                .is_some_and(|value| value == expected);
            if !matches {
                return Err("The decoder could not provide compatible SDR video colors.".into());
            }
        }
    }
    Ok(())
}

fn validate_layout(
    format: u32,
    size: (usize, usize),
    planes: &[(usize, usize)],
) -> Result<(), String> {
    let (width, height) = size;
    if format != kCVPixelFormatType_420YpCbCr8BiPlanarFullRange
        || width == 0
        || height == 0
        || planes.len() != 2
        || planes[0] != size
        || planes[1] != (width.div_ceil(2), height.div_ceil(2))
    {
        return Err("The decoder returned a video format this renderer cannot display.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_buffers_that_would_violate_the_surface_renderer_contract() {
        let format = kCVPixelFormatType_420YpCbCr8BiPlanarFullRange;
        assert!(validate_layout(format, (640, 360), &[(640, 360), (320, 180)]).is_ok());
        assert!(validate_layout(format, (0, 0), &[(0, 0), (0, 0)]).is_err());
        assert!(validate_layout(0, (640, 360), &[(640, 360), (320, 180)]).is_err());
        assert!(validate_layout(format, (640, 360), &[(640, 360)]).is_err());
        assert!(validate_layout(format, (640, 360), &[(640, 360), (640, 360)]).is_err());
    }
}
