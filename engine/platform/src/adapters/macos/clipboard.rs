use crate::PreparedClipboardContent;
use objc2::{rc::autoreleasepool, runtime::ProtocolObject};
use objc2_app_kit::{
    NSPasteboard, NSPasteboardItem, NSPasteboardTypePNG, NSPasteboardTypeString,
    NSPasteboardWriting
};
use objc2_foundation::{NSArray, NSData, NSString, NSURL};

pub(crate) fn write(content: PreparedClipboardContent) -> Result<u64, String> {
    autoreleasepool(|_| {
        let objects = match content {
            PreparedClipboardContent::Text(value) => {
                let item = NSPasteboardItem::new();
                if !item.setString_forType(&NSString::from_str(&value), unsafe {
                    NSPasteboardTypeString
                }) {
                    return Err("Could not prepare clipboard text.".into());
                }
                NSArray::from_retained_slice(&[
                    ProtocolObject::<dyn NSPasteboardWriting>::from_retained(item)
                ])
            }
            PreparedClipboardContent::Png(bytes) => {
                let item = NSPasteboardItem::new();
                let data = NSData::with_bytes(&bytes);
                if !item.setData_forType(&data, unsafe { NSPasteboardTypePNG }) {
                    return Err("Could not prepare clipboard PNG.".into());
                }
                NSArray::from_retained_slice(&[
                    ProtocolObject::<dyn NSPasteboardWriting>::from_retained(item)
                ])
            }
            PreparedClipboardContent::Files(paths) => {
                let mut urls = Vec::with_capacity(paths.len());
                for path in paths {
                    if !std::path::Path::new(&path).exists() {
                        return Err(format!("Clipboard file does not exist: {path}"));
                    }
                    urls.push(ProtocolObject::<dyn NSPasteboardWriting>::from_retained(
                        NSURL::fileURLWithPath(&NSString::from_str(&path))
                    ));
                }
                NSArray::from_retained_slice(&urls)
            }
        };
        let pasteboard = NSPasteboard::generalPasteboard();
        let revision = pasteboard.clearContents();
        if !pasteboard.writeObjects(&objects) {
            return Err("The pasteboard rejected the write.".into());
        }
        if pasteboard.changeCount() != revision {
            return Err("Clipboard ownership changed during the write.".into());
        }
        Ok(revision as u64)
    })
}
