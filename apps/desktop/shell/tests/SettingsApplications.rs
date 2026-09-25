use crate::{SettingsApplicationUpdate, SettingsApplications, SettingsEvent, SettingsSaveResult};
use std::sync::{Arc, Mutex};

fn _update(extension: &str, request_id: u64, completed: u32) -> SettingsApplicationUpdate {
    SettingsApplicationUpdate {
        extension_id: extension.to_owned(),
        key: "enabled".to_owned(),
        request_id,
        result: SettingsSaveResult::Running {
            progress: Some(nanika_protocol::OperationProgress {
                label: "Applying".to_owned(),
                completed,
                total: Some(100_000),
            }),
        },
    }
}

fn _receipt(event: &SettingsEvent) -> u64 {
    match event {
        SettingsEvent::Application {
            delivery_id: Some(id),
            ..
        } => *id,
        _ => panic!("expected a progress receipt"),
    }
}

#[test]
fn slow_consumer_has_one_in_flight_message_and_receives_only_latest_progress() {
    let mut state = SettingsApplications::default();
    state.subscribe(tauri::ipc::Channel::new(|_| Ok(())));
    let (_, first) = state.record(_update("test.extension", 1, 0)).unwrap();
    for completed in 1..=100_000 {
        assert!(
            state
                .record(_update("test.extension", 1, completed))
                .is_none()
        );
    }
    let (_, next) = state.acknowledge(_receipt(&first)).unwrap();
    let SettingsEvent::Application { update, .. } = &next else {
        panic!("application")
    };
    let SettingsSaveResult::Running {
        progress: Some(progress),
    } = &update.result
    else {
        panic!("progress")
    };
    assert_eq!(progress.completed, 100_000);
    assert!(state.acknowledge(_receipt(&first)).is_none());
    assert!(state.acknowledge(_receipt(&next)).is_none());
    println!("100001 progress updates: 1 queued before receipt, 1 latest update after receipt");
}

#[test]
fn terminal_bypasses_progress_receipt_and_removes_obsolete_pending_progress() {
    let mut state = SettingsApplications::default();
    state.subscribe(tauri::ipc::Channel::new(|_| Ok(())));
    let (_, first) = state.record(_update("test.extension", 1, 0)).unwrap();
    assert!(state.record(_update("test.extension", 1, 1)).is_none());
    let mut terminal = _update("test.extension", 1, 2);
    terminal.result = SettingsSaveResult::Failed {
        error: "Rejected".to_owned(),
    };
    let (_, event) = state.record(terminal).unwrap();
    assert!(matches!(
        event,
        SettingsEvent::Application {
            delivery_id: None,
            update: SettingsApplicationUpdate {
                result: SettingsSaveResult::Failed { .. },
                ..
            }
        }
    ));
    assert!(state.record(_update("test.extension", 1, 3)).is_none());
    assert!(state.acknowledge(_receipt(&first)).is_none());
    assert!(matches!(
        state.latest["test.extension"].result,
        SettingsSaveResult::Failed { .. }
    ));
}

#[test]
fn progress_is_fair_between_extensions_and_bounded_across_operations() {
    let mut state = SettingsApplications::default();
    state.subscribe(tauri::ipc::Channel::new(|_| Ok(())));
    let (_, first) = state.record(_update("a.extension", 1, 0)).unwrap();
    assert!(state.record(_update("b.extension", 2, 0)).is_none());
    assert!(state.record(_update("a.extension", 3, 1)).is_none());
    let (_, second) = state.acknowledge(_receipt(&first)).unwrap();
    let SettingsEvent::Application { update, .. } = &second else {
        panic!("application")
    };
    assert_eq!(update.extension_id, "b.extension");
    let (_, third) = state.acknowledge(_receipt(&second)).unwrap();
    let SettingsEvent::Application { update, .. } = &third else {
        panic!("application")
    };
    assert_eq!(update.request_id, 3);
}

#[test]
fn old_session_receipts_and_failures_cannot_release_or_disconnect_a_new_session() {
    let mut state = SettingsApplications::default();
    let old = tauri::ipc::Channel::new(|_| Ok(()));
    state.subscribe(old.clone());
    let (_, first) = state.record(_update("test.extension", 1, 0)).unwrap();
    let new = tauri::ipc::Channel::new(|_| Ok(()));
    state.subscribe(new.clone());
    let (_, second) = state.record(_update("test.extension", 1, 1)).unwrap();
    assert_ne!(_receipt(&first), _receipt(&second));
    assert!(state.record(_update("test.extension", 1, 2)).is_none());
    assert!(state.acknowledge(_receipt(&first)).is_none());
    state.disconnect(old.id());
    assert_eq!(state.updates.as_ref().unwrap().id(), new.id());
    assert!(state.acknowledge(_receipt(&second)).is_some());
    state.disconnect(new.id());
    assert!(state.updates.is_none());
    assert!(state.latest.contains_key("test.extension"));
}

#[test]
fn late_submission_cannot_erase_progress_and_transport_can_acknowledge_without_lock() {
    let shared = Arc::new(Mutex::new(SettingsApplications::default()));
    let observer = Arc::clone(&shared);
    let channel = tauri::ipc::Channel::new(move |body| {
        let tauri::ipc::InvokeResponseBody::Json(json) = body else {
            panic!("JSON")
        };
        let event: serde_json::Value = serde_json::from_str(&json).unwrap();
        let id = event["deliveryId"].as_u64().unwrap();
        let mut state = observer
            .try_lock()
            .expect("delivery must release shared state");
        assert!(state.acknowledge(id).is_none());
        Ok(())
    });
    shared.lock().unwrap().subscribe(channel);
    let delivery = shared
        .lock()
        .unwrap()
        .record(_update("test.extension", 1, 1))
        .unwrap();
    let mut initial = _update("test.extension", 1, 0);
    initial.result = SettingsSaveResult::Running { progress: None };
    assert!(shared.lock().unwrap().record(initial).is_none());
    delivery.0.send(delivery.1).unwrap();
    assert!(
        shared
            .lock()
            .unwrap()
            .record(_update("test.extension", 1, 2))
            .is_some()
    );
}

#[test]
fn lifecycle_delivery_is_bounded_and_receipts_are_session_bound() {
    let mut state = SettingsApplications::default();
    state.subscribe(tauri::ipc::Channel::new(|_| Ok(())));
    state.lifecycle_revision = 1;
    let (
        _,
        SettingsEvent::Lifecycle {
            delivery_id: first, ..
        },
    ) = state.next_lifecycle().unwrap()
    else {
        panic!("lifecycle");
    };
    for revision in 2..=100_000 {
        state.lifecycle_revision = revision;
        assert!(state.next_lifecycle().is_none());
    }
    let (
        _,
        SettingsEvent::Lifecycle {
            delivery_id: second,
            revision,
            ..
        },
    ) = state.acknowledge(first).unwrap()
    else {
        panic!("lifecycle");
    };
    assert_eq!(revision, 100_000);
    assert!(state.acknowledge(first).is_none());
    state.subscribe(tauri::ipc::Channel::new(|_| Ok(())));
    let (
        _,
        SettingsEvent::Lifecycle {
            delivery_id: current,
            ..
        },
    ) = state.next_lifecycle().unwrap()
    else {
        panic!("lifecycle");
    };
    state.lifecycle_revision += 1;
    assert!(state.acknowledge(second).is_none());
    assert!(state.next_lifecycle().is_none());
    assert!(state.acknowledge(current).is_some());
}
