use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

static AVAILABLE: AtomicBool = AtomicBool::new(false);
static HOTKEY_ID: AtomicU32 = AtomicU32::new(0);
static DELIVERY_NANOS: AtomicU64 = AtomicU64::new(0);

pub(crate) fn record_hotkey_delivery(hotkey_id: u32, delivery: Duration) {
    HOTKEY_ID.store(hotkey_id, Ordering::Relaxed);
    DELIVERY_NANOS.store(
        delivery.as_nanos().min(u64::MAX as u128) as u64,
        Ordering::Relaxed,
    );
    AVAILABLE.store(true, Ordering::Release);
}

/// Consume native delivery timing for the next matching hotkey press.
pub fn take_hotkey_delivery_delay(hotkey_id: u32) -> Option<Duration> {
    if !AVAILABLE.load(Ordering::Acquire) || HOTKEY_ID.load(Ordering::Relaxed) != hotkey_id {
        return None;
    }
    if !AVAILABLE.swap(false, Ordering::Acquire) {
        return None;
    }
    Some(Duration::from_nanos(DELIVERY_NANOS.load(Ordering::Relaxed)))
}

/// Capture delivery latency while the native hotkey callback is active.
pub fn current_hotkey_delivery_delay(hotkey_id: u32) -> Option<Duration> {
    #[cfg(target_os = "macos")]
    {
        take_hotkey_delivery_delay(hotkey_id)
            .or_else(crate::hotkey_timing_macos::current_delivery_delay)
    }
    #[cfg(windows)]
    {
        take_hotkey_delivery_delay(hotkey_id)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = hotkey_id;
        None
    }
}
