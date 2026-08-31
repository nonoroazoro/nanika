use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

use global_hotkey::GlobalHotKeyEvent;

static AVAILABLE: AtomicBool = AtomicBool::new(false);
static HOTKEY_ID: AtomicU32 = AtomicU32::new(0);
static DELIVERY_NANOS: AtomicU64 = AtomicU64::new(0);
type HotkeyEventHandler = Arc<dyn Fn(GlobalHotKeyEvent, Option<Duration>) + Send + Sync + 'static>;
static HOTKEY_EVENT_HANDLER: OnceLock<RwLock<Option<HotkeyEventHandler>>> = OnceLock::new();

pub(crate) fn record_hotkey_delivery(hotkey_id: u32, delivery: Duration) {
    HOTKEY_ID.store(hotkey_id, Ordering::Relaxed);
    DELIVERY_NANOS.store(
        delivery.as_nanos().min(u64::MAX as u128) as u64,
        Ordering::Relaxed,
    );
    AVAILABLE.store(true, Ordering::Release);
}

pub(crate) fn dispatch_hotkey_event(event: GlobalHotKeyEvent, delivery: Option<Duration>) {
    let handler = HOTKEY_EVENT_HANDLER
        .get()
        .and_then(|handler| handler.read().ok().and_then(|handler| handler.clone()));
    if let Some(handler) = handler {
        handler(event, delivery);
    }
}

pub(crate) fn set_hotkey_event_handler<F>(handler: F)
where
    F: Fn(GlobalHotKeyEvent, Option<Duration>) + Send + Sync + 'static,
{
    let slot = HOTKEY_EVENT_HANDLER.get_or_init(|| RwLock::new(None));
    *slot.write().unwrap_or_else(|error| error.into_inner()) = Some(Arc::new(handler));
    GlobalHotKeyEvent::set_event_handler(Some(|event: GlobalHotKeyEvent| {
        let delivery = current_hotkey_delivery_delay(event.id);
        dispatch_hotkey_event(event, delivery);
    }));
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
