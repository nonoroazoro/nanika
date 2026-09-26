use std::sync::mpsc;
use std::time::Duration;

use super::LauncherRefresh;

#[test]
fn opens_are_nonblocking_and_coalesce_while_discovery_runs() {
    let (started, starts) = mpsc::channel();
    let (release, releases) = mpsc::channel();
    let worker = LauncherRefresh::spawn(move || {
        let _ = started.send(());
        let _ = releases.recv();
    })
    .unwrap();
    worker.wake();
    let first = starts.recv_timeout(Duration::from_secs(3));
    for _ in 0..100 {
        worker.wake();
    }
    let overlapped = starts.try_recv().is_ok();
    let _ = release.send(());
    let extra = starts.recv_timeout(Duration::from_millis(50));
    worker.wake();
    let second = starts.recv_timeout(Duration::from_secs(3));
    let _ = release.send(());
    drop(release);
    drop(worker);
    assert!(first.is_ok());
    assert!(!overlapped, "opens must not overlap active discovery");
    assert!(
        matches!(extra, Err(mpsc::RecvTimeoutError::Timeout)),
        "reopens share the active scan without queuing another"
    );
    assert!(second.is_ok(), "an open after completion starts a new scan");
}

#[test]
fn idle_worker_does_not_scan_and_shutdown_closes_it() {
    let (started, starts) = mpsc::channel();
    let worker = LauncherRefresh::spawn(move || {
        let _ = started.send(());
    })
    .unwrap();
    let idle = starts.recv_timeout(Duration::from_millis(50));
    drop(worker);
    assert!(matches!(idle, Err(mpsc::RecvTimeoutError::Timeout)));
    assert!(matches!(starts.recv(), Err(mpsc::RecvError)));
}
