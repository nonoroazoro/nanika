use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct QueryInterrupt<'a> {
    cancelled: &'a AtomicBool,
}

impl<'a> QueryInterrupt<'a> {
    pub(crate) fn new(cancelled: &'a AtomicBool) -> Self {
        Self { cancelled }
    }
}

impl fend_core::Interrupt for QueryInterrupt<'_> {
    fn should_interrupt(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}
