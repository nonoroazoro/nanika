use std::time::Duration;

/// Fixed bounds applied to one extension process.
#[derive(Debug, Clone)]
pub struct ExtensionLimits {
    pub handshake_timeout: Duration,
    pub action_timeout: Duration,
    pub settings_timeout: Duration,
    pub shutdown_timeout: Duration,
    pub frame_queue_capacity: usize,
    pub stderr_tail_bytes: usize,
    pub max_restarts: u32,
}

impl Default for ExtensionLimits {
    fn default() -> Self {
        Self {
            handshake_timeout: Duration::from_secs(2),
            action_timeout: Duration::from_secs(10),
            settings_timeout: Duration::from_secs(5),
            shutdown_timeout: Duration::from_secs(2),
            frame_queue_capacity: 1,
            stderr_tail_bytes: 64 * 1024,
            max_restarts: 3,
        }
    }
}
