use std::time::{Duration, Instant};

use crate::OverlayMotion;

#[test]
fn transition_uses_a_fixed_timeline() {
    let start = Instant::now();
    let mut motion = OverlayMotion::new(false);
    motion.set_target_at(true, start);
    assert!(motion.advance_at(start + Duration::from_millis(70)));
    assert!((motion.value() - 0.5).abs() < 0.001);
    assert!(!motion.advance_at(start + Duration::from_millis(140)));
    assert_eq!(motion.value(), 1.0);
}

#[test]
fn interrupted_transition_continues_from_the_current_value() {
    let start = Instant::now();
    let mut motion = OverlayMotion::new(false);
    motion.set_target_at(true, start);
    motion.set_target_at(false, start + Duration::from_millis(70));
    assert!((motion.value() - 0.5).abs() < 0.001);
    assert!(motion.advance_at(start + Duration::from_millis(125)));
    assert!((motion.value() - 0.25).abs() < 0.001);
}

#[test]
fn reduced_motion_jumps_without_a_repaint_loop() {
    let mut motion = OverlayMotion::new(true);
    motion.set_target(true);
    assert!(!motion.advance());
    assert_eq!(motion.value(), 1.0);
    assert!(!motion.is_active());
}
