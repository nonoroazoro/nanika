use std::time::Instant;

/// Fend interrupt that bounds one preview evaluation.
pub(crate) struct DeadlineInterrupt {
    pub(crate) deadline: Instant,
}

impl fend_core::Interrupt for DeadlineInterrupt {
    fn should_interrupt(&self) -> bool {
        Instant::now() >= self.deadline
    }
}
