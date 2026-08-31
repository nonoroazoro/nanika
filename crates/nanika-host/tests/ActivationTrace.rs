use std::time::{Duration, Instant};

use crate::ActivationTrace;

#[test]
fn activation_at_target_is_not_slow() {
    let started_at = Instant::now();
    let trace = ActivationTrace::new(1, "hotkey", started_at, None);

    assert!(!trace.finish(started_at + Duration::from_millis(50)));
}

#[test]
fn activation_over_target_is_slow() {
    let started_at = Instant::now();
    let trace = ActivationTrace::new(2, "hotkey", started_at, None);

    assert!(trace.finish(started_at + Duration::from_millis(51)));
}

#[test]
fn native_delivery_is_included_in_the_slow_threshold() {
    let started_at = Instant::now();
    let trace = ActivationTrace::new(3, "hotkey", started_at, Some(Duration::from_millis(45)));

    assert!(trace.finish(started_at + Duration::from_millis(6)));
}
