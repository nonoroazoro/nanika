use std::time::Duration;

use crate::{record_hotkey_delivery, take_hotkey_delivery_delay};

#[test]
fn delivery_timing_is_consumed_only_by_the_matching_shortcut() {
    record_hotkey_delivery(7, Duration::from_millis(23));

    assert_eq!(take_hotkey_delivery_delay(8), None);
    assert_eq!(
        take_hotkey_delivery_delay(7),
        Some(Duration::from_millis(23))
    );
    assert_eq!(take_hotkey_delivery_delay(7), None);
}
