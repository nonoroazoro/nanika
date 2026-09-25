use super::ExtensionOperationGate;
use std::sync::{Arc, mpsc};

#[test]
fn releasing_admission_wakes_recovery_even_without_an_operation_result() {
    let gate = Arc::new(ExtensionOperationGate::default());
    let (sent, received) = mpsc::sync_channel(1);
    gate.set_release_notifier(Arc::new(move || {
        let _ = sent.try_send(());
    }));
    let reservation = gate.reserve("extension").unwrap();
    assert!(gate.reserve("extension").is_err());
    assert!(received.try_recv().is_err());
    drop(reservation);
    received
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();
    let reservation = gate.reserve("extension").unwrap();
    gate.close();
    assert!(gate.reserve("other").is_err());
    drop(reservation);
    received
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();
    gate.wait_idle();
}
