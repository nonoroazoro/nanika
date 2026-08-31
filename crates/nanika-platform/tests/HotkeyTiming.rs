use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use global_hotkey::{GlobalHotKeyEvent, HotKeyState};

use crate::{
    dispatch_hotkey_event, record_hotkey_delivery, set_hotkey_event_handler,
    take_hotkey_delivery_delay,
};

#[test]
fn delivery_timing_is_consumed_only_by_the_matching_hotkey() {
    record_hotkey_delivery(7, Duration::from_millis(23));

    assert_eq!(take_hotkey_delivery_delay(8), None);
    assert_eq!(
        take_hotkey_delivery_delay(7),
        Some(Duration::from_millis(23))
    );
    assert_eq!(take_hotkey_delivery_delay(7), None);
}

#[test]
fn hotkey_event_handler_can_be_replaced() {
    let first_count = Arc::new(AtomicUsize::new(0));
    let first_handler_count = Arc::clone(&first_count);
    set_hotkey_event_handler(move |_, _| {
        first_handler_count.fetch_add(1, Ordering::Relaxed);
    });
    dispatch_hotkey_event(
        GlobalHotKeyEvent {
            id: 1,
            state: HotKeyState::Pressed,
        },
        None,
    );

    let second_count = Arc::new(AtomicUsize::new(0));
    let second_handler_count = Arc::clone(&second_count);
    set_hotkey_event_handler(move |_, _| {
        second_handler_count.fetch_add(1, Ordering::Relaxed);
    });
    dispatch_hotkey_event(
        GlobalHotKeyEvent {
            id: 1,
            state: HotKeyState::Pressed,
        },
        None,
    );

    assert_eq!(first_count.load(Ordering::Relaxed), 1);
    assert_eq!(second_count.load(Ordering::Relaxed), 1);
}
