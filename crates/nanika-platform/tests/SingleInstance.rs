use nanika_platform::{InstanceRole, acquire_instance, signal_activate};

#[test]
fn second_launch_signals_the_primary() {
    let identity = format!("com.nanika.test.{}", std::process::id());
    let root = std::env::temp_dir().join(&identity);
    let primary = acquire_instance(&identity, &root).expect("primary should acquire");
    let mut instance = match primary {
        InstanceRole::Primary(instance) => instance,
        InstanceRole::Secondary => panic!("first launch became secondary"),
    };
    let events = instance.take_events().expect("event receiver should exist");

    assert!(matches!(
        acquire_instance(&identity, &root).expect("second launch should acquire role"),
        InstanceRole::Secondary
    ));
    signal_activate(&identity, &root).expect("second launch should signal activation");
    assert_eq!(
        events
            .recv_timeout(std::time::Duration::from_secs(1))
            .expect("primary should receive activation"),
        nanika_platform::PlatformEvent::Open
    );
    drop(events);
    drop(instance);

    let restarted = acquire_instance(&identity, &root).expect("restarted host should acquire");
    let InstanceRole::Primary(restarted) = restarted else {
        panic!("restarted host became secondary");
    };
    drop(restarted);

    let _ = std::fs::remove_file(root.join("nanika.instance.lock"));
    let _ = std::fs::remove_dir(root);
}
