use std::ptr::NonNull;
use std::sync::mpsc::SyncSender;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{
    NSCurrentLocaleDidChangeNotification, NSNotification, NSNotificationCenter, NSObjectProtocol
};

/// Observes native locale notifications without reading preferences on the notification thread.
pub struct LanguageChangeObserver {
    _center: Retained<NSNotificationCenter>,
    _observer: Retained<ProtocolObject<dyn NSObjectProtocol>>
}

// NotificationCenter registration/removal is thread-safe. This opaque token owns only
// a sendable channel callback, never an AppKit object or a reference to the posting thread.
unsafe impl Send for LanguageChangeObserver {}

impl LanguageChangeObserver {
    pub fn install(wakes: SyncSender<()>) -> Result<Self, String> {
        let center = NSNotificationCenter::defaultCenter();
        let callback = block2::RcBlock::new(move |_: NonNull<NSNotification>| {
            // A full slot already promises a fresh read. Never block Foundation delivery.
            let _ = wakes.try_send(());
        });
        let observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSCurrentLocaleDidChangeNotification),
                None,
                None,
                &callback
            )
        };
        Ok(Self {
            _center: center,
            _observer: observer
        })
    }
}

impl Drop for LanguageChangeObserver {
    fn drop(&mut self) {
        unsafe {
            self._center
                .removeObserver(AsRef::<objc2::runtime::AnyObject>::as_ref(&*self._observer));
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/adapters/macos/LanguageChangeObserver.rs"]
mod tests;
