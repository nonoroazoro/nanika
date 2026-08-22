use std::sync::mpsc::SyncSender;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_foundation::MainThreadMarker;

use crate::{MacMenuTargetIvars, PlatformEvent};

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = MacMenuTargetIvars]
    pub(crate) struct MacMenuTarget;

    impl MacMenuTarget {
        #[unsafe(method(open:))]
        fn open(&self, _sender: Option<&AnyObject>) {
            let _ = self.ivars().events.try_send(PlatformEvent::Open);
        }

        #[unsafe(method(settings:))]
        fn settings(&self, _sender: Option<&AnyObject>) {
            let _ = self.ivars().events.try_send(PlatformEvent::Settings);
        }

        #[unsafe(method(rescanApplications:))]
        fn rescan_applications(&self, _sender: Option<&AnyObject>) {
            let _ = self
                .ivars()
                .events
                .try_send(PlatformEvent::RescanApplications);
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            let _ = self.ivars().events.try_send(PlatformEvent::Quit);
        }
    }
);

impl MacMenuTarget {
    pub(crate) fn new(mtm: MainThreadMarker, events: SyncSender<PlatformEvent>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(MacMenuTargetIvars { events });
        unsafe { msg_send![super(this), init] }
    }
}
