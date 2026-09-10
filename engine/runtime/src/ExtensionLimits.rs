/// Hard transport resource bounds applied to one extension process.
#[derive(Debug, Clone)]
pub struct ExtensionLimits {
    pub frame_queue_capacity: usize,
    pub stderr_tail_bytes: usize,
}

impl Default for ExtensionLimits {
    fn default() -> Self {
        Self {
            frame_queue_capacity: 1,
            stderr_tail_bytes: 64 * 1024,
        }
    }
}
