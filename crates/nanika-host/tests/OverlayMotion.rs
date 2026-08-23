use std::time::{Duration, Instant};

use crate::OverlayMotion;

#[test]
fn summon_is_ready_immediately() {
    let mut motion = OverlayMotion::new(false);
    motion.show();

    assert_eq!(motion.value(), 1.0);
    assert!(motion.target_visible());
    assert!(!motion.is_active());
}

#[test]
fn dismissal_uses_a_fixed_timeline() {
    let start = Instant::now();
    let mut motion = OverlayMotion::new(false);
    motion.show();
    motion.hide_at(start);

    assert!(motion.advance_at(start + Duration::from_millis(55)));
    assert!((motion.value() - 0.5).abs() < 0.001);
    assert!(!motion.advance_at(start + Duration::from_millis(110)));
    assert_eq!(motion.value(), 0.0);
}

#[test]
fn summon_interrupts_dismissal_immediately() {
    let start = Instant::now();
    let mut motion = OverlayMotion::new(false);
    motion.show();
    motion.hide_at(start);
    assert!(motion.advance_at(start + Duration::from_millis(55)));
    assert!((motion.value() - 0.5).abs() < 0.001);

    motion.show();
    assert_eq!(motion.value(), 1.0);
    assert!(!motion.is_active());
}

#[test]
fn reduced_motion_jumps_without_a_repaint_loop() {
    let mut motion = OverlayMotion::new(true);
    motion.show();
    assert!(!motion.advance());
    assert_eq!(motion.value(), 1.0);
    assert!(!motion.is_active());

    motion.hide();
    assert_eq!(motion.value(), 0.0);
    assert!(!motion.is_active());
}
