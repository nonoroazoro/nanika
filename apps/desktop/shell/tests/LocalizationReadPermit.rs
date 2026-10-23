use super::LocalizationReadPermit;
use std::sync::{Arc, Barrier, atomic::AtomicUsize};

#[test]
fn concurrent_reads_reject_overload_and_release_capacity_after_work() {
    let capacity = nanika_localization::MAX_RESOURCE_LOADS;
    let pending = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(Barrier::new(capacity * 2 + 1));
    let release = Arc::new(Barrier::new(capacity * 2 + 1));
    let (send, receive) = std::sync::mpsc::channel();
    let workers = (0..capacity * 2)
        .map(|_| {
            let pending = pending.clone();
            let started = started.clone();
            let release = release.clone();
            let send = send.clone();
            std::thread::spawn(move || {
                started.wait();
                let permit = LocalizationReadPermit::acquire(pending);
                send.send(permit.is_ok()).unwrap();
                release.wait();
                drop(permit);
            })
        })
        .collect::<Vec<_>>();
    started.wait();
    let admitted = (0..capacity * 2)
        .filter(|_| {
            receive
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap()
        })
        .count();
    release.wait();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(admitted, capacity);
    let permits = (0..capacity)
        .map(|_| LocalizationReadPermit::acquire(pending.clone()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        LocalizationReadPermit::acquire(pending.clone()).err(),
        Some(nanika_host::OperationError::new(
            nanika_host::OperationErrorCode::LocalizationBusy
        ))
    );
    drop(permits);
    assert!(LocalizationReadPermit::acquire(pending).is_ok());
}
