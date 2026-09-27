use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

#[test]
fn queue_is_bounded_and_shutdown_drains_admitted_work_even_after_disconnect() {
    let (started, ready) = mpsc::sync_channel(1);
    let (release, blocked) = mpsc::sync_channel(1);
    let completed = Arc::new(AtomicUsize::new(0));
    let observed = completed.clone();
    let service = SystemActionService::_spawn(move |_| {
        if observed.load(Ordering::SeqCst) == 0 {
            started.send(()).unwrap();
            blocked.recv().unwrap();
        }
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        service
            .submit(SystemAction::Lock)
            .unwrap()
            .try_recv()
            .unwrap(),
        Ok(HostServiceResponse::SystemActionSubmitted)
    );
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    for _ in 0..16 {
        drop(service.submit(SystemAction::Lock).unwrap());
    }
    assert!(
        service
            .submit(SystemAction::Lock)
            .unwrap_err()
            .contains("not accepted")
    );
    release.send(()).unwrap();
    drop(service);
    assert_eq!(completed.load(Ordering::SeqCst), 17);
}

#[test]
fn dispatch_failure_does_not_produce_completion_messages_or_retry() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let service = SystemActionService::_spawn(move |_| {
        observed.fetch_add(1, Ordering::SeqCst);
        Err("OS dispatch failed".to_owned())
    })
    .unwrap();
    let receipt = service.submit(SystemAction::Lock).unwrap();
    assert_eq!(
        receipt.try_recv().unwrap(),
        Ok(HostServiceResponse::SystemActionSubmitted)
    );
    drop(service);
    assert!(matches!(
        receipt.try_recv(),
        Err(mpsc::TryRecvError::Disconnected)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
