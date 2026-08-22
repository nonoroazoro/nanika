use std::time::Duration;

use nanika_config::ConfigStore;

use crate::{HostConfig, HostConfigService};

#[test]
fn host_config_updates_run_through_the_owner_and_preserve_comments() {
    let root =
        std::env::temp_dir().join(format!("nanika-host-config-owner-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let store = ConfigStore::open(root.join("data"), root.join("config"))
        .expect("config store should open");
    let path = store.config_file();
    std::fs::write(
        &path,
        "{\n  // keep\n  \"formatVersion\": 1,\n  \"hotkey\": \"Ctrl+Space\",\n  \"reducedMotion\": false\n}\n",
    )
    .expect("host config should exist");
    let service = HostConfigService::spawn(store).expect("config owner should spawn");

    let updated = service
        .update("Alt+Space".to_owned(), true)
        .expect("update should enqueue")
        .recv_timeout(Duration::from_secs(1))
        .expect("owner should reply")
        .expect("update should succeed");

    assert_eq!(
        updated,
        HostConfig {
            format_version: 1,
            hotkey: "Alt+Space".to_owned(),
            reduced_motion: true,
        }
    );
    assert!(
        std::fs::read_to_string(&path)
            .expect("host config should be readable")
            .contains("// keep")
    );
    service.shutdown();
    let _ = std::fs::remove_dir_all(root);
}
