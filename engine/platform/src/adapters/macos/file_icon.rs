use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSCompositingOperation, NSGraphicsContext, NSImageInterpolation, NSWorkspace};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGImageAlphaInfo, CGImageByteOrderInfo, kCGColorSpaceSRGB,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::path::Path;

pub(crate) fn pixels(path: &Path, _icon_index: i32, size: u32) -> std::io::Result<Vec<u8>> {
    autoreleasepool(|_| workspace_pixels(path, size as usize))
}

fn workspace_pixels(bundle: &Path, size: usize) -> std::io::Result<Vec<u8>> {
    let image =
        NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(&bundle.to_string_lossy()));
    let color_space = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }))
        .ok_or_else(|| std::io::Error::other("could not create the sRGB icon color space"))?;
    let mut pixels = vec![0_u8; size * size * 4];
    {
        // The initialized RGBA buffer outlives both drawing contexts and is not accessed while borrowed by the native renderer.
        let context = unsafe {
            CGBitmapContextCreate(
                pixels.as_mut_ptr().cast(),
                size,
                size,
                8,
                size * 4,
                Some(&color_space),
                CGImageAlphaInfo::PremultipliedLast.0 | CGImageByteOrderInfo::Order32Big.0,
            )
        }
        .ok_or_else(|| std::io::Error::other("could not create the system icon bitmap context"))?;
        let graphics = NSGraphicsContext::graphicsContextWithCGContext_flipped(&context, false);
        NSGraphicsContext::saveGraphicsState_class();
        let _restore = RestoreGraphicsState;
        NSGraphicsContext::setCurrentContext(Some(&graphics));
        graphics.setImageInterpolation(NSImageInterpolation::High);
        let rectangle = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(size as f64, size as f64),
        );
        // No hints are supplied, so there are no untyped Objective-C dictionary values to validate.
        unsafe {
            image.drawInRect_fromRect_operation_fraction_respectFlipped_hints(
                rectangle,
                NSRect::ZERO,
                NSCompositingOperation::Copy,
                1.0,
                false,
                None,
            );
        }
    }
    // Native drawing produces premultiplied channels; normalization and PNG encoding require straight RGBA.
    unpremultiply(&mut pixels);
    Ok(pixels)
}

struct RestoreGraphicsState;

impl Drop for RestoreGraphicsState {
    fn drop(&mut self) {
        NSGraphicsContext::restoreGraphicsState_class();
    }
}

fn unpremultiply(pixels: &mut [u8]) {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel = (u32::from(*channel) * 255 + alpha / 2)
                .checked_div(alpha)
                .unwrap_or(0)
                .min(255) as u8;
        }
    }
}

pub(crate) fn stamp(metadata: &std::fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec()
    )
}

#[cfg(test)]
#[path = "../../../tests/adapters/macos/file_icon.rs"]
mod tests;
