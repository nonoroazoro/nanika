use block2::RcBlock;
use objc2::{AnyThread, rc::autoreleasepool};
use objc2_app_kit::{NSCompositingOperation, NSGraphicsContext, NSImageInterpolation, NSWorkspace};
use objc2_core_foundation::{CFBoolean, CFDictionary, CFNumber, CFType, CFURL};
use objc2_core_graphics::{
    CGBitmapContextCreate, CGColorSpace, CGContext, CGImage, CGImageAlphaInfo,
    CGImageByteOrderInfo, CGInterpolationQuality, kCGColorSpaceSRGB,
};
use objc2_foundation::{NSError, NSPoint, NSRect, NSSize, NSString, NSURL};
use objc2_image_io::{
    CGImageSource, kCGImageSourceCreateThumbnailFromImageAlways,
    kCGImageSourceCreateThumbnailWithTransform, kCGImageSourceShouldCache,
    kCGImageSourceThumbnailMaxPixelSize,
};
use objc2_quick_look_thumbnailing::{
    QLThumbnailGenerationRequest, QLThumbnailGenerationRequestRepresentationTypes,
    QLThumbnailGenerator, QLThumbnailRepresentation,
};
use std::path::Path;

pub(crate) fn pixels(path: &Path, _icon_index: i32, size: u32) -> std::io::Result<Vec<u8>> {
    autoreleasepool(|_| workspace_pixels(path, size as usize))
}

pub(crate) fn shell_pixels(path: &Path, size: u32) -> std::io::Result<Vec<u8>> {
    autoreleasepool(|_| {
        image_io_pixels(path, size as usize)
            .or_else(|_| quick_look_pixels(path, size as usize))
            .or_else(|_| workspace_pixels(path, size as usize))
    })
}

pub(crate) fn list_pixels(path: &Path, size: u32) -> std::io::Result<Vec<u8>> {
    autoreleasepool(|_| workspace_pixels(path, size as usize))
}

fn image_io_pixels(path: &Path, size: usize) -> std::io::Result<Vec<u8>> {
    let url = CFURL::from_file_path(path)
        .ok_or_else(|| std::io::Error::other("could not create the image file URL"))?;
    let source = unsafe { CGImageSource::with_url(&url, None) }
        .ok_or_else(|| std::io::Error::other("the file is not a supported image"))?;
    let max_size = CFNumber::new_isize(size as isize);
    let options = CFDictionary::<CFType, CFType>::from_slices(
        &[
            unsafe { kCGImageSourceCreateThumbnailFromImageAlways }.as_ref(),
            unsafe { kCGImageSourceThumbnailMaxPixelSize }.as_ref(),
            unsafe { kCGImageSourceCreateThumbnailWithTransform }.as_ref(),
            unsafe { kCGImageSourceShouldCache }.as_ref(),
        ],
        &[
            CFBoolean::new(true).as_ref(),
            max_size.as_ref(),
            CFBoolean::new(true).as_ref(),
            CFBoolean::new(false).as_ref(),
        ],
    );
    let image = unsafe { source.thumbnail_at_index(0, Some(options.as_ref())) }
        .ok_or_else(|| std::io::Error::other("Image I/O could not generate a thumbnail"))?;
    cg_image_pixels(&image, size)
}

fn quick_look_pixels(path: &Path, size: usize) -> std::io::Result<Vec<u8>> {
    let source = NSString::from_str(&path.to_string_lossy());
    let url = NSURL::fileURLWithPath(&source);
    let request = unsafe {
        QLThumbnailGenerationRequest::initWithFileAtURL_size_scale_representationTypes(
            QLThumbnailGenerationRequest::alloc(),
            &url,
            NSSize::new(size as f64, size as f64),
            1.0,
            QLThumbnailGenerationRequestRepresentationTypes::Thumbnail,
        )
    };
    unsafe {
        request.setIconMode(false);
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let completion = RcBlock::new(
        move |representation: *mut QLThumbnailRepresentation, error: *mut NSError| {
            let result = autoreleasepool(|_| {
                let Some(representation) = (unsafe { representation.as_ref() }) else {
                    return Err(quick_look_error(error));
                };
                let image = unsafe { representation.CGImage() };
                cg_image_pixels(&image, size)
            });
            let _ = sender.send(result);
        },
    );
    let generator = unsafe { QLThumbnailGenerator::sharedGenerator() };
    unsafe {
        generator.generateBestRepresentationForRequest_completionHandler(&request, &completion);
    }
    receiver
        .recv()
        .map_err(|_| std::io::Error::other("Quick Look closed without returning a thumbnail"))?
}

fn quick_look_error(error: *mut NSError) -> std::io::Error {
    let message = unsafe { error.as_ref() }
        .map(|error| error.localizedDescription().to_string())
        .unwrap_or_else(|| "Quick Look did not provide a thumbnail".to_owned());
    std::io::Error::other(message)
}

fn cg_image_pixels(image: &CGImage, size: usize) -> std::io::Result<Vec<u8>> {
    let source_width = CGImage::width(Some(image));
    let source_height = CGImage::height(Some(image));
    if source_width == 0 || source_height == 0 {
        return Err(std::io::Error::other(
            "Quick Look returned an empty thumbnail",
        ));
    }
    let side = size as f64;
    let scale = (side / source_width as f64).min(side / source_height as f64);
    let width = source_width as f64 * scale;
    let height = source_height as f64 * scale;
    let rectangle = NSRect::new(
        NSPoint::new((side - width) / 2.0, (side - height) / 2.0),
        NSSize::new(width, height),
    );
    let color_space = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }))
        .ok_or_else(|| std::io::Error::other("could not create the sRGB thumbnail color space"))?;
    let mut pixels = vec![0_u8; size * size * 4];
    {
        // The initialized RGBA buffer outlives the bitmap context and is not accessed while borrowed by Core Graphics.
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
        .ok_or_else(|| std::io::Error::other("could not create the thumbnail bitmap context"))?;
        CGContext::set_interpolation_quality(Some(&context), CGInterpolationQuality::High);
        CGContext::translate_ctm(Some(&context), 0.0, side);
        CGContext::scale_ctm(Some(&context), 1.0, -1.0);
        CGContext::draw_image(Some(&context), rectangle, Some(image));
    }
    unpremultiply(&mut pixels);
    if pixels.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 0) {
        return Err(std::io::Error::other(
            "Quick Look returned a transparent thumbnail",
        ));
    }
    Ok(pixels)
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
