use super::LanguageChangeObserver;
use objc2_foundation::{NSCurrentLocaleDidChangeNotification, NSNotificationCenter, ns_string};
use std::sync::mpsc;

#[test]
fn foundation_notifications_coalesce_and_removal_works_from_the_shutdown_thread() {
    let (wakes, received) = mpsc::sync_channel(1);
    let observer = LanguageChangeObserver::install(wakes).unwrap();
    let center = NSNotificationCenter::defaultCenter();
    unsafe {
        center.postNotificationName_object(ns_string!("NanikaUnrelatedNotification"), None);
    }
    assert!(received.try_recv().is_err());
    for _ in 0..100 {
        unsafe {
            center.postNotificationName_object(NSCurrentLocaleDidChangeNotification, None);
        }
    }
    received.try_recv().unwrap();
    assert!(received.try_recv().is_err());
    std::thread::spawn(move || drop(observer)).join().unwrap();
    unsafe {
        center.postNotificationName_object(NSCurrentLocaleDidChangeNotification, None);
    }
    assert!(matches!(
        received.try_recv(),
        Err(mpsc::TryRecvError::Disconnected)
    ));
}
