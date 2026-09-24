use std::path::Path;

use objc2::rc::autoreleasepool;
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSArray, NSString, NSURL};

pub(crate) fn reveal(path: &Path) -> std::io::Result<()> {
    path.symlink_metadata()?;
    let path = path
        .to_str()
        .ok_or_else(|| std::io::Error::other("reveal path is not UTF-8"))?;
    // The process launcher is a Rust thread without an AppKit event-loop pool.
    autoreleasepool(|_| {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        NSWorkspace::sharedWorkspace()
            .activateFileViewerSelectingURLs(&NSArray::from_retained_slice(&[url]));
    });
    Ok(())
}
