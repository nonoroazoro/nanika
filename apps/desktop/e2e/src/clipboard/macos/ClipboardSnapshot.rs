use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::NSArray;

pub(crate) struct ClipboardSnapshot {
    _items: Retained<NSArray<ProtocolObject<dyn NSPasteboardWriting>>>
}

impl ClipboardSnapshot {
    pub(crate) fn capture() -> Result<Self, String> {
        let pasteboard = NSPasteboard::generalPasteboard();
        let revision = pasteboard.changeCount();
        let mut items = Vec::new();
        let mut bytes = 0_usize;
        if let Some(originals) = pasteboard.pasteboardItems() {
            for original in originals {
                let item = NSPasteboardItem::new();
                for format in original.types() {
                    // Materialize promised data before replacing its owner, preserving every item and type.
                    let data = original
                        .dataForType(&format)
                        .ok_or("Cannot safely snapshot a promised clipboard format")?;
                    bytes = bytes.saturating_add(data.length());
                    if bytes > super::MAX_SNAPSHOT_BYTES {
                        return Err(
                            "Clipboard snapshot exceeds 64 MiB; clipboard was not changed.".into()
                        );
                    }
                    if !item.setData_forType(&data, &format) {
                        return Err("Could not prepare clipboard restoration".into());
                    }
                }
                items.push(ProtocolObject::<dyn NSPasteboardWriting>::from_retained(
                    item
                ));
            }
        }
        if pasteboard.changeCount() != revision {
            return Err("Clipboard changed while taking its snapshot".into());
        }
        Ok(Self {
            _items: NSArray::from_retained_slice(&items)
        })
    }

    pub(crate) fn restore(self) -> Result<(), String> {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();
        if !self._items.is_empty() && !pasteboard.writeObjects(&self._items) {
            return Err("Could not restore the original clipboard".into());
        }
        Ok(())
    }
}
