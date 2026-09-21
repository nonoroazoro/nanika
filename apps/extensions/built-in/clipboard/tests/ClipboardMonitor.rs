use super::ClipboardCaptureGate;

#[test]
fn internal_revision_is_ignored_and_next_external_revision_is_captured() {
    let mut gate = ClipboardCaptureGate::default();
    gate.begin_write();
    assert!(!gate.complete_write(11));
    assert!(!gate.observe(11));
    assert!(gate.observe(12));
}

#[test]
fn intermediate_write_revision_does_not_escape_the_gate() {
    let mut gate = ClipboardCaptureGate::default();
    gate.begin_write();
    assert!(!gate.observe(20));
    assert!(!gate.complete_write(21));
    assert!(!gate.observe(21));
    assert!(gate.observe(22));
}

#[test]
fn external_revision_observed_before_the_receipt_is_captured() {
    let mut gate = ClipboardCaptureGate::default();
    gate.begin_write();
    assert!(!gate.observe(31));
    assert!(gate.complete_write(30));
}

#[test]
fn failed_write_releases_an_observed_change_for_capture() {
    let mut gate = ClipboardCaptureGate::default();
    gate.begin_write();
    assert!(!gate.observe(40));
    assert!(gate.cancel_write());
    assert!(gate.observe(41));
}
