use std::sync::mpsc;
use std::time::Duration;

use super::LanguageRefresh;

#[test]
fn notifications_during_a_read_coalesce_to_one_followup_without_polling() {
    let (started, reads) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let refresh = LanguageRefresh::spawn(move || {
        started.send(()).unwrap();
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
    })
    .unwrap();
    assert!(reads.recv_timeout(Duration::from_millis(30)).is_err());
    refresh.wake();
    reads.recv_timeout(Duration::from_secs(1)).unwrap();
    for _ in 0..100 {
        refresh.wake();
    }
    release.send(()).unwrap();
    reads.recv_timeout(Duration::from_secs(1)).unwrap();
    release.send(()).unwrap();
    assert!(reads.recv_timeout(Duration::from_millis(30)).is_err());
    drop(refresh);
    assert!(reads.recv().is_err());
}

#[test]
fn shutdown_discards_an_unstarted_refresh_and_joins_the_active_read() {
    let (started, reads) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let refresh = LanguageRefresh::spawn(move || {
        started.send(()).unwrap();
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
    })
    .unwrap();
    refresh.wake();
    reads.recv_timeout(Duration::from_secs(1)).unwrap();
    refresh.wake();
    let stopping = refresh._stopping.clone();
    let (done, completion) = mpsc::channel();
    let shutdown = std::thread::spawn(move || {
        drop(refresh);
        done.send(()).unwrap();
    });
    while !stopping.load(std::sync::atomic::Ordering::Acquire) {
        std::thread::yield_now();
    }
    assert!(completion.try_recv().is_err());
    release.send(()).unwrap();
    completion.recv_timeout(Duration::from_secs(1)).unwrap();
    shutdown.join().unwrap();
    assert!(reads.recv().is_err());
}
